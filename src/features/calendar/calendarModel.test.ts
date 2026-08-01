import { describe, expect, it } from "vitest";
import type { Task } from "../tasks/taskModel";
import {
  buildCalendarInputs,
  buildRecurringEventInputs,
  eventToCalendarInput,
  formatEventRange,
  taskToCalendarInput,
  type EventRecord,
} from "./calendarModel";

function event(overrides: Partial<EventRecord>): EventRecord {
  return {
    id: "e1",
    title: "打合せ",
    description: null,
    startAtUtc: "2026-07-17T01:00:00Z",
    endAtUtc: "2026-07-17T02:00:00Z",
    timezone: "Asia/Tokyo",
    allDay: false,
    eventType: "event",
    recurrenceRule: null,
    meetingId: null,
    taskId: null,
    location: null,
    createdAt: "2026-07-01T00:00:00Z",
    updatedAt: "2026-07-01T00:00:00Z",
    ...overrides,
  };
}

function task(overrides: Partial<Task>): Task {
  return {
    id: "t1",
    title: "タスク",
    description: null,
    dueAtUtc: null,
    timezone: "Asia/Tokyo",
    priority: "medium",
    color: "blue",
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

describe("eventToCalendarInput", () => {
  it("時刻付き予定はUTCのまま渡す", () => {
    const input = eventToCalendarInput(event({}));
    expect(input.id).toBe("event:e1");
    expect(input.title).toBe("予定 · 打合せ");
    expect(input.start).toBe("2026-07-17T01:00:00Z");
    expect(input.end).toBe("2026-07-17T02:00:00Z");
    expect(input.allDay).toBe(false);
    expect(input.extendedProps).toEqual({ kind: "event", sourceId: "e1" });
  });

  it("終日予定はTokyoの日付文字列へ変換する", () => {
    const input = eventToCalendarInput(
      event({
        allDay: true,
        startAtUtc: "2026-07-16T15:00:00Z",
        endAtUtc: "2026-07-18T15:00:00Z",
      }),
    );
    expect(input.start).toBe("2026-07-17");
    expect(input.end).toBe("2026-07-19");
    expect(input.allDay).toBe(true);
  });

  it("終了なしの終日予定はendを持たない", () => {
    const input = eventToCalendarInput(
      event({ allDay: true, startAtUtc: "2026-07-16T15:00:00Z", endAtUtc: null }),
    );
    expect(input.start).toBe("2026-07-17");
    expect(input.end).toBeUndefined();
  });
});

describe("taskToCalendarInput", () => {
  it("期日なしはnull", () => {
    expect(taskToCalendarInput(task({}))).toBeNull();
  });

  it("時刻付きは時刻イベント", () => {
    const input = taskToCalendarInput(task({ dueAtUtc: "2026-07-17T01:00:00Z" }));
    expect(input?.id).toBe("task:t1");
    expect(input?.title).toBe("タスク · タスク");
    expect(input?.start).toBe("2026-07-17T01:00:00Z");
    expect(input?.allDay).toBe(false);
    expect(input?.classNames).toContain("cal-task");
  });

  it("日付のみは終日イベント", () => {
    const input = taskToCalendarInput(task({ dueAtUtc: "2026-07-16T15:00:00Z" }));
    expect(input?.start).toBe("2026-07-17");
    expect(input?.allDay).toBe(true);
  });

  it("完了タスクは完了クラスが付く", () => {
    const input = taskToCalendarInput(
      task({ status: "completed", dueAtUtc: "2026-07-17T01:00:00Z" }),
    );
    expect(input?.classNames).toContain("cal-task--completed");
    expect(input?.extendedProps.completed).toBe(true);
  });
});

describe("buildRecurringEventInputs", () => {
  const base = {
    title: "朝会",
    startAtUtc: "2026-08-01T00:00:00.000Z",
    endAtUtc: "2026-08-01T00:30:00.000Z",
    location: "オンライン",
  };

  it("毎日の予定を指定回数へ展開する", () => {
    const inputs = buildRecurringEventInputs(base, "daily", 3);
    expect(inputs.map((input) => input.startAtUtc)).toEqual([
      "2026-08-01T00:00:00.000Z",
      "2026-08-02T00:00:00.000Z",
      "2026-08-03T00:00:00.000Z",
    ]);
    expect(inputs[2].endAtUtc).toBe("2026-08-03T00:30:00.000Z");
    expect(inputs[2].location).toBe("オンライン");
  });

  it("毎週の予定を7日ずつずらす", () => {
    const inputs = buildRecurringEventInputs(base, "weekly", 2);
    expect(inputs[1].startAtUtc).toBe("2026-08-08T00:00:00.000Z");
  });

  it("繰り返しなしは入力を1件だけ返す", () => {
    expect(buildRecurringEventInputs(base, "none", 99)).toEqual([base]);
  });
});

describe("buildCalendarInputs", () => {
  const events = [event({})];
  const tasks = [
    task({ id: "t1", dueAtUtc: "2026-07-17T01:00:00Z" }),
    task({ id: "t2", status: "completed", dueAtUtc: "2026-07-17T02:00:00Z" }),
    task({ id: "t3" }),
  ];

  it("予定とタスクをまとめ、期日なしタスクは除く", () => {
    const inputs = buildCalendarInputs(events, tasks, true);
    expect(inputs.map((i) => i.id)).toEqual(["event:e1", "task:t1", "task:t2"]);
  });

  it("完了タスクを除外できる", () => {
    const inputs = buildCalendarInputs(events, tasks, false);
    expect(inputs.map((i) => i.id)).toEqual(["event:e1", "task:t1"]);
  });
});

describe("formatEventRange", () => {
  it("同日の時刻範囲", () => {
    expect(formatEventRange(event({}))).toBe("2026年7月17日 10:00〜11:00");
  });

  it("終日は日付のみ", () => {
    expect(
      formatEventRange(event({ allDay: true, startAtUtc: "2026-07-16T15:00:00Z", endAtUtc: null })),
    ).toBe("2026年7月17日 終日");
  });

  it("日をまたぐ範囲は両端の日付を出す", () => {
    expect(
      formatEventRange(event({ startAtUtc: "2026-07-17T01:00:00Z", endAtUtc: "2026-07-18T02:00:00Z" })),
    ).toBe("2026年7月17日 10:00〜2026年7月18日 11:00");
  });
});
