import { useEffect } from "react";
import { useNavigate } from "react-router-dom";
import { useMeetingStore } from "../../stores/useMeetingStore";

function timeLabel(startMs: number): string {
  const total = Math.floor(startMs / 1000);
  return `${String(Math.floor(total / 60)).padStart(2, "0")}:${String(total % 60).padStart(2, "0")}`;
}

/// ワークスペース編集中に会議の進行を確認・操作できるサイドパネル（§5.1右ペイン）。
export function WorkspaceMeetingPanel() {
  const navigate = useNavigate();
  const activeMeeting = useMeetingStore((s) => s.activeMeeting);
  const segments = useMeetingStore((s) => s.segments);
  const status = useMeetingStore((s) => s.activeMeeting?.status);
  const busy = useMeetingStore((s) => s.busy);
  const pause = useMeetingStore((s) => s.pause);
  const resume = useMeetingStore((s) => s.resume);
  const stop = useMeetingStore((s) => s.stop);
  const loadMeetings = useMeetingStore((s) => s.loadMeetings);

  useEffect(() => {
    void loadMeetings();
  }, [loadMeetings]);

  if (!activeMeeting) {
    return (
      <div className="ws-meeting-panel">
        <h2 className="ws-meeting-panel__title">会議</h2>
        <p className="ws-meeting-panel__note">
          録音しながらここにメモを取れます。会議を開始すると、文字起こしがこのパネルに流れます。
        </p>
        <button
          type="button"
          className="ws-meeting-panel__start"
          onClick={() => navigate("/meetings")}
        >
          会議を開始
        </button>
      </div>
    );
  }

  const recent = segments.slice(-30);
  return (
    <div className="ws-meeting-panel">
      <div className="ws-meeting-panel__header">
        <h2 className="ws-meeting-panel__title">{activeMeeting.title}</h2>
        <span className={`meeting-status meeting-status--${status}`}>
          {status === "recording" ? "録音中" : "一時停止"}
        </span>
      </div>
      <div className="ws-meeting-panel__controls">
        {status === "recording" ? (
          <button type="button" disabled={busy} onClick={() => void pause()}>
            一時停止
          </button>
        ) : (
          <button type="button" disabled={busy} onClick={() => void resume()}>
            再開
          </button>
        )}
        <button type="button" disabled={busy} onClick={() => void stop()}>
          録音を終了
        </button>
        <button type="button" onClick={() => navigate("/meetings")}>
          会議画面へ
        </button>
      </div>
      <div className="ws-meeting-panel__segments">
        {recent.length === 0 ? (
          <p className="ws-meeting-panel__empty">文字起こしを待っています…</p>
        ) : (
          recent.map((segment) => (
            <div key={segment.id} className="ws-meeting-panel__segment">
              <span className="ws-meeting-panel__seg-time">
                {segment.source === "mic" ? "🎤" : "🔊"} {timeLabel(segment.startMs)}
              </span>
              <span className="ws-meeting-panel__seg-text">{segment.text}</span>
            </div>
          ))
        )}
      </div>
    </div>
  );
}
