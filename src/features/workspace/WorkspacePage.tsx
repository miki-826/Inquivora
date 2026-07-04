import { ThreePaneLayout } from "../../components/layout/ThreePaneLayout";
import { PanePlaceholder } from "../../components/common/PanePlaceholder";
import { FileTree } from "./FileTree";
import { useRestoreWorkspaceOnMount } from "./useRestoreWorkspaceOnMount";

export function WorkspacePage() {
  useRestoreWorkspaceOnMount();

  return (
    <ThreePaneLayout
      left={<FileTree />}
      right={<PanePlaceholder title="AI・会議パネル" description="Phase 5以降で実装予定" />}
    >
      <PanePlaceholder
        title="ワークスペース"
        description="ファイルを選択するとここにエディタが表示されます"
      />
    </ThreePaneLayout>
  );
}
