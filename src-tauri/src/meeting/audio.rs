use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager};

use crate::database::meetings::TranscriptSegment;
use crate::error::AppError;

pub const SAMPLE_RATE: u32 = 16000;

fn audio_error(message: impl Into<String>) -> AppError {
    AppError::new("AUDIO_EXPORT_FAILED", message.into(), false)
}

/// 会議の録音チャンク保存先（§9.7 app_data/audio/{meetingId}）。
pub fn meeting_audio_dir(app: &AppHandle, meeting_id: &str) -> Result<PathBuf, AppError> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|e| audio_error(format!("データフォルダを解決できません: {e}")))?
        .join("audio")
        .join(meeting_id))
}

struct Chunk {
    start_ms: i64,
    end_ms: i64,
    samples: Vec<i16>,
}

/// PCM16 mono WAVのdataチャンクからi16サンプル列を取り出す。
pub fn parse_wav_pcm16(bytes: &[u8]) -> Result<Vec<i16>, AppError> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(audio_error("WAVヘッダーが不正です"));
    }
    let mut pos = 12;
    while pos + 8 <= bytes.len() {
        let id = &bytes[pos..pos + 4];
        let size = u32::from_le_bytes([bytes[pos + 4], bytes[pos + 5], bytes[pos + 6], bytes[pos + 7]])
            as usize;
        let body_start = pos + 8;
        if id == b"data" {
            let end = (body_start + size).min(bytes.len());
            let data = &bytes[body_start..end];
            let samples = data
                .chunks_exact(2)
                .map(|b| i16::from_le_bytes([b[0], b[1]]))
                .collect();
            return Ok(samples);
        }
        pos = body_start + size + (size & 1);
    }
    Err(audio_error("WAVにdataチャンクがありません"))
}

/// PCM16 mono WAVのバイト列を生成する。
pub fn write_wav_pcm16(samples: &[i16], sample_rate: u32) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&sample_rate.to_le_bytes());
    out.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for &sample in samples {
        out.extend_from_slice(&sample.to_le_bytes());
    }
    out
}

/// 時系列順のチャンクを、直前チャンクとの重なり（1秒オーバーラップ）を先頭から
/// 取り除いて連結する。無音でスキップされた区間は詰められる。
fn merge_chunks(chunks: &[Chunk]) -> Vec<i16> {
    let mut out = Vec::new();
    let mut prev_end: Option<i64> = None;
    for chunk in chunks {
        let trim_ms = prev_end.map(|pe| (pe - chunk.start_ms).max(0)).unwrap_or(0);
        let trim_samples = (trim_ms * SAMPLE_RATE as i64 / 1000) as usize;
        let start = trim_samples.min(chunk.samples.len());
        out.extend_from_slice(&chunk.samples[start..]);
        prev_end = Some(chunk.end_ms);
    }
    out
}

/// 複数音源のチャンクを、各チャンクの開始時刻に配置して加算合成する（全体を1トラックで聴く用）。
/// 省メモリのためi16バッファ上で飽和加算する（8GB級のPCでも通し録音を扱えるように）。
fn mix_chunks(chunks: &[Chunk]) -> Vec<i16> {
    let sample_at = |ms: i64| (ms.max(0) * SAMPLE_RATE as i64 / 1000) as usize;
    let total = chunks
        .iter()
        .map(|c| sample_at(c.start_ms) + c.samples.len())
        .max()
        .unwrap_or(0);
    let mut acc = vec![0i16; total];
    for chunk in chunks {
        let offset = sample_at(chunk.start_ms);
        for (i, &sample) in chunk.samples.iter().enumerate() {
            if let Some(slot) = acc.get_mut(offset + i) {
                *slot = slot.saturating_add(sample);
            }
        }
    }
    acc
}

