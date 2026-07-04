import { execFileSync } from "node:child_process";
import { resolve } from "node:path";
import process from "node:process";
import { runPreflight } from "./release-preflight.mjs";

const projectRoot = resolve(import.meta.dirname, "..");

function run(command, args, options = {}) {
  const result = execFileSync(command, args, {
    cwd: projectRoot,
    encoding: "utf8",
    shell: process.platform === "win32" && command === "npm",
    stdio: ["ignore", "pipe", "pipe"],
    ...options,
  });
  return typeof result === "string" ? result.trim() : "";
}

function fail(message) {
  console.error(`\nリリース中止: ${message}`);
  process.exit(1);
}

const versionArg = process.argv.find((_, i) => process.argv[i - 1] === "--version");
if (!versionArg) {
  fail("使用方法: npm run release:windows -- --version 0.1.0");
}

console.log(`=== Inquivora Windows正式リリース v${versionArg} ===\n`);

const { config } = runPreflight({ version: versionArg });
const branch = config.branch;
const tag = `${config.releaseTagPrefix ?? "v"}${versionArg}`;

console.log(`\n--- GitHubへ ${branch} をpush ---`);
run("git", ["push", "release", `HEAD:${branch}`], { stdio: "inherit" });

console.log("--- remote SHAとlocal HEADの一致を検証 ---");
const localSha = run("git", ["rev-parse", "HEAD"]);
const remoteRef = run("git", ["ls-remote", "release", `refs/heads/${branch}`]);
const remoteSha = remoteRef.split(/\s+/)[0] ?? "";
if (!remoteSha || localSha !== remoteSha) {
  fail(`SHA不一致のためタグ作成とビルドを中止します (local=${localSha} remote=${remoteSha})`);
}
console.log(`  ✔ SHA一致: ${localSha}`);

console.log(`--- annotated tag ${tag} を作成・push ---`);
run("git", ["tag", "-a", tag, "-m", `Inquivora ${tag}`], { stdio: "inherit" });
run("git", ["push", "release", tag], { stdio: "inherit" });

console.log(`\n完了: GitHub Actionsが ${tag} (${localSha.slice(0, 7)}) からWindowsインストーラーを生成します。`);
console.log("進行状況: https://github.com/miki-826/Inquivora/actions");
