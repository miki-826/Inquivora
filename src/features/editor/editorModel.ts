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

export function languageForExtension(extension: string | null): string {
  throw new Error("未実装");
}

export function viewTypeForFile(
  category: FileCategory,
  extension: string | null,
): EditorTab["viewType"] {
  throw new Error("未実装");
}

export function addOrActivateTab(
  tabs: EditorTab[],
  activeTabId: string | null,
  newTab: EditorTab,
): { tabs: EditorTab[]; activeTabId: string | null } {
  throw new Error("未実装");
}

export function closeTab(
  tabs: EditorTab[],
  activeTabId: string | null,
  closeId: string,
): { tabs: EditorTab[]; activeTabId: string | null } {
  throw new Error("未実装");
}
