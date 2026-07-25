import { invoke } from "@tauri-apps/api/core";

export type Reminder = {
  id: string;
  taskId: string | null;
  eventId: string | null;
  notifyAtUtc: string;
  timezone: string;
  status: string;
  sentAtUtc: string | null;
  repeatIntervalMinutes: number | null;
  createdAt: string;
  updatedAt: string;
};

export type ReminderInput = {
  taskId?: string;
  eventId?: string;
  notifyAtUtc: string;
  repeatIntervalMinutes?: number | null;
};

export function createReminder(input: ReminderInput): Promise<Reminder> {
  return invoke("reminder_create", { input });
}

export function updateReminder(id: string, patch: { notifyAtUtc: string }): Promise<Reminder> {
  return invoke("reminder_update", { id, patch });
}

export function deleteReminder(id: string): Promise<void> {
  return invoke("reminder_delete", { id });
}

export function listUpcomingReminders(): Promise<Reminder[]> {
  return invoke("reminder_list_upcoming");
}

export function listRemindersForTarget(target: {
  taskId?: string;
  eventId?: string;
}): Promise<Reminder[]> {
  return invoke("reminder_list_for_target", {
    taskId: target.taskId ?? null,
    eventId: target.eventId ?? null,
  });
}

export function sendTestNotification(): Promise<void> {
  return invoke("notification_test");
}

export function reconcileNotifications(): Promise<number> {
  return invoke("notification_reconcile");
}
