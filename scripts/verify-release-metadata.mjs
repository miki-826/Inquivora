import { appendFileSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import process from "node:process";
import {
  extractCargoLockPackageVersion,
  extractCargoVersion,
  findReleaseReadmeMismatches,
} from "./release-lib.mjs";

const projectRoot = resolve(import.meta.dirname, "..");
const config = JSON.parse(readFileSync(resolve(projectRoot, "release.config.json"), "utf8"));
const tagPrefix = config.releaseTagPrefix ?? "v";
const tag = process.env.RELEASE_TAG;

if (!tag || !tag.startsWith(tagPrefix)) {
  throw new Error(`RELEASE_TAGは ${tagPrefix} で始まる必要があります（現在: ${tag ?? "未指定"}）`);
}

const version = tag.slice(tagPrefix.length);
const pkg = JSON.parse(readFileSync(resolve(projectRoot, "package.json"), "utf8"));
const packageLock = JSON.parse(readFileSync(resolve(projectRoot, "package-lock.json"), "utf8"));
const tauriConf = JSON.parse(
  readFileSync(resolve(projectRoot, "src-tauri", "tauri.conf.json"), "utf8"),
);
const versions = {
  tag: version,
  "package.json": pkg.version,
  "package-lock.json": packageLock.version,
  "package-lock.json packages['']": packageLock.packages?.[""]?.version,
  "tauri.conf.json": tauriConf.version,
  "Cargo.toml": extractCargoVersion(
    readFileSync(resolve(projectRoot, "src-tauri", "Cargo.toml"), "utf8"),
  ),
  "Cargo.lock": extractCargoLockPackageVersion(
    readFileSync(resolve(projectRoot, "src-tauri", "Cargo.lock"), "utf8"),
    "inquivora",
  ),
};

const mismatchedVersions = Object.entries(versions).filter(([, value]) => value !== version);
if (mismatchedVersions.length > 0) {
  throw new Error(
    `リリースバージョンが一致しません: ${Object.entries(versions)
      .map(([name, value]) => `${name}=${value}`)
      .join(" ")}`,
  );
}

const readmeMismatches = findReleaseReadmeMismatches(
  readFileSync(resolve(projectRoot, "README.md"), "utf8"),
  version,
  tagPrefix,
);
if (readmeMismatches.length > 0) {
  throw new Error(`READMEのリリース表記が一致しません: ${readmeMismatches.join(", ")}`);
}

const releaseName = `Inquivora ${tag}`;
const prerelease = version.includes("-");
console.log(`リリースメタデータ確認成功: ${releaseName} (prerelease=${prerelease})`);

if (process.env.GITHUB_OUTPUT) {
  appendFileSync(
    process.env.GITHUB_OUTPUT,
    `version=${version}\nrelease_name=${releaseName}\nprerelease=${prerelease}\n`,
    "utf8",
  );
}
