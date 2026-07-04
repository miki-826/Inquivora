use std::path::Path;

use serde::Serialize;

use crate::error::AppError;
use crate::workspace::encoding::{self, FileEncoding, LineEnding};

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

fn io_error(detail: impl Into<String>) -> AppError {
    AppError::new("FILE_IO_ERROR", detail, true)
}

const PREVIEW_CHUNK_BYTES: u64 = 256 * 1024;

pub fn read_mode_for_size(size_bytes: u64) -> ReadMode {
    if size_bytes >= PREVIEW_THRESHOLD {
        ReadMode::Preview
    } else if size_bytes >= READ_ONLY_THRESHOLD {
        ReadMode::ReadOnly
    } else {
        ReadMode::Normal
    }
}

fn modified_at_rfc3339(path: &Path) -> Result<String, AppError> {
    let modified = std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .map_err(|e| io_error(format!("更新日時を取得できません: {e}")))?;
    Ok(chrono::DateTime::<chrono::Utc>::from(modified).to_rfc3339())
}

/// 100MB超は先頭・末尾のみ読み込み、判定した文字コードで損失許容デコードする（§8.4）
fn read_preview(path: &Path, size_bytes: u64) -> Result<(String, FileEncoding), AppError> {
    use std::io::{Read, Seek, SeekFrom};
    let mut file = std::fs::File::open(path).map_err(|e| io_error(format!("開けません: {e}")))?;
    let mut head = vec![0u8; PREVIEW_CHUNK_BYTES as usize];
    file.read_exact(&mut head).map_err(|e| io_error(format!("読み込めません: {e}")))?;
    let mut tail = vec![0u8; PREVIEW_CHUNK_BYTES as usize];
    file.seek(SeekFrom::Start(size_bytes - PREVIEW_CHUNK_BYTES))
        .and_then(|_| file.read_exact(&mut tail))
        .map_err(|e| io_error(format!("読み込めません: {e}")))?;

    let (encoding_impl, encoding, head_body): (&'static encoding_rs::Encoding, FileEncoding, &[u8]) =
        if head.starts_with(&[0xEF, 0xBB, 0xBF]) {
            (encoding_rs::UTF_8, FileEncoding::Utf8Bom, &head[3..])
        } else if head.starts_with(&[0xFF, 0xFE]) {
            (encoding_rs::UTF_16LE, FileEncoding::Utf16Le, &head[2..])
        } else if head.starts_with(&[0xFE, 0xFF]) {
            (encoding_rs::UTF_16BE, FileEncoding::Utf16Be, &head[2..])
        } else if std::str::from_utf8(&head).is_ok() {
            (encoding_rs::UTF_8, FileEncoding::Utf8, &head[..])
        } else {
            (encoding_rs::SHIFT_JIS, FileEncoding::ShiftJis, &head[..])
        };
    let (head_text, _, _) = encoding_impl.decode(head_body);
    let (tail_text, _, _) = encoding_impl.decode(&tail);
    Ok((format!("{head_text}\n\n…（中略）…\n\n{tail_text}"), encoding))
}

/// テキストファイルを読み込み、文字コード・改行コードを判定して返す（§7.5〜7.6、§8.4）
pub fn read_text_file(path: &Path) -> Result<FileContent, AppError> {
    let size_bytes = std::fs::metadata(path)
        .map_err(|e| io_error(format!("ファイル情報を取得できません: {e}")))?
        .len();
    let read_mode = read_mode_for_size(size_bytes);
    let (content, encoding) = if read_mode == ReadMode::Preview {
        read_preview(path, size_bytes)?
    } else {
        let bytes = std::fs::read(path).map_err(|e| io_error(format!("読み込めません: {e}")))?;
        encoding::detect_and_decode(&bytes)?
    };
    Ok(FileContent {
        line_ending: encoding::detect_line_ending(&content),
        content,
        encoding,
        size_bytes,
        modified_at: modified_at_rfc3339(path)?,
        read_mode,
    })
}

/// 一時ファイル→fsync→置換のアトミック保存（§7.7）。
/// 元の文字コード・改行コードを維持して書き込む。
pub fn write_text_atomic(
    path: &Path,
    content: &str,
    encoding_kind: FileEncoding,
    line_ending: LineEnding,
) -> Result<FileMeta, AppError> {
    use std::io::Write;

    if let Ok(meta) = std::fs::metadata(path) {
        if read_mode_for_size(meta.len()) != ReadMode::Normal {
            return Err(AppError::new(
                "FILE_TOO_LARGE",
                "10MB以上のファイルは読み取り専用のため保存できません",
                false,
            ));
        }
    }

    let normalized = encoding::normalize_line_endings(content, line_ending);
    let bytes = encoding::encode(&normalized, encoding_kind)?;

    let dir = path
        .parent()
        .ok_or_else(|| io_error("保存先の親ディレクトリを特定できません"))?;
    let file_name = path
        .file_name()
        .ok_or_else(|| io_error("保存先ファイル名が不正です"))?
        .to_string_lossy();
    let temp_path = dir.join(format!(".{file_name}.{}.tmp", uuid::Uuid::new_v4()));

    let write_result = (|| -> std::io::Result<()> {
        let mut file = std::fs::File::create(&temp_path)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&temp_path, path)
    })();
    if let Err(e) = write_result {
        let _ = std::fs::remove_file(&temp_path);
        return Err(io_error(format!("保存に失敗しました: {e}")));
    }

    Ok(FileMeta {
        size_bytes: bytes.len() as u64,
        modified_at: modified_at_rfc3339(path)?,
    })
}

