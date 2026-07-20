use std::path::{Path, PathBuf};

use rusqlite::Connection;
use serde::Serialize;

use crate::error::AppError;

pub const DEFAULT_MODEL: &str = "small";
pub const SELECTED_MODEL_KEY: &str = "whisper.model";

pub struct ModelSpec {
    pub name: &'static str,
    pub display_name: &'static str,
    pub size_mb: u64,
    pub size_bytes: u64,
    pub sha256: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelStatus {
    pub name: String,
    pub display_name: String,
    pub size_mb: u64,
    pub downloaded: bool,
    pub selected: bool,
}

const CATALOG: &[ModelSpec] = &[
    ModelSpec {
        name: "tiny",
        display_name: "Tiny（最小・低精度）",
        size_mb: 75,
        size_bytes: 77_691_713,
        sha256: "be07e048e1e599ad46341c8d2a135645097a538221678b7acdd1b1919c6e1b21",
    },
    ModelSpec {
        name: "base",
        display_name: "Base（小型・軽量）",
        size_mb: 142,
        size_bytes: 147_951_465,
        sha256: "60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe",
    },
    ModelSpec {
        name: "small",
        display_name: "Small（推奨・日本語向け）",
        size_mb: 466,
        size_bytes: 487_601_967,
        sha256: "1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b",
    },
];

pub fn catalog() -> &'static [ModelSpec] {
    CATALOG
}

pub fn model_spec(name: &str) -> Option<&'static ModelSpec> {
    CATALOG.iter().find(|m| m.name == name)
}

pub fn model_url(name: &str) -> Option<String> {
    model_spec(name).map(|m| {
        format!("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-{}.bin", m.name)
    })
}

pub fn model_path(models_dir: &Path, name: &str) -> Option<PathBuf> {
    model_spec(name).map(|m| models_dir.join(format!("ggml-{}.bin", m.name)))
}

pub fn model_status(models_dir: &Path, selected: &str) -> Vec<ModelStatus> {
    CATALOG
        .iter()
        .map(|m| ModelStatus {
            name: m.name.to_string(),
            display_name: m.display_name.to_string(),
            size_mb: m.size_mb,
            downloaded: models_dir.join(format!("ggml-{}.bin", m.name)).is_file(),
            selected: m.name == selected,
        })
        .collect()
}

pub fn selected_model(conn: &Connection) -> Result<String, AppError> {
    let stored = crate::database::settings::get_setting(conn, SELECTED_MODEL_KEY)?
        .and_then(|v| v.as_str().map(str::to_string));
    Ok(match stored {
        Some(name) if model_spec(&name).is_some() => name,
        _ => DEFAULT_MODEL.to_string(),
    })
}

pub fn set_selected_model(conn: &Connection, name: &str) -> Result<(), AppError> {
    if model_spec(name).is_none() {
        return Err(AppError::new(
            "VALIDATION_ERROR",
            format!("不明なWhisperモデルです: {name}"),
            false,
        ));
    }
    crate::database::settings::set_setting(conn, SELECTED_MODEL_KEY, &serde_json::Value::String(name.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::open_database;

    fn temp_conn() -> (tempfile::TempDir, Connection) {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_database(&dir.path().join("test.db")).unwrap();
        (dir, conn)
    }

    #[test]
    fn カタログはtiny_base_smallを含む() {
        let names: Vec<&str> = catalog().iter().map(|m| m.name).collect();
        assert_eq!(names, vec!["tiny", "base", "small"]);
        assert!(catalog().iter().all(|model| model.sha256.len() == 64));
        assert!(catalog().iter().all(|model| model.size_bytes > 0));
    }

    #[test]
    fn モデルurlはhugging_faceのggmlバイナリを指す() {
        assert_eq!(
            model_url("small").unwrap(),
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin"
        );
        assert!(model_url("unknown").is_none());
    }

    #[test]
    fn モデルパスはggmlファイル名になる() {
        let path = model_path(Path::new("C:/data/models"), "base").unwrap();
        assert!(path.ends_with("ggml-base.bin"));
        assert!(model_path(Path::new("C:/data/models"), "huge").is_none());
    }

    #[test]
    fn モデル状態はファイル有無と選択を反映する() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("ggml-tiny.bin"), b"dummy").unwrap();
        let status = model_status(dir.path(), "small");
        let tiny = status.iter().find(|s| s.name == "tiny").unwrap();
        let small = status.iter().find(|s| s.name == "small").unwrap();
        assert!(tiny.downloaded);
        assert!(!tiny.selected);
        assert!(!small.downloaded);
        assert!(small.selected);
    }

    #[test]
    fn 選択モデルの既定はsmall() {
        let (_dir, conn) = temp_conn();
        assert_eq!(selected_model(&conn).unwrap(), "small");
    }

    #[test]
    fn 選択モデルを変更でき未知の名前は拒否する() {
        let (_dir, conn) = temp_conn();
        set_selected_model(&conn, "base").unwrap();
        assert_eq!(selected_model(&conn).unwrap(), "base");
        let err = set_selected_model(&conn, "huge").unwrap_err();
        assert_eq!(err.code, "VALIDATION_ERROR");
    }
}
