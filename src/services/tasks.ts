import { invoke } from "@tauri-apps/api/core";
import type { Task, TaskColor, TaskFilter, TaskPriority, TaskStatus } from "../features/tasks/taskModel";

export type TaskInput = {
  title: string;
  description?: string | null;
  dueAtUtc?: string | null;
  priority?: TaskPriority;
  color?: TaskColor;
  status?: TaskStatus;
  assignee?: string | null;
  projectName?: string | null;
  meetingId?: string | null;
  linkedFilePath?: string | null;
};

export type TaskPatch = {
  title?: string;
  description?: string | null;
  dueAtUtc?: string | null;
  timezone?: string;
  priority?: TaskPriority;
  color?: TaskColor;
  status?: TaskStatus;
  assignee?: string | null;
  projectName?: string | null;
  meetingId?: string | null;
  linkedFilePath?: string | null;
};

export function createTask(input: TaskInput): Promise<Task> {
  return invoke("task_create", { input });
}

export function updateTask(id: string, patch: TaskPatch): Promise<Task> {
  return invoke("task_update", { id, patch });
}

export function deleteTask(id: string): Promise<void> {
  return invoke("task_delete", { id });
}

export function getTask(id: string): Promise<Task> {
  return invoke("task_get", { id });
}

export function listTasks(filter?: TaskFilter): Promise<Task[]> {
  return invoke("task_list", { filter: filter ?? null });
}

export function completeTask(id: string): Promise<Task> {
  return invoke("task_complete", { id });
}

export function reopenTask(id: string): Promise<Task> {
  return invoke("task_reopen", { id });
}

export type CandidateAcceptPatch = {
  title?: string;
  description?: string | null;
  dueAtUtc?: string | null;
  priority?: TaskPriority;
  assignee?: string | null;
};

export function acceptTaskCandidate(
  candidateId: string,
  patch?: CandidateAcceptPatch,
): Promise<Task> {
  return invoke("task_accept_candidate", { candidateId, patch: patch ?? null });
}
