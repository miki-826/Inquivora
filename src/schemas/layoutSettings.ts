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

export type NavigationPosition = "side" | "top" | "right" | "bottom";
export type TaskListFontSize = "small" | "medium" | "large";
export type UiDensity = "comfortable" | "compact";
export type EditorFontSize = "small" | "medium" | "large";

export type LayoutSettings = {
  leftSidebarWidth: number;
  rightSidebarWidth: number;
  lastScreen: string;
  navigationPosition: NavigationPosition;
  taskListFontSize: TaskListFontSize;
  uiDensity: UiDensity;
  showStatusBar: boolean;
  reduceMotion: boolean;
  editorFontSize: EditorFontSize;
  editorWordWrap: boolean;
};

export const DEFAULT_LAYOUT_SETTINGS: LayoutSettings = {
  leftSidebarWidth: 320,
  rightSidebarWidth: 360,
  lastScreen: "/workspace",
  navigationPosition: "side",
  taskListFontSize: "small",
  uiDensity: "comfortable",
  showStatusBar: true,
  reduceMotion: false,
  editorFontSize: "medium",
  editorWordWrap: true,
};

const layoutSettingsSchema = z.object({
  leftSidebarWidth: z.number().optional(),
  rightSidebarWidth: z.number().optional(),
  lastScreen: z.string().optional(),
  navigationPosition: z.enum(["side", "top", "right", "bottom"]).optional(),
  taskListFontSize: z.enum(["small", "medium", "large"]).optional(),
  uiDensity: z.enum(["comfortable", "compact"]).optional(),
  showStatusBar: z.boolean().optional(),
  reduceMotion: z.boolean().optional(),
  editorFontSize: z.enum(["small", "medium", "large"]).optional(),
  editorWordWrap: z.boolean().optional(),
});

export function parseLayoutSettings(value: unknown): LayoutSettings {
  const result = layoutSettingsSchema.safeParse(value);
  if (!result.success) {
    return DEFAULT_LAYOUT_SETTINGS;
  }
  const lastScreen = result.data.lastScreen ?? DEFAULT_LAYOUT_SETTINGS.lastScreen;
  const isKnownScreen = (SCREEN_PATHS as readonly string[]).includes(lastScreen);
  return {
    leftSidebarWidth: clampSidebarWidth(
      result.data.leftSidebarWidth ?? DEFAULT_LAYOUT_SETTINGS.leftSidebarWidth,
    ),
    rightSidebarWidth: clampSidebarWidth(
      result.data.rightSidebarWidth ?? DEFAULT_LAYOUT_SETTINGS.rightSidebarWidth,
    ),
    lastScreen: isKnownScreen ? lastScreen : DEFAULT_LAYOUT_SETTINGS.lastScreen,
    navigationPosition:
      result.data.navigationPosition ?? DEFAULT_LAYOUT_SETTINGS.navigationPosition,
    taskListFontSize: result.data.taskListFontSize ?? DEFAULT_LAYOUT_SETTINGS.taskListFontSize,
    uiDensity: result.data.uiDensity ?? DEFAULT_LAYOUT_SETTINGS.uiDensity,
    showStatusBar: result.data.showStatusBar ?? DEFAULT_LAYOUT_SETTINGS.showStatusBar,
    reduceMotion: result.data.reduceMotion ?? DEFAULT_LAYOUT_SETTINGS.reduceMotion,
    editorFontSize: result.data.editorFontSize ?? DEFAULT_LAYOUT_SETTINGS.editorFontSize,
    editorWordWrap: result.data.editorWordWrap ?? DEFAULT_LAYOUT_SETTINGS.editorWordWrap,
  };
}
