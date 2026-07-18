import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import process from "node:process";
import { pathToFileURL } from "node:url";
import {
  extractCargoVersion,
  findForbiddenTrackedFiles,
  normalizeRepoUrl,
  scanForSecretLikeStrings,
} from "./release-lib.mjs";

const projectRoot = resolve(import.meta.dirname, "..");
const failures = [];

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

function check(label, fn) {
  try {
    fn();
    console.log(`  ✔ ${label}`);
  } catch (error) {
    failures.push({ label, message: error.message });
    console.error(`  ✘ ${label}: ${error.message}`);
  }
}

export function runPreflight({ version = null, skipChecks = false } = {}) {
  console.log("=== Inquivora リリースPreflight ===");

  check("Windows環境である", () => {
    if (process.platform !== "win32") {
      throw new Error(`win32が必要です（現在: ${process.platform}）`);
    }
  });

  for (const [tool, args] of [
    ["node", ["--version"]],
    ["npm", ["--version"]],
    ["rustc", ["--version"]],
    ["cargo", ["--version"]],
    ["git", ["--version"]],
  ]) {
    check(`${tool} が利用可能`, () => run(tool, args));
  }

  const hasSidecar = existsSync(resolve(projectRoot, "native")) || existsSync(resolve(projectRoot, "sidecars"));
  if (hasSidecar) {
    check(".NET SDK が利用可能", () => run("dotnet", ["--list-sdks"]));
    check("Sidecarバイナリが配置済み", () => {
      const binary = resolve(
        projectRoot,
        "src-tauri",
        "binaries",
        "inquivora-native-x86_64-pc-windows-msvc.exe",
      );
      if (!existsSync(binary)) {
        throw new Error("npm run sidecar:build を実行してください");
      }
    });
  } else {
    console.log("  - Sidecar未実装のため.NET SDK検査をスキップ");
  }

  let config = null;
  check("release.config.json が存在し妥当", () => {
    const configPath = resolve(projectRoot, "release.config.json");
    if (!existsSync(configPath)) throw new Error("release.config.jsonがありません");
    config = JSON.parse(readFileSync(configPath, "utf8"));
    for (const field of ["repositoryUrl", "branch"]) {
      if (!config[field]) throw new Error(`${field} が未設定です`);
    }
  });

  check("release remote がrelease.config.jsonと一致", () => {
    const remoteUrl = run("git", ["remote", "get-url", "release"]);
    if (normalizeRepoUrl(remoteUrl) !== normalizeRepoUrl(config.repositoryUrl)) {
      throw new Error(`remote(${remoteUrl}) と設定(${config.repositoryUrl}) が不一致`);
    }
  });

  check("現在ブランチが設定ブランチと一致", () => {
    const branch = run("git", ["rev-parse", "--abbrev-ref", "HEAD"]);
    if (branch !== config.branch) {
      throw new Error(`現在 ${branch}、設定は ${config.branch}`);
    }
  });

  check("作業ツリーがcleanである", () => {
    const status = run("git", ["status", "--porcelain"]);
    if (status) throw new Error(`未コミットの変更があります:\n${status}`);
  });

  let projectVersion = null;
  check("package.json / tauri.conf.json / Cargo.toml のバージョンが一致", () => {
    const pkg = JSON.parse(readFileSync(resolve(projectRoot, "package.json"), "utf8"));
    const tauriConf = JSON.parse(
      readFileSync(resolve(projectRoot, "src-tauri", "tauri.conf.json"), "utf8"),
    );
    const cargoVersion = extractCargoVersion(
      readFileSync(resolve(projectRoot, "src-tauri", "Cargo.toml"), "utf8"),
    );
    if (pkg.version !== tauriConf.version || pkg.version !== cargoVersion) {
      throw new Error(
        `package.json=${pkg.version} tauri.conf.json=${tauriConf.version} Cargo.toml=${cargoVersion}`,
      );
    }
    projectVersion = pkg.version;
    if (version && version !== projectVersion) {
      throw new Error(`指定バージョン ${version} とプロジェクト ${projectVersion} が不一致`);
    }
  });

  if (version) {
    const tag = `${config?.releaseTagPrefix ?? "v"}${version}`;
    check(`タグ ${tag} が未使用である`, () => {
      const local = run("git", ["tag", "--list", tag]);
      if (local) throw new Error(`ローカルに ${tag} が存在します`);
      const remote = run("git", ["ls-remote", "release", `refs/tags/${tag}`]);
      if (remote) throw new Error(`GitHubに ${tag} が存在します`);
    });
  }

  let trackedFiles = [];
  check("禁止ファイルがGit追跡対象に含まれない", () => {
    trackedFiles = run("git", ["ls-files"]).split("\n").filter(Boolean);
    const forbidden = findForbiddenTrackedFiles(trackedFiles);
    if (forbidden.length > 0) {
      throw new Error(`禁止ファイル: ${forbidden.join(", ")}`);
    }
  });

  check("ソース内にAPIキーらしい文字列がない", () => {
    const textExtensions = /\.(ts|tsx|js|jsx|mjs|rs|cs|json|toml|yml|yaml|md|sql|html|css|ps1|sh|bat|env\.example)$/i;
    const findings = [];
    for (const file of trackedFiles) {
      if (!textExtensions.test(file)) continue;
      const content = readFileSync(resolve(projectRoot, file), "utf8");
      for (const finding of scanForSecretLikeStrings(content)) {
        findings.push(`${file}: ${finding.pattern} (${finding.sample})`);
      }
    }
    if (findings.length > 0) {
      throw new Error(findings.join("; "));
    }
  });

  if (!skipChecks) {
    const stepOptions = { stdio: "inherit" };
    check("lint", () => run("npm", ["run", "lint"], stepOptions));
    check("typecheck", () => run("npm", ["run", "typecheck"], stepOptions));
    check("frontend test", () => run("npm", ["test"], stepOptions));
    check("Rust test", () => run("npm", ["run", "test:rust"], stepOptions));
    if (hasSidecar) {
      check(".NET test", () => run("npm", ["run", "test:dotnet"], stepOptions));
    }
  }

  if (failures.length > 0) {
    console.error(`\nPreflight失敗: ${failures.length}件`);
    process.exit(1);
  }
  console.log("\nPreflight成功");
  return { config, version: projectVersion };
}

if (process.argv[1] && pathToFileURL(process.argv[1]).href === import.meta.url) {
  const versionArg = process.argv.find((_, i) => process.argv[i - 1] === "--version");
  runPreflight({ version: versionArg ?? null });
}
