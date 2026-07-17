use tauri::State;

use crate::database::tasks::{self, Task, TaskFilter, TaskInput, TaskPatch};
use crate::error::AppError;
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

#[tauri::command]
pub fn task_create(state: State<'_, DbState>, input: TaskInput) -> Result<Task, AppError> {
    with_conn(&state, |conn| tasks::create_task(conn, &input))
}

#[tauri::command]
pub fn task_update(state: State<'_, DbState>, id: String, patch: TaskPatch) -> Result<Task, AppError> {
    with_conn(&state, |conn| tasks::update_task(conn, &id, &patch))
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
    with_conn(&state, |conn| tasks::complete_task(conn, &id))
}

#[tauri::command]
pub fn task_reopen(state: State<'_, DbState>, id: String) -> Result<Task, AppError> {
    with_conn(&state, |conn| tasks::reopen_task(conn, &id))
}
