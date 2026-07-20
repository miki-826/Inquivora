import { ThreePaneLayout } from "../../components/layout/ThreePaneLayout";
import { EditorArea } from "../editor/EditorArea";
import { useEditorStore } from "../../stores/useEditorStore";
import { FileTree } from "./FileTree";
import { WorkspaceMeetingPanel } from "./WorkspaceMeetingPanel";
import { useRestoreWorkspaceOnMount } from "./useRestoreWorkspaceOnMount";

export function WorkspacePage() {
  useRestoreWorkspaceOnMount();
  const openFile = useEditorStore((s) => s.openFile);

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