fn load_chunks(segments: &[TranscriptSegment], source: Option<&str>) -> Vec<Chunk> {
    let mut chunks: Vec<Chunk> = Vec::new();
    for segment in segments {
        if let Some(src) = source {
            if segment.source != src {
                continue;
            }
        }
        let Some(path) = segment.audio_chunk_path.as_ref() else {
            continue;
        };
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        if let Ok(samples) = parse_wav_pcm16(&bytes) {
            chunks.push(Chunk {
                start_ms: segment.start_ms,
                end_ms: segment.end_ms,
                samples,
            });
        }
    }
    chunks.sort_by_key(|c| c.start_ms);
    chunks
}

const MAX_MIX_SAMPLES: usize = 90 * 60 * SAMPLE_RATE as usize;

fn no_audio_error() -> AppError {
    AppError::new(
        "MEETING_NO_AUDIO",
        "書き出せる録音がありません（無音のみ、または録音チャンクが見つかりません）",
        false,
    )
}

/// 会議の録音を音源ごとに1つのWAVへ書き出す。書き出したファイルパスを返す。
pub fn export_recording(
    audio_dir: &Path,
    meeting_id: &str,
    segments: &[TranscriptSegment],
) -> Result<Vec<String>, AppError> {
    let mut written = Vec::new();
    for source in ["mic", "loopback"] {
        let chunks = load_chunks(segments, Some(source));
        if chunks.is_empty() {
            continue;
        }
        let merged = merge_chunks(&chunks);
        let out_path = audio_dir.join(format!("{meeting_id}_{source}.wav"));
        std::fs::write(&out_path, write_wav_pcm16(&merged, SAMPLE_RATE))
            .map_err(|e| audio_error(format!("録音の書き出しに失敗しました: {e}")))?;
        written.push(out_path.to_string_lossy().into_owned());
    }
    if written.is_empty() {
        return Err(no_audio_error());
    }
    Ok(written)
}

