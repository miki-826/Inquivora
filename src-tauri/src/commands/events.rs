use tauri::State;

use crate::database::events::{self, EventInput, EventPatch, EventRecord};
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
pub fn event_create(state: State<'_, DbState>, input: EventInput) -> Result<EventRecord, AppError> {
    with_conn(&state, |conn| events::create_event(conn, &input))
}

#[tauri::command]
pub fn event_update(
    state: State<'_, DbState>,
    id: String,
    patch: EventPatch,
) -> Result<EventRecord, AppError> {
    with_conn(&state, |conn| events::update_event(conn, &id, &patch))
}

#[tauri::command]
pub fn event_delete(state: State<'_, DbState>, id: String) -> Result<(), AppError> {
    with_conn(&state, |conn| events::delete_event(conn, &id))
}

#[tauri::command]
pub fn event_get_range(
    state: State<'_, DbState>,
    start_utc: String,
    end_utc: String,
) -> Result<Vec<EventRecord>, AppError> {
    with_conn(&state, |conn| {
        events::list_events_in_range(conn, &start_utc, &end_utc)
    })
}
