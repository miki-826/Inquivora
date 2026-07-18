use tauri::State;

use crate::database::settings;
use crate::database::tasks::{self, Task, TaskFilter, TaskInput, TaskPatch};
use crate::error::AppError;
use crate::notifications::schedule::parse_settings;
use crate::notifications::sync::sync_after_task_saved;
use crate::DbState;

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
