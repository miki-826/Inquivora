use std::time::Duration;

use serde_json::json;
use tauri::AppHandle;
use tauri_plugin_shell::process::CommandEvent;
use tauri_plugin_shell::ShellExt;

use crate::error::AppError;

fn credential_error(message: String) -> AppError {
    AppError::new("CREDENTIAL_ERROR", message, false)
}

/// Sidecarのcredentialモードへコマンドを送り、応答を1件受け取る。
/// シークレットはstdoutパイプ内のみを通り、ログへは出さない。
async fn run_credential_command(
    app: &AppHandle,
    command_line: &str,
) -> Result<CredentialResponse, AppError> {
    let sidecar = app
        .shell()
        .sidecar("inquivora-native")
        .map_err(|e| credential_error(format!("Sidecarを解決できません: {e}")))?
        .args(["credential"]);
    let (mut rx, mut child) = sidecar
        .spawn()
        .map_err(|e| credential_error(format!("Sidecarを起動できません: {e}")))?;
    child
        .write(format!("{command_line}\n").as_bytes())
        .map_err(|e| credential_error(format!("Sidecarへ書き込めません: {e}")))?;

    let wait_response = async {
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(line) => {
                    let text = String::from_utf8_lossy(&line);
                    let trimmed = text.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    return parse_credential_response(trimmed);
                }
                CommandEvent::Terminated(_) => {
                    return Err(credential_error("Sidecarが応答せず終了しました".to_string()));
                }
                _ => {}
            }
        }
        Err(credential_error("Sidecarからの応答がありません".to_string()))
    };
    let result = match tokio::time::timeout(Duration::from_secs(10), wait_response).await {
        Ok(result) => result,
        Err(_) => Err(credential_error("Sidecarの応答がタイムアウトしました".to_string())),
    };
    drop(child);
    result
}

pub async fn set_secret(
    app: &AppHandle,
    profile_id: &str,
    user_name: &str,
    secret: &str,
) -> Result<(), AppError> {
    match run_credential_command(app, &build_set_command(profile_id, user_name, secret)).await? {
        CredentialResponse::Ok => Ok(()),
        CredentialResponse::Error { code, message } => Err(AppError::new(code, message, false)),
        _ => Err(credential_error("APIキーの保存に失敗しました".to_string())),
    }
}

pub async fn get_secret(app: &AppHandle, profile_id: &str) -> Result<Option<String>, AppError> {
    match run_credential_command(app, &build_get_command(profile_id)).await? {
        CredentialResponse::Secret(secret) => Ok(Some(secret)),
        CredentialResponse::NotFound => Ok(None),
        CredentialResponse::Error { code, message } => Err(AppError::new(code, message, false)),
        CredentialResponse::Ok => Ok(None),
    }
}

pub async fn has_secret(app: &AppHandle, profile_id: &str) -> Result<bool, AppError> {
    match run_credential_command(app, &build_has_command(profile_id)).await? {
        CredentialResponse::Ok => Ok(true),
        CredentialResponse::NotFound => Ok(false),
        CredentialResponse::Error { code, message } => Err(AppError::new(code, message, false)),
        CredentialResponse::Secret(_) => Ok(true),
    }
}

pub async fn delete_secret(app: &AppHandle, profile_id: &str) -> Result<(), AppError> {
    match run_credential_command(app, &build_delete_command(profile_id)).await? {
        CredentialResponse::Ok | CredentialResponse::NotFound => Ok(()),
        CredentialResponse::Error { code, message } => Err(AppError::new(code, message, false)),
        CredentialResponse::Secret(_) => Ok(()),
    }
}

pub fn credential_target(profile_id: &str) -> String {
    format!("Inquivora/API/{profile_id}")
}

pub fn build_set_command(profile_id: &str, user_name: &str, secret: &str) -> String {
    json!({
        "command": "set",
        "target": credential_target(profile_id),
        "userName": user_name,
        "secret": secret,
    })
    .to_string()
}

