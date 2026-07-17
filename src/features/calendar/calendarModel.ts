import type { Task } from "../tasks/taskModel";

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
export function eventToCalendarInput(_event: EventRecord): CalendarInput {
  throw new Error("未実装");
}

/** タスクをFullCalendar入力へ変換する（期日なしはnull）（§13.4） */
export function taskToCalendarInput(_task: Task): CalendarInput | null {
  throw new Error("未実装");
}

/** 予定＋タスクをカレンダー表示用へまとめる（完了タスクは設定で除外） */
export function buildCalendarInputs(
  _events: EventRecord[],
  _tasks: Task[],
  _showCompletedTasks: boolean,
): CalendarInput[] {
  throw new Error("未実装");
}

/** 詳細パネル用の日時表示（Asia/Tokyo） */
export function formatEventRange(_event: EventRecord): string {
  throw new Error("未実装");
}
