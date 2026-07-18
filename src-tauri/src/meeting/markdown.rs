use chrono::{DateTime, Duration};
use chrono_tz::Asia::Tokyo;

use crate::error::AppError;

pub fn start_marker(meeting_id: &str) -> String {
    format!("<!-- inquivora:meeting:{meeting_id}:start -->")
}

pub fn end_marker(meeting_id: &str) -> String {
    format!("<!-- inquivora:meeting:{meeting_id}:end -->")
}

pub fn initial_block(meeting_id: &str) -> String {
    format!(
        "{}\n\n## 文字起こし\n\n{}\n",
        start_marker(meeting_id),
        end_marker(meeting_id)
    )
}

pub fn format_segment(time_label: &str, speaker_label: &str, text: &str) -> String {
    format!("### {time_label} {speaker_label}\n\n{text}\n")
}

pub fn speaker_label_for_source(source: &str) -> &'static str {
    if source == "mic" {
        "自分"
    } else {
        "PC音声"
    }
}

pub fn segment_time_label(started_at_utc: &str, offset_ms: i64) -> String {
    match DateTime::parse_from_rfc3339(started_at_utc) {
        Ok(started) => (started + Duration::milliseconds(offset_ms))
            .with_timezone(&Tokyo)
            .format("%H:%M")
            .to_string(),
        Err(_) => "--:--".to_string(),
    }
}

/// 終了マーカー直前へセグメントを挿入する。マーカー前後の空行を維持する。
pub fn insert_before_end_marker(
    content: &str,
    meeting_id: &str,
    segment_md: &str,
) -> Result<String, AppError> {
    let marker = end_marker(meeting_id);
    let pos = content.find(&marker).ok_or_else(|| {
        AppError::new(
            "FILE_CONFLICT",
            "会議の終了マーカーが見つかりません。ファイルが外部で変更された可能性があります",
            false,
        )
    })?;
    let mut result = content[..pos].to_string();
    while !result.ends_with("\n\n") {
        result.push('\n');
    }
    result.push_str(segment_md.trim_end());
    result.push_str("\n\n");
    result.push_str(&content[pos..]);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn マーカーは仕様書の形式で生成される() {
        assert_eq!(
            start_marker("m-1"),
            "<!-- inquivora:meeting:m-1:start -->"
        );
        assert_eq!(end_marker("m-1"), "<!-- inquivora:meeting:m-1:end -->");
    }

    #[test]
    fn 初期ブロックはマーカーと文字起こし見出しを含む() {
        let block = initial_block("m-1");
        assert!(block.starts_with("<!-- inquivora:meeting:m-1:start -->"));
        assert!(block.contains("## 文字起こし"));
        assert!(block.trim_end().ends_with("<!-- inquivora:meeting:m-1:end -->"));
    }

    #[test]
    fn セグメントは時刻と話者付きで整形される() {
        assert_eq!(
            format_segment("10:03", "PC音声", "8月から試験導入を開始します。"),
            "### 10:03 PC音声\n\n8月から試験導入を開始します。\n"
        );
    }

    #[test]
    fn 音源から話者表記を決定する() {
        assert_eq!(speaker_label_for_source("mic"), "自分");
        assert_eq!(speaker_label_for_source("loopback"), "PC音声");
        assert_eq!(speaker_label_for_source("system"), "PC音声");
    }

    #[test]
    fn 発言時刻は会議開始時刻と経過msから東京時刻になる() {
        assert_eq!(
            segment_time_label("2026-07-17T01:00:00Z", 180_000),
            "10:03"
        );
    }

    #[test]
    fn 終了マーカー直前へ挿入される() {
        let content = format!(
            "# メモ\n\n{}\n\n## 文字起こし\n\n### 10:02 自分\n\n最初の発言\n\n{}\n\n後続の本文\n",
            start_marker("m-1"),
            end_marker("m-1")
        );
        let inserted = insert_before_end_marker(
            &content,
            "m-1",
            "### 10:03 PC音声\n\n次の発言\n",
        )
        .unwrap();
        let end_pos = inserted.find(&end_marker("m-1")).unwrap();
        let new_pos = inserted.find("次の発言").unwrap();
        assert!(new_pos < end_pos);
        assert!(inserted.contains("最初の発言"));
        assert!(inserted.ends_with("後続の本文\n"));
        assert!(
            inserted.contains("次の発言\n\n<!-- inquivora:meeting:m-1:end -->"),
            "挿入後もマーカー前に空行が保たれる: {inserted}"
        );
    }

    #[test]
    fn 終了マーカーがない場合はエラーになる() {
        let err = insert_before_end_marker("# メモ\n", "m-1", "### x\n").unwrap_err();
        assert_eq!(err.code, "FILE_CONFLICT");
    }
}
