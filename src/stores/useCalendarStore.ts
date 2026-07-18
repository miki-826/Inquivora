import { create } from "zustand";
import type { EventRecord } from "../features/calendar/calendarModel";
import type { Task } from "../features/tasks/taskModel";
import * as eventService from "../services/events";
import type { EventInput, EventPatch } from "../services/events";
import { listTasks } from "../services/tasks";

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
  updateEvent: (id: string, patch: EventPatch) => Promise<boolean>;
  removeEvent: (id: string) => Promise<void>;
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
      await get().reload();
      set({ error: null });
      return event;
    } catch (err) {
      set({ error: errorMessage(err) });
      return null;
    }
  },

  updateEvent: async (id, patch) => {
    try {
      await eventService.updateEvent(id, patch);
      await get().reload();
      set({ error: null });
      return true;
    } catch (err) {
      set({ error: errorMessage(err) });
      await get().reload();
      return false;
    }
  },

  removeEvent: async (id) => {
    try {
      await eventService.deleteEvent(id);
      await get().reload();
      set({ error: null });
    } catch (err) {
      set({ error: errorMessage(err) });
    }
  },
}));
