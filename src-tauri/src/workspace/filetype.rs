use serde::Serialize;

/// ファイル種別（§7.4）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FileCategory {
    Edit,
    Preview,
    External,
    Unknown,
}

const EDIT_EXTENSIONS: &[&str] = &[
    "md", "txt", "log", "csv", "json", "jsonl", "yaml", "yml", "xml", "ini", "conf", "env",
    "html", "htm", "css", "scss", "js", "jsx", "ts", "tsx", "py", "ps1", "bat", "sh", "sql", "rs",
    "cs", "java",
];

const PREVIEW_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "svg", "pdf", "wav", "mp3", "m4a", "mp4", "webm",
];

const EXTERNAL_EXTENSIONS: &[&str] = &["docx", "xlsx", "pptx", "dwg", "zip", "7z", "exe", "dll"];

/// 拡張子（ドットなし）から種別を判定する（§7.4）
pub fn category_for_extension(extension: &str) -> FileCategory {
    let ext = extension.to_lowercase();
    if EDIT_EXTENSIONS.contains(&ext.as_str()) {
        FileCategory::Edit
    } else if PREVIEW_EXTENSIONS.contains(&ext.as_str()) {
        FileCategory::Preview
    } else if EXTERNAL_EXTENSIONS.contains(&ext.as_str()) {
        FileCategory::External
    } else {
        FileCategory::Unknown
    }
}

/// 先頭サンプル（最大8KB）からテキストらしさを判定する（§7.5）
pub fn is_probably_text(sample: &[u8]) -> bool {
    let sample = &sample[..sample.len().min(8192)];
    if sample.is_empty() {
        return true;
    }
    if sample.contains(&0) {
        return false;
    }
    let control_count = sample
        .iter()
        .filter(|&&b| b < 0x20 && b != b'\t' && b != b'\n' && b != b'\r' && b != 0x0C)
        .count();
    if control_count * 10 > sample.len() {
        return false;
    }
    if std::str::from_utf8(sample).is_ok() {
        return true;
    }
    let (_, _, had_errors) = encoding_rs::SHIFT_JIS.decode(sample);
    !had_errors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 編集対象拡張子を判定する() {
        for ext in ["md", "txt", "json", "ts", "rs", "sql", "env", "yml", "html", "htm"] {
            assert_eq!(category_for_extension(ext), FileCategory::Edit, "{ext}");
        }
    }

    #[test]
    fn プレビュー対象拡張子を判定する() {
        for ext in ["png", "jpg", "svg", "pdf", "wav", "mp4"] {
            assert_eq!(category_for_extension(ext), FileCategory::Preview, "{ext}");
        }
    }

    #[test]
    fn 外部アプリ対象拡張子を判定する() {
        for ext in ["docx", "xlsx", "zip", "exe", "dll"] {
            assert_eq!(category_for_extension(ext), FileCategory::External, "{ext}");
        }
    }

    #[test]
    fn 大文字拡張子も判定する() {
        assert_eq!(category_for_extension("MD"), FileCategory::Edit);
        assert_eq!(category_for_extension("PNG"), FileCategory::Preview);
    }

    #[test]
    fn 未知拡張子はunknownを返す() {
        assert_eq!(category_for_extension("xyz"), FileCategory::Unknown);
        assert_eq!(category_for_extension(""), FileCategory::Unknown);
    }

    #[test]
    fn nulバイトを含むものはバイナリ() {
        assert!(!is_probably_text(b"hello\x00world"));
    }

    #[test]
    fn utf8日本語はテキスト() {
        assert!(is_probably_text("これはテキストファイルです。".as_bytes()));
    }

    #[test]
    fn shift_jisはテキスト() {
        let (bytes, _, _) = encoding_rs::SHIFT_JIS.encode("こんにちは、世界。改行もあります。\r\n");
        assert!(is_probably_text(&bytes));
    }

    #[test]
    fn 制御文字が多いものはバイナリ() {
        let mut data = Vec::new();
        for i in 0..1024u32 {
            data.push((i % 32) as u8);
        }
        assert!(!is_probably_text(&data));
    }

    #[test]
    fn 空データはテキスト扱い() {
        assert!(is_probably_text(b""));
    }
}
