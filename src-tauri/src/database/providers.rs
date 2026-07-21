use std::collections::HashMap;

use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::database::double_option;
use crate::error::AppError;

pub const FEATURE_KEYS: &[&str] = &[
    "transcription.batch",
    "transcription.realtime",
    "meeting.summary",
    "editor.ai",
];

const PROTECTED_HEADERS: &[&str] = &[
    "authorization",
    "x-api-key",
    "proxy-authorization",
    "cookie",
    "openai-organization",
    "openai-project",
];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiProviderProfile {
    pub id: String,
    pub display_name: String,
    pub provider_type: String,
    pub base_url: String,
    pub auth_type: String,
    pub organization_id: Option<String>,
    pub project_id: Option<String>,
    pub model_id: Option<String>,
    pub custom_prompt: Option<String>,
    pub default_headers: HashMap<String, String>,
    pub timeout_ms: i64,
    pub capabilities: Vec<String>,
    pub enabled: bool,
    pub last_test_status: Option<String>,
    pub last_tested_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

fn default_timeout_ms() -> i64 {
    60000
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInput {
    pub display_name: String,
    pub provider_type: String,
    pub base_url: String,
    pub auth_type: String,
    #[serde(default)]
    pub organization_id: Option<String>,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub model_id: Option<String>,
    #[serde(default)]
    pub custom_prompt: Option<String>,
    #[serde(default)]
    pub default_headers: HashMap<String, String>,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: i64,
    #[serde(default)]
    pub capabilities: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ProviderPatch {
    pub display_name: Option<String>,
    pub provider_type: Option<String>,
    pub base_url: Option<String>,
    pub auth_type: Option<String>,
    #[serde(deserialize_with = "double_option")]
    pub organization_id: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub project_id: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub model_id: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub custom_prompt: Option<Option<String>>,
    pub default_headers: Option<HashMap<String, String>>,
    pub timeout_ms: Option<i64>,
    pub capabilities: Option<Vec<String>>,
}

fn validation_error(message: impl Into<String>) -> AppError {
    AppError::new("VALIDATION_ERROR", message, false)
}

/// §10.3 base_url制約: https必須（ローカルホストのみhttp許可）・末尾スラッシュ正規化。
pub fn normalize_base_url(raw: &str) -> Result<String, AppError> {
    let trimmed = raw.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err(validation_error("Base URLを入力してください"));
    }
    let parsed = reqwest::Url::parse(trimmed)
        .map_err(|_| validation_error("Base URLの形式が正しくありません"))?;
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(validation_error(
            "Base URLにユーザー名やパスワードは指定できません",
        ));
    }
    let host = parsed
        .host_str()
        .ok_or_else(|| validation_error("Base URLのホストがありません"))?;
    let host_ip = host.trim_start_matches('[').trim_end_matches(']');
    let loopback = host.eq_ignore_ascii_case("localhost")
        || host_ip
            .parse::<std::net::IpAddr>()
            .map(|ip| ip.is_loopback())
            .unwrap_or(false);
    match parsed.scheme() {
        "https" => Ok(trimmed.to_string()),
        "http" if loopback => Ok(trimmed.to_string()),
        "http" => Err(validation_error(
            "平文HTTPはローカルホスト（localhost / 127.0.0.1 / [::1]）のみ許可されます",
        )),
        _ => Err(validation_error(
            "Base URLはhttps://（ローカルAPIはhttp://localhost等）で指定してください",
        )),
    }
}

fn validate_headers(headers: &HashMap<String, String>) -> Result<(), AppError> {
    for name in headers.keys() {
        if PROTECTED_HEADERS.contains(&name.to_ascii_lowercase().as_str()) {
            return Err(validation_error(format!(
                "カスタムヘッダーへ認証情報（{name}）は保存できません。APIキー欄を使用してください"
            )));
        }
    }
    Ok(())
}

fn map_unique_violation(err: rusqlite::Error) -> AppError {
    if let rusqlite::Error::SqliteFailure(e, _) = &err {
        if e.code == rusqlite::ErrorCode::ConstraintViolation {
            return validation_error("同じ表示名のProviderが既に存在します");
        }
    }
    err.into()
}

fn row_to_profile(row: &Row) -> rusqlite::Result<ApiProviderProfile> {
    let headers_json: String = row.get("default_headers_json")?;
    let capabilities_json: String = row.get("capabilities_json")?;
    Ok(ApiProviderProfile {
        id: row.get("id")?,
        display_name: row.get("display_name")?,
        provider_type: row.get("provider_type")?,
        base_url: row.get("base_url")?,
        auth_type: row.get("auth_type")?,
        organization_id: row.get("organization_id")?,
        project_id: row.get("project_id")?,
        model_id: row.get("model_id")?,
        custom_prompt: row.get("custom_prompt")?,
        default_headers: serde_json::from_str(&headers_json).unwrap_or_default(),
        timeout_ms: row.get("timeout_ms")?,
        capabilities: serde_json::from_str(&capabilities_json).unwrap_or_default(),
        enabled: row.get::<_, i64>("enabled")? != 0,
        last_test_status: row.get("last_test_status")?,
        last_tested_at: row.get("last_tested_at")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

const PROFILE_COLUMNS: &str = "id, display_name, provider_type, base_url, auth_type, credential_target, \
     organization_id, project_id, model_id, custom_prompt, default_headers_json, timeout_ms, capabilities_json, enabled, \
     last_test_status, last_tested_at, created_at, updated_at";

pub fn create_provider(conn: &Connection, input: ProviderInput) -> Result<ApiProviderProfile, AppError> {
    let base_url = normalize_base_url(&input.base_url)?;
    validate_headers(&input.default_headers)?;
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO api_provider_profiles
            (id, display_name, provider_type, base_url, auth_type, credential_target,
             organization_id, project_id, model_id, custom_prompt, default_headers_json, timeout_ms, capabilities_json,
             enabled, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, 1, ?14, ?14)",
        rusqlite::params![
            id,
            input.display_name,
            input.provider_type,
            base_url,
            input.auth_type,
            crate::api::credentials::credential_target(&id),
            input.organization_id,
            input.project_id,
            input.model_id,
            input.custom_prompt,
            serde_json::to_string(&input.default_headers).unwrap_or_else(|_| "{}".to_string()),
            input.timeout_ms,
            serde_json::to_string(&input.capabilities).unwrap_or_else(|_| "[]".to_string()),
            now,
        ],
    )
    .map_err(map_unique_violation)?;
    get_provider(conn, &id)
}

pub fn get_provider(conn: &Connection, id: &str) -> Result<ApiProviderProfile, AppError> {
    conn.query_row(
        &format!("SELECT {PROFILE_COLUMNS} FROM api_provider_profiles WHERE id = ?1"),
        [id],
        row_to_profile,
    )
    .optional()?
    .ok_or_else(|| AppError::new("API_PROVIDER_NOT_FOUND", "Providerが見つかりません", false))
}

pub fn list_providers(conn: &Connection) -> Result<Vec<ApiProviderProfile>, AppError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {PROFILE_COLUMNS} FROM api_provider_profiles ORDER BY display_name"
    ))?;
    let rows = stmt.query_map([], row_to_profile)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn update_provider(
    conn: &Connection,
    id: &str,
    patch: ProviderPatch,
) -> Result<ApiProviderProfile, AppError> {
    let current = get_provider(conn, id)?;
    let base_url = match patch.base_url {
        Some(raw) => normalize_base_url(&raw)?,
        None => current.base_url,
    };
    let default_headers = patch.default_headers.unwrap_or(current.default_headers);
    validate_headers(&default_headers)?;
    let organization_id = patch.organization_id.unwrap_or(current.organization_id);
    let project_id = patch.project_id.unwrap_or(current.project_id);
    let model_id = patch.model_id.unwrap_or(current.model_id);
    let custom_prompt = patch.custom_prompt.unwrap_or(current.custom_prompt);
    conn.execute(
        "UPDATE api_provider_profiles SET
            display_name = ?2, provider_type = ?3, base_url = ?4, auth_type = ?5,
            organization_id = ?6, project_id = ?7, default_headers_json = ?8,
            timeout_ms = ?9, capabilities_json = ?10, updated_at = ?11,
            model_id = ?12, custom_prompt = ?13
         WHERE id = ?1",
        rusqlite::params![
            id,
            patch.display_name.unwrap_or(current.display_name),
            patch.provider_type.unwrap_or(current.provider_type),
            base_url,
            patch.auth_type.unwrap_or(current.auth_type),
            organization_id,
            project_id,
            serde_json::to_string(&default_headers).unwrap_or_else(|_| "{}".to_string()),
            patch.timeout_ms.unwrap_or(current.timeout_ms),
            serde_json::to_string(&patch.capabilities.unwrap_or(current.capabilities))
                .unwrap_or_else(|_| "[]".to_string()),
            Utc::now().to_rfc3339(),
            model_id,
            custom_prompt,
        ],
    )
    .map_err(map_unique_violation)?;
    get_provider(conn, id)
}

pub fn delete_provider(conn: &Connection, id: &str) -> Result<(), AppError> {
    let affected = conn.execute("DELETE FROM api_provider_profiles WHERE id = ?1", [id])?;
    if affected == 0 {
        return Err(AppError::new(
            "API_PROVIDER_NOT_FOUND",
            "Providerが見つかりません",
            false,
        ));
    }
    Ok(())
}

pub fn set_provider_enabled(conn: &Connection, id: &str, enabled: bool) -> Result<(), AppError> {
    let affected = conn.execute(
        "UPDATE api_provider_profiles SET enabled = ?2, updated_at = ?3 WHERE id = ?1",
        rusqlite::params![id, enabled as i64, Utc::now().to_rfc3339()],
    )?;
    if affected == 0 {
        return Err(AppError::new(
            "API_PROVIDER_NOT_FOUND",
            "Providerが見つかりません",
            false,
        ));
    }
    Ok(())
}

pub fn record_test_result(conn: &Connection, id: &str, status: &str) -> Result<(), AppError> {
    conn.execute(
        "UPDATE api_provider_profiles SET last_test_status = ?2, last_tested_at = ?3 WHERE id = ?1",
        rusqlite::params![id, status, Utc::now().to_rfc3339()],
    )?;
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureBinding {
    pub feature_key: String,
    pub provider_profile_id: Option<String>,
    pub model_id: Option<String>,
    pub fallback_provider_profile_id: Option<String>,
    pub fallback_model_id: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BindingInput {
    pub provider_profile_id: Option<String>,
    pub model_id: Option<String>,
    pub fallback_provider_profile_id: Option<String>,
    pub fallback_model_id: Option<String>,
}

pub fn set_binding(conn: &Connection, feature_key: &str, input: BindingInput) -> Result<(), AppError> {
    if !FEATURE_KEYS.contains(&feature_key) {
        return Err(validation_error(format!("不明な機能キーです: {feature_key}")));
    }
    conn.execute(
        "INSERT INTO ai_feature_bindings
            (feature_key, provider_profile_id, model_id,
             fallback_provider_profile_id, fallback_model_id, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(feature_key) DO UPDATE SET
            provider_profile_id = excluded.provider_profile_id,
            model_id = excluded.model_id,
            fallback_provider_profile_id = excluded.fallback_provider_profile_id,
            fallback_model_id = excluded.fallback_model_id,
            updated_at = excluded.updated_at",
        rusqlite::params![
            feature_key,
            input.provider_profile_id,
            input.model_id,
            input.fallback_provider_profile_id,
            input.fallback_model_id,
            Utc::now().to_rfc3339(),
        ],
    )?;
    Ok(())
}

pub fn get_binding(conn: &Connection, feature_key: &str) -> Result<Option<FeatureBinding>, AppError> {
    Ok(conn
        .query_row(
            "SELECT feature_key, provider_profile_id, model_id,
                    fallback_provider_profile_id, fallback_model_id, updated_at
             FROM ai_feature_bindings WHERE feature_key = ?1",
            [feature_key],
            |row| {
                Ok(FeatureBinding {
                    feature_key: row.get(0)?,
                    provider_profile_id: row.get(1)?,
                    model_id: row.get(2)?,
                    fallback_provider_profile_id: row.get(3)?,
                    fallback_model_id: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            },
        )
        .optional()?)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageInput {
    pub provider_profile_id: String,
    pub feature_key: String,
    pub model_id: String,
    #[serde(default)]
    pub entity_id: Option<String>,
    #[serde(default)]
    pub input_units: Option<i64>,
    #[serde(default)]
    pub output_units: Option<i64>,
    #[serde(default)]
    pub audio_duration_ms: Option<i64>,
    #[serde(default)]
    pub latency_ms: Option<i64>,
    pub status: String,
    #[serde(default)]
    pub error_code: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageLog {
    pub id: String,
    pub provider_profile_id: String,
    pub feature_key: String,
    pub model_id: String,
    pub entity_id: Option<String>,
    pub input_units: Option<i64>,
    pub output_units: Option<i64>,
    pub audio_duration_ms: Option<i64>,
    pub latency_ms: Option<i64>,
    pub status: String,
    pub error_code: Option<String>,
    pub created_at: String,
}

pub fn insert_usage(conn: &Connection, input: UsageInput) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO api_usage_logs
            (id, provider_profile_id, feature_key, model_id, entity_id, input_units,
             output_units, audio_duration_ms, latency_ms, status, error_code, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        rusqlite::params![
            Uuid::new_v4().to_string(),
            input.provider_profile_id,
            input.feature_key,
            input.model_id,
            input.entity_id,
            input.input_units,
            input.output_units,
            input.audio_duration_ms,
            input.latency_ms,
            input.status,
            input.error_code,
            Utc::now().to_rfc3339(),
        ],
    )?;
    Ok(())
}

pub fn list_usage(
    conn: &Connection,
    provider_profile_id: Option<&str>,
    limit: i64,
) -> Result<Vec<UsageLog>, AppError> {
    let map_row = |row: &Row| -> rusqlite::Result<UsageLog> {
        Ok(UsageLog {
            id: row.get(0)?,
            provider_profile_id: row.get(1)?,
            feature_key: row.get(2)?,
            model_id: row.get(3)?,
            entity_id: row.get(4)?,
            input_units: row.get(5)?,
            output_units: row.get(6)?,
            audio_duration_ms: row.get(7)?,
            latency_ms: row.get(8)?,
            status: row.get(9)?,
            error_code: row.get(10)?,
            created_at: row.get(11)?,
        })
    };
    const COLUMNS: &str = "id, provider_profile_id, feature_key, model_id, entity_id, input_units,
             output_units, audio_duration_ms, latency_ms, status, error_code, created_at";
    match provider_profile_id {
        Some(provider) => {
            let mut stmt = conn.prepare(&format!(
                "SELECT {COLUMNS} FROM api_usage_logs WHERE provider_profile_id = ?1
                 ORDER BY created_at DESC, rowid DESC LIMIT ?2"
            ))?;
            let rows = stmt.query_map(rusqlite::params![provider, limit], map_row)?;
            Ok(rows.collect::<Result<Vec<_>, _>>()?)
        }
        None => {
            let mut stmt = conn.prepare(&format!(
                "SELECT {COLUMNS} FROM api_usage_logs ORDER BY created_at DESC, rowid DESC LIMIT ?1"
            ))?;
            let rows = stmt.query_map([limit], map_row)?;
            Ok(rows.collect::<Result<Vec<_>, _>>()?)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::open_database;
    use rusqlite::Connection;

    fn open_temp_db() -> (tempfile::TempDir, Connection) {
        let dir = tempfile::tempdir().expect("一時ディレクトリを作成できない");
        let conn = open_database(&dir.path().join("test.db")).expect("DBを開けない");
        (dir, conn)
    }

    fn sample_input(name: &str) -> ProviderInput {
        ProviderInput {
            display_name: name.to_string(),
            provider_type: "openai".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            auth_type: "bearer".to_string(),
            organization_id: None,
            project_id: None,
            model_id: None,
            custom_prompt: None,
            default_headers: Default::default(),
            timeout_ms: 60000,
            capabilities: vec![
                "transcription.batch".to_string(),
                "models.list".to_string(),
            ],
        }
    }

    #[test]
    fn base_urlの末尾スラッシュは正規化される() {
        assert_eq!(
            normalize_base_url("https://api.openai.com/v1/").unwrap(),
            "https://api.openai.com/v1"
        );
        assert_eq!(
            normalize_base_url("  https://api.openai.com/v1  ").unwrap(),
            "https://api.openai.com/v1"
        );
    }

    #[test]
    fn httpはローカルホストのみ許可される() {
        assert!(normalize_base_url("http://localhost:1234/v1").is_ok());
        assert!(normalize_base_url("http://127.0.0.1:8080/v1").is_ok());
        assert!(normalize_base_url("http://[::1]:8080/v1").is_ok());
        let err = normalize_base_url("http://example.com/v1").unwrap_err();
        assert_eq!(err.code, "VALIDATION_ERROR");
    }

    #[test]
    fn 非httpスキームは拒否される() {
        assert!(normalize_base_url("file:///c:/x").is_err());
        assert!(normalize_base_url("javascript:alert(1)").is_err());
        assert!(normalize_base_url("data:text/plain,x").is_err());
        assert!(normalize_base_url("api.openai.com/v1").is_err());
    }

    #[test]
    fn urlパーサーでuserinfoと不正hostを拒否する() {
        assert!(normalize_base_url("http://localhost:80@evil.example/v1").is_err());
        assert!(normalize_base_url("https://user:pass@example.com/v1").is_err());
        assert!(normalize_base_url("https://").is_err());
        assert!(normalize_base_url("http://127.1.2.3:8080/v1").is_ok());
        assert!(normalize_base_url("http://[::1]:8080/v1").is_ok());
    }

    #[test]
    fn providerを作成して取得できる() {
        let (_dir, conn) = open_temp_db();
        let created = create_provider(&conn, sample_input("OpenAI Personal")).unwrap();
        assert_eq!(created.display_name, "OpenAI Personal");
        assert_eq!(created.base_url, "https://api.openai.com/v1");
        assert!(created.enabled);
        let fetched = get_provider(&conn, &created.id).unwrap();
        assert_eq!(fetched.capabilities, created.capabilities);
    }

    #[test]
    fn model_idとcustom_promptを保存して更新できる() {
        let (_dir, conn) = open_temp_db();
        let mut input = sample_input("A");
        input.model_id = Some("gpt-4o-mini".to_string());
        input.custom_prompt = Some("敬体でまとめて".to_string());
        let created = create_provider(&conn, input).unwrap();
        assert_eq!(created.model_id.as_deref(), Some("gpt-4o-mini"));
        assert_eq!(created.custom_prompt.as_deref(), Some("敬体でまとめて"));
        let patch: ProviderPatch =
            serde_json::from_str(r#"{"modelId":"claude-sonnet-5","customPrompt":null}"#).unwrap();
        let updated = update_provider(&conn, &created.id, patch).unwrap();
        assert_eq!(updated.model_id.as_deref(), Some("claude-sonnet-5"));
        assert!(updated.custom_prompt.is_none());
    }

    #[test]
    fn 表示名の重複はvalidation_errorになる() {
        let (_dir, conn) = open_temp_db();
        create_provider(&conn, sample_input("A")).unwrap();
        let err = create_provider(&conn, sample_input("A")).unwrap_err();
        assert_eq!(err.code, "VALIDATION_ERROR");
    }

    #[test]
    fn カスタムヘッダーへ認証系ヘッダーは保存できない() {
        let (_dir, conn) = open_temp_db();
        let mut input = sample_input("X");
        input
            .default_headers
            .insert("Authorization".to_string(), "Bearer sk-abc".to_string());
        let err = create_provider(&conn, input).unwrap_err();
        assert_eq!(err.code, "VALIDATION_ERROR");
        let mut input2 = sample_input("Y");
        input2
            .default_headers
            .insert("x-api-key".to_string(), "abc".to_string());
        assert!(create_provider(&conn, input2).is_err());
    }

    #[test]
    fn provider一覧は表示名順で返る() {
        let (_dir, conn) = open_temp_db();
        create_provider(&conn, sample_input("B社ゲートウェイ")).unwrap();
        create_provider(&conn, sample_input("A社ゲートウェイ")).unwrap();
        let list = list_providers(&conn).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].display_name, "A社ゲートウェイ");
    }

    #[test]
    fn providerをパッチ更新できる() {
        let (_dir, conn) = open_temp_db();
        let created = create_provider(&conn, sample_input("A")).unwrap();
        let patch: ProviderPatch = serde_json::from_str(
            r#"{"displayName":"改名","baseUrl":"http://localhost:1234/v1/","timeoutMs":30000,"organizationId":"org-1"}"#,
        )
        .unwrap();
        let updated = update_provider(&conn, &created.id, patch).unwrap();
        assert_eq!(updated.display_name, "改名");
        assert_eq!(updated.base_url, "http://localhost:1234/v1");
        assert_eq!(updated.timeout_ms, 30000);
        assert_eq!(updated.organization_id.as_deref(), Some("org-1"));
    }

    #[test]
    fn パッチでorganization_idをnullへ戻せる() {
        let (_dir, conn) = open_temp_db();
        let mut input = sample_input("A");
        input.organization_id = Some("org-1".to_string());
        let created = create_provider(&conn, input).unwrap();
        let patch: ProviderPatch = serde_json::from_str(r#"{"organizationId":null}"#).unwrap();
        let updated = update_provider(&conn, &created.id, patch).unwrap();
        assert!(updated.organization_id.is_none());
    }

    #[test]
    fn 存在しないproviderはnot_foundになる() {
        let (_dir, conn) = open_temp_db();
        let err = get_provider(&conn, "missing").unwrap_err();
        assert_eq!(err.code, "API_PROVIDER_NOT_FOUND");
    }

    #[test]
    fn providerを削除できる() {
        let (_dir, conn) = open_temp_db();
        let created = create_provider(&conn, sample_input("A")).unwrap();
        delete_provider(&conn, &created.id).unwrap();
        assert!(get_provider(&conn, &created.id).is_err());
    }

    #[test]
    fn 有効無効を切り替えられる() {
        let (_dir, conn) = open_temp_db();
        let created = create_provider(&conn, sample_input("A")).unwrap();
        set_provider_enabled(&conn, &created.id, false).unwrap();
        assert!(!get_provider(&conn, &created.id).unwrap().enabled);
    }

    #[test]
    fn 接続テスト結果を記録できる() {
        let (_dir, conn) = open_temp_db();
        let created = create_provider(&conn, sample_input("A")).unwrap();
        record_test_result(&conn, &created.id, "success").unwrap();
        let fetched = get_provider(&conn, &created.id).unwrap();
        assert_eq!(fetched.last_test_status.as_deref(), Some("success"));
        assert!(fetched.last_tested_at.is_some());
    }

    #[test]
    fn feature_bindingを設定して取得できる() {
        let (_dir, conn) = open_temp_db();
        let provider = create_provider(&conn, sample_input("A")).unwrap();
        let input = BindingInput {
            provider_profile_id: Some(provider.id.clone()),
            model_id: Some("whisper-1".to_string()),
            fallback_provider_profile_id: None,
            fallback_model_id: None,
        };
        set_binding(&conn, "transcription.batch", input).unwrap();
        let binding = get_binding(&conn, "transcription.batch").unwrap().unwrap();
        assert_eq!(binding.provider_profile_id.as_deref(), Some(provider.id.as_str()));
        assert_eq!(binding.model_id.as_deref(), Some("whisper-1"));
    }

    #[test]
    fn feature_bindingは上書きできる() {
        let (_dir, conn) = open_temp_db();
        let provider = create_provider(&conn, sample_input("A")).unwrap();
        let make = |model: &str| BindingInput {
            provider_profile_id: Some(provider.id.clone()),
            model_id: Some(model.to_string()),
            fallback_provider_profile_id: None,
            fallback_model_id: None,
        };
        set_binding(&conn, "meeting.summary", make("gpt-a")).unwrap();
        set_binding(&conn, "meeting.summary", make("gpt-b")).unwrap();
        let binding = get_binding(&conn, "meeting.summary").unwrap().unwrap();
        assert_eq!(binding.model_id.as_deref(), Some("gpt-b"));
    }

    #[test]
    fn 不正なfeature_keyは拒否される() {
        let (_dir, conn) = open_temp_db();
        let input = BindingInput {
            provider_profile_id: None,
            model_id: None,
            fallback_provider_profile_id: None,
            fallback_model_id: None,
        };
        let err = set_binding(&conn, "unknown.feature", input).unwrap_err();
        assert_eq!(err.code, "VALIDATION_ERROR");
    }

    #[test]
    fn provider削除でbindingのproviderはnullになる() {
        let (_dir, conn) = open_temp_db();
        let provider = create_provider(&conn, sample_input("A")).unwrap();
        set_binding(
            &conn,
            "transcription.batch",
            BindingInput {
                provider_profile_id: Some(provider.id.clone()),
                model_id: Some("whisper-1".to_string()),
                fallback_provider_profile_id: None,
                fallback_model_id: None,
            },
        )
        .unwrap();
        delete_provider(&conn, &provider.id).unwrap();
        let binding = get_binding(&conn, "transcription.batch").unwrap().unwrap();
        assert!(binding.provider_profile_id.is_none());
    }

    #[test]
    fn 使用量ログを記録して新しい順に取得できる() {
        let (_dir, conn) = open_temp_db();
        let provider = create_provider(&conn, sample_input("A")).unwrap();
        for (i, status) in ["success", "error"].iter().enumerate() {
            insert_usage(
                &conn,
                UsageInput {
                    provider_profile_id: provider.id.clone(),
                    feature_key: "transcription.batch".to_string(),
                    model_id: "whisper-1".to_string(),
                    entity_id: Some(format!("m-{i}")),
                    input_units: None,
                    output_units: None,
                    audio_duration_ms: Some(20000),
                    latency_ms: Some(800),
                    status: status.to_string(),
                    error_code: if *status == "error" {
                        Some("API_TIMEOUT".to_string())
                    } else {
                        None
                    },
                },
            )
            .unwrap();
        }
        let logs = list_usage(&conn, Some(&provider.id), 10).unwrap();
        assert_eq!(logs.len(), 2);
        let errors: Vec<_> = logs.iter().filter(|l| l.status == "error").collect();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].error_code.as_deref(), Some("API_TIMEOUT"));
    }
}
