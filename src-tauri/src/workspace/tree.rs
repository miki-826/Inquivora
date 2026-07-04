use std::path::Path;

use serde::Serialize;

use crate::error::AppError;
use crate::workspace::filetype::FileCategory;

/// 初期無視設定（§7.3）
pub const DEFAULT_IGNORE_PATTERNS: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "dist",
    "build",
    ".next",
    ".venv",
    "__pycache__",
];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeEntry {
    pub name: String,
    pub relative_path: String,
    pub is_folder: bool,
    pub has_children: bool,
    pub size_bytes: u64,
    pub extension: Option<String>,
    pub category: FileCategory,
}

fn io_error(detail: impl Into<String>) -> AppError {
    AppError::new("FILE_IO_ERROR", detail, true)
}

fn is_ignored(name: &str, ignore_patterns: &[String]) -> bool {
    ignore_patterns.iter().any(|p| p.eq_ignore_ascii_case(name))
}

/// フォルダに無視対象以外の子が1つでもあるかを調べる（展開矢印表示用）
fn folder_has_children(path: &Path, ignore_patterns: &[String]) -> bool {
    let Ok(entries) = std::fs::read_dir(path) else {
        return false;
    };
    for entry in entries.flatten() {
        if !is_ignored(&entry.file_name().to_string_lossy(), ignore_patterns) {
            return true;
        }
    }
    false
}

/// 指定フォルダ直下の子要素を返す（遅延読み込み・§7.3）。
/// 無視パターンに一致する名前は除外し、フォルダ優先・名前順で返す。
pub fn list_children(
    root: &Path,
    relative_path: &str,
    ignore_patterns: &[String],
) -> Result<Vec<TreeEntry>, AppError> {
    let dir = crate::workspace::paths::resolve_in_workspace(root, relative_path)?;
    let read_dir = std::fs::read_dir(&dir)
        .map_err(|e| io_error(format!("フォルダを読み取れません ({}): {e}", dir.display())))?;

    let prefix = relative_path.trim_matches('/');
    let mut entries = Vec::new();
    for entry in read_dir {
        let entry = entry.map_err(|e| io_error(format!("フォルダを読み取れません: {e}")))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if is_ignored(&name, ignore_patterns) {
            continue;
        }
        let metadata = entry
            .metadata()
            .map_err(|e| io_error(format!("ファイル情報を取得できません ({name}): {e}")))?;
        let is_folder = metadata.is_dir();
        let extension = if is_folder {
            None
        } else {
            entry.path().extension().map(|e| e.to_string_lossy().to_lowercase())
        };
        entries.push(TreeEntry {
            relative_path: if prefix.is_empty() {
                name.clone()
            } else {
                format!("{prefix}/{name}")
            },
            is_folder,
            has_children: is_folder && folder_has_children(&entry.path(), ignore_patterns),
            size_bytes: if is_folder { 0 } else { metadata.len() },
            category: match &extension {
                Some(ext) => crate::workspace::filetype::category_for_extension(ext),
                None => FileCategory::Unknown,
            },
            extension,
            name,
        });
    }
    entries.sort_by(|a, b| {
        b.is_folder
            .cmp(&a.is_folder)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn ignore() -> Vec<String> {
        DEFAULT_IGNORE_PATTERNS.iter().map(|s| s.to_string()).collect()
    }

    fn setup() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("docs").join("sub")).unwrap();
        fs::create_dir_all(root.join("node_modules").join("pkg")).unwrap();
        fs::create_dir_all(root.join(".git")).unwrap();
        fs::create_dir_all(root.join("empty")).unwrap();
        fs::write(root.join("readme.md"), "# hi").unwrap();
        fs::write(root.join("Zeta.txt"), "z").unwrap();
        fs::write(root.join("docs").join("a.md"), "a").unwrap();
        fs::write(root.join("docs").join("image.png"), [137u8, 80]).unwrap();
        dir
    }

    #[test]
    fn ルート直下を列挙し無視パターンを除外する() {
        let dir = setup();
        let entries = list_children(dir.path(), "", &ignore()).unwrap();
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["docs", "empty", "readme.md", "Zeta.txt"]);
    }

    #[test]
    fn フォルダが先で名前は大文字小文字を無視した順() {
        let dir = setup();
        let entries = list_children(dir.path(), "", &ignore()).unwrap();
        assert!(entries[0].is_folder && entries[1].is_folder);
        assert!(!entries[2].is_folder && !entries[3].is_folder);
    }

    #[test]
    fn 子の有無を返す() {
        let dir = setup();
        let entries = list_children(dir.path(), "", &ignore()).unwrap();
        let docs = entries.iter().find(|e| e.name == "docs").unwrap();
        let empty = entries.iter().find(|e| e.name == "empty").unwrap();
        assert!(docs.has_children);
        assert!(!empty.has_children);
    }

    #[test]
    fn サブフォルダを列挙できる() {
        let dir = setup();
        let entries = list_children(dir.path(), "docs", &ignore()).unwrap();
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["sub", "a.md", "image.png"]);
        let md = entries.iter().find(|e| e.name == "a.md").unwrap();
        assert_eq!(md.relative_path, "docs/a.md");
        assert_eq!(md.extension.as_deref(), Some("md"));
        assert_eq!(md.category, FileCategory::Edit);
    }

    #[test]
    fn パストラバーサルを拒否する() {
        let dir = setup();
        let err = list_children(dir.path(), "../outside", &ignore()).unwrap_err();
        assert_eq!(err.code, "PATH_OUTSIDE_WORKSPACE");
    }

    #[test]
    fn 存在しないフォルダはfile_io_error() {
        let dir = setup();
        let err = list_children(dir.path(), "missing", &ignore()).unwrap_err();
        assert_eq!(err.code, "FILE_IO_ERROR");
    }
}
