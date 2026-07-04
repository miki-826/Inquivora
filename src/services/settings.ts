import { invoke } from "@tauri-apps/api/core";

export async function loadSetting<T = unknown>(key: string): Promise<T | null> {
  return (await invoke<T | null>("settings_get", { key })) ?? null;
}

export async function saveSetting(key: string, value: unknown): Promise<void> {
  await invoke("settings_set", { key, value });
}
