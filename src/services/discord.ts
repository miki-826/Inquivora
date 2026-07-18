import { invoke } from "@tauri-apps/api/core";

export async function setDiscordWebhook(url: string): Promise<void> {
  await invoke("discord_webhook_set", { url });
}

export async function hasDiscordWebhook(): Promise<boolean> {
  return (await invoke("discord_webhook_has")) === true;
}

export async function deleteDiscordWebhook(): Promise<void> {
  await invoke("discord_webhook_delete");
}

export async function testDiscordWebhook(): Promise<void> {
  await invoke("discord_webhook_test");
}
