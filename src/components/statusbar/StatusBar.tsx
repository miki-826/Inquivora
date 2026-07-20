import { useEditorStore } from "../../stores/useEditorStore";
import packageInfo from "../../../package.json";

const ENCODING_LABELS: Record<string, string> = {
  utf8: "UTF-8",
  "utf8-bom": "UTF-8 BOM",
  utf16le: "UTF-16 LE",
  utf16be: "UTF-16 BE",
  shift_jis: "Shift_JIS",
};

export function StatusBar() {
  const activeTab = useEditorStore((s) => s.tabs.find((t) => t.id === s.activeTabId));
  const saveError = useEditorStore((s) =>
    s.activeTabId ? s.saveErrors[s.activeTabId] : undefined,
  );

  return (
    <footer className="status-bar">
      <span>{saveError ? `保存失敗: ${saveError}` : activeTab?.isDirty ? "未保存の変更" : "準備完了"}</span>
      <span className="status-bar__spacer" />
      {activeTab && (
        <>
          <span>
            行 {activeTab.cursorLine}, 列 {activeTab.cursorColumn}
          </span>
          <span>{ENCODING_LABELS[activeTab.encoding] ?? activeTab.encoding}</span>
          <span>{activeTab.lineEnding}</span>
        </>
      )}
      <span>Inquivora {packageInfo.version}</span>
    </footer>
  );
}
