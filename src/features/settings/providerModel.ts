import { z } from "zod";

export const PROVIDER_TYPES = ["openai", "gemini"] as const;
export type ProviderType = (typeof PROVIDER_TYPES)[number];

export type ProviderPreset = {
  label: string;
  baseUrl: string;
  authType: string;
  needsApiKey: boolean;
  editableBaseUrl: boolean;
  models: string[];
  defaultModel: string;
  summaryModels: SummaryModelOption[];
  defaultSummaryModel: string;
  transcriptionModels: string[];
  defaultTranscriptionModel: string;
};

export type SummaryModelOption = {
  id: string;
  label: string;
};

/// 対応AIごとの既定設定。UIはこの表だけを見せ、Base URLや認証方式の手入力を不要にする。
export const PROVIDER_PRESETS: Record<ProviderType, ProviderPreset> = {
  openai: {
    label: "ChatGPT（OpenAI）",
    baseUrl: "https://api.openai.com/v1",
    authType: "bearer",
    needsApiKey: true,
    editableBaseUrl: false,
    models: ["gpt-4o", "gpt-4o-mini", "gpt-4.1", "o4-mini"],
    defaultModel: "gpt-4o-mini",
    summaryModels: [
      { id: "gpt-5.6-sol", label: "GPT-5.6 Sol（最高品質）" },
      { id: "gpt-5.6-terra", label: "GPT-5.6 Terra（品質・コストのバランス）" },
      { id: "gpt-5.6-luna", label: "GPT-5.6 Luna（高速・低コスト）" },
      { id: "gpt-5.4", label: "GPT-5.4（高品質）" },
      { id: "gpt-5.4-mini", label: "GPT-5.4 mini（軽量）" },
      { id: "gpt-5-mini", label: "GPT-5 mini（軽量）" },
      { id: "gpt-4.1", label: "GPT-4.1（従来互換）" },
      { id: "gpt-4o", label: "GPT-4o（従来互換）" },
      { id: "gpt-4o-mini", label: "GPT-4o mini（従来の低コスト）" },
    ],
    defaultSummaryModel: "gpt-5.6-terra",
    transcriptionModels: [
      "gpt-transcribe",
      "gpt-4o-transcribe",
      "gpt-4o-mini-transcribe",
      "whisper-1",
    ],
    defaultTranscriptionModel: "gpt-4o-mini-transcribe",
  },
  gemini: {
    label: "Gemini（Google）",
    baseUrl: "https://generativelanguage.googleapis.com/v1beta",
    authType: "x-goog-api-key",
    needsApiKey: true,
    editableBaseUrl: false,
    models: ["gemini-3.6-flash", "gemini-3.5-flash", "gemini-3.5-flash-lite"],
    defaultModel: "gemini-3.6-flash",
    summaryModels: [
      { id: "gemini-3.6-flash", label: "Gemini 3.6 Flash（推奨）" },
      { id: "gemini-3.5-flash", label: "Gemini 3.5 Flash（従来互換）" },
      { id: "gemini-3.5-flash-lite", label: "Gemini 3.5 Flash-Lite（高速・低コスト）" },
    ],
    defaultSummaryModel: "gemini-3.6-flash",
    transcriptionModels: ["gemini-3.6-flash", "gemini-3.5-flash", "gemini-3.5-flash-lite"],
    defaultTranscriptionModel: "gemini-3.6-flash",
  },
};

export const DEFAULT_PROVIDER_TYPE: ProviderType = "openai";

export function providerTypeLabel(type: ProviderType): string {
  return PROVIDER_PRESETS[type].label;
}

export function isProviderType(value: string): value is ProviderType {
  return (PROVIDER_TYPES as readonly string[]).includes(value);
}

/// 保存済みだが対応をやめた種類の案内文。無い場合はnull。
const RETIRED_PROVIDER_LABELS: Record<string, string> = {
  anthropic: "Claude（Anthropic）",
  ollama: "Ollama（ローカル）",
  openai_compatible: "OpenAI互換",
};

export function retiredProviderNotice(providerType: string): string | null {
  const label = RETIRED_PROVIDER_LABELS[providerType];
  if (!label) return null;
  return `${label} は現在サポートしていません。削除して ChatGPT か Gemini を登録してください。`;
}

export const FEATURE_KEYS = [
  "transcription.batch",
  "transcription.realtime",
  "meeting.summary",
  "editor.ai",
] as const;
export type FeatureKey = (typeof FEATURE_KEYS)[number];
export const ACTIVE_FEATURE_KEYS = ["transcription.batch", "meeting.summary"] as const satisfies readonly FeatureKey[];

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
  modelId: z.string().nullish(),
  customPrompt: z.string().nullish(),
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
  let url: URL;
  try {
    url = new URL(trimmed);
  } catch {
    return "Base URLの形式が正しくありません";
  }
  if (url.username || url.password) {
    return "Base URLにユーザー名やパスワードは指定できません";
  }
  if (!url.hostname) return "Base URLのホストがありません";
  if (url.protocol === "https:") return null;
  if (url.protocol === "http:") {
    const host = url.hostname.toLowerCase();
    const loopback = host === "localhost" || host === "[::1]" || /^127(?:\.\d{1,3}){3}$/.test(host);
    if (loopback) return null;
    return "平文HTTPはローカルホスト（localhost / 127.0.0.1 / [::1]）のみ許可されます";
  }
  return "Base URLはhttps://で指定してください（ローカルAPIはhttp://localhost等）";
}

export function capabilitiesForType(providerType: ProviderType): string[] {
  void providerType;
  return ["transcription.batch", "text.generate", "text.structured_output", "models.list"];
}
