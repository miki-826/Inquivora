import { formatInTimeZone, fromZonedTime } from "date-fns-tz";
import { Trash2 } from "lucide-react";
import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { PanePlaceholder } from "../../components/common/PanePlaceholder";
import { ReminderSection } from "../notifications/ReminderSection";
import type { EventInput, EventPatch } from "../../services/events";
import { useCalendarStore } from "../../stores/useCalendarStore";
import { useTaskStore } from "../../stores/useTaskStore";
import {
  PRIORITY_LABELS,
  STATUS_LABELS,
  TOKYO_TZ,
  formatDueLabel,
  tokyoDateString,
} from "../tasks/taskModel";
import { formatEventRange, shiftDateString, type EventRecord } from "./calendarModel";

export type EventDraft = {
  title: string;
  allDay: boolean;
  startDate: string;
  startTime: string;
  endDate: string;
  endTime: string;
  location: string;
  description: string;
};

export type CalendarSelection =
  | { type: "draft"; draft: EventDraft }
  | { type: "event"; id: string }
  | { type: "task"; id: string }
  | null;

function draftFromEvent(event: EventRecord): EventDraft {
  if (event.allDay) {
    const startDate = tokyoDateString(event.startAtUtc);
    return {
      title: event.title,
      allDay: true,
      startDate,
      startTime: "",
      endDate: event.endAtUtc ? shiftDateString(tokyoDateString(event.endAtUtc), -1) : startDate,
      endTime: "",
      location: event.location ?? "",
      description: event.description ?? "",
    };
  }
  return {
    title: event.title,
    allDay: false,
    startDate: formatInTimeZone(event.startAtUtc, TOKYO_TZ, "yyyy-MM-dd"),
    startTime: formatInTimeZone(event.startAtUtc, TOKYO_TZ, "HH:mm"),
    endDate: event.endAtUtc ? formatInTimeZone(event.endAtUtc, TOKYO_TZ, "yyyy-MM-dd") : "",
    endTime: event.endAtUtc ? formatInTimeZone(event.endAtUtc, TOKYO_TZ, "HH:mm") : "",
    location: event.location ?? "",
    description: event.description ?? "",
  };
}

function draftToPayload(draft: EventDraft): EventInput | null {
  if (!draft.title.trim() || !draft.startDate) return null;
  if (draft.allDay) {
    const endDate = draft.endDate || draft.startDate;
    return {
      title: draft.title.trim(),
      allDay: true,
      startAtUtc: fromZonedTime(`${draft.startDate}T00:00:00`, TOKYO_TZ).toISOString(),
      endAtUtc:
        endDate !== draft.startDate
          ? fromZonedTime(`${shiftDateString(endDate, 1)}T00:00:00`, TOKYO_TZ).toISOString()
          : null,
      location: draft.location.trim() || null,
      description: draft.description.trim() || null,
    };
  }
  const endDate = draft.endDate || draft.startDate;
  return {
    title: draft.title.trim(),
    allDay: false,
    startAtUtc: fromZonedTime(
      `${draft.startDate}T${draft.startTime || "00:00"}:00`,
      TOKYO_TZ,
    ).toISOString(),
    endAtUtc: draft.endTime
      ? fromZonedTime(`${endDate}T${draft.endTime}:00`, TOKYO_TZ).toISOString()
      : null,
    location: draft.location.trim() || null,
    description: draft.description.trim() || null,
  };
}

