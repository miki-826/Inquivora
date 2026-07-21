use std::collections::HashSet;
use std::path::PathBuf;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
#[cfg(target_os = "windows")]
use std::process::Command;

use serde::Deserialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::commands::workspace::{active_root, WorkspaceState};
use crate::database::search::{self, SearchResult};
use crate::error::AppError;
use crate::search as indexer;
use crate::DbState;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn lock_error(e: impl std::fmt::Display) -> AppError {
    AppError::new("STATE_LOCK_FAILED", format!("状態ロックに失敗: {e}"), true)
}

/// §17.7 全文検索。空クエリは空配列を返す。
#[tauri::command]
pub fn search_global(
    db: State<'_, DbState>,
    query: String,
    entity_types: Option<Vec<String>>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<SearchResult>, AppError> {
    let conn = db.0.lock().map_err(lock_error)?;
    search::search(
        &conn,
        &query,
        &entity_types.unwrap_or_default(),
        limit.unwrap_or(50).clamp(1, 200),
        offset.unwrap_or(0).max(0),
    )
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ComputerFileResult {
    path: String,
    name: String,
}

/// Windows Search のインデックスを使い、PC全体からファイル名を検索する。
/// ディスクを毎回走査しないため、大きなPCでもUIをブロックしない。
#[tauri::command]
pub async fn search_computer_files(
    query: String,
    limit: Option<usize>,
) -> Result<Vec<SearchResult>, AppError> {
    let trimmed = query.trim().to_owned();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    let limit = limit.unwrap_or(100).clamp(1, 200);

    tauri::async_runtime::spawn_blocking(move || search_computer_files_blocking(&trimmed, limit))
        .await
        .map_err(|error| {
            AppError::new(
                "COMPUTER_SEARCH_FAILED",
                format!("PC全体の検索処理を開始できませんでした: {error}"),
                true,
            )
        })?
}

/// PC全体検索の結果をエクスプローラーで選択表示する。
/// ワークスペース外のパスを扱うため、WorkspaceState によるパス制限とは分離する。
#[tauri::command]
pub fn search_reveal_computer_file(path: String) -> Result<(), AppError> {
    #[cfg(target_os = "windows")]
    {
        let target = PathBuf::from(path);
        if !target.is_absolute() || !target.is_file() {
            return Err(AppError::new(
                "COMPUTER_FILE_NOT_FOUND",
                "表示するファイルが見つかりません",
                false,
            ));
        }
        Command::new("explorer.exe")
            .arg(format!("/select,{}", target.display()))
            .spawn()
            .map_err(|error| {
                AppError::new(
                    "COMPUTER_FILE_REVEAL_FAILED",
                    format!("エクスプローラーを起動できません: {error}"),
                    true,
                )
            })?;
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = path;
        Err(AppError::new(
            "COMPUTER_FILE_REVEAL_UNAVAILABLE",
            "PC全体検索結果の表示はWindowsでのみ利用できます",
            false,
        ))
    }
}

#[cfg(target_os = "windows")]
fn search_computer_files_blocking(
    query: &str,
    limit: usize,
) -> Result<Vec<SearchResult>, AppError> {
    const SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
$query = $env:INQUIVORA_SEARCH_QUERY
$limit = [Math]::Min([Math]::Max([int]$env:INQUIVORA_SEARCH_LIMIT, 1), 200)
$escaped = $query.Replace("'", "''").Replace("[", "[[]").Replace("%", "[%]").Replace("_", "[_]")
$connection = New-Object System.Data.OleDb.OleDbConnection("Provider=Search.CollatorDSO;Extended Properties='Application=Windows';")
$items = @()
try {
  $connection.Open()
  $command = $connection.CreateCommand()
  $command.CommandText = "SELECT TOP $limit System.ItemUrl, System.FileName FROM SYSTEMINDEX WHERE System.FileName LIKE '%$escaped%' AND System.ItemType <> 'Directory'"
  $reader = $command.ExecuteReader()
  try {
    while ($reader.Read()) {
      if ($reader.IsDBNull(0)) { continue }
      $itemUrl = $reader.GetString(0)
      try { $path = ([Uri]$itemUrl).LocalPath } catch { continue }
      if ([string]::IsNullOrWhiteSpace($path) -or -not [System.IO.Path]::IsPathRooted($path)) { continue }
      $name = if ($reader.IsDBNull(1)) { [System.IO.Path]::GetFileName($path) } else { $reader.GetString(1) }
      $items += [PSCustomObject]@{ Path = $path; Name = $name }
    }
  } finally {
    if ($reader) { $reader.Dispose() }
  }
} finally {
  $connection.Dispose()
}
ConvertTo-Json -InputObject @($items) -Compress
"#;

    let output = Command::new("powershell.exe")
        // Windows Search を使う PowerShell をコンソールなしで起動する。
        // stdout/stderr はパイプで受け取るため、検索結果やエラー表示は従来どおり維持される。
        .creation_flags(CREATE_NO_WINDOW)
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-WindowStyle",
            "Hidden",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            SCRIPT,
        ])
        .env("INQUIVORA_SEARCH_QUERY", query)
        .env("INQUIVORA_SEARCH_LIMIT", limit.to_string())
        .output()
        .map_err(|error| {
            AppError::new(
                "COMPUTER_SEARCH_UNAVAILABLE",
                format!("Windows検索を起動できませんでした: {error}"),
                true,
            )
        })?;

    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(AppError::new(
            "COMPUTER_SEARCH_FAILED",
            if detail.is_empty() {
                "Windows検索インデックスを利用できませんでした".to_owned()
            } else {
                format!("Windows検索インデックスを利用できませんでした: {detail}")
            },
            true,
        ));
    }

    let json = String::from_utf8(output.stdout).map_err(|error| {
        AppError::new(
            "COMPUTER_SEARCH_FAILED",
            format!("検索結果の文字コードを読み取れませんでした: {error}"),
            true,
        )
    })?;
    let files: Vec<ComputerFileResult> = serde_json::from_str(json.trim()).map_err(|error| {
        AppError::new(
            "COMPUTER_SEARCH_FAILED",
            format!("検索結果を読み取れませんでした: {error}"),
            true,
        )
    })?;

    Ok(files
        .into_iter()
        .map(|file| SearchResult {
            entity_type: "file".to_owned(),
            entity_id: file.path.clone(),
            title: file.name,
            snippet: String::new(),
            path: Some(file.path),
        })
        .collect())
}

