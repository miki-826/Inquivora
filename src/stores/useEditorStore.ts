import { invoke } from "@tauri-apps/api/core";
import { create } from "zustand";
import {
  addOrActivateTab,
  closeTab as closeTabModel,
  languageForExtension,
  reorderTabs,
  viewTypeForFile,
  type EditorTab,
} from "../features/editor/editorModel";
import type { TreeEntry } from "../features/workspace/treeModel";
import * as ws from "../services/workspace";
import type { ReadMode } from "../services/workspace";
import { useWorkspaceStore } from "./useWorkspaceStore";

const AUTOSAVE_DEBOUNCE_MS = 800;
const OWN_SAVE_SUPPRESS_MS = 2000;
const TABS_PERSIST_DEBOUNCE_MS = 500;

type RecentTabRecord = {
  path: string;
  isPinned: boolean;
  cursorLine: number;
  cursorColumn: number;
};

export type ConflictState = {
  tabId: string;
  showDiff: boolean;
  diskContent: string | null;
};

type EditorStore = {
  tabs: EditorTab[];
  activeTabId: string | null;
  contents: Record<string, string>;
  readModes: Record<string, ReadMode>;
  saveErrors: Record<string, string>;
  previewVisible: Record<string, boolean>;
  conflict: ConflictState | null;
  openFile: (entry: TreeEntry) => Promise<void>;
  openPath: (absolutePath: string, options?: Partial<Pick<EditorTab, "isPinned" | "cursorLine" | "cursorColumn">>) => Promise<void>;
  activateTab: (tabId: string) => void;
  reorderTab: (fromId: string, toId: string) => void;
  closeTab: (tabId: string) => Promise<void>;
  closeAllTabs: () => void;
  updateContent: (tabId: string, content: string) => void;
  setCursor: (tabId: string, line: number, column: number) => void;
  togglePin: (tabId: string) => void;
  togglePreview: (tabId: string) => void;
  saveTab: (tabId: string) => Promise<boolean>;
  saveActiveTab: () => Promise<void>;
  saveAllTabs: () => Promise<void>;
  handleExternalChanges: (paths: string[]) => Promise<void>;
  resolveConflictReload: () => Promise<void>;
  resolveConflictOverwrite: () => Promise<void>;
  resolveConflictShowDiff: () => Promise<void>;
  resolveConflictSaveAs: () => Promise<void>;
  dismissConflict: () => void;
  restoreTabs: (workspaceId: string) => Promise<void>;
};

function messageOf(error: unknown): string {
  if (error && typeof error === "object" && "message" in error) {
    return String((error as { message: unknown }).message);
  }
  return String(error);
}

