import { describe, expect, it } from "vitest";
import { DEFAULT_LAYOUT_SETTINGS, parseLayoutSettings } from "./layoutSettings";

describe("parseLayoutSettings", () => {
  it("正しい値はそのまま採用する", () => {
    const value = { leftSidebarWidth: 280, rightSidebarWidth: 400, lastScreen: "/tasks" };
    expect(parseLayoutSettings(value)).toEqual(value);
  });

  it("nullや不正値は既定値を返す", () => {
    expect(parseLayoutSettings(null)).toEqual(DEFAULT_LAYOUT_SETTINGS);
    expect(parseLayoutSettings("broken")).toEqual(DEFAULT_LAYOUT_SETTINGS);
    expect(parseLayoutSettings({ leftSidebarWidth: "wide" })).toEqual(DEFAULT_LAYOUT_SETTINGS);
  });

  it("範囲外の幅はクランプする", () => {
    const parsed = parseLayoutSettings({
      leftSidebarWidth: 10,
      rightSidebarWidth: 9999,
      lastScreen: "/meetings",
    });
    expect(parsed.leftSidebarWidth).toBe(200);
    expect(parsed.rightSidebarWidth).toBe(600);
    expect(parsed.lastScreen).toBe("/meetings");
  });

  it("未知の画面パスは既定画面へ戻す", () => {
    const parsed = parseLayoutSettings({
      leftSidebarWidth: 320,
      rightSidebarWidth: 360,
      lastScreen: "https://evil.example/",
    });
    expect(parsed.lastScreen).toBe("/workspace");
  });
});
