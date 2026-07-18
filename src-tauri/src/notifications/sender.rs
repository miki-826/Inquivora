use std::time::Duration;

use serde_json::json;
use tauri::AppHandle;
use tauri_plugin_shell::process::CommandEvent;
use tauri_plugin_shell::ShellExt;

use crate::error::AppError;
use crate::notifications::schedule::NotificationPayload;

fn failed(message: String) -> AppError {
    AppError::new("NOTIFICATION_FAILED", message, true)
}

/// Sidecarのnotifyモードへ§14.4のNDJSONコマンドを送り、表示ACKを待つ。
pub async fn send_notification(
    app: &AppHandle,
    payload: &NotificationPayload,
    silent: bool,
) -> Result<(), AppError> {
    let command = json!({
        "command": "notify",
        "notificationId": payload.notification_id,
        "title": payload.title,
        "body": payload.body,
        "launchUri": payload.launch_uri,
        "silent": silent,
    });
    let sidecar = app
        .shell()
        .sidecar("inquivora-native")
        .map_err(|e| failed(format!("通知Sidecarを解決できません: {e}")))?
        .args(["notify"]);
    let (mut rx, mut child) = sidecar
        .spawn()
        .map_err(|e| failed(format!("通知Sidecarを起動できません: {e}")))?;
    child
        .write(format!("{command}\n").as_bytes())
        .map_err(|e| failed(format!("通知Sidecarへ書き込めません: {e}")))?;

    let wait_ack = async {
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(line) => {
                    let text = String::from_utf8_lossy(&line);
                    let Ok(value) = serde_json::from_str::<serde_json::Value>(text.trim()) else {
                        continue;
                    };
                    match value.get("type").and_then(|v| v.as_str()) {
                        Some("notification.shown") => return Ok(()),
                        Some("notification.error") => {
                            let message = value
                                .get("message")
                                .and_then(|v| v.as_str())
                                .unwrap_or("不明なエラー");
                            return Err(failed(format!("通知の表示に失敗しました: {message}")));
                        }
                        _ => {}
                    }
                }
                CommandEvent::Terminated(_) => {
                    return Err(failed("通知Sidecarが応答せず終了しました".to_string()));
                }
                _ => {}
            }
        }
        Err(failed("通知Sidecarからの応答がありません".to_string()))
    };
    let result = match tokio::time::timeout(Duration::from_secs(20), wait_ack).await {
        Ok(result) => result,
        Err(_) => Err(failed("通知Sidecarの応答がタイムアウトしました".to_string())),
    };
    drop(child);
    result
}
