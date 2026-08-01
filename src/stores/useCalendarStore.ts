import { create } from "zustand";
import type { EventRecord } from "../features/calendar/calendarModel";
import type { Task } from "../features/tasks/taskModel";
import * as eventService from "../services/events";
import type { EventInput, EventPatch } from "../services/events";
import { listTasks, updateTask as saveTask, type TaskPatch } from "../services/tasks";

type CalendarState = {
  events: EventRecord[];
  tasks: Task[];
  rangeStartUtc: string | null;
  rangeEndUtc: string | null;
  error: string | null;
  focusEventId: string | null;
  loadRange: (startUtc: string, endUtc: string) => Promise<void>;
  reload: () => Promise<void>;
  createEvent: (input: EventInput) => Promise<EventRecord | null>;
  createEvents: (inputs: EventInput[]) => Promise<EventRecord[] | null>;
  updateEvent: (id: string, patch: EventPatch) => Promise<boolean>;
  removeEvent: (id: string) => Promise<void>;
  updateTask: (id: string, patch: TaskPatch) => Promise<boolean>;
  scheduleTask: (id: string, dueAtUtc: string) => Promise<boolean>;
  setFocusEventId: (id: string | null) => void;
};

function errorMessage(err: unknown): string {
  if (err && typeof err === "object" && "message" in err) {
    return String((err as { message: unknown }).message);
  }
  return String(err);
}

export const useCalendarStore = create<CalendarState>((set, get) => ({
  events: [],
  tasks: [],
  rangeStartUtc: null,
  rangeEndUtc: null,
  error: null,
  focusEventId: null,

  setFocusEventId: (id) => set({ focusEventId: id }),

  loadRange: async (startUtc, endUtc) => {
    set({ rangeStartUtc: startUtc, rangeEndUtc: endUtc });
    try {
      const [events, tasks] = await Promise.all([
        eventService.getEventsInRange(startUtc, endUtc),
        listTasks(),
      ]);
      set({ events, tasks, error: null });
    } catch (err) {
      set({ error: errorMessage(err) });
    }
  },

  reload: async () => {
    const { rangeStartUtc, rangeEndUtc } = get();
    if (rangeStartUtc && rangeEndUtc) {
      await get().loadRange(rangeStartUtc, rangeEndUtc);
    }
  },

  createEvent: async (input) => {
    try {
      const event = await eventService.createEvent(input);
      set((state) => ({ events: [...state.events, event], error: null }));
      return event;
    } catch (err) {
      set({ error: errorMessage(err) });
      return null;
    }
  },

  createEvents: async (inputs) => {
    try {
      const events = await eventService.createEvents(inputs);
      set((state) => ({ events: [...state.events, ...events], error: null }));
      return events;
    } catch (err) {
      set({ error: errorMessage(err) });
      return null;
    }
  },

  updateEvent: async (id, patch) => {
    const snapshot = get().events;
    const current = snapshot.find((event) => event.id === id);
    if (current) {
      set({
        events: snapshot.map((event) =>
          event.id === id
            ? { ...event, ...patch, updatedAt: new Date().toISOString() }
            : event,
        ),
      });
    }
    try {
      const saved = await eventService.updateEvent(id, patch);
      set((state) => ({
        events: state.events.map((event) => (event.id === id ? saved : event)),
        error: null,
      }));
      return true;
    } catch (err) {
      set({ events: snapshot, error: errorMessage(err) });
      return false;
    }
  },

  removeEvent: async (id) => {
    const snapshot = get().events;
    set({ events: snapshot.filter((event) => event.id !== id) });
    try {
      await eventService.deleteEvent(id);
      set({ error: null });
    } catch (err) {
      set({ events: snapshot, error: errorMessage(err) });
    }
  },

  updateTask: async (id, patch) => {
    const snapshot = get().tasks;
    const current = snapshot.find((task) => task.id === id);
    if (!current) return false;
    set({
      tasks: snapshot.map((task) =>
        task.id === id ? { ...task, ...patch, updatedAt: new Date().toISOString() } : task,
      ),
    });
    try {
      const saved = await saveTask(id, patch);
      set((state) => ({
        tasks: state.tasks.map((task) => (task.id === id ? saved : task)),
        error: null,
      }));
      return true;
    } catch (err) {
      set({ tasks: snapshot, error: errorMessage(err) });
      return false;
    }
  },

  scheduleTask: (id, dueAtUtc) => get().updateTask(id, { dueAtUtc }),
}));
