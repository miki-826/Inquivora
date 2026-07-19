use std::path::Path;

use serde::Serialize;
use tauri::State;

use crate::commands::workspace::{active_root, WorkspaceState};
use crate::database::settings;
use crate::error::AppError;
use crate::workspace::encoding::{FileEncoding, LineEnding};
use crate::workspace::filetype::{self, FileCategory};
use crate::workspace::ops::{self, FileContent, FileMeta};
use crate::workspace::paths::ensure_within_workspace;
use crate::workspace::tree::{self, TreeEntry, DEFAULT_IGNORE_PATTERNS};
use crate::search as indexer;
use crate::DbState;

pub const IGNORE_PATTERNS_KEY: &str = "workspace.ignorePatterns";

fn lock_error(e: impl std::fmt::Display) -> AppError {
    AppError::new("STATE_LOCK_FAILED", format!("状態ロックに失敗: {e}"), true)
}

fn ignore_patterns(db: &State<'_, DbState>) -> Result<Vec<String>, AppError> {
    let conn = db.0.lock().map_err(lock_error)?;
    match settings::get_setting(&conn, IGNORE_PATTERNS_KEY)? {
        Some(serde_json::Value::Array(items)) => Ok(items
            .into_iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect()),
        _ => Ok(DEFAULT_IGNORE_PATTERNS.iter().map(|s| s.to_string()).collect()),
    }
}

/// アクティブワークスペース配下の絶対パスであることを検証する（§20.1）
fn checked_path<'a>(ws: &WorkspaceState, path: &'a str) -> Result<&'a Path, AppError> {
    let root = active_root(ws)?;
    let target = Path::new(path);
    ensure_within_workspace(&root, target)?;
    Ok(target)
}

#[tauri::command]
pub fn file_list_children(
    db: State<'_, DbState>,
    ws: State<'_, WorkspaceState>,
    relative_path: String,
) -> Result<Vec<TreeEntry>, AppError> {
    let root = active_root(&ws)?;
    tree::list_children(&root, &relative_path, &ignore_patterns(&db)?)
}

#[tauri::command]
pub fn file_read(ws: State<'_, WorkspaceState>, path: String) -> Result<FileContent, AppError> {
    ops::read_text_file(checked_path(&ws, &path)?)
}

#[tauri::command]
pub fn file_write_atomic(
    db: State<'_, DbState>,
    ws: State<'_, WorkspaceState>,
    path: String,
    content: String,
    encoding: FileEncoding,
    line_ending: LineEnding,
) -> Result<FileMeta, AppError> {
    let target = checked_path(&ws, &path)?;
    let meta = ops::write_text_atomic(target, &content, encoding, line_ending)?;
    if let Ok(conn) = db.0.lock() {
        let name = target
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let _ = indexer::index_file(&conn, &target.to_string_lossy(), &name, &content);
    }
    Ok(meta)
}

#[tauri::command]
pub fn file_create(
    ws: State<'_, WorkspaceState>,
    path: String,
    entry_type: String,
) -> Result<(), AppError> {
    ops::create_entry(checked_path(&ws, &path)?, entry_type == "folder")
}

#[tauri::command]
pub fn file_rename(
    db: State<'_, DbState>,
    ws: State<'_, WorkspaceState>,
    old_path: String,
    new_path: String,
) -> Result<(), AppError> {
    let old = checked_path(&ws, &old_path)?;
    let new = checked_path(&ws, &new_path)?;
    ops::rename_entry(old, new)?;
    if let Ok(conn) = db.0.lock() {
        let _ = indexer::remove_entity(&conn, indexer::TYPE_FILE, &old.to_string_lossy());
        if let Ok(file) = ops::read_text_file(new) {
            let name = new
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let _ = indexer::index_file(&conn, &new.to_string_lossy(), &name, &file.content);
        }
    }
    Ok(())
}

#[tauri::command]
pub fn file_delete(
    db: State<'_, DbState>,
    ws: State<'_, WorkspaceState>,
    path: String,
    use_recycle_bin: bool,
) -> Result<(), AppError> {
    let target = checked_path(&ws, &path)?;
    ops::delete_entry(target, use_recycle_bin)?;
    if let Ok(conn) = db.0.lock() {
        let _ = indexer::remove_entity(&conn, indexer::TYPE_FILE, &target.to_string_lossy());
    }
    Ok(())
}

