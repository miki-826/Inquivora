import { describe, expect, it } from "vitest";
import { parseThemePreference, resolveTheme, themeLabel } from "./themeModel";

describe("parseThemePreference", () => {
  it("有効な値はそのまま、無効はsystem", () => {
    expect(parseThemePreference("light")).toBe("light");
    expect(parseThemePreference("dark")).toBe("dark");
    expect(parseThemePreference("system")).toBe("system");
    expect(parseThemePreference("xxx")).toBe("system");
    expect(parseThemePreference(null)).toBe("system");
  });
});

describe("resolveTheme", () => {
  it("明示指定はOS設定に関わらず優先する", () => {
    expect(resolveTheme("dark", false)).toBe("dark");
    expect(resolveTheme("light", true)).toBe("light");
  });

  it("systemはOSのダーク設定に従う", () => {
    expect(resolveTheme("system", true)).toBe("dark");
    expect(resolveTheme("system", false)).toBe("light");
  });
});

describe("themeLabel", () => {
  it("日本語ラベルを返す", () => {
    expect(themeLabel("dark")).toBe("ダーク");
    expect(themeLabel("system")).toBe("OSに合わせる");
  });
});
