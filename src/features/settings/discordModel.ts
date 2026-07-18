import { z } from "zod";

export const discordSettingsSchema = z.object({
  enabled: z.boolean().default(false),
  realtime: z.boolean().default(false),
  summary: z.boolean().default(true),
});

export type DiscordSettings = z.infer<typeof discordSettingsSchema>;

export const DISCORD_SETTINGS_KEY = "discord";

export function parseDiscordSettings(value: unknown): DiscordSettings {
  const parsed = discordSettingsSchema.safeParse(value ?? {});
  return parsed.success ? parsed.data : { enabled: false, realtime: false, summary: true };
}

const WEBHOOK_PREFIXES = [
  "https://discord.com/api/webhooks/",
  "https://discordapp.com/api/webhooks/",
];

export function validateWebhookUrl(url: string): string | null {
  const trimmed = url.trim();
  if (WEBHOOK_PREFIXES.some((prefix) => trimmed.startsWith(prefix))) {
    return null;
  }
  return "Discord Webhook URL（https://discord.com/api/webhooks/…）を入力してください";
}
