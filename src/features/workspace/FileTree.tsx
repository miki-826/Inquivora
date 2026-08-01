import { useVirtualizer } from "@tanstack/react-virtual";
import {
  ChevronDown,
  ChevronRight,
  FilePlus,
  FileText,
  Folder,
  FolderOpen,
  FolderPlus,
  Image,
  Package,
  RefreshCw,
  Trash2,
} from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { useWorkspaceStore } from "../../stores/useWorkspaceStore";
import { EDITOR_FILE_DRAG_TYPE } from "../editor/editorModel";
import { TreeContextMenu, type MenuState } from "./TreeContextMenu";
import { flattenTree, type TreeEntry, type TreeRow } from "./treeModel";

const ROW_HEIGHT = 26;

type DisplayRow =
  | { kind: "entry"; row: TreeRow }
  | { kind: "input"; depth: number };

type FileTreeProps = {
  onOpenFile?: (entry: TreeEntry) => void;
};

function entryIcon(entry: TreeEntry, expanded: boolean) {
  if (entry.isFolder) {
    return expanded ? <FolderOpen size={15} aria-hidden /> : <Folder size={15} aria-hidden />;
  }
  if (entry.category === "preview") return <Image size={15} aria-hidden />;
  if (entry.category === "external") return <Package size={15} aria-hidden />;
  return <FileText size={15} aria-hidden />;
}

