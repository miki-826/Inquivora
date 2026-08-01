using System.Text.Json;

namespace Inquivora.Native.Protocol;

public sealed record AudioCommand(
    string Command,
    string? MicDeviceId,
    string? LoopbackDeviceId,
    int ChunkSeconds,
    string? OutputDir,
    double MicGain,
    double LoopbackGain);

/// <summary>
/// §9.7 Sidecar音声プロトコル。stdinコマンドとstdoutイベントのNDJSON。
/// </summary>
public static class AudioProtocol
{
    private static readonly JsonSerializerOptions Options = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        PropertyNameCaseInsensitive = true,
    };

    private static readonly string[] Commands = ["start", "pause", "resume", "stop", "listDevices"];

    private sealed record RawCommand(
        string? Command,
        string? MicDeviceId,
        string? LoopbackDeviceId,
        int? ChunkSeconds,
        string? OutputDir,
        double? MicGain,
        double? LoopbackGain);

    public static AudioCommand Parse(string line)
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
            throw new ProtocolException(
                "INVALID_COMMAND", "start・pause・resume・stop・listDevicesのいずれかを指定してください");
        }
        if (raw.Command == "start")
        {
            if (string.IsNullOrWhiteSpace(raw.OutputDir))
            {
                throw new ProtocolException("INVALID_COMMAND", "startにはoutputDirが必須です");
            }
            if (string.IsNullOrWhiteSpace(raw.MicDeviceId) && string.IsNullOrWhiteSpace(raw.LoopbackDeviceId))
            {
                throw new ProtocolException(
                    "INVALID_COMMAND", "micDeviceIdまたはloopbackDeviceIdのいずれかが必要です");
            }
        }
        return new AudioCommand(
            raw.Command,
            raw.MicDeviceId,
            raw.LoopbackDeviceId,
            raw.ChunkSeconds ?? 20,
            raw.OutputDir,
            Math.Clamp(raw.MicGain ?? 1.5, 0.5, 4.0),
            Math.Clamp(raw.LoopbackGain ?? 1.0, 0.5, 4.0));
    }

    private static string Serialize(object value) =>
        JsonSerializer.Serialize(value, value.GetType(), Options);

    public static string StartedEvent(string sessionId) =>
        Serialize(new { type = "audio.started", sessionId });

    public static string StoppedEvent(string sessionId) =>
        Serialize(new { type = "audio.stopped", sessionId });

    public static string LevelEvent(string source, double rms) =>
        Serialize(new { type = "audio.level", source, rms });

    public static string ChunkEvent(string source, string path, long startMs, long endMs) =>
        Serialize(new { type = "audio.chunk", source, path, startMs, endMs });

    public static string DeviceLostEvent(string source, string deviceId) =>
        Serialize(new { type = "audio.deviceLost", source, deviceId });

    public static string ErrorEvent(string code, string message) =>
        Serialize(new { type = "audio.error", code, message });

    public static string DevicesEvent(object mic, object loopback) =>
        Serialize(new { type = "audio.devices", mic, loopback });
}
