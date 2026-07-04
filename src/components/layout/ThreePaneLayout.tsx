import type { ReactNode } from "react";
import { useSettingsStore } from "../../stores/useSettingsStore";
import { ResizeHandle } from "./ResizeHandle";

type ThreePaneLayoutProps = {
  left?: ReactNode;
  right?: ReactNode;
  children: ReactNode;
};

export function ThreePaneLayout({ left, right, children }: ThreePaneLayoutProps) {
  const leftWidth = useSettingsStore((s) => s.leftSidebarWidth);
  const rightWidth = useSettingsStore((s) => s.rightSidebarWidth);
  const setLeftWidth = useSettingsStore((s) => s.setLeftSidebarWidth);
  const setRightWidth = useSettingsStore((s) => s.setRightSidebarWidth);

  return (
    <div className="three-pane">
      {left && (
        <>
          <aside className="three-pane__left" style={{ width: leftWidth }}>
            {left}
          </aside>
          <ResizeHandle side="left" width={leftWidth} onResize={setLeftWidth} />
        </>
      )}
      <section className="three-pane__center">{children}</section>
      {right && (
        <>
          <ResizeHandle side="right" width={rightWidth} onResize={setRightWidth} />
          <aside className="three-pane__right" style={{ width: rightWidth }}>
            {right}
          </aside>
        </>
      )}
    </div>
  );
}
