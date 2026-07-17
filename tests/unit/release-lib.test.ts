import { describe, expect, it } from "vitest";
import {
  extractCargoVersion,
  findForbiddenTrackedFiles,
  normalizeRepoUrl,
  scanForSecretLikeStrings,
} from "../../scripts/release-lib.mjs";

describe("normalizeRepoUrl", () => {
  it("SSH形式とHTTPS形式を同一視できる", () => {
    expect(normalizeRepoUrl("git@github.com:miki-826/Inquivora.git")).toBe(
      normalizeRepoUrl("https://github.com/miki-826/Inquivora.git"),
    );
  });

  it("末尾.gitの有無を同一視できる", () => {
    expect(normalizeRepoUrl("https://github.com/miki-826/Inquivora")).toBe(
      normalizeRepoUrl("https://github.com/miki-826/Inquivora.git"),
    );
  });

  it("別リポジトリは一致しない", () => {
    expect(normalizeRepoUrl("https://github.com/other/Inquivora.git")).not.toBe(
      normalizeRepoUrl("https://github.com/miki-826/Inquivora.git"),
    );
  });
});

describe("findForbiddenTrackedFiles", () => {
  it("秘密情報・ローカルデータのファイルを検出する", () => {
    const files = [
      ".env",
      ".env.production",
      "data/app.db",
      "recordings/meeting.wav",
      "logs/app.log",
      "cert/sign.pfx",
      "keys/private.pem",
    ];
    const found = findForbiddenTrackedFiles(files);
    expect(found).toEqual(files);
  });

  it("許可されたファイルは検出しない", () => {
    const files = [
      ".env.example",
      "src/main.tsx",
      "src-tauri/migrations/001_init.sql",
      "docs/README.md",
      "package-lock.json",
    ];
    expect(findForbiddenTrackedFiles(files)).toEqual([]);
  });
});

describe("scanForSecretLikeStrings", () => {
  it("既知形式のAPIキーらしい文字列を検出する", () => {
    const fakeOpenAi = "sk-" + "a1B2".repeat(8);
    const fakeGitHub = "ghp_" + "x".repeat(36);
    const fakeAws = "AKIA" + "ABCDEFGHIJKLMNOP";
    const pemHeader = ["-----BEGIN", "PRIVATE KEY-----"].join(" ");
    expect(scanForSecretLikeStrings(`key=${fakeOpenAi}`).length).toBeGreaterThan(0);
    expect(scanForSecretLikeStrings(`token: ${fakeGitHub}`).length).toBeGreaterThan(0);
    expect(scanForSecretLikeStrings(`id = ${fakeAws}`).length).toBeGreaterThan(0);
    expect(scanForSecretLikeStrings(pemHeader).length).toBeGreaterThan(0);
  });

  it("通常のコードは検出しない", () => {
    const source = 'const apiKey = await invoke("api_provider_has_secret", { id });';
    expect(scanForSecretLikeStrings(source)).toEqual([]);
  });

  it("単語の途中にsk-を含むCSSクラス名は検出しない", () => {
    const css = ".task-filter__preset--active { color: red; }";
    const tsx = 'className="task-filter__preset task-filter__preset--active"';
    expect(scanForSecretLikeStrings(css)).toEqual([]);
    expect(scanForSecretLikeStrings(tsx)).toEqual([]);
  });
});

describe("extractCargoVersion", () => {
  it("[package]セクションのversionを取得する", () => {
    const toml = '[package]\nname = "inquivora"\nversion = "0.1.0"\n\n[dependencies]\nserde = { version = "1" }\n';
    expect(extractCargoVersion(toml)).toBe("0.1.0");
  });

  it("versionが見つからない場合はnullを返す", () => {
    expect(extractCargoVersion("[dependencies]\nserde = \"1\"\n")).toBeNull();
  });
});
