import { create } from "zustand";

export const SIDEBAR_WIDTH_MIN = 200;
export const SIDEBAR_WIDTH_MAX = 600;

export function clampSidebarWidth(width: number): number {
  return Math.min(SIDEBAR_WIDTH_MAX, Math.max(SIDEBAR_WIDTH_MIN, Math.round(width)));
}

type SettingsState = {
  leftSidebarWidth: number;
  rightSidebarWidth: number;
  lastScreen: string;
  setLeftSidebarWidth: (width: number) => void;
  setRightSidebarWidth: (width: number) => void;
  setLastScreen: (path: string) => void;
};

export const useSettingsStore = create<SettingsState>((set) => ({
  leftSidebarWidth: 320,
  rightSidebarWidth: 360,
  lastScreen: "/workspace",
  setLeftSidebarWidth: (width) => set({ leftSidebarWidth: clampSidebarWidth(width) }),
  setRightSidebarWidth: (width) => set({ rightSidebarWidth: clampSidebarWidth(width) }),
  setLastScreen: (path) => set({ lastScreen: path }),
}));
