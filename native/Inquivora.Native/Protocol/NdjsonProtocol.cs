using System.Text.Json;

namespace Inquivora.Native.Protocol;

public sealed record NotifyCommand(
    string NotificationId,
    string Title,
    string Body,
    string LaunchUri,
    bool Silent);

public sealed class ProtocolException : Exception
{
    public string Code { get; }

    public ProtocolException(string code, string message) : base(message)
    {
        Code = code;
    }
}

public static class NdjsonProtocol
{
    private static readonly JsonSerializerOptions Options = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        PropertyNameCaseInsensitive = true,
    };

    private sealed record RawCommand(
        string? Command,
        string? NotificationId,
        string? Title,
        string? Body,
        string? LaunchUri,
        bool? Silent);

    public static NotifyCommand ParseNotifyCommand(string line)
    {
        line = line.TrimStart((char)0xFEFF);
        RawCommand? raw;
        try
        {
            raw = JsonSerializer.Deserialize<RawCommand>(line, Options);
        }
        catch (JsonException ex)
        {
            throw new ProtocolException("INVALID_COMMAND", $"JSONを解釈できません: {ex.Message}");
        }
        if (raw is null || raw.Command != "notify")
        {
            throw new ProtocolException("INVALID_COMMAND", "notifyコマンドではありません");
        }
        if (string.IsNullOrWhiteSpace(raw.NotificationId)
            || string.IsNullOrWhiteSpace(raw.Title)
            || string.IsNullOrWhiteSpace(raw.Body)
            || string.IsNullOrWhiteSpace(raw.LaunchUri))
        {
            throw new ProtocolException(
                "INVALID_COMMAND",
                "notificationId・title・body・launchUriは必須です");
        }
        return new NotifyCommand(raw.NotificationId, raw.Title, raw.Body, raw.LaunchUri, raw.Silent ?? false);
    }

    private static string Serialize(object value) =>
        JsonSerializer.Serialize(value, value.GetType(), Options);

    public static string NotificationShown(string notificationId) =>
        Serialize(new { type = "notification.shown", notificationId });

    public static string NotificationError(string? notificationId, string code, string message) =>
        Serialize(new { type = "notification.error", notificationId, code, message });
}
