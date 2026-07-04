use std::path::{Path, PathBuf};
use std::sync::Mutex;

use tauri::{AppHandle, State};

use crate::database::workspaces::{self, WorkspaceRecord};
use crate::error::AppError;
use crate::workspace::watcher;
use crate::DbState;

pub struct ActiveWorkspace {
    pub id: String,
    pub root: PathBuf,
}

#[derive(Default)]
pub struct WorkspaceState {
    pub active: Mutex<Option<ActiveWorkspace>>,
    pub watcher: Mutex<Option<notify::RecommendedWatcher>>,
}

fn lock_error(e: impl std::fmt::Display) -> AppError {
    AppError::new("STATE_LOCK_FAILED", format!("状態ロックに失敗: {e}"), true)
}

/// アクティブワークスペースのルートを返す。未オープンはWORKSPACE_NOT_FOUND。
pub fn active_root(ws: &WorkspaceState) -> Result<PathBuf, AppError> {
    ws.active
        .lock()
        .map_err(lock_error)?
        .as_ref()
        .map(|a| a.root.clone())
        .ok_or_else(|| AppError::new("WORKSPACE_NOT_FOUND", "ワークスペースを開いていません", false))
}

fn activate(
    app: &AppHandle,
    ws: &WorkspaceState,
    record: &WorkspaceRecord,
) -> Result<(), AppError> {
    let root = PathBuf::from(&record.root_path);
    let new_watcher = watcher::start(app.clone(), root.clone())?;
    *ws.watcher.lock().map_err(lock_error)? = Some(new_watcher);
    *ws.active.lock().map_err(lock_error)? = Some(ActiveWorkspace {
        id: record.id.clone(),
        root,
    });
    Ok(())
}

#[tauri::command]
pub fn workspace_open(
    app: AppHandle,
    db: State<'_, DbState>,
    ws: State<'_, WorkspaceState>,
    path: String,
) -> Result<WorkspaceRecord, AppError> {
    let root = Path::new(&path);
    if !root.is_dir() {
        return Err(AppError::new(
            "WORKSPACE_NOT_FOUND",
            format!("フォルダが存在しません: {path}"),
            false,
        ));
    }
    let name = root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.clone());
    let record = {
        let conn = db.0.lock().map_err(lock_error)?;
        workspaces::open_workspace(&conn, &path, &name)?
    };
    activate(&app, &ws, &record)?;
    Ok(record)
}

#[tauri::command]
pub fn workspace_create(
    app: AppHandle,
    db: State<'_, DbState>,
    ws: State<'_, WorkspaceState>,
    path: String,
    name: String,
) -> Result<WorkspaceRecord, AppError> {
    let root = Path::new(&path).join(&name);
    std::fs::create_dir_all(&root)
        .map_err(|e| AppError::new("FILE_IO_ERROR", format!("フォルダを作成できません: {e}"), true))?;
    workspace_open(app, db, ws, root.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn workspace_list_recent(db: State<'_, DbState>) -> Result<Vec<WorkspaceRecord>, AppError> {
    let conn = db.0.lock().map_err(lock_error)?;
    workspaces::list_recent_workspaces(&conn)
}

#[tauri::command]
pub fn workspace_close(ws: State<'_, WorkspaceState>, id: String) -> Result<(), AppError> {
    let mut active = ws.active.lock().map_err(lock_error)?;
    if active.as_ref().map(|a| a.id == id).unwrap_or(false) {
        *active = None;
        *ws.watcher.lock().map_err(lock_error)? = None;
    }
    Ok(())
}
