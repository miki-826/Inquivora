use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::json;
use tauri::async_runtime::Receiver;
use tauri::{AppHandle, Manager};
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
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

/// 常駐Whisper Sidecar。モデルを1度だけロードし、チャンクごとに使い回す（毎回の起動・モデル再読込を避ける）。
struct WhisperProc {
    child: CommandChild,
    rx: Receiver<CommandEvent>,
    model_path: PathBuf,
}

#[derive(Default)]
pub struct WhisperRuntime(pub tokio::sync::Mutex<Option<WhisperProc>>);

impl WhisperRuntime {
    /// アイドル時にSidecarを終了させてモデル分のメモリを解放する。
    pub async fn shutdown(&self) {
        if let Some(mut proc) = self.0.lock().await.take() {
            let _ = proc.child.write(b"{\"command\":\"stop\"}\n");
            let _ = proc.child.kill();
        }
    }

    pub async fn is_running(&self) -> bool {
        self.0.lock().await.is_some()
    }
}

fn spawn_proc(app: &AppHandle, model_path: &Path) -> Result<WhisperProc, AppError> {
    let (rx, child) = app
        .shell()
        .sidecar("inquivora-native")
        .map_err(|e| whisper_error(format!("Sidecarを解決できません: {e}")))?
        .args(["whisper"])
        .spawn()
        .map_err(|e| whisper_error(format!("Sidecarを起動できません: {e}")))?;
    Ok(WhisperProc {
        child,
        rx,
        model_path: model_path.to_path_buf(),
    })
}

/// 常駐Sidecarへ音声チャンクを送りローカル文字起こしする。
/// プロセスは使い回し、モデル変更時のみ再起動する。
pub async fn transcribe(
    app: &AppHandle,
    model_path: &Path,
    wav_path: &Path,
) -> Result<String, AppError> {
    let runtime = app.state::<WhisperRuntime>();
    let mut guard = runtime.0.lock().await;

    // 未起動、またはモデルが変わった場合は起動し直す。
    let needs_spawn = match guard.as_ref() {
        Some(proc) => proc.model_path != model_path,
        None => true,
    };
    if needs_spawn {
        if let Some(mut old) = guard.take() {
            let _ = old.child.write(b"{\"command\":\"stop\"}\n");
            let _ = old.child.kill();
        }
        *guard = Some(spawn_proc(app, model_path)?);
    }

    let proc = guard.as_mut().expect("直前に起動済み");
    let command = build_transcribe_command(model_path, wav_path);
    if proc.child.write(format!("{command}\n").as_bytes()).is_err() {
        *guard = None;
        return Err(whisper_error("Sidecarへ書き込めません（再起動します）".to_string()));
    }

    let wait_response = async {
        while let Some(event) = proc.rx.recv().await {
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

    match tokio::time::timeout(TRANSCRIBE_TIMEOUT, wait_response).await {
        Ok(Ok(text)) => Ok(text),
        Ok(Err(err)) => {
            // プロセス異常時は破棄して次回に再起動させる。
            *guard = None;
            Err(err)
        }
        Err(_) => {
            *guard = None;
            Err(whisper_error(
                "ローカル文字起こしがタイムアウトしました".to_string(),
            ))
        }
    }
}
