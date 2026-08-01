import type { FileCategory } from "../workspace/treeModel";
import type { FileEncoding, LineEnding } from "../../services/workspace";

/// §8.2 タブモデル
export type EditorTab = {
  id: string;
  path: string;
  name: string;
  language: string;
  encoding: FileEncoding;
  lineEnding: LineEnding;
  isDirty: boolean;
  isPinned: boolean;
  cursorLine: number;
  cursorColumn: number;
  viewType: "editor" | "markdown-preview" | "image" | "pdf" | "audio" | "video";
};

const LANGUAGE_BY_EXTENSION: Record<string, string> = {
  md: "markdown",
  txt: "plaintext",
  log: "plaintext",
  csv: "plaintext",
  json: "json",
  jsonl: "json",
  yaml: "yaml",
  yml: "yaml",
  xml: "xml",
  ini: "ini",
  conf: "ini",
  env: "ini",
  html: "html",
  htm: "html",
  css: "css",
  scss: "scss",
  js: "javascript",
  jsx: "javascript",
  ts: "typescript",
  tsx: "typescript",
  py: "python",
  ps1: "powershell",
  bat: "bat",
  sh: "shell",
  sql: "sql",
  rs: "rust",
  cs: "csharp",
  java: "java",
};

export function languageForExtension(extension: string | null): string {
  if (!extension) return "plaintext";
  return LANGUAGE_BY_EXTENSION[extension.toLowerCase()] ?? "plaintext";
}

const PREVIEWABLE_LANGUAGES = new Set(["markdown", "html"]);

/** エディタ横のプレビュー表示に対応する言語か */
export function isPreviewableLanguage(language: string): boolean {
  return PREVIEWABLE_LANGUAGES.has(language);
}

const IMAGE_EXTENSIONS = new Set(["png", "jpg", "jpeg", "gif", "webp", "svg", "jpe"]);
const AUDIO_EXTENSIONS = new Set(["wav", "mp3", "m4a"]);
const VIDEO_EXTENSIONS = new Set(["mp4", "webm"]);

export function viewTypeForFile(
  category: FileCategory,
  extension: string | null,
): EditorTab["viewType"] {
  if (category !== "preview" || !extension) return "editor";
  const ext = extension.toLowerCase();
  if (IMAGE_EXTENSIONS.has(ext)) return "image";
  if (ext === "pdf") return "pdf";
  if (AUDIO_EXTENSIONS.has(ext)) return "audio";
  if (VIDEO_EXTENSIONS.has(ext)) return "video";
  return "editor";
}

export function addOrActivateTab(
  tabs: EditorTab[],
  _activeTabId: string | null,
  newTab: EditorTab,
): { tabs: EditorTab[]; activeTabId: string | null } {
  const existing = tabs.find((t) => t.path === newTab.path);
  if (existing) {
    return { tabs, activeTabId: existing.id };
  }
  return { tabs: [...tabs, newTab], activeTabId: newTab.id };
}

export function shouldActivateFileFromTree(
  tabs: EditorTab[],
  activeTabId: string | null,
): boolean {
  return !tabs.find((tab) => tab.id === activeTabId)?.isPinned;
}

export function canActivateEditorTab(
  tabs: EditorTab[],
  activeTabId: string | null,
  targetTabId: string,
): boolean {
  if (activeTabId === targetTabId) return true;
  return !tabs.find((tab) => tab.id === activeTabId)?.isPinned;
}

/// ドラッグ元タブ(fromId)をドロップ先タブ(toId)の位置へ移動した配列を返す。
export function reorderTabs(tabs: EditorTab[], fromId: string, toId: string): EditorTab[] {
  const fromIndex = tabs.findIndex((t) => t.id === fromId);
  const toIndex = tabs.findIndex((t) => t.id === toId);
  if (fromIndex === -1 || toIndex === -1 || fromIndex === toIndex) {
    return tabs;
  }
  const next = [...tabs];
  const [moved] = next.splice(fromIndex, 1);
  next.splice(toIndex, 0, moved);
  return next;
}

export type EditorPane = "primary" | "secondary";
export const EDITOR_FILE_DRAG_TYPE = "text/inquivora-editor-file";

export type PaneAssignment = {
  activeTabId: string | null;
  secondaryTabId: string | null;
};

/// タブをドロップ先のペインへ割り当てる。既に反対側で開いていれば左右を入れ替える
/// （VS Codeと同じく、分割中はどちらのペインへもドラッグで移せるようにするため）。
export function assignTabToPane(
  panes: PaneAssignment,
  tabId: string,
  pane: EditorPane,
): PaneAssignment {
  const { activeTabId, secondaryTabId } = panes;
  if (pane === "primary") {
    if (activeTabId === tabId) return panes;
    return {
      activeTabId: tabId,
      secondaryTabId: secondaryTabId === tabId ? activeTabId : secondaryTabId,
    };
  }
  if (secondaryTabId === tabId) return panes;
  // 分割していない状態で左のタブを右へ落としても、左が空になるだけなので何もしない。
  if (secondaryTabId === null && activeTabId === tabId) return panes;
  return {
    activeTabId: activeTabId === tabId ? secondaryTabId : activeTabId,
    secondaryTabId: tabId,
  };
}

export const SPLIT_RATIO_MIN = 0.2;
export const SPLIT_RATIO_MAX = 0.8;
export const SPLIT_RATIO_DEFAULT = 0.5;

export function clampSplitRatio(ratio: number): number {
  if (!Number.isFinite(ratio)) return SPLIT_RATIO_DEFAULT;
  return Math.min(SPLIT_RATIO_MAX, Math.max(SPLIT_RATIO_MIN, ratio));
}

export function closeTab(
  tabs: EditorTab[],
  activeTabId: string | null,
  closeId: string,
): { tabs: EditorTab[]; activeTabId: string | null } {
  const index = tabs.findIndex((t) => t.id === closeId);
  const remaining = tabs.filter((t) => t.id !== closeId);
  if (activeTabId !== closeId) {
    return { tabs: remaining, activeTabId };
  }
  if (remaining.length === 0) {
    return { tabs: remaining, activeTabId: null };
  }
  const next = remaining[Math.min(index, remaining.length - 1)];
  return { tabs: remaining, activeTabId: next.id };
}
