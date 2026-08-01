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

function replaceTask(tasks: Task[], task: Task): Task[] {
  const index = tasks.findIndex((item) => item.id === task.id);
  if (index < 0) return [task, ...tasks];
  const next = [...tasks];
  next[index] = task;
  return next;
}

function optimisticTask(task: Task, patch: TaskPatch): Task {
  const status = patch.status ?? task.status;
  return {
    ...task,
    ...patch,
    status,
    updatedAt: new Date().toISOString(),
    completedAt:
      status === "completed" ? task.completedAt ?? new Date().toISOString() : null,
  };
}

export function duplicateTaskInput(source: Task): TaskInput {
  return {
    title: `${source.title}（コピー）`,
    description: source.description,
    priority: source.priority,
    color: source.color,
    assignee: source.assignee,
    projectName: source.projectName,
    linkedFilePath: source.linkedFilePath,
  };
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
    set({ filter, loading: true });
    void taskService
      .listTasks(filter)
      .then((tasks) => set({ tasks, loading: false, error: null }))
      .catch((err) => set({ loading: false, error: errorMessage(err) }));
  },

  select: (id) => set({ selectedId: id }),

  createTask: async (input) => {
    try {
      const task = await taskService.createTask(input);
      set((state) => ({
        tasks: replaceTask(state.tasks, task),
        allTasks: replaceTask(state.allTasks, task),
        selectedId: task.id,
        error: null,
      }));
      void taskService
        .listTasks(get().filter)
        .then((tasks) => set({ tasks }))
        .catch(() => undefined);
      return task;
    } catch (err) {
      set({ error: errorMessage(err) });
      return null;
    }
  },

  updateTask: async (id, patch) => {
    const snapshot = get();
    const current =
      snapshot.allTasks.find((task) => task.id === id) ??
      snapshot.tasks.find((task) => task.id === id);
    if (current) {
      const optimistic = optimisticTask(current, patch);
      set({
        tasks: replaceTask(snapshot.tasks, optimistic),
        allTasks: replaceTask(snapshot.allTasks, optimistic),
      });
    }
    try {
      const saved = await taskService.updateTask(id, patch);
      set((state) => ({
        tasks: replaceTask(state.tasks, saved),
        allTasks: replaceTask(state.allTasks, saved),
        error: null,
      }));
      void taskService
        .listTasks(get().filter)
        .then((tasks) => set({ tasks }))
        .catch(() => undefined);
    } catch (err) {
      set({
        tasks: snapshot.tasks,
        allTasks: snapshot.allTasks,
        error: errorMessage(err),
      });
    }
  },

  removeTask: async (id) => {
    const snapshot = get();
    set({
      tasks: snapshot.tasks.filter((task) => task.id !== id),
      allTasks: snapshot.allTasks.filter((task) => task.id !== id),
      selectedId: snapshot.selectedId === id ? null : snapshot.selectedId,
    });
    try {
      await taskService.deleteTask(id);
      set({ error: null });
    } catch (err) {
      set({
        tasks: snapshot.tasks,
        allTasks: snapshot.allTasks,
        selectedId: snapshot.selectedId,
        error: errorMessage(err),
      });
    }
  },

  toggleComplete: async (task) => {
    const snapshot = get();
    const nextStatus = task.status === "completed" ? "todo" : "completed";
    const optimistic = optimisticTask(task, { status: nextStatus });
    set({
      tasks: replaceTask(snapshot.tasks, optimistic),
      allTasks: replaceTask(snapshot.allTasks, optimistic),
    });
    try {
      const saved =
        task.status === "completed"
          ? await taskService.reopenTask(task.id)
          : await taskService.completeTask(task.id);
      set((state) => ({
        tasks: replaceTask(state.tasks, saved),
        allTasks: replaceTask(state.allTasks, saved),
        error: null,
      }));
      void taskService
        .listTasks(get().filter)
        .then((tasks) => set({ tasks }))
        .catch(() => undefined);
    } catch (err) {
      set({
        tasks: snapshot.tasks,
        allTasks: snapshot.allTasks,
        error: errorMessage(err),
      });
    }
  },

  duplicateTask: async (id) => {
    const source =
      get().allTasks.find((t) => t.id === id) ?? get().tasks.find((t) => t.id === id);
    if (!source) return;
    await get().createTask(duplicateTaskInput(source));
  },
}));
