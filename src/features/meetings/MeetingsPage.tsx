import { ThreePaneLayout } from "../../components/layout/ThreePaneLayout";
import { PanePlaceholder } from "../../components/common/PanePlaceholder";

export function MeetingsPage() {
  return (
    <ThreePaneLayout
      left={<PanePlaceholder title="議事録一覧" description="Phase 5以降で実装予定" />}
      right={<PanePlaceholder title="要約・決定事項・タスク候補" description="Phase 6で実装予定" />}
    >
      <PanePlaceholder title="議事録" description="関連ファイルがここに表示されます" />
    </ThreePaneLayout>
  );
}
