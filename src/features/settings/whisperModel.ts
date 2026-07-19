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

export type ModelHint = {
  tagline: string;
  recommended: boolean;
};

const MODEL_HINTS: Record<string, ModelHint> = {
  tiny: { tagline: "高速・低精度。とりあえず試す・非力なPC向け", recommended: false },
  base: { tagline: "推奨。速度と精度のバランスが良く一般的なPC向け", recommended: true },
  small: { tagline: "高精度。日本語をしっかり認識したいとき向け（やや重い）", recommended: false },
};

/// 初心者向けにモデルの特徴を返す。未知のモデルでも汎用の説明を返す。
export function modelHint(name: string): ModelHint {
  return MODEL_HINTS[name] ?? { tagline: "文字起こし用モデル", recommended: false };
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
