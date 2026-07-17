using Inquivora.Native.Protocol;

namespace Inquivora.Native.Notifications;

/// <summary>
/// stdinのNDJSON notifyコマンドを処理し、結果イベントをstdoutへ1行ずつ返す（§14.4）。
/// ログはstderrへ出し、stdoutへ混在させない（§9.7）。
/// </summary>
public static class NotifyMode
{
    public static int Run(TextReader input, TextWriter output, TextWriter log)
    {
        try
        {
            NotificationSender.EnsureRegistered();
        }
        catch (Exception ex)
        {
            log.WriteLine($"通知の初期化に失敗: {ex}");
            output.WriteLine(NdjsonProtocol.NotificationError(
                null, "NOTIFICATION_FAILED", $"通知の初期化に失敗しました: {ex.Message}"));
            output.Flush();
            return 1;
        }

        var failures = 0;
        string? line;
        while ((line = input.ReadLine()) is not null)
        {
            if (string.IsNullOrWhiteSpace(line))
            {
                continue;
            }
            string? notificationId = null;
            try
            {
                var command = NdjsonProtocol.ParseNotifyCommand(line);
                notificationId = command.NotificationId;
                var xml = ToastXmlBuilder.Build(command.Title, command.Body, command.LaunchUri, command.Silent);
                NotificationSender.Show(xml);
                output.WriteLine(NdjsonProtocol.NotificationShown(command.NotificationId));
            }
            catch (ProtocolException ex)
            {
                failures++;
                output.WriteLine(NdjsonProtocol.NotificationError(notificationId, ex.Code, ex.Message));
            }
            catch (Exception ex)
            {
                failures++;
                log.WriteLine($"通知の表示に失敗: {ex}");
                output.WriteLine(NdjsonProtocol.NotificationError(
                    notificationId, "NOTIFICATION_FAILED", ex.Message));
            }
            output.Flush();
        }

        // 表示直後のプロセス終了でトーストが失われないよう少し待つ
        Thread.Sleep(750);
        return failures == 0 ? 0 : 1;
    }
}
