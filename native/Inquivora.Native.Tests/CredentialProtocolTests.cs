using System.Text.Json;
using Inquivora.Native.Credentials;
using Inquivora.Native.Protocol;
using Xunit;

namespace Inquivora.Native.Tests;

public class CredentialProtocolTests
{
    [Fact]
    public void Setコマンドを解析できる()
    {
        var command = CredentialProtocol.Parse(
            """{"command":"set","target":"Inquivora/API/p1","userName":"openai","secret":"sk-abc"}""");
        Assert.Equal("set", command.Command);
        Assert.Equal("Inquivora/API/p1", command.Target);
        Assert.Equal("openai", command.UserName);
        Assert.Equal("sk-abc", command.Secret);
    }

    [Theory]
    [InlineData("get")]
    [InlineData("has")]
    [InlineData("delete")]
    public void Get_Has_Deleteコマンドを解析できる(string name)
    {
        var command = CredentialProtocol.Parse(
            $$"""{"command":"{{name}}","target":"Inquivora/API/p1"}""");
        Assert.Equal(name, command.Command);
        Assert.Equal("Inquivora/API/p1", command.Target);
    }

    [Fact]
    public void Setにsecretがないと例外になる()
    {
        var ex = Assert.Throws<ProtocolException>(() =>
            CredentialProtocol.Parse("""{"command":"set","target":"Inquivora/API/p1"}"""));
        Assert.Equal("INVALID_COMMAND", ex.Code);
    }

    [Fact]
    public void 不明なコマンドは例外になる()
    {
        Assert.Throws<ProtocolException>(() =>
            CredentialProtocol.Parse("""{"command":"steal","target":"x"}"""));
        Assert.Throws<ProtocolException>(() => CredentialProtocol.Parse("not json"));
        Assert.Throws<ProtocolException>(() =>
            CredentialProtocol.Parse("""{"command":"get"}"""));
    }

    [Fact]
    public void 応答イベントを構築できる()
    {
        Assert.Equal("credential.ok", JsonDocument.Parse(CredentialProtocol.Ok())
            .RootElement.GetProperty("type").GetString());
        var secret = JsonDocument.Parse(CredentialProtocol.SecretResponse("sk-abc")).RootElement;
        Assert.Equal("credential.secret", secret.GetProperty("type").GetString());
        Assert.Equal("sk-abc", secret.GetProperty("secret").GetString());
        Assert.Equal("credential.notFound", JsonDocument.Parse(CredentialProtocol.NotFound())
            .RootElement.GetProperty("type").GetString());
        var error = JsonDocument.Parse(CredentialProtocol.Error("CRED_WRITE_FAILED", "失敗")).RootElement;
        Assert.Equal("credential.error", error.GetProperty("type").GetString());
        Assert.Equal("CRED_WRITE_FAILED", error.GetProperty("code").GetString());
    }
}
