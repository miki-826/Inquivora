use std::path::Path;

use serde::Deserialize;
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_shell::process::CommandEvent;
use tauri_plugin_shell::ShellExt;

use crate::database::jobs;
use crate::database::meetings::{self, Meeting, MeetingStatus, TranscriptSegment};
use crate::error::AppError;
use crate::meeting::{files, session};
use crate::whisper;
use crate::DbState;

fn lock_error(e: impl std::fmt::Display) -> AppError {
    AppError::database(format!("DB接続ロックに失敗: {e}"))
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingStartInput {
    pub title: String,
    pub target_file_path: String,
    #[serde(default)]
    pub workspace_id: Option<String>,
    #[serde(default = "default_true")]
    pub mic: bool,
    #[serde(default = "default_true")]
    pub loopback: bool,
    #[serde(default)]
    pub mic_device_id: Option<String>,
    #[serde(default)]
    pub loopback_device_id: Option<String>,
    #[serde(default)]
    pub chunk_seconds: Option<i64>,
}

#[tauri::command]
pub async fn meeting_start(app: AppHandle, input: MeetingStartInput) -> Result<Meeting, AppError> {
    if !input.mic && !input.loopback {
        return Err(AppError::new(
            "VALIDATION_ERROR",
            "マイクかPC音声のいずれかを有効にしてください",
            false,
        ));
    }
    let meeting = {
        let state = app.state::<DbState>();
        let conn = state.0.lock().map_err(lock_error)?;
        let selected = whisper::models::selected_model(&conn)?;
        let models_dir = whisper::download::models_dir(&app)?;
        let local_available = whisper::models::model_path(&models_dir, &selected)
            .map(|p| p.is_file())
            .unwrap_or(false);
        whisper::route::resolve_transcription_route(&conn, local_available)?;
        meetings::create_meeting(
            &conn,
            meetings::MeetingInput {
                title: input.title.clone(),
                workspace_id: input.workspace_id.clone(),
                target_file_path: input.target_file_path.clone(),
                timezone: "Asia/Tokyo".to_string(),
            },
        )?
    };
    files::ensure_marker_block(
        Path::new(&meeting.target_file_path),
        &meeting.id,
        &meeting.title,
    )?;
    let _ = app.emit(
        "meeting:file-updated",
        json!({ "path": meeting.target_file_path }),
    );
    session::start_session(
        &app,
        &meeting.id,
        session::StartOptions {
            mic: input.mic,
            loopback: input.loopback,
            mic_device_id: input.mic_device_id.clone(),
            loopback_device_id: input.loopback_device_id.clone(),
            chunk_seconds: input.chunk_seconds.unwrap_or(20).clamp(5, 60),
        },
    )
    .await?;
    Ok(meeting)
}

#[tauri::command]
pub fn meeting_pause(app: AppHandle, meeting_id: String) -> Result<(), AppError> {
    session::send_control(&app, &meeting_id, "pause")?;
    let state = app.state::<DbState>();
    let conn = state.0.lock().map_err(lock_error)?;
    meetings::set_meeting_status(&conn, &meeting_id, MeetingStatus::Paused)
}

#[tauri::command]
pub fn meeting_resume(app: AppHandle, meeting_id: String) -> Result<(), AppError> {
    session::send_control(&app, &meeting_id, "resume")?;
    let state = app.state::<DbState>();
    let conn = state.0.lock().map_err(lock_error)?;
    meetings::set_meeting_status(&conn, &meeting_id, MeetingStatus::Recording)
}

#[tauri::command]
pub fn meeting_stop(app: AppHandle, meeting_id: String) -> Result<Meeting, AppError> {
    if session::has_session(&app, &meeting_id) {
        session::send_control(&app, &meeting_id, "stop")?;
    }
    let state = app.state::<DbState>();
    let conn = state.0.lock().map_err(lock_error)?;
    meetings::end_meeting(&conn, &meeting_id)?;
    meetings::get_meeting(&conn, &meeting_id)
}

#[tauri::command]
pub fn meeting_get(state: State<'_, DbState>, meeting_id: String) -> Result<Meeting, AppError> {
    let conn = state.0.lock().map_err(lock_error)?;
    meetings::get_meeting(&conn, &meeting_id)
}

#[tauri::command]
pub fn meeting_list(state: State<'_, DbState>, limit: Option<i64>) -> Result<Vec<Meeting>, AppError> {
    let conn = state.0.lock().map_err(lock_error)?;
    meetings::list_meetings(&conn, limit.unwrap_or(100))
}

#[tauri::command]
pub fn meeting_delete(app: AppHandle, meeting_id: String) -> Result<(), AppError> {
    if session::has_session(&app, &meeting_id) {
        let _ = session::send_control(&app, &meeting_id, "stop");
    }
    let state = app.state::<DbState>();
    let conn = state.0.lock().map_err(lock_error)?;
    jobs::cancel_jobs_for_entity(&conn, &meeting_id)?;
    meetings::delete_meeting(&conn, &meeting_id)
}

#[tauri::command]
pub fn meeting_list_segments(
    state: State<'_, DbState>,
    meeting_id: String,
) -> Result<Vec<TranscriptSegment>, AppError> {
    let conn = state.0.lock().map_err(lock_error)?;
    meetings::list_segments(&conn, &meeting_id)
}

/// §9.5 対象ファイルが閉じている場合のRust側追記。フロントの判断で呼び出す。
#[tauri::command]
pub fn meeting_append_segment(
    state: State<'_, DbState>,
    meeting_id: String,
    segment_markdown: String,
) -> Result<(), AppError> {
    let meeting = {
        let conn = state.0.lock().map_err(lock_error)?;
        meetings::get_meeting(&conn, &meeting_id)?
    };
    files::append_segment_to_file(
        Path::new(&meeting.target_file_path),
        &meeting_id,
        &segment_markdown,
    )
}

/// Sidecarのaudioモードで録音デバイス一覧を取得する。
#[tauri::command]
pub async fn meeting_list_devices(app: AppHandle) -> Result<serde_json::Value, AppError> {
    let failed = |message: String| AppError::new("AUDIO_DEVICE_NOT_FOUND", message, false);
    let sidecar = app
        .shell()
        .sidecar("inquivora-native")
        .map_err(|e| failed(format!("音声Sidecarを解決できません: {e}")))?
        .args(["audio", "--session", "device-scan"]);
    let (mut rx, mut child) = sidecar
        .spawn()
        .map_err(|e| failed(format!("音声Sidecarを起動できません: {e}")))?;
    child
        .write(b"{\"command\":\"listDevices\"}\n{\"command\":\"stop\"}\n")
        .map_err(|e| failed(format!("音声Sidecarへ書き込めません: {e}")))?;
    let wait = async {
        while let Some(event) = rx.recv().await {
            if let CommandEvent::Stdout(line) = event {
                let text = String::from_utf8_lossy(&line);
                let Ok(value) = serde_json::from_str::<serde_json::Value>(text.trim()) else {
                    continue;
                };
                if value["type"] == "audio.devices" {
                    return Ok(value);
                }
            }
        }
        Err(failed("デバイス一覧を取得できません".to_string()))
    };
    let result = match tokio::time::timeout(std::time::Duration::from_secs(15), wait).await {
        Ok(result) => result,
        Err(_) => Err(failed("デバイス一覧の取得がタイムアウトしました".to_string())),
    };
    drop(child);
    result
}
