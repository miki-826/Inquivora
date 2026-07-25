import { execFileSync } from "node:child_process";
import process from "node:process";

// このアプリはサーバーを持たないTauriデスクトップSPAで、React RouterのRSC（React Server Components）
// モードは使用していない。GHSA-qwww-vcr4-c8h2（RSC Mode CSRF Bypass）はRSCモード利用時のみ影響するため
// 監査ゲートから除外する。react-router-dom v7 は 7.12.0未満に別の脆弱性（XSS/DoS等）があり、
// 現行の 7.18.1 が最も修正の進んだ版のため据え置く。
const ALLOWLIST = new Set(["GHSA-qwww-vcr4-c8h2"]);
const LEVELS = ["info", "low", "moderate", "high", "critical"];
const MIN_LEVEL = "moderate";

function runAuditJson() {
  const options = {
    encoding: "utf8",
    shell: process.platform === "win32",
    maxBuffer: 32 * 1024 * 1024,
  };
  try {
    return JSON.parse(execFileSync("npm", ["audit", "--omit=dev", "--json"], options));
  } catch (error) {
    // npm audit は脆弱性検出時に非0で終了するが、JSONは stdout に出力される。
    if (error.stdout) return JSON.parse(error.stdout);
    throw error;
  }
}

const report = runAuditJson();
const minIndex = LEVELS.indexOf(MIN_LEVEL);
const blocking = new Set();

for (const [name, vuln] of Object.entries(report.vulnerabilities ?? {})) {
  if (LEVELS.indexOf(vuln.severity) < minIndex) continue;
  for (const via of vuln.via ?? []) {
    if (typeof via !== "object") continue; // 文字列は他パッケージへの参照なので実体側で評価する
    const ghsa = (via.url ?? "").split("/").pop() ?? "";
    if (ALLOWLIST.has(ghsa)) continue;
    blocking.add(`${name}: ${via.title ?? "?"} (${ghsa || via.url || "?"}) [${via.severity ?? vuln.severity}]`);
  }
}

if (blocking.size > 0) {
  console.error("npm audit: 許可リスト外の脆弱性が検出されました:");
  for (const line of blocking) console.error(`  - ${line}`);
  process.exit(1);
}

console.log(`npm audit: moderate以上の未許可脆弱性なし（許可: ${[...ALLOWLIST].join(", ") || "なし"}）`);
