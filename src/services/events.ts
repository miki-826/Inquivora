import { invoke } from "@tauri-apps/api/core";
import type { EventRecord } from "../features/calendar/calendarModel";

export type EventInput = {
  title: string;
  startAtUtc: string;
  endAtUtc?: string | null;
  description?: string | null;
  allDay?: boolean;
  eventType?: string;
  recurrenceRule?: string | null;
  meetingId?: string | null;
  taskId?: string | null;
  location?: string | null;
};

export type EventPatch = {
  title?: string;
  startAtUtc?: string;
  endAtUtc?: string | null;
  description?: string | null;
  timezone?: string;
  allDay?: boolean;
  eventType?: string;
  recurrenceRule?: string | null;
  location?: string | null;
};

export function createEvent(input: EventInput): Promise<EventRecord> {
  return invoke("event_create", { input });
}

export function createEvents(inputs: EventInput[]): Promise<EventRecord[]> {
  return invoke("event_create_many", { inputs });
}

export function updateEvent(id: string, patch: EventPatch): Promise<EventRecord> {
  return invoke("event_update", { id, patch });
}

export function deleteEvent(id: string): Promise<void> {
  return invoke("event_delete", { id });
}

export function getEvent(id: string): Promise<EventRecord> {
  return invoke("event_get", { id });
}

export function getEventsInRange(startUtc: string, endUtc: string): Promise<EventRecord[]> {
  return invoke("event_get_range", { startUtc, endUtc });
}
