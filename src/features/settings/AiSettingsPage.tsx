import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";
import { ThreePaneLayout } from "../../components/layout/ThreePaneLayout";
import * as api from "../../services/providers";
import * as whisperApi from "../../services/whisper";
import {
  downloadPercent,
  formatModelSize,
  modelHint,
  type WhisperModelStatus,
} from "./whisperModel";
import type { ConnectionTestResult, FeatureBinding } from "../../services/providers";
import {
  PROVIDER_PRESETS,
  PROVIDER_TYPES,
  capabilitiesForType,
  isProviderType,
  providerTypeLabel,
  validateBaseUrl,
  type Provider,
  type ProviderType,
} from "./providerModel";
import { SettingsNav } from "./SettingsNav";

const SUMMARY_FEATURE_KEY = "meeting.summary";

function messageOf(error: unknown): string {
  if (error && typeof error === "object" && "message" in error) {
    return String((error as { message: unknown }).message);
  }
  return String(error);
}

type FormState = {
  id: string | null;
  providerType: ProviderType;
  baseUrl: string;
  apiKey: string;
  modelId: string;
  customPrompt: string;
};

function emptyForm(): FormState {
  const preset = PROVIDER_PRESETS.anthropic;
  return {
    id: null,
    providerType: "anthropic",
    baseUrl: preset.baseUrl,
    apiKey: "",
    modelId: preset.defaultModel,
    customPrompt: "",
  };
}

function formFromProvider(provider: Provider): FormState {
  const providerType: ProviderType = isProviderType(provider.providerType)
    ? provider.providerType
    : "anthropic";
  const preset = PROVIDER_PRESETS[providerType];
  return {
    id: provider.id,
    providerType,
    baseUrl: provider.baseUrl || preset.baseUrl,
    apiKey: "",
    modelId: provider.modelId ?? preset.defaultModel,
    customPrompt: provider.customPrompt ?? "",
  };
}

/// 表示名は種類ラベルから自動生成する。同種を複数登録した場合は連番を付けて一意にする。
function uniqueDisplayName(base: string, providers: Provider[], selfId: string | null): string {
  const taken = new Set(providers.filter((p) => p.id !== selfId).map((p) => p.displayName));
  if (!taken.has(base)) return base;
  for (let i = 2; i < 100; i += 1) {
    const candidate = `${base} ${i}`;
    if (!taken.has(candidate)) return candidate;
  }
  return `${base} ${Date.now()}`;
}

