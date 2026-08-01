import { describe, expect, it } from "vitest";
import {
  FEATURE_KEYS,
  PROVIDER_PRESETS,
  PROVIDER_TYPES,
  capabilitiesForType,
  featureLabel,
  isProviderType,
  parseProviderList,
  providerTypeLabel,
  retiredProviderNotice,
  validateBaseUrl,
} from "./providerModel";

const sampleProvider = {
  id: "p1",
  displayName: "OpenAI Personal",
  providerType: "openai",
  baseUrl: "https://api.openai.com/v1",
  authType: "bearer",
  organizationId: null,
  projectId: null,
  defaultHeaders: {},
  timeoutMs: 60000,
  capabilities: ["transcription.batch", "models.list"],
  enabled: true,
  lastTestStatus: null,
  lastTestedAt: null,
  createdAt: "2026-07-18T00:00:00Z",
  updatedAt: "2026-07-18T00:00:00Z",
  hasSecret: true,
};

describe("parseProviderList", () => {
  it("バックエンド応答を解析できる", () => {
    const providers = parseProviderList([sampleProvider]);
    expect(providers).toHaveLength(1);
    expect(providers[0].displayName).toBe("OpenAI Personal");
    expect(providers[0].hasSecret).toBe(true);
  });

  it("不正な要素は除外する", () => {
    const providers = parseProviderList([sampleProvider, { broken: true }]);
    expect(providers).toHaveLength(1);
  });
});

describe("validateBaseUrl", () => {
  it("httpsは許可される", () => {
    expect(validateBaseUrl("https://api.openai.com/v1")).toBeNull();
  });

  it("httpはローカルホストのみ許可される", () => {
    expect(validateBaseUrl("http://localhost:1234/v1")).toBeNull();
    expect(validateBaseUrl("http://127.0.0.1:8080/v1")).toBeNull();
    expect(validateBaseUrl("http://example.com/v1")).not.toBeNull();
  });

  it("空や非httpスキームは拒否される", () => {
    expect(validateBaseUrl("")).not.toBeNull();
    expect(validateBaseUrl("file:///c:/x")).not.toBeNull();
    expect(validateBaseUrl("api.openai.com")).not.toBeNull();
  });

  it("ユーザー情報付きURLでlocalhost制約を回避できない", () => {
    expect(validateBaseUrl("http://localhost:80@evil.example/v1")).not.toBeNull();
    expect(validateBaseUrl("https://user:pass@example.com/v1")).not.toBeNull();
  });

  it("IPv6ループバックを許可し空ホストを拒否する", () => {
    expect(validateBaseUrl("http://[::1]:8080/v1")).toBeNull();
    expect(validateBaseUrl("https://")).not.toBeNull();
  });
});

describe("capabilitiesForType", () => {
  it("openaiは文字起こしとテキスト生成へ対応する", () => {
    const capabilities = capabilitiesForType("openai");
    expect(capabilities).toContain("transcription.batch");
    expect(capabilities).toContain("text.generate");
    expect(capabilities).toContain("models.list");
  });

  it("全プロバイダーで既定の構成は同じ", () => {
    for (const type of PROVIDER_TYPES) {
      expect(capabilitiesForType(type)).toEqual(capabilitiesForType("openai"));
    }
  });
});

describe("プロバイダープリセット", () => {
  it("対応AIはChatGPTとGeminiの2種類", () => {
    expect([...PROVIDER_TYPES]).toEqual(["openai", "gemini"]);
  });

  it("各プリセットに既定URL・既定モデル・候補モデルがある", () => {
    for (const type of PROVIDER_TYPES) {
      const preset = PROVIDER_PRESETS[type];
      expect(preset.baseUrl.length).toBeGreaterThan(0);
      expect(preset.models).toContain(preset.defaultModel);
      expect(preset.summaryModels.map((model) => model.id)).toContain(
        preset.defaultSummaryModel,
      );
      expect(preset.transcriptionModels).toContain(preset.defaultTranscriptionModel);
      expect(providerTypeLabel(type)).toBe(preset.label);
    }
  });

  it("議事録候補は構造化出力対応を確認したモデルだけを持つ", () => {
    expect(PROVIDER_PRESETS.openai.summaryModels.map((model) => model.id)).toEqual([
      "gpt-5.6-sol",
      "gpt-5.6-terra",
      "gpt-5.6-luna",
      "gpt-5.4",
      "gpt-5.4-mini",
      "gpt-5-mini",
      "gpt-4.1",
      "gpt-4o",
      "gpt-4o-mini",
    ]);
    expect(PROVIDER_PRESETS.gemini.summaryModels.map((model) => model.id)).toEqual([
      "gemini-3.6-flash",
      "gemini-3.5-flash",
      "gemini-3.5-flash-lite",
    ]);
  });

  it("OpenAI文字起こしは音声対応モデルを既定にする", () => {
    expect(PROVIDER_PRESETS.openai.defaultTranscriptionModel).toBe("gpt-4o-mini-transcribe");
    expect(PROVIDER_PRESETS.openai.transcriptionModels).toContain("gpt-transcribe");
  });

  it("全プリセットがAPIキー必須・URL固定", () => {
    for (const type of PROVIDER_TYPES) {
      expect(PROVIDER_PRESETS[type].needsApiKey).toBe(true);
      expect(PROVIDER_PRESETS[type].editableBaseUrl).toBe(false);
    }
  });

  it("Geminiの既定モデルは現行の2.5系", () => {
    expect(PROVIDER_PRESETS.gemini.defaultModel).toBe("gemini-3.6-flash");
    expect(PROVIDER_PRESETS.gemini.models.every((model) => model.startsWith("gemini-3."))).toBe(
      true,
    );
    expect(PROVIDER_PRESETS.gemini.defaultTranscriptionModel).toBe("gemini-3.6-flash");
  });

  it("isProviderTypeは廃止したClaude/Ollamaを受理しない", () => {
    expect(isProviderType("openai")).toBe(true);
    expect(isProviderType("gemini")).toBe(true);
    expect(isProviderType("anthropic")).toBe(false);
    expect(isProviderType("ollama")).toBe(false);
    expect(isProviderType("openai_compatible")).toBe(false);
  });

  it("廃止した種類は日本語の説明付きで案内する", () => {
    expect(retiredProviderNotice("anthropic")).toContain("Claude");
    expect(retiredProviderNotice("ollama")).toContain("Ollama");
    expect(retiredProviderNotice("openai")).toBeNull();
  });
});

describe("featureLabel", () => {
  it("全ての機能キーへ日本語ラベルがある", () => {
    for (const key of FEATURE_KEYS) {
      expect(featureLabel(key)).not.toBe(key);
      expect(featureLabel(key).length).toBeGreaterThan(0);
    }
  });

  it("バッチ文字起こしのラベルを返す", () => {
    expect(featureLabel("transcription.batch")).toBe("バッチ文字起こし");
  });
});
