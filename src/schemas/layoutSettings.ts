import { z } from "zod";

export const SIDEBAR_WIDTH_MIN = 200;
export const SIDEBAR_WIDTH_MAX = 600;

export function clampSidebarWidth(width: number): number {
  return Math.min(SIDEBAR_WIDTH_MAX, Math.max(SIDEBAR_WIDTH_MIN, Math.round(width)));
}

export const SCREEN_PATHS = [
  "/workspace",
  "/search",
  "/meetings",
  "/tasks",
  "/calendar",
  "/settings",
] as const;

export type LayoutSettings = {
  leftSidebarWidth: number;
  rightSidebarWidth: number;
  lastScreen: string;
};

export const DEFAULT_LAYOUT_SETTINGS: LayoutSettings = {
  leftSidebarWidth: 320,
  rightSidebarWidth: 360,
  lastScreen: "/workspace",
};

const layoutSettingsSchema = z.object({
  leftSidebarWidth: z.number(),
  rightSidebarWidth: z.number(),
  lastScreen: z.string(),
});

export function parseLayoutSettings(value: unknown): LayoutSettings {
  const result = layoutSettingsSchema.safeParse(value);
  if (!result.success) {
    return DEFAULT_LAYOUT_SETTINGS;
  }
  const isKnownScreen = (SCREEN_PATHS as readonly string[]).includes(result.data.lastScreen);
  return {
    leftSidebarWidth: clampSidebarWidth(result.data.leftSidebarWidth),
    rightSidebarWidth: clampSidebarWidth(result.data.rightSidebarWidth),
    lastScreen: isKnownScreen ? result.data.lastScreen : DEFAULT_LAYOUT_SETTINGS.lastScreen,
  };
}
