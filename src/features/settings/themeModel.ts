export const THEME_SETTING_KEY = "ui.theme";

export type ThemePreference = "light" | "dark" | "system";
export type ResolvedTheme = "light" | "dark";

export const THEME_OPTIONS: { value: ThemePreference; label: string }[] = [
  { value: "light", label: "ライト" },
  { value: "dark", label: "ダーク" },
  { value: "system", label: "OSに合わせる" },
];

export function parseThemePreference(value: unknown): ThemePreference {
  return value === "light" || value === "dark" || value === "system" ? value : "system";
}

/// 設定値とOSのダーク設定から、実際に適用するテーマを決める。
export function resolveTheme(pref: ThemePreference, systemPrefersDark: boolean): ResolvedTheme {
  if (pref === "system") {
    return systemPrefersDark ? "dark" : "light";
  }
  return pref;
}

export function themeLabel(pref: ThemePreference): string {
  return THEME_OPTIONS.find((o) => o.value === pref)?.label ?? "OSに合わせる";
}
