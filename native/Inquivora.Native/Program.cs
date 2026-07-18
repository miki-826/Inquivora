using System.CommandLine;
using System.Text;
using Inquivora.Native.Audio;
using Inquivora.Native.Credentials;
using Inquivora.Native.Notifications;
using Inquivora.Native.Whisper;

// stdin/stdoutはRust側とUTF-8のNDJSONで交換する。Console既定はOSコードページ
// （日本語環境ではShift-JIS）のため、明示的にUTF-8ストリームを構築する
var utf8 = new UTF8Encoding(false);
using var stdin = new StreamReader(Console.OpenStandardInput(), utf8);
using var stdout = new StreamWriter(Console.OpenStandardOutput(), utf8) { AutoFlush = true };
using var stderr = new StreamWriter(Console.OpenStandardError(), utf8) { AutoFlush = true };

var rootCommand = new RootCommand("Inquivora native sidecar");

var notifyCommand = new Command("notify", "stdinのNDJSONコマンドでWindows通知を表示する");
notifyCommand.SetHandler(() =>
{
    Environment.ExitCode = NotifyMode.Run(stdin, stdout, stderr);
});
rootCommand.AddCommand(notifyCommand);

var credentialCommand = new Command("credential", "stdinのNDJSONコマンドでWindows Credential Managerを操作する");
credentialCommand.SetHandler(() =>
{
    Environment.ExitCode = CredentialMode.Run(stdin, stdout, stderr);
});
rootCommand.AddCommand(credentialCommand);

var sessionOption = new Option<string>("--session", "録音セッションID") { IsRequired = true };
var audioCommand = new Command("audio", "stdinの制御コマンドでマイク・ループバック録音を行う");
audioCommand.AddOption(sessionOption);
audioCommand.SetHandler(session =>
{
    Environment.ExitCode = AudioMode.Run(session, stdin, stdout, stderr);
}, sessionOption);
rootCommand.AddCommand(audioCommand);

var whisperCommand = new Command("whisper", "stdinのNDJSONコマンドでローカルWhisper文字起こしを行う");
whisperCommand.SetHandler(() =>
{
    Environment.ExitCode = WhisperMode.Run(stdin, stdout, stderr);
});
rootCommand.AddCommand(whisperCommand);

return await rootCommand.InvokeAsync(args);
