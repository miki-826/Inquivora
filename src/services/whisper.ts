import { invoke } from "@tauri-apps/api/core";
import {
  parseWhisperStatus,
  type WhisperModelStatus,
} from "../features/settings/whisperModel";

export async function getWhisperModelStatus(): Promise<WhisperModelStatus[]> {
  return parseWhisperStatus(await invoke("whisper_model_status"));
}

export async function selectWhisperModel(name: string): Promise<void> {
  await invoke("whisper_model_select", { name });
}

export async function downloadWhisperModel(name: string): Promise<void> {
  await invoke("whisper_model_download", { name });
}

export async function deleteWhisperModel(name: string): Promise<void> {
  await invoke("whisper_model_delete", { name });
}
