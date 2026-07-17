namespace Inquivora.Native.Protocol;

public sealed record NotifyCommand(
    string NotificationId,
    string Title,
    string Body,
    string LaunchUri,
    bool Silent);

public sealed class ProtocolException : Exception
{
    public string Code { get; }

    public ProtocolException(string code, string message) : base(message)
    {
        Code = code;
    }
}

public static class NdjsonProtocol
{
    public static NotifyCommand ParseNotifyCommand(string line)
    {
        throw new NotImplementedException();
    }

    public static string NotificationShown(string notificationId)
    {
        throw new NotImplementedException();
    }

    public static string NotificationError(string? notificationId, string code, string message)
    {
        throw new NotImplementedException();
    }
}
