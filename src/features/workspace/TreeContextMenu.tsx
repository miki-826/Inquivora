import { useEffect, useRef } from "react";
import { useWorkspaceStore } from "../../stores/useWorkspaceStore";
import { parentPath, type TreeEntry } from "./treeModel";

export type MenuState = {
  x: number;
  y: number;
  target: TreeEntry | null;
};

type MenuItem = {
  label: string;
  danger?: boolean;
  disabled?: boolean;
  action: () => void;
};

type TreeContextMenuProps = {
  menu: MenuState;
  onClose: () => void;
  onOpenFile?: (entry: TreeEntry) => void;
};

export function TreeContextMenu({ menu, onClose, onOpenFile }: TreeContextMenuProps) {
  const ref = useRef<HTMLDivElement>(null);
  const store = useWorkspaceStore;

  useEffect(() => {
    const close = (e: MouseEvent) => {
      if (!ref.current?.contains(e.target as Node)) onClose();
    };
    const onEscape = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("mousedown", close);
    window.addEventListener("keydown", onEscape);
    return () => {
      window.removeEventListener("mousedown", close);
      window.removeEventListener("keydown", onEscape);
    };
  }, [onClose]);

  const target = menu.target;
  const folderPath = target ? (target.isFolder ? target.relativePath : parentPath(target.relativePath)) : "";
  const clipboard = useWorkspaceStore((s) => s.clipboard);

  const items: MenuItem[] = [
    {
      label: "新規ファイル",
      action: () => store.getState().startEditing({ mode: "create-file", targetPath: folderPath }),
    },
    {
      label: "新規フォルダ",
      action: () => store.getState().startEditing({ mode: "create-folder", targetPath: folderPath }),
    },
  ];

  if (target) {
    if (!target.isFolder) {
      items.unshift({ label: "開く", action: () => onOpenFile?.(target) });
    }
    items.push(
      {
        label: "名前を変更",
        action: () => store.getState().startEditing({ mode: "rename", targetPath: target.relativePath }),
      },
      { label: "コピー", action: () => store.getState().setClipboard(target.relativePath, false) },
      { label: "切り取り", action: () => store.getState().setClipboard(target.relativePath, true) },
    );
  }

  items.push({
    label: "貼り付け",
    disabled: !clipboard,
    action: () => store.getState().paste(folderPath),
  });

  if (target) {
    items.push(
      { label: "パスをコピー", action: () => store.getState().copyPathToClipboard(target.relativePath) },
      { label: "エクスプローラーで表示", action: () => store.getState().reveal(target.relativePath) },
      { label: "既定のアプリで開く", action: () => store.getState().openExternal(target.relativePath) },
      {
        label: "削除（ごみ箱へ移動）",
        danger: true,
        action: () => store.getState().removeEntry(target.relativePath),
      },
    );
  }

  items.push({ label: "更新", action: () => store.getState().refresh() });

  const maxY = typeof window !== "undefined" ? window.innerHeight - items.length * 30 - 16 : menu.y;
  return (
    <div
      ref={ref}
      className="context-menu"
      role="menu"
      style={{ left: menu.x, top: Math.min(menu.y, Math.max(8, maxY)) }}
    >
      {items.map((item) => (
        <button
          key={item.label}
          type="button"
          role="menuitem"
          className={`context-menu__item${item.danger ? " context-menu__item--danger" : ""}`}
          disabled={item.disabled}
          onClick={() => {
            onClose();
            item.action();
          }}
        >
          {item.label}
        </button>
      ))}
    </div>
  );
}
