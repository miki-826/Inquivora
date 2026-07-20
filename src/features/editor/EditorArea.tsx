import { DiffEditor, Editor } from "@monaco-editor/react";
import { listen } from "@tauri-apps/api/event";
import { Eye, EyeOff, Pin, PinOff, X } from "lucide-react";
import { useEffect } from "react";
import { PanePlaceholder } from "../../components/common/PanePlaceholder";
import { useEditorStore } from "../../stores/useEditorStore";
import { useWorkspaceStore } from "../../stores/useWorkspaceStore";
import type { EditorTab } from "./editorModel";
import { MarkdownPreview } from "./MarkdownPreview";
import { MediaView } from "./MediaView";
import "./monacoSetup";

function TabBar() {
  const tabs = useEditorStore((s) => s.tabs);
  const activeTabId = useEditorStore((s) => s.activeTabId);
  const saveErrors = useEditorStore((s) => s.saveErrors);
  const store = useEditorStore;

  if (tabs.length === 0) return null;
  return (
    <div className="editor-tabs" role="tablist" aria-label="開いているファイル">
      {tabs.map((tab) => {
        const isActive = tab.id === activeTabId;
        const hasError = saveErrors[tab.id] !== undefined;
        return (
          <div
            key={tab.id}
            role="tab"
            aria-selected={isActive}
            tabIndex={0}
            draggable
            className={[
              "editor-tab",
              isActive ? "editor-tab--active" : "",
              hasError ? "editor-tab--error" : "",
            ]
              .filter(Boolean)
              .join(" ")}
            title={saveErrors[tab.id] ?? tab.path}
            onDragStart={(e) => {
              e.dataTransfer.setData("text/inquivora-tab", tab.id);
              e.dataTransfer.effectAllowed = "move";
            }}
            onDragOver={(e) => {
              if (e.dataTransfer.types.includes("text/inquivora-tab")) {
                e.preventDefault();
              }
            }}
            onDrop={(e) => {
              const fromId = e.dataTransfer.getData("text/inquivora-tab");
              if (fromId) {
                e.preventDefault();
                store.getState().reorderTab(fromId, tab.id);
              }
            }}
            onClick={() => store.getState().activateTab(tab.id)}
            onKeyDown={(e) => {
              if (e.key === "Enter") store.getState().activateTab(tab.id);
            }}
            onAuxClick={(e) => {
              if (e.button === 1) store.getState().closeTab(tab.id);
            }}
          >
            {tab.isPinned && <Pin size={11} aria-label="ピン留め済み" />}
            <span className="editor-tab__name">{tab.name}</span>
            {tab.isDirty && <span className="editor-tab__dirty" aria-label="未保存" />}
            <button
              type="button"
              className="editor-tab__close"
              aria-label={`${tab.name} を閉じる`}
              onClick={(e) => {
                e.stopPropagation();
                store.getState().closeTab(tab.id);
              }}
            >
              <X size={12} aria-hidden />
            </button>
          </div>
        );
      })}
    </div>
  );
}

function EditorToolbar({ tab }: { tab: EditorTab }) {
  const previewVisible = useEditorStore((s) => Boolean(s.previewVisible[tab.id]));
  const readMode = useEditorStore((s) => s.readModes[tab.id] ?? "normal");
  const saveError = useEditorStore((s) => s.saveErrors[tab.id]);
  const store = useEditorStore;

  return (
    <div className="editor-toolbar">
      <span className="editor-toolbar__path" title={tab.path}>
        {tab.path}
      </span>
      {readMode !== "normal" && (
        <span className="editor-toolbar__badge">
          {readMode === "preview" ? "先頭・末尾プレビュー（100MB超）" : "読み取り専用（10MB超）"}
        </span>
      )}
      {saveError && (
        <span className="editor-toolbar__badge editor-toolbar__badge--error" role="alert">
          保存失敗: {saveError}
        </span>
      )}
      <div className="editor-toolbar__actions">
        {tab.language === "markdown" && (
          <button
            type="button"
            title={previewVisible ? "プレビューを閉じる" : "Markdownプレビュー"}
            aria-label={previewVisible ? "プレビューを閉じる" : "Markdownプレビュー"}
            onClick={() => store.getState().togglePreview(tab.id)}
          >
            {previewVisible ? <EyeOff size={14} aria-hidden /> : <Eye size={14} aria-hidden />}
          </button>
        )}
        <button
          type="button"
          title={tab.isPinned ? "ピン留めを外す" : "ピン留め"}
          aria-label={tab.isPinned ? "ピン留めを外す" : "ピン留め"}
          onClick={() => store.getState().togglePin(tab.id)}
        >
          {tab.isPinned ? <PinOff size={14} aria-hidden /> : <Pin size={14} aria-hidden />}
        </button>
      </div>
    </div>
  );
}

