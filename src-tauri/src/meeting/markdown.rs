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

pub fn ai_start_marker(meeting_id: &str) -> String {
    format!("<!-- inquivora:meeting:{meeting_id}:ai:start -->")
}

pub fn ai_end_marker(meeting_id: &str) -> String {
    format!("<!-- inquivora:meeting:{meeting_id}:ai:end -->")
}

fn bullet_section(heading: &str, prefix: &str, items: &[String]) -> String {
    let mut out = format!("## {heading}\n\n");
    if items.is_empty() {
        out.push_str("- （なし）\n");
    } else {
        for item in items {
            out.push_str(prefix);
            out.push_str(item.trim());
            out.push('\n');
        }
    }
    out
}

/// §11.4 追記テンプレートのAI議事録ブロックを生成する（AIマーカーで囲む）。
pub fn format_ai_block(
    meeting_id: &str,
    summary: &str,
    decisions: &[String],
    tasks: &[String],
    questions: &[String],
) -> String {
    format!(
        "{}\n\n## AI要約\n\n{}\n\n{}\n{}\n{}\n{}",
        ai_start_marker(meeting_id),
        summary.trim(),
        bullet_section("決定事項", "- ", decisions),
        bullet_section("タスク候補", "- [ ] ", tasks),
        bullet_section("未確認事項", "- ", questions),
        ai_end_marker(meeting_id)
    )
}

/// AI議事録ブロックを終了マーカー直前へ挿入する。既存ブロックがあれば置き換える。
pub fn upsert_ai_block(
    content: &str,
    meeting_id: &str,
    ai_block: &str,
) -> Result<String, AppError> {
    let ai_start = ai_start_marker(meeting_id);
    let ai_end = ai_end_marker(meeting_id);
    if let (Some(start), Some(end)) = (content.find(&ai_start), content.find(&ai_end)) {
        if end > start {
            let end = end + ai_end.len();
            let mut result = content[..start].to_string();
            result.push_str(ai_block.trim_end());
            result.push_str(&content[end..]);
            return Ok(result);
        }
    }
    insert_before_end_marker(content, meeting_id, ai_block)
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

    #[test]
    fn ai議事録ブロックはテンプレート形式で生成される() {
        let block = format_ai_block(
            "m-1",
            "要約本文",
            &["8月から試験導入".to_string()],
            &["見積り作成".to_string()],
            &[],
        );
        assert!(block.starts_with("<!-- inquivora:meeting:m-1:ai:start -->"));
        assert!(block.contains("## AI要約\n\n要約本文"));
        assert!(block.contains("## 決定事項\n\n- 8月から試験導入"));
        assert!(block.contains("## タスク候補\n\n- [ ] 見積り作成"));
        assert!(block.contains("## 未確認事項\n\n- （なし）"));
        assert!(block.trim_end().ends_with("<!-- inquivora:meeting:m-1:ai:end -->"));
    }

    #[test]
    fn ai議事録ブロックは終了マーカー前に挿入される() {
        let content = format!(
            "# メモ\n\n{}\n\n## 文字起こし\n\n{}\n",
            start_marker("m-1"),
            end_marker("m-1")
        );
        let block = format_ai_block("m-1", "要約", &[], &[], &[]);
        let updated = upsert_ai_block(&content, "m-1", &block).unwrap();
        let ai_pos = updated.find("## AI要約").unwrap();
        let end_pos = updated.find(&end_marker("m-1")).unwrap();
        assert!(ai_pos < end_pos);
    }

    #[test]
    fn ai議事録ブロックは再生成で置き換えられ二重化しない() {
        let content = format!(
            "# メモ\n\n{}\n\n## 文字起こし\n\n{}\n",
            start_marker("m-1"),
            end_marker("m-1")
        );
        let first = upsert_ai_block(&content, "m-1", &format_ai_block("m-1", "旧要約", &[], &[], &[])).unwrap();
        let second = upsert_ai_block(&first, "m-1", &format_ai_block("m-1", "新要約", &[], &[], &[])).unwrap();
        assert_eq!(second.matches("## AI要約").count(), 1);
        assert!(second.contains("新要約"));
        assert!(!second.contains("旧要約"));
    }
}
