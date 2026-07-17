import { describe, expect, it } from "vitest";
import {
  buildDueAtUtc,
  dueGroupOf,
  formatDueLabel,
  groupTasks,
  isDateOnlyDue,
  splitDueAtUtc,
  type Task,
} from "./taskModel";

function task(overrides: Partial<Task>): Task {
  return {
    id: "t1",
    title: "タスク",
    description: null,
    dueAtUtc: null,
    timezone: "Asia/Tokyo",
    priority: "medium",
    status: "todo",
    assignee: null,
    projectName: null,
    meetingId: null,
    linkedFilePath: null,
    createdAt: "2026-07-01T00:00:00Z",
    updatedAt: "2026-07-01T00:00:00Z",
    completedAt: null,
    ...overrides,
  };
}

// Asia/Tokyoで2026-07-17（金）12:00
const now = new Date("2026-07-17T03:00:00Z");

describe("isDateOnlyDue", () => {
  it("Tokyoの0時ちょうどは日付のみ", () => {
    expect(isDateOnlyDue("2026-07-16T15:00:00Z")).toBe(true);
  });

  it("時刻付きは日付のみではない", () => {
    expect(isDateOnlyDue("2026-07-17T01:00:00Z")).toBe(false);
    expect(isDateOnlyDue("2026-07-16T15:30:00Z")).toBe(false);
  });
});

describe("dueGroupOf", () => {
  it("完了タスクは期日に関わらずcompleted", () => {
    expect(
      dueGroupOf(task({ status: "completed", dueAtUtc: "2026-07-17T01:00:00Z" }), now),
    ).toBe("completed");
  });

  it("期日なしはnone", () => {
    expect(dueGroupOf(task({}), now)).toBe("none");
  });

  it("日付のみ: 昨日は期限切れ・今日は今日", () => {
    expect(dueGroupOf(task({ dueAtUtc: "2026-07-15T15:00:00Z" }), now)).toBe("overdue");
    expect(dueGroupOf(task({ dueAtUtc: "2026-07-16T15:00:00Z" }), now)).toBe("today");
  });

  it("時刻付き: 今日でも過ぎていれば期限切れ", () => {
    expect(dueGroupOf(task({ dueAtUtc: "2026-07-17T01:00:00Z" }), now)).toBe("overdue");
    expect(dueGroupOf(task({ dueAtUtc: "2026-07-17T09:00:00Z" }), now)).toBe("today");
  });

  it("明日・今週・来週以降を区別する", () => {
    expect(dueGroupOf(task({ dueAtUtc: "2026-07-17T15:00:00Z" }), now)).toBe("tomorrow");
    expect(dueGroupOf(task({ dueAtUtc: "2026-07-19T01:00:00Z" }), now)).toBe("thisWeek");
    expect(dueGroupOf(task({ dueAtUtc: "2026-07-20T01:00:00Z" }), now)).toBe("later");
  });
});

describe("groupTasks", () => {
  it("表示順のグループへ振り分け、空グループは除く", () => {
    const tasks = [
      task({ id: "a", dueAtUtc: "2026-07-15T15:00:00Z" }),
      task({ id: "b", dueAtUtc: "2026-07-16T15:00:00Z" }),
      task({ id: "c" }),
      task({ id: "d", status: "completed", dueAtUtc: "2026-07-16T15:00:00Z" }),
    ];
    const groups = groupTasks(tasks, now);
    expect(groups.map((g) => g.group)).toEqual(["overdue", "today", "none", "completed"]);
    expect(groups[0].label).toBe("期限切れ");
    expect(groups[0].tasks.map((t) => t.id)).toEqual(["a"]);
    expect(groups[3].tasks.map((t) => t.id)).toEqual(["d"]);
  });
});

describe("formatDueLabel", () => {
  it("同じ年は月日と時刻", () => {
    expect(formatDueLabel("2026-07-17T01:00:00Z", now)).toBe("7月17日 10:00");
  });

  it("日付のみは時刻を省く", () => {
    expect(formatDueLabel("2026-07-16T15:00:00Z", now)).toBe("7月17日");
  });

  it("別の年は年を含む", () => {
    expect(formatDueLabel("2027-01-09T15:00:00Z", now)).toBe("2027年1月10日");
  });
});

describe("buildDueAtUtc / splitDueAtUtc", () => {
  it("日付と時刻からUTCを作る", () => {
    expect(buildDueAtUtc("2026-07-17", "10:00")).toBe("2026-07-17T01:00:00.000Z");
  });

  it("時刻が空ならTokyoの0時", () => {
    expect(buildDueAtUtc("2026-07-17", "")).toBe("2026-07-16T15:00:00.000Z");
  });

  it("日付が空ならnull", () => {
    expect(buildDueAtUtc("", "10:00")).toBeNull();
  });

  it("UTCをフォーム入力へ往復できる", () => {
    expect(splitDueAtUtc("2026-07-17T01:00:00Z")).toEqual({ date: "2026-07-17", time: "10:00" });
    expect(splitDueAtUtc("2026-07-16T15:00:00Z")).toEqual({ date: "2026-07-17", time: "" });
    expect(splitDueAtUtc(null)).toEqual({ date: "", time: "" });
  });
});