function ProviderForm({
  form,
  setForm,
  providers,
  onSaved,
  onCancel,
}: {
  form: FormState;
  setForm: (form: FormState) => void;
  providers: Provider[];
  onSaved: () => Promise<void>;
  onCancel: () => void;
}) {
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const preset = PROVIDER_PRESETS[form.providerType];

  const save = async () => {
    const model = form.modelId.trim() || preset.defaultModel;
    const baseUrl = preset.editableBaseUrl ? form.baseUrl.trim() : preset.baseUrl;
    if (preset.editableBaseUrl) {
      const urlError = validateBaseUrl(baseUrl);
      if (urlError) {
        setError(urlError);
        return;
      }
    }
    const input = {
      displayName: uniqueDisplayName(providerTypeLabel(form.providerType), providers, form.id),
      providerType: form.providerType,
      baseUrl,
      authType: preset.authType,
      modelId: model,
      customPrompt: form.customPrompt.trim() || null,
      timeoutMs: 60000,
      capabilities: capabilitiesForType(form.providerType),
    };
    setSaving(true);
    setError(null);
    try {
      let providerId = form.id;
      if (providerId) {
        await api.updateProvider(providerId, input);
      } else {
        const created = await api.createProvider(input);
        providerId = created?.id ?? null;
      }
      // APIキーは保存コマンドへ渡した直後にフォーム状態から破棄する
      if (providerId && preset.needsApiKey && form.apiKey.trim() !== "") {
        await api.setProviderSecret(providerId, form.apiKey);
      }
      // 登録したAIを議事録まとめに自動割当（この AI を使う）
      if (providerId) {
        await api.setFeatureBinding(SUMMARY_FEATURE_KEY, {
          providerProfileId: providerId,
          modelId: model,
          fallbackProviderProfileId: null,
          fallbackModelId: null,
        });
      }
      setForm({ ...form, apiKey: "" });
      await onSaved();
    } catch (err) {
      setError(messageOf(err));
    } finally {
      setSaving(false);
    }
  };

  return (
    <section className="settings-section">
      <h2 className="settings-section__title">{form.id ? "AIを編集" : "AIを追加"}</h2>
      <label className="settings-field">
        AIの種類
        <select
          value={form.providerType}
          onChange={(e) => {
            const providerType = e.target.value as ProviderType;
            const next = PROVIDER_PRESETS[providerType];
            setForm({
              ...form,
              providerType,
              baseUrl: next.baseUrl,
              modelId: next.defaultModel,
            });
          }}
        >
          {PROVIDER_TYPES.map((type) => (
            <option key={type} value={type}>
              {PROVIDER_PRESETS[type].label}
            </option>
          ))}
        </select>
      </label>
      {preset.editableBaseUrl && (
        <label className="settings-field">
          接続先URL
          <input
            type="text"
            value={form.baseUrl}
            placeholder={preset.baseUrl}
            onChange={(e) => setForm({ ...form, baseUrl: e.target.value })}
          />
        </label>
      )}
      {preset.needsApiKey && (
        <label className="settings-field">
          APIキー{form.id ? "（変更する場合のみ入力）" : ""}
          <input
            type="password"
            value={form.apiKey}
            autoComplete="off"
            placeholder="sk-..."
            onChange={(e) => setForm({ ...form, apiKey: e.target.value })}
          />
        </label>
      )}
      <label className="settings-field">
        モデル
        <input
          type="text"
          list={`models-${form.providerType}`}
          value={form.modelId}
          placeholder={preset.defaultModel}
          onChange={(e) => setForm({ ...form, modelId: e.target.value })}
        />
        <datalist id={`models-${form.providerType}`}>
          {preset.models.map((model) => (
            <option key={model} value={model} />
          ))}
        </datalist>
      </label>
      <label className="settings-field">
        プロンプト（任意）
        <textarea
          rows={4}
          value={form.customPrompt}
          placeholder="議事録のまとめ方の希望を書いてください（例: 決定事項を箇条書きで／敬体で など）"
          onChange={(e) => setForm({ ...form, customPrompt: e.target.value })}
        />
      </label>
      <p className="settings-note">
        プロンプトは議事録まとめAIへの追加の指示として使われます（出力の形式は保たれます）。
      </p>
      {error && (
        <p className="settings-actions__error" role="alert">
          {error}
        </p>
      )}
      <div className="settings-actions">
        <button type="button" disabled={saving} onClick={() => void save()}>
          {saving ? "保存中…" : "保存"}
        </button>
        <button type="button" onClick={onCancel}>
          キャンセル
        </button>
      </div>
    </section>
  );
}

function ProviderCard({
  provider,
  isSummaryProvider,
  onEdit,
  onUse,
  onReload,
}: {
  provider: Provider;
  isSummaryProvider: boolean;
  onEdit: () => void;
  onUse: () => Promise<void>;
  onReload: () => Promise<void>;
}) {
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<ConnectionTestResult | undefined>(undefined);

  const run = async (label: string, action: () => Promise<void>) => {
    setBusy(label);
    setError(null);
    try {
      await action();
    } catch (err) {
      setError(messageOf(err));
    } finally {
      setBusy(null);
    }
  };

  const typeLabel = isProviderType(provider.providerType)
    ? providerTypeLabel(provider.providerType)
    : provider.providerType;

  return (
    <div className="provider-card">
      <div className="provider-card__header">
        <span className="provider-card__name">{typeLabel}</span>
        {isSummaryProvider && <span className="provider-card__badge">議事録まとめに使用中</span>}
        {!provider.enabled && <span className="provider-card__disabled">無効</span>}
      </div>
      {provider.modelId && <div className="provider-card__meta">モデル: {provider.modelId}</div>}
      <div className="provider-card__meta">
        APIキー: {provider.hasSecret ? "設定済み" : "未設定"}
        {provider.lastTestedAt &&
          ` ／ 最終接続テスト: ${provider.lastTestStatus === "success" ? "成功" : "失敗"}`}
      </div>
      {result && (
        <div className={result.success ? "provider-card__ok" : "provider-card__error"}>
          {result.userMessage}
          {result.latencyMs != null && `（${result.latencyMs}ms）`}
        </div>
      )}
      {error && (
        <div className="provider-card__error" role="alert">
          {error}
        </div>
      )}
      <div className="provider-card__actions">
        <button type="button" onClick={onEdit}>
          編集
        </button>
        {!isSummaryProvider && provider.enabled && (
          <button
            type="button"
            disabled={busy !== null}
            onClick={() => void run("use", onUse)}
          >
            {busy === "use" ? "設定中…" : "このAIを使う"}
          </button>
        )}
        <button
          type="button"
          disabled={busy !== null}
          onClick={() =>
            void run("test", async () => {
              setResult(await api.testProvider(provider.id));
              await onReload();
            })
          }
        >
          {busy === "test" ? "テスト中…" : "接続テスト"}
        </button>
        <button
          type="button"
          disabled={busy !== null}
          onClick={() =>
            void run("enable", async () => {
              await api.enableProvider(provider.id, !provider.enabled);
              await onReload();
            })
          }
        >
          {provider.enabled ? "無効にする" : "有効にする"}
        </button>
        <button
          type="button"
          className="provider-card__danger"
          disabled={busy !== null}
          onClick={() => {
            if (window.confirm(`${typeLabel} を削除しますか？APIキーも削除されます。`)) {
              void run("delete", async () => {
                await api.deleteProvider(provider.id);
                await onReload();
              });
            }
          }}
        >
          削除
        </button>
      </div>
    </div>
  );
}