/// ファイルまたはフォルダを作成する。既存パスにはFILE_IO_ERRORを返す。
pub fn create_entry(path: &Path, is_folder: bool) -> Result<(), AppError> {
    if path.exists() {
        return Err(io_error(format!("すでに存在します: {}", path.display())));
    }
    if is_folder {
        std::fs::create_dir_all(path).map_err(|e| io_error(format!("フォルダを作成できません: {e}")))
    } else {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| io_error(format!("親フォルダを作成できません: {e}")))?;
        }
        std::fs::File::create_new(path)
            .map(|_| ())
            .map_err(|e| io_error(format!("ファイルを作成できません: {e}")))
    }
}

pub fn rename_entry(old_path: &Path, new_path: &Path) -> Result<(), AppError> {
    if new_path.exists() {
        return Err(io_error(format!("変更先がすでに存在します: {}", new_path.display())));
    }
    std::fs::rename(old_path, new_path).map_err(|e| io_error(format!("名前を変更できません: {e}")))
}

/// 削除。既定はごみ箱を使用する（§20.1）。
pub fn delete_entry(path: &Path, use_recycle_bin: bool) -> Result<(), AppError> {
    if use_recycle_bin {
        trash::delete(path).map_err(|e| io_error(format!("ごみ箱へ移動できません: {e}")))
    } else if path.is_dir() {
        std::fs::remove_dir_all(path).map_err(|e| io_error(format!("削除できません: {e}")))
    } else {
        std::fs::remove_file(path).map_err(|e| io_error(format!("削除できません: {e}")))
    }
}

/// ファイルまたはフォルダ（再帰）をコピーする。
pub fn copy_entry(source: &Path, destination: &Path) -> Result<(), AppError> {
    if destination.exists() {
        return Err(io_error(format!("コピー先がすでに存在します: {}", destination.display())));
    }
    if source.is_dir() {
        std::fs::create_dir_all(destination)
            .map_err(|e| io_error(format!("コピー先を作成できません: {e}")))?;
        for entry in std::fs::read_dir(source).map_err(|e| io_error(format!("読み取れません: {e}")))? {
            let entry = entry.map_err(|e| io_error(format!("読み取れません: {e}")))?;
            copy_entry(&entry.path(), &destination.join(entry.file_name()))?;
        }
        Ok(())
    } else {
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| io_error(format!("コピー先を作成できません: {e}")))?;
        }
        std::fs::copy(source, destination)
            .map(|_| ())
            .map_err(|e| io_error(format!("コピーできません: {e}")))
    }
}

/// プレビュー用にファイルをbase64で返す（画像等・10MB上限）
pub fn read_file_base64(_path: &Path) -> Result<String, AppError> {
    todo!()
}

pub fn move_entry(source: &Path, destination: &Path) -> Result<(), AppError> {
    if destination.exists() {
        return Err(io_error(format!("移動先がすでに存在します: {}", destination.display())));
    }
    if std::fs::rename(source, destination).is_ok() {
        return Ok(());
    }
    copy_entry(source, destination)?;
    delete_entry(source, false)
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
    fn base64でファイルを読める() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("img.png");
        fs::write(&path, [1u8, 2, 3, 255]).unwrap();
        assert_eq!(read_file_base64(&path).unwrap(), "AQID/w==");
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
