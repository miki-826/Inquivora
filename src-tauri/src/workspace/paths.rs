use std::path::{Path, PathBuf};

use crate::error::AppError;

fn outside_workspace(detail: impl Into<String>) -> AppError {
    AppError::new("PATH_OUTSIDE_WORKSPACE", detail, false)
}

/// ワークスペースルートからの相対パスを検証して絶対パスへ解決する（§20.1）。
/// `..`・絶対パス・ドライブ指定を含む相対パスは PATH_OUTSIDE_WORKSPACE で拒否する。
pub fn resolve_in_workspace(root: &Path, relative: &str) -> Result<PathBuf, AppError> {
    let normalized = relative.replace('\\', "/");
    if normalized.starts_with('/') || normalized.contains(':') {
        return Err(outside_workspace(format!("絶対パスは指定できません: {relative}")));
    }
    let mut resolved = root.to_path_buf();
    for segment in normalized.split('/') {
        match segment {
            "" | "." => continue,
            ".." => return Err(outside_workspace(format!("親ディレクトリ参照は許可されません: {relative}"))),
            _ => resolved.push(segment),
        }
    }
    Ok(resolved)
}

/// パスを比較用セグメント列へ正規化する。`.` は除去し、`..` は None（拒否）を返す。
fn normalized_segments(path: &Path) -> Option<Vec<String>> {
    let raw = path.to_string_lossy().replace('\\', "/");
    let mut segments = Vec::new();
    for segment in raw.split('/') {
        match segment {
            "" | "." => continue,
            ".." => return None,
            _ => segments.push(segment.to_lowercase()),
        }
    }
    Some(segments)
}

/// 絶対パスがワークスペースルート配下であることを検証する（§20.1）。
/// Windowsの大文字小文字差・区切り文字差を吸収し、前方一致の誤判定
/// （`C:\ws` と `C:\ws2`）を起こさない。
pub fn ensure_within_workspace(root: &Path, absolute: &Path) -> Result<(), AppError> {
    let root_segments = normalized_segments(root)
        .ok_or_else(|| outside_workspace("ワークスペースルートが不正です"))?;
    let target_segments = normalized_segments(absolute)
        .ok_or_else(|| outside_workspace(format!("親ディレクトリ参照は許可されません: {}", absolute.display())))?;
    if target_segments.len() < root_segments.len()
        || target_segments[..root_segments.len()] != root_segments[..]
    {
        return Err(outside_workspace(format!(
            "ワークスペース外のパスです: {}",
            absolute.display()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> PathBuf {
        PathBuf::from(r"C:\Users\test\workspace")
    }

    #[test]
    fn 通常の相対パスを解決できる() {
        let resolved = resolve_in_workspace(&root(), "docs/notes.md").unwrap();
        assert_eq!(resolved, root().join("docs").join("notes.md"));
    }

    #[test]
    fn バックスラッシュ区切りも解決できる() {
        let resolved = resolve_in_workspace(&root(), r"docs\sub\a.txt").unwrap();
        assert_eq!(resolved, root().join("docs").join("sub").join("a.txt"));
    }

    #[test]
    fn 空文字はルート自身を返す() {
        assert_eq!(resolve_in_workspace(&root(), "").unwrap(), root());
    }

    #[test]
    fn 親ディレクトリ参照を拒否する() {
        let err = resolve_in_workspace(&root(), "../secret.txt").unwrap_err();
        assert_eq!(err.code, "PATH_OUTSIDE_WORKSPACE");
        let err = resolve_in_workspace(&root(), r"docs\..\..\secret.txt").unwrap_err();
        assert_eq!(err.code, "PATH_OUTSIDE_WORKSPACE");
    }

    #[test]
    fn 絶対パスやドライブ指定を拒否する() {
        for bad in [r"C:\Windows\system.ini", r"\\server\share\a.txt", r"\etc\passwd", "D:evil.txt"] {
            let err = resolve_in_workspace(&root(), bad).unwrap_err();
            assert_eq!(err.code, "PATH_OUTSIDE_WORKSPACE", "拒否されるべき: {bad}");
        }
    }

    #[test]
    fn カレント参照は除去して解決する() {
        let resolved = resolve_in_workspace(&root(), "./docs/./a.md").unwrap();
        assert_eq!(resolved, root().join("docs").join("a.md"));
    }

    #[test]
    fn 配下の絶対パスは許可する() {
        ensure_within_workspace(&root(), &root().join("docs").join("a.md")).unwrap();
        ensure_within_workspace(&root(), &root()).unwrap();
    }

    #[test]
    fn 大文字小文字差を許容する() {
        ensure_within_workspace(
            Path::new(r"C:\Users\Test\Workspace"),
            Path::new(r"c:\users\test\workspace\a.md"),
        )
        .unwrap();
    }

    #[test]
    fn ルート外の絶対パスを拒否する() {
        let err = ensure_within_workspace(&root(), Path::new(r"C:\Users\test\other\a.md")).unwrap_err();
        assert_eq!(err.code, "PATH_OUTSIDE_WORKSPACE");
    }

    #[test]
    fn 名前が前方一致するだけの別フォルダを拒否する() {
        let err = ensure_within_workspace(
            Path::new(r"C:\ws"),
            Path::new(r"C:\ws2\a.md"),
        )
        .unwrap_err();
        assert_eq!(err.code, "PATH_OUTSIDE_WORKSPACE");
    }

    #[test]
    fn 親参照を含む絶対パスを拒否する() {
        let err = ensure_within_workspace(&root(), &root().join("docs").join("..").join("..").join("a.md"))
            .unwrap_err();
        assert_eq!(err.code, "PATH_OUTSIDE_WORKSPACE");
    }
}
