import { formatInTimeZone } from "date-fns-tz";
import { z } from "zod";

export const meetingSchema = z.object({
  id: z.string(),
  workspaceId: z.string().nullish(),
  title: z.string(),
  startedAtUtc: z.string(),
  endedAtUtc: z.string().nullish(),
  timezone: z.string(),
  targetFilePath: z.string(),
  startMarker: z.string(),
  endMarker: z.string(),
  summary: z.string().nullish(),
  status: z.enum(["recording", "paused", "completed"]),
  createdAt: z.string(),
  updatedAt: z.string(),
});

export type Meeting = z.infer<typeof meetingSchema>;

export const segmentSchema = z.object({
  id: z.string(),
  meetingId: z.string(),
  source: z.string(),
  speakerLabel: z.string(),
  startMs: z.number(),
  endMs: z.number(),
  text: z.string(),
  status: z.string(),
  audioChunkPath: z.string().nullish(),
  createdAt: z.string(),
});

export type TranscriptSegment = z.infer<typeof segmentSchema>;

export function parseMeeting(value: unknown): Meeting | null {
  const parsed = meetingSchema.safeParse(value);
  return parsed.success ? parsed.data : null;
}

const STATUS_LABELS: Record<Meeting["status"], string> = {
  recording: "録音中",
  paused: "一時停止",
  completed: "終了",
};

export function meetingStatusLabel(status: Meeting["status"]): string {
  return STATUS_LABELS[status];
}

/// §9.5 Monaco Model側の挿入。終了マーカー直前へセグメントを差し込んだ本文を返す。
export function insertBeforeEndMarker(
  content: string,
  endMarker: string,
  segmentMarkdown: string,
): string | null {
  const markerPos = content.indexOf(endMarker);
  if (markerPos < 0) {
    return null;
  }
  let before = content.slice(0, markerPos);
  while (!before.endsWith("\n\n")) {
    before += "\n";
  }
  return `${before}${segmentMarkdown.trimEnd()}\n\n${content.slice(markerPos)}`;
}

/// 保存済みデバイスIDが現在の一覧に存在すればそれを、なければ既定デバイスを返す。
export function pickDeviceId(
  savedId: string | null | undefined,
  devices: { id: string }[],
): string {
  if (savedId && savedId !== "default" && devices.some((d) => d.id === savedId)) {
    return savedId;
  }
  return "default";
}

export function defaultMeetingFileName(title: string, now: Date): string {
  const date = formatInTimeZone(now, "Asia/Tokyo", "yyyy-MM-dd");
  const safeTitle = title.trim().replace(/[/\\:*?"<>|]/g, "-") || "会議";
  return `${date}_${safeTitle}.md`;
}
