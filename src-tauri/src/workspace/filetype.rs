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

/// 拡張子（小文字・ドットなし）から種別を判定する（§7.4）
pub fn category_for_extension(_extension: &str) -> FileCategory {
    todo!()
}

/// 先頭サンプル（最大8KB）からテキストらしさを判定する（§7.5）
pub fn is_probably_text(_sample: &[u8]) -> bool {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 編集対象拡張子を判定する() {
        for ext in ["md", "txt", "json", "ts", "rs", "sql", "env", "yml"] {
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
