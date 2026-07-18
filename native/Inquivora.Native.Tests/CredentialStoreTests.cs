using Inquivora.Native.Credentials;
using Xunit;

namespace Inquivora.Native.Tests;

public class CredentialStoreTests
{
    [Fact]
    public void 保存_取得_存在確認_削除のラウンドトリップができる()
    {
        var target = $"Inquivora/Test/{Guid.NewGuid()}";
        try
        {
            CredentialStore.Write(target, "openai", "sk-test-secret");
            Assert.True(CredentialStore.Exists(target));
            Assert.Equal("sk-test-secret", CredentialStore.Read(target));
            CredentialStore.Write(target, "openai", "sk-rotated");
            Assert.Equal("sk-rotated", CredentialStore.Read(target));
        }
        finally
        {
            CredentialStore.Delete(target);
        }
        Assert.False(CredentialStore.Exists(target));
        Assert.Null(CredentialStore.Read(target));
    }

    [Fact]
    public void 存在しないターゲットの削除はfalseを返す()
    {
        Assert.False(CredentialStore.Delete($"Inquivora/Test/{Guid.NewGuid()}"));
    }

    [Fact]
    public void 日本語や記号を含むシークレットを維持できる()
    {
        var target = $"Inquivora/Test/{Guid.NewGuid()}";
        const string secret = "sk-テスト=+/秘密鍵🔑";
        try
        {
            CredentialStore.Write(target, "user", secret);
            Assert.Equal(secret, CredentialStore.Read(target));
        }
        finally
        {
            CredentialStore.Delete(target);
        }
    }
}