pub fn build_get_command(profile_id: &str) -> String {
    json!({ "command": "get", "target": credential_target(profile_id) }).to_string()
}

pub fn build_has_command(profile_id: &str) -> String {
    json!({ "command": "has", "target": credential_target(profile_id) }).to_string()
}

pub fn build_delete_command(profile_id: &str) -> String {
    json!({ "command": "delete", "target": credential_target(profile_id) }).to_string()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CredentialResponse {
    Ok,
    Secret(String),
    NotFound,
    Error { code: String, message: String },
}

pub fn parse_credential_response(line: &str) -> Result<CredentialResponse, AppError> {
    let value: serde_json::Value =
        serde_json::from_str(line.trim().trim_start_matches('\u{feff}')).map_err(|_| {
            AppError::new("CREDENTIAL_ERROR", "資格情報の応答を解釈できません", false)
        })?;
    match value["type"].as_str() {
        Some("credential.ok") => Ok(CredentialResponse::Ok),
        Some("credential.secret") => value["secret"]
            .as_str()
            .map(|secret| CredentialResponse::Secret(secret.to_string()))
            .ok_or_else(|| {
                AppError::new("CREDENTIAL_ERROR", "資格情報の応答にsecretがありません", false)
            }),
        Some("credential.notFound") => Ok(CredentialResponse::NotFound),
        Some("credential.error") => Ok(CredentialResponse::Error {
            code: value["code"].as_str().unwrap_or("CREDENTIAL_ERROR").to_string(),
            message: value["message"].as_str().unwrap_or("資格情報の操作に失敗しました").to_string(),
        }),
        _ => Err(AppError::new(
            "CREDENTIAL_ERROR",
            "資格情報の応答種別が不明です",
            false,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credential_targetはprofile_idから決まる() {
        assert_eq!(credential_target("abc-123"), "Inquivora/API/abc-123");
    }

    #[test]
    fn setコマンドのjsonを構築できる() {
        let json = build_set_command("p1", "openai", "sk-secret");
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["command"], "set");
        assert_eq!(value["target"], "Inquivora/API/p1");
        assert_eq!(value["userName"], "openai");
        assert_eq!(value["secret"], "sk-secret");
        assert!(!json.contains('\n'), "NDJSONは1行で構築する");
    }

    #[test]
    fn get_has_deleteコマンドを構築できる() {
        for (builder, command) in [
            (build_get_command as fn(&str) -> String, "get"),
            (build_has_command, "has"),
            (build_delete_command, "delete"),
        ] {
            let json = builder("p1");
            let value: serde_json::Value = serde_json::from_str(&json).unwrap();
            assert_eq!(value["command"], command);
            assert_eq!(value["target"], "Inquivora/API/p1");
        }
    }

    #[test]
    fn 応答行からsecretを取り出せる() {
        let parsed =
            parse_credential_response(r#"{"type":"credential.secret","secret":"sk-abc"}"#).unwrap();
        assert_eq!(parsed, CredentialResponse::Secret("sk-abc".to_string()));
    }

    #[test]
    fn 応答行のok_notfound_errorを判別できる() {
        assert_eq!(
            parse_credential_response(r#"{"type":"credential.ok"}"#).unwrap(),
            CredentialResponse::Ok
        );
        assert_eq!(
            parse_credential_response(r#"{"type":"credential.notFound"}"#).unwrap(),
            CredentialResponse::NotFound
        );
        match parse_credential_response(r#"{"type":"credential.error","code":"CRED_WRITE_FAILED","message":"x"}"#)
            .unwrap()
        {
            CredentialResponse::Error { code, .. } => assert_eq!(code, "CRED_WRITE_FAILED"),
            other => panic!("errorとして解釈されるべき: {other:?}"),
        }
    }

    #[test]
    fn 不正な応答行はエラーになる() {
        assert!(parse_credential_response("not json").is_err());
        assert!(parse_credential_response(r#"{"type":"unknown.event"}"#).is_err());
    }
}
