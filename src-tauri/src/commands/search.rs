use std::collections::HashSet;
use std::path::PathBuf;

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
        let mut count = 0usize;
        // DB由来（タスク/予定/会議）と全索引のクリアは1トランザクションで短時間ロック
        if let Ok(mut conn) = handle.state::<DbState>().0.lock() {
            count += indexer::reindex_entities(&mut conn).unwrap_or(0);
        }
        // ファイルはパスだけ収集し、バッチ単位で「ロック外読み込み→短時間ロック書込」を繰り返す。
        // 一度に全ファイルをメモリへ載せないため省メモリ。
        if let Some(root) = root {
            let paths = indexer::collect_workspace_file_paths(&root);
            for batch in paths.chunks(indexer::INDEX_BATCH) {
                let docs = indexer::read_file_docs(batch);
                if let Ok(mut conn) = handle.state::<DbState>().0.lock() {
                    let _ = indexer::write_file_docs(&mut conn, &docs);
                }
                count += docs.len();
            }
        }
        let _ = handle.emit("search:index-done", count);
    });
    Ok(())
}

/// 変更されたパスだけを索引へ反映する（ウォッチャの外部変更追従用）。
#[tauri::command]
pub fn search_index_paths(db: State<'_, DbState>, paths: Vec<String>) -> Result<(), AppError> {
    let mut seen = HashSet::new();
    let paths: Vec<String> = paths
        .into_iter()
        .filter(|path| seen.insert(path.clone()))
        .collect();
    let path_bufs: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();

    // ディスクI/OはDB mutexの外で行う。これにより大量の監視イベント中でも
    // タスク・予定・設定など他のDB操作を待たせない。
    let docs = indexer::read_file_docs(&path_bufs);
    let mut conn = db.0.lock().map_err(lock_error)?;
    indexer::replace_file_docs(&mut conn, &paths, &docs)
}
