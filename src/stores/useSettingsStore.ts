import { create } from "zustand";
import {
  clampSidebarWidth,
  DEFAULT_LAYOUT_SETTINGS,
  parseLayoutSettings,
  SIDEBAR_WIDTH_MAX,
  SIDEBAR_WIDTH_MIN,
  type NavigationPosition,
  type TaskListFontSize,
} from "../schemas/layoutSettings";
import { loadSetting, saveSetting } from "../services/settings";

export { clampSidebarWidth, SIDEBAR_WIDTH_MAX, SIDEBAR_WIDTH_MIN };

const LAYOUT_SETTING_KEY = "ui.layout";
const PERSIST_DEBOUNCE_MS = 500;

type SettingsState = {
  leftSidebarWidth: number;
  rightSidebarWidth: number;
  lastScreen: string;
  navigationPosition: NavigationPosition;
  taskListFontSize: TaskListFontSize;
  hydrated: boolean;
  hydrate: () => Promise<void>;
  setLeftSidebarWidth: (width: number) => void;
  setRightSidebarWidth: (width: number) => void;
  setLastScreen: (path: string) => void;
  setNavigationPosition: (position: NavigationPosition) => void;
  setTaskListFontSize: (size: TaskListFontSize) => void;
};

let persistTimer: ReturnType<typeof setTimeout> | undefined;

function schedulePersist(get: () => SettingsState) {
  clearTimeout(persistTimer);
  persistTimer = setTimeout(() => {
    const { leftSidebarWidth, rightSidebarWidth, lastScreen, navigationPosition, taskListFontSize } = get();
    saveSetting(LAYOUT_SETTING_KEY, {
      leftSidebarWidth,
      rightSidebarWidth,
      lastScreen,
      navigationPosition,
      taskListFontSize,
    }).catch((error) => console.error("レイアウト設定の保存に失敗しました", error));
  }, PERSIST_DEBOUNCE_MS);
}

export const useSettingsStore = create<SettingsState>((set, get) => ({
  ...DEFAULT_LAYOUT_SETTINGS,
  hydrated: false,
  hydrate: async () => {
    try {
      const stored = await loadSetting(LAYOUT_SETTING_KEY);
      set({ ...parseLayoutSettings(stored), hydrated: true });
    } catch (error) {
      console.error("レイアウト設定の読み込みに失敗しました", error);
      set({ hydrated: true });
    }
  },
  setLeftSidebarWidth: (width) => {
    set({ leftSidebarWidth: clampSidebarWidth(width) });
    schedulePersist(get);
  },
  setRightSidebarWidth: (width) => {
    set({ rightSidebarWidth: clampSidebarWidth(width) });
    schedulePersist(get);
  },
  setLastScreen: (path) => {
    set({ lastScreen: path });
    schedulePersist(get);
  },
  setNavigationPosition: (navigationPosition) => {
    set({ navigationPosition });
    schedulePersist(get);
  },
  setTaskListFontSize: (taskListFontSize) => {
    set({ taskListFontSize });
    schedulePersist(get);
  },
}));
