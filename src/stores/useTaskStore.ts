import { create } from "zustand";
import type { Task, TaskFilter } from "../features/tasks/taskModel";
import * as taskService from "../services/tasks";
import type { TaskInput, TaskPatch } from "../services/tasks";

type TaskState = {
  tasks: Task[];
  allTasks: Task[];
  filter: TaskFilter;
  selectedId: string | null;
  loading: boolean;
  error: string | null;
  load: () => Promise<void>;
  setFilter: (filter: TaskFilter) => void;
  select: (id: string | null) => void;
  createTask: (input: TaskInput) => Promise<Task | null>;
  updateTask: (id: string, patch: TaskPatch) => Promise<void>;
  removeTask: (id: string) => Promise<void>;
  toggleComplete: (task: Task) => Promise<void>;
  duplicateTask: (id: string) => Promise<void>;
};

function errorMessage(err: unknown): string {
  if (err && typeof err === "object" && "message" in err) {
    return String((err as { message: unknown }).message);
  }
  return String(err);
}

export const useTaskStore = create<TaskState>((set, get) => ({
  tasks: [],
  allTasks: [],
  filter: { preset: "open" },
  selectedId: null,
  loading: false,
  error: null,

  load: async () => {
    set({ loading: true });
    try {
      const [tasks, allTasks] = await Promise.all([
        taskService.listTasks(get().filter),
        taskService.listTasks(),
      ]);
      set({ tasks, allTasks, loading: false, error: null });
    } catch (err) {
      set({ loading: false, error: errorMessage(err) });
    }
  },

  setFilter: (filter) => {
    set({ filter });
    void get().load();
  },

  select: (id) => set({ selectedId: id }),

  createTask: async (input) => {
    try {
      const task = await taskService.createTask(input);
      await get().load();
      set({ selectedId: task.id, error: null });
      return task;
    } catch (err) {
      set({ error: errorMessage(err) });
      return null;
    }
  },

  updateTask: async (id, patch) => {
    try {
      await taskService.updateTask(id, patch);
      await get().load();
      set({ error: null });
    } catch (err) {
      set({ error: errorMessage(err) });
    }
  },

  removeTask: async (id) => {
    try {
      await taskService.deleteTask(id);
      if (get().selectedId === id) set({ selectedId: null });
      await get().load();
      set({ error: null });
    } catch (err) {
      set({ error: errorMessage(err) });
    }
  },

  toggleComplete: async (task) => {
    try {
      if (task.status === "completed") {
        await taskService.reopenTask(task.id);
      } else {
        await taskService.completeTask(task.id);
      }
      await get().load();
      set({ error: null });
    } catch (err) {
      set({ error: errorMessage(err) });
    }
  },

  duplicateTask: async (id) => {
    const source =
      get().allTasks.find((t) => t.id === id) ?? get().tasks.find((t) => t.id === id);
    if (!source) return;
    await get().createTask({
      title: `${source.title}（コピー）`,
      description: source.description,
      dueAtUtc: source.dueAtUtc,
      priority: source.priority,
      color: source.color,
      assignee: source.assignee,
      projectName: source.projectName,
      linkedFilePath: source.linkedFilePath,
    });
  },
}));
