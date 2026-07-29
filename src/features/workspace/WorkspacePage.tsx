import { useEffect, useRef } from "react";
import { useLocation, useNavigate } from "react-router-dom";
import { ThreePaneLayout } from "../../components/layout/ThreePaneLayout";
import { EditorArea } from "../editor/EditorArea";
import { useEditorStore } from "../../stores/useEditorStore";
import { useWorkspaceStore } from "../../stores/useWorkspaceStore";
import { FileTree } from "./FileTree";
import { WorkspaceMeetingPanel } from "./WorkspaceMeetingPanel";
import { useRestoreWorkspaceOnMount } from "./useRestoreWorkspaceOnMount";

export function WorkspacePage() {
  useRestoreWorkspaceOnMount();
  const location = useLocation();
  const navigate = useNavigate();
  const openFile = useEditorStore((s) => s.openFile);
  const openPath = useEditorStore((s) => s.openPath);
  const revealPath = useWorkspaceStore((s) => s.revealPath);
  const workspace = useWorkspaceStore((s) => s.workspace);
  const handledLocationKey = useRef<string | null>(null);
  const requestedPath = (location.state as { openPath?: unknown } | null)?.openPath;

  useEffect(() => {
    if (
      typeof requestedPath !== "string" ||
      !workspace ||
      handledLocationKey.current === location.key
    ) {
      return;
    }
    handledLocationKey.current = location.key;
    void (async () => {
      await revealPath(requestedPath);
      await openPath(requestedPath);
      navigate("/workspace", { replace: true, state: null });
    })();
  }, [location.key, navigate, openPath, requestedPath, revealPath, workspace]);

  return (
    <ThreePaneLayout
      left={<FileTree onOpenFile={openFile} />}
      right={<WorkspaceMeetingPanel />}
      leftLabel="ファイルツリー"
      rightLabel="会議パネル"
    >
      <EditorArea />
    </ThreePaneLayout>
  );
}
