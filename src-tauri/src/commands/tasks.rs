use serde::Deserialize;
use tauri::State;

use crate::database::tasks::{self, Task, TaskFilter, TaskInput, TaskPatch, TaskPriority};
use crate::database::{meeting_ai, meetings, settings};
use crate::error::AppError;
use crate::notifications::schedule::parse_settings;
use crate::notifications::sync::sync_after_task_saved;
use crate::DbState;

fn priority_from_str(value: &str) -> TaskPriority {
    match value {
        "high" => TaskPriority::High,
        "low" => TaskPriority::Low,
        _ => TaskPriority::Medium,
    }
}

fn with_conn<T>(
    state: &State<'_, DbState>,
    f: impl FnOnce(&rusqlite::Connection) -> Result<T, AppError>,
) -> Result<T, AppError> {
    let conn = state
        .0
        .lock()
        .map_err(|e| AppError::database(format!("DB接続ロックに失敗: {e}")))?;
    f(&conn)
}

fn sync_reminders(
    conn: &rusqlite::Connection,
    old: Option<&Task>,
    task: &Task,
) -> Result<(), AppError> {
    let config = parse_settings(settings::get_setting(conn, "notifications")?);
    sync_after_task_saved(conn, old, task, &config, chrono::Utc::now())
}

#[tauri::command]
pub fn task_create(state: State<'_, DbState>, input: TaskInput) -> Result<Task, AppError> {
    with_conn(&state, |conn| {
        let task = tasks::create_task(conn, &input)?;
        sync_reminders(conn, None, &task)?;
        Ok(task)
    })
}

#[tauri::command]
pub fn task_update(state: State<'_, DbState>, id: String, patch: TaskPatch) -> Result<Task, AppError> {
    with_conn(&state, |conn| {
        let old = tasks::get_task(conn, &id)?;
        let task = tasks::update_task(conn, &id, &patch)?;
        sync_reminders(conn, Some(&old), &task)?;
        Ok(task)
    })
}

#[tauri::command]
pub fn task_delete(state: State<'_, DbState>, id: String) -> Result<(), AppError> {
    with_conn(&state, |conn| tasks::delete_task(conn, &id))
}

#[tauri::command]
pub fn task_get(state: State<'_, DbState>, id: String) -> Result<Task, AppError> {
    with_conn(&state, |conn| tasks::get_task(conn, &id))
}

#[tauri::command]
pub fn task_list(
    state: State<'_, DbState>,
    filter: Option<TaskFilter>,
) -> Result<Vec<Task>, AppError> {
    with_conn(&state, |conn| {
        tasks::list_tasks(conn, &filter.unwrap_or_default(), chrono::Utc::now())
    })
}

#[tauri::command]
pub fn task_complete(state: State<'_, DbState>, id: String) -> Result<Task, AppError> {
    with_conn(&state, |conn| {
        let old = tasks::get_task(conn, &id)?;
        let task = tasks::complete_task(conn, &id)?;
        sync_reminders(conn, Some(&old), &task)?;
        Ok(task)
    })
}

#[tauri::command]
pub fn task_reopen(state: State<'_, DbState>, id: String) -> Result<Task, AppError> {
    with_conn(&state, |conn| {
        let old = tasks::get_task(conn, &id)?;
        let task = tasks::reopen_task(conn, &id)?;
        sync_reminders(conn, Some(&old), &task)?;
        Ok(task)
    })
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CandidateAcceptPatch {
    pub title: Option<String>,
    pub description: Option<String>,
    pub due_at_utc: Option<String>,
    pub priority: Option<TaskPriority>,
    pub assignee: Option<String>,
}

/// §10.11 タスク候補を正式タスクへ登録し、候補を承認済みにする。
#[tauri::command]
pub fn task_accept_candidate(
    state: State<'_, DbState>,
    candidate_id: String,
    patch: Option<CandidateAcceptPatch>,
) -> Result<Task, AppError> {
    let patch = patch.unwrap_or_default();
    with_conn(&state, |conn| {
        let candidate = meeting_ai::get_candidate(conn, &candidate_id)?;
        if candidate.status == "accepted" {
            return Err(AppError::new(
                "CANDIDATE_ALREADY_ACCEPTED",
                "このタスク候補はすでに登録済みです",
                false,
            ));
        }
        let meeting = meetings::get_meeting(conn, &candidate.meeting_id).ok();
        let input = TaskInput {
            title: patch.title.unwrap_or(candidate.title),
            description: patch.description.or(candidate.description),
            due_at_utc: patch.due_at_utc.or(candidate.due_at),
            timezone: "Asia/Tokyo".to_string(),
            priority: patch
                .priority
                .unwrap_or_else(|| priority_from_str(&candidate.priority)),
            status: None,
            assignee: patch.assignee.or(candidate.assignee),
            project_name: None,
            meeting_id: Some(candidate.meeting_id.clone()),
            linked_file_path: meeting.map(|m| m.target_file_path),
        };
        let task = tasks::create_task(conn, &input)?;
        sync_reminders(conn, None, &task)?;
        meeting_ai::set_candidate_status(conn, &candidate_id, "accepted")?;
        Ok(task)
    })
}
