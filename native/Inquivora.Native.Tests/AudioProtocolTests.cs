using System.Text.Json;
using Inquivora.Native.Protocol;
using Xunit;

namespace Inquivora.Native.Tests;

public class AudioProtocolTests
{
    [Fact]
    public void Startコマンドを解析できる()
    {
        var command = AudioProtocol.Parse(
            """{"command":"start","micDeviceId":"default","loopbackDeviceId":"default","chunkSeconds":20,"outputDir":"C:/audio"}""");
        Assert.Equal("start", command.Command);
        Assert.Equal("default", command.MicDeviceId);
        Assert.Equal("default", command.LoopbackDeviceId);
        Assert.Equal(20, command.ChunkSeconds);
        Assert.Equal("C:/audio", command.OutputDir);
        Assert.Equal(1.5, command.MicGain);
        Assert.Equal(1.0, command.LoopbackGain);
    }

    [Fact]
    public void GainValuesAreClampedToSupportedRange()
    {
        var command = AudioProtocol.Parse(
            """{"command":"start","micDeviceId":"default","outputDir":"C:/audio","micGain":9,"loopbackGain":0.1}""");
        Assert.Equal(4.0, command.MicGain);
        Assert.Equal(0.5, command.LoopbackGain);
    }

    [Fact]
    public void ChunkSecondsは省略時20になる()
    {
        var command = AudioProtocol.Parse(
            """{"command":"start","micDeviceId":"default","outputDir":"C:/audio"}""");
        Assert.Equal(20, command.ChunkSeconds);
        Assert.Null(command.LoopbackDeviceId);
    }

    [Theory]
    [InlineData("pause")]
    [InlineData("resume")]
    [InlineData("stop")]
    [InlineData("listDevices")]
    public void 制御コマンドを解析できる(string name)
    {
        var command = AudioProtocol.Parse($$"""{"command":"{{name}}"}""");
        Assert.Equal(name, command.Command);
    }

    [Fact]
    public void Startにはマイクかループバックが必要()
    {
        var ex = Assert.Throws<ProtocolException>(() =>
            AudioProtocol.Parse("""{"command":"start","outputDir":"C:/audio"}"""));
        Assert.Equal("INVALID_COMMAND", ex.Code);
    }

    [Fact]
    public void StartにはoutputDirが必要()
    {
        Assert.Throws<ProtocolException>(() =>
            AudioProtocol.Parse("""{"command":"start","micDeviceId":"default"}"""));
    }

    [Fact]
    public void 不明なコマンドは例外になる()
    {
        Assert.Throws<ProtocolException>(() => AudioProtocol.Parse("""{"command":"jump"}"""));
        Assert.Throws<ProtocolException>(() => AudioProtocol.Parse("not json"));
    }

    [Fact]
    public void チャンクイベントを構築できる()
    {
        var json = AudioProtocol.ChunkEvent("mic", "C:/audio/c0.wav", 0, 20000);
        var root = JsonDocument.Parse(json).RootElement;
        Assert.Equal("audio.chunk", root.GetProperty("type").GetString());
        Assert.Equal("mic", root.GetProperty("source").GetString());
        Assert.Equal("C:/audio/c0.wav", root.GetProperty("path").GetString());
        Assert.Equal(0, root.GetProperty("startMs").GetInt64());
        Assert.Equal(20000, root.GetProperty("endMs").GetInt64());
        Assert.DoesNotContain('\n', json);
    }

    [Fact]
    public void 各種イベントを構築できる()
    {
        Assert.Equal("audio.started", JsonDocument.Parse(AudioProtocol.StartedEvent("s1"))
            .RootElement.GetProperty("type").GetString());
        Assert.Equal("audio.stopped", JsonDocument.Parse(AudioProtocol.StoppedEvent("s1"))
            .RootElement.GetProperty("type").GetString());
        var level = JsonDocument.Parse(AudioProtocol.LevelEvent("loopback", 0.42)).RootElement;
        Assert.Equal("audio.level", level.GetProperty("type").GetString());
        Assert.Equal(0.42, level.GetProperty("rms").GetDouble(), 2);
        var error = JsonDocument.Parse(AudioProtocol.ErrorEvent("CAPTURE_FAILED", "失敗")).RootElement;
        Assert.Equal("audio.error", error.GetProperty("type").GetString());
        Assert.Equal("CAPTURE_FAILED", error.GetProperty("code").GetString());
        var lost = JsonDocument.Parse(AudioProtocol.DeviceLostEvent("loopback", "dev-1")).RootElement;
        Assert.Equal("audio.deviceLost", lost.GetProperty("type").GetString());
        Assert.Equal("dev-1", lost.GetProperty("deviceId").GetString());
    }
}