function EventForm({
  heading,
  initial,
  submitLabel,
  meta,
  onSubmit,
  onDelete,
  onCancel,
}: {
  heading: string;
  initial: EventDraft;
  submitLabel: string;
  meta?: string;
  onSubmit: (payload: EventInput) => void;
  onDelete?: () => void;
  onCancel: () => void;
}) {
  const [draft, setDraft] = useState(initial);
  const patch = (partial: Partial<EventDraft>) => setDraft((d) => ({ ...d, ...partial }));

  return (
    <form
      className="event-form"
      onSubmit={(e) => {
        e.preventDefault();
        const payload = draftToPayload(draft);
        if (payload) onSubmit(payload);
      }}
    >
      <h2 className="pane-title">{heading}</h2>
      {meta && <p className="event-form__meta">{meta}</p>}
      <input
        type="text"
        value={draft.title}
        placeholder="タイトル"
        aria-label="タイトル"
        onChange={(e) => patch({ title: e.target.value })}
      />
      <label className="event-form__allday">
        <input
          type="checkbox"
          checked={draft.allDay}
          onChange={(e) => patch({ allDay: e.target.checked })}
        />
        終日
      </label>
      <div className="event-form__grid">
        <label>
          開始日
          <input
            type="date"
            value={draft.startDate}
            onChange={(e) => patch({ startDate: e.target.value })}
          />
        </label>
        {!draft.allDay && (
          <label>
            開始時刻
            <input
              type="time"
              value={draft.startTime}
              onChange={(e) => patch({ startTime: e.target.value })}
            />
          </label>
        )}
        <label>
          終了日
          <input
            type="date"
            value={draft.endDate}
            onChange={(e) => patch({ endDate: e.target.value })}
          />
        </label>
        {!draft.allDay && (
          <label>
            終了時刻
            <input
              type="time"
              value={draft.endTime}
              onChange={(e) => patch({ endTime: e.target.value })}
            />
          </label>
        )}
      </div>
      <label className="event-form__field">
        場所
        <input
          type="text"
          value={draft.location}
          onChange={(e) => patch({ location: e.target.value })}
        />
      </label>
      <label className="event-form__field">
        メモ
        <textarea
          rows={4}
          value={draft.description}
          onChange={(e) => patch({ description: e.target.value })}
        />
      </label>
      <div className="event-form__actions">
        <button type="submit" className="event-form__submit" disabled={!draft.title.trim()}>
          {submitLabel}
        </button>
        <button type="button" onClick={onCancel}>
          キャンセル
        </button>
        {onDelete && (
          <button type="button" className="event-form__delete" onClick={onDelete}>
            <Trash2 size={13} aria-hidden /> 削除
          </button>
        )}
      </div>
    </form>
  );
}

function TaskSummary({ taskId, onClose }: { taskId: string; onClose: () => void }) {
  const task = useCalendarStore((s) => s.tasks.find((t) => t.id === taskId));
  const selectTask = useTaskStore((s) => s.select);
  const navigate = useNavigate();

  if (!task) {
    return <PanePlaceholder title="タスク" description="タスクが見つかりません" />;
  }
  return (
    <div className="event-panel__task">
      <h2 className="pane-title">タスク</h2>
      <p className="event-panel__task-title">{task.title}</p>
      {task.dueAtUtc && <p>期日: {formatDueLabel(task.dueAtUtc, new Date())}</p>}
      <p>
        優先度: {PRIORITY_LABELS[task.priority]} ／ 状態: {STATUS_LABELS[task.status]}
      </p>
      {task.description && <p className="event-panel__task-memo">{task.description}</p>}
      <div className="event-form__actions">
        <button
          type="button"
          onClick={() => {
            selectTask(task.id);
            navigate("/tasks");
          }}
        >
          タスク画面で開く
        </button>
        <button type="button" onClick={onClose}>
          閉じる
        </button>
      </div>
    </div>
  );
}

export function EventPanel({
  selection,
  onClose,
}: {
  selection: CalendarSelection;
  onClose: () => void;
}) {
  const events = useCalendarStore((s) => s.events);
  const createEvent = useCalendarStore((s) => s.createEvent);
  const updateEvent = useCalendarStore((s) => s.updateEvent);
  const removeEvent = useCalendarStore((s) => s.removeEvent);

  if (!selection) {
    return (
      <PanePlaceholder
        title="予定詳細"
        description="日付をクリックすると予定を作成、予定をクリックすると編集できます"
      />
    );
  }

  if (selection.type === "task") {
    return <TaskSummary taskId={selection.id} onClose={onClose} />;
  }

  if (selection.type === "draft") {
    return (
      <EventForm
        heading="新しい予定"
        initial={selection.draft}
        submitLabel="作成"
        onCancel={onClose}
        onSubmit={(payload) => {
          void createEvent(payload).then((created) => {
            if (created) onClose();
          });
        }}
      />
    );
  }

  const event = events.find((e) => e.id === selection.id);
  if (!event) {
    return <PanePlaceholder title="予定詳細" description="予定が見つかりません" />;
  }
  return (
    <div className="event-panel__edit">
      <EventForm
        key={`${event.id}:${event.updatedAt}`}
        heading="予定を編集"
        initial={draftFromEvent(event)}
        submitLabel="保存"
        meta={formatEventRange(event)}
        onCancel={onClose}
        onSubmit={(payload) => {
          void updateEvent(event.id, payload as EventPatch).then((ok) => {
            if (ok) onClose();
          });
        }}
        onDelete={() => {
          void removeEvent(event.id).then(onClose);
        }}
      />
      <ReminderSection eventId={event.id} />
    </div>
  );
}
