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
  TASK_COLOR_LABELS,
  TASK_COLOR_VALUES,
  TOKYO_TZ,
  buildDueAtUtc,
  splitDueAtUtc,
  tokyoDateString,
  type TaskColor,
  type TaskPriority,
  type TaskStatus,
} from "../tasks/taskModel";
import {
  buildRecurringEventInputs,
  formatEventRange,
  shiftDateString,
  type EventRecord,
  type EventRepeat,
} from "./calendarModel";
import "../tasks/taskColors.css";

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
  allowRecurrence,
  onSubmit,
  onDelete,
  onCancel,
}: {
  heading: string;
  initial: EventDraft;
  submitLabel: string;
  meta?: string;
  allowRecurrence?: boolean;
  onSubmit: (payload: EventInput, repeat: EventRepeat, repeatCount: number) => void;
  onDelete?: () => void;
  onCancel: () => void;
}) {
  const [draft, setDraft] = useState(initial);
  const [repeat, setRepeat] = useState<EventRepeat>("none");
  const [repeatCount, setRepeatCount] = useState(4);
  const patch = (partial: Partial<EventDraft>) => setDraft((d) => ({ ...d, ...partial }));

  return (
    <form
      className="event-form"
      onSubmit={(e) => {
        e.preventDefault();
        const payload = draftToPayload(draft);
        if (payload) onSubmit(payload, repeat, repeatCount);
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
      {allowRecurrence && (
        <div className="event-form__recurrence" aria-label="繰り返し設定">
          <label>
            繰り返し
            <select
              value={repeat}
              onChange={(event) => setRepeat(event.target.value as EventRepeat)}
            >
              <option value="none">繰り返さない</option>
              <option value="daily">毎日</option>
              <option value="weekly">毎週</option>
            </select>
          </label>
          {repeat !== "none" && (
            <label>
              作成回数
              <input
                type="number"
                min={2}
                max={100}
                value={repeatCount}
                onChange={(event) => setRepeatCount(Number(event.target.value))}
              />
            </label>
          )}
          {repeat !== "none" && (
            <p>初回を含めて{Math.max(2, Math.min(100, repeatCount || 2))}件を一括作成します</p>
          )}
        </div>
      )}
      <div className="event-form__actions">
        <button
          type="submit"
          className="event-form__submit button-primary"
          disabled={!draft.title.trim()}
          aria-label={submitLabel}
        >
          <span className="event-form__submit-label">{submitLabel}</span>
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

/// カレンダーで選択したタスクを、この予定詳細ペイン内で直接編集する。
function CalendarTaskEditor({ taskId, onClose }: { taskId: string; onClose: () => void }) {
  const task = useCalendarStore((s) => s.tasks.find((t) => t.id === taskId));
  const updateTask = useCalendarStore((s) => s.updateTask);
  const selectTask = useTaskStore((s) => s.select);
  const navigate = useNavigate();
  const initial = splitDueAtUtc(task?.dueAtUtc ?? null);
  const [dueDate, setDueDate] = useState(initial.date);
  const [dueTime, setDueTime] = useState(initial.time);

  if (!task) {
    return <PanePlaceholder title="タスク" description="タスクが見つかりません" />;
  }

  const commitDue = (date: string, time: string) => {
    setDueDate(date);
    setDueTime(time);
    void updateTask(task.id, { dueAtUtc: buildDueAtUtc(date, time) });
  };

  return (
    <div className="calendar-task-editor">
      <h2 className="pane-title">タスクを編集</h2>
      <input
        className="calendar-task-editor__title"
        type="text"
        defaultValue={task.title}
        aria-label="タスク名"
        onBlur={(event) => {
          const title = event.currentTarget.value.trim();
          if (title && title !== task.title) void updateTask(task.id, { title });
        }}
        onKeyDown={(event) => {
          if (event.key === "Enter") event.currentTarget.blur();
        }}
      />
      <div className="calendar-task-editor__grid">
        <label>
          状態
          <select
            value={task.status}
            onChange={(event) =>
              void updateTask(task.id, { status: event.target.value as TaskStatus })
            }
          >
            {(Object.keys(STATUS_LABELS) as TaskStatus[]).map((status) => (
              <option key={status} value={status}>
                {STATUS_LABELS[status]}
              </option>
            ))}
          </select>
        </label>
        <label>
          優先度
          <select
            value={task.priority}
            onChange={(event) =>
              void updateTask(task.id, { priority: event.target.value as TaskPriority })
            }
          >
            {(Object.keys(PRIORITY_LABELS) as TaskPriority[]).map((priority) => (
              <option key={priority} value={priority}>
                {PRIORITY_LABELS[priority]}
              </option>
            ))}
          </select>
        </label>
        <label>
          期日
          <input
            type="date"
            value={dueDate}
            onChange={(event) => commitDue(event.target.value, event.target.value ? dueTime : "")}
          />
        </label>
        <label>
          時刻
          <input
            type="time"
            value={dueTime}
            disabled={!dueDate}
            onChange={(event) => commitDue(dueDate, event.target.value)}
          />
        </label>
      </div>
      <fieldset className="task-color-picker calendar-task-editor__colors">
        <legend>カレンダーの色</legend>
        {(Object.keys(TASK_COLOR_LABELS) as TaskColor[]).map((color) => (
          <label key={color} title={TASK_COLOR_LABELS[color]}>
            <input
              type="radio"
              name={`calendar-task-color-${task.id}`}
              checked={task.color === color}
              onChange={() => void updateTask(task.id, { color })}
            />
            <span style={{ backgroundColor: TASK_COLOR_VALUES[color] }} aria-hidden />
            <span className="sr-only">{TASK_COLOR_LABELS[color]}</span>
          </label>
        ))}
      </fieldset>
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
  const createEvents = useCalendarStore((s) => s.createEvents);
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
    return (
      <CalendarTaskEditor key={selection.id} taskId={selection.id} onClose={onClose} />
    );
  }

  if (selection.type === "draft") {
    return (
      <EventForm
        heading="新しい予定"
        initial={selection.draft}
        submitLabel="作成"
        allowRecurrence
        onCancel={onClose}
        onSubmit={(payload, repeat, repeatCount) => {
          if (repeat === "none") {
            void createEvent(payload).then((created) => {
              if (created) onClose();
            });
            return;
          }
          const count = Number.isFinite(repeatCount)
            ? Math.max(2, Math.min(100, Math.trunc(repeatCount)))
            : 2;
          const inputs = buildRecurringEventInputs(payload, repeat, count);
          void createEvents(inputs).then((created) => {
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