function nameOf(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

function extensionOf(path: string): string | null {
  const name = nameOf(path);
  const dot = name.lastIndexOf(".");
  return dot > 0 ? name.slice(dot + 1).toLowerCase() : null;
}

const autosaveTimers = new Map<string, ReturnType<typeof setTimeout>>();
const suppressedPaths = new Map<string, number>();
let tabsPersistTimer: ReturnType<typeof setTimeout> | undefined;
let tabIdCounter = 0;

function nextTabId(): string {
  tabIdCounter += 1;
  return `tab-${Date.now()}-${tabIdCounter}`;
}

function suppressOwnChange(path: string) {
  suppressedPaths.set(path.toLowerCase(), Date.now() + OWN_SAVE_SUPPRESS_MS);
}

function isSuppressed(path: string): boolean {
  const until = suppressedPaths.get(path.toLowerCase());
  return until !== undefined && Date.now() < until;
}

export const useEditorStore = create<EditorStore>((set, get) => {
  function schedulePersistTabs() {
    clearTimeout(tabsPersistTimer);
    tabsPersistTimer = setTimeout(() => {
      const workspace = useWorkspaceStore.getState().workspace;
      if (!workspace) return;
      const tabs: RecentTabRecord[] = get().tabs.map((t) => ({
        path: t.path,
        isPinned: t.isPinned,
        cursorLine: t.cursorLine,
        cursorColumn: t.cursorColumn,
      }));
      invoke("tabs_save", { workspaceId: workspace.id, tabs }).catch((error) =>
        console.error("タブ状態の保存に失敗", error),
      );
    }, TABS_PERSIST_DEBOUNCE_MS);
  }

  function scheduleAutosave(tabId: string) {
    clearTimeout(autosaveTimers.get(tabId));
    autosaveTimers.set(
      tabId,
      setTimeout(() => {
        autosaveTimers.delete(tabId);
        void get().saveTab(tabId);
      }, AUTOSAVE_DEBOUNCE_MS),
    );
  }

  function patchTab(tabId: string, patch: Partial<EditorTab>) {
    set((state) => ({
      tabs: state.tabs.map((t) => (t.id === tabId ? { ...t, ...patch } : t)),
    }));
  }

  async function loadIntoTab(tab: EditorTab): Promise<EditorTab> {
    if (tab.viewType !== "editor" && tab.viewType !== "markdown-preview") {
      return tab;
    }
    const file = await ws.readFile(tab.path);
    set((state) => ({
      contents: { ...state.contents, [tab.id]: file.content },
      readModes: { ...state.readModes, [tab.id]: file.readMode },
    }));
    return {
      ...tab,
      encoding: file.encoding,
      lineEnding: file.lineEnding,
      isDirty: false,
    };
  }

  return {
    tabs: [],
    activeTabId: null,
    contents: {},
    readModes: {},
    saveErrors: {},
    previewVisible: {},
    conflict: null,

    openFile: async (entry) => {
      const absolute = useWorkspaceStore.getState().absolutePath(entry.relativePath);
      if (entry.category === "external") {
        await useWorkspaceStore.getState().openExternal(entry.relativePath);
        return;
      }
      await get().openPath(absolute);
    },

    openPath: async (absolutePath, options = {}) => {
      const existing = get().tabs.find((t) => t.path === absolutePath);
      if (existing) {
        set({ activeTabId: existing.id });
        return;
      }
      const extension = extensionOf(absolutePath);
      const detected = await ws.detectType(absolutePath).catch(() => null);
      const fileCategory = detected?.category ?? "edit";
      if (fileCategory === "external") {
        await ws.openExternal(absolutePath).catch((error) => console.error(error));
        return;
      }
      if (fileCategory === "unknown" && detected && !detected.isProbablyText) {
        await ws.openExternal(absolutePath).catch((error) => console.error(error));
        return;
      }
      let tab: EditorTab = {
        id: nextTabId(),
        path: absolutePath,
        name: nameOf(absolutePath),
        language: languageForExtension(extension),
        encoding: "utf8",
        lineEnding: "LF",
        isDirty: false,
        isPinned: options.isPinned ?? false,
        cursorLine: options.cursorLine ?? 1,
        cursorColumn: options.cursorColumn ?? 1,
        viewType: viewTypeForFile(fileCategory, extension),
      };
      try {
        tab = await loadIntoTab(tab);
      } catch (error) {
        console.error("ファイルを開けません", error);
        useWorkspaceStore.setState({ error: messageOf(error) });
        return;
      }
      set((state) => {
        const next = addOrActivateTab(state.tabs, state.activeTabId, tab);
        return { tabs: next.tabs, activeTabId: next.activeTabId };
      });
      schedulePersistTabs();
    },

    activateTab: (tabId) => {
      set({ activeTabId: tabId });
    },

    reorderTab: (fromId, toId) => {
      set((state) => ({ tabs: reorderTabs(state.tabs, fromId, toId) }));
      schedulePersistTabs();
    },

    closeTab: async (tabId) => {
      const tab = get().tabs.find((t) => t.id === tabId);
      if (!tab) return;
      clearTimeout(autosaveTimers.get(tabId));
      autosaveTimers.delete(tabId);
      if (tab.isDirty) {
        const saved = await get().saveTab(tabId);
        if (!saved) return;
      }
      set((state) => {
        const next = closeTabModel(state.tabs, state.activeTabId, tabId);
        const contents = { ...state.contents };
        const readModes = { ...state.readModes };
        const saveErrors = { ...state.saveErrors };
        const previewVisible = { ...state.previewVisible };
        delete contents[tabId];
        delete readModes[tabId];
        delete saveErrors[tabId];
        delete previewVisible[tabId];
        return {
          tabs: next.tabs,
          activeTabId: next.activeTabId,
          contents,
          readModes,
          saveErrors,
          previewVisible,
        };
      });
      schedulePersistTabs();
    },

    closeAllTabs: () => {
      autosaveTimers.forEach((timer) => clearTimeout(timer));
      autosaveTimers.clear();
      set({
        tabs: [],
        activeTabId: null,
        contents: {},
        readModes: {},
        saveErrors: {},
        previewVisible: {},
        conflict: null,
      });
    },

    updateContent: (tabId, content) => {
      const readMode = get().readModes[tabId] ?? "normal";
      if (readMode !== "normal") return;
      set((state) => ({ contents: { ...state.contents, [tabId]: content } }));
      patchTab(tabId, { isDirty: true });
      scheduleAutosave(tabId);
    },

    setCursor: (tabId, line, column) => {
      patchTab(tabId, { cursorLine: line, cursorColumn: column });
      schedulePersistTabs();
    },

    togglePin: (tabId) => {
      const tab = get().tabs.find((t) => t.id === tabId);
      if (tab) {
        patchTab(tabId, { isPinned: !tab.isPinned });
        schedulePersistTabs();
      }
    },

    togglePreview: (tabId) => {
      set((state) => ({
        previewVisible: { ...state.previewVisible, [tabId]: !state.previewVisible[tabId] },
      }));
    },

    saveTab: async (tabId) => {
      const { tabs, contents } = get();
      const tab = tabs.find((t) => t.id === tabId);
      const content = contents[tabId];
      if (!tab || content === undefined || !tab.isDirty) return true;
      try {
        suppressOwnChange(tab.path);
        await ws.writeFileAtomic(tab.path, content, tab.encoding, tab.lineEnding);
        suppressOwnChange(tab.path);
        patchTab(tabId, { isDirty: false });
        set((state) => {
          const saveErrors = { ...state.saveErrors };
          delete saveErrors[tabId];
          return { saveErrors };
        });
        return true;
      } catch (error) {
        console.error("保存に失敗", error);
        set((state) => ({
          saveErrors: { ...state.saveErrors, [tabId]: messageOf(error) },
        }));
        return false;
      }
    },

    saveActiveTab: async () => {
      const { activeTabId } = get();
      if (activeTabId) await get().saveTab(activeTabId);
    },

    saveAllTabs: async () => {
      for (const tab of get().tabs) {
        if (tab.isDirty) await get().saveTab(tab.id);
      }
    },

    handleExternalChanges: async (paths) => {
      const { tabs } = get();
      for (const changed of paths) {
        if (isSuppressed(changed)) continue;
        const tab = tabs.find((t) => t.path.toLowerCase() === changed.toLowerCase());
        if (!tab) continue;
        if (tab.viewType !== "editor" && tab.viewType !== "markdown-preview") continue;
        if (!tab.isDirty) {
          try {
            const reloaded = await loadIntoTab(tab);
            patchTab(tab.id, {
              encoding: reloaded.encoding,
              lineEnding: reloaded.lineEnding,
              isDirty: false,
            });
          } catch (error) {
            console.error("外部変更の再読込に失敗", error);
          }
        } else if (!get().conflict) {
          set({ conflict: { tabId: tab.id, showDiff: false, diskContent: null } });
        }
      }
    },

    resolveConflictReload: async () => {
      const conflict = get().conflict;
      if (!conflict) return;
      const tab = get().tabs.find((t) => t.id === conflict.tabId);
      if (tab) {
        try {
          const reloaded = await loadIntoTab(tab);
          patchTab(tab.id, {
            encoding: reloaded.encoding,
            lineEnding: reloaded.lineEnding,
            isDirty: false,
          });
        } catch (error) {
          console.error("外部内容の読み込みに失敗", error);
        }
      }
      set({ conflict: null });
    },

    resolveConflictOverwrite: async () => {
      const conflict = get().conflict;
      if (!conflict) return;
      const tab = get().tabs.find((t) => t.id === conflict.tabId);
      if (tab) {
        patchTab(tab.id, { isDirty: true });
        await get().saveTab(tab.id);
      }
      set({ conflict: null });
    },

    resolveConflictShowDiff: async () => {
      const conflict = get().conflict;
      if (!conflict) return;
      const tab = get().tabs.find((t) => t.id === conflict.tabId);
      if (!tab) {
        set({ conflict: null });
        return;
      }
      try {
        const disk = await ws.readFile(tab.path);
        set({ conflict: { ...conflict, showDiff: true, diskContent: disk.content } });
      } catch (error) {
        console.error("差分表示用の読み込みに失敗", error);
      }
    },

    resolveConflictSaveAs: async () => {
      const conflict = get().conflict;
      if (!conflict) return;
      const tab = get().tabs.find((t) => t.id === conflict.tabId);
      const content = get().contents[conflict.tabId];
      if (!tab || content === undefined) {
        set({ conflict: null });
        return;
      }
      const { save } = await import("@tauri-apps/plugin-dialog");
      const target = await save({
        defaultPath: tab.path.replace(/(\.[^.\\/]+)?$/, "（競合コピー）$1"),
        title: "別名で保存",
      });
      if (typeof target === "string" && target) {
        try {
          suppressOwnChange(target);
          await ws.writeFileAtomic(target, content, tab.encoding, tab.lineEnding);
          const reloaded = await loadIntoTab(tab);
          patchTab(tab.id, {
            encoding: reloaded.encoding,
            lineEnding: reloaded.lineEnding,
            isDirty: false,
          });
          set({ conflict: null });
        } catch (error) {
          console.error("別名保存に失敗", error);
          useWorkspaceStore.setState({ error: messageOf(error) });
        }
      }
    },

    dismissConflict: () => set({ conflict: null }),

    restoreTabs: async (workspaceId) => {
      try {
        const stored = await invoke<RecentTabRecord[]>("tabs_load", { workspaceId });
        for (const record of stored) {
          await get().openPath(record.path, {
            isPinned: record.isPinned,
            cursorLine: record.cursorLine,
            cursorColumn: record.cursorColumn,
          });
        }
      } catch (error) {
        console.error("タブの復元に失敗", error);
      }
    },
  };
});
