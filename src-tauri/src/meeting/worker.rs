use std::time::Duration;

use serde_json::json;
use tauri::{AppHandle, Emitter, Manager};

use crate::api::credentials;
use crate::api::http::{self, TranscribeRequest};
use crate::database::providers::{self, ApiProviderProfile, UsageInput};
use crate::database::{jobs, meetings};
use crate::error::AppError;
use crate::meeting::{dedupe, markdown};
use crate::whisper;
use crate::DbState;

const POLL_INTERVAL: Duration = Duration::from_secs(2);
pub const TRANSCRIPTION_FEATURE: &str = "transcription.batch";

struct ChunkInfo {
    path: String,
    source: String,
    start_ms: i64,
    end_ms: i64,
}

enum TranscriptionBackend {
    Api { profile: ApiProviderProfile, model: String },
    Local { model_path: std::path::PathBuf, model_name: String },
}

struct JobContext {
    job_id: String,
    meeting: meetings::Meeting,
    backend: TranscriptionBackend,
    chunk: ChunkInfo,
}

/// §10.12 APIジョブワーカー。文字起こしジョブを順に処理する。
pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(POLL_INTERVAL).await;
            match process_next(&app).await {
                Ok(true) => continue,
                Ok(false) => {}
                Err(err) => eprintln!("APIジョブ処理に失敗: {}", err.message),
            }
        }
    });
}

async fn process_next(app: &AppHandle) -> Result<bool, AppError> {
    let Some(context) = prepare_job(app)? else {
        return Ok(false);
    };
    let result = match &context.backend {
        TranscriptionBackend::Api { profile, model } => {
            let secret = credentials::get_secret(app, &profile.id).await?;
            if secret.is_none() {
                return finish_with_error(
                    app,
                    &context,
                    AppError::new(
                        "API_AUTH_FAILED",
                        "APIキーが設定されていません。設定画面で登録してください",
                        false,
                    ),
                )
                .map(|_| true);
            }
            let runtime = http::runtime_from_profile(profile, secret);
            http::transcribe(
                &runtime,
                TranscribeRequest {
                    audio_path: context.chunk.path.clone(),
                    model: model.clone(),
                    language: "ja".to_string(),
                    prompt: None,
                },
            )
            .await
            .map(|transcription| transcription.text)
        }
        TranscriptionBackend::Local { model_path, .. } => {
            whisper::sidecar::transcribe(app, model_path, std::path::Path::new(&context.chunk.path))
                .await
        }
    };
    match result {
        Ok(text) => finish_with_success(app, &context, text)?,
        Err(err) => finish_with_error(app, &context, err)?,
    }
    Ok(true)
}

fn lock_error(e: impl std::fmt::Display) -> AppError {
    AppError::database(format!("DB接続ロックに失敗: {e}"))
}

fn prepare_job(app: &AppHandle) -> Result<Option<JobContext>, AppError> {
    let state = app.state::<DbState>();
    let conn = state.0.lock().map_err(lock_error)?;
    let Some(job) = jobs::next_pending_job(&conn)? else {
        return Ok(None);
    };
    if job.job_type != "transcription" {
        jobs::fail_job(&conn, &job.id, "UNSUPPORTED_JOB_TYPE", false)?;
        return Ok(None);
    }
    let chunk = parse_chunk(job.request_path.as_deref());
    let Some(chunk) = chunk else {
        jobs::fail_job(&conn, &job.id, "INVALID_JOB_PAYLOAD", false)?;
        return Ok(None);
    };
    let Some(meeting_id) = job.entity_id.clone() else {
        jobs::fail_job(&conn, &job.id, "INVALID_JOB_PAYLOAD", false)?;
        return Ok(None);
    };
    let Ok(meeting) = meetings::get_meeting(&conn, &meeting_id) else {
        jobs::cancel_jobs_for_entity(&conn, &meeting_id)?;
        return Ok(None);
    };
    let local_model = local_model_ready(app, &conn)?;
    let route = match whisper::route::resolve_transcription_route(&conn, local_model.is_some()) {
        Ok(route) => route,
        Err(err) => {
            jobs::fail_job(&conn, &job.id, &err.code, false)?;
            emit_transcription_error(app, &meeting_id, &err.code, &err.message);
            return Ok(None);
        }
    };
    let backend = match route {
        whisper::route::TranscriptionRoute::Api { provider_id, model } => {
            let profile = providers::get_provider(&conn, &provider_id)?;
            TranscriptionBackend::Api { profile, model }
        }
        whisper::route::TranscriptionRoute::Local => {
            let (model_path, model_name) = local_model.expect("ローカル経路はモデル確認済み");
            TranscriptionBackend::Local { model_path, model_name }
        }
    };
    jobs::mark_job_processing(&conn, &job.id)?;
    Ok(Some(JobContext {
        job_id: job.id,
        meeting,
        backend,
        chunk,
    }))
}

