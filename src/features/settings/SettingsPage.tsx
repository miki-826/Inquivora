import { ThreePaneLayout } from "../../components/layout/ThreePaneLayout";
import { PanePlaceholder } from "../../components/common/PanePlaceholder";

export function SettingsPage() {
  return (
    <ThreePaneLayout
      left={
        <div className="pane-section">
          <div className="pane-section__title">設定</div>
          <div>一般 / エディタ / 会議 / AI・API / 通知</div>
        </div>
      }
    >
      <PanePlaceholder title="設定" description="各設定画面は今後のPhaseで実装予定" />
    </ThreePaneLayout>
  );
}
