using System.CommandLine;
using Inquivora.Native.Audio;
using Inquivora.Native.Credentials;
using Inquivora.Native.Notifications;

var rootCommand = new RootCommand("Inquivora native sidecar");

var notifyCommand = new Command("notify", "stdinのNDJSONコマンドでWindows通知を表示する");
notifyCommand.SetHandler(() =>
{
    Environment.ExitCode = NotifyMode.Run(Console.In, Console.Out, Console.Error);
});
rootCommand.AddCommand(notifyCommand);

var credentialCommand = new Command("credential", "stdinのNDJSONコマンドでWindows Credential Managerを操作する");
credentialCommand.SetHandler(() =>
{
    Environment.ExitCode = CredentialMode.Run(Console.In, Console.Out, Console.Error);
});
rootCommand.AddCommand(credentialCommand);

var sessionOption = new Option<string>("--session", "録音セッションID") { IsRequired = true };
var audioCommand = new Command("audio", "stdinの制御コマンドでマイク・ループバック録音を行う");
audioCommand.AddOption(sessionOption);
audioCommand.SetHandler(session =>
{
    Environment.ExitCode = AudioMode.Run(session, Console.In, Console.Out, Console.Error);
}, sessionOption);
rootCommand.AddCommand(audioCommand);

return await rootCommand.InvokeAsync(args);
