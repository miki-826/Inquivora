use std::path::Path;

use serde::Deserialize;
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_shell::process::CommandEvent;
use tauri_plugin_shell::ShellExt;

use crate::api::{credentials, http};
use crate::database::meeting_ai::{self, MeetingDecision, TaskCandidate};
use crate::database::providers::{self, UsageInput};
use crate::database::jobs;
use crate::database::meetings::{self, Meeting, MeetingStatus, TranscriptSegment};
use crate::error::AppError;
use crate::meeting::summary::{self, MEETING_SUMMARY_FEATURE};
use crate::meeting::{files, markdown, session};
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

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingSummaryResult {
    pub summary: String,
    pub decisions: Vec<MeetingDecision>,
    pub task_candidates: Vec<TaskCandidate>,
    pub open_questions: Vec<crate::api::client::AiOpenQuestion>,
}

/// 議事録生成が可能か（meeting.summaryのAPI設定が有効か）を返す。
#[tauri::command]
pub fn meeting_summary_available(state: State<'_, DbState>) -> Result<bool, AppError> {
    let conn = state.0.lock().map_err(lock_error)?;
    summary::summary_available(&conn)
}

#[tauri::command]
pub fn meeting_list_decisions(
    state: State<'_, DbState>,
    meeting_id: String,
) -> Result<Vec<MeetingDecision>, AppError> {
    let conn = state.0.lock().map_err(lock_error)?;
    meeting_ai::list_decisions(&conn, &meeting_id)
}

#[tauri::command]
pub fn meeting_list_candidates(
    state: State<'_, DbState>,
    meeting_id: String,
) -> Result<Vec<TaskCandidate>, AppError> {
    let conn = state.0.lock().map_err(lock_error)?;
    meeting_ai::list_candidates(&conn, &meeting_id)
}

fn record_summary_usage(
    conn: &rusqlite::Connection,
    provider_id: &str,
    model: &str,
    meeting_id: &str,
    status: &str,
    error_code: Option<String>,
) {
    let _ = providers::insert_usage(
        conn,
        UsageInput {
            provider_profile_id: provider_id.to_string(),
            feature_key: MEETING_SUMMARY_FEATURE.to_string(),
            model_id: model.to_string(),
            entity_id: Some(meeting_id.to_string()),
            input_units: None,
            output_units: None,
            audio_duration_ms: None,
            latency_ms: None,
            status: status.to_string(),
            error_code,
        },
    );
}

/// §11.3 議事録生成。API設定時のみ有効（ローカルWhisperでは生成しない）。
#[tauri::command]
pub async fn meeting_generate_summary(
    app: AppHandle,
    meeting_id: String,
) -> Result<MeetingSummaryResult, AppError> {
    let (meeting, segments, profile, model) = {
        let state = app.state::<DbState>();
        let conn = state.0.lock().map_err(lock_error)?;
        let meeting = meetings::get_meeting(&conn, &meeting_id)?;
        let segments = meetings::list_segments(&conn, &meeting_id)?;
        if segments.is_empty() {
            return Err(AppError::new(
                "MEETING_NO_TRANSCRIPT",
                "文字起こしがまだありません。録音・文字起こしの完了後に生成してください",
                false,
            ));
        }
        let (profile, model) = summary::resolve_summary_provider(&conn)?;
        (meeting, segments, profile, model)
    };

    let secret = credentials::get_secret(&app, &profile.id).await?;
    if secret.is_none() {
        return Err(AppError::new(
            "API_AUTH_FAILED",
            "APIキーが設定されていません。設定画面で登録してください",
            false,
        ));
    }
    let transcript_text = summary::build_transcript_text(&meeting, &segments);
    let request = http::SummaryRequest {
        model: model.clone(),
        system_prompt: summary::system_prompt(),
        user_content: summary::build_user_content(&meeting, &transcript_text, ""),
    };
    let runtime = http::runtime_from_profile(&profile, secret);
    let output = match http::generate_summary(&runtime, request).await {
        Ok(output) => output,
        Err(err) => {
            let state = app.state::<DbState>();
            if let Ok(conn) = state.0.lock() {
                record_summary_usage(&conn, &profile.id, &model, &meeting_id, "error", Some(err.code.clone()));
            }
            return Err(err);
        }
    };

    let decision_inputs: Vec<meeting_ai::DecisionInput> = output
        .decisions
        .iter()
        .map(|d| meeting_ai::DecisionInput {
            text: d.text.clone(),
            source_start_ms: d.source_start_ms,
        })
        .collect();
    let candidate_inputs: Vec<meeting_ai::CandidateInput> = output
        .task_candidates
        .iter()
        .map(|c| meeting_ai::CandidateInput {
            title: c.title.clone(),
            description: c.description.clone(),
            due_at: c.due_at.clone(),
            priority: c.priority.clone(),
            assignee: c.assignee.clone(),
            source_start_ms: c.source_start_ms,
        })
        .collect();

    let (decisions, task_candidates) = {
        let state = app.state::<DbState>();
        let conn = state.0.lock().map_err(lock_error)?;
        meeting_ai::set_summary(&conn, &meeting_id, &output.summary)?;
        meeting_ai::replace_decisions(&conn, &meeting_id, &decision_inputs)?;
        meeting_ai::replace_candidates(&conn, &meeting_id, &candidate_inputs)?;
        record_summary_usage(&conn, &profile.id, &model, &meeting_id, "success", None);
        let decisions = meeting_ai::list_decisions(&conn, &meeting_id)?;
        let task_candidates = meeting_ai::list_candidates(&conn, &meeting_id)?;
        (decisions, task_candidates)
    };

    let decision_lines: Vec<String> = output.decisions.iter().map(|d| d.text.clone()).collect();
    let task_lines: Vec<String> = output.task_candidates.iter().map(|c| c.title.clone()).collect();
    let question_lines: Vec<String> = output.open_questions.iter().map(|q| q.text.clone()).collect();
    let ai_block = markdown::format_ai_block(
        &meeting_id,
        &output.summary,
        &decision_lines,
        &task_lines,
        &question_lines,
    );
    match files::write_ai_block_to_file(Path::new(&meeting.target_file_path), &meeting_id, &ai_block) {
        Ok(()) => {
            let _ = app.emit("meeting:file-updated", json!({ "path": meeting.target_file_path }));
        }
        Err(err) => eprintln!("議事録ブロックの追記に失敗: {}", err.message),
    }
    let _ = app.emit("meeting:summary-updated", json!({ "meetingId": meeting_id }));

    Ok(MeetingSummaryResult {
        summary: output.summary,
        decisions,
        task_candidates,
        open_questions: output.open_questions,
    })
}
