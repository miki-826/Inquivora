import { ThreePaneLayout } from "../../components/layout/ThreePaneLayout";
import { PanePlaceholder } from "../../components/common/PanePlaceholder";

export function CalendarPage() {
  return (
    <ThreePaneLayout right={<PanePlaceholder title="予定詳細" description="選択日の予定がここに表示されます" />}>
      <PanePlaceholder title="カレンダー" description="月・週・日カレンダー（Phase 3で実装予定）" />
    </ThreePaneLayout>
  );
}
