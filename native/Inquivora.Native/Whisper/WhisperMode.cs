using System.Diagnostics;
using System.Text;
using Whisper.net;

namespace Inquivora.Native.Whisper;

/// <summary>
/// stdinのNDJSONコマンドでローカルWhisper（whisper.cpp）文字起こしを行う。
/// モデルはmodelPathごとにキャッシュし、同一プロセス内の連続transcribeで再利用する。
/// </summary>
public static class WhisperMode
{
    // UIを優先するため、推論に使うスレッドはCPUの半分に抑える（残りをUI・OSへ残す）。
    private static readonly int Threads = Math.Max(1, Environment.ProcessorCount / 2);

    public static int Run(TextReader input, TextWriter output, TextWriter log)
    {
        // 文字起こし中もアプリ本体（UI）が優先されるよう、このプロセスの優先度を下げる。
        try
        {
            Process.GetCurrentProcess().PriorityClass = ProcessPriorityClass.BelowNormal;
        }
        catch
        {
            // 権限等で設定できなくても続行する。
        }

        WhisperFactory? factory = null;
        string? loadedModelPath = null;
        try
        {
            string? line;
            while ((line = input.ReadLine()) is not null)
            {
                if (string.IsNullOrWhiteSpace(line))
                {
                    continue;
                }
                var command = WhisperProtocol.Parse(line);
                if (command is null)
                {
                    output.WriteLine(WhisperProtocol.ErrorEvent("INVALID_COMMAND", "コマンドを解釈できません"));
                    output.Flush();
                    continue;
                }
                if (command.Kind == WhisperCommandKind.Stop)
                {
                    return 0;
                }
                try
                {
                    if (!File.Exists(command.ModelPath))
                    {
                        output.WriteLine(WhisperProtocol.ErrorEvent(
                            "WHISPER_MODEL_NOT_FOUND",
                            "Whisperモデルファイルが見つかりません。設定画面でダウンロードしてください"));
                        output.Flush();
                        continue;
                    }
                    if (!File.Exists(command.WavPath))
                    {
                        output.WriteLine(WhisperProtocol.ErrorEvent(
                            "WHISPER_AUDIO_NOT_FOUND", "音声チャンクファイルが見つかりません"));
                        output.Flush();
                        continue;
                    }
                    if (factory is null || loadedModelPath != command.ModelPath)
                    {
                        factory?.Dispose();
                        factory = WhisperFactory.FromPath(command.ModelPath!);
                        loadedModelPath = command.ModelPath;
                    }
                    var text = Transcribe(factory, command.WavPath!, command.Language);
                    output.WriteLine(WhisperProtocol.ResultEvent(text));
                }
                catch (Exception ex)
                {
                    log.WriteLine($"Whisper文字起こしに失敗: {ex.GetType().Name}: {ex.Message}");
                    output.WriteLine(WhisperProtocol.ErrorEvent("WHISPER_FAILED", ex.Message));
                }
                output.Flush();
            }
            return 0;
        }
        finally
        {
            factory?.Dispose();
        }
    }

    private static string Transcribe(WhisperFactory factory, string wavPath, string language)
    {
        using var processor = factory.CreateBuilder().WithLanguage(language).WithThreads(Threads).Build();
        using var stream = File.OpenRead(wavPath);
        var builder = new StringBuilder();
        var task = Task.Run(async () =>
        {
            await foreach (var segment in processor.ProcessAsync(stream))
            {
                builder.Append(segment.Text);
            }
        });
        task.GetAwaiter().GetResult();
        return WhisperProtocol.SanitizeText(builder.ToString());
    }
}
