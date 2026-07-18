use std::path::Path;
use std::time::Duration;

use serde_json::json;
use tauri::AppHandle;
use tauri_plugin_shell::process::CommandEvent;
use tauri_plugin_shell::ShellExt;

use crate::error::AppError;

const TRANSCRIBE_TIMEOUT: Duration = Duration::from_secs(300);

fn whisper_error(message: String) -> AppError {
    AppError::new("WHISPER_FAILED", message, true)
}

pub fn build_transcribe_command(model_path: &Path, wav_path: &Path) -> String {
    json!({
        "command": "transcribe",
        "modelPath": model_path.to_string_lossy(),
        "wavPath": wav_path.to_string_lossy(),
        "language": "ja",
    })
    .to_string()
}

pub fn parse_transcribe_response(line: &str) -> Result<String, AppError> {
    let value: serde_json::Value = serde_json::from_str(line.trim().trim_start_matches('\u{feff}'))
        .map_err(|_| whisper_error("Whisperの応答を解釈できません".to_string()))?;
    match value["event"].as_str() {
        Some("transcribe.result") => Ok(value["text"].as_str().unwrap_or("").to_string()),
        Some("transcribe.error") => Err(AppError::new(
            value["code"].as_str().unwrap_or("WHISPER_FAILED"),
            value["message"]
                .as_str()
                .unwrap_or("ローカル文字起こしに失敗しました"),
            false,
        )),
        _ => Err(whisper_error("Whisperの応答種別が不明です".to_string())),
    }
}

/// Sidecarのwhisperモードで音声チャンクをローカル文字起こしする。
pub async fn transcribe(
    app: &AppHandle,
    model_path: &Path,
    wav_path: &Path,
) -> Result<String, AppError> {
    let sidecar = app
        .shell()
        .sidecar("inquivora-native")
        .map_err(|e| whisper_error(format!("Sidecarを解決できません: {e}")))?
        .args(["whisper"]);
    let (mut rx, mut child) = sidecar
        .spawn()
        .map_err(|e| whisper_error(format!("Sidecarを起動できません: {e}")))?;
    let command = build_transcribe_command(model_path, wav_path);
    child
        .write(format!("{command}\n").as_bytes())
        .map_err(|e| whisper_error(format!("Sidecarへ書き込めません: {e}")))?;
    let _ = child.write(b"{\"command\":\"stop\"}\n");

    let wait_response = async {
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(line) => {
                    let text = String::from_utf8_lossy(&line);
                    let trimmed = text.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    return parse_transcribe_response(trimmed);
                }
                CommandEvent::Terminated(_) => {
                    return Err(whisper_error("Sidecarが応答せず終了しました".to_string()));
                }
                _ => {}
            }
        }
        Err(whisper_error("Sidecarからの応答がありません".to_string()))
    };
    let result = match tokio::time::timeout(TRANSCRIBE_TIMEOUT, wait_response).await {
        Ok(result) => result,
        Err(_) => Err(whisper_error(
            "ローカル文字起こしがタイムアウトしました".to_string(),
        )),
    };
    drop(child);
    result
}
