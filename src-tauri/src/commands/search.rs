use tauri::State;

use crate::commands::workspace::{active_root, WorkspaceState};
use crate::database::search::{self, SearchResult};
use crate::error::AppError;
use crate::search as indexer;
use crate::DbState;

fn lock_error(e: impl std::fmt::Display) -> AppError {
    AppError::new("STATE_LOCK_FAILED", format!("状態ロックに失敗: {e}"), true)
}

/// §17.7 全文検索。空クエリは空配列を返す。
#[tauri::command]
pub fn search_global(
    db: State<'_, DbState>,
    query: String,
    entity_types: Option<Vec<String>>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<SearchResult>, AppError> {
    let conn = db.0.lock().map_err(lock_error)?;
    search::search(
        &conn,
        &query,
        &entity_types.unwrap_or_default(),
        limit.unwrap_or(50).clamp(1, 200),
        offset.unwrap_or(0).max(0),
    )
}

/// §17.7 索引の全再構築。アクティブワークスペース配下のファイルも走査する。
#[tauri::command]
pub fn search_reindex_workspace(
    db: State<'_, DbState>,
    ws: State<'_, WorkspaceState>,
) -> Result<usize, AppError> {
    let root = active_root(&ws).ok();
    let conn = db.0.lock().map_err(lock_error)?;
    indexer::reindex(&conn, root.as_deref())
}
