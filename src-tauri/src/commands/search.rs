use tauri::{AppHandle, Emitter, Manager, State};

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

/// §15.3/§17.7 索引の全再構築。UIをブロックしないよう別スレッドで実行し、
/// ファイル走査はDBロック外で行い、書込のみ1トランザクションで短時間ロックする。
#[tauri::command]
pub fn search_reindex_workspace(app: AppHandle, ws: State<'_, WorkspaceState>) -> Result<(), AppError> {
    let root = active_root(&ws).ok();
    let handle = app.clone();
    std::thread::spawn(move || {
        let _ = handle.emit("search:index-started", ());
        let file_docs = root
            .as_deref()
            .map(indexer::collect_workspace_docs)
            .unwrap_or_default();
        let count = match handle.state::<DbState>().0.lock() {
            Ok(mut conn) => indexer::reindex(&mut conn, file_docs).unwrap_or(0),
            Err(_) => 0,
        };
        let _ = handle.emit("search:index-done", count);
    });
    Ok(())
}

/// 変更されたパスだけを索引へ反映する（ウォッチャの外部変更追従用）。
#[tauri::command]
pub fn search_index_paths(db: State<'_, DbState>, paths: Vec<String>) -> Result<(), AppError> {
    let conn = db.0.lock().map_err(lock_error)?;
    for path in &paths {
        let _ = indexer::sync_path(&conn, path);
    }
    Ok(())
}
