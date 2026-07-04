import { ThreePaneLayout } from "../../components/layout/ThreePaneLayout";
import { PanePlaceholder } from "../../components/common/PanePlaceholder";

export function TasksPage() {
  return (
    <ThreePaneLayout
      left={<PanePlaceholder title="フィルター" description="今日・今週・期限切れ・完了" />}
      right={<PanePlaceholder title="タスク詳細" description="Phase 3で実装予定" />}
    >
      <PanePlaceholder title="タスク" description="期日昇順のタスク一覧（Phase 3で実装予定）" />
    </ThreePaneLayout>
  );
}
