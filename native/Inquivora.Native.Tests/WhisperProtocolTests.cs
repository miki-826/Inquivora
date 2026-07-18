using System.Text.Json;
using Inquivora.Native.Whisper;
using Xunit;

namespace Inquivora.Native.Tests;

public class WhisperProtocolTests
{
    [Fact]
    public void Parse_TranscribeCommand()
    {
        var command = WhisperProtocol.Parse(
            """{"command":"transcribe","modelPath":"C:\\models\\ggml-small.bin","wavPath":"C:\\audio\\mic_0001.wav","language":"ja"}""");
        Assert.NotNull(command);
        Assert.Equal(WhisperCommandKind.Transcribe, command!.Kind);
        Assert.Equal(@"C:\models\ggml-small.bin", command.ModelPath);
        Assert.Equal(@"C:\audio\mic_0001.wav", command.WavPath);
        Assert.Equal("ja", command.Language);
    }

    [Fact]
    public void Parse_LanguageDefaultsToJa()
    {
        var command = WhisperProtocol.Parse(
            """{"command":"transcribe","modelPath":"m.bin","wavPath":"a.wav"}""");
        Assert.Equal("ja", command!.Language);
    }

    [Fact]
    public void Parse_StopCommand()
    {
        var command = WhisperProtocol.Parse("""{"command":"stop"}""");
        Assert.Equal(WhisperCommandKind.Stop, command!.Kind);
    }

    [Fact]
    public void Parse_MissingModelPathReturnsNull()
    {
        Assert.Null(WhisperProtocol.Parse("""{"command":"transcribe","wavPath":"a.wav"}"""));
    }

    [Fact]
    public void Parse_MissingWavPathReturnsNull()
    {
        Assert.Null(WhisperProtocol.Parse("""{"command":"transcribe","modelPath":"m.bin"}"""));
    }

    [Fact]
    public void Parse_UnknownCommandReturnsNull()
    {
        Assert.Null(WhisperProtocol.Parse("""{"command":"unknown"}"""));
        Assert.Null(WhisperProtocol.Parse("not json"));
    }

    [Fact]
    public void ResultEvent_ContainsText()
    {
        var json = WhisperProtocol.ResultEvent("こんにちは 会議を始めます");
        using var doc = JsonDocument.Parse(json);
        Assert.Equal("transcribe.result", doc.RootElement.GetProperty("event").GetString());
        Assert.Equal("こんにちは 会議を始めます", doc.RootElement.GetProperty("text").GetString());
    }

    [Fact]
    public void ErrorEvent_ContainsCodeAndMessage()
    {
        var json = WhisperProtocol.ErrorEvent("WHISPER_MODEL_LOAD_FAILED", "モデルの読み込みに失敗");
        using var doc = JsonDocument.Parse(json);
        Assert.Equal("transcribe.error", doc.RootElement.GetProperty("event").GetString());
        Assert.Equal("WHISPER_MODEL_LOAD_FAILED", doc.RootElement.GetProperty("code").GetString());
        Assert.Equal("モデルの読み込みに失敗", doc.RootElement.GetProperty("message").GetString());
    }

    [Fact]
    public void Parse_ToleratesBom()
    {
        var command = WhisperProtocol.Parse(
            "﻿" + """{"command":"transcribe","modelPath":"m.bin","wavPath":"a.wav"}""");
        Assert.NotNull(command);
    }
}
