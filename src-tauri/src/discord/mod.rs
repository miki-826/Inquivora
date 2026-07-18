use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::error::AppError;

/// Webhook URLはAPIキーと同様にCredential Managerへ保存する（§10.4）
pub const WEBHOOK_CREDENTIAL_ID: &str = "discord-webhook";
pub const SETTINGS_KEY: &str = "discord";

pub const EMBED_COLOR: u32 = 0x00bfff;
pub const DESCRIPTION_LIMIT: usize = 4096;
pub const MAX_EMBEDS_PER_POST: usize = 10;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct DiscordSettings {
    pub enabled: bool,
    pub realtime: bool,
    pub summary: bool,
}

impl Default for DiscordSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            realtime: false,
            summary: true,
        }
    }
}

pub fn load_settings(conn: &Connection) -> Result<DiscordSettings, AppError> {
    let stored = crate::database::settings::get_setting(conn, SETTINGS_KEY)?;
    Ok(stored
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_default())
}

pub fn validate_webhook_url(url: &str) -> Result<String, AppError> {
    let trimmed = url.trim();
    let allowed = ["https://discord.com/api/webhooks/", "https://discordapp.com/api/webhooks/"];
    if allowed.iter().any(|prefix| trimmed.starts_with(prefix)) {
        return Ok(trimmed.to_string());
    }
    Err(AppError::new(
        "VALIDATION_ERROR",
        "Discord Webhook URL（https://discord.com/api/webhooks/…）を入力してください",
        false,
    ))
}

pub fn segment_embed(
    meeting_title: &str,
    time_label: &str,
    speaker: &str,
    text: &str,
) -> serde_json::Value {
    serde_json::json!({
        "author": { "name": format!("{time_label} {speaker}") },
        "description": text,
        "footer": { "text": meeting_title },
        "color": EMBED_COLOR,
    })
}

/// 上限文字数で分割する。改行位置を優先し、なければ文字数で切る。
pub fn split_description(text: &str, limit: usize) -> Vec<String> {
    let mut parts = Vec::new();
    let mut rest = text.trim();
    while !rest.is_empty() {
        if rest.chars().count() <= limit {
            parts.push(rest.to_string());
            break;
        }
        let byte_limit = rest
            .char_indices()
            .nth(limit)
            .map(|(index, _)| index)
            .unwrap_or(rest.len());
        let cut = rest[..byte_limit].rfind('\n').unwrap_or(byte_limit);
        let (head, tail) = rest.split_at(cut);
        parts.push(head.trim_end_matches('\n').to_string());
        rest = tail.trim_start_matches('\n').trim_start();
    }
    parts
}

pub fn summary_embeds(meeting_title: &str, full_text: &str) -> Vec<serde_json::Value> {
    let parts = split_description(full_text, DESCRIPTION_LIMIT);
    let total = parts.len();
    parts
        .into_iter()
        .enumerate()
        .map(|(index, description)| {
            let title = if total > 1 {
                format!("{meeting_title} の文字起こし ({}/{})", index + 1, total)
            } else {
                format!("{meeting_title} の文字起こし")
            };
            serde_json::json!({
                "title": title,
                "description": description,
                "color": EMBED_COLOR,
            })
        })
        .collect()
}

/// WebhookへEmbedを投稿する。1回の投稿は最大10 Embedまで。
pub async fn post_webhook(url: &str, embeds: &[serde_json::Value]) -> Result<(), AppError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| AppError::new("DISCORD_POST_FAILED", format!("HTTPクライアント初期化に失敗: {e}"), true))?;
    for chunk in embeds.chunks(MAX_EMBEDS_PER_POST) {
        let response = client
            .post(url)
            .json(&serde_json::json!({ "embeds": chunk }))
            .send()
            .await
            .map_err(|e| AppError::new("DISCORD_POST_FAILED", format!("Discordへの投稿に失敗: {e}"), true))?;
        if !response.status().is_success() {
            return Err(AppError::new(
                "DISCORD_POST_FAILED",
                format!("Discordへの投稿に失敗しました (HTTP {})", response.status().as_u16()),
                true,
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::open_database;
    use crate::database::settings::set_setting;
    use serde_json::json;

    fn temp_conn() -> (tempfile::TempDir, Connection) {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_database(&dir.path().join("test.db")).unwrap();
        (dir, conn)
    }

    #[test]
    fn 設定未保存なら既定値() {
        let (_dir, conn) = temp_conn();
        let settings = load_settings(&conn).unwrap();
        assert_eq!(settings, DiscordSettings::default());
        assert!(!settings.enabled);
        assert!(settings.summary);
    }

    #[test]
    fn 保存済み設定を読み込める() {
        let (_dir, conn) = temp_conn();
        set_setting(
            &conn,
            SETTINGS_KEY,
            &json!({ "enabled": true, "realtime": true, "summary": false }),
        )
        .unwrap();
        let settings = load_settings(&conn).unwrap();
        assert!(settings.enabled);
        assert!(settings.realtime);
        assert!(!settings.summary);
    }

    #[test]
    fn webhook_urlはdiscordのwebhookのみ許可() {
        let url = "https://discord.com/api/webhooks/123/token-abc";
        assert_eq!(validate_webhook_url(url).unwrap(), url);
        assert_eq!(
            validate_webhook_url("  https://discordapp.com/api/webhooks/1/t  ").unwrap(),
            "https://discordapp.com/api/webhooks/1/t"
        );
        assert_eq!(
            validate_webhook_url("https://example.com/api/webhooks/1/t").unwrap_err().code,
            "VALIDATION_ERROR"
        );
        assert_eq!(
            validate_webhook_url("http://discord.com/api/webhooks/1/t").unwrap_err().code,
            "VALIDATION_ERROR"
        );
        assert_eq!(validate_webhook_url("").unwrap_err().code, "VALIDATION_ERROR");
    }

    #[test]
    fn セグメントembedは話者と時刻をヘッダーにする() {
        let embed = segment_embed("週次定例", "10:05", "自分", "こんにちは");
        assert_eq!(embed["author"]["name"], "10:05 自分");
        assert_eq!(embed["description"], "こんにちは");
        assert_eq!(embed["footer"]["text"], "週次定例");
        assert_eq!(embed["color"], EMBED_COLOR);
    }

    #[test]
    fn 説明文は上限で分割される() {
        let text = "あ".repeat(10);
        let parts = split_description(&text, 4);
        assert_eq!(parts, vec!["ああああ", "ああああ", "ああ"]);
    }

    #[test]
    fn 説明文の分割は改行を優先する() {
        let text = "1行目\n2行目\n3行目";
        let parts = split_description(text, 8);
        assert_eq!(parts, vec!["1行目\n2行目", "3行目"]);
    }

    #[test]
    fn まとめembedはタイトルとページ番号を持つ() {
        let text = "あ".repeat(DESCRIPTION_LIMIT + 10);
        let embeds = summary_embeds("週次定例", &text);
        assert_eq!(embeds.len(), 2);
        assert_eq!(embeds[0]["title"], "週次定例 の文字起こし (1/2)");
        assert_eq!(embeds[1]["title"], "週次定例 の文字起こし (2/2)");
        assert_eq!(embeds[0]["color"], EMBED_COLOR);
    }

    #[test]
    fn 短いまとめはページ番号なし() {
        let embeds = summary_embeds("週次定例", "短い本文");
        assert_eq!(embeds.len(), 1);
        assert_eq!(embeds[0]["title"], "週次定例 の文字起こし");
        assert_eq!(embeds[0]["description"], "短い本文");
    }
}
