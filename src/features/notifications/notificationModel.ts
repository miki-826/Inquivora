import { formatInTimeZone } from "date-fns-tz";
import { z } from "zod";
import { TOKYO_TZ } from "../tasks/taskModel";

export type DeepLinkTarget = {
  type: "task" | "event" | "meeting";
  id: string;
};

const notificationSettingsSchema = z.object({
  enabled: z.boolean().default(true),
  sound: z.boolean().default(true),
  defaultNotifyTime: z
    .string()
    .regex(/^\d{2}:\d{2}$/)
    .default("09:00"),
  taskLeadMinutes: z.number().int().min(0).nullable().default(30),
  eventLeadMinutes: z.number().int().min(0).nullable().default(10),
});

export type NotificationSettings = z.infer<typeof notificationSettingsSchema>;

export const DEFAULT_NOTIFICATION_SETTINGS: NotificationSettings = {
  enabled: true,
  sound: true,
  defaultNotifyTime: "09:00",
  taskLeadMinutes: 30,
  eventLeadMinutes: 10,
};

export function parseNotificationSettings(value: unknown): NotificationSettings {
  const result = notificationSettingsSchema.safeParse(value ?? {});
  return result.success ? result.data : DEFAULT_NOTIFICATION_SETTINGS;
}

const DEEP_LINK_TYPES = new Set(["task", "event", "meeting"]);

/// §14.3 inquivora://open?type=...&id=... を解析する。
export function parseDeepLink(url: string): DeepLinkTarget | null {
  let parsed: URL;
  try {
    parsed = new URL(url);
  } catch {
    return null;
  }
  if (parsed.protocol !== "inquivora:") return null;
  const host = parsed.host || parsed.pathname.replace(/^\/+|\/+$/g, "");
  if (host !== "open") return null;
  const type = parsed.searchParams.get("type");
  const id = parsed.searchParams.get("id");
  if (!type || !id || !DEEP_LINK_TYPES.has(type)) return null;
  return { type: type as DeepLinkTarget["type"], id };
}

export function formatNotifyLabel(notifyAtUtc: string): string {
  return formatInTimeZone(notifyAtUtc, TOKYO_TZ, "yyyy年M月d日 HH:mm");
}
