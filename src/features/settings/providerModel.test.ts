import { describe, expect, it } from "vitest";
import {
  FEATURE_KEYS,
  capabilitiesForType,
  featureLabel,
  parseProviderList,
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

  it("openai_compatibleも既定で同じ構成になる", () => {
    expect(capabilitiesForType("openai_compatible")).toEqual(capabilitiesForType("openai"));
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
