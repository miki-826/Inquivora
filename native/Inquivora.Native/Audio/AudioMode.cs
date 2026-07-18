using Inquivora.Native.Protocol;

namespace Inquivora.Native.Audio;

/// <summary>
/// §9.7 audioモード。stdinの制御コマンドで録音を管理し、イベントをstdoutへ返す。
/// </summary>
public static class AudioMode
{
    public static int Run(string sessionId, TextReader input, TextWriter output, TextWriter log)
    {
        var outputLock = new object();
        void Emit(string line)
        {
            lock (outputLock)
            {
                output.WriteLine(line);
                output.Flush();
            }
        }

        using var engine = new CaptureEngine(sessionId, Emit, log);
        string? line;
        while ((line = input.ReadLine()) is not null)
        {
            if (string.IsNullOrWhiteSpace(line))
            {
                continue;
            }
            try
            {
                var command = AudioProtocol.Parse(line);
                switch (command.Command)
                {
                    case "listDevices":
                        Emit(AudioProtocol.DevicesEvent(
                            CaptureEngine.ListMicDevices(), CaptureEngine.ListLoopbackDevices()));
                        break;
                    case "start":
                        engine.Start(command);
                        break;
                    case "pause":
                        engine.Pause();
                        break;
                    case "resume":
                        engine.Resume();
                        break;
                    case "stop":
                        engine.Stop();
                        return 0;
                }
            }
            catch (ProtocolException ex)
            {
                Emit(AudioProtocol.ErrorEvent(ex.Code, ex.Message));
            }
            catch (Exception ex)
            {
                log.WriteLine($"audioコマンド処理に失敗: {ex}");
                Emit(AudioProtocol.ErrorEvent("AUDIO_CAPTURE_FAILED", ex.Message));
            }
        }
        engine.Stop();
        return 0;
    }
}
