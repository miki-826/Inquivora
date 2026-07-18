import { z } from "zod";

export const whisperModelStatusSchema = z.object({
  name: z.string(),
  displayName: z.string(),
  sizeMb: z.number(),
  downloaded: z.boolean(),
  selected: z.boolean(),
});

export type WhisperModelStatus = z.infer<typeof whisperModelStatusSchema>;

export function parseWhisperStatus(value: unknown): WhisperModelStatus[] {
  if (!Array.isArray(value)) {
    return [];
  }
  return value.flatMap((entry) => {
    const parsed = whisperModelStatusSchema.safeParse(entry);
    return parsed.success ? [parsed.data] : [];
  });
}

export function formatModelSize(sizeMb: number): string {
  if (sizeMb >= 1000) {
    return `約${(sizeMb / 1000).toFixed(1)}GB`;
  }
  return `約${sizeMb}MB`;
}

export function downloadPercent(receivedBytes: number, totalBytes: number | null): number | null {
  if (!totalBytes) {
    return null;
  }
  return Math.min(100, Math.round((receivedBytes / totalBytes) * 100));
}
