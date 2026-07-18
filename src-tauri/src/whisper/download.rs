use std::path::{Path, PathBuf};

use futures_util::StreamExt;
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::AsyncWriteExt;

use crate::error::AppError;
use crate::whisper::models;

fn download_error(message: String) -> AppError {
    AppError::new("WHISPER_DOWNLOAD_FAILED", message, true)
}

pub fn models_dir(app: &AppHandle) -> Result<PathBuf, AppError> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| download_error(format!("データディレクトリを解決できません: {e}")))?
        .join("models");
    Ok(dir)
}

/// Whisperモデルをダウンロードし、完了後に正式なファイル名へ置き換える。
/// 進捗は whisper:model-download-progress イベントで通知する。
pub async fn download_model(app: &AppHandle, name: &str) -> Result<(), AppError> {
    let url = models::model_url(name)
        .ok_or_else(|| AppError::new("VALIDATION_ERROR", format!("不明なWhisperモデルです: {name}"), false))?;
    let dir = models_dir(app)?;
    let target = models::model_path(&dir, name).expect("カタログ検証済み");
    if target.is_file() {
        return Ok(());
    }
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| download_error(format!("モデル保存先を作成できません: {e}")))?;
    let temp_path = dir.join(format!("ggml-{name}.bin.download"));

    let client = reqwest::Client::builder()
        .build()
        .map_err(|e| download_error(format!("HTTPクライアントを初期化できません: {e}")))?;
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| download_error(format!("モデルのダウンロードを開始できません: {e}")))?;
    if !response.status().is_success() {
        return Err(download_error(format!(
            "モデルのダウンロードに失敗しました (HTTP {})",
            response.status().as_u16()
        )));
    }
    let total_bytes = response.content_length();
    let mut received: u64 = 0;
    let mut file = tokio::fs::File::create(&temp_path)
        .await
        .map_err(|e| download_error(format!("一時ファイルを作成できません: {e}")))?;
    let mut stream = response.bytes_stream();
    let mut last_emitted = 0u64;
    while let Some(chunk) = stream.next().await {
        let chunk = match chunk {
            Ok(chunk) => chunk,
            Err(e) => {
                cleanup(&temp_path).await;
                return Err(download_error(format!("ダウンロード中に切断されました: {e}")));
            }
        };
        if let Err(e) = file.write_all(&chunk).await {
            cleanup(&temp_path).await;
            return Err(download_error(format!("モデルの書き込みに失敗しました: {e}")));
        }
        received += chunk.len() as u64;
        if received - last_emitted >= 4 * 1024 * 1024 {
            last_emitted = received;
            emit_progress(app, name, received, total_bytes);
        }
    }
    if let Err(e) = file.flush().await {
        cleanup(&temp_path).await;
        return Err(download_error(format!("モデルの書き込みに失敗しました: {e}")));
    }
    drop(file);
    if let Some(total) = total_bytes {
        if received != total {
            cleanup(&temp_path).await;
            return Err(download_error(
                "ダウンロードが途中で終了しました。再試行してください".to_string(),
            ));
        }
    }
    tokio::fs::rename(&temp_path, &target)
        .await
        .map_err(|e| download_error(format!("モデルファイルの配置に失敗しました: {e}")))?;
    emit_progress(app, name, received, total_bytes.or(Some(received)));
    Ok(())
}

async fn cleanup(temp_path: &Path) {
    let _ = tokio::fs::remove_file(temp_path).await;
}

fn emit_progress(app: &AppHandle, name: &str, received: u64, total: Option<u64>) {
    let _ = app.emit(
        "whisper:model-download-progress",
        json!({ "name": name, "receivedBytes": received, "totalBytes": total }),
    );
}
