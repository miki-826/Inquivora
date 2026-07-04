use std::path::PathBuf;

use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tauri::{AppHandle, Emitter};

use crate::error::AppError;

/// 外部変更イベントのペイロード（§7.8）
#[derive(Clone, serde::Serialize)]
pub struct ExternalChangePayload {
    pub paths: Vec<String>,
}

/// ワークスペースルートを再帰監視し、変更を `file:external-changed` として通知する（§7.8）。
/// 自アプリのアトミック保存で使う一時ファイル（.tmp）は除外する。
pub fn start(app: AppHandle, root: PathBuf) -> Result<RecommendedWatcher, AppError> {
    let mut watcher = notify::recommended_watcher(move |result: notify::Result<notify::Event>| {
        let Ok(event) = result else { return };
        if !matches!(
            event.kind,
            EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
        ) {
            return;
        }
        let paths: Vec<String> = event
            .paths
            .iter()
            .filter(|p| p.extension().map(|e| e != "tmp").unwrap_or(true))
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        if !paths.is_empty() {
            let _ = app.emit("file:external-changed", ExternalChangePayload { paths });
        }
    })
    .map_err(|e| AppError::new("FILE_IO_ERROR", format!("ファイル監視を開始できません: {e}"), true))?;
    watcher
        .watch(&root, RecursiveMode::Recursive)
        .map_err(|e| AppError::new("FILE_IO_ERROR", format!("ファイル監視を開始できません: {e}"), true))?;
    Ok(watcher)
}
