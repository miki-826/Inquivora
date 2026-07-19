import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { PanePlaceholder } from "../../components/common/PanePlaceholder";
import { ThreePaneLayout } from "../../components/layout/ThreePaneLayout";
import {
  exportMeetingRecording,
  isTranscriptionReady,
  listAudioDevices,
  meetingHasAudio,
  revealMeetingAudio,
  type AudioDevice,
} from "../../services/meetings";
import { loadSetting, saveSetting } from "../../services/settings";
import { useMeetingStore } from "../../stores/useMeetingStore";
import { useWorkspaceStore } from "../../stores/useWorkspaceStore";
import {
  defaultMeetingFileName,
  meetingStatusLabel,
  pickDeviceId,
  type Meeting,
  type TranscriptSegment,
} from "./meetingModel";

const AUDIO_DEVICES_KEY = "audio.devices";

type SavedDevices = {
  micDeviceId?: string;
  loopbackDeviceId?: string;
};

function parseSavedDevices(value: unknown): SavedDevices {
  if (typeof value !== "object" || value === null) {
    return {};
  }
  const record = value as Record<string, unknown>;
  return {
    micDeviceId: typeof record.micDeviceId === "string" ? record.micDeviceId : undefined,
    loopbackDeviceId:
      typeof record.loopbackDeviceId === "string" ? record.loopbackDeviceId : undefined,
  };
}

function DeviceSelect({
  label,
  devices,
  value,
  onChange,
}: {
  label: string;
  devices: AudioDevice[];
  value: string;
  onChange: (id: string) => void;
}) {
  return (
    <label className="settings-field">
      {label}
      <select value={value} onChange={(e) => onChange(e.target.value)}>
        <option value="default">システム既定</option>
        {devices.map((device) => (
          <option key={device.id} value={device.id}>
            {device.name}
            {device.isDefault ? "（既定）" : ""}
          </option>
        ))}
      </select>
    </label>
  );
}

function formatTokyoTime(utc: string): string {
  const date = new Date(utc.endsWith("Z") ? utc : `${utc}Z`);
  if (Number.isNaN(date.getTime())) {
    return utc;
  }
  return date.toLocaleString("ja-JP", { timeZone: "Asia/Tokyo" });
}

function segmentTimeLabel(segment: TranscriptSegment): string {
  const totalSeconds = Math.floor(segment.startMs / 1000);
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return `${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`;
}

type StartDialogProps = {
  onClose: () => void;
};