export function FileTree({ onOpenFile }: FileTreeProps) {
  const workspace = useWorkspaceStore((s) => s.workspace);
  const recent = useWorkspaceStore((s) => s.recent);
  const children = useWorkspaceStore((s) => s.children);
  const expanded = useWorkspaceStore((s) => s.expanded);
  const selectedPath = useWorkspaceStore((s) => s.selectedPath);
  const editing = useWorkspaceStore((s) => s.editing);
  const error = useWorkspaceStore((s) => s.error);
  const store = useWorkspaceStore;

  const [menu, setMenu] = useState<MenuState | null>(null);
  const [dropTarget, setDropTarget] = useState<string | null>(null);
  const [draggingPath, setDraggingPath] = useState<string | null>(null);
  const [trashHover, setTrashHover] = useState(false);
  const [inputValue, setInputValue] = useState("");
  const scrollRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  const expandedSet = useMemo(() => new Set(expanded), [expanded]);
  const rows = useMemo<DisplayRow[]>(() => {
    const flat = flattenTree(children, expandedSet).map<DisplayRow>((row) => ({
      kind: "entry",
      row,
    }));
    if (!editing) return flat;
    if (editing.mode === "rename") return flat;
    const parent = editing.targetPath;
    if (parent === "") return [{ kind: "input", depth: 0 }, ...flat];
    const index = flat.findIndex(
      (r) => r.kind === "entry" && r.row.entry.relativePath === parent,
    );
    if (index === -1) return [{ kind: "input", depth: 0 }, ...flat];
    const depth = (flat[index] as { row: TreeRow }).row.depth + 1;
    return [...flat.slice(0, index + 1), { kind: "input", depth }, ...flat.slice(index + 1)];
  }, [children, expandedSet, editing]);

  const virtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => ROW_HEIGHT,
    overscan: 12,
  });

  useEffect(() => {
    if (editing) {
      setInputValue(editing.mode === "rename" ? editing.targetPath.split("/").pop() ?? "" : "");
      requestAnimationFrame(() => {
        inputRef.current?.focus();
        inputRef.current?.select();
      });
    }
  }, [editing]);

  if (!workspace) {
    return (
      <div className="file-tree file-tree--empty">
        <p className="file-tree__empty-text">フォルダを開いてワークスペースを開始します。</p>
        <button
          type="button"
          className="file-tree__open-button"
          onClick={() => store.getState().openWorkspaceByDialog()}
        >
          <FolderOpen size={16} aria-hidden />
          フォルダを開く
        </button>
        {recent.length > 0 && (
          <div className="file-tree__recent">
            <h3>最近使用したワークスペース</h3>
            {recent.map((r) => (
              <button
                key={r.id}
                type="button"
                className="file-tree__recent-item"
                title={r.rootPath}
                onClick={() => store.getState().openWorkspacePath(r.rootPath)}
              >
                {r.name}
              </button>
            ))}
          </div>
        )}
      </div>
    );
  }

  const commitInput = () => {
    store.getState().commitEditing(inputValue);
  };

  const handleDropInto = (e: React.DragEvent, targetFolder: string) => {
    const files = Array.from(e.dataTransfer.files);
    if (files.length > 0) {
      void store.getState().importFiles(targetFolder, files);
      return;
    }
    const source = e.dataTransfer.getData("text/inquivora-path");
    if (source) void store.getState().moveByDrop(source, targetFolder);
  };

  const isOverTreeRow = (target: EventTarget | null): boolean =>
    target instanceof Element && target.closest(".file-tree__row") !== null;

  const renderInput = (depth: number) => (
    <div className="file-tree__row" style={{ paddingLeft: 8 + depth * 14 }}>
      <span className="file-tree__chevron" />
      <input
        ref={inputRef}
        className="file-tree__input"
        value={inputValue}
        aria-label="名前を入力"
        onChange={(e) => setInputValue(e.target.value)}
        onBlur={commitInput}
        onKeyDown={(e) => {
          if (e.key === "Enter") commitInput();
          if (e.key === "Escape") store.getState().cancelEditing();
        }}
      />
    </div>
  );

  return (
    <div
      className="file-tree"
      onContextMenu={(e) => {
        e.preventDefault();
        setMenu({ x: e.clientX, y: e.clientY, target: null });
      }}
    >
      <header className="file-tree__header">
        <span className="file-tree__title" title={workspace.rootPath}>
          {workspace.name}
        </span>
        <div className="file-tree__actions">
          <button
            type="button"
            title="新規ファイル"
            aria-label="新規ファイル"
            onClick={() => store.getState().startEditing({ mode: "create-file", targetPath: "" })}
          >
            <FilePlus size={15} aria-hidden />
          </button>
          <button
            type="button"
            title="新規フォルダ"
            aria-label="新規フォルダ"
            onClick={() => store.getState().startEditing({ mode: "create-folder", targetPath: "" })}
          >
            <FolderPlus size={15} aria-hidden />
          </button>
          <button
            type="button"
            title="更新"
            aria-label="更新"
            onClick={() => store.getState().refresh()}
          >
            <RefreshCw size={15} aria-hidden />
          </button>
          <button
            type="button"
            title="別のフォルダを開く"
            aria-label="別のフォルダを開く"
            onClick={() => store.getState().openWorkspaceByDialog()}
          >
            <FolderOpen size={15} aria-hidden />
          </button>
        </div>
      </header>
      {error && (
        <div className="file-tree__error" role="alert">
          <span>{error}</span>
          <button type="button" onClick={() => store.getState().clearError()} aria-label="閉じる">
            ×
          </button>
        </div>
      )}
      <div
        ref={scrollRef}
        className={`file-tree__scroll${dropTarget === "" ? " file-tree__scroll--drop" : ""}`}
        role="tree"
        aria-label="ファイルツリー"
        onDragOver={(e) => {
          if (isOverTreeRow(e.target)) return;
          e.preventDefault();
          setDropTarget("");
        }}
        onDragLeave={(e) => {
          if (!e.currentTarget.contains(e.relatedTarget as Node | null)) {
            setDropTarget(null);
          }
        }}
        onDrop={(e) => {
          if (isOverTreeRow(e.target)) return;
          e.preventDefault();
          setDropTarget(null);
          handleDropInto(e, "");
        }}
      >
        <div style={{ height: virtualizer.getTotalSize(), position: "relative" }}>
          {virtualizer.getVirtualItems().map((item) => {
            const displayRow = rows[item.index];
            const style: React.CSSProperties = {
              position: "absolute",
              top: 0,
              left: 0,
              width: "100%",
              height: item.size,
              transform: `translateY(${item.start}px)`,
            };
            if (displayRow.kind === "input") {
              return (
                <div key="tree-input" style={style}>
                  {renderInput(displayRow.depth)}
                </div>
              );
            }
            const { entry, depth } = displayRow.row;
            const isExpanded = expandedSet.has(entry.relativePath);
            const isRenaming =
              editing?.mode === "rename" && editing.targetPath === entry.relativePath;
            if (isRenaming) {
              return (
                <div key={entry.relativePath} style={style}>
                  {renderInput(depth)}
                </div>
              );
            }
            const classNames = ["file-tree__row"];
            if (selectedPath === entry.relativePath) classNames.push("file-tree__row--selected");
            if (dropTarget === entry.relativePath) classNames.push("file-tree__row--drop");
            return (
              <div key={entry.relativePath} style={style}>
                <div
                  className={classNames.join(" ")}
                  style={{ paddingLeft: 8 + depth * 14 }}
                  role="treeitem"
                  aria-selected={selectedPath === entry.relativePath}
                  aria-expanded={entry.isFolder ? isExpanded : undefined}
                  tabIndex={0}
                  draggable
                  onDragStart={(e) => {
                    e.dataTransfer.setData("text/inquivora-path", entry.relativePath);
                    if (!entry.isFolder && entry.category !== "external") {
                      e.dataTransfer.setData(EDITOR_FILE_DRAG_TYPE, entry.relativePath);
                    }
                    e.dataTransfer.effectAllowed = "copyMove";
                    setDraggingPath(entry.relativePath);
                  }}
                  onDragEnd={() => setDraggingPath(null)}
                  onDragOver={(e) => {
                    if (!entry.isFolder) {
                      e.stopPropagation();
                      return;
                    }
                    e.preventDefault();
                    e.stopPropagation();
                    setDropTarget(entry.relativePath);
                  }}
                  onDrop={(e) => {
                    if (!entry.isFolder) {
                      e.preventDefault();
                      e.stopPropagation();
                      setDropTarget(null);
                      return;
                    }
                    e.preventDefault();
                    e.stopPropagation();
                    setDropTarget(null);
                    handleDropInto(e, entry.relativePath);
                  }}
                  onClick={() => {
                    store.getState().select(entry.relativePath);
                    if (entry.isFolder) {
                      store.getState().toggleFolder(entry.relativePath);
                    } else {
                      onOpenFile?.(entry);
                    }
                  }}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") {
                      store.getState().select(entry.relativePath);
                      if (entry.isFolder) store.getState().toggleFolder(entry.relativePath);
                      else onOpenFile?.(entry);
                    }
                  }}
                  onContextMenu={(e) => {
                    e.preventDefault();
                    e.stopPropagation();
                    store.getState().select(entry.relativePath);
                    setMenu({ x: e.clientX, y: e.clientY, target: entry });
                  }}
                >
                  <span className="file-tree__chevron">
                    {entry.isFolder &&
                      entry.hasChildren &&
                      (isExpanded ? (
                        <ChevronDown size={13} aria-hidden />
                      ) : (
                        <ChevronRight size={13} aria-hidden />
                      ))}
                  </span>
                  {entryIcon(entry, isExpanded)}
                  <span className="file-tree__name">{entry.name}</span>
                </div>
              </div>
            );
          })}
        </div>
      </div>
      {draggingPath && (
        <div
          className={`file-tree__trash${trashHover ? " file-tree__trash--hover" : ""}`}
          role="button"
          aria-label="ここへドラッグしてごみ箱へ移動"
          onDragOver={(e) => {
            e.preventDefault();
            e.stopPropagation();
            setTrashHover(true);
          }}
          onDragLeave={() => setTrashHover(false)}
          onDrop={(e) => {
            e.preventDefault();
            e.stopPropagation();
            setTrashHover(false);
            const source = e.dataTransfer.getData("text/inquivora-path");
            setDraggingPath(null);
            if (source) void store.getState().removeEntry(source);
          }}
        >
          <Trash2 size={15} aria-hidden />
          ここへドロップで削除（ごみ箱へ移動）
        </div>
      )}
      {menu && (
        <TreeContextMenu
          menu={menu}
          onClose={() => setMenu(null)}
          onOpenFile={onOpenFile}
        />
      )}
    </div>
  );
}
