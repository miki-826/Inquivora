import { invoke } from "@tauri-apps/api/core";
import {
  meetingSchema,
  parseMeeting,
  segmentSchema,
  type Meeting,
  type TranscriptSegment,
} from "../features/meetings/meetingModel";

export type MeetingStartInput = {
  title: string;
  targetFilePath: string;
  workspaceId?: string | null;
  mic: boolean;
  loopback: boolean;
  chunkSeconds?: number;
};

export type AudioDevice = {
  id: string;
  name: string;
  isDefault: boolean;
};

export async function startMeeting(input: MeetingStartInput): Promise<Meeting | null> {
  return parseMeeting(await invoke("meeting_start", { input }));
}

export async function pauseMeeting(meetingId: string): Promise<void> {
  await invoke("meeting_pause", { meetingId });
}

export async function resumeMeeting(meetingId: string): Promise<void> {
  await invoke("meeting_resume", { meetingId });
}

export async function stopMeeting(meetingId: string): Promise<Meeting | null> {
  return parseMeeting(await invoke("meeting_stop", { meetingId }));
}

export async function getMeeting(meetingId: string): Promise<Meeting | null> {
  return parseMeeting(await invoke("meeting_get", { meetingId }));
}

export async function listMeetings(limit?: number): Promise<Meeting[]> {
  const value = await invoke("meeting_list", { limit: limit ?? 100 });
  if (!Array.isArray(value)) {
    return [];
  }
  return value.flatMap((entry) => {
    const parsed = meetingSchema.safeParse(entry);
    return parsed.success ? [parsed.data] : [];
  });
}

export async function deleteMeeting(meetingId: string): Promise<void> {
  await invoke("meeting_delete", { meetingId });
}

export async function listSegments(meetingId: string): Promise<TranscriptSegment[]> {
  const value = await invoke("meeting_list_segments", { meetingId });
  if (!Array.isArray(value)) {
    return [];
  }
  return value.flatMap((entry) => {
    const parsed = segmentSchema.safeParse(entry);
    return parsed.success ? [parsed.data] : [];
  });
}

export async function appendSegmentToFile(
  meetingId: string,
  segmentMarkdown: string,
): Promise<void> {
  await invoke("meeting_append_segment", { meetingId, segmentMarkdown });
}

export async function listAudioDevices(): Promise<{ mic: AudioDevice[]; loopback: AudioDevice[] }> {
  const value = await invoke<{ mic?: AudioDevice[]; loopback?: AudioDevice[] }>(
    "meeting_list_devices",
  );
  return { mic: value.mic ?? [], loopback: value.loopback ?? [] };
}
