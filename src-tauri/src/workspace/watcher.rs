use std::path::PathBuf;

use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tauri::{AppHandle, Emitter};

use crate::error::AppError;

const IGNORED_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    "target",
    "dist",
    "build",
    ".next",
    ".venv",
    "__pycache__",
];

fn should_emit_path(root: &std::path::Path, path: &std::path::Path) -> bool {
    let relative = path.strip_prefix(root).unwrap_or(path);
    if relative.components().any(|component| {
        let name = component.as_os_str().to_string_lossy();
        IGNORED_DIRS.iter().any(|ignored| name.eq_ignore_ascii_case(ignored))
    }) {
        return false;
    }
    !path
        .extension()
        .map(|extension| extension.to_string_lossy().eq_ignore_ascii_case("tmp"))
        .unwrap_or(false)
}

/// 外部変更イベントのペイロード（§7.8）
#[derive(Clone, serde::Serialize)]
pub struct ExternalChangePayload {
    pub paths: Vec<String>,
}

/// ワークスペースルートを再帰監視し、変更を `file:external-changed` として通知する（§7.8）。
/// 自アプリのアトミック保存で使う一時ファイル（.tmp）は除外する。
pub fn start(app: AppHandle, root: PathBuf) -> Result<RecommendedWatcher, AppError> {
    let event_root = root.clone();
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
            .filter(|path| should_emit_path(&event_root, path))
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

#[cfg(test)]
mod tests {
    use super::should_emit_path;
    use std::path::Path;

    #[test]
    fn 重い生成物ディレクトリと一時ファイルを通知しない() {
        let root = Path::new("C:/workspace");
        assert!(!should_emit_path(root, Path::new("C:/workspace/node_modules/pkg/a.js")));
        assert!(!should_emit_path(root, Path::new("C:/workspace/.git/index")));
        assert!(!should_emit_path(root, Path::new("C:/workspace/note.md.tmp")));
        assert!(should_emit_path(root, Path::new("C:/workspace/notes/today.md")));
    }
}
