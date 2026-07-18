using System.CommandLine;
using Inquivora.Native.Audio;
using Inquivora.Native.Credentials;
using Inquivora.Native.Notifications;
using Inquivora.Native.Whisper;

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

var whisperCommand = new Command("whisper", "stdinのNDJSONコマンドでローカルWhisper文字起こしを行う");
whisperCommand.SetHandler(() =>
{
    Environment.ExitCode = WhisperMode.Run(Console.In, Console.Out, Console.Error);
});
rootCommand.AddCommand(whisperCommand);

return await rootCommand.InvokeAsync(args);