type DownloadProgress = {
  name: string;
  receivedBytes: number;
  totalBytes: number | null;
};

function WhisperSection() {
  const [models, setModels] = useState<WhisperModelStatus[]>([]);
  const [progress, setProgress] = useState<DownloadProgress | null>(null);
  const [downloading, setDownloading] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      setModels(await whisperApi.getWhisperModelStatus());
      setError(null);
    } catch (err) {
      setError(messageOf(err));
    }
  }, []);

  useEffect(() => {
    void whisperApi
      .getWhisperModelStatus()
      .then(setModels)
      .catch((err) => setError(messageOf(err)));
    let unlisten: (() => void) | null = null;
    void listen<DownloadProgress>("whisper:model-download-progress", (event) => {
      setProgress(event.payload);
    }).then((fn) => {
      unlisten = fn;
    });
    return () => {
      unlisten?.();
    };
  }, []);

  const select = async (name: string) => {
    try {
      await whisperApi.selectWhisperModel(name);
      await refresh();
    } catch (err) {
      setError(messageOf(err));
    }
  };

  const remove = async (name: string, label: string) => {
    if (!window.confirm(`${label} を削除しますか？必要になれば再ダウンロードできます。`)) {
      return;
    }
    try {
      await whisperApi.deleteWhisperModel(name);
      await refresh();
    } catch (err) {
      setError(messageOf(err));
    }
  };

  const download = async (name: string) => {
    setDownloading(name);
    setProgress(null);
    try {
      await whisperApi.downloadWhisperModel(name);
      await refresh();
    } catch (err) {
      setError(messageOf(err));
    } finally {
      setDownloading(null);
      setProgress(null);
    }
  };

  const percent =
    progress && downloading === progress.name
      ? downloadPercent(progress.receivedBytes, progress.totalBytes)
      : null;

  const downloadedSizeMb = models
    .filter((m) => m.downloaded)
    .reduce((sum, m) => sum + m.sizeMb, 0);

  return (
    <section className="settings-section">
      <h2 className="settings-section__title">ローカル文字起こし（内蔵Whisper）</h2>
      <p className="settings-note">
        文字起こしはAPIを設定しなくても、選択したWhisperモデルでPC内だけで処理します。
      </p>
      {error && (
        <p className="settings-actions__error" role="alert">
          {error}
        </p>
      )}
      <div className="whisper-models">
        {models.map((model) => {
          const hint = modelHint(model.name);
          return (
            <div key={model.name} className="whisper-model">
              <label className="whisper-model__select">
                <input
                  type="radio"
                  name="whisper-model"
                  checked={model.selected}
                  onChange={() => void select(model.name)}
                />
                <span className="whisper-model__name">
                  {model.displayName}
                  {hint.recommended && <span className="whisper-model__badge">推奨</span>}
                </span>
                <span className="whisper-model__size">{formatModelSize(model.sizeMb)}</span>
              </label>
              <p className="whisper-model__hint">{hint.tagline}</p>
              {model.downloaded ? (
                <div className="whisper-model__actions">
                  <span className="whisper-model__downloaded">ダウンロード済み</span>
                  <button
                    type="button"
                    className="provider-card__danger"
                    disabled={model.selected}
                    title={model.selected ? "使用中のモデルは削除できません" : undefined}
                    onClick={() => void remove(model.name, model.displayName)}
                  >
                    削除
                  </button>
                </div>
              ) : (
                <button
                  type="button"
                  disabled={downloading !== null}
                  onClick={() => void download(model.name)}
                >
                  {downloading === model.name
                    ? percent !== null
                      ? `ダウンロード中… ${percent}%`
                      : "ダウンロード中…"
                    : "ダウンロード"}
                </button>
              )}
            </div>
          );
        })}
      </div>
      {downloadedSizeMb > 0 && (
        <p className="settings-note">ダウンロード済み合計: {formatModelSize(downloadedSizeMb)}</p>
      )}
    </section>
  );
}

