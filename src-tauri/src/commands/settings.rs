use serde_json::Value;
use tauri::State;

use crate::database::settings;
use crate::error::AppError;
use crate::DbState;

#[tauri::command]
pub fn settings_get(state: State<'_, DbState>, key: String) -> Result<Option<Value>, AppError> {
    let conn = state
        .0
        .lock()
        .map_err(|e| AppError::database(format!("DB接続ロックに失敗: {e}")))?;
    settings::get_setting(&conn, &key)
}

#[tauri::command]
pub fn settings_set(state: State<'_, DbState>, key: String, value: Value) -> Result<(), AppError> {
    let conn = state
        .0
        .lock()
        .map_err(|e| AppError::database(format!("DB接続ロックに失敗: {e}")))?;
    settings::set_setting(&conn, &key, &value)
}
