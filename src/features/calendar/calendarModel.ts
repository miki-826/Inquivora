import { formatInTimeZone } from "date-fns-tz";
import type { EventInput } from "../../services/events";
import {
  TOKYO_TZ,
  TASK_COLOR_VALUES,
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
  backgroundColor?: string;
  borderColor?: string;
  textColor?: string;
  extendedProps: {
    kind: "event" | "task";
    sourceId: string;
    completed?: boolean;
  };
};

export type EventRepeat = "none" | "daily" | "weekly";
export type CalendarItemFilter = "all" | "event" | "task";

export type ScreenBounds = { left: number; right: number; top: number; bottom: number };

export function isPointInsideBounds(
  x: number,
  y: number,
  bounds: ScreenBounds,
): boolean {
  return x >= bounds.left && x <= bounds.right && y >= bounds.top && y <= bounds.bottom;
}

export function nextCalendarItemFilter(
  current: CalendarItemFilter,
  selected: Exclude<CalendarItemFilter, "all">,
): CalendarItemFilter {
  return current === selected ? "all" : selected;
}

/** 1件の予定入力を日次・週次の独立した予定へ展開する。 */
export function buildRecurringEventInputs(
  input: EventInput,
  repeat: EventRepeat,
  count: number,
): EventInput[] {
  if (repeat === "none") return [input];
  const normalizedCount = Number.isFinite(count)
    ? Math.max(1, Math.min(100, Math.trunc(count)))
    : 1;
  const stepDays = repeat === "daily" ? 1 : 7;
  const shiftIso = (value: string, days: number) =>
    new Date(Date.parse(value) + days * 86_400_000).toISOString();
  return Array.from({ length: normalizedCount }, (_, index) => {
    const days = index * stepDays;
    return {
      ...input,
      startAtUtc: shiftIso(input.startAtUtc, days),
      endAtUtc: input.endAtUtc ? shiftIso(input.endAtUtc, days) : input.endAtUtc,
    };
  });
}

/** 日付文字列（yyyy-MM-dd）を日数分ずらす */
export function shiftDateString(dateString: string, days: number): string {
  const ms = Date.parse(`${dateString}T00:00:00Z`) + days * 86_400_000;
  return new Date(ms).toISOString().slice(0, 10);
}

/** 予定をFullCalendar入力へ変換する。終日はTokyoの日付文字列（endは排他的）で表す */
export function eventToCalendarInput(event: EventRecord): CalendarInput {
  const input: CalendarInput = {
    id: `event:${event.id}`,
    title: `予定 · ${event.title}`,
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
    title: `タスク · ${task.title}`,
    start: allDay ? tokyoDateString(task.dueAtUtc) : task.dueAtUtc,
    allDay,
    classNames: completed ? ["cal-task", "cal-task--completed"] : ["cal-task"],
    backgroundColor: TASK_COLOR_VALUES[task.color],
    borderColor: TASK_COLOR_VALUES[task.color],
    textColor: "#ffffff",
    extendedProps: { kind: "task", sourceId: task.id, ...(completed ? { completed } : {}) },
  };
}

/** 予定＋タスクをカレンダー表示用へまとめる（完了タスクは設定で除外） */
export function buildCalendarInputs(
  events: EventRecord[],
  tasks: Task[],
  showCompletedTasks: boolean,
  filter: CalendarItemFilter = "all",
): CalendarInput[] {
  const eventInputs = filter === "task" ? [] : events.map(eventToCalendarInput);
  if (filter === "event") return eventInputs;
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
