use crate::{ValidationCase, ValidationReport, ValidationStatus};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

#[derive(Debug)]
pub struct VideoValidationPaths {
    pub reference_config: PathBuf,
    pub test_video: Option<PathBuf>,
    pub summary_path: Option<PathBuf>,
    pub out_dir: PathBuf,
    pub scripts_dir: Option<PathBuf>,
    pub thresholds_path: Option<PathBuf>,
    pub event_observations: Option<PathBuf>,
    pub strict: bool,
    pub python: Option<PathBuf>,
}

#[derive(Debug, Serialize)]
pub struct VideoValidationSummary {
    pub label: String,
    pub summary_path: String,
    pub thresholds: ValidationThresholds,
    pub checks: Vec<MetricCheck>,
    pub status: ValidationStatus,
    pub failures: usize,
    pub drift: DriftSummary,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    Pass,
    Fail,
    Missing,
}

#[derive(Debug, Serialize)]
pub struct MetricCheck {
    pub metric: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
    pub threshold: f64,
    pub status: CheckStatus,
}

#[derive(Debug, Serialize, Clone)]
pub struct ValidationThresholds {
    pub ssim_min: f64,
    pub psnr_min: f64,
    pub vmaf_min: f64,
    pub audio_lufs_delta_max: f64,
    pub audio_peak_delta_max: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_drift_max_seconds: Option<f64>,
}

impl Default for ValidationThresholds {
    fn default() -> Self {
        Self {
            ssim_min: 0.95,
            psnr_min: 35.0,
            vmaf_min: 90.0,
            audio_lufs_delta_max: 2.0,
            audio_peak_delta_max: 2.0,
            event_drift_max_seconds: None,
        }
    }
}

#[derive(Debug, Default, Clone)]
struct ThresholdOverrides {
    ssim_min: Option<f64>,
    psnr_min: Option<f64>,
    vmaf_min: Option<f64>,
    audio_lufs_delta_max: Option<f64>,
    audio_peak_delta_max: Option<f64>,
    event_drift_max_seconds: Option<f64>,
}

impl ValidationThresholds {
    fn apply_overrides(&mut self, overrides: ThresholdOverrides) {
        if let Some(value) = overrides.ssim_min {
            self.ssim_min = value;
        }
        if let Some(value) = overrides.psnr_min {
            self.psnr_min = value;
        }
        if let Some(value) = overrides.vmaf_min {
            self.vmaf_min = value;
        }
        if let Some(value) = overrides.audio_lufs_delta_max {
            self.audio_lufs_delta_max = value;
        }
        if let Some(value) = overrides.audio_peak_delta_max {
            self.audio_peak_delta_max = value;
        }
        if overrides.event_drift_max_seconds.is_some() {
            self.event_drift_max_seconds = overrides.event_drift_max_seconds;
        }
    }
}

#[derive(Debug)]
struct ReferenceVideo {
    label: String,
    reference_video: PathBuf,
    expected: ExpectedVideo,
    comparison: ComparisonSettings,
    thresholds: ThresholdOverrides,
    events: Vec<ReferenceEvent>,
}

#[derive(Debug, Default)]
pub struct ExpectedVideo {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fps: Option<f64>,
    pub audio_rate: Option<u32>,
}

