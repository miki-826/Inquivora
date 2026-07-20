import { create } from "zustand";
import {
  parseThemePreference,
  resolveTheme,
  THEME_SETTING_KEY,
  type ThemePreference,
} from "../features/settings/themeModel";
import { loadSetting, saveSetting } from "../services/settings";

type ThemeStore = {
  preference: ThemePreference;
  init: () => Promise<void>;
  setPreference: (pref: ThemePreference) => void;
};

const media = typeof window !== "undefined" ? window.matchMedia("(prefers-color-scheme: dark)") : null;

function apply(pref: ThemePreference): void {
  const resolved = resolveTheme(pref, media?.matches ?? false);
  document.documentElement.setAttribute("data-theme", resolved);
}

export const useThemeStore = create<ThemeStore>((set, get) => ({
  preference: "system",
  init: async () => {
    const stored = await loadSetting(THEME_SETTING_KEY).catch(() => null);
    const pref = parseThemePreference(stored);
    set({ preference: pref });
    apply(pref);
    // OS設定変更へ追従（preference が system のときのみ見た目が変わる）
    media?.addEventListener("change", () => {
      if (get().preference === "system") {
        apply("system");
      }
    });
  },
  setPreference: (pref) => {
    set({ preference: pref });
    apply(pref);
    void saveSetting(THEME_SETTING_KEY, pref).catch((err) =>
      console.error("テーマ設定の保存に失敗しました", err),
    );
  },
}));