function MeetingStartDialog({ onClose }: StartDialogProps) {
  const workspace = useWorkspaceStore((s) => s.workspace);
  const start = useMeetingStore((s) => s.start);
  const busy = useMeetingStore((s) => s.busy);
  const navigate = useNavigate();

  const [title, setTitle] = useState("");
  const [customPath, setCustomPath] = useState<string | null>(null);
  const [mic, setMic] = useState(true);
  const [loopback, setLoopback] = useState(true);
  const [micDevices, setMicDevices] = useState<AudioDevice[]>([]);
  const [loopbackDevices, setLoopbackDevices] = useState<AudioDevice[]>([]);
  const [micDeviceId, setMicDeviceId] = useState("default");
  const [loopbackDeviceId, setLoopbackDeviceId] = useState("default");
  const [transcriptionReady, setTranscriptionReady] = useState(true);
  const [localError, setLocalError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    void isTranscriptionReady()
      .then((ready) => {
        if (!cancelled) setTranscriptionReady(ready);
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const [devices, stored] = await Promise.all([
          listAudioDevices(),
          loadSetting(AUDIO_DEVICES_KEY).catch(() => null),
        ]);
        if (cancelled) {
          return;
        }
        const saved = parseSavedDevices(stored);
        setMicDevices(devices.mic);
        setLoopbackDevices(devices.loopback);
        setMicDeviceId(pickDeviceId(saved.micDeviceId, devices.mic));
        setLoopbackDeviceId(pickDeviceId(saved.loopbackDeviceId, devices.loopback));
      } catch (err) {
        console.error("録音デバイス一覧の取得に失敗", err);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  const fileName = defaultMeetingFileName(title, new Date());
  const targetFilePath =
    customPath ?? (workspace ? `${workspace.rootPath}\\meetings\\${fileName}` : fileName);

  const submit = async () => {
    if (!targetFilePath.trim()) {
      setLocalError("文字起こし先ファイルを指定してください");
      return;
    }
    if (!mic && !loopback) {
      setLocalError("マイクとPC音声の少なくとも一方を有効にしてください");
      return;
    }
    setLocalError(null);
    void saveSetting(AUDIO_DEVICES_KEY, { micDeviceId, loopbackDeviceId }).catch((err) =>
      console.error("録音デバイス設定の保存に失敗", err),
    );
    const ok = await start({
      title: title.trim() || "会議",
      targetFilePath: targetFilePath.trim(),
      workspaceId: workspace?.id ?? null,
      mic,
      loopback,
      micDeviceId: micDeviceId === "default" ? undefined : micDeviceId,
      loopbackDeviceId: loopbackDeviceId === "default" ? undefined : loopbackDeviceId,
    });
    if (ok) {
      onClose();
    }
  };

  return (
    <div className="meeting-dialog__backdrop">
      <div className="meeting-dialog" role="dialog" aria-label="会議を開始">
        <h2 className="meeting-dialog__title">会議を開始</h2>
        {!transcriptionReady && (
          <div className="meeting-setup-hint" role="alert">
            <p>
              文字起こしの準備ができていません。ローカルのWhisperモデルをダウンロードするか、
              AI（接続先）を設定すると文字起こしできます。
            </p>
            <button
              type="button"
              className="meeting-setup-hint__button"
              onClick={() => {
                onClose();
                navigate("/settings/ai");
              }}
            >
              文字起こしの設定を開く
            </button>
          </div>
        )}
        <label className="settings-field">
          タイトル
          <input
            type="text"
            value={title}
            placeholder="会議"
            onChange={(e) => setTitle(e.target.value)}
          />
        </label>
        <label className="settings-field">
          文字起こし先ファイル
          <input
            type="text"
            value={targetFilePath}
            onChange={(e) => setCustomPath(e.target.value)}
          />
        </label>
        <label className="settings-field settings-field--toggle">
          <input type="checkbox" checked={mic} onChange={(e) => setMic(e.target.checked)} />
          マイクを録音する
        </label>
        {mic && micDevices.length > 0 && (
          <DeviceSelect
            label="使用するマイク"
            devices={micDevices}
            value={micDeviceId}
            onChange={setMicDeviceId}
          />
        )}
        <label className="settings-field settings-field--toggle">
          <input
            type="checkbox"
            checked={loopback}
            onChange={(e) => setLoopback(e.target.checked)}
          />
          PC音声（スピーカー出力）を録音する
        </label>
        {loopback && loopbackDevices.length > 0 && (
          <DeviceSelect
            label="使用する出力デバイス"
            devices={loopbackDevices}
            value={loopbackDeviceId}
            onChange={setLoopbackDeviceId}
          />
        )}
        <div className="meeting-dialog__privacy">
          <p>開始すると以下が実行されます:</p>
          <ul>
            {mic && <li>マイクの録音</li>}
            {loopback && <li>PC音声（ループバック）の録音</li>}
            <li>
              録音チャンクの文字起こし（API Provider設定時は外部送信・未設定時は内蔵Whisperでローカル処理）
            </li>
          </ul>
        </div>
        {localError && (
          <p className="settings-actions__error" role="alert">
            {localError}
          </p>
        )}
        <div className="meeting-dialog__actions">
          <button type="button" onClick={onClose} disabled={busy}>
            キャンセル
          </button>
          <button type="button" className="meeting-dialog__start" onClick={() => void submit()} disabled={busy}>
            {busy ? "開始中…" : "録音を開始"}
          </button>
        </div>
      </div>
    </div>
  );
}

function LevelMeter({ label, value }: { label: string; value: number }) {
  const percent = Math.min(100, Math.round(value * 300));
  return (
    <div className="level-meter">
      <span className="level-meter__label">{label}</span>
      <div className="level-meter__bar">
        <div className="level-meter__fill" style={{ width: `${percent}%` }} />
      </div>
    </div>
  );
}

function ActiveMeetingView({ meeting }: { meeting: Meeting }) {
  const levels = useMeetingStore((s) => s.levels);
  const segments = useMeetingStore((s) => s.segments);
  const pause = useMeetingStore((s) => s.pause);
  const resume = useMeetingStore((s) => s.resume);
  const stop = useMeetingStore((s) => s.stop);
  const busy = useMeetingStore((s) => s.busy);

  return (
    <div className="meeting-active">
      <div className="meeting-active__header">
        <h2 className="meeting-active__title">{meeting.title}</h2>
        <span className={`meeting-status meeting-status--${meeting.status}`}>
          {meetingStatusLabel(meeting.status)}
        </span>
      </div>
      <p className="meeting-active__file">{meeting.targetFilePath}</p>
      <div className="meeting-active__levels">
        <LevelMeter label="マイク" value={levels.mic} />
        <LevelMeter label="PC音声" value={levels.loopback} />
      </div>
      <div className="meeting-active__controls">
        {meeting.status === "recording" ? (
          <button type="button" onClick={() => void pause()} disabled={busy}>
            一時停止
          </button>
        ) : (
          <button type="button" onClick={() => void resume()} disabled={busy}>
            再開
          </button>
        )}
        <button
          type="button"
          className="meeting-active__stop"
          onClick={() => void stop()}
          disabled={busy}
        >
          録音を終了
        </button>
      </div>
      <SegmentList segments={segments} />
    </div>
  );
}

function SegmentList({ segments }: { segments: TranscriptSegment[] }) {
  if (segments.length === 0) {
    return <p className="meeting-segments__empty">確定したセグメントはまだありません</p>;
  }
  return (
    <ul className="meeting-segments">
      {segments.map((segment) => (
        <li
          key={segment.id}
          className={`meeting-segment meeting-segment--${segment.source === "mic" ? "mic" : "loopback"}`}
        >
          <div className="meeting-segment__header">
            <span
              className={`meeting-segment__avatar meeting-segment__avatar--${
                segment.source === "mic" ? "mic" : "loopback"
              }`}
              aria-hidden="true"
            >
              {segment.source === "mic" ? "🎤" : "🔊"}
            </span>
            <span className="meeting-segment__speaker">{segment.speakerLabel}</span>
            <span className="meeting-segment__time">{segmentTimeLabel(segment)}</span>
          </div>
          <p className="meeting-segment__text">{segment.text}</p>
        </li>
      ))}
    </ul>
  );
}

function MeetingList() {
  const meetings = useMeetingStore((s) => s.meetings);
  const selectedMeetingId = useMeetingStore((s) => s.selectedMeetingId);
  const activeMeeting = useMeetingStore((s) => s.activeMeeting);
  const selectMeeting = useMeetingStore((s) => s.selectMeeting);
  const remove = useMeetingStore((s) => s.remove);

  if (meetings.length === 0) {
    return (
      <div className="pane-section">
        <div className="pane-section__title">会議一覧</div>
        <p className="meeting-list__empty">まだ会議がありません</p>
      </div>
    );
  }

  return (
    <div className="pane-section">
      <div className="pane-section__title">会議一覧</div>
      <ul className="meeting-list">
        {meetings.map((meeting) => (
          <li
            key={meeting.id}
            className={`meeting-list__item${
              meeting.id === selectedMeetingId ? " meeting-list__item--selected" : ""
            }`}
          >
            <button
              type="button"
              className="meeting-list__select"
              onClick={() => void selectMeeting(meeting.id)}
            >
              <span className="meeting-list__title">{meeting.title}</span>
              <span className="meeting-list__meta">
                {formatTokyoTime(meeting.startedAtUtc)}・{meetingStatusLabel(meeting.status)}
              </span>
            </button>
            {meeting.id !== activeMeeting?.id && (
              <button
                type="button"
                className="meeting-list__delete"
                aria-label={`${meeting.title}を削除`}
                onClick={() => {
                  if (window.confirm(`「${meeting.title}」を削除しますか？（ファイルは残ります）`)) {
                    void remove(meeting.id);
                  }
                }}
              >
                削除
              </button>
            )}
          </li>
        ))}
      </ul>
    </div>
  );
}

function RecordingActions({ meetingId }: { meetingId: string }) {
  const [hasAudio, setHasAudio] = useState(false);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    void meetingHasAudio(meetingId)
      .then((has) => {
        if (!cancelled) setHasAudio(has);
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, [meetingId]);

  if (!hasAudio) {
    return null;
  }

  const exportAndReveal = async () => {
    setBusy(true);
    setMessage(null);
    try {
      const files = await exportMeetingRecording(meetingId);
      setMessage(`録音を書き出しました（${files.length}ファイル）`);
      await revealMeetingAudio(meetingId);
    } catch (err) {
      setMessage(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="meeting-recording">
      <h3 className="meeting-recording__title">録音</h3>
      <p className="meeting-recording__note">
        この会議の録音（マイク・PC音声）が保存されています。書き出すと音源ごとに1つのWAVへまとめ、フォルダを開きます。
      </p>
      <div className="meeting-recording__actions">
        <button type="button" disabled={busy} onClick={() => void exportAndReveal()}>
          {busy ? "書き出し中…" : "録音を書き出して開く"}
        </button>
        <button type="button" onClick={() => void revealMeetingAudio(meetingId)}>
          録音フォルダを開く
        </button>
      </div>
      {message && <p className="meeting-recording__message">{message}</p>}
    </div>
  );
}

function SelectedMeetingView() {
  const meetings = useMeetingStore((s) => s.meetings);
  const selectedMeetingId = useMeetingStore((s) => s.selectedMeetingId);
  const segments = useMeetingStore((s) => s.segments);
  const meeting = meetings.find((m) => m.id === selectedMeetingId);

  if (!meeting) {
    return (
      <PanePlaceholder
        title="会議"
        description="左の一覧から会議を選択するか、新しい会議を開始してください"
      />
    );
  }
  return (
    <div className="meeting-active">
      <div className="meeting-active__header">
        <h2 className="meeting-active__title">{meeting.title}</h2>
        <span className={`meeting-status meeting-status--${meeting.status}`}>
          {meetingStatusLabel(meeting.status)}
        </span>
      </div>
      <p className="meeting-active__file">{meeting.targetFilePath}</p>
      <RecordingActions meetingId={meeting.id} />
      <SegmentList segments={segments} />
    </div>
  );
}

function AiMeetingPanel() {
  const selectedMeetingId = useMeetingStore((s) => s.selectedMeetingId);
  const ai = useMeetingStore((s) => s.ai);
  const summaryAvailable = useMeetingStore((s) => s.summaryAvailable);
  const generating = useMeetingStore((s) => s.generatingSummary);
  const generateSummary = useMeetingStore((s) => s.generateSummary);
  const acceptCandidate = useMeetingStore((s) => s.acceptCandidate);

  if (!selectedMeetingId) {
    return (
      <PanePlaceholder
        title="要約・決定事項・タスク候補"
        description="会議を選択すると議事録を表示します"
      />
    );
  }

  return (
    <div className="ai-panel">
      <div className="ai-panel__header">
        <h2 className="ai-panel__title">AI議事録</h2>
        <button
          type="button"
          className="ai-panel__generate"
          disabled={!summaryAvailable || generating}
          onClick={() => void generateSummary(selectedMeetingId)}
        >
          {generating ? "生成中…" : ai.summary ? "再生成" : "議事録を生成"}
        </button>
      </div>
      {!summaryAvailable && (
        <p className="ai-panel__note">
          議事録の生成にはAPIプロバイダーの設定が必要です。設定画面のAI設定で「議事録・タスク抽出」にProviderとモデルを割り当ててください（内蔵Whisperのみでは生成できません）。
        </p>
      )}

      <section className="ai-panel__section">
        <h3>要約</h3>
        {ai.summary ? (
          <p className="ai-panel__summary">{ai.summary}</p>
        ) : (
          <p className="ai-panel__empty">まだ生成されていません</p>
        )}
      </section>

      <section className="ai-panel__section">
        <h3>決定事項</h3>
        {ai.decisions.length === 0 ? (
          <p className="ai-panel__empty">なし</p>
        ) : (
          <ul className="ai-panel__list">
            {ai.decisions.map((d) => (
              <li key={d.id}>{d.text}</li>
            ))}
          </ul>
        )}
      </section>

      <section className="ai-panel__section">
        <h3>タスク候補</h3>
        {ai.candidates.length === 0 ? (
          <p className="ai-panel__empty">なし</p>
        ) : (
          <ul className="ai-panel__candidates">
            {ai.candidates.map((c) => (
              <li key={c.id} className="ai-panel__candidate">
                <div className="ai-panel__candidate-main">
                  <span className={`ai-panel__priority ai-panel__priority--${c.priority}`}>
                    {c.priority === "high" ? "高" : c.priority === "low" ? "低" : "中"}
                  </span>
                  <span className="ai-panel__candidate-title">{c.title}</span>
                </div>
                {c.assignee && <span className="ai-panel__assignee">担当: {c.assignee}</span>}
                {c.status === "accepted" ? (
                  <span className="ai-panel__accepted">登録済み</span>
                ) : (
                  <button
                    type="button"
                    className="ai-panel__accept"
                    onClick={() => void acceptCandidate(c.id)}
                  >
                    タスク登録
                  </button>
                )}
              </li>
            ))}
          </ul>
        )}
      </section>

      {ai.openQuestions.length > 0 && (
        <section className="ai-panel__section">
          <h3>未確認事項</h3>
          <ul className="ai-panel__list">
            {ai.openQuestions.map((q, i) => (
              <li key={i}>{q.text}</li>
            ))}
          </ul>
        </section>
      )}
    </div>
  );
}

export function MeetingsPage() {
  const activeMeeting = useMeetingStore((s) => s.activeMeeting);
  const error = useMeetingStore((s) => s.error);
  const clearError = useMeetingStore((s) => s.clearError);
  const loadMeetings = useMeetingStore((s) => s.loadMeetings);
  const [dialogOpen, setDialogOpen] = useState(false);

  useEffect(() => {
    void loadMeetings();
  }, [loadMeetings]);

  return (
    <ThreePaneLayout
      left={<MeetingList />}
      right={<AiMeetingPanel />}
    >
      <div className="meeting-page">
        {error && (
          <div className="meeting-error" role="alert">
            <span>{error}</span>
            <button type="button" onClick={clearError}>
              閉じる
            </button>
          </div>
        )}
        {!activeMeeting && (
          <div className="meeting-page__toolbar">
            <button type="button" className="meeting-page__start" onClick={() => setDialogOpen(true)}>
              新しい会議を開始
            </button>
          </div>
        )}
        {activeMeeting ? <ActiveMeetingView meeting={activeMeeting} /> : <SelectedMeetingView />}
        {dialogOpen && <MeetingStartDialog onClose={() => setDialogOpen(false)} />}
      </div>
    </ThreePaneLayout>
  );
}