function UsageSection() {
  const [logs, setLogs] = useState<api.UsageLog[]>([]);
  useEffect(() => {
    void api
      .listUsage(null, 20)
      .then(setLogs)
      .catch((err) => console.error("使用量の取得に失敗", err));
  }, []);
  if (logs.length === 0) {
    return null;
  }
  return (
    <section className="settings-section">
      <h2 className="settings-section__title">最近のAPI使用</h2>
      <div className="usage-list">
        {logs.map((log) => (
          <div key={log.id} className="usage-list__row">
            <span>{log.createdAt.slice(0, 16).replace("T", " ")}</span>
            <span>{log.modelId}</span>
            <span>{log.status === "success" ? "成功" : `失敗 (${log.errorCode ?? "?"})`}</span>
            {log.audioDurationMs != null && <span>{Math.round(log.audioDurationMs / 1000)}秒</span>}
          </div>
        ))}
      </div>
    </section>
  );
}

export function AiSettingsPage() {
  const [providers, setProviders] = useState<Provider[]>([]);
  const [summaryBinding, setSummaryBinding] = useState<FeatureBinding | null>(null);
  const [form, setForm] = useState<FormState | null>(null);
  const [error, setError] = useState<string | null>(null);

  const reload = useCallback(async () => {
    try {
      const [nextProviders, binding] = await Promise.all([
        api.listProviders(),
        api.getFeatureBinding(SUMMARY_FEATURE_KEY),
      ]);
      setProviders(nextProviders);
      setSummaryBinding(binding);
      setError(null);
    } catch (err) {
      setError(messageOf(err));
    }
  }, []);

  useEffect(() => {
    void Promise.all([api.listProviders(), api.getFeatureBinding(SUMMARY_FEATURE_KEY)])
      .then(([nextProviders, binding]) => {
        setProviders(nextProviders);
        setSummaryBinding(binding);
      })
      .catch((err) => setError(messageOf(err)));
  }, []);

  const assignToSummary = async (provider: Provider) => {
    await api.setFeatureBinding(SUMMARY_FEATURE_KEY, {
      providerProfileId: provider.id,
      modelId: provider.modelId ?? PROVIDER_PRESETS[
        isProviderType(provider.providerType) ? provider.providerType : "anthropic"
      ].defaultModel,
      fallbackProviderProfileId: null,
      fallbackModelId: null,
    });
    await reload();
  };

  return (
    <ThreePaneLayout left={<SettingsNav />}>
      <div className="settings-page">
        <section className="settings-section">
          <h2 className="settings-section__title">AI（接続先）</h2>
          <p className="settings-note">
            議事録のまとめに使うAIを登録します。AIの種類を選び、APIキーとモデル、必要なら独自のプロンプトを入れるだけです。APIキーはあなたのPCのWindows資格情報マネージャーにのみ保存され、アプリのデータベースやログには残りません。
          </p>
          {error && (
            <p className="settings-actions__error" role="alert">
              {error}
            </p>
          )}
          {providers.map((provider) => (
            <ProviderCard
              key={provider.id}
              provider={provider}
              isSummaryProvider={summaryBinding?.providerProfileId === provider.id}
              onEdit={() => setForm(formFromProvider(provider))}
              onUse={() => assignToSummary(provider)}
              onReload={reload}
            />
          ))}
          {!form && (
            <div className="settings-actions">
              <button type="button" onClick={() => setForm(emptyForm())}>
                AIを追加
              </button>
            </div>
          )}
        </section>
        {form && (
          <ProviderForm
            form={form}
            setForm={setForm}
            providers={providers}
            onSaved={async () => {
              setForm(null);
              await reload();
            }}
            onCancel={() => setForm(null)}
          />
        )}
        <WhisperSection />
        <UsageSection />
      </div>
    </ThreePaneLayout>
  );
}