#[derive(Debug, Default)]
pub struct ComparisonSettings {
    pub offset_seconds: f64,
    pub trim_start_seconds: f64,
    pub duration_seconds: Option<f64>,
    pub no_vmaf: bool,
    pub thresholds_path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct ReferenceEvent {
    pub id: String,
    pub label: Option<String>,
    pub timecode: String,
    pub seconds: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DriftStatus {
    Observed,
    Missing,
}

#[derive(Debug, Serialize)]
pub struct DriftEvent {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub expected_timecode: String,
    pub expected_seconds: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed_timecode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed_seconds: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drift_seconds: Option<f64>,
    pub status: DriftStatus,
}

#[derive(Debug, Serialize)]
pub struct DriftSummary {
    pub events: Vec<DriftEvent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_abs_drift_seconds: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub average_abs_drift_seconds: Option<f64>,
    pub missing_events: usize,
}

#[derive(Debug, Deserialize)]
struct AvSummary {
    label: Option<String>,
    video: Option<SummaryVideo>,
    audio: Option<SummaryAudio>,
    events: Option<Vec<SummaryEvent>>,
}

#[derive(Debug, Deserialize)]
struct SummaryVideo {
    ssim: Option<SummaryMetric>,
    psnr: Option<SummaryMetric>,
    vmaf: Option<SummaryVmaf>,
}

#[derive(Debug, Deserialize)]
struct SummaryMetric {
    average: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct SummaryVmaf {
    average: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct SummaryAudio {
    reference: Option<SummaryAudioMetric>,
    test: Option<SummaryAudioMetric>,
}

#[derive(Debug, Deserialize)]
struct SummaryAudioMetric {
    integrated_lufs: Option<f64>,
    true_peak_dbtp: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct SummaryEvent {
    id: String,
    observed_timecode: Option<String>,
    observed_seconds: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct EventObservationFile {
    schema_version: Option<String>,
    observations: Vec<EventObservationEntry>,
}

#[derive(Debug, Deserialize)]
struct EventObservationEntry {
    id: String,
    observed_timecode: Option<String>,
    observed_seconds: Option<f64>,
}

pub fn run_video_validation(paths: VideoValidationPaths) -> Result<ValidationReport, String> {
    let start = Instant::now();
    let reference = load_reference_video_toml(&paths.reference_config)?;

    let summary_path = if let Some(summary_path) = paths.summary_path.clone() {
        summary_path
    } else {
        let test_video = paths
            .test_video
            .clone()
            .ok_or_else(|| "test video is required when summary is not provided".to_string())?;
        let scripts_dir = paths
            .scripts_dir
            .clone()
            .unwrap_or_else(default_scripts_dir);
        let python = paths
            .python
            .clone()
            .unwrap_or_else(|| PathBuf::from("python3"));
        run_compare_av(
            &python,
            &scripts_dir,
            &reference,
            &test_video,
            &paths.out_dir,
        )?
    };

    let summary = load_summary(&summary_path)?;
    let thresholds = resolve_thresholds(&reference, paths.thresholds_path.as_deref())?;

    let observations = if let Some(path) = &paths.event_observations {
        Some(load_event_observations(path)?)
    } else {
        None
    };

    let video_summary = evaluate_summary(
        &summary,
        &summary_path,
        &reference,
        thresholds,
        paths.strict,
        observations.as_ref(),
    )?;

    let status = video_summary.status;
    let duration_ms = start.elapsed().as_millis();
    let case = ValidationCase {
        name: "video_av_compare".to_string(),
        status,
        duration_ms,
        details: Some(format!("summary: {}", summary_path.display())),
    };

    let (passed, failed) = match status {
        ValidationStatus::Passed => (1, 0),
        ValidationStatus::Failed => (0, 1),
    };

    Ok(ValidationReport {
        generated_at: chrono_stamp(),
        total: 1,
        passed,
        failed,
        cases: vec![case],
        video: Some(video_summary),
    })
}

fn load_summary(path: &Path) -> Result<AvSummary, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|err| format!("read summary {}: {err}", path.display()))?;
    serde_json::from_str(&text).map_err(|err| format!("parse summary json: {err}"))
}

fn resolve_thresholds(
    reference: &ReferenceVideo,
    thresholds_path: Option<&Path>,
) -> Result<ValidationThresholds, String> {
    let mut thresholds = ValidationThresholds::default();

    if let Some(path) = reference.comparison.thresholds_path.as_deref() {
        thresholds.apply_overrides(load_thresholds_json(path)?);
    }
    if let Some(path) = thresholds_path {
        thresholds.apply_overrides(load_thresholds_json(path)?);
    }
    thresholds.apply_overrides(reference.thresholds.clone());

    Ok(thresholds)
}

fn load_thresholds_json(path: &Path) -> Result<ThresholdOverrides, String> {
    let text = std::fs::read_to_string(path).map_err(|err| format!("read thresholds: {err}"))?;
    #[derive(Deserialize, Default)]
    struct ThresholdFile {
        ssim_min: Option<f64>,
        psnr_min: Option<f64>,
        vmaf_min: Option<f64>,
        audio_lufs_delta_max: Option<f64>,
        audio_peak_delta_max: Option<f64>,
        event_drift_max_seconds: Option<f64>,
    }
    let parsed: ThresholdFile =
        serde_json::from_str(&text).map_err(|err| format!("parse thresholds json: {err}"))?;
    Ok(ThresholdOverrides {
        ssim_min: parsed.ssim_min,
        psnr_min: parsed.psnr_min,
        vmaf_min: parsed.vmaf_min,
        audio_lufs_delta_max: parsed.audio_lufs_delta_max,
        audio_peak_delta_max: parsed.audio_peak_delta_max,
        event_drift_max_seconds: parsed.event_drift_max_seconds,
    })
}

fn evaluate_summary(
    summary: &AvSummary,
    summary_path: &Path,
    reference: &ReferenceVideo,
    thresholds: ValidationThresholds,
    strict: bool,
    observations: Option<&EventObservationFile>,
) -> Result<VideoValidationSummary, String> {
    let label = summary
        .label
        .clone()
        .unwrap_or_else(|| reference.label.clone());

    let ssim_avg = summary
        .video
        .as_ref()
        .and_then(|video| video.ssim.as_ref())
        .and_then(|metric| metric.average);
    let psnr_avg = summary
        .video
        .as_ref()
        .and_then(|video| video.psnr.as_ref())
        .and_then(|metric| metric.average);
    let vmaf_avg = summary
        .video
        .as_ref()
        .and_then(|video| video.vmaf.as_ref())
        .and_then(|metric| metric.average);

    let (ref_lufs, test_lufs, ref_peak, test_peak) = extract_audio_metrics(summary);
    let lufs_delta = ref_lufs.zip(test_lufs).map(|(a, b)| (a - b).abs());
    let peak_delta = ref_peak.zip(test_peak).map(|(a, b)| (a - b).abs());

    let mut checks = Vec::new();
    let mut failures = 0usize;

    check_min(
        "ssim_avg",
        ssim_avg,
        thresholds.ssim_min,
        strict,
        &mut checks,
        &mut failures,
    );
    check_min(
        "psnr_avg",
        psnr_avg,
        thresholds.psnr_min,
        strict,
        &mut checks,
        &mut failures,
    );

    if vmaf_avg.is_some() {
        check_min(
            "vmaf_avg",
            vmaf_avg,
            thresholds.vmaf_min,
            strict,
            &mut checks,
            &mut failures,
        );
    } else {
        checks.push(MetricCheck {
            metric: "vmaf_avg".to_string(),
            value: None,
            threshold: thresholds.vmaf_min,
            status: CheckStatus::Missing,
        });
        if strict {
            failures += 1;
        }
    }

    check_max(
        "audio_lufs_delta",
        lufs_delta,
        thresholds.audio_lufs_delta_max,
        strict,
        &mut checks,
        &mut failures,
    );
    check_max(
        "audio_peak_delta",
        peak_delta,
        thresholds.audio_peak_delta_max,
        strict,
        &mut checks,
        &mut failures,
    );

    let (drift_summary, drift_max) = build_drift_summary(summary, reference, observations)?;

    if let Some(max_allowed) = thresholds.event_drift_max_seconds {
        check_max(
            "event_drift_max_seconds",
            drift_max,
            max_allowed,
            strict,
            &mut checks,
            &mut failures,
        );
    }

    let status = if failures > 0 {
        ValidationStatus::Failed
    } else {
        ValidationStatus::Passed
    };

    Ok(VideoValidationSummary {
        label,
        summary_path: summary_path.display().to_string(),
        thresholds,
        checks,
        status,
        failures,
        drift: drift_summary,
    })
}

fn extract_audio_metrics(
    summary: &AvSummary,
) -> (Option<f64>, Option<f64>, Option<f64>, Option<f64>) {
    let ref_audio = summary
        .audio
        .as_ref()
        .and_then(|audio| audio.reference.as_ref());
    let test_audio = summary.audio.as_ref().and_then(|audio| audio.test.as_ref());

    let ref_lufs = ref_audio.and_then(|metric| metric.integrated_lufs);
    let test_lufs = test_audio.and_then(|metric| metric.integrated_lufs);
    let ref_peak = ref_audio.and_then(|metric| metric.true_peak_dbtp);
    let test_peak = test_audio.and_then(|metric| metric.true_peak_dbtp);

    (ref_lufs, test_lufs, ref_peak, test_peak)
}

fn check_min(
    label: &str,
    value: Option<f64>,
    threshold: f64,
    strict: bool,
    checks: &mut Vec<MetricCheck>,
    failures: &mut usize,
) {
    let status = match value {
        None => {
            if strict {
                *failures += 1;
            }
            CheckStatus::Missing
        }
        Some(current) if current < threshold => {
            *failures += 1;
            CheckStatus::Fail
        }
        Some(_) => CheckStatus::Pass,
    };

    checks.push(MetricCheck {
        metric: label.to_string(),
        value,
        threshold,
        status,
    });
}

fn check_max(
    label: &str,
    value: Option<f64>,
    threshold: f64,
    strict: bool,
    checks: &mut Vec<MetricCheck>,
    failures: &mut usize,
) {
    let status = match value {
        None => {
            if strict {
                *failures += 1;
            }
            CheckStatus::Missing
        }
        Some(current) if current > threshold => {
            *failures += 1;
            CheckStatus::Fail
        }
        Some(_) => CheckStatus::Pass,
    };

    checks.push(MetricCheck {
        metric: label.to_string(),
        value,
        threshold,
        status,
    });
}

fn build_drift_summary(
    summary: &AvSummary,
    reference: &ReferenceVideo,
    observations: Option<&EventObservationFile>,
) -> Result<(DriftSummary, Option<f64>), String> {
    let observed = if let Some(observations) = observations {
        build_observation_map(&observations.observations)?
    } else if let Some(events) = &summary.events {
        build_observation_map(events)?
    } else {
        BTreeMap::new()
    };

    let mut events = Vec::new();
    let mut drift_values = Vec::new();
    let mut missing = 0usize;

    for reference_event in &reference.events {
        let observed_entry = observed.get(&reference_event.id);
        let (observed_timecode, observed_seconds) = match observed_entry {
            Some(entry) => (entry.timecode.clone(), Some(entry.seconds)),
            None => (None, None),
        };

        let drift_seconds = observed_seconds.map(|seconds| seconds - reference_event.seconds);
        if let Some(drift) = drift_seconds {
            drift_values.push(drift.abs());
        } else {
            missing += 1;
        }

        events.push(DriftEvent {
            id: reference_event.id.clone(),
            label: reference_event.label.clone(),
            expected_timecode: reference_event.timecode.clone(),
            expected_seconds: reference_event.seconds,
            observed_timecode,
            observed_seconds,
            drift_seconds,
            status: if observed_seconds.is_some() {
                DriftStatus::Observed
            } else {
                DriftStatus::Missing
            },
        });
    }

    let max_abs_drift_seconds = drift_values
        .iter()
        .cloned()
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let average_abs_drift_seconds = if drift_values.is_empty() {
        None
    } else {
        Some(drift_values.iter().sum::<f64>() / drift_values.len() as f64)
    };

    let summary = DriftSummary {
        events,
        max_abs_drift_seconds,
        average_abs_drift_seconds,
        missing_events: missing,
    };

    Ok((summary, max_abs_drift_seconds))
}

struct ObservedEvent {
    timecode: Option<String>,
    seconds: f64,
}

fn build_observation_map<T>(entries: &[T]) -> Result<BTreeMap<String, ObservedEvent>, String>
where
    T: ObservationLike,
{
    let mut map = BTreeMap::new();
    for entry in entries {
        let id = entry.id().to_string();
        let timecode = entry.observed_timecode().map(|value| value.to_string());
        let seconds = if let Some(value) = entry.observed_seconds() {
            value
        } else if let Some(value) = entry.observed_timecode() {
            parse_timecode_to_seconds(value)?
        } else {
            return Err(format!(
                "event {id} missing observed_seconds or observed_timecode"
            ));
        };
        map.insert(id, ObservedEvent { timecode, seconds });
    }
    Ok(map)
}

trait ObservationLike {
    fn id(&self) -> &str;
    fn observed_timecode(&self) -> Option<&str>;
    fn observed_seconds(&self) -> Option<f64>;
}

impl ObservationLike for EventObservationEntry {
    fn id(&self) -> &str {
        &self.id
    }

    fn observed_timecode(&self) -> Option<&str> {
        self.observed_timecode.as_deref()
    }

    fn observed_seconds(&self) -> Option<f64> {
        self.observed_seconds
    }
}

impl ObservationLike for SummaryEvent {
    fn id(&self) -> &str {
        &self.id
    }

    fn observed_timecode(&self) -> Option<&str> {
        self.observed_timecode.as_deref()
    }

    fn observed_seconds(&self) -> Option<f64> {
        self.observed_seconds
    }
}

fn load_reference_video_toml(path: &Path) -> Result<ReferenceVideo, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|err| format!("read reference video config {}: {err}", path.display()))?;
    parse_reference_video_toml(&text).map_err(|err| format!("{} ({})", err, path.display()))
}

fn parse_reference_video_toml(text: &str) -> Result<ReferenceVideo, String> {
    #[derive(Copy, Clone)]
    enum Section {
        Root,
        Expected,
        Comparison,
        Thresholds,
        Event,
    }

    let mut section = Section::Root;
    let mut schema_version = None;
    let mut label = None;
    let mut reference_video = None;
    let mut expected = ExpectedVideo::default();
    let mut comparison = ComparisonSettings::default();
    let mut thresholds = ThresholdOverrides::default();
    let mut events: Vec<ReferenceEvent> = Vec::new();
    let mut current_event: Option<ReferenceEvent> = None;

    for (index, raw_line) in text.lines().enumerate() {
        let mut line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((prefix, _)) = line.split_once('#') {
            line = prefix.trim();
        }
        if line.is_empty() {
            continue;
        }

        if line.starts_with("[[") && line.ends_with("]]") {
            if let Some(event) = current_event.take() {
                events.push(event);
            }
            let name = &line[2..line.len() - 2];
            if name.trim() != "events" {
                return Err(format!("unsupported table array [{}]", name.trim()));
            }
            section = Section::Event;
            current_event = Some(ReferenceEvent {
                id: String::new(),
                label: None,
                timecode: String::new(),
                seconds: 0.0,
            });
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            if let Some(event) = current_event.take() {
                events.push(event);
            }
            let name = &line[1..line.len() - 1];
            section = match name.trim() {
                "expected" => Section::Expected,
                "comparison" => Section::Comparison,
                "thresholds" => Section::Thresholds,
                other => {
                    return Err(format!("unsupported section [{}]", other));
                }
            };
            continue;
        }

        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| format!("line {} missing '='", index + 1))?;
        let key = key.trim();
        let value = value.trim();

        match section {
            Section::Root => match key {
                "schema_version" => schema_version = Some(parse_string(value)?),
                "label" => label = Some(parse_string(value)?),
                "reference_video" => reference_video = Some(PathBuf::from(parse_string(value)?)),
                _ => return Err(format!("unknown root key '{}'", key)),
            },
            Section::Expected => match key {
                "width" => expected.width = Some(parse_u32(value)?),
                "height" => expected.height = Some(parse_u32(value)?),
                "fps" => expected.fps = Some(parse_f64(value)?),
                "audio_rate" => expected.audio_rate = Some(parse_u32(value)?),
                _ => return Err(format!("unknown expected key '{}'", key)),
            },
            Section::Comparison => match key {
                "offset_seconds" => comparison.offset_seconds = parse_f64(value)?,
                "trim_start_seconds" => comparison.trim_start_seconds = parse_f64(value)?,
                "duration_seconds" => comparison.duration_seconds = Some(parse_f64(value)?),
                "no_vmaf" => comparison.no_vmaf = parse_bool(value)?,
                "thresholds_path" => {
                    comparison.thresholds_path = Some(PathBuf::from(parse_string(value)?))
                }
                _ => return Err(format!("unknown comparison key '{}'", key)),
            },
            Section::Thresholds => match key {
                "ssim_min" => thresholds.ssim_min = Some(parse_f64(value)?),
                "psnr_min" => thresholds.psnr_min = Some(parse_f64(value)?),
                "vmaf_min" => thresholds.vmaf_min = Some(parse_f64(value)?),
                "audio_lufs_delta_max" => thresholds.audio_lufs_delta_max = Some(parse_f64(value)?),
                "audio_peak_delta_max" => thresholds.audio_peak_delta_max = Some(parse_f64(value)?),
                "event_drift_max_seconds" => {
                    thresholds.event_drift_max_seconds = Some(parse_f64(value)?)
                }
                _ => return Err(format!("unknown thresholds key '{}'", key)),
            },
            Section::Event => {
                let event = current_event
                    .as_mut()
                    .ok_or_else(|| "event entry missing".to_string())?;
                match key {
                    "id" => event.id = parse_string(value)?,
                    "label" => event.label = Some(parse_string(value)?),
                    "timecode" => {
                        let timecode = parse_string(value)?;
                        let seconds = parse_timecode_to_seconds(&timecode)?;
                        event.timecode = timecode;
                        event.seconds = seconds;
                    }
                    _ => return Err(format!("unknown event key '{}'", key)),
                }
            }
        }
    }

    if let Some(event) = current_event.take() {
        events.push(event);
    }

    let schema_version = schema_version.ok_or_else(|| "schema_version is required".to_string())?;
    if schema_version != "v1" {
        return Err(format!("unsupported schema_version '{}'", schema_version));
    }

    let label = label.unwrap_or_else(|| "reference-video".to_string());
    let reference_video =
        reference_video.ok_or_else(|| "reference_video is required".to_string())?;

    for event in &events {
        if event.id.is_empty() {
            return Err("event id is required".to_string());
        }
        if event.timecode.is_empty() {
            return Err(format!("event {} missing timecode", event.id));
        }
    }

    Ok(ReferenceVideo {
        label,
        reference_video,
        expected,
        comparison,
        thresholds,
        events,
    })
}

fn parse_string(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.starts_with('"') && value.ends_with('"') {
        Ok(value[1..value.len() - 1].to_string())
    } else {
        Err(format!("expected quoted string, got '{}'", value))
    }
}

fn parse_u32(value: &str) -> Result<u32, String> {
    value
        .trim()
        .parse::<u32>()
        .map_err(|err| format!("invalid integer '{}': {err}", value))
}

fn parse_f64(value: &str) -> Result<f64, String> {
    value
        .trim()
        .parse::<f64>()
        .map_err(|err| format!("invalid number '{}': {err}", value))
}

fn parse_bool(value: &str) -> Result<bool, String> {
    match value.trim() {
        "true" => Ok(true),
        "false" => Ok(false),
        other => Err(format!("invalid bool '{}'", other)),
    }
}

fn parse_timecode_to_seconds(value: &str) -> Result<f64, String> {
    let parts: Vec<&str> = value.split(':').collect();
    if parts.is_empty() || parts.len() > 3 {
        return Err(format!("invalid timecode '{}'", value));
    }

    let mut seconds = 0.0;
    let mut multiplier = 1.0;
    for part in parts.iter().rev() {
        let component: f64 = part
            .trim()
            .parse()
            .map_err(|err| format!("invalid timecode segment '{}': {err}", part))?;
        seconds += component * multiplier;
        multiplier *= 60.0;
    }
    Ok(seconds)
}

fn default_scripts_dir() -> PathBuf {
    if let Ok(home) = std::env::var("CODEX_HOME") {
        PathBuf::from(home).join("skills/static-recomp-av-compare/scripts")
    } else {
        PathBuf::from("skills/static-recomp-av-compare/scripts")
    }
}

fn run_compare_av(
    python: &Path,
    scripts_dir: &Path,
    reference: &ReferenceVideo,
    test_video: &Path,
    out_dir: &Path,
) -> Result<PathBuf, String> {
    let compare_script = scripts_dir.join("compare_av.py");
    let compare_out_dir = out_dir.join("av-compare");
    std::fs::create_dir_all(&compare_out_dir)
        .map_err(|err| format!("create compare dir: {err}"))?;

    let mut cmd = Command::new(python);
    cmd.arg(compare_script)
        .arg("--ref")
        .arg(&reference.reference_video)
        .arg("--test")
        .arg(test_video)
        .arg("--out-dir")
        .arg(&compare_out_dir)
        .arg("--label")
        .arg(&reference.label);

    if let Some(width) = reference.expected.width {
        cmd.arg("--width").arg(width.to_string());
    }
    if let Some(height) = reference.expected.height {
        cmd.arg("--height").arg(height.to_string());
    }
    if let Some(fps) = reference.expected.fps {
        cmd.arg("--fps").arg(fps.to_string());
    }
    if let Some(audio_rate) = reference.expected.audio_rate {
        cmd.arg("--audio-rate").arg(audio_rate.to_string());
    }
    if reference.comparison.offset_seconds != 0.0 {
        cmd.arg("--offset")
            .arg(reference.comparison.offset_seconds.to_string());
    }
    if reference.comparison.trim_start_seconds != 0.0 {
        cmd.arg("--trim-start")
            .arg(reference.comparison.trim_start_seconds.to_string());
    }
    if let Some(duration) = reference.comparison.duration_seconds {
        cmd.arg("--duration").arg(duration.to_string());
    }
    if reference.comparison.no_vmaf {
        cmd.arg("--no-vmaf");
    }

    let status = cmd
        .status()
        .map_err(|err| format!("run compare_av.py: {err}"))?;
    if !status.success() {
        return Err(format!("compare_av.py failed with status {}", status));
    }

    Ok(compare_out_dir.join("summary.json"))
}

fn load_event_observations(path: &Path) -> Result<EventObservationFile, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|err| format!("read event observations {}: {err}", path.display()))?;
    let parsed: EventObservationFile =
        serde_json::from_str(&text).map_err(|err| format!("parse event observations: {err}"))?;
    if let Some(schema_version) = parsed.schema_version.as_deref() {
        if schema_version != "v1" {
            return Err(format!(
                "unsupported event observation schema '{}'",
                schema_version
            ));
        }
    }
    Ok(parsed)
}

