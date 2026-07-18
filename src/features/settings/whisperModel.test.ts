import { describe, expect, it } from "vitest";
import {
  downloadPercent,
  formatModelSize,
  parseWhisperStatus,
} from "./whisperModel";

describe("parseWhisperStatus", () => {
  it("モデル状態の配列を検証して返す", () => {
    const parsed = parseWhisperStatus([
      { name: "small", displayName: "Small（推奨・日本語向け）", sizeMb: 466, downloaded: true, selected: true },
      { name: "tiny", displayName: "Tiny（最小・低精度）", sizeMb: 75, downloaded: false, selected: false },
    ]);
    expect(parsed).toHaveLength(2);
    expect(parsed[0].name).toBe("small");
    expect(parsed[0].downloaded).toBe(true);
  });

  it("不正な要素は取り除く", () => {
    const parsed = parseWhisperStatus([
      { name: "small", displayName: "Small", sizeMb: 466, downloaded: true, selected: true },
      { name: 123 },
      "invalid",
    ]);
    expect(parsed).toHaveLength(1);
  });

  it("配列でなければ空配列", () => {
    expect(parseWhisperStatus(null)).toEqual([]);
    expect(parseWhisperStatus({})).toEqual([]);
  });
});

describe("formatModelSize", () => {
  it("MBはそのまま、1000MB以上はGB表記", () => {
    expect(formatModelSize(466)).toBe("約466MB");
    expect(formatModelSize(75)).toBe("約75MB");
    expect(formatModelSize(1500)).toBe("約1.5GB");
  });
});

describe("downloadPercent", () => {
  it("受信済みバイトから百分率を計算する", () => {
    expect(downloadPercent(50, 200)).toBe(25);
    expect(downloadPercent(200, 200)).toBe(100);
  });

  it("合計不明・0のときはnull", () => {
    expect(downloadPercent(50, null)).toBeNull();
    expect(downloadPercent(50, 0)).toBeNull();
  });

  it("100を超えない", () => {
    expect(downloadPercent(300, 200)).toBe(100);
  });
});
