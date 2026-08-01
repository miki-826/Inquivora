import { execFileSync } from "node:child_process";
import {
  existsSync,
  readFileSync,
  readdirSync,
  writeFileSync,
} from "node:fs";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(fileURLToPath(new URL("..", import.meta.url)));
const utf8 = { cwd: root, encoding: "utf8", maxBuffer: 64 * 1024 * 1024 };
const noticeName = /^(license|licence|copying|notice|third[-_. ]party)/i;

function run(command, args) {
  if (process.platform === "win32" && command === "npm") {
    const npmCli = join(dirname(process.execPath), "node_modules", "npm", "bin", "npm-cli.js");
    return execFileSync(process.execPath, [npmCli, ...args], utf8);
  }
  return execFileSync(command, args, utf8);
}

function clean(text) {
  return text
    .replace(/^\uFEFF/, "")
    .replaceAll("\r\n", "\n")
    .split("\n")
    .map((line) => line.trimEnd())
    .join("\n")
    .trim();
}

function noticeFiles(directory) {
  if (!existsSync(directory)) return [];
  return readdirSync(directory, { withFileTypes: true })
    .filter((entry) => entry.isFile() && noticeName.test(entry.name))
    .map((entry) => join(directory, entry.name))
    .sort((a, b) => a.localeCompare(b));
}

function sourceOf(pkg) {
  if (typeof pkg.repository === "string") return pkg.repository;
  return pkg.repository?.url || pkg.homepage || pkg.resolved || "(not declared)";
}

function generateNodeNotices() {
  const packages = JSON.parse(run("npm", ["query", ":not(.dev)", "--json"]))
    .filter((pkg) => pkg.location && !pkg.dev)
    .filter((pkg, index, all) => all.findIndex((item) => item.pkgid === pkg.pkgid) === index)
    .sort((a, b) => a.pkgid.localeCompare(b.pkgid));

  const blocks = packages.map((pkg) => {
    const files = noticeFiles(pkg.path);
    const texts = files.length
      ? files.map((file) => `--- ${basename(file)} ---\n${clean(readFileSync(file, "utf8"))}`).join("\n\n")
      : "No standalone license file was present in the installed package; see the declared SPDX expression above.";
    return [
      "================================================================================",
      pkg.pkgid,
      `Declared license: ${pkg.license || "not declared"}`,
      `Source: ${sourceOf(pkg)}`,
      "",
      texts,
    ].join("\n");
  });

  const output = [
    "Third-Party Licenses - Node.js production dependencies",
    "Generated from package-lock.json and installed production dependency metadata.",
    `Packages: ${packages.length}`,
    "",
    ...blocks,
    "",
  ].join("\n");
  writeFileSync(join(root, "THIRD_PARTY_LICENSES_NODE.txt"), output, "utf8");
}

const mitText = `Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.`;

function decodeXml(text) {
  return text
    .replaceAll("&amp;", "&")
    .replaceAll("&lt;", "<")
    .replaceAll("&gt;", ">")
    .replaceAll("&quot;", '"')
    .replaceAll("&apos;", "'");
}

function tag(xml, name) {
  return decodeXml(xml.match(new RegExp(`<${name}(?:\\s[^>]*)?>([\\s\\S]*?)</${name}>`, "i"))?.[1]?.trim() || "");
}

