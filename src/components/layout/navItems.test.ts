import { describe, expect, it } from "vitest";
import { NAV_ITEMS } from "./navItems";

describe("NAV_ITEMS", () => {
  it("仕様書§5.2の6画面を定義する", () => {
    expect(NAV_ITEMS.map((item) => item.id)).toEqual([
      "workspace",
      "search",
      "meetings",
      "tasks",
      "calendar",
      "settings",
    ]);
  });

  it("仕様書§5.3の画面ルートと一致する", () => {
    expect(NAV_ITEMS.map((item) => item.path)).toEqual([
      "/workspace",
      "/search",
      "/meetings",
      "/tasks",
      "/calendar",
      "/settings",
    ]);
  });

  it("すべての項目に日本語ラベルとアイコンがある", () => {
    for (const item of NAV_ITEMS) {
      expect(item.label.length).toBeGreaterThan(0);
      expect(item.icon).toBeDefined();
    }
  });
});
