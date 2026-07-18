import { useEffect, useState } from "react";
import { PanePlaceholder } from "../../components/common/PanePlaceholder";
import { ThreePaneLayout } from "../../components/layout/ThreePaneLayout";
import { useMeetingStore } from "../../stores/useMeetingStore";
import { useWorkspaceStore } from "../../stores/useWorkspaceStore";
import {
  defaultMeetingFileName,
  meetingStatusLabel,
  type Meeting,
  type TranscriptSegment,
} from "./meetingModel";

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

  const [title, setTitle] = useState("");
  const [customPath, setCustomPath] = useState<string | null>(null);
  const [mic, setMic] = useState(true);
  const [loopback, setLoopback] = useState(true);
  const [localError, setLocalError] = useState<string | null>(null);

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
    const ok = await start({
      title: title.trim() || "会議",
      targetFilePath: targetFilePath.trim(),
      workspaceId: workspace?.id ?? null,
      mic,
      loopback,
    });
    if (ok) {
      onClose();
    }
  };

  return (
    <div className="meeting-dialog__backdrop">
      <div className="meeting-dialog" role="dialog" aria-label="会議を開始">
        <h2 className="meeting-dialog__title">会議を開始</h2>
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
        <label className="settings-field settings-field--toggle">
          <input
            type="checkbox"
            checked={loopback}
            onChange={(e) => setLoopback(e.target.checked)}
          />
          PC音声（スピーカー出力）を録音する
        </label>
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
        <li key={segment.id} className="meeting-segment">
          <span className="meeting-segment__meta">
            {segmentTimeLabel(segment)} {segment.speakerLabel}
          </span>
          <span className="meeting-segment__text">{segment.text}</span>
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
      <SegmentList segments={segments} />
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
      right={<PanePlaceholder title="要約・決定事項・タスク候補" description="Phase 6で実装予定" />}
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
