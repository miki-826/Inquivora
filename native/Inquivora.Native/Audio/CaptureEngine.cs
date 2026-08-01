using Inquivora.Native.Protocol;
using NAudio.CoreAudioApi;
using NAudio.Wave;

namespace Inquivora.Native.Audio;

/// <summary>
/// WASAPIでマイク・ループバックを取得し、16kHz mono 16bitのチャンクWAVを出力する（§9.6）。
/// </summary>
public sealed class CaptureEngine : IDisposable
{
    public const int TargetSampleRate = 16000;

    private readonly string _sessionId;
    private readonly Action<string> _emit;
    private readonly TextWriter _log;
    private SourceCapture? _mic;
    private SourceCapture? _loopback;

    public CaptureEngine(string sessionId, Action<string> emit, TextWriter log)
    {
        _sessionId = sessionId;
        _emit = emit;
        _log = log;
    }

    public void Start(AudioCommand command)
    {
        Directory.CreateDirectory(command.OutputDir!);
        if (!string.IsNullOrWhiteSpace(command.MicDeviceId))
        {
            _mic = SourceCapture.Create("mic", command.MicDeviceId!, command, _emit, _log);
        }
        if (!string.IsNullOrWhiteSpace(command.LoopbackDeviceId))
        {
            _loopback = SourceCapture.Create("loopback", command.LoopbackDeviceId!, command, _emit, _log);
        }
        _mic?.Start();
        _loopback?.Start();
        _emit(AudioProtocol.StartedEvent(_sessionId));
    }

    public void Pause()
    {
        _mic?.Pause();
        _loopback?.Pause();
    }

    public void Resume()
    {
        _mic?.Resume();
        _loopback?.Resume();
    }

    public void Stop()
    {
        _mic?.Stop();
        _loopback?.Stop();
        _emit(AudioProtocol.StoppedEvent(_sessionId));
    }

    public void Dispose()
    {
        _mic?.Dispose();
        _loopback?.Dispose();
    }

    public static object[] ListMicDevices()
    {
        using var enumerator = new MMDeviceEnumerator();
        return ListDevices(enumerator, DataFlow.Capture);
    }

    public static object[] ListLoopbackDevices()
    {
        using var enumerator = new MMDeviceEnumerator();
        return ListDevices(enumerator, DataFlow.Render);
    }

    private static object[] ListDevices(MMDeviceEnumerator enumerator, DataFlow flow)
    {
        var defaultId = enumerator.HasDefaultAudioEndpoint(flow, Role.Multimedia)
            ? enumerator.GetDefaultAudioEndpoint(flow, Role.Multimedia).ID
            : null;
        return enumerator
            .EnumerateAudioEndPoints(flow, DeviceState.Active)
            .Select(device => (object)new
            {
                id = device.ID,
                name = device.FriendlyName,
                isDefault = device.ID == defaultId,
            })
            .ToArray();
    }
}

internal sealed class SourceCapture : IDisposable
{
    private readonly string _source;
    private readonly string _outputDir;
    private readonly Action<string> _emit;
    private readonly TextWriter _log;
    private readonly WasapiCapture _capture;
    private readonly AudioChunker _chunker;
    private readonly double _gain;
    private readonly object _lock = new();
    private int _chunkIndex;
    private long _levelSamples;
    private double _levelSumSquares;
    private bool _stopped;

    private SourceCapture(
        string source,
        WasapiCapture capture,
        AudioCommand command,
        Action<string> emit,
        TextWriter log)
    {
        _source = source;
        _capture = capture;
        _outputDir = command.OutputDir!;
        _emit = emit;
        _log = log;
        _gain = source == "mic" ? command.MicGain : command.LoopbackGain;
        _chunker = new AudioChunker(CaptureEngine.TargetSampleRate, command.ChunkSeconds, 1);
        _capture.DataAvailable += OnDataAvailable;
        _capture.RecordingStopped += OnRecordingStopped;
    }