/// 選択中のWhisperモデルがダウンロード済みならそのパスを返す。
fn local_model_ready(
    app: &AppHandle,
    conn: &rusqlite::Connection,
) -> Result<Option<(std::path::PathBuf, String)>, AppError> {
    let name = whisper::models::selected_model(conn)?;
    let dir = whisper::download::models_dir(app)?;
    let path = whisper::models::model_path(&dir, &name).expect("選択モデルはカタログ検証済み");
    Ok(path.is_file().then_some((path, name)))
}

fn parse_chunk(request_path: Option<&str>) -> Option<ChunkInfo> {
    let value: serde_json::Value = serde_json::from_str(request_path?).ok()?;
    Some(ChunkInfo {
        path: value["path"].as_str()?.to_string(),
        source: value["source"].as_str()?.to_string(),
        start_ms: value["startMs"].as_i64()?,
        end_ms: value["endMs"].as_i64()?,
    })
}

fn record_usage(
    conn: &rusqlite::Connection,
    context: &JobContext,
    status: &str,
    error_code: Option<String>,
) -> Result<(), AppError> {
    let TranscriptionBackend::Api { profile, model } = &context.backend else {
        return Ok(());
    };
    providers::insert_usage(
        conn,
        UsageInput {
            provider_profile_id: profile.id.clone(),
            feature_key: TRANSCRIPTION_FEATURE.to_string(),
            model_id: model.clone(),
            entity_id: Some(context.meeting.id.clone()),
            input_units: None,
            output_units: None,
            audio_duration_ms: Some(context.chunk.end_ms - context.chunk.start_ms),
            latency_ms: None,
            status: status.to_string(),
            error_code,
        },
    )
}

fn finish_with_success(app: &AppHandle, context: &JobContext, text: String) -> Result<(), AppError> {
    let state = app.state::<DbState>();
    let conn = state.0.lock().map_err(lock_error)?;
    record_usage(&conn, context, "success", None)?;
    let trimmed = text.trim();
    if !trimmed.is_empty() {
        let previous = meetings::last_segment_text(&conn, &context.meeting.id, &context.chunk.source)?;
        let deduped = dedupe::dedupe_overlap(previous.as_deref().unwrap_or(""), trimmed);
        if !deduped.is_empty() {
            let speaker = markdown::speaker_label_for_source(&context.chunk.source);
            let segment = meetings::insert_segment(
                &conn,
                meetings::SegmentInput {
                    meeting_id: context.meeting.id.clone(),
                    source: context.chunk.source.clone(),
                    speaker_label: speaker.to_string(),
                    start_ms: context.chunk.start_ms,
                    end_ms: context.chunk.end_ms,
                    text: deduped.clone(),
                    audio_chunk_path: Some(context.chunk.path.clone()),
                },
            )?;
            let time_label =
                markdown::segment_time_label(&context.meeting.started_at, context.chunk.start_ms);
            let segment_markdown = markdown::format_segment(&time_label, speaker, &deduped);
            let _ = app.emit(
                "meeting:segment-committed",
                json!({
                    "meetingId": context.meeting.id,
                    "targetFilePath": context.meeting.target_file_path,
                    "segmentMarkdown": segment_markdown,
                    "segment": segment,
                }),
            );
        }
    }
    jobs::complete_job(&conn, &context.job_id)
}

fn finish_with_error(app: &AppHandle, context: &JobContext, err: AppError) -> Result<(), AppError> {
    let state = app.state::<DbState>();
    let conn = state.0.lock().map_err(lock_error)?;
    record_usage(&conn, context, "error", Some(err.code.clone()))?;
    jobs::fail_job(&conn, &context.job_id, &err.code, err.retryable)?;
    emit_transcription_error(app, &context.meeting.id, &err.code, &err.message);
    Ok(())
}

fn emit_transcription_error(app: &AppHandle, meeting_id: &str, code: &str, message: &str) {
    let _ = app.emit(
        "meeting:transcription-error",
        json!({ "meetingId": meeting_id, "code": code, "message": message }),
    );
}
