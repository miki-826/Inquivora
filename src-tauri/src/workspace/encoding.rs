use serde::{Deserialize, Serialize};

use crate::error::AppError;

/// 対応文字コード（§7.6）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileEncoding {
    #[serde(rename = "utf8")]
    Utf8,
    #[serde(rename = "utf8-bom")]
    Utf8Bom,
    #[serde(rename = "utf16le")]
    Utf16Le,
    #[serde(rename = "utf16be")]
    Utf16Be,
    #[serde(rename = "shift_jis")]
    ShiftJis,
}

/// 改行コード（§8.2）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineEnding {
    #[serde(rename = "LF")]
    Lf,
    #[serde(rename = "CRLF")]
    Crlf,
}

/// バイト列の文字コードを判定してデコードする（§7.6）。
/// 非対応の場合は FILE_ENCODING_UNSUPPORTED を返す。
pub fn detect_and_decode(_bytes: &[u8]) -> Result<(String, FileEncoding), AppError> {
    todo!()
}

/// テキストの改行コードを判定する。最初に現れた改行を採用し、改行なしはLFとする。
pub fn detect_line_ending(_text: &str) -> LineEnding {
    todo!()
}

/// テキストを指定文字コードでエンコードする。BOM付きはBOMを先頭へ付与する。
pub fn encode(_text: &str, _encoding: FileEncoding) -> Result<Vec<u8>, AppError> {
    todo!()
}

/// 改行コードを指定へ統一する。
pub fn normalize_line_endings(_text: &str, _line_ending: LineEnding) -> String {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf8日本語を判定できる() {
        let (text, enc) = detect_and_decode("こんにちは世界".as_bytes()).unwrap();
        assert_eq!(text, "こんにちは世界");
        assert_eq!(enc, FileEncoding::Utf8);
    }

    #[test]
    fn utf8_bomを判定しbomは本文に含めない() {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice("メモ".as_bytes());
        let (text, enc) = detect_and_decode(&bytes).unwrap();
        assert_eq!(text, "メモ");
        assert_eq!(enc, FileEncoding::Utf8Bom);
    }

    #[test]
    fn utf16leを判定できる() {
        let mut bytes = vec![0xFF, 0xFE];
        for unit in "あA".encode_utf16() {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }
        let (text, enc) = detect_and_decode(&bytes).unwrap();
        assert_eq!(text, "あA");
        assert_eq!(enc, FileEncoding::Utf16Le);
    }

    #[test]
    fn utf16beを判定できる() {
        let mut bytes = vec![0xFE, 0xFF];
        for unit in "あA".encode_utf16() {
            bytes.extend_from_slice(&unit.to_be_bytes());
        }
        let (text, enc) = detect_and_decode(&bytes).unwrap();
        assert_eq!(text, "あA");
        assert_eq!(enc, FileEncoding::Utf16Be);
    }

    #[test]
    fn shift_jisを判定できる() {
        let (bytes, _, _) = encoding_rs::SHIFT_JIS.encode("日本語のテスト文章です。漢字とひらがなを含みます。");
        let (text, enc) = detect_and_decode(&bytes).unwrap();
        assert_eq!(text, "日本語のテスト文章です。漢字とひらがなを含みます。");
        assert_eq!(enc, FileEncoding::ShiftJis);
    }

    #[test]
    fn 空バイト列はutf8扱い() {
        let (text, enc) = detect_and_decode(b"").unwrap();
        assert_eq!(text, "");
        assert_eq!(enc, FileEncoding::Utf8);
    }

    #[test]
    fn 全エンコーディングでラウンドトリップできる() {
        let original = "テスト text 123\r\n2行目";
        for enc in [
            FileEncoding::Utf8,
            FileEncoding::Utf8Bom,
            FileEncoding::Utf16Le,
            FileEncoding::Utf16Be,
            FileEncoding::ShiftJis,
        ] {
            let bytes = encode(original, enc).unwrap();
            let (decoded, detected) = detect_and_decode(&bytes).unwrap();
            assert_eq!(decoded, original, "{enc:?}");
            assert_eq!(detected, enc, "{enc:?}");
        }
    }

    #[test]
    fn 改行コードを判定する() {
        assert_eq!(detect_line_ending("a\r\nb"), LineEnding::Crlf);
        assert_eq!(detect_line_ending("a\nb"), LineEnding::Lf);
        assert_eq!(detect_line_ending("改行なし"), LineEnding::Lf);
        assert_eq!(detect_line_ending("a\nb\r\nc"), LineEnding::Lf);
    }

    #[test]
    fn 改行コードを統一できる() {
        assert_eq!(normalize_line_endings("a\nb\r\nc", LineEnding::Crlf), "a\r\nb\r\nc");
        assert_eq!(normalize_line_endings("a\r\nb\nc", LineEnding::Lf), "a\nb\nc");
    }

    #[test]
    fn serdeの表現が仕様どおり() {
        assert_eq!(serde_json::to_string(&FileEncoding::Utf8Bom).unwrap(), "\"utf8-bom\"");
        assert_eq!(serde_json::to_string(&FileEncoding::ShiftJis).unwrap(), "\"shift_jis\"");
        assert_eq!(serde_json::to_string(&LineEnding::Crlf).unwrap(), "\"CRLF\"");
    }
}
