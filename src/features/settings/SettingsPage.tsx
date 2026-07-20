import { disable, enable, isEnabled } from "@tauri-apps/plugin-autostart";
import { useEffect, useState } from "react";
import { ThreePaneLayout } from "../../components/layout/ThreePaneLayout";
import { reconcileNotifications, sendTestNotification } from "../../services/reminders";
import { loadSetting, saveSetting } from "../../services/settings";
import {
  parseNotificationSettings,
  type NotificationSettings,
} from "../notifications/notificationModel";
import { useThemeStore } from "../../stores/useThemeStore";
import { useSettingsStore } from "../../stores/useSettingsStore";
import { SettingsNav } from "./SettingsNav";
import { THEME_OPTIONS } from "./themeModel";

const NOTIFICATION_SETTING_KEY = "notifications";

function NotificationSettingsSection() {
  const [settings, setSettings] = useState<NotificationSettings | null>(null);
  const [testState, setTestState] = useState<"idle" | "sending" | "ok" | "error">("idle");
  const [testError, setTestError] = useState<string | null>(null);

  useEffect(() => {
    void loadSetting(NOTIFICATION_SETTING_KEY).then((stored) =>
      setSettings(parseNotificationSettings(stored)),
    );
  }, []);

  if (!settings) return null;

  const update = (partial: Partial<NotificationSettings>) => {
    const next = { ...settings, ...partial };
    setSettings(next);
    void saveSetting(NOTIFICATION_SETTING_KEY, next).then(() => reconcileNotifications());
  };

  const sendTest = async () => {
    setTestState("sending");
    setTestError(null);
    try {
      await sendTestNotification();
      setTestState("ok");
    } catch (err) {
      setTestState("error");
      setTestError(err instanceof Error ? err.message : String(err));
    }
  };

  const leadMinutes = (value: string): number | null => {
    if (value === "") return null;
    const parsed = Number(value);
    return Number.isInteger(parsed) && parsed >= 0 ? parsed : null;
  };

  return (
    <section className="settings-section">
      <h2 className="settings-section__title">通知</h2>
      <label className="settings-field settings-field--toggle">
        <input
          type="checkbox"
          checked={settings.enabled}
          onChange={(e) => update({ enabled: e.target.checked })}
        />
        通知を有効にする
      </label>
      <label className="settings-field settings-field--toggle">
        <input
          type="checkbox"
          checked={settings.sound}
          disabled={!settings.enabled}
          onChange={(e) => update({ sound: e.target.checked })}
        />
        通知音を鳴らす
      </label>
      <label className="settings-field">
        既定通知時刻（日付のみの期日・終日予定）
        <input
          type="time"
          value={settings.defaultNotifyTime}
          disabled={!settings.enabled}
          onChange={(e) => update({ defaultNotifyTime: e.target.value || "09:00" })}
        />
      </label>
      <label className="settings-field">
        タスクの既定リマインド（期日の何分前・空欄で自動作成しない）
        <input
          type="number"
          min={0}
          value={settings.taskLeadMinutes ?? ""}
          placeholder="なし"
          disabled={!settings.enabled}
          onChange={(e) => update({ taskLeadMinutes: leadMinutes(e.target.value) })}
        />
      </label>
      <label className="settings-field">
        予定の既定リマインド（開始の何分前・空欄で自動作成しない）
        <input
          type="number"
          min={0}
          value={settings.eventLeadMinutes ?? ""}
          placeholder="なし"
          disabled={!settings.enabled}
          onChange={(e) => update({ eventLeadMinutes: leadMinutes(e.target.value) })}
        />
      </label>
      <p className="settings-note">
        既定リマインドは、これから作成・期日変更するタスクと予定に適用されます。
      </p>
      <div className="settings-actions">
        <button type="button" disabled={testState === "sending"} onClick={() => void sendTest()}>
          {testState === "sending" ? "送信中…" : "テスト通知を送信"}
        </button>
        {testState === "ok" && <span className="settings-actions__ok">通知を送信しました</span>}
        {testState === "error" && (
          <span className="settings-actions__error" role="alert">
            {testError}
          </span>
        )}
      </div>
    </section>
  );
}

function GeneralSettingsSection() {
  const [autostart, setAutostart] = useState<boolean | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void isEnabled()
      .then(setAutostart)
      .catch((err) => setError(err instanceof Error ? err.message : String(err)));
  }, []);

  const toggle = async (next: boolean) => {
    try {
      if (next) {
        await enable();
      } else {
        await disable();
      }
      setAutostart(next);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  return (
    <section className="settings-section">
      <h2 className="settings-section__title">一般</h2>
      <label className="settings-field settings-field--toggle">
        <input
          type="checkbox"
          checked={autostart ?? false}
          disabled={autostart === null}
          onChange={(e) => void toggle(e.target.checked)}
        />
        Windows起動時にInquivoraを自動起動する
      </label>
      <p className="settings-note">
        閉じるボタンではタスクトレイへ格納され、常駐中のみ通知が届きます。
      </p>
      {error && (
        <p className="settings-actions__error" role="alert">
          {error}
        </p>
      )}
    </section>
  );
}

function AppearanceSettingsSection() {
  const preference = useThemeStore((s) => s.preference);
  const setPreference = useThemeStore((s) => s.setPreference);
  const navigationPosition = useSettingsStore((s) => s.navigationPosition);
  const setNavigationPosition = useSettingsStore((s) => s.setNavigationPosition);

  return (
    <section className="settings-section">
      <h2 className="settings-section__title">外観</h2>
      <div className="settings-field">
        テーマ
        <div className="theme-options">
          {THEME_OPTIONS.map((option) => (
            <button
              key={option.value}
              type="button"
              className={`theme-option${preference === option.value ? " theme-option--on" : ""}`}
              onClick={() => setPreference(option.value)}
            >
              {option.label}
            </button>
          ))}
        </div>
      </div>
      <p className="settings-note">
        「OSに合わせる」を選ぶと、Windowsのライト/ダーク設定に自動で追従します。
      </p>
      <div className="settings-field">
        ツール一覧の位置
        <div className="theme-options" role="group" aria-label="ツール一覧の位置">
          <button
            type="button"
            className={`theme-option${navigationPosition === "side" ? " theme-option--on" : ""}`}
            onClick={() => setNavigationPosition("side")}
          >
            左側
          </button>
          <button
            type="button"
            className={`theme-option${navigationPosition === "top" ? " theme-option--on" : ""}`}
            onClick={() => setNavigationPosition("top")}
          >
            上部
          </button>
        </div>
      </div>
    </section>
  );
}

export function SettingsPage() {
  return (
    <ThreePaneLayout left={<SettingsNav />}>
      <div className="settings-page">
        <AppearanceSettingsSection />
        <NotificationSettingsSection />
        <GeneralSettingsSection />
      </div>
    </ThreePaneLayout>
  );
}
