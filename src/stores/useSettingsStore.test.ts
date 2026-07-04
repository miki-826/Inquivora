import { describe, expect, it } from "vitest";
import { clampSidebarWidth, SIDEBAR_WIDTH_MAX, SIDEBAR_WIDTH_MIN } from "./useSettingsStore";

describe("clampSidebarWidth", () => {
  it("範囲内の値はそのまま返す", () => {
    expect(clampSidebarWidth(320)).toBe(320);
  });

  it("最小値未満は最小値へ丸める", () => {
    expect(clampSidebarWidth(50)).toBe(SIDEBAR_WIDTH_MIN);
  });

  it("最大値超過は最大値へ丸める", () => {
    expect(clampSidebarWidth(9999)).toBe(SIDEBAR_WIDTH_MAX);
  });

  it("小数は整数へ丸める", () => {
    expect(clampSidebarWidth(320.6)).toBe(321);
  });
});
