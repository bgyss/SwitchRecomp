use crate::ValidationStatus;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

const AUDIO_CHUNK_BYTES: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Timecode {
    pub seconds: f64,
}

impl Timecode {
    pub fn from_seconds(seconds: f64) -> Result<Self, String> {
        if seconds.is_finite() && seconds >= 0.0 {
            Ok(Self { seconds })
        } else {
            Err(format!("invalid timecode seconds: {seconds}"))
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err("timecode is empty".to_string());
        }
        if let Ok(seconds) = trimmed.parse::<f64>() {
            return Self::from_seconds(seconds);
        }
        let parts: Vec<&str> = trimmed.split(':').collect();
        if parts.len() > 3 {
            return Err(format!("timecode has too many segments: {value}"));
        }
        let mut secs = 0.0;
        let mut multiplier = 1.0;
        for (idx, part) in parts.iter().rev().enumerate() {
            if idx == 0 {
                secs += part
                    .parse::<f64>()
                    .map_err(|_| format!("invalid timecode seconds segment: {value}"))?;
            } else {
                let unit = part
                    .parse::<u64>()
                    .map_err(|_| format!("invalid timecode segment: {value}"))?;
                multiplier *= 60.0;
                secs += unit as f64 * multiplier;
            }
        }
        Self::from_seconds(secs)
    }

    pub fn to_frame_index(&self, fps: f32) -> Result<usize, String> {
        if !fps.is_finite() || fps <= 0.0 {
            return Err(format!("invalid fps: {fps}"));
        }
        let frame = (self.seconds * fps as f64).round();
        if frame < 0.0 {
            Err("timecode produced negative frame index".to_string())
        } else {
            Ok(frame as usize)
        }
    }
}

impl fmt::Display for Timecode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let total_ms = (self.seconds * 1000.0).round() as u64;
        let ms = total_ms % 1000;
        let total_secs = total_ms / 1000;
        let secs = total_secs % 60;
        let total_mins = total_secs / 60;
        let mins = total_mins % 60;
        let hours = total_mins / 60;
        write!(f, "{hours:02}:{mins:02}:{secs:02}.{ms:03}")
    }
}