    public static SourceCapture Create(
        string source,
        string deviceId,
        AudioCommand command,
        Action<string> emit,
        TextWriter log)
    {
        using var enumerator = new MMDeviceEnumerator();
        var flow = source == "mic" ? DataFlow.Capture : DataFlow.Render;
        MMDevice device;
        try
        {
            device = deviceId == "default"
                ? enumerator.GetDefaultAudioEndpoint(flow, Role.Multimedia)
                : enumerator.GetDevice(deviceId);
        }
        catch (Exception ex)
        {
            throw new ProtocolException("AUDIO_DEVICE_NOT_FOUND", $"音声デバイスが見つかりません: {ex.Message}");
        }
        WasapiCapture capture = source == "mic"
            ? new WasapiCapture(device)
            : new WasapiLoopbackCapture(device);
        return new SourceCapture(source, capture, command, emit, log);
    }

    public void Start() => _capture.StartRecording();

    public void Pause()
    {
        _capture.StopRecording();
    }

    public void Resume()
    {
        if (!_stopped)
        {
            _capture.StartRecording();
        }
    }

    public void Stop()
    {
        _stopped = true;
        _capture.StopRecording();
        lock (_lock)
        {
            var rest = _chunker.Flush();
            if (rest is not null)
            {
                EmitChunk(rest);
            }
        }
    }

    private void OnDataAvailable(object? sender, WaveInEventArgs e)
    {
        try
        {
            var format = _capture.WaveFormat;
            var floats = ConvertToFloat(e.Buffer, e.BytesRecorded, format);
            var mono = _source == "mic"
                ? LinearResampler.ToStrongestChannelMono(floats, format.Channels)
                : LinearResampler.ToMono(floats, format.Channels);
            var resampled = LinearResampler.Resample(mono, format.SampleRate, CaptureEngine.TargetSampleRate);
            var adjusted = AudioMath.ApplyGain(resampled, _gain);
            lock (_lock)
            {
                if (_stopped)
                {
                    return;
                }
                foreach (var chunk in _chunker.Add(adjusted))
                {
                    EmitChunk(chunk);
                }
                TrackLevel(adjusted);
            }
        }
        catch (Exception ex)
        {
            _log.WriteLine($"音声データ処理に失敗 ({_source}): {ex}");
            _emit(AudioProtocol.ErrorEvent("AUDIO_CAPTURE_FAILED", ex.Message));
        }
    }

    private void OnRecordingStopped(object? sender, StoppedEventArgs e)
    {
        if (e.Exception is not null && !_stopped)
        {
            _log.WriteLine($"録音が停止 ({_source}): {e.Exception}");
            _emit(AudioProtocol.DeviceLostEvent(_source, _capture.WaveFormat.ToString() ?? _source));
        }
    }

    private void EmitChunk(AudioChunk chunk)
    {
        if (AudioMath.IsSilent(chunk.Samples))
        {
            return;
        }
        var path = Path.Combine(_outputDir, $"{_source}_{_chunkIndex++:D4}.wav");
        WavFile.WritePcm16Mono(path, chunk.Samples, CaptureEngine.TargetSampleRate);
        _emit(AudioProtocol.ChunkEvent(_source, path, chunk.StartMs, chunk.EndMs));
    }

    private void TrackLevel(float[] samples)
    {
        foreach (var sample in samples)
        {
            _levelSumSquares += (double)sample * sample;
        }
        _levelSamples += samples.Length;
        if (_levelSamples >= CaptureEngine.TargetSampleRate)
        {
            var rms = Math.Sqrt(_levelSumSquares / _levelSamples);
            _emit(AudioProtocol.LevelEvent(_source, Math.Round(rms, 4)));
            _levelSamples = 0;
            _levelSumSquares = 0;
        }
    }

    private static float[] ConvertToFloat(byte[] buffer, int bytes, WaveFormat format)
    {
        if (format.BitsPerSample == 32)
        {
            var samples = new float[bytes / 4];
            Buffer.BlockCopy(buffer, 0, samples, 0, bytes);
            return samples;
        }
        if (format.BitsPerSample == 16)
        {
            var samples = new float[bytes / 2];
            for (var i = 0; i < samples.Length; i++)
            {
                samples[i] = BitConverter.ToInt16(buffer, i * 2) / 32768f;
            }
            return samples;
        }
        throw new NotSupportedException($"未対応の音声形式です: {format.BitsPerSample}bit");
    }

    public void Dispose()
    {
        _capture.DataAvailable -= OnDataAvailable;
        _capture.RecordingStopped -= OnRecordingStopped;
        _capture.Dispose();
    }
}
