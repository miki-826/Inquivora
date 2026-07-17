using System.CommandLine;
using Inquivora.Native.Notifications;

var rootCommand = new RootCommand("Inquivora native sidecar");

var notifyCommand = new Command("notify", "stdinのNDJSONコマンドでWindows通知を表示する");
notifyCommand.SetHandler(() =>
{
    Environment.ExitCode = NotifyMode.Run(Console.In, Console.Out, Console.Error);
});
rootCommand.AddCommand(notifyCommand);

return await rootCommand.InvokeAsync(args);
