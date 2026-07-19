using System.Text.Json;

namespace Inquivora.Native.Whisper;

public enum WhisperCommandKind
{
    Transcribe,
    Stop,
}

public sealed record WhisperCommand(
    WhisperCommandKind Kind,
    string? ModelPath,
    string? WavPath,
    string Language);

/// <summary>
/// ローカルWhisper文字起こしのNDJSONプロトコル。
/// </summary>
public static class WhisperProtocol
{
    private static readonly JsonSerializerOptions Options = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        PropertyNameCaseInsensitive = true,
        Encoder = System.Text.Encodings.Web.JavaScriptEncoder.UnsafeRelaxedJsonEscaping,
    };

    private sealed record RawCommand(string? Command, string? ModelPath, string? WavPath, string? Language);

    public static WhisperCommand? Parse(string line)
    {
        line = line.TrimStart((char)0xFEFF);
        RawCommand? raw;
        try
        {
            raw = JsonSerializer.Deserialize<RawCommand>(line, Options);
        }
        catch (JsonException)
        {
            return null;
        }
        switch (raw?.Command)
        {
            case "stop":
                return new WhisperCommand(WhisperCommandKind.Stop, null, null, "ja");
            case "transcribe":
                if (string.IsNullOrWhiteSpace(raw.ModelPath) || string.IsNullOrWhiteSpace(raw.WavPath))
                {
                    return null;
                }
                var language = string.IsNullOrWhiteSpace(raw.Language) ? "ja" : raw.Language;
                return new WhisperCommand(WhisperCommandKind.Transcribe, raw.ModelPath, raw.WavPath, language);
            default:
                return null;
        }
    }

    /// <summary>
    /// whisper.cppのトークン分断で混入するU+FFFDを除去する。
    /// </summary>
    public static string SanitizeText(string text) =>
        text.Replace("�", string.Empty).Trim();

    private static string Serialize(object value) =>
        JsonSerializer.Serialize(value, value.GetType(), Options);

    public static string ResultEvent(string text) =>
        Serialize(new { @event = "transcribe.result", text });

    public static string ErrorEvent(string code, string message) =>
        Serialize(new { @event = "transcribe.error", code, message });
}
