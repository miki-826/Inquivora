import { describe, expect, it } from "vitest";
import { parseDiscordSettings, validateWebhookUrl } from "./discordModel";

describe("parseDiscordSettings", () => {
  it("未保存なら既定値（無効・まとめのみ）", () => {
    const settings = parseDiscordSettings(null);
    expect(settings).toEqual({ enabled: false, realtime: false, summary: true });
  });

  it("保存済み設定を読み込む", () => {
    const settings = parseDiscordSettings({ enabled: true, realtime: true, summary: false });
    expect(settings).toEqual({ enabled: true, realtime: true, summary: false });
  });

  it("欠けたキーは既定値で補う", () => {
    const settings = parseDiscordSettings({ enabled: true });
    expect(settings).toEqual({ enabled: true, realtime: false, summary: true });
  });

  it("不正な値は既定値に戻す", () => {
    expect(parseDiscordSettings("broken")).toEqual({
      enabled: false,
      realtime: false,
      summary: true,
    });
  });
});

describe("validateWebhookUrl", () => {
  it("discordのwebhook URLのみ許可する", () => {
    expect(validateWebhookUrl("https://discord.com/api/webhooks/123/abc")).toBeNull();
    expect(validateWebhookUrl(" https://discordapp.com/api/webhooks/1/t ")).toBeNull();
  });

  it("それ以外はエラーメッセージを返す", () => {
    expect(validateWebhookUrl("https://example.com/api/webhooks/1/t")).toBeTruthy();
    expect(validateWebhookUrl("http://discord.com/api/webhooks/1/t")).toBeTruthy();
    expect(validateWebhookUrl("")).toBeTruthy();
  });
});
