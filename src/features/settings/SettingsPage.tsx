import { isTauri } from "@tauri-apps/api/core";
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
import thirdPartyNotices from "../../../THIRD_PARTY_NOTICES.md?raw";

const NOTIFICATION_SETTING_KEY = "notifications";

function NotificationSettingsSection() {
  const [settings, setSettings] = useState<NotificationSettings | null>(() =>
    isTauri() ? null : parseNotificationSettings(null),
  );
  const [testState, setTestState] = useState<"idle" | "sending" | "ok" | "error">("idle");
  const [testError, setTestError] = useState<string | null>(null);

  useEffect(() => {
    if (!isTauri()) return;
    void loadSetting(NOTIFICATION_SETTING_KEY).then((stored) =>
      setSettings(parseNotificationSettings(stored)),
    );
  }, []);

  if (!settings) return null;

  const update = (partial: Partial<NotificationSettings>) => {
    const next = { ...settings, ...partial };
    setSettings(next);
    if (!isTauri()) return;
    void saveSetting(NOTIFICATION_SETTING_KEY, next).then(() => reconcileNotifications());
  };

  const sendTest = async () => {
    if (!isTauri()) return;
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
  const [autostart, setAutostart] = useState<boolean | null>(() => (isTauri() ? null : false));
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!isTauri()) return;
    void isEnabled()
      .then(setAutostart)
      .catch((err) => setError(err instanceof Error ? err.message : String(err)));
  }, []);

  const toggle = async (next: boolean) => {
    if (!isTauri()) {
      setAutostart(next);
      setError(null);
      return;
    }
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
  const taskListFontSize = useSettingsStore((s) => s.taskListFontSize);
  const setTaskListFontSize = useSettingsStore((s) => s.setTaskListFontSize);
  const uiDensity = useSettingsStore((s) => s.uiDensity);
  const setUiDensity = useSettingsStore((s) => s.setUiDensity);
  const showStatusBar = useSettingsStore((s) => s.showStatusBar);
  const setShowStatusBar = useSettingsStore((s) => s.setShowStatusBar);
  const reduceMotion = useSettingsStore((s) => s.reduceMotion);
  const setReduceMotion = useSettingsStore((s) => s.setReduceMotion);
  const editorFontSize = useSettingsStore((s) => s.editorFontSize);
  const setEditorFontSize = useSettingsStore((s) => s.setEditorFontSize);
  const editorWordWrap = useSettingsStore((s) => s.editorWordWrap);
  const setEditorWordWrap = useSettingsStore((s) => s.setEditorWordWrap);
  const editorSaveMode = useSettingsStore((s) => s.editorSaveMode);
  const setEditorSaveMode = useSettingsStore((s) => s.setEditorSaveMode);

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
          <button
            type="button"
            className={`theme-option${navigationPosition === "right" ? " theme-option--on" : ""}`}
            onClick={() => setNavigationPosition("right")}
          >
            右側
          </button>
          <button
            type="button"
            className={`theme-option${navigationPosition === "bottom" ? " theme-option--on" : ""}`}
            onClick={() => setNavigationPosition("bottom")}
          >
            下部
          </button>
        </div>
      </div>
      <div className="settings-field">
        画面の情報量
        <div className="theme-options" role="group" aria-label="画面の情報量">
          <button
            type="button"
            className={`theme-option${uiDensity === "comfortable" ? " theme-option--on" : ""}`}
            onClick={() => setUiDensity("comfortable")}
          >
            標準
          </button>
          <button
            type="button"
            className={`theme-option${uiDensity === "compact" ? " theme-option--on" : ""}`}
            onClick={() => setUiDensity("compact")}
          >
            コンパクト
          </button>
        </div>
      </div>
      <div className="settings-field">
        エディタの文字サイズ
        <div className="theme-options" role="group" aria-label="エディタの文字サイズ">
          {[
            ["small", "小（13px）"],
            ["medium", "中（14px）"],
            ["large", "大（16px）"],
          ].map(([value, label]) => (
            <button
              key={value}
              type="button"
              className={`theme-option${editorFontSize === value ? " theme-option--on" : ""}`}
              onClick={() => setEditorFontSize(value as "small" | "medium" | "large")}
            >
              {label}
            </button>
          ))}
        </div>
      </div>
      <label className="settings-field settings-field--toggle">
        <input
          type="checkbox"
          checked={editorWordWrap}
          onChange={(event) => setEditorWordWrap(event.target.checked)}
        />
        エディタで長い行を折り返す
      </label>
      <div className="settings-field">
        メモ帳の保存方法
        <div className="theme-options" role="group" aria-label="メモ帳の保存方法">
          <button
            type="button"
            className={`theme-option${editorSaveMode === "auto" ? " theme-option--on" : ""}`}
            aria-pressed={editorSaveMode === "auto"}
            onClick={() => {
              setEditorSaveMode("auto");
              void import("../../stores/useEditorStore").then(({ useEditorStore }) =>
                useEditorStore.getState().saveAllTabs(),
              );
            }}
          >
            自動保存（推奨）
          </button>
          <button
            type="button"
            className={`theme-option${editorSaveMode === "manual" ? " theme-option--on" : ""}`}
            aria-pressed={editorSaveMode === "manual"}
            onClick={() => {
              setEditorSaveMode("manual");
              void import("../../stores/useEditorStore").then(({ useEditorStore }) =>
                useEditorStore.getState().cancelPendingAutosaves(),
              );
            }}
          >
            手動保存
          </button>
        </div>
      </div>
      <p className="settings-note">
        自動保存は入力から約0.8秒後に保存します。手動保存では保存ボタンまたは Ctrl+S を使います。
      </p>
      <label className="settings-field settings-field--toggle">
        <input
          type="checkbox"
          checked={showStatusBar}
          onChange={(event) => setShowStatusBar(event.target.checked)}
        />
        下部のステータスバーを表示する
      </label>
      <label className="settings-field settings-field--toggle">
        <input
          type="checkbox"
          checked={reduceMotion}
          onChange={(event) => setReduceMotion(event.target.checked)}
        />
        画面のアニメーションを減らす
      </label>
      <div className="settings-field">
        カレンダーのタスク文字サイズ
        <div className="theme-options" role="group" aria-label="カレンダーのタスク文字サイズ">
          {[
            ["small", "小"],
            ["medium", "中"],
            ["large", "大"],
          ].map(([value, label]) => (
            <button
              key={value}
              type="button"
              className={`theme-option${taskListFontSize === value ? " theme-option--on" : ""}`}
              onClick={() => setTaskListFontSize(value as "small" | "medium" | "large")}
            >
              {label}
            </button>
          ))}
        </div>
      </div>
    </section>
  );
}

function LicenseSection() {
  return (
    <section className="settings-section">
      <h2 className="settings-section__title">ライセンス</h2>
      <p className="settings-note">
        Inquivora は Monaco Editor を使用しています。Monaco Editor は Microsoft Corporation
        により MIT License で提供されています。
      </p>
      <details className="settings-license">
        <summary>Monaco Editor の MIT ライセンス全文</summary>
        <pre>{thirdPartyNotices}</pre>
      </details>
      <p className="settings-note">
        同じ内容の THIRD_PARTY_NOTICES.md を配布パッケージにも同梱しています。
      </p>
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
        <LicenseSection />
      </div>
    </ThreePaneLayout>
  );
}
