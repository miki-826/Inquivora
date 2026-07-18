import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";
import { ThreePaneLayout } from "../../components/layout/ThreePaneLayout";
import * as discordApi from "../../services/discord";
import * as api from "../../services/providers";
import { loadSetting, saveSetting } from "../../services/settings";
import * as whisperApi from "../../services/whisper";
import {
  DISCORD_SETTINGS_KEY,
  parseDiscordSettings,
  validateWebhookUrl,
  type DiscordSettings,
} from "./discordModel";
import {
  downloadPercent,
  formatModelSize,
  type WhisperModelStatus,
} from "./whisperModel";
import type { ConnectionTestResult, FeatureBinding } from "../../services/providers";
import {
  FEATURE_KEYS,
  capabilitiesForType,
  featureLabel,
  validateBaseUrl,
  type FeatureKey,
  type Provider,
  type ProviderType,
} from "./providerModel";
import { SettingsNav } from "./SettingsNav";

function messageOf(error: unknown): string {
  if (error && typeof error === "object" && "message" in error) {
    return String((error as { message: unknown }).message);
  }
  return String(error);
}

type FormState = {
  id: string | null;
  displayName: string;
  providerType: ProviderType;
  baseUrl: string;
  authType: string;
  apiKey: string;
  organizationId: string;
  projectId: string;
  timeoutSeconds: string;
};

function emptyForm(): FormState {
  return {
    id: null,
    displayName: "",
    providerType: "openai",
    baseUrl: "https://api.openai.com/v1",
    authType: "bearer",
    apiKey: "",
    organizationId: "",
    projectId: "",
    timeoutSeconds: "60",
  };
}

function formFromProvider(provider: Provider): FormState {
  return {
    id: provider.id,
    displayName: provider.displayName,
    providerType: provider.providerType === "openai_compatible" ? "openai_compatible" : "openai",
    baseUrl: provider.baseUrl,
    authType: provider.authType,
    apiKey: "",
    organizationId: provider.organizationId ?? "",
    projectId: provider.projectId ?? "",
    timeoutSeconds: String(Math.round(provider.timeoutMs / 1000)),
  };
}

