export type DeepLinkTarget = {
  type: "task" | "event" | "meeting";
  id: string;
};

export type NotificationSettings = {
  enabled: boolean;
  sound: boolean;
  defaultNotifyTime: string;
  taskLeadMinutes: number | null;
  eventLeadMinutes: number | null;
};

export const DEFAULT_NOTIFICATION_SETTINGS: NotificationSettings = {
  enabled: true,
  sound: true,
  defaultNotifyTime: "09:00",
  taskLeadMinutes: 30,
  eventLeadMinutes: 10,
};

export function parseNotificationSettings(_value: unknown): NotificationSettings {
  throw new Error("未実装");
}

export function parseDeepLink(_url: string): DeepLinkTarget | null {
  throw new Error("未実装");
}

export function formatNotifyLabel(_notifyAtUtc: string): string {
  throw new Error("未実装");
}
