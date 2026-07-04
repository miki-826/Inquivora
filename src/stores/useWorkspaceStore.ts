import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { create } from "zustand";
import {
  isSameOrDescendant,
  joinPath,
  parentPath,
  type TreeEntry,
} from "../features/workspace/treeModel";
import { loadSetting, saveSetting } from "../services/settings";
import * as ws from "../services/workspace";
import type { WorkspaceRecord } from "../services/workspace";

const LAST_WORKSPACE_KEY = "workspace.lastOpenedPath";

export type TreeEditing = {
  mode: "create-file" | "create-folder" | "rename";
  targetPath: string;
};

type WorkspaceStore = {
  workspace: WorkspaceRecord | null;
  recent: WorkspaceRecord[];
  children: Record<string, TreeEntry[] | undefined>;
  expanded: string[];
  selectedPath: string | null;
  editing: TreeEditing | null;
  clipboard: { path: string; cut: boolean } | null;
  error: string | null;
  restoring: boolean;
  loadRecent: () => Promise<void>;
  openWorkspaceByDialog: () => Promise<void>;
  openWorkspacePath: (path: string) => Promise<void>;
  restoreLastWorkspace: () => Promise<void>;
  toggleFolder: (relativePath: string) => Promise<void>;
  refresh: () => Promise<void>;
  select: (relativePath: string | null) => void;
  startEditing: (editing: TreeEditing) => void;
  cancelEditing: () => void;
  commitEditing: (name: string) => Promise<void>;
  removeEntry: (relativePath: string) => Promise<void>;
  setClipboard: (relativePath: string, cut: boolean) => void;
  paste: (targetFolder: string) => Promise<void>;
  moveByDrop: (source: string, targetFolder: string) => Promise<void>;
  copyPathToClipboard: (relativePath: string) => Promise<void>;
  reveal: (relativePath: string) => Promise<void>;
  openExternal: (relativePath: string) => Promise<void>;
  absolutePath: (relativePath: string) => string;
  clearError: () => void;
};

function messageOf(error: unknown): string {
  if (error && typeof error === "object" && "message" in error) {
    return String((error as { message: unknown }).message);
  }
  return String(error);
}

function treeStateKey(workspaceId: string): string {
  return `workspace.tree.${workspaceId}`;
}

