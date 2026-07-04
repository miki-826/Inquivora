import { ThreePaneLayout } from "../../components/layout/ThreePaneLayout";
import { PanePlaceholder } from "../../components/common/PanePlaceholder";

export function SearchPage() {
  return (
    <ThreePaneLayout>
      <PanePlaceholder title="全文検索" description="Phase 7で実装予定" />
    </ThreePaneLayout>
  );
}
