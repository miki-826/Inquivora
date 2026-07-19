use std::path::Path;

use crate::error::AppError;
use crate::meeting::markdown;
use crate::workspace::encoding::{FileEncoding, LineEnding};
use crate::workspace::ops::{read_text_file, write_text_atomic};

/// 会議開始時に対象ファイルへマーカーブロックを準備する（§9.3）。
/// ファイルがなければタイトル見出し付きで新規作成し、あれば末尾へ追記する。
pub fn ensure_marker_block(path: &Path, meeting_id: &str, title: &str) -> Result<(), AppError> {
    if path.exists() {
        let file = read_text_file(path)?;
        if file.content.contains(&markdown::start_marker(meeting_id)) {
            return Ok(());
        }
        let mut content = file.content;
        while !content.is_empty() && !content.ends_with("\n\n") {
            content.push('\n');
        }
        content.push_str(&markdown::initial_block(meeting_id));
        write_text_atomic(path, &content, file.encoding, file.line_ending)?;
    } else {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| AppError::new("FILE_IO_ERROR", format!("フォルダを作成できません: {e}"), false))?;
        }
        let content = format!("# {title}\n\n{}", markdown::initial_block(meeting_id));
        write_text_atomic(path, &content, FileEncoding::Utf8, LineEnding::Lf)?;
    }
    Ok(())
}

/// 閉じているファイルへセグメントを追記する（§9.5）。文字コード・改行コードを維持する。
pub fn append_segment_to_file(
    path: &Path,
    meeting_id: &str,
    segment_md: &str,
) -> Result<(), AppError> {
    let file = read_text_file(path)?;
    let updated = markdown::insert_before_end_marker(&file.content, meeting_id, segment_md)?;
    write_text_atomic(path, &updated, file.encoding, file.line_ending)?;
    Ok(())
}

/// 議事録AIブロックを対象ファイルへ書き込む（既存ブロックは置き換える）。
pub fn write_ai_block_to_file(
    path: &Path,
    meeting_id: &str,
    ai_block: &str,
) -> Result<(), AppError> {
    let file = read_text_file(path)?;
    let updated = markdown::upsert_ai_block(&file.content, meeting_id, ai_block)?;
    write_text_atomic(path, &updated, file.encoding, file.line_ending)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 新規ファイルはタイトルとマーカー付きで作成される() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("meeting.md");
        ensure_marker_block(&path, "m-1", "定例会議").unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.starts_with("# 定例会議\n"));
        assert!(content.contains("<!-- inquivora:meeting:m-1:start -->"));
        assert!(content.contains("## 文字起こし"));
        assert!(content.contains("<!-- inquivora:meeting:m-1:end -->"));
    }

    #[test]
    fn 既存ファイルには末尾へマーカーが追記される() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notes.md");
        std::fs::write(&path, "# 既存メモ\n\n本文\n").unwrap();
        ensure_marker_block(&path, "m-2", "会議").unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.starts_with("# 既存メモ\n\n本文\n"));
        assert!(content.contains("<!-- inquivora:meeting:m-2:start -->"));
    }

    #[test]
    fn マーカーが既にあれば二重追記しない() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notes.md");
        ensure_marker_block(&path, "m-3", "会議").unwrap();
        ensure_marker_block(&path, "m-3", "会議").unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content.matches("inquivora:meeting:m-3:start").count(), 1);
    }

    #[test]
    fn 閉じているファイルへセグメントを追記できる() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("meeting.md");
        ensure_marker_block(&path, "m-4", "会議").unwrap();
        append_segment_to_file(&path, "m-4", "### 10:03 自分\n\n発言内容\n").unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        let segment_pos = content.find("発言内容").unwrap();
        let end_pos = content.find("<!-- inquivora:meeting:m-4:end -->").unwrap();
        assert!(segment_pos < end_pos);
    }

    #[test]
    fn ai議事録ブロックを書き込み再生成で置き換える() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("meeting.md");
        ensure_marker_block(&path, "m-5", "会議").unwrap();
        let first = markdown::format_ai_block("m-5", "旧要約", &[], &[], &[]);
        write_ai_block_to_file(&path, "m-5", &first).unwrap();
        let second = markdown::format_ai_block("m-5", "新要約", &[], &[], &[]);
        write_ai_block_to_file(&path, "m-5", &second).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content.matches("## AI要約").count(), 1);
        assert!(content.contains("新要約"));
        assert!(!content.contains("旧要約"));
    }
}
