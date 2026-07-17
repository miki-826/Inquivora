using System.Text.Json;
using Inquivora.Native.Protocol;
using Xunit;

namespace Inquivora.Native.Tests;

public class NdjsonProtocolTests
{
    [Fact]
    public void 正常なnotifyコマンドを解釈できる()
    {
        var line = """{"command":"notify","notificationId":"r1","title":"リマインダー","body":"定例会が10:00から始まります。","launchUri":"inquivora://open?type=event&id=e1","silent":true}""";
        var command = NdjsonProtocol.ParseNotifyCommand(line);
        Assert.Equal("r1", command.NotificationId);
        Assert.Equal("リマインダー", command.Title);
        Assert.Equal("定例会が10:00から始まります。", command.Body);
        Assert.Equal("inquivora://open?type=event&id=e1", command.LaunchUri);
        Assert.True(command.Silent);
    }

    [Fact]
    public void silent省略時はfalseになる()
    {
        var line = """{"command":"notify","notificationId":"r1","title":"t","body":"b","launchUri":"inquivora://open?type=task&id=t1"}""";
        Assert.False(NdjsonProtocol.ParseNotifyCommand(line).Silent);
    }

    [Fact]
    public void jsonでない行は拒否する()
    {
        var ex = Assert.Throws<ProtocolException>(() => NdjsonProtocol.ParseNotifyCommand("これはJSONではない"));
        Assert.Equal("INVALID_COMMAND", ex.Code);
    }

    [Fact]
    public void 別コマンドは拒否する()
    {
        var line = """{"command":"start","notificationId":"r1","title":"t","body":"b","launchUri":"inquivora://open"}""";
        var ex = Assert.Throws<ProtocolException>(() => NdjsonProtocol.ParseNotifyCommand(line));
        Assert.Equal("INVALID_COMMAND", ex.Code);
    }

    [Fact]
    public void 必須項目の欠落は拒否する()
    {
        var line = """{"command":"notify","notificationId":"r1","title":"","body":"b","launchUri":"inquivora://open?type=task&id=t1"}""";
        var ex = Assert.Throws<ProtocolException>(() => NdjsonProtocol.ParseNotifyCommand(line));
        Assert.Equal("INVALID_COMMAND", ex.Code);
    }

    [Fact]
    public void shownイベントを1行のcamelCaseJsonで出力する()
    {
        var line = NdjsonProtocol.NotificationShown("r1");
        Assert.DoesNotContain('\n', line);
        using var doc = JsonDocument.Parse(line);
        Assert.Equal("notification.shown", doc.RootElement.GetProperty("type").GetString());
        Assert.Equal("r1", doc.RootElement.GetProperty("notificationId").GetString());
    }

    [Fact]
    public void errorイベントにコードとメッセージが入る()
    {
        var line = NdjsonProtocol.NotificationError("r1", "NOTIFICATION_FAILED", "表示に失敗");
        using var doc = JsonDocument.Parse(line);
        Assert.Equal("notification.error", doc.RootElement.GetProperty("type").GetString());
        Assert.Equal("r1", doc.RootElement.GetProperty("notificationId").GetString());
        Assert.Equal("NOTIFICATION_FAILED", doc.RootElement.GetProperty("code").GetString());
        Assert.Equal("表示に失敗", doc.RootElement.GetProperty("message").GetString());
    }
}
