namespace Inquivora.Native.Audio;

public sealed record AudioChunk(float[] Samples, long StartMs, long EndMs);

/// <summary>
/// §9.6 チャンク分割。チャンク長20秒・オーバーラップ1秒（ストライド19秒）。
/// サンプルは録音開始からの連続タイムラインで管理する。
/// </summary>
public sealed class AudioChunker
{
    private readonly int _sampleRate;
    private readonly int _chunkSamples;
    private readonly int _strideSamples;
    private readonly List<float> _buffer = [];
    private long _bufferStart;
    private long _totalWritten;
    private long _nextChunkStart;
    private long _lastEmittedEnd;

    public AudioChunker(int sampleRate, int chunkSeconds, int overlapSeconds)
    {
        if (overlapSeconds >= chunkSeconds)
        {
            throw new ArgumentException("オーバーラップはチャンク長より短くしてください");
        }
        _sampleRate = sampleRate;
        _chunkSamples = sampleRate * chunkSeconds;
        _strideSamples = sampleRate * (chunkSeconds - overlapSeconds);
    }

    public IReadOnlyList<AudioChunk> Add(float[] samples)
    {
        _buffer.AddRange(samples);
        _totalWritten += samples.Length;
        var chunks = new List<AudioChunk>();
        while (_totalWritten - _nextChunkStart >= _chunkSamples)
        {
            chunks.Add(TakeChunk(_nextChunkStart, _nextChunkStart + _chunkSamples));
            _lastEmittedEnd = _nextChunkStart + _chunkSamples;
            _nextChunkStart += _strideSamples;
            TrimBuffer();
        }
        return chunks;
    }

    /// <summary>停止時の端数チャンク。未送信部分が1秒未満なら破棄する。</summary>
    public AudioChunk? Flush()
    {
        if (_totalWritten - _lastEmittedEnd < _sampleRate || _totalWritten <= _nextChunkStart)
        {
            return null;
        }
        var chunk = TakeChunk(_nextChunkStart, _totalWritten);
        _lastEmittedEnd = _totalWritten;
        _nextChunkStart = _totalWritten;
        TrimBuffer();
        return chunk;
    }

    private AudioChunk TakeChunk(long start, long end)
    {
        var offset = (int)(start - _bufferStart);
        var length = (int)(end - start);
        var samples = new float[length];
        _buffer.CopyTo(offset, samples, 0, length);
        return new AudioChunk(samples, ToMs(start), ToMs(end));
    }

    private void TrimBuffer()
    {
        var keepFrom = (int)(_nextChunkStart - _bufferStart);
        if (keepFrom > 0)
        {
            _buffer.RemoveRange(0, keepFrom);
            _bufferStart = _nextChunkStart;
        }
    }

    private long ToMs(long samplePosition) => samplePosition * 1000 / _sampleRate;
}