function ConflictDialog() {
  const conflict = useEditorStore((s) => s.conflict);
  const tabs = useEditorStore((s) => s.tabs);
  const contents = useEditorStore((s) => s.contents);
  const store = useEditorStore;

  if (!conflict) return null;
  const tab = tabs.find((t) => t.id === conflict.tabId);
  if (!tab) return null;

  return (
    <div className="conflict-overlay" role="dialog" aria-modal="true" aria-label="外部変更の競合">
      <div className={`conflict-dialog${conflict.showDiff ? " conflict-dialog--wide" : ""}`}>
        <h2>外部変更を検出しました</h2>
        <p>
          「{tab.name}」がアプリ外で変更されましたが、未保存の編集があります。どうしますか？
        </p>
        {conflict.showDiff && conflict.diskContent !== null && (
          <div className="conflict-dialog__diff">
            <div className="conflict-dialog__diff-labels">
              <span>ディスク上の内容</span>
              <span>現在の編集内容</span>
            </div>
            <DiffEditor
              original={conflict.diskContent}
              modified={contents[tab.id] ?? ""}
              language={tab.language}
              options={{ readOnly: true, renderSideBySide: true, minimap: { enabled: false } }}
              height="100%"
            />
          </div>
        )}
        <div className="conflict-dialog__actions">
          <button type="button" onClick={() => store.getState().resolveConflictReload()}>
            外部内容を読み込む
          </button>
          <button type="button" onClick={() => store.getState().resolveConflictOverwrite()}>
            現在の内容で上書き
          </button>
          {!conflict.showDiff && (
            <button type="button" onClick={() => store.getState().resolveConflictShowDiff()}>
              差分を表示
            </button>
          )}
          <button type="button" onClick={() => store.getState().resolveConflictSaveAs()}>
            別名保存
          </button>
        </div>
      </div>
    </div>
  );
}

function ActivePane() {
  const tabs = useEditorStore((s) => s.tabs);
  const activeTabId = useEditorStore((s) => s.activeTabId);
  const contents = useEditorStore((s) => s.contents);
  const readModes = useEditorStore((s) => s.readModes);
  const previewVisible = useEditorStore((s) => s.previewVisible);
  const store = useEditorStore;

  const tab = tabs.find((t) => t.id === activeTabId);
  if (!tab) {
    return (
      <PanePlaceholder
        title="ワークスペース"
        description="ファイルツリーからファイルを選択すると、ここで編集できます"
      />
    );
  }

  if (tab.viewType === "image" || tab.viewType === "pdf" || tab.viewType === "audio" || tab.viewType === "video") {
    return <MediaView key={tab.path} tab={tab} />;
  }

  const content = contents[tab.id] ?? "";
  const readMode = readModes[tab.id] ?? "normal";
  const showPreview = Boolean(previewVisible[tab.id]) && tab.language === "markdown";

  return (
    <div className="editor-pane">
      <EditorToolbar tab={tab} />
      <div className="editor-pane__body">
        <div className="editor-pane__monaco">
          <Editor
            path={tab.path}
            language={tab.language}
            value={content}
            onChange={(value) => {
              if (value !== undefined) store.getState().updateContent(tab.id, value);
            }}
            onMount={(editor) => {
              editor.setPosition({ lineNumber: tab.cursorLine, column: tab.cursorColumn });
              editor.onDidChangeCursorPosition((e) => {
                store.getState().setCursor(tab.id, e.position.lineNumber, e.position.column);
              });
            }}
            options={{
              readOnly: readMode !== "normal",
              wordWrap: "on",
              minimap: { enabled: false },
              automaticLayout: true,
              renderWhitespace: "none",
              fontSize: 14,
              mouseWheelZoom: true,
            }}
          />
        </div>
        {showPreview && <MarkdownPreview content={content} />}
      </div>
    </div>
  );
}

export function EditorArea() {
  const workspaceId = useWorkspaceStore((s) => s.workspace?.id ?? null);

  useEffect(() => {
    useEditorStore.getState().closeAllTabs();
    if (workspaceId) {
      useEditorStore.getState().restoreTabs(workspaceId);
    }
  }, [workspaceId]);

  useEffect(() => {
    const unlisten = listen<{ paths: string[] }>("file:external-changed", (event) => {
      useEditorStore.getState().handleExternalChanges(event.payload.paths);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (!e.ctrlKey || e.key.toLowerCase() !== "s") return;
      e.preventDefault();
      if (e.shiftKey) {
        useEditorStore.getState().saveAllTabs();
      } else {
        useEditorStore.getState().saveActiveTab();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  return (
    <div className="editor-area">
      <TabBar />
      <div className="editor-area__content">
        <ActivePane />
      </div>
      <ConflictDialog />
    </div>
  );
}
