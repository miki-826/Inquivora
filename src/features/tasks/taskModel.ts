import { formatInTimeZone, fromZonedTime } from "date-fns-tz";

export type TaskPriority = "high" | "medium" | "low";
export type TaskStatus = "todo" | "in_progress" | "on_hold" | "completed" | "cancelled";
export type TaskColor = "blue" | "indigo" | "violet" | "pink" | "red" | "orange" | "green" | "teal";

export type Task = {
  id: string;
  title: string;
  description: string | null;
  dueAtUtc: string | null;
  timezone: string;
  priority: TaskPriority;
  color: TaskColor;
  status: TaskStatus;
  assignee: string | null;
  projectName: string | null;
  meetingId: string | null;
  linkedFilePath: string | null;
  createdAt: string;
  updatedAt: string;
  completedAt: string | null;
};

export type TaskPreset =
  | "all"
  | "open"
  | "inProgress"
  | "completed"
  | "today"
  | "thisWeek"
  | "overdue";

export type TaskFilter = {
  preset?: TaskPreset;
  priority?: TaskPriority;
  projectName?: string;
  assignee?: string;
};

export type DueGroup =
  | "overdue"
  | "today"
  | "tomorrow"
  | "thisWeek"
  | "later"
  | "none"
  | "completed";

export const TOKYO_TZ = "Asia/Tokyo";

export const DUE_GROUP_LABELS: Record<DueGroup, string> = {
  overdue: "期限切れ",
  today: "今日",
  tomorrow: "明日",
  thisWeek: "今週",
  later: "来週以降",
  none: "期日なし",
  completed: "完了",
};

export const PRIORITY_LABELS: Record<TaskPriority, string> = {
  high: "高",
  medium: "中",
  low: "低",
};

export const TASK_COLOR_LABELS: Record<TaskColor, string> = {
  blue: "青",
  indigo: "藍",
  violet: "紫",
  pink: "桃",
  red: "赤",
  orange: "橙",
  green: "緑",
  teal: "青緑",
};

export const TASK_COLOR_VALUES: Record<TaskColor, string> = {
  blue: "#2563eb",
  indigo: "#4f46e5",
  violet: "#7c3aed",
  pink: "#db2777",
  red: "#dc2626",
  orange: "#ea580c",
  green: "#16a34a",
  teal: "#0f766e",
};

export const STATUS_LABELS: Record<TaskStatus, string> = {
  todo: "未着手",
  in_progress: "進行中",
  on_hold: "保留",
  completed: "完了",
  cancelled: "中止",
};

export function tokyoDateString(instant: Date | string): string {
  return formatInTimeZone(instant, TOKYO_TZ, "yyyy-MM-dd");
}

function dayNumber(dateString: string): number {
  return Date.parse(`${dateString}T00:00:00Z`) / 86_400_000;
}

function mondayStartOffset(dateString: string): number {
  const weekday = new Date(`${dateString}T00:00:00Z`).getUTCDay();
  return (weekday + 6) % 7;
}

/** 期日がAsia/Tokyoの0時ちょうど（=日付のみ）かを判定する（§13.4） */
export function isDateOnlyDue(dueAtUtc: string): boolean {
  return formatInTimeZone(dueAtUtc, TOKYO_TZ, "HH:mm:ss") === "00:00:00";
}

/** タスクの表示グループを判定する（§12.1の表示順） */
export function dueGroupOf(task: Task, now: Date): DueGroup {
  if (task.status === "completed") return "completed";
  if (!task.dueAtUtc) return "none";
  const today = dayNumber(tokyoDateString(now));
  const dueDay = dayNumber(tokyoDateString(task.dueAtUtc));
  if (isDateOnlyDue(task.dueAtUtc)) {
    if (dueDay < today) return "overdue";
  } else if (Date.parse(task.dueAtUtc) < now.getTime()) {
    return "overdue";
  }
  if (dueDay === today) return "today";
  if (dueDay === today + 1) return "tomorrow";
  const weekStart = today - mondayStartOffset(tokyoDateString(now));
  if (dueDay >= weekStart && dueDay < weekStart + 7) return "thisWeek";
  return "later";
}

const GROUP_ORDER: DueGroup[] = [
  "overdue",
  "today",
  "tomorrow",
  "thisWeek",
  "later",
  "none",
  "completed",
];

/** §12.1の表示順でグループ化する（空グループは除く） */
export function groupTasks(
  tasks: Task[],
  now: Date,
): { group: DueGroup; label: string; tasks: Task[] }[] {
  const byGroup = new Map<DueGroup, Task[]>();
  for (const task of tasks) {
    const group = dueGroupOf(task, now);
    const list = byGroup.get(group);
    if (list) {
      list.push(task);
    } else {
      byGroup.set(group, [task]);
    }
  }
  return GROUP_ORDER.filter((group) => byGroup.has(group)).map((group) => ({
    group,
    label: DUE_GROUP_LABELS[group],
    tasks: byGroup.get(group) ?? [],
  }));
}

/** 期日ラベル（Asia/Tokyo表示、日付のみは時刻を省く） */
export function formatDueLabel(dueAtUtc: string, now: Date): string {
  const sameYear =
    formatInTimeZone(dueAtUtc, TOKYO_TZ, "yyyy") === formatInTimeZone(now, TOKYO_TZ, "yyyy");
  const datePart = formatInTimeZone(dueAtUtc, TOKYO_TZ, sameYear ? "M月d日" : "yyyy年M月d日");
  if (isDateOnlyDue(dueAtUtc)) return datePart;
  return `${datePart} ${formatInTimeZone(dueAtUtc, TOKYO_TZ, "HH:mm")}`;
}

/** フォームの日付・時刻入力からUTC期日を作る（時刻空はTokyo 0時=日付のみ） */
export function buildDueAtUtc(date: string, time: string): string | null {
  if (!date) return null;
  return fromZonedTime(`${date}T${time || "00:00"}:00`, TOKYO_TZ).toISOString();
}

/** UTC期日をフォーム入力用の日付・時刻へ分解する */
export function splitDueAtUtc(dueAtUtc: string | null): { date: string; time: string } {
  if (!dueAtUtc) return { date: "", time: "" };
  return {
    date: tokyoDateString(dueAtUtc),
    time: isDateOnlyDue(dueAtUtc) ? "" : formatInTimeZone(dueAtUtc, TOKYO_TZ, "HH:mm"),
  };
}
