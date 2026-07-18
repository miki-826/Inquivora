import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync } from "node:fs";
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
console.log("Sidecarを src-tauri/binaries/inquivora-native-x86_64-pc-windows-msvc.exe へ配置しました");