export const useWorkspaceStore = create<WorkspaceStore>((set, get) => {
  async function loadChildrenOf(relativePath: string): Promise<void> {
    const entries = await ws.listChildren(relativePath);
    set((state) => ({ children: { ...state.children, [relativePath]: entries } }));
  }

  async function runTreeAction(action: () => Promise<void>): Promise<void> {
    try {
      await action();
      set({ error: null });
    } catch (error) {
      console.error(error);
      set({ error: messageOf(error) });
    }
  }

  async function persistExpanded(): Promise<void> {
    const { workspace, expanded } = get();
    if (!workspace) return;
    await saveSetting(treeStateKey(workspace.id), { expanded }).catch(() => undefined);
  }

  return {
    workspace: null,
    recent: [],
    children: {},
    expanded: [],
    selectedPath: null,
    editing: null,
    clipboard: null,
    error: null,
    restoring: true,

    loadRecent: async () => {
      try {
        set({ recent: await ws.listRecentWorkspaces() });
      } catch (error) {
        console.error("最近のワークスペース取得に失敗", error);
      }
    },

    openWorkspaceByDialog: async () => {
      const path = await openDialog({ directory: true, title: "ワークスペースフォルダを選択" });
      if (typeof path === "string") {
        await get().openWorkspacePath(path);
      }
    },

    openWorkspacePath: async (path) => {
      await runTreeAction(async () => {
        const workspace = await ws.openWorkspace(path);
        const stored = await loadSetting<{ expanded?: string[] }>(treeStateKey(workspace.id)).catch(
          () => null,
        );
        const expanded = Array.isArray(stored?.expanded) ? stored.expanded : [];
        set({
          workspace,
          expanded,
          children: {},
          selectedPath: null,
          editing: null,
          clipboard: null,
        });
        await saveSetting(LAST_WORKSPACE_KEY, path).catch(() => undefined);
        await loadChildrenOf("");
        await Promise.all(
          expanded.map((folder) => loadChildrenOf(folder).catch(() => undefined)),
        );
        await get().loadRecent();
      });
    },

    restoreLastWorkspace: async () => {
      try {
        const path = await loadSetting<string>(LAST_WORKSPACE_KEY);
        if (typeof path === "string" && path) {
          await get().openWorkspacePath(path);
        }
        await get().loadRecent();
      } catch (error) {
        console.error("前回ワークスペースの復元に失敗", error);
      } finally {
        set({ restoring: false });
      }
    },

    toggleFolder: async (relativePath) => {
      const { expanded } = get();
      if (expanded.includes(relativePath)) {
        set({ expanded: expanded.filter((p) => p !== relativePath) });
      } else {
        set({ expanded: [...expanded, relativePath] });
        if (!get().children[relativePath]) {
          await runTreeAction(() => loadChildrenOf(relativePath));
        }
      }
      await persistExpanded();
    },

    refresh: async () => {
      const { children, expanded } = get();
      await runTreeAction(async () => {
        const loaded = Object.keys(children);
        const keep = loaded.filter((path) => path === "" || expanded.includes(path));
        set({ children: {} });
        await Promise.all(keep.map((path) => loadChildrenOf(path).catch(() => undefined)));
      });
    },

    select: (relativePath) => set({ selectedPath: relativePath }),
    startEditing: (editing) => set({ editing }),
    cancelEditing: () => set({ editing: null }),

    commitEditing: async (name) => {
      const { editing, absolutePath } = get();
      const trimmed = name.trim();
      if (!editing || !trimmed) {
        set({ editing: null });
        return;
      }
      await runTreeAction(async () => {
        if (editing.mode === "rename") {
          const newRelative = joinPath(parentPath(editing.targetPath), trimmed);
          await ws.renameEntry(absolutePath(editing.targetPath), absolutePath(newRelative));
          await loadChildrenOf(parentPath(editing.targetPath));
        } else {
          const relative = joinPath(editing.targetPath, trimmed);
          await ws.createEntry(
            absolutePath(relative),
            editing.mode === "create-folder" ? "folder" : "file",
          );
          await loadChildrenOf(editing.targetPath);
        }
        set({ editing: null });
      });
    },

    removeEntry: async (relativePath) => {
      await runTreeAction(async () => {
        await ws.deleteEntry(get().absolutePath(relativePath), true);
        await loadChildrenOf(parentPath(relativePath));
      });
    },

    setClipboard: (relativePath, cut) => set({ clipboard: { path: relativePath, cut } }),

    paste: async (targetFolder) => {
      const { clipboard, absolutePath } = get();
      if (!clipboard) return;
      const name = clipboard.path.split("/").pop() ?? clipboard.path;
      const destination = joinPath(targetFolder, name);
      if (clipboard.cut && isSameOrDescendant(clipboard.path, destination)) return;
      await runTreeAction(async () => {
        if (clipboard.cut) {
          await ws.moveEntry(absolutePath(clipboard.path), absolutePath(destination));
          set({ clipboard: null });
          await loadChildrenOf(parentPath(clipboard.path));
        } else {
          await ws.copyEntry(absolutePath(clipboard.path), absolutePath(destination));
        }
        await loadChildrenOf(targetFolder);
      });
    },

    moveByDrop: async (source, targetFolder) => {
      if (isSameOrDescendant(source, targetFolder) || parentPath(source) === targetFolder) return;
      const name = source.split("/").pop() ?? source;
      await runTreeAction(async () => {
        await ws.moveEntry(
          get().absolutePath(source),
          get().absolutePath(joinPath(targetFolder, name)),
        );
        await loadChildrenOf(parentPath(source));
        await loadChildrenOf(targetFolder);
      });
    },

    copyPathToClipboard: async (relativePath) => {
      await navigator.clipboard.writeText(get().absolutePath(relativePath));
    },

    reveal: async (relativePath) => {
      await runTreeAction(() => ws.revealInExplorer(get().absolutePath(relativePath)));
    },

    openExternal: async (relativePath) => {
      await runTreeAction(() => ws.openExternal(get().absolutePath(relativePath)));
    },

    absolutePath: (relativePath) => {
      const root = get().workspace?.rootPath ?? "";
      if (relativePath === "") return root;
      return `${root}\\${relativePath.split("/").join("\\")}`;
    },

    clearError: () => set({ error: null }),
  };
});
