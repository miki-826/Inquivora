import { invoke } from "@tauri-apps/api/core";
import type { FileCategory, TreeEntry } from "../features/workspace/treeModel";

export type WorkspaceRecord = {
  id: string;
  name: string;
  rootPath: string;
  lastOpenedAt: string;
  createdAt: string;
  updatedAt: string;
};

export type FileEncoding = "utf8" | "utf8-bom" | "utf16le" | "utf16be" | "shift_jis";
export type LineEnding = "LF" | "CRLF";
export type ReadMode = "normal" | "read-only" | "preview";

export type FileContent = {
  content: string;
  encoding: FileEncoding;
  lineEnding: LineEnding;
  sizeBytes: number;
  modifiedAt: string;
  readMode: ReadMode;
};

export type FileMeta = {
  sizeBytes: number;
  modifiedAt: string;
};

export type DetectedType = {
  category: FileCategory;
  isProbablyText: boolean;
};

export function openWorkspace(path: string): Promise<WorkspaceRecord> {
  return invoke("workspace_open", { path });
}

export function listRecentWorkspaces(): Promise<WorkspaceRecord[]> {
  return invoke("workspace_list_recent");
}

export function closeWorkspace(id: string): Promise<void> {
  return invoke("workspace_close", { id });
}

export function listChildren(relativePath: string): Promise<TreeEntry[]> {
  return invoke("file_list_children", { relativePath });
}

export function readFile(path: string): Promise<FileContent> {
  return invoke("file_read", { path });
}

export function writeFileAtomic(
  path: string,
  content: string,
  encoding: FileEncoding,
  lineEnding: LineEnding,
): Promise<FileMeta> {
  return invoke("file_write_atomic", { path, content, encoding, lineEnding });
}

export function createEntry(path: string, entryType: "file" | "folder"): Promise<void> {
  return invoke("file_create", { path, entryType });
}

export function renameEntry(oldPath: string, newPath: string): Promise<void> {
  return invoke("file_rename", { oldPath, newPath });
}

export function deleteEntry(path: string, useRecycleBin = true): Promise<void> {
  return invoke("file_delete", { path, useRecycleBin });
}

export function copyEntry(source: string, destination: string): Promise<void> {
  return invoke("file_copy", { source, destination });
}

export function moveEntry(source: string, destination: string): Promise<void> {
  return invoke("file_move", { source, destination });
}

export function revealInExplorer(path: string): Promise<void> {
  return invoke("file_reveal", { path });
}

export function openExternal(path: string): Promise<void> {
  return invoke("file_open_external", { path });
}

export function detectType(path: string): Promise<DetectedType> {
  return invoke("file_detect_type", { path });
}
