using Inquivora.Native.Protocol;

namespace Inquivora.Native.Credentials;

/// <summary>
/// stdinのNDJSONコマンドでCredential Managerを操作する。ログはstderrへ出す。
/// シークレット値は応答のsecretフィールド以外へ絶対に出力しない。
/// </summary>
public static class CredentialMode
{
    public static int Run(TextReader input, TextWriter output, TextWriter log)
    {
        string? line;
        while ((line = input.ReadLine()) is not null)
        {
            if (string.IsNullOrWhiteSpace(line))
            {
                continue;
            }
            try
            {
                var command = CredentialProtocol.Parse(line);
                switch (command.Command)
                {
                    case "set":
                        CredentialStore.Write(command.Target, command.UserName ?? "api", command.Secret!);
                        output.WriteLine(CredentialProtocol.Ok());
                        break;
                    case "get":
                        var secret = CredentialStore.Read(command.Target);
                        output.WriteLine(secret is null
                            ? CredentialProtocol.NotFound()
                            : CredentialProtocol.SecretResponse(secret));
                        break;
                    case "has":
                        output.WriteLine(CredentialStore.Exists(command.Target)
                            ? CredentialProtocol.Ok()
                            : CredentialProtocol.NotFound());
                        break;
                    case "delete":
                        output.WriteLine(CredentialStore.Delete(command.Target)
                            ? CredentialProtocol.Ok()
                            : CredentialProtocol.NotFound());
                        break;
                }
            }
            catch (ProtocolException ex)
            {
                output.WriteLine(CredentialProtocol.Error(ex.Code, ex.Message));
            }
            catch (Exception ex)
            {
                log.WriteLine($"資格情報の操作に失敗: {ex.GetType().Name}");
                output.WriteLine(CredentialProtocol.Error("CREDENTIAL_ERROR", ex.Message));
            }
            output.Flush();
        }
        return 0;
    }
}
