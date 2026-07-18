import { z } from "zod";

export const PROVIDER_TYPES = ["openai", "openai_compatible"] as const;
export type ProviderType = (typeof PROVIDER_TYPES)[number];

export const FEATURE_KEYS = [
  "transcription.batch",
  "transcription.realtime",
  "meeting.summary",
  "editor.ai",
] as const;
export type FeatureKey = (typeof FEATURE_KEYS)[number];

const FEATURE_LABELS: Record<FeatureKey, string> = {
  "transcription.batch": "バッチ文字起こし",
  "transcription.realtime": "リアルタイム文字起こし",
  "meeting.summary": "議事録・タスク抽出",
  "editor.ai": "エディタAI",
};

export function featureLabel(key: FeatureKey): string {
  return FEATURE_LABELS[key];
}

export const providerSchema = z.object({
  id: z.string(),
  displayName: z.string(),
  providerType: z.string(),
  baseUrl: z.string(),
  authType: z.string(),
  organizationId: z.string().nullish(),
  projectId: z.string().nullish(),
  defaultHeaders: z.record(z.string(), z.string()).default({}),
  timeoutMs: z.number(),
  capabilities: z.array(z.string()).default([]),
  enabled: z.boolean(),
  lastTestStatus: z.string().nullish(),
  lastTestedAt: z.string().nullish(),
  createdAt: z.string(),
  updatedAt: z.string(),
  hasSecret: z.boolean().default(false),
});

export type Provider = z.infer<typeof providerSchema>;

export function parseProviderList(value: unknown): Provider[] {
  if (!Array.isArray(value)) {
    return [];
  }
  return value.flatMap((entry) => {
    const parsed = providerSchema.safeParse(entry);
    return parsed.success ? [parsed.data] : [];
  });
}

/// §10.3 base_url制約のフロント側検証。エラーメッセージまたはnullを返す。
export function validateBaseUrl(raw: string): string | null {
  const trimmed = raw.trim();
  if (trimmed.length === 0) {
    return "Base URLを入力してください";
  }
  const lower = trimmed.toLowerCase();
  if (lower.startsWith("https://")) {
    return lower.length > "https://".length ? null : "Base URLのホストがありません";
  }
  if (lower.startsWith("http://")) {
    const rest = lower.slice("http://".length);
    for (const host of ["localhost", "127.0.0.1", "[::1]"]) {
      if (rest === host || rest.startsWith(`${host}:`) || rest.startsWith(`${host}/`)) {
        return null;
      }
    }
    return "平文HTTPはローカルホスト（localhost / 127.0.0.1 / [::1]）のみ許可されます";
  }
  return "Base URLはhttps://で指定してください（ローカルAPIはhttp://localhost等）";
}

export function capabilitiesForType(providerType: ProviderType): string[] {
  void providerType;
  return ["transcription.batch", "text.generate", "text.structured_output", "models.list"];
}
