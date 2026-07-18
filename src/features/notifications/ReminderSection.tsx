import { Bell, Trash2 } from "lucide-react";
import { useEffect, useState } from "react";
import {
  createReminder,
  deleteReminder,
  listRemindersForTarget,
  type Reminder,
} from "../../services/reminders";
import { formatNotifyLabel } from "./notificationModel";

const STATUS_LABELS: Record<string, string> = {
  sent: "送信済み",
  cancelled: "取消",
  expired: "期限切れ",
};

/// §12.3 通知追加。タスクまたは予定のリマインダー一覧・追加・削除。
export function ReminderSection({ taskId, eventId }: { taskId?: string; eventId?: string }) {
  const [reminders, setReminders] = useState<Reminder[]>([]);
  const [draft, setDraft] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [refreshKey, setRefreshKey] = useState(0);

  useEffect(() => {
    let active = true;
    listRemindersForTarget({ taskId, eventId })
      .then((list) => {
        if (!active) return;
        setReminders(list);
        setError(null);
      })
      .catch((err) => {
        if (active) setError(err instanceof Error ? err.message : String(err));
      });
    return () => {
      active = false;
    };
  }, [taskId, eventId, refreshKey]);

  const reload = () => setRefreshKey((key) => key + 1);

  const add = async () => {
    if (!draft) return;
    try {
      await createReminder({ taskId, eventId, notifyAtUtc: new Date(draft).toISOString() });
      setDraft("");
      reload();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const remove = async (id: string) => {
    try {
      await deleteReminder(id);
      reload();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  return (
    <div className="reminder-section">
      <div className="reminder-section__title">
        <Bell size={13} aria-hidden /> 通知
      </div>
      {error && (
        <p className="reminder-section__error" role="alert">
          {error}
        </p>
      )}
      {reminders.length === 0 && <p className="reminder-section__empty">通知はありません</p>}
      <ul className="reminder-section__list">
        {reminders.map((reminder) => (
          <li key={reminder.id} className="reminder-section__item">
            <span
              className={
                reminder.status === "scheduled"
                  ? "reminder-section__time"
                  : "reminder-section__time reminder-section__time--inactive"
              }
            >
              {formatNotifyLabel(reminder.notifyAtUtc)}
            </span>
            {reminder.status !== "scheduled" && (
              <span className="reminder-section__status">
                {STATUS_LABELS[reminder.status] ?? reminder.status}
              </span>
            )}
            <button
              type="button"
              className="reminder-section__delete"
              aria-label="通知を削除"
              onClick={() => void remove(reminder.id)}
            >
              <Trash2 size={12} aria-hidden />
            </button>
          </li>
        ))}
      </ul>
      <div className="reminder-section__add">
        <input
          type="datetime-local"
          value={draft}
          aria-label="通知日時"
          onChange={(e) => setDraft(e.target.value)}
        />
        <button type="button" disabled={!draft} onClick={() => void add()}>
          追加
        </button>
      </div>
    </div>
  );
}