function generateDotnetNotices() {
  const project = "native/Inquivora.Native/Inquivora.Native.csproj";
  const graph = JSON.parse(run("dotnet", ["list", project, "package", "--include-transitive", "--format", "json"]));
  const framework = graph.projects[0].frameworks[0];
  const packages = [...framework.topLevelPackages, ...(framework.transitivePackages || [])]
    .sort((a, b) => a.id.localeCompare(b.id));
  const nugetRoot = process.env.NUGET_PACKAGES || join(process.env.USERPROFILE, ".nuget", "packages");

  const packageBlocks = packages.map((pkg) => {
    const directory = join(nugetRoot, pkg.id.toLowerCase(), pkg.resolvedVersion);
    const nuspecPath = readdirSync(directory).find((name) => name.endsWith(".nuspec"));
    const nuspec = nuspecPath ? readFileSync(join(directory, nuspecPath), "utf8") : "";
    const files = noticeFiles(directory);
    const texts = files.length
      ? files.map((file) => `--- ${basename(file)} ---\n${clean(readFileSync(file, "utf8"))}`).join("\n\n")
      : "The package did not contain a standalone license file. The applicable license text is reproduced below where required.";
    return [
      "================================================================================",
      `${pkg.id} ${pkg.resolvedVersion}`,
      `Authors: ${tag(nuspec, "authors") || "not declared"}`,
      `Declared license: ${tag(nuspec, "license") || tag(nuspec, "licenseUrl") || "not declared"}`,
      `Project URL: ${tag(nuspec, "projectUrl") || "not declared"}`,
      "",
      texts,
    ].join("\n");
  });

  const runtimeBase = join(nugetRoot, "microsoft.netcore.app.runtime.win-x64");
  const runtimeVersions = readdirSync(runtimeBase, { withFileTypes: true })
    .filter((entry) => entry.isDirectory() && entry.name.startsWith("8."))
    .map((entry) => entry.name)
    .sort((a, b) => b.localeCompare(a, undefined, { numeric: true }));
  if (!runtimeVersions.length) throw new Error("Microsoft.NETCore.App.Runtime.win-x64 8.x was not found.");
  const runtimeVersion = runtimeVersions[0];
  const runtimeDirectory = join(runtimeBase, runtimeVersion);

  const nsisCopying = join(process.env.LOCALAPPDATA, "tauri", "NSIS", "COPYING");
  if (!existsSync(nsisCopying)) throw new Error(`NSIS notice was not found: ${nsisCopying}`);

  const supplemental = [
    "================================================================================",
    "NAudio.Core / NAudio.Wasapi 2.2.1",
    "MIT License - Copyright 2020 Mark Heath",
    "Source: https://github.com/naudio/NAudio/tree/v2.2.1",
    "",
    mitText,
    "",
    "================================================================================",
    "System.CommandLine 2.0.0-beta4.22272.1",
    "MIT License - Copyright (c) .NET Foundation and Contributors. All rights reserved.",
    "Source: https://github.com/dotnet/command-line-api/tree/209b724a3c843253d3071e8348c353b297b0b8b5",
    "",
    mitText,
    "",
    "================================================================================",
    "Whisper.net / Whisper.net.Runtime 1.7.4",
    "MIT License - Copyright (c) 2024 sandrohanea",
    "Source: https://github.com/sandrohanea/whisper.net/tree/1.7.4",
    "",
    mitText,
    "",
    "================================================================================",
    "whisper.cpp (native libraries carried by Whisper.net.Runtime 1.7.4)",
    "MIT License - Copyright (c) 2023-2024 The ggml authors",
    "Source revision: https://github.com/ggml-org/whisper.cpp/tree/3de9deead5759eb038966990e3cb5d83984ae467",
    "",
    mitText,
    "",
    "================================================================================",
    `Microsoft.NETCore.App.Runtime.win-x64 ${runtimeVersion}`,
    `NuGet package: Microsoft.NETCore.App.Runtime.win-x64 ${runtimeVersion}`,
    "",
    "--- LICENSE.TXT ---",
    clean(readFileSync(join(runtimeDirectory, "LICENSE.TXT"), "utf8")),
    "",
    "--- THIRD-PARTY-NOTICES.TXT ---",
    clean(readFileSync(join(runtimeDirectory, "THIRD-PARTY-NOTICES.TXT"), "utf8")),
    "",
    "================================================================================",
    "NSIS (installer tooling; applicable notices)",
    "Source: https://nsis.sourceforge.io/Docs/AppendixI.html",
    "",
    clean(readFileSync(nsisCopying, "utf8")),
  ].join("\n");

  const output = [
    "Third-Party Licenses - .NET sidecar and Windows installer",
    "Generated from the resolved NuGet graph, .NET runtime pack, and NSIS distribution.",
    `NuGet packages: ${packages.length}`,
    "",
    ...packageBlocks,
    "",
    supplemental,
    "",
  ].join("\n");
  writeFileSync(join(root, "THIRD_PARTY_LICENSES_DOTNET.txt"), output, "utf8");
}

function generateRustNotices() {
  const outputPath = join(root, "THIRD_PARTY_LICENSES_RUST.txt");
  run("cargo", [
    "about",
    "generate",
    "--manifest-path",
    "src-tauri/Cargo.toml",
    "--locked",
    "--config",
    "src-tauri/about.toml",
    "--output-file",
    outputPath,
    "src-tauri/about.hbs",
  ]);
  writeFileSync(outputPath, `${clean(readFileSync(outputPath, "utf8"))}\n`, "utf8");
}

generateNodeNotices();
generateDotnetNotices();
generateRustNotices();
console.log("Generated Node.js, Rust, and .NET/installer third-party license reports.");
