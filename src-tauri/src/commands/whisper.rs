use tauri::{AppHandle, State};

use crate::error::AppError;
use crate::whisper::{download, models};
use crate::DbState;

fn lock_error(e: impl std::fmt::Display) -> AppError {
    AppError::database(format!("DB接続ロックに失敗: {e}"))
}

#[tauri::command]
pub fn whisper_model_status(
    app: AppHandle,
    state: State<DbState>,
) -> Result<Vec<models::ModelStatus>, AppError> {
    let conn = state.0.lock().map_err(lock_error)?;
    let selected = models::selected_model(&conn)?;
    let dir = download::models_dir(&app)?;
    Ok(models::model_status(&dir, &selected))
}

#[tauri::command]
pub fn whisper_model_select(state: State<DbState>, name: String) -> Result<(), AppError> {
    let conn = state.0.lock().map_err(lock_error)?;
    models::set_selected_model(&conn, &name)
}

#[tauri::command]
pub async fn whisper_model_download(app: AppHandle, name: String) -> Result<(), AppError> {
    download::download_model(&app, &name).await
}

#[tauri::command]
pub fn whisper_model_delete(app: AppHandle, name: String) -> Result<(), AppError> {
    let dir = download::models_dir(&app)?;
    let path = models::model_path(&dir, &name).ok_or_else(|| {
        AppError::new("VALIDATION_ERROR", format!("不明なWhisperモデルです: {name}"), false)
    })?;
    if path.is_file() {
        std::fs::remove_file(&path).map_err(|e| {
            AppError::new("FILE_DELETE_FAILED", format!("モデルの削除に失敗しました: {e}"), false)
        })?;
    }
    Ok(())
}
