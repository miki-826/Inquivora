import { invoke } from "@tauri-apps/api/core";
import {
  meetingDecisionSchema,
  meetingSchema,
  meetingSummaryResultSchema,
  parseMeeting,
  segmentSchema,
  taskCandidateSchema,
  type Meeting,
  type MeetingDecision,
  type MeetingSummaryResult,
  type TaskCandidate,
  type TranscriptSegment,
} from "../features/meetings/meetingModel";

export type MeetingStartInput = {
  title: string;
  targetFilePath: string;
  workspaceId?: string | null;
  mic: boolean;
  loopback: boolean;
  micDeviceId?: string;
  loopbackDeviceId?: string;
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

export async function isSummaryAvailable(): Promise<boolean> {
  return (await invoke("meeting_summary_available")) === true;
}

export async function generateSummary(meetingId: string): Promise<MeetingSummaryResult> {
  return meetingSummaryResultSchema.parse(await invoke("meeting_generate_summary", { meetingId }));
}

export async function listDecisions(meetingId: string): Promise<MeetingDecision[]> {
  const value = await invoke("meeting_list_decisions", { meetingId });
  if (!Array.isArray(value)) {
    return [];
  }
  return value.flatMap((entry) => {
    const parsed = meetingDecisionSchema.safeParse(entry);
    return parsed.success ? [parsed.data] : [];
  });
}

export async function listCandidates(meetingId: string): Promise<TaskCandidate[]> {
  const value = await invoke("meeting_list_candidates", { meetingId });
  if (!Array.isArray(value)) {
    return [];
  }
  return value.flatMap((entry) => {
    const parsed = taskCandidateSchema.safeParse(entry);
    return parsed.success ? [parsed.data] : [];
  });
}