function ProviderForm({
  form,
  setForm,
  onSaved,
  onCancel,
}: {
  form: FormState;
  setForm: (form: FormState) => void;
  onSaved: () => Promise<void>;
  onCancel: () => void;
}) {
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const save = async () => {
    if (form.displayName.trim() === "") {
      setError("表示名を入力してください");
      return;
    }
    const urlError = validateBaseUrl(form.baseUrl);
    if (urlError) {
      setError(urlError);
      return;
    }
    const timeoutSeconds = Number(form.timeoutSeconds);
    const input = {
      displayName: form.displayName.trim(),
      providerType: form.providerType,
      baseUrl: form.baseUrl.trim(),
      authType: form.authType,
      organizationId: form.organizationId.trim() || null,
      projectId: form.projectId.trim() || null,
      timeoutMs: Number.isFinite(timeoutSeconds) && timeoutSeconds > 0 ? timeoutSeconds * 1000 : 60000,
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
      // §10.4 APIキーは保存コマンドへ渡した直後にフォーム状態から破棄する
      if (providerId && form.apiKey.trim() !== "") {
        await api.setProviderSecret(providerId, form.apiKey);
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
      <h2 className="settings-section__title">
        {form.id ? "Providerを編集" : "Providerを追加"}
      </h2>
      <label className="settings-field">
        表示名
        <input
          type="text"
          value={form.displayName}
          placeholder="OpenAI Personal"
          onChange={(e) => setForm({ ...form, displayName: e.target.value })}
        />
      </label>
      <label className="settings-field">
        Provider種別
        <select
          value={form.providerType}
          onChange={(e) => {
            const providerType = e.target.value as ProviderType;
            setForm({
              ...form,
              providerType,
              baseUrl:
                providerType === "openai" && form.baseUrl.trim() === ""
                  ? "https://api.openai.com/v1"
                  : form.baseUrl,
            });
          }}
        >
          <option value="openai">OpenAI</option>
          <option value="openai_compatible">OpenAI互換</option>
        </select>
      </label>
      <label className="settings-field">
        Base URL
        <input
          type="text"
          value={form.baseUrl}
          placeholder="https://api.openai.com/v1"
          onChange={(e) => setForm({ ...form, baseUrl: e.target.value })}
        />
      </label>
      <label className="settings-field">
        認証方式
        <select value={form.authType} onChange={(e) => setForm({ ...form, authType: e.target.value })}>
          <option value="bearer">Bearer</option>
          <option value="x-api-key">x-api-key</option>
          <option value="none">なし</option>
        </select>
      </label>
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
      <label className="settings-field">
        Organization ID（任意）
        <input
          type="text"
          value={form.organizationId}
          onChange={(e) => setForm({ ...form, organizationId: e.target.value })}
        />
      </label>
      <label className="settings-field">
        Project ID（任意）
        <input
          type="text"
          value={form.projectId}
          onChange={(e) => setForm({ ...form, projectId: e.target.value })}
        />
      </label>
      <label className="settings-field">
        タイムアウト（秒）
        <input
          type="number"
          min={1}
          value={form.timeoutSeconds}
          onChange={(e) => setForm({ ...form, timeoutSeconds: e.target.value })}
        />
      </label>
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
  testResult,
  onEdit,
  onReload,
}: {
  provider: Provider;
  testResult: ConnectionTestResult | undefined;
  onEdit: () => void;
  onReload: () => Promise<void>;
}) {
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<ConnectionTestResult | undefined>(testResult);
  const [modelList, setModelList] = useState<string[] | null>(null);

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

  return (
    <div className="provider-card">
      <div className="provider-card__header">
        <span className="provider-card__name">{provider.displayName}</span>
        <span className="provider-card__type">
          {provider.providerType === "openai" ? "OpenAI" : "OpenAI互換"}
        </span>
        {!provider.enabled && <span className="provider-card__disabled">無効</span>}
      </div>
      <div className="provider-card__meta">{provider.baseUrl}</div>
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
      {modelList && (
        <div className="provider-card__meta">
          モデル: {modelList.slice(0, 8).join(", ")}
          {modelList.length > 8 && ` ほか${modelList.length - 8}件`}
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
            void run("models", async () => {
              setModelList(await api.listProviderModels(provider.id));
            })
          }
        >
          {busy === "models" ? "取得中…" : "モデル一覧"}
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
            if (window.confirm(`${provider.displayName} を削除しますか？APIキーも削除されます。`)) {
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

function BindingRow({
  featureKey,
  providers,
}: {
  featureKey: FeatureKey;
  providers: Provider[];
}) {
  const [binding, setBinding] = useState<FeatureBinding | null>(null);
  const [loaded, setLoaded] = useState(false);
  const [models, setModels] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    void api
      .getFeatureBinding(featureKey)
      .then((value) => {
        if (active) {
          setBinding(value);
          setLoaded(true);
        }
      })
      .catch((err) => {
        if (active) {
          setError(messageOf(err));
          setLoaded(true);
        }
      });
    return () => {
      active = false;
    };
  }, [featureKey]);

  const save = (providerProfileId: string | null, modelId: string | null) => {
    const next: FeatureBinding = {
      featureKey,
      providerProfileId,
      modelId,
      fallbackProviderProfileId: binding?.fallbackProviderProfileId ?? null,
      fallbackModelId: binding?.fallbackModelId ?? null,
      updatedAt: new Date().toISOString(),
    };
    setBinding(next);
    void api
      .setFeatureBinding(featureKey, {
        providerProfileId,
        modelId,
        fallbackProviderProfileId: next.fallbackProviderProfileId,
        fallbackModelId: next.fallbackModelId,
      })
      .catch((err) => setError(messageOf(err)));
  };

  const fetchModels = async () => {
    if (!binding?.providerProfileId) {
      return;
    }
    try {
      setModels(await api.listProviderModels(binding.providerProfileId));
      setError(null);
    } catch (err) {
      setError(messageOf(err));
    }
  };

  if (!loaded) {
    return null;
  }

  const datalistId = `models-${featureKey}`;
  return (
    <div className="binding-row">
      <div className="binding-row__label">{featureLabel(featureKey)}</div>
      <select
        value={binding?.providerProfileId ?? ""}
        onChange={(e) => save(e.target.value || null, binding?.modelId ?? null)}
      >
        <option value="">未設定</option>
        {providers
          .filter((p) => p.enabled)
          .map((p) => (
            <option key={p.id} value={p.id}>
              {p.displayName}
            </option>
          ))}
      </select>
      <input
        type="text"
        list={datalistId}
        placeholder="モデルID（手入力可）"
        value={binding?.modelId ?? ""}
        onChange={(e) => save(binding?.providerProfileId ?? null, e.target.value || null)}
      />
      <datalist id={datalistId}>
        {models.map((model) => (
          <option key={model} value={model} />
        ))}
      </datalist>
      <button type="button" disabled={!binding?.providerProfileId} onClick={() => void fetchModels()}>
        取得
      </button>
      {error && (
        <span className="settings-actions__error" role="alert">
          {error}
        </span>
      )}
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

  return (
    <section className="settings-section">
      <h2 className="settings-section__title">ローカル文字起こし（内蔵Whisper）</h2>
      <p className="settings-note">
        APIを設定しなくても、選択したWhisperモデルで文字起こしをローカル処理します。
        API Providerを「バッチ文字起こし」に割り当てた場合はAPIを優先します。
      </p>
      {error && (
        <p className="settings-actions__error" role="alert">
          {error}
        </p>
      )}
      <div className="whisper-models">
        {models.map((model) => (
          <div key={model.name} className="whisper-model">
            <label className="whisper-model__select">
              <input
                type="radio"
                name="whisper-model"
                checked={model.selected}
                onChange={() => void select(model.name)}
              />
              <span>{model.displayName}</span>
              <span className="whisper-model__size">{formatModelSize(model.sizeMb)}</span>
            </label>
            {model.downloaded ? (
              <span className="whisper-model__downloaded">ダウンロード済み</span>
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
        ))}
      </div>
    </section>
  );
}

function DiscordSection() {
  const [settings, setSettings] = useState<DiscordSettings | null>(null);
  const [hasWebhook, setHasWebhook] = useState(false);
  const [webhookUrl, setWebhookUrl] = useState("");
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void loadSetting(DISCORD_SETTINGS_KEY)
      .then((stored) => setSettings(parseDiscordSettings(stored)))
      .catch((err) => setError(messageOf(err)));
    void discordApi
      .hasDiscordWebhook()
      .then(setHasWebhook)
      .catch(() => setHasWebhook(false));
  }, []);

  if (!settings) return null;

  const update = (partial: Partial<DiscordSettings>) => {
    const next = { ...settings, ...partial };
    setSettings(next);
    void saveSetting(DISCORD_SETTINGS_KEY, next).catch((err) => setError(messageOf(err)));
  };

  const saveWebhook = async () => {
    const validationError = validateWebhookUrl(webhookUrl);
    if (validationError) {
      setError(validationError);
      return;
    }
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      await discordApi.setDiscordWebhook(webhookUrl.trim());
      setWebhookUrl("");
      setHasWebhook(true);
      setMessage("Webhook URLを保存しました");
    } catch (err) {
      setError(messageOf(err));
    } finally {
      setBusy(false);
    }
  };

  const removeWebhook = async () => {
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      await discordApi.deleteDiscordWebhook();
      setHasWebhook(false);
      setMessage("Webhook URLを削除しました");
    } catch (err) {
      setError(messageOf(err));
    } finally {
      setBusy(false);
    }
  };

  const testWebhook = async () => {
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      await discordApi.testDiscordWebhook();
      setMessage("テスト投稿を送信しました。Discordのチャンネルを確認してください");
    } catch (err) {
      setError(messageOf(err));
    } finally {
      setBusy(false);
    }
  };

  return (
    <section className="settings-section">
      <h2 className="settings-section__title">Discord連携</h2>
      <p className="settings-note">
        Webhook URLを登録すると、会議の文字起こしをDiscordのチャンネルへEmbedカードで投稿できます。
        URLはAPIキーと同様にWindows Credential Managerへ保存されます。
      </p>
      <label className="settings-field">
        Webhook URL {hasWebhook && <span className="settings-actions__ok">（登録済み）</span>}
        <input
          type="password"
          placeholder={hasWebhook ? "登録済み（変更する場合のみ入力）" : "https://discord.com/api/webhooks/…"}
          value={webhookUrl}
          onChange={(e) => setWebhookUrl(e.target.value)}
        />
      </label>
      <div className="settings-actions">
        <button type="button" disabled={busy || !webhookUrl.trim()} onClick={() => void saveWebhook()}>
          保存
        </button>
        <button type="button" disabled={busy || !hasWebhook} onClick={() => void testWebhook()}>
          テスト投稿
        </button>
        {hasWebhook && (
          <button
            type="button"
            className="provider-card__danger"
            disabled={busy}
            onClick={() => void removeWebhook()}
          >
            削除
          </button>
        )}
      </div>
      <label className="settings-field settings-field--toggle">
        <input
          type="checkbox"
          checked={settings.enabled}
          onChange={(e) => update({ enabled: e.target.checked })}
        />
        Discordへの投稿を有効にする
      </label>
      <label className="settings-field settings-field--toggle">
        <input
          type="checkbox"
          checked={settings.realtime}
          disabled={!settings.enabled}
          onChange={(e) => update({ realtime: e.target.checked })}
        />
        確定セグメントごとにリアルタイム投稿する
      </label>
      <label className="settings-field settings-field--toggle">
        <input
          type="checkbox"
          checked={settings.summary}
          disabled={!settings.enabled}
          onChange={(e) => update({ summary: e.target.checked })}
        />
        会議終了時に文字起こし全文をまとめて投稿する
      </label>
      {message && <p className="settings-actions__ok">{message}</p>}
      {error && (
        <p className="settings-actions__error" role="alert">
          {error}
        </p>
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
  const [form, setForm] = useState<FormState | null>(null);
  const [error, setError] = useState<string | null>(null);

  const reload = useCallback(async () => {
    try {
      setProviders(await api.listProviders());
      setError(null);
    } catch (err) {
      setError(messageOf(err));
    }
  }, []);

  useEffect(() => {
    void api
      .listProviders()
      .then(setProviders)
      .catch((err) => setError(messageOf(err)));
  }, []);

  return (
    <ThreePaneLayout left={<SettingsNav />}>
      <div className="settings-page">
        <section className="settings-section">
          <h2 className="settings-section__title">API Provider</h2>
          <p className="settings-note">
            APIキーはWindows Credential Managerへ保存され、アプリのデータベースやログには含まれません。
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
              testResult={undefined}
              onEdit={() => setForm(formFromProvider(provider))}
              onReload={reload}
            />
          ))}
          {!form && (
            <div className="settings-actions">
              <button type="button" onClick={() => setForm(emptyForm())}>
                Providerを追加
              </button>
            </div>
          )}
        </section>
        {form && (
          <ProviderForm
            form={form}
            setForm={setForm}
            onSaved={async () => {
              setForm(null);
              await reload();
            }}
            onCancel={() => setForm(null)}
          />
        )}
        <WhisperSection />
        <section className="settings-section">
          <h2 className="settings-section__title">用途別モデル設定</h2>
          <p className="settings-note">
            モデル一覧の取得に失敗しても、モデルIDを直接入力すれば使用できます。
          </p>
          {FEATURE_KEYS.map((key) => (
            <BindingRow key={key} featureKey={key} providers={providers} />
          ))}
        </section>
        <DiscordSection />
        <UsageSection />
      </div>
    </ThreePaneLayout>
  );
}
