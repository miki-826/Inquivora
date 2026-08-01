import { describe, expect, it } from "vitest";
import type { Task } from "../features/tasks/taskModel";
import { duplicateTaskInput } from "./useTaskStore";

const SOURCE_TASK: Task = {
  id: "task-1",
  title: "期限付きタスク",
  description: "説明",
  dueAtUtc: "2026-08-10T03:00:00Z",
  timezone: "Asia/Tokyo",
  priority: "high",
  color: "blue",
  status: "in_progress",
  assignee: "自分",
  projectName: "開発",
  meetingId: "meeting-1",
  linkedFilePath: "C:/notes/task.md",
  createdAt: "2026-08-01T00:00:00Z",
  updatedAt: "2026-08-01T00:00:00Z",
  completedAt: null,
};

describe("duplicateTaskInput", () => {
  it("内容を複製しつつ期日は引き継がない", () => {
    const input = duplicateTaskInput(SOURCE_TASK);
    expect(input.title).toBe("期限付きタスク（コピー）");
    expect(input).not.toHaveProperty("dueAtUtc");
    expect(input.description).toBe(SOURCE_TASK.description);
    expect(input.priority).toBe(SOURCE_TASK.priority);
  });
});
