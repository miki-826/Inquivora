using Microsoft.Win32;
using Windows.Data.Xml.Dom;
using Windows.UI.Notifications;

namespace Inquivora.Native.Notifications;

/// <summary>
/// 未パッケージアプリからWindowsトースト通知を表示する。
/// AUMIDをHKCUレジストリへ登録し、表示名「Inquivora」で通知する。
/// </summary>
public static class NotificationSender
{
    private const string Aumid = "Inquivora.App";

    public static void EnsureRegistered()
    {
        using var key = Registry.CurrentUser.CreateSubKey(
            @"Software\Classes\AppUserModelId\" + Aumid);
        key.SetValue("DisplayName", "Inquivora");
    }

    public static void Show(string toastXml)
    {
        var document = new XmlDocument();
        document.LoadXml(toastXml);
        ToastNotificationManager.CreateToastNotifier(Aumid).Show(new ToastNotification(document));
    }
}
