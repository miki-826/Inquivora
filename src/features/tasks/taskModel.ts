export type TaskPriority = "high" | "medium" | "low";
export type TaskStatus = "todo" | "in_progress" | "on_hold" | "completed" | "cancelled";

export type Task = {
  id: string;
  title: string;
  description: string | null;
  dueAtUtc: string | null;
  timezone: string;
  priority: TaskPriority;
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

export const STATUS_LABELS: Record<TaskStatus, string> = {
  todo: "未着手",
  in_progress: "進行中",
  on_hold: "保留",
  completed: "完了",
  cancelled: "中止",
};

/** 期日がAsia/Tokyoの0時ちょうど（=日付のみ）かを判定する（§13.4） */
export function isDateOnlyDue(_dueAtUtc: string): boolean {
  throw new Error("未実装");
}

/** タスクの表示グループを判定する（§12.1の表示順） */
export function dueGroupOf(_task: Task, _now: Date): DueGroup {
  throw new Error("未実装");
}

/** §12.1の表示順でグループ化する（空グループは除く） */
export function groupTasks(
  _tasks: Task[],
  _now: Date,
): { group: DueGroup; label: string; tasks: Task[] }[] {
  throw new Error("未実装");
}

/** 期日ラベル（Asia/Tokyo表示、日付のみは時刻を省く） */
export function formatDueLabel(_dueAtUtc: string, _now: Date): string {
  throw new Error("未実装");
}

/** フォームの日付・時刻入力からUTC期日を作る（時刻空はTokyo 0時=日付のみ） */
export function buildDueAtUtc(_date: string, _time: string): string | null {
  throw new Error("未実装");
}

/** UTC期日をフォーム入力用の日付・時刻へ分解する */
export function splitDueAtUtc(_dueAtUtc: string | null): { date: string; time: string } {
  throw new Error("未実装");
}
