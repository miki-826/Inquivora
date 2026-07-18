import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync, readdirSync } from "node:fs";
import { resolve } from "node:path";

const projectRoot = resolve(import.meta.dirname, "..");
const publishDir = resolve(
  projectRoot,
  "native/Inquivora.Native/bin/Release/net8.0-windows10.0.19041.0/win-x64/publish",
);

execFileSync(
  "dotnet",
  [
    "publish",
    "native/Inquivora.Native/Inquivora.Native.csproj",
    "-c",
    "Release",
    "-r",
    "win-x64",
    "--self-contained",
    "true",
    "-p:PublishSingleFile=true",
  ],
  { cwd: projectRoot, stdio: "inherit" },
);

const binariesDir = resolve(projectRoot, "src-tauri", "binaries");
mkdirSync(binariesDir, { recursive: true });
copyFileSync(
  resolve(publishDir, "inquivora-native.exe"),
  resolve(binariesDir, "inquivora-native-x86_64-pc-windows-msvc.exe"),
);

// Whisper.netのネイティブDLLは単一ファイルへ埋め込めないため、
// Sidecarの実行ファイル横（runtimes/win-x64）へリソースとして同梱する
const whisperRuntimeSrc = resolve(publishDir, "runtimes", "win-x64");
const whisperRuntimeDest = resolve(binariesDir, "runtimes", "win-x64");
mkdirSync(whisperRuntimeDest, { recursive: true });
for (const file of readdirSync(whisperRuntimeSrc)) {
  if (file.endsWith(".dll")) {
    copyFileSync(resolve(whisperRuntimeSrc, file), resolve(whisperRuntimeDest, file));
  }
}
console.log("Sidecarを src-tauri/binaries/inquivora-native-x86_64-pc-windows-msvc.exe へ配置しました");