#[tauri::command]
pub fn file_copy(
    ws: State<'_, WorkspaceState>,
    source: String,
    destination: String,
) -> Result<(), AppError> {
    let src = checked_path(&ws, &source)?;
    let dst = checked_path(&ws, &destination)?;
    ops::copy_entry(src, dst)
}

#[tauri::command]
pub fn file_move(
    ws: State<'_, WorkspaceState>,
    source: String,
    destination: String,
) -> Result<(), AppError> {
    let src = checked_path(&ws, &source)?;
    let dst = checked_path(&ws, &destination)?;
    ops::move_entry(src, dst)
}

#[tauri::command]
pub fn file_reveal(ws: State<'_, WorkspaceState>, path: String) -> Result<(), AppError> {
    let target = checked_path(&ws, &path)?;
    std::process::Command::new("explorer.exe")
        .arg(format!("/select,{}", target.display()))
        .spawn()
        .map_err(|e| AppError::new("FILE_IO_ERROR", format!("エクスプローラーを起動できません: {e}"), true))?;
    Ok(())
}

#[tauri::command]
pub fn file_open_external(
    app: tauri::AppHandle,
    ws: State<'_, WorkspaceState>,
    path: String,
) -> Result<(), AppError> {
    use tauri_plugin_opener::OpenerExt;
    let target = checked_path(&ws, &path)?;
    app.opener()
        .open_path(target.to_string_lossy(), None::<&str>)
        .map_err(|e| AppError::new("FILE_IO_ERROR", format!("既定アプリで開けません: {e}"), true))
}

#[tauri::command]
pub fn file_read_base64(ws: State<'_, WorkspaceState>, path: String) -> Result<String, AppError> {
    ops::read_file_base64(checked_path(&ws, &path)?)
}

/// エクスプローラー等からドロップされたファイルをbase64で受け取り、
/// target_dir配下へ取り込む。同名衝突時は連番で別名保存する。
#[tauri::command]
pub fn file_import(
    ws: State<'_, WorkspaceState>,
    target_dir: String,
    name: String,
    contents_base64: String,
) -> Result<(), AppError> {
    use base64::Engine;
    let dir = checked_path(&ws, &target_dir)?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(contents_base64.as_bytes())
        .map_err(|e| AppError::new("VALIDATION_ERROR", format!("取込データが不正です: {e}"), false))?;
    ops::import_file(dir, &name, &bytes)?;
    Ok(())
}

#[tauri::command]
pub fn tabs_save(
    db: State<'_, DbState>,
    workspace_id: String,
    tabs: Vec<crate::database::tabs::RecentTab>,
) -> Result<(), AppError> {
    let conn = db.0.lock().map_err(lock_error)?;
    crate::database::tabs::save_tabs(&conn, &workspace_id, &tabs)
}

#[tauri::command]
pub fn tabs_load(
    db: State<'_, DbState>,
    workspace_id: String,
) -> Result<Vec<crate::database::tabs::RecentTab>, AppError> {
    let conn = db.0.lock().map_err(lock_error)?;
    crate::database::tabs::load_tabs(&conn, &workspace_id)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedType {
    pub category: FileCategory,
    pub is_probably_text: bool,
}

#[tauri::command]
pub fn file_detect_type(ws: State<'_, WorkspaceState>, path: String) -> Result<DetectedType, AppError> {
    use std::io::Read;
    let target = checked_path(&ws, &path)?;
    let category = target
        .extension()
        .map(|e| filetype::category_for_extension(&e.to_string_lossy()))
        .unwrap_or(FileCategory::Unknown);
    let is_probably_text = if category == FileCategory::Edit {
        true
    } else {
        let mut sample = vec![0u8; 8192];
        let read = std::fs::File::open(target)
            .and_then(|mut f| f.read(&mut sample))
            .map_err(|e| AppError::new("FILE_IO_ERROR", format!("読み込めません: {e}"), true))?;
        filetype::is_probably_text(&sample[..read])
    };
    Ok(DetectedType {
        category,
        is_probably_text,
    })
}
