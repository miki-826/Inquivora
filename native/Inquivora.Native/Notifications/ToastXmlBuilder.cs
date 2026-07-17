using System.Security;

namespace Inquivora.Native.Notifications;

public static class ToastXmlBuilder
{
    public static string Build(string title, string body, string launchUri, bool silent)
    {
        if (!Uri.TryCreate(launchUri, UriKind.Absolute, out var uri) || uri.Scheme != "inquivora")
        {
            throw new ArgumentException($"起動URIはinquivoraスキームである必要があります: {launchUri}");
        }
        var escapedUri = SecurityElement.Escape(launchUri);
        var escapedTitle = SecurityElement.Escape(title);
        var escapedBody = SecurityElement.Escape(body);
        var audio = silent ? "<audio silent=\"true\"/>" : "";
        return $"<toast activationType=\"protocol\" launch=\"{escapedUri}\">"
            + $"<visual><binding template=\"ToastGeneric\"><text>{escapedTitle}</text><text>{escapedBody}</text></binding></visual>"
            + audio
            + "</toast>";
    }
}
