import { ThreePaneLayout } from "../../components/layout/ThreePaneLayout";
import { PanePlaceholder } from "../../components/common/PanePlaceholder";
import { EditorArea } from "../editor/EditorArea";
import { useEditorStore } from "../../stores/useEditorStore";
import { FileTree } from "./FileTree";
import { useRestoreWorkspaceOnMount } from "./useRestoreWorkspaceOnMount";

export function WorkspacePage() {
  useRestoreWorkspaceOnMount();
  const openFile = useEditorStore((s) => s.openFile);

  return (
    <ThreePaneLayout
      left={<FileTree onOpenFile={openFile} />}
      right={<PanePlaceholder title="AI・会議パネル" description="Phase 5以降で実装予定" />}
    >
      <EditorArea />
    </ThreePaneLayout>
  );
}
