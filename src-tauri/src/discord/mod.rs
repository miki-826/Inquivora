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

pub fn load_settings(_conn: &Connection) -> Result<DiscordSettings, AppError> {
    todo!()
}

pub fn validate_webhook_url(_url: &str) -> Result<String, AppError> {
    todo!()
}

pub fn segment_embed(
    _meeting_title: &str,
    _time_label: &str,
    _speaker: &str,
    _text: &str,
) -> serde_json::Value {
    todo!()
}

pub fn split_description(_text: &str, _limit: usize) -> Vec<String> {
    todo!()
}

pub fn summary_embeds(_meeting_title: &str, _full_text: &str) -> Vec<serde_json::Value> {
    todo!()
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
