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

fn unsupported(detail: impl Into<String>) -> AppError {
    AppError::new("FILE_ENCODING_UNSUPPORTED", detail, false)
}

const UTF8_BOM: &[u8] = &[0xEF, 0xBB, 0xBF];

fn decode_utf16(bytes: &[u8], le: bool) -> Result<String, AppError> {
    if !bytes.len().is_multiple_of(2) {
        return Err(unsupported("UTF-16のバイト長が不正です"));
    }
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|pair| {
            if le {
                u16::from_le_bytes([pair[0], pair[1]])
            } else {
                u16::from_be_bytes([pair[0], pair[1]])
            }
        })
        .collect();
    String::from_utf16(&units).map_err(|_| unsupported("UTF-16として不正なデータです"))
}

/// バイト列の文字コードを判定してデコードする（§7.6）。
/// 非対応の場合は FILE_ENCODING_UNSUPPORTED を返す。
pub fn detect_and_decode(bytes: &[u8]) -> Result<(String, FileEncoding), AppError> {
    if bytes.starts_with(UTF8_BOM) {
        let text = std::str::from_utf8(&bytes[3..])
            .map_err(|_| unsupported("UTF-8 BOM付きファイルの本文が不正です"))?;
        return Ok((text.to_string(), FileEncoding::Utf8Bom));
    }
    if bytes.starts_with(&[0xFF, 0xFE]) {
        return Ok((decode_utf16(&bytes[2..], true)?, FileEncoding::Utf16Le));
    }
    if bytes.starts_with(&[0xFE, 0xFF]) {
        return Ok((decode_utf16(&bytes[2..], false)?, FileEncoding::Utf16Be));
    }
    if let Ok(text) = std::str::from_utf8(bytes) {
        return Ok((text.to_string(), FileEncoding::Utf8));
    }
    let (text, _, had_errors) = encoding_rs::SHIFT_JIS.decode(bytes);
    if !had_errors {
        return Ok((text.into_owned(), FileEncoding::ShiftJis));
    }
    Err(unsupported("対応していない文字コードです（UTF-8 / UTF-16 / Shift_JIS のみ対応）"))
}

/// テキストの改行コードを判定する。最初に現れた改行を採用し、改行なしはLFとする。
pub fn detect_line_ending(text: &str) -> LineEnding {
    match text.find('\n') {
        Some(pos) if pos > 0 && text.as_bytes()[pos - 1] == b'\r' => LineEnding::Crlf,
        _ => LineEnding::Lf,
    }
}

/// テキストを指定文字コードでエンコードする。BOM付きはBOMを先頭へ付与する。
pub fn encode(text: &str, encoding: FileEncoding) -> Result<Vec<u8>, AppError> {
    match encoding {
        FileEncoding::Utf8 => Ok(text.as_bytes().to_vec()),
        FileEncoding::Utf8Bom => {
            let mut bytes = UTF8_BOM.to_vec();
            bytes.extend_from_slice(text.as_bytes());
            Ok(bytes)
        }
        FileEncoding::Utf16Le => {
            let mut bytes = vec![0xFF, 0xFE];
            for unit in text.encode_utf16() {
                bytes.extend_from_slice(&unit.to_le_bytes());
            }
            Ok(bytes)
        }
        FileEncoding::Utf16Be => {
            let mut bytes = vec![0xFE, 0xFF];
            for unit in text.encode_utf16() {
                bytes.extend_from_slice(&unit.to_be_bytes());
            }
            Ok(bytes)
        }
        FileEncoding::ShiftJis => {
            let (bytes, _, had_errors) = encoding_rs::SHIFT_JIS.encode(text);
            if had_errors {
                return Err(unsupported("Shift_JISで表現できない文字が含まれています"));
            }
            Ok(bytes.into_owned())
        }
    }
}

/// 改行コードを指定へ統一する。
pub fn normalize_line_endings(text: &str, line_ending: LineEnding) -> String {
    let lf = text.replace("\r\n", "\n");
    match line_ending {
        LineEnding::Lf => lf,
        LineEnding::Crlf => lf.replace('\n', "\r\n"),
    }
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
