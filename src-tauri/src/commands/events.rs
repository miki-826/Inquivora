use tauri::State;

use crate::database::events::{self, EventInput, EventPatch, EventRecord};
use crate::database::settings;
use crate::error::AppError;
use crate::notifications::schedule::parse_settings;
use crate::notifications::sync::sync_after_event_saved;
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
    old: Option<&EventRecord>,
    event: &EventRecord,
) -> Result<(), AppError> {
    let config = parse_settings(settings::get_setting(conn, "notifications")?);
    sync_after_event_saved(conn, old, event, &config, chrono::Utc::now())
}

#[tauri::command]
pub fn event_create(state: State<'_, DbState>, input: EventInput) -> Result<EventRecord, AppError> {
    with_conn(&state, |conn| {
        let event = events::create_event(conn, &input)?;
        sync_reminders(conn, None, &event)?;
        Ok(event)
    })
}

#[tauri::command]
pub fn event_update(
    state: State<'_, DbState>,
    id: String,
    patch: EventPatch,
) -> Result<EventRecord, AppError> {
    with_conn(&state, |conn| {
        let old = events::get_event(conn, &id)?;
        let event = events::update_event(conn, &id, &patch)?;
        sync_reminders(conn, Some(&old), &event)?;
        Ok(event)
    })
}

#[tauri::command]
pub fn event_get(state: State<'_, DbState>, id: String) -> Result<EventRecord, AppError> {
    with_conn(&state, |conn| events::get_event(conn, &id))
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
