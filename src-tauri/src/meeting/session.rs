use std::collections::HashMap;
use std::sync::Mutex;

use serde_json::json;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;

use crate::database::jobs::{enqueue_job, JobInput};
use crate::error::AppError;
use crate::DbState;

#[derive(Default)]
pub struct AudioSessions(pub Mutex<HashMap<String, CommandChild>>);

pub struct StartOptions {
    pub mic: bool,
    pub loopback: bool,
    pub chunk_seconds: i64,
}

fn audio_error(message: String) -> AppError {
    AppError::new("AUDIO_CAPTURE_FAILED", message, false)
}

/// Sidecarのaudioモードを起動し、チャンクイベントを文字起こしジョブへ変換する（§9.7）。
pub async fn start_session(
    app: &AppHandle,
    meeting_id: &str,
    options: StartOptions,
) -> Result<(), AppError> {
    let output_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| audio_error(format!("データフォルダを解決できません: {e}")))?
        .join("audio")
        .join(meeting_id);
    std::fs::create_dir_all(&output_dir)
        .map_err(|e| audio_error(format!("録音フォルダを作成できません: {e}")))?;

    let sidecar = app
        .shell()
        .sidecar("inquivora-native")
        .map_err(|e| audio_error(format!("音声Sidecarを解決できません: {e}")))?
        .args(["audio", "--session", meeting_id]);
    let (mut rx, mut child) = sidecar
        .spawn()
        .map_err(|e| audio_error(format!("音声Sidecarを起動できません: {e}")))?;

    let start_command = json!({
        "command": "start",
        "micDeviceId": options.mic.then_some("default"),
        "loopbackDeviceId": options.loopback.then_some("default"),
        "chunkSeconds": options.chunk_seconds,
        "outputDir": output_dir.to_string_lossy(),
    });
    child
        .write(format!("{start_command}\n").as_bytes())
        .map_err(|e| audio_error(format!("音声Sidecarへ書き込めません: {e}")))?;

    let sessions = app.state::<AudioSessions>();
    sessions
        .0
        .lock()
        .map_err(|e| audio_error(format!("セッション状態のロックに失敗: {e}")))?
        .insert(meeting_id.to_string(), child);

    let app_handle = app.clone();
    let meeting = meeting_id.to_string();
    tauri::async_runtime::spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(line) => {
                    handle_sidecar_event(&app_handle, &meeting, &String::from_utf8_lossy(&line));
                }
                CommandEvent::Stderr(line) => {
                    eprintln!("audio sidecar: {}", String::from_utf8_lossy(&line).trim_end());
                }
                CommandEvent::Terminated(_) => {
                    if let Ok(mut sessions) = app_handle.state::<AudioSessions>().0.lock() {
                        sessions.remove(&meeting);
                    }
                    let _ = app_handle.emit("meeting:audio-stopped", json!({ "meetingId": meeting }));
                    break;
                }
                _ => {}
            }
        }
    });
    Ok(())
}

fn handle_sidecar_event(app: &AppHandle, meeting_id: &str, line: &str) {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(line.trim()) else {
        return;
    };
    match value.get("type").and_then(|v| v.as_str()) {
        Some("audio.chunk") => {
            let payload = json!({
                "path": value["path"],
                "source": value["source"],
                "startMs": value["startMs"],
                "endMs": value["endMs"],
            });
            if let Some(state) = app.try_state::<DbState>() {
                if let Ok(conn) = state.0.lock() {
                    let result = enqueue_job(
                        &conn,
                        JobInput {
                            job_type: "transcription".to_string(),
                            provider_profile_id: None,
                            model_id: None,
                            capability: Some("transcription.batch".to_string()),
                            entity_id: Some(meeting_id.to_string()),
                            request_path: Some(payload.to_string()),
                        },
                    );
                    if let Err(err) = result {
                        eprintln!("文字起こしジョブの登録に失敗: {}", err.message);
                    }
                }
            }
        }
        Some("audio.level") => {
            let _ = app.emit(
                "meeting:audio-level",
                json!({
                    "meetingId": meeting_id,
                    "source": value["source"],
                    "rms": value["rms"],
                }),
            );
        }
        Some("audio.started") => {
            let _ = app.emit("meeting:audio-started", json!({ "meetingId": meeting_id }));
        }
        Some("audio.error") => {
            let _ = app.emit(
                "meeting:audio-error",
                json!({
                    "meetingId": meeting_id,
                    "code": value["code"],
                    "message": value["message"],
                }),
            );
        }
        Some("audio.deviceLost") => {
            let _ = app.emit(
                "meeting:audio-error",
                json!({
                    "meetingId": meeting_id,
                    "code": "AUDIO_DEVICE_LOST",
                    "message": "録音デバイスが切断されました",
                }),
            );
        }
        _ => {}
    }
}

pub fn send_control(app: &AppHandle, meeting_id: &str, command: &str) -> Result<(), AppError> {
    let sessions = app.state::<AudioSessions>();
    let mut sessions = sessions
        .0
        .lock()
        .map_err(|e| audio_error(format!("セッション状態のロックに失敗: {e}")))?;
    let child = sessions.get_mut(meeting_id).ok_or_else(|| {
        AppError::new(
            "AUDIO_SESSION_NOT_FOUND",
            "録音セッションが見つかりません",
            false,
        )
    })?;
    child
        .write(format!("{}\n", json!({ "command": command })).as_bytes())
        .map_err(|e| audio_error(format!("音声Sidecarへ書き込めません: {e}")))?;
    Ok(())
}

pub fn has_session(app: &AppHandle, meeting_id: &str) -> bool {
    app.state::<AudioSessions>()
        .0
        .lock()
        .map(|sessions| sessions.contains_key(meeting_id))
        .unwrap_or(false)
}
