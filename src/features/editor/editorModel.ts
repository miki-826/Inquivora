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

const IMAGE_EXTENSIONS = new Set(["png", "jpg", "jpeg", "gif", "webp", "svg"]);
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
  activeTabId: string | null,
  newTab: EditorTab,
): { tabs: EditorTab[]; activeTabId: string | null } {
  const existing = tabs.find((t) => t.path === newTab.path);
  if (existing) {
    return { tabs, activeTabId: existing.id };
  }
  return { tabs: [...tabs, newTab], activeTabId: newTab.id };
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