#[cfg(not(target_os = "windows"))]
fn search_computer_files_blocking(
    _query: &str,
    _limit: usize,
) -> Result<Vec<SearchResult>, AppError> {
    Err(AppError::new(
        "COMPUTER_SEARCH_UNAVAILABLE",
        "PC全体の検索はWindows版でのみ利用できます",
        false,
    ))
}

/// §15.3/§17.7 索引の全再構築。UIをブロックしないよう別スレッドで実行し、
/// ファイル走査はDBロック外で行い、書込のみ1トランザクションで短時間ロックする。
#[tauri::command]
pub fn search_reindex_workspace(app: AppHandle, ws: State<'_, WorkspaceState>) -> Result<(), AppError> {
    let root = active_root(&ws).ok();
    let handle = app.clone();
    std::thread::spawn(move || {
        let _ = handle.emit("search:index-started", ());
        let mut count = 0usize;
        // DB由来（タスク/予定/会議）と全索引のクリアは1トランザクションで短時間ロック
        if let Ok(mut conn) = handle.state::<DbState>().0.lock() {
            count += indexer::reindex_entities(&mut conn).unwrap_or(0);
        }
        // ファイルはパスだけ収集し、バッチ単位で「ロック外読み込み→短時間ロック書込」を繰り返す。
        // 一度に全ファイルをメモリへ載せないため省メモリ。
        if let Some(root) = root {
            let paths = indexer::collect_workspace_file_paths(&root);
            for batch in paths.chunks(indexer::INDEX_BATCH) {
                let docs = indexer::read_file_docs(batch);
                if let Ok(mut conn) = handle.state::<DbState>().0.lock() {
                    let _ = indexer::write_file_docs(&mut conn, &docs);
                }
                count += docs.len();
            }
        }
        let _ = handle.emit("search:index-done", count);
    });
    Ok(())
}

/// 変更されたパスだけを索引へ反映する（ウォッチャの外部変更追従用）。
#[tauri::command]
pub fn search_index_paths(db: State<'_, DbState>, paths: Vec<String>) -> Result<(), AppError> {
    let mut seen = HashSet::new();
    let paths: Vec<String> = paths
        .into_iter()
        .filter(|path| seen.insert(path.clone()))
        .collect();
    let path_bufs: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();

    // ディスクI/OはDB mutexの外で行う。これにより大量の監視イベント中でも
    // タスク・予定・設定など他のDB操作を待たせない。
    let docs = indexer::read_file_docs(&path_bufs);
    let mut conn = db.0.lock().map_err(lock_error)?;
    indexer::replace_file_docs(&mut conn, &paths, &docs)
}
