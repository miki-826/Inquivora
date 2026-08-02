export function normalizeRepoUrl(url) {
  let normalized = url.trim().toLowerCase();
  const sshMatch = normalized.match(/^git@([^:]+):(.+)$/);
  if (sshMatch) {
    normalized = `${sshMatch[1]}/${sshMatch[2]}`;
  } else {
    normalized = normalized.replace(/^(https?|ssh|git):\/\//, "").replace(/^git@/, "");
  }
  return normalized.replace(/\.git$/, "").replace(/\/+$/, "");
}

const FORBIDDEN_EXTENSIONS = [
  ".db",
  ".db-shm",
  ".db-wal",
  ".sqlite",
  ".sqlite3",
  ".wav",
  ".mp3",
  ".m4a",
  ".log",
  ".pfx",
  ".p12",
  ".pem",
  ".key",
  ".cer",
  ".crt",
];

const FORBIDDEN_DIRECTORIES = ["logs/", "recordings/", "credentials/", "local-data/"];

export function findForbiddenTrackedFiles(paths) {
  return paths.filter((path) => {
    const normalized = path.replace(/\\/g, "/");
    const basename = normalized.split("/").pop() ?? "";
    if (basename === ".env" || (basename.startsWith(".env.") && basename !== ".env.example")) {
      return true;
    }
    if (FORBIDDEN_EXTENSIONS.some((ext) => normalized.toLowerCase().endsWith(ext))) {
      return true;
    }
    return FORBIDDEN_DIRECTORIES.some(
      (dir) => normalized.startsWith(dir) || normalized.includes(`/${dir}`),
    );
  });
}

const SECRET_PATTERNS = [
  { name: "OpenAI APIキー", regex: /(?<![A-Za-z0-9-])sk-[A-Za-z0-9_-]{20,}/ },
  { name: "GitHubトークン", regex: /gh[pousr]_[A-Za-z0-9]{36,}/ },
  { name: "GitHub fine-grainedトークン", regex: /github_pat_[A-Za-z0-9_]{22,}/ },
  { name: "AWSアクセスキー", regex: /AKIA[0-9A-Z]{16}/ },
  { name: "Slackトークン", regex: /xox[baprs]-[A-Za-z0-9-]{10,}/ },
  { name: "秘密鍵ヘッダー", regex: /-----BEGIN [A-Z ]*PRIVATE KEY-----/ },
];

export function scanForSecretLikeStrings(text) {
  const findings = [];
  for (const { name, regex } of SECRET_PATTERNS) {
    const match = text.match(regex);
    if (match) {
      findings.push({ pattern: name, sample: `${match[0].slice(0, 8)}…` });
    }
  }
  return findings;
}

export function extractCargoVersion(toml) {
  const packageSection = toml.split(/^\[/m).find((section) => section.startsWith("package]"));
  if (!packageSection) return null;
  const match = packageSection.match(/^version\s*=\s*"([^"]+)"/m);
  return match ? match[1] : null;
}

export function extractCargoLockPackageVersion(lockfile, packageName) {
  const packageBlocks = lockfile.split(/\r?\n(?=\[\[package\]\])/);
  for (const block of packageBlocks) {
    const name = block.match(/^name\s*=\s*"([^"]+)"/m)?.[1];
    if (name !== packageName) continue;
    return block.match(/^version\s*=\s*"([^"]+)"/m)?.[1] ?? null;
  }
  return null;
}

export function findReleaseReadmeMismatches(readme, version, tagPrefix = "v") {
  const tag = `${tagPrefix}${version}`;
  const expectedValues = [
    ["公開リリース名", `Inquivora ${tag}`],
    ["リリースURL", `/releases/tag/${tag}`],
    ["インストーラーファイル名", `Inquivora_${version}_x64-setup.exe`],
  ];

  return expectedValues
    .filter(([, value]) => !readme.includes(value))
    .map(([label, value]) => `${label}: ${value}`);
}