/// 全音源を合成した「通し録音」を1つのWAVへ書き出し、そのパスを返す（全体再生用）。
pub fn export_mixed(
    audio_dir: &Path,
    meeting_id: &str,
    segments: &[TranscriptSegment],
) -> Result<String, AppError> {
    let chunks = load_chunks(segments, None);
    if chunks.is_empty() {
        return Err(no_audio_error());
    }
    let last_sample = chunks
        .iter()
        .map(|c| (c.start_ms.max(0) * SAMPLE_RATE as i64 / 1000) as usize + c.samples.len())
        .max()
        .unwrap_or(0);
    if last_sample > MAX_MIX_SAMPLES {
        return Err(AppError::new(
            "MEETING_AUDIO_TOO_LONG",
            "録音が長すぎるためアプリ内で通し再生できません。書き出してフォルダから再生してください",
            false,
        ));
    }
    let mixed = mix_chunks(&chunks);
    let out_path = audio_dir.join(format!("{meeting_id}_full.wav"));
    std::fs::write(&out_path, write_wav_pcm16(&mixed, SAMPLE_RATE))
        .map_err(|e| audio_error(format!("録音の書き出しに失敗しました: {e}")))?;
    Ok(out_path.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segment(source: &str, start_ms: i64, end_ms: i64, path: &str) -> TranscriptSegment {
        TranscriptSegment {
            id: "s".to_string(),
            meeting_id: "m".to_string(),
            source: source.to_string(),
            speaker_label: "x".to_string(),
            start_ms,
            end_ms,
            text: "t".to_string(),
            status: "confirmed".to_string(),
            audio_chunk_path: Some(path.to_string()),
            created_at: "t".to_string(),
        }
    }

    #[test]
    fn wavを書き出して読み戻せる() {
        let samples: Vec<i16> = vec![0, 100, -100, 32767, -32768];
        let bytes = write_wav_pcm16(&samples, SAMPLE_RATE);
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(parse_wav_pcm16(&bytes).unwrap(), samples);
    }

    #[test]
    fn 不正なwavはエラー() {
        assert!(parse_wav_pcm16(b"not a wav").is_err());
    }

    #[test]
    fn チャンク結合は重なり1秒分を先頭から取り除く() {
        // 各チャンク20000ms、stride19000msで1秒重なる。1秒=16000サンプル。
        let a = Chunk { start_ms: 0, end_ms: 20000, samples: vec![1i16; 20000 * 16] };
        let b = Chunk { start_ms: 19000, end_ms: 39000, samples: vec![2i16; 20000 * 16] };
        let merged = merge_chunks(&[a, b]);
        // aは全長(20000ms=320000)、bは先頭1000ms(16000)を除いた19000ms(304000)
        assert_eq!(merged.len(), 320000 + 304000);
        assert_eq!(merged[0], 1);
        assert_eq!(merged[320000], 2);
    }

    #[test]
    fn 重なりのないチャンクはそのまま連結する() {
        let a = Chunk { start_ms: 0, end_ms: 1000, samples: vec![1i16; 16000] };
        let b = Chunk { start_ms: 5000, end_ms: 6000, samples: vec![2i16; 16000] };
        assert_eq!(merge_chunks(&[a, b]).len(), 32000);
    }

    #[test]
    fn 音源ごとにwavを書き出す() {
        let dir = tempfile::tempdir().unwrap();
        let mic_chunk = dir.path().join("mic_0000.wav");
        std::fs::write(&mic_chunk, write_wav_pcm16(&vec![500i16; 16000], SAMPLE_RATE)).unwrap();
        let segments = vec![segment("mic", 0, 1000, &mic_chunk.to_string_lossy())];
        let written = export_recording(dir.path(), "m1", &segments).unwrap();
        assert_eq!(written.len(), 1);
        assert!(written[0].ends_with("m1_mic.wav"));
        let out = parse_wav_pcm16(&std::fs::read(&written[0]).unwrap()).unwrap();
        assert_eq!(out.len(), 16000);
    }

    #[test]
    fn 書き出せる録音がなければエラー() {
        let dir = tempfile::tempdir().unwrap();
        let err = export_recording(dir.path(), "m1", &[]).unwrap_err();
        assert_eq!(err.code, "MEETING_NO_AUDIO");
    }

    #[test]
    fn 合成は開始時刻に配置して加算する() {
        // mic: 0msから1000ms(16000サンプル)値10、loopback: 500msから1000ms値20
        let a = Chunk { start_ms: 0, end_ms: 1000, samples: vec![10i16; 16000] };
        let b = Chunk { start_ms: 500, end_ms: 1500, samples: vec![20i16; 16000] };
        let mixed = mix_chunks(&[a, b]);
        // 全長は1500ms=24000サンプル
        assert_eq!(mixed.len(), 24000);
        assert_eq!(mixed[0], 10); // 0-500msはmicのみ
        assert_eq!(mixed[8000], 30); // 500-1000msは加算 10+20
        assert_eq!(mixed[20000], 20); // 1000-1500msはloopbackのみ
    }

    #[test]
    fn 合成clampは範囲内に収める() {
        let a = Chunk { start_ms: 0, end_ms: 1, samples: vec![30000i16] };
        let b = Chunk { start_ms: 0, end_ms: 1, samples: vec![30000i16] };
        assert_eq!(mix_chunks(&[a, b])[0], i16::MAX);
    }

    #[test]
    fn 通し録音を1ファイルへ書き出す() {
        let dir = tempfile::tempdir().unwrap();
        let mic = dir.path().join("mic_0000.wav");
        let loop_ = dir.path().join("loopback_0000.wav");
        std::fs::write(&mic, write_wav_pcm16(&vec![100i16; 16000], SAMPLE_RATE)).unwrap();
        std::fs::write(&loop_, write_wav_pcm16(&vec![50i16; 16000], SAMPLE_RATE)).unwrap();
        let segments = vec![
            segment("mic", 0, 1000, &mic.to_string_lossy()),
            segment("loopback", 0, 1000, &loop_.to_string_lossy()),
        ];
        let path = export_mixed(dir.path(), "m1", &segments).unwrap();
        assert!(path.ends_with("m1_full.wav"));
        let out = parse_wav_pcm16(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(out.len(), 16000);
        assert_eq!(out[0], 150); // 加算合成
    }
}