fn chrono_stamp() -> String {
    let now = std::time::SystemTime::now();
    let secs = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{secs}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo_root() -> PathBuf {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        manifest_dir
            .parent()
            .and_then(|path| path.parent())
            .unwrap_or(&manifest_dir)
            .to_path_buf()
    }

    #[test]
    fn parse_reference_video_config() {
        let path = repo_root().join("samples/validation/reference_video.toml");
        let reference = load_reference_video_toml(&path).expect("load reference config");
        assert_eq!(reference.label, "sample-first-level");
        assert_eq!(reference.expected.width, Some(1920));
        assert_eq!(reference.expected.height, Some(1080));
        assert_eq!(reference.expected.fps, Some(60.0));
        assert_eq!(reference.events.len(), 3);
    }

    #[test]
    fn summary_passes_with_observations() {
        let root = repo_root();
        let summary_path = root.join("samples/validation/summary_pass.json");
        let summary = load_summary(&summary_path).expect("load summary");
        let reference =
            load_reference_video_toml(&root.join("samples/validation/reference_video.toml"))
                .expect("reference");
        let thresholds = resolve_thresholds(&reference, None).expect("thresholds");
        let observations =
            load_event_observations(&root.join("samples/validation/event_observations.json"))
                .expect("observations");

        let report = evaluate_summary(
            &summary,
            &summary_path,
            &reference,
            thresholds,
            true,
            Some(&observations),
        )
        .expect("evaluate summary");

        assert_eq!(report.status, ValidationStatus::Passed);
        assert_eq!(report.failures, 0);
        assert!(report.drift.max_abs_drift_seconds.is_some());
    }

    #[test]
    fn summary_fails_on_thresholds() {
        let root = repo_root();
        let summary_path = root.join("samples/validation/summary_fail.json");
        let summary = load_summary(&summary_path).expect("load summary");
        let reference =
            load_reference_video_toml(&root.join("samples/validation/reference_video.toml"))
                .expect("reference");
        let thresholds = resolve_thresholds(&reference, None).expect("thresholds");

        let report = evaluate_summary(&summary, &summary_path, &reference, thresholds, false, None)
            .expect("evaluate summary");

        assert_eq!(report.status, ValidationStatus::Failed);
        assert!(report.failures > 0);
    }
}
