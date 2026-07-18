using System.Text.Json;
using Inquivora.Native.Protocol;

namespace Inquivora.Native.Credentials;

public sealed record CredentialCommand(
    string Command,
    string Target,
    string? UserName,
    string? Secret);

/// <summary>
/// §10.4 Credential Manager操作のNDJSONプロトコル。secretはstdout応答以外へ出さない。
/// </summary>
public static class CredentialProtocol
{
    private static readonly JsonSerializerOptions Options = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        PropertyNameCaseInsensitive = true,
    };

    private static readonly string[] Commands = ["set", "get", "has", "delete"];

    private sealed record RawCommand(string? Command, string? Target, string? UserName, string? Secret);

    public static CredentialCommand Parse(string line)
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
        if (raw?.Command is null || !Commands.Contains(raw.Command))
        {
            throw new ProtocolException("INVALID_COMMAND", "set・get・has・deleteのいずれかを指定してください");
        }
        if (string.IsNullOrWhiteSpace(raw.Target))
        {
            throw new ProtocolException("INVALID_COMMAND", "targetは必須です");
        }
        if (raw.Command == "set" && string.IsNullOrEmpty(raw.Secret))
        {
            throw new ProtocolException("INVALID_COMMAND", "setにはsecretが必須です");
        }
        return new CredentialCommand(raw.Command, raw.Target, raw.UserName, raw.Secret);
    }

    private static string Serialize(object value) =>
        JsonSerializer.Serialize(value, value.GetType(), Options);

    public static string Ok() => Serialize(new { type = "credential.ok" });

    public static string SecretResponse(string secret) =>
        Serialize(new { type = "credential.secret", secret });

    public static string NotFound() => Serialize(new { type = "credential.notFound" });

    public static string Error(string code, string message) =>
        Serialize(new { type = "credential.error", code, message });
}
