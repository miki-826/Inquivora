import { describe, expect, it } from "vitest";
import {
  DEFAULT_NOTIFICATION_SETTINGS,
  formatNotifyLabel,
  parseDeepLink,
  parseNotificationSettings,
} from "./notificationModel";

describe("parseNotificationSettings", () => {
  it("欠落や不正な値は既定値になる", () => {
    expect(parseNotificationSettings(null)).toEqual(DEFAULT_NOTIFICATION_SETTINGS);
    expect(parseNotificationSettings(undefined)).toEqual(DEFAULT_NOTIFICATION_SETTINGS);
    expect(parseNotificationSettings("壊れた値")).toEqual(DEFAULT_NOTIFICATION_SETTINGS);
    expect(parseNotificationSettings({ enabled: "yes" })).toEqual(DEFAULT_NOTIFICATION_SETTINGS);
  });

  it("保存された設定を解釈できる", () => {
    const parsed = parseNotificationSettings({
      enabled: false,
      sound: false,
      defaultNotifyTime: "08:30",
      taskLeadMinutes: null,
      eventLeadMinutes: 15,
    });
    expect(parsed).toEqual({
      enabled: false,
      sound: false,
      defaultNotifyTime: "08:30",
      taskLeadMinutes: null,
      eventLeadMinutes: 15,
    });
  });

  it("一部欠落は既定値で補う", () => {
    const parsed = parseNotificationSettings({ enabled: false });
    expect(parsed.enabled).toBe(false);
    expect(parsed.sound).toBe(true);
    expect(parsed.defaultNotifyTime).toBe("09:00");
    expect(parsed.taskLeadMinutes).toBe(30);
    expect(parsed.eventLeadMinutes).toBe(10);
  });
});

describe("parseDeepLink", () => {
  it("タスク・予定・議事録のリンクを解釈できる", () => {
    expect(parseDeepLink("inquivora://open?type=task&id=t1")).toEqual({
      type: "task",
      id: "t1",
    });
    expect(parseDeepLink("inquivora://open?type=event&id=e1")).toEqual({
      type: "event",
      id: "e1",
    });
    expect(parseDeepLink("inquivora://open?type=meeting&id=m1")).toEqual({
      type: "meeting",
      id: "m1",
    });
  });

  it("inquivora以外のスキームはnullになる", () => {
    expect(parseDeepLink("https://example.com/open?type=task&id=t1")).toBeNull();
  });

  it("不正・不完全なリンクはnullになる", () => {
    expect(parseDeepLink("ただの文字列")).toBeNull();
    expect(parseDeepLink("inquivora://open?type=task")).toBeNull();
    expect(parseDeepLink("inquivora://open?type=unknown&id=x")).toBeNull();
    expect(parseDeepLink("inquivora://other?type=task&id=t1")).toBeNull();
  });
});

describe("formatNotifyLabel", () => {
  it("通知時刻を日本時間で表示する", () => {
    expect(formatNotifyLabel("2026-07-18T00:30:00Z")).toBe("2026年7月18日 09:30");
    expect(formatNotifyLabel("2026-12-31T15:00:00Z")).toBe("2027年1月1日 00:00");
  });
});
