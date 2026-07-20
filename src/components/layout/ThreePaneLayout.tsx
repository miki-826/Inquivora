import { useState, type ReactNode } from "react";
import { PanelLeftClose, PanelLeftOpen, PanelRightClose, PanelRightOpen } from "lucide-react";
import { useSettingsStore } from "../../stores/useSettingsStore";
import { ResizeHandle } from "./ResizeHandle";

type ThreePaneLayoutProps = {
  left?: ReactNode;
  right?: ReactNode;
  children: ReactNode;
  leftLabel?: string;
  rightLabel?: string;
};

export function ThreePaneLayout({
  left,
  right,
  children,
  leftLabel = "左パネル",
  rightLabel = "右パネル",
}: ThreePaneLayoutProps) {
  const leftWidth = useSettingsStore((s) => s.leftSidebarWidth);
  const rightWidth = useSettingsStore((s) => s.rightSidebarWidth);
  const setLeftWidth = useSettingsStore((s) => s.setLeftSidebarWidth);
  const setRightWidth = useSettingsStore((s) => s.setRightSidebarWidth);
  const [leftOpen, setLeftOpen] = useState(true);
  const [rightOpen, setRightOpen] = useState(true);

  return (
    <div className="three-pane">
      {left && leftOpen && (
        <>
          <aside className="three-pane__left" style={{ width: leftWidth }}>
            <div className="three-pane__content">{left}</div>
            <button
              type="button"
              className="three-pane__toggle three-pane__toggle--left"
              onClick={() => setLeftOpen(false)}
              aria-label={`${leftLabel}を閉じる`}
              title={`${leftLabel}を閉じる`}
            >
              <PanelLeftClose size={16} aria-hidden="true" />
            </button>
          </aside>
          <ResizeHandle side="left" width={leftWidth} onResize={setLeftWidth} />
        </>
      )}
      {left && !leftOpen && (
        <button
          type="button"
          className="three-pane__restore three-pane__restore--left"
          onClick={() => setLeftOpen(true)}
          aria-label={`${leftLabel}を開く`}
          title={`${leftLabel}を開く`}
        >
          <PanelLeftOpen size={17} aria-hidden="true" />
        </button>
      )}
      <section className="three-pane__center">{children}</section>
      {right && !rightOpen && (
        <button
          type="button"
          className="three-pane__restore three-pane__restore--right"
          onClick={() => setRightOpen(true)}
          aria-label={`${rightLabel}を開く`}
          title={`${rightLabel}を開く`}
        >
          <PanelRightOpen size={17} aria-hidden="true" />
        </button>
      )}
      {right && rightOpen && (
        <>
          <ResizeHandle side="right" width={rightWidth} onResize={setRightWidth} />
          <aside className="three-pane__right" style={{ width: rightWidth }}>
            <div className="three-pane__content">{right}</div>
            <button
              type="button"
              className="three-pane__toggle three-pane__toggle--right"
              onClick={() => setRightOpen(false)}
              aria-label={`${rightLabel}を閉じる`}
              title={`${rightLabel}を閉じる`}
            >
              <PanelRightClose size={16} aria-hidden="true" />
            </button>
          </aside>
        </>
      )}
    </div>
  );
}
