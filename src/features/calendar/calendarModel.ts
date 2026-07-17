import { formatInTimeZone } from "date-fns-tz";
import {
  TOKYO_TZ,
  isDateOnlyDue,
  tokyoDateString,
  type Task,
} from "../tasks/taskModel";

export type EventRecord = {
  id: string;
  title: string;
  description: string | null;
  startAtUtc: string;
  endAtUtc: string | null;
  timezone: string;
  allDay: boolean;
  eventType: string;
  recurrenceRule: string | null;
  meetingId: string | null;
  taskId: string | null;
  location: string | null;
  createdAt: string;
  updatedAt: string;
};

export type CalendarInput = {
  id: string;
  title: string;
  start: string;
  end?: string;
  allDay: boolean;
  classNames: string[];
  extendedProps: {
    kind: "event" | "task";
    sourceId: string;
    completed?: boolean;
  };
};

/** 予定をFullCalendar入力へ変換する。終日はTokyoの日付文字列（endは排他的）で表す */
export function eventToCalendarInput(event: EventRecord): CalendarInput {
  const input: CalendarInput = {
    id: `event:${event.id}`,
    title: event.title,
    start: event.allDay ? tokyoDateString(event.startAtUtc) : event.startAtUtc,
    allDay: event.allDay,
    classNames: ["cal-event"],
    extendedProps: { kind: "event", sourceId: event.id },
  };
  if (event.endAtUtc) {
    input.end = event.allDay ? tokyoDateString(event.endAtUtc) : event.endAtUtc;
  }
  return input;
}

/** タスクをFullCalendar入力へ変換する（期日なしはnull）（§13.4） */
export function taskToCalendarInput(task: Task): CalendarInput | null {
  if (!task.dueAtUtc) return null;
  const completed = task.status === "completed";
  const allDay = isDateOnlyDue(task.dueAtUtc);
  return {
    id: `task:${task.id}`,
    title: task.title,
    start: allDay ? tokyoDateString(task.dueAtUtc) : task.dueAtUtc,
    allDay,
    classNames: completed ? ["cal-task", "cal-task--completed"] : ["cal-task"],
    extendedProps: { kind: "task", sourceId: task.id, ...(completed ? { completed } : {}) },
  };
}

/** 予定＋タスクをカレンダー表示用へまとめる（完了タスクは設定で除外） */
export function buildCalendarInputs(
  events: EventRecord[],
  tasks: Task[],
  showCompletedTasks: boolean,
): CalendarInput[] {
  const eventInputs = events.map(eventToCalendarInput);
  const taskInputs = tasks
    .filter((task) => showCompletedTasks || task.status !== "completed")
    .map(taskToCalendarInput)
    .filter((input): input is CalendarInput => input !== null);
  return [...eventInputs, ...taskInputs];
}

/** 詳細パネル用の日時表示（Asia/Tokyo） */
export function formatEventRange(event: EventRecord): string {
  const startDate = formatInTimeZone(event.startAtUtc, TOKYO_TZ, "yyyy年M月d日");
  if (event.allDay) {
    if (event.endAtUtc && tokyoDateString(event.endAtUtc) !== tokyoDateString(event.startAtUtc)) {
      return `${startDate}〜${formatInTimeZone(event.endAtUtc, TOKYO_TZ, "yyyy年M月d日")} 終日`;
    }
    return `${startDate} 終日`;
  }
  const startTime = formatInTimeZone(event.startAtUtc, TOKYO_TZ, "HH:mm");
  if (!event.endAtUtc) return `${startDate} ${startTime}`;
  const sameDay = tokyoDateString(event.endAtUtc) === tokyoDateString(event.startAtUtc);
  const end = sameDay
    ? formatInTimeZone(event.endAtUtc, TOKYO_TZ, "HH:mm")
    : `${formatInTimeZone(event.endAtUtc, TOKYO_TZ, "yyyy年M月d日")} ${formatInTimeZone(event.endAtUtc, TOKYO_TZ, "HH:mm")}`;
  return `${startDate} ${startTime}〜${end}`;
}
