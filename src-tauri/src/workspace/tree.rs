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

/// 指定フォルダ直下の子要素を返す（遅延読み込み・§7.3）。
/// 無視パターンに一致する名前は除外し、フォルダ優先・名前順で返す。
pub fn list_children(
    _root: &Path,
    _relative_path: &str,
    _ignore_patterns: &[String],
) -> Result<Vec<TreeEntry>, AppError> {
    todo!()
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
