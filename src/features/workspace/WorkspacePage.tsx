import { ThreePaneLayout } from "../../components/layout/ThreePaneLayout";
import { PanePlaceholder } from "../../components/common/PanePlaceholder";

export function WorkspacePage() {
  return (
    <ThreePaneLayout
      left={<PanePlaceholder title="ファイルツリー" description="Phase 2で実装予定" />}
      right={<PanePlaceholder title="AI・会議パネル" description="Phase 5以降で実装予定" />}
    >
      <PanePlaceholder
        title="ワークスペース"
        description="フォルダを開くとここにエディタが表示されます（Phase 2で実装予定）"
      />
    </ThreePaneLayout>
  );
}