impl Serialize for Timecode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Timecode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct TimecodeVisitor;

        impl serde::de::Visitor<'_> for TimecodeVisitor {
            type Value = Timecode;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("timecode string (HH:MM:SS.mmm) or seconds value")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Timecode::parse(value).map_err(E::custom)
            }

            fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Timecode::from_seconds(value).map_err(E::custom)
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Timecode::from_seconds(value as f64).map_err(E::custom)
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Timecode::from_seconds(value as f64).map_err(E::custom)
            }
        }

        deserializer.deserialize_any(TimecodeVisitor)
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct VideoSpec {
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub fps: f32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Timeline {
    pub start: Timecode,
    pub end: Timecode,
    #[serde(default)]
    pub events: Vec<TimelineEvent>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TimelineEvent {
    pub name: String,
    pub time: Timecode,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum HashFormat {
    List,
    Directory,
    File,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct HashSource {
    pub format: HashFormat,
    pub path: PathBuf,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct HashSources {
    pub frames: HashSource,
    #[serde(default)]
    pub audio: Option<HashSource>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct VideoThresholds {
    pub frame_match_ratio: f32,
    #[serde(default)]
    pub audio_match_ratio: Option<f32>,
    pub max_drift_frames: i32,
    #[serde(default)]
    pub max_dropped_frames: usize,
    #[serde(default)]
    pub max_audio_drift_chunks: Option<i32>,
}

impl Default for VideoThresholds {
    fn default() -> Self {
        Self {
            frame_match_ratio: 0.92,
            audio_match_ratio: Some(0.9),
            max_drift_frames: 3,
            max_dropped_frames: 0,
            max_audio_drift_chunks: None,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ReferenceVideoConfig {
    pub video: VideoSpec,
    pub timeline: Timeline,
    #[serde(default)]
    pub hashes: Option<HashSources>,
    #[serde(default)]
    pub thresholds: VideoThresholds,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CaptureVideoConfig {
    pub video: VideoSpec,
    pub hashes: HashSources,
}

#[derive(Debug, Serialize)]
pub struct VideoValidationReport {
    pub status: ValidationStatus,
    pub reference: VideoRunSummary,
    pub capture: VideoRunSummary,
    pub timeline: TimelineSummary,
    pub frame_comparison: HashComparisonReport,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_comparison: Option<HashComparisonReport>,
    pub drift: DriftSummary,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub failures: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct VideoRunSummary {
    pub path: String,
    pub width: u32,
    pub height: u32,
    pub fps: f32,
    pub frame_hashes: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_hashes: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct TimelineSummary {
    pub start: Timecode,
    pub end: Timecode,
    pub start_frame: usize,
    pub end_frame: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<TimelineEvent>,
}

#[derive(Debug, Serialize)]
pub struct HashComparisonReport {
    pub matched: usize,
    pub compared: usize,
    pub match_ratio: f32,
    pub threshold: f32,
    pub offset: i32,
    pub length_delta: i32,
    pub reference_total: usize,
    pub capture_total: usize,
}

#[derive(Debug, Serialize)]
pub struct DriftSummary {
    pub frame_offset: i32,
    pub frame_offset_seconds: f64,
    pub length_delta_frames: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_offset_chunks: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_length_delta_chunks: Option<i32>,
}

#[derive(Debug)]
struct Alignment {
    offset: i32,
    compared: usize,
    matched: usize,
    match_ratio: f32,
}

#[derive(Debug, Copy, Clone)]
enum HashRole {
    Frames,
    Audio,
}

pub fn run_video_validation(
    reference_path: &Path,
    capture_path: &Path,
) -> Result<VideoValidationReport, String> {
    let reference_src = fs::read_to_string(reference_path).map_err(|err| err.to_string())?;
    let capture_src = fs::read_to_string(capture_path).map_err(|err| err.to_string())?;
    let reference: ReferenceVideoConfig =
        toml::from_str(&reference_src).map_err(|err| format!("invalid reference config: {err}"))?;
    let capture: CaptureVideoConfig =
        toml::from_str(&capture_src).map_err(|err| format!("invalid capture config: {err}"))?;

    let reference_dir = reference_path
        .parent()
        .ok_or_else(|| "reference config has no parent dir".to_string())?;
    let capture_dir = capture_path
        .parent()
        .ok_or_else(|| "capture config has no parent dir".to_string())?;

    let reference_hashes = reference
        .hashes
        .clone()
        .ok_or_else(|| "reference hashes missing".to_string())?;
    let ref_frames = load_hashes(&reference_hashes.frames, reference_dir, HashRole::Frames)?;
    let ref_audio = match &reference_hashes.audio {
        Some(source) => Some(load_hashes(source, reference_dir, HashRole::Audio)?),
        None => None,
    };

    let capture_frames = load_hashes(&capture.hashes.frames, capture_dir, HashRole::Frames)?;
    let capture_audio = match &capture.hashes.audio {
        Some(source) => Some(load_hashes(source, capture_dir, HashRole::Audio)?),
        None => None,
    };

    let timeline_start = reference
        .timeline
        .start
        .to_frame_index(reference.video.fps)?;
    let timeline_end = reference.timeline.end.to_frame_index(reference.video.fps)?;
    if timeline_end <= timeline_start {
        return Err("timeline end must be after start".to_string());
    }
    if timeline_start >= ref_frames.len() {
        return Err("timeline start beyond reference frame hashes".to_string());
    }

    let mut failures = Vec::new();
    let clamped_end = timeline_end.min(ref_frames.len());
    if timeline_end > ref_frames.len() {
        failures.push(format!(
            "reference frame hashes cover {}, timeline ends at {}",
            ref_frames.len(),
            timeline_end
        ));
    }

    if reference.video.width != capture.video.width
        || reference.video.height != capture.video.height
    {
        failures.push(format!(
            "resolution mismatch: reference {}x{}, capture {}x{}",
            reference.video.width,
            reference.video.height,
            capture.video.width,
            capture.video.height
        ));
    }
    if (reference.video.fps - capture.video.fps).abs() > f32::EPSILON {
        failures.push(format!(
            "fps mismatch: reference {:.3}, capture {:.3}",
            reference.video.fps, capture.video.fps
        ));
    }

    let ref_slice = &ref_frames[timeline_start..clamped_end];
    let max_drift = reference.thresholds.max_drift_frames;
    let alignment = best_alignment(ref_slice, &capture_frames, max_drift);
    let length_delta = capture_frames.len() as i32 - ref_slice.len() as i32;
    let frame_match_ratio = if alignment.compared == 0 {
        0.0
    } else {
        alignment.match_ratio
    };
    if frame_match_ratio < reference.thresholds.frame_match_ratio {
        failures.push(format!(
            "frame match ratio {:.3} below threshold {:.3}",
            frame_match_ratio, reference.thresholds.frame_match_ratio
        ));
    }
    if alignment.offset.abs() > reference.thresholds.max_drift_frames {
        failures.push(format!(
            "frame drift {} exceeds max {}",
            alignment.offset, reference.thresholds.max_drift_frames
        ));
    }
    let length_delta_abs = length_delta.unsigned_abs() as usize;
    if length_delta_abs > reference.thresholds.max_dropped_frames {
        failures.push(format!(
            "frame length delta {} exceeds max dropped {}",
            length_delta, reference.thresholds.max_dropped_frames
        ));
    }

    let audio_report = match (ref_audio.as_ref(), capture_audio.as_ref()) {
        (Some(reference_audio), Some(capture_audio)) => {
            let max_audio_drift = reference
                .thresholds
                .max_audio_drift_chunks
                .unwrap_or(reference.thresholds.max_drift_frames);
            let audio_alignment = best_alignment(reference_audio, capture_audio, max_audio_drift);
            let audio_length_delta = capture_audio.len() as i32 - reference_audio.len() as i32;
            let audio_match_ratio = if audio_alignment.compared == 0 {
                0.0
            } else {
                audio_alignment.match_ratio
            };
            if let Some(threshold) = reference.thresholds.audio_match_ratio {
                if audio_match_ratio < threshold {
                    failures.push(format!(
                        "audio match ratio {:.3} below threshold {:.3}",
                        audio_match_ratio, threshold
                    ));
                }
            }
            if audio_alignment.offset.abs() > max_audio_drift {
                failures.push(format!(
                    "audio drift {} exceeds max {}",
                    audio_alignment.offset, max_audio_drift
                ));
            }
            Some(HashComparisonReport {
                matched: audio_alignment.matched,
                compared: audio_alignment.compared,
                match_ratio: audio_match_ratio,
                threshold: reference.thresholds.audio_match_ratio.unwrap_or(0.0),
                offset: audio_alignment.offset,
                length_delta: audio_length_delta,
                reference_total: reference_audio.len(),
                capture_total: capture_audio.len(),
            })
        }
        (None, None) => None,
        _ => {
            failures.push("audio hashes missing on one side".to_string());
            None
        }
    };

    let status = if failures.is_empty() {
        ValidationStatus::Passed
    } else {
        ValidationStatus::Failed
    };

    let drift = DriftSummary {
        frame_offset: alignment.offset,
        frame_offset_seconds: alignment.offset as f64 / reference.video.fps as f64,
        length_delta_frames: length_delta,
        audio_offset_chunks: audio_report.as_ref().map(|report| report.offset),
        audio_length_delta_chunks: audio_report.as_ref().map(|report| report.length_delta),
    };

    let frame_report = HashComparisonReport {
        matched: alignment.matched,
        compared: alignment.compared,
        match_ratio: frame_match_ratio,
        threshold: reference.thresholds.frame_match_ratio,
        offset: alignment.offset,
        length_delta,
        reference_total: ref_slice.len(),
        capture_total: capture_frames.len(),
    };

    Ok(VideoValidationReport {
        status,
        reference: VideoRunSummary {
            path: reference.video.path.display().to_string(),
            width: reference.video.width,
            height: reference.video.height,
            fps: reference.video.fps,
            frame_hashes: ref_frames.len(),
            audio_hashes: ref_audio.as_ref().map(|items| items.len()),
        },
        capture: VideoRunSummary {
            path: capture.video.path.display().to_string(),
            width: capture.video.width,
            height: capture.video.height,
            fps: capture.video.fps,
            frame_hashes: capture_frames.len(),
            audio_hashes: capture_audio.as_ref().map(|items| items.len()),
        },
        timeline: TimelineSummary {
            start: reference.timeline.start,
            end: reference.timeline.end,
            start_frame: timeline_start,
            end_frame: clamped_end,
            events: reference.timeline.events.clone(),
        },
        frame_comparison: frame_report,
        audio_comparison: audio_report,
        drift,
        failures,
    })
}

pub fn hash_frames_dir(path: &Path) -> Result<Vec<String>, String> {
    load_dir_hashes(path)
}

pub fn hash_audio_file(path: &Path) -> Result<Vec<String>, String> {
    load_file_hashes(path)
}

pub fn write_hash_list(path: &Path, hashes: &[String]) -> Result<(), String> {
    if hashes.is_empty() {
        return Err("hash list is empty".to_string());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("create hash list dir {}: {err}", parent.display()))?;
    }
    let mut output = String::new();
    for hash in hashes {
        output.push_str(hash);
        output.push('\n');
    }
    fs::write(path, output).map_err(|err| format!("write hash list {}: {err}", path.display()))
}

fn best_alignment(reference: &[String], capture: &[String], max_offset: i32) -> Alignment {
    let mut best = Alignment {
        offset: 0,
        compared: 0,
        matched: 0,
        match_ratio: 0.0,
    };

    for offset in -max_offset..=max_offset {
        let mut matched = 0;
        let mut compared = 0;
        for (idx, reference_hash) in reference.iter().enumerate() {
            let capture_idx = idx as i32 + offset;
            if capture_idx < 0 || capture_idx >= capture.len() as i32 {
                continue;
            }
            compared += 1;
            if reference_hash == &capture[capture_idx as usize] {
                matched += 1;
            }
        }
        if compared == 0 {
            continue;
        }
        let ratio = matched as f32 / compared as f32;
        let ordering = ratio
            .partial_cmp(&best.match_ratio)
            .unwrap_or(Ordering::Less);
        let better = match ordering {
            Ordering::Greater => true,
            Ordering::Equal => {
                if compared > best.compared {
                    true
                } else {
                    let offset_abs = offset.abs();
                    let best_abs = best.offset.abs();
                    compared == best.compared && offset_abs < best_abs
                }
            }
            Ordering::Less => false,
        };
        if better {
            best = Alignment {
                offset,
                compared,
                matched,
                match_ratio: ratio,
            };
        }
    }

    best
}

fn load_hashes(
    source: &HashSource,
    base_dir: &Path,
    role: HashRole,
) -> Result<Vec<String>, String> {
    let resolved = resolve_path(base_dir, &source.path);
    match source.format {
        HashFormat::List => load_hash_list(&resolved),
        HashFormat::Directory => match role {
            HashRole::Frames => load_dir_hashes(&resolved),
            HashRole::Audio => Err("audio hashes do not support directory format".to_string()),
        },
        HashFormat::File => match role {
            HashRole::Audio => load_file_hashes(&resolved),
            HashRole::Frames => Err("frame hashes do not support file format".to_string()),
        },
    }
}

fn resolve_path(base_dir: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_dir.join(path)
    }
}

fn load_hash_list(path: &Path) -> Result<Vec<String>, String> {
    let content = fs::read_to_string(path)
        .map_err(|err| format!("read hash list {}: {err}", path.display()))?;
    let mut hashes = Vec::new();
    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        let hash = match parts.len() {
            1 => parts[0],
            2 => parts[1],
            _ => {
                return Err(format!(
                    "invalid hash list entry at {}:{}",
                    path.display(),
                    line_num + 1
                ))
            }
        };
        hashes.push(hash.to_string());
    }
    if hashes.is_empty() {
        return Err(format!("hash list {} is empty", path.display()));
    }
    Ok(hashes)
}

fn load_dir_hashes(path: &Path) -> Result<Vec<String>, String> {
    let mut entries: Vec<PathBuf> = fs::read_dir(path)
        .map_err(|err| format!("read hash dir {}: {err}", path.display()))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|entry| entry.is_file())
        .collect();
    entries.sort();
    if entries.is_empty() {
        return Err(format!("hash dir {} is empty", path.display()));
    }
    let mut hashes = Vec::new();
    for entry in entries {
        let bytes =
            fs::read(&entry).map_err(|err| format!("read hash file {}: {err}", entry.display()))?;
        hashes.push(sha256_bytes(&bytes));
    }
    Ok(hashes)
}

fn load_file_hashes(path: &Path) -> Result<Vec<String>, String> {
    let bytes =
        fs::read(path).map_err(|err| format!("read hash file {}: {err}", path.display()))?;
    if bytes.is_empty() {
        return Err(format!("hash file {} is empty", path.display()));
    }
    let mut hashes = Vec::new();
    for chunk in bytes.chunks(AUDIO_CHUNK_BYTES) {
        hashes.push(sha256_bytes(chunk));
    }
    Ok(hashes)
}

fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    format!("{:x}", digest)
}
