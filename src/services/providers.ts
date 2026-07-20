import { invoke } from "@tauri-apps/api/core";
import {
  parseProviderList,
  providerSchema,
  type FeatureKey,
  type Provider,
} from "../features/settings/providerModel";

export type ProviderInput = {
  displayName: string;
  providerType: string;
  baseUrl: string;
  authType: string;
  organizationId?: string | null;
  projectId?: string | null;
  defaultHeaders?: Record<string, string>;
  timeoutMs?: number;
  capabilities: string[];
};

export type ProviderPatch = Partial<ProviderInput>;

export type ConnectionTestResult = {
  success: boolean;
  checkedAt: string;
  latencyMs?: number | null;
  authenticated: boolean;
  modelsEndpointAvailable?: boolean | null;
  capabilities: string[];
  errorCode?: string | null;
  userMessage?: string | null;
};

export type FeatureBinding = {
  featureKey: string;
  providerProfileId: string | null;
  modelId: string | null;
  fallbackProviderProfileId: string | null;
  fallbackModelId: string | null;
  updatedAt: string;
};

export type UsageLog = {
  id: string;
  providerProfileId: string;
  featureKey: string;
  modelId: string;
  entityId: string | null;
  audioDurationMs: number | null;
  latencyMs: number | null;
  status: string;
  errorCode: string | null;
  createdAt: string;
};

export type AiDisclosure = {
  mode: "api" | "local";
  providerName: string | null;
  modelId: string | null;
  sendTarget: string;
};

export async function listProviders(): Promise<Provider[]> {
  return parseProviderList(await invoke("api_provider_list"));
}

export async function getProvider(id: string): Promise<Provider | null> {
  const parsed = providerSchema.safeParse(await invoke("api_provider_get", { id }));
  return parsed.success ? parsed.data : null;
}

export async function createProvider(input: ProviderInput): Promise<Provider | null> {
  const parsed = providerSchema.safeParse(await invoke("api_provider_create", { input }));
  return parsed.success ? parsed.data : null;
}

export async function updateProvider(id: string, patch: ProviderPatch): Promise<void> {
  await invoke("api_provider_update", { id, patch });
}

export async function deleteProvider(id: string): Promise<void> {
  await invoke("api_provider_delete", { id });
}

export async function enableProvider(id: string, enabled: boolean): Promise<void> {
  await invoke("api_provider_enable", { id, enabled });
}

export async function setProviderSecret(id: string, secret: string): Promise<void> {
  await invoke("api_provider_set_secret", { id, secret });
}

export async function deleteProviderSecret(id: string): Promise<void> {
  await invoke("api_provider_delete_secret", { id });
}

export async function testProvider(id: string): Promise<ConnectionTestResult> {
  return await invoke<ConnectionTestResult>("api_provider_test", { id });
}

export async function listProviderModels(id: string): Promise<string[]> {
  return await invoke<string[]>("api_provider_list_models", { id });
}

export async function getFeatureBinding(featureKey: string): Promise<FeatureBinding | null> {
  return await invoke<FeatureBinding | null>("ai_feature_binding_get", { featureKey });
}

export async function setFeatureBinding(
  featureKey: string,
  input: {
    providerProfileId?: string | null;
    modelId?: string | null;
    fallbackProviderProfileId?: string | null;
    fallbackModelId?: string | null;
  },
): Promise<void> {
  await invoke("ai_feature_binding_set", { featureKey, input });
}

export async function listUsage(
  providerProfileId?: string | null,
  limit?: number,
): Promise<UsageLog[]> {
  return await invoke<UsageLog[]>("api_usage_list", {
    providerProfileId: providerProfileId ?? null,
    limit: limit ?? 100,
  });
}

export async function getAiDisclosure(featureKey: FeatureKey): Promise<AiDisclosure> {
  const [providers, binding] = await Promise.all([
    listProviders(),
    getFeatureBinding(featureKey),
  ]);
  const required = featureKey === "transcription.batch"
    ? "transcription.batch"
    : "text.structured_output";
  const candidates = binding
    ? [
        [binding.providerProfileId, binding.modelId],
        [binding.fallbackProviderProfileId, binding.fallbackModelId],
      ]
    : [];
  for (const [providerId, modelId] of candidates) {
    const provider = providers.find(
      (item) => item.id === providerId && item.enabled && item.capabilities.includes(required),
    );
    if (provider && modelId) {
      return {
        mode: "api",
        providerName: provider.displayName,
        modelId,
        sendTarget:
          featureKey === "transcription.batch"
            ? "録音した音声チャンク"
            : "文字起こしと会議メモ",
      };
    }
  }
  return {
    mode: "local",
    providerName: null,
    modelId: null,
    sendTarget: "録音した音声チャンク（PC内だけで処理）",
  };
}
