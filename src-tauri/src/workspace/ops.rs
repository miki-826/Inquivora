use std::path::Path;

use serde::Serialize;

use crate::error::AppError;
use crate::workspace::encoding::{FileEncoding, LineEnding};

/// 大容量ファイルの取り扱い（§8.4）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReadMode {
    Normal,
    ReadOnly,
    Preview,
}

pub const READ_ONLY_THRESHOLD: u64 = 10 * 1024 * 1024;
pub const PREVIEW_THRESHOLD: u64 = 100 * 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileContent {
    pub content: String,
    pub encoding: FileEncoding,
    pub line_ending: LineEnding,
    pub size_bytes: u64,
    pub modified_at: String,
    pub read_mode: ReadMode,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileMeta {
    pub size_bytes: u64,
    pub modified_at: String,
}

pub fn read_mode_for_size(_size_bytes: u64) -> ReadMode {
    todo!()
}

/// テキストファイルを読み込み、文字コード・改行コードを判定して返す（§7.5〜7.6、§8.4）
pub fn read_text_file(_path: &Path) -> Result<FileContent, AppError> {
    todo!()
}

/// 一時ファイル→fsync→置換のアトミック保存（§7.7）。
/// 元の文字コード・改行コードを維持して書き込む。
pub fn write_text_atomic(
    _path: &Path,
    _content: &str,
    _encoding: FileEncoding,
    _line_ending: LineEnding,
) -> Result<FileMeta, AppError> {
    todo!()
}

/// ファイルまたはフォルダを作成する。既存パスにはFILE_IO_ERRORを返す。
pub fn create_entry(_path: &Path, _is_folder: bool) -> Result<(), AppError> {
    todo!()
}

pub fn rename_entry(_old_path: &Path, _new_path: &Path) -> Result<(), AppError> {
    todo!()
}

/// 削除。既定はごみ箱を使用する（§20.1）。
pub fn delete_entry(_path: &Path, _use_recycle_bin: bool) -> Result<(), AppError> {
    todo!()
}

/// ファイルまたはフォルダ（再帰）をコピーする。
pub fn copy_entry(_source: &Path, _destination: &Path) -> Result<(), AppError> {
    todo!()
}

pub fn move_entry(_source: &Path, _destination: &Path) -> Result<(), AppError> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn サイズにより読み取りモードが変わる() {
        assert_eq!(read_mode_for_size(0), ReadMode::Normal);
        assert_eq!(read_mode_for_size(READ_ONLY_THRESHOLD - 1), ReadMode::Normal);
        assert_eq!(read_mode_for_size(READ_ONLY_THRESHOLD), ReadMode::ReadOnly);
        assert_eq!(read_mode_for_size(PREVIEW_THRESHOLD - 1), ReadMode::ReadOnly);
        assert_eq!(read_mode_for_size(PREVIEW_THRESHOLD), ReadMode::Preview);
    }

    #[test]
    fn utf8ファイルを読み込める() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("メモ.md");
        fs::write(&path, "# 見出し\r\n本文です。\r\n").unwrap();
        let file = read_text_file(&path).unwrap();
        assert_eq!(file.content, "# 見出し\r\n本文です。\r\n");
        assert_eq!(file.encoding, FileEncoding::Utf8);
        assert_eq!(file.line_ending, LineEnding::Crlf);
        assert_eq!(file.read_mode, ReadMode::Normal);
        assert!(file.modified_at.contains('T'));
    }

    #[test]
    fn shift_jisファイルの読み書きで文字コードを維持する() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sjis.txt");
        let (bytes, _, _) = encoding_rs::SHIFT_JIS.encode("日本語テキスト\r\n");
        fs::write(&path, &bytes).unwrap();

        let file = read_text_file(&path).unwrap();
        assert_eq!(file.encoding, FileEncoding::ShiftJis);

        write_text_atomic(&path, "更新済み\r\n", file.encoding, file.line_ending).unwrap();
        let (expected, _, _) = encoding_rs::SHIFT_JIS.encode("更新済み\r\n");
        assert_eq!(fs::read(&path).unwrap(), expected.into_owned());
    }

    #[test]
    fn アトミック保存は既存ファイルを置換し一時ファイルを残さない() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.txt");
        fs::write(&path, "old").unwrap();

        let meta = write_text_atomic(&path, "new content\n", FileEncoding::Utf8, LineEnding::Lf).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "new content\n");
        assert_eq!(meta.size_bytes, "new content\n".len() as u64);

        let leftovers: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|name| name != "a.txt")
            .collect();
        assert!(leftovers.is_empty(), "一時ファイルが残存: {leftovers:?}");
    }

    #[test]
    fn アトミック保存は改行コードを統一する() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.txt");
        write_text_atomic(&path, "a\nb\nc", FileEncoding::Utf8, LineEnding::Crlf).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "a\r\nb\r\nc");
    }

    #[test]
    fn ファイルとフォルダを作成できる() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("new.md");
        let folder = dir.path().join("docs");
        create_entry(&file, false).unwrap();
        create_entry(&folder, true).unwrap();
        assert!(file.is_file());
        assert!(folder.is_dir());
    }

    #[test]
    fn 既存パスへの作成は失敗する() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("dup.md");
        create_entry(&file, false).unwrap();
        let err = create_entry(&file, false).unwrap_err();
        assert_eq!(err.code, "FILE_IO_ERROR");
    }

    #[test]
    fn 名前変更できる() {
        let dir = tempfile::tempdir().unwrap();
        let old = dir.path().join("old.md");
        let new = dir.path().join("new.md");
        fs::write(&old, "x").unwrap();
        rename_entry(&old, &new).unwrap();
        assert!(!old.exists());
        assert_eq!(fs::read_to_string(&new).unwrap(), "x");
    }

    #[test]
    fn 名前変更先が既存なら失敗する() {
        let dir = tempfile::tempdir().unwrap();
        let old = dir.path().join("old.md");
        let new = dir.path().join("new.md");
        fs::write(&old, "x").unwrap();
        fs::write(&new, "y").unwrap();
        assert!(rename_entry(&old, &new).is_err());
        assert_eq!(fs::read_to_string(&new).unwrap(), "y");
    }

    #[test]
    fn ごみ箱を使わない削除はフォルダも再帰的に消す() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().join("sub");
        fs::create_dir(&folder).unwrap();
        fs::write(folder.join("a.txt"), "x").unwrap();
        delete_entry(&folder, false).unwrap();
        assert!(!folder.exists());
    }

    #[test]
    fn フォルダを再帰コピーできる() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src");
        fs::create_dir_all(src.join("nested")).unwrap();
        fs::write(src.join("a.txt"), "A").unwrap();
        fs::write(src.join("nested").join("b.txt"), "B").unwrap();

        let dst = dir.path().join("dst");
        copy_entry(&src, &dst).unwrap();
        assert_eq!(fs::read_to_string(dst.join("a.txt")).unwrap(), "A");
        assert_eq!(fs::read_to_string(dst.join("nested").join("b.txt")).unwrap(), "B");
        assert!(src.exists());
    }

    #[test]
    fn ファイルを移動できる() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("a.txt");
        let dst_dir = dir.path().join("moved");
        fs::create_dir(&dst_dir).unwrap();
        fs::write(&src, "x").unwrap();
        move_entry(&src, &dst_dir.join("a.txt")).unwrap();
        assert!(!src.exists());
        assert_eq!(fs::read_to_string(dst_dir.join("a.txt")).unwrap(), "x");
    }

    #[test]
    fn バイナリファイルの読み込みはエラーになる() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bin.dat");
        fs::write(&path, [0u8, 159, 146, 150, 0, 1, 2]).unwrap();
        let err = read_text_file(&path).unwrap_err();
        assert_eq!(err.code, "FILE_ENCODING_UNSUPPORTED");
    }
}
