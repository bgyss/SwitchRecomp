use recomp_pipeline::{run_pipeline, PipelineOptions};
use recomp_runtime::RuntimeConfig;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::Instant;

pub mod video;
pub use video::{
    hash_audio_file, hash_frames_dir, run_video_validation, write_hash_list, CaptureVideoConfig,
    HashFormat, HashSource, HashSources, ReferenceVideoConfig, Timecode, VideoValidationReport,
};

#[derive(Debug, Serialize)]
pub struct ValidationReport {
    pub generated_at: String,
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub cases: Vec<ValidationCase>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video: Option<VideoValidationReport>,
}

#[derive(Debug, Serialize)]
pub struct ValidationCase {
    pub name: String,
    pub status: ValidationStatus,
    pub duration_ms: u128,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

#[derive(Debug, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ValidationStatus {
    Passed,
    Failed,
}

pub struct BaselinePaths {
    pub repo_root: PathBuf,
    pub out_dir: PathBuf,
}

pub fn run_baseline(paths: BaselinePaths) -> ValidationReport {
    let mut cases = Vec::new();

    cases.push(run_case("runtime_config_defaults", || {
        let config = RuntimeConfig::default();
        if matches!(
            config.performance_mode,
            recomp_runtime::PerformanceMode::Handheld
        ) {
            Ok(())
        } else {
            Err("runtime config default is not handheld".to_string())
        }
    }));

    cases.push(run_case("pipeline_minimal_sample", || {
        let samples = paths.repo_root.join("samples/minimal");
        let module = samples.join("module.json");
        let config = samples.join("title.toml");
        let provenance = samples.join("provenance.toml");
        let out_dir = paths.out_dir.join("pipeline-minimal");
        let runtime = paths.repo_root.join("crates/recomp-runtime");

        let report = run_pipeline(PipelineOptions {
            module_path: module,
            config_path: config,
            provenance_path: provenance,
            out_dir,
            runtime_path: runtime,
        })
        .map_err(|err| err.to_string())?;

        if report.files_written.len() != 3 {
            return Err(format!(
                "expected 3 files, got {}",
                report.files_written.len()
            ));
        }
        if report.detected_inputs.is_empty() {
            return Err("no detected inputs reported".to_string());
        }
        Ok(())
    }));

    let (passed, failed) = cases.iter().fold((0, 0), |acc, case| match case.status {
        ValidationStatus::Passed => (acc.0 + 1, acc.1),
        ValidationStatus::Failed => (acc.0, acc.1 + 1),
    });

    ValidationReport {
        generated_at: chrono_stamp(),
        total: cases.len(),
        passed,
        failed,
        cases,
        video: None,
    }
}

fn run_case<F>(name: &str, runner: F) -> ValidationCase
where
    F: FnOnce() -> Result<(), String>,
{
    let start = Instant::now();
    let result = runner();
    let duration_ms = start.elapsed().as_millis();
    match result {
        Ok(()) => ValidationCase {
            name: name.to_string(),
            status: ValidationStatus::Passed,
            duration_ms,
            details: None,
        },
        Err(details) => ValidationCase {
            name: name.to_string(),
            status: ValidationStatus::Failed,
            duration_ms,
            details: Some(details),
        },
    }
}

fn chrono_stamp() -> String {
    let now = std::time::SystemTime::now();
    let secs = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{secs}")
}

pub fn write_report(out_dir: &Path, report: &ValidationReport) -> Result<(), String> {
    std::fs::create_dir_all(out_dir)
        .map_err(|err| format!("create report dir {}: {err}", out_dir.display()))?;
    let json_path = out_dir.join("validation-report.json");
    let text_path = out_dir.join("validation-report.txt");
    let json = serde_json::to_string_pretty(report).map_err(|err| err.to_string())?;
    std::fs::write(&json_path, json).map_err(|err| err.to_string())?;
    let text = render_text_report(report);
    std::fs::write(&text_path, text).map_err(|err| err.to_string())?;
    Ok(())
}

fn render_text_report(report: &ValidationReport) -> String {
    let mut out = String::new();
    out.push_str("SwitchRecomp Baseline Validation Report\n");
    out.push_str(&format!("generated_at: {}\n", report.generated_at));
    out.push_str(&format!(
        "total: {} passed: {} failed: {}\n\n",
        report.total, report.passed, report.failed
    ));
    for case in &report.cases {
        out.push_str(&format!(
            "- {}: {:?} ({} ms)\n",
            case.name, case.status, case.duration_ms
        ));
        if let Some(details) = &case.details {
            out.push_str(&format!("  details: {details}\n"));
        }
    }
    if let Some(video) = &report.video {
        out.push_str("\nVideo validation summary\n");
        out.push_str(&format!("status: {:?}\n", video.status));
        out.push_str(&format!(
            "frame match: {:.3} ({} of {}, offset {} frames)\n",
            video.frame_comparison.match_ratio,
            video.frame_comparison.matched,
            video.frame_comparison.compared,
            video.frame_comparison.offset
        ));
        out.push_str(&format!(
            "frame drift: {} frames ({:.3} sec)\n",
            video.drift.frame_offset, video.drift.frame_offset_seconds
        ));
        if let Some(audio) = &video.audio_comparison {
            out.push_str(&format!(
                "audio match: {:.3} ({} of {}, offset {} chunks)\n",
                audio.match_ratio, audio.matched, audio.compared, audio.offset
            ));
        }
        if !video.failures.is_empty() {
            out.push_str("video failures:\n");
            for failure in &video.failures {
                out.push_str(&format!("- {failure}\n"));
            }
        }
    }
    out
}

pub fn run_video_suite(reference_path: &Path, capture_path: &Path) -> ValidationReport {
    let start = Instant::now();
    let mut cases = Vec::new();
    let (status, details, video_report) = match run_video_validation(reference_path, capture_path) {
        Ok(report) => (
            report.status,
            Some(format!(
                "frame_match_ratio={:.3} drift_frames={}",
                report.frame_comparison.match_ratio, report.drift.frame_offset
            )),
            Some(report),
        ),
        Err(err) => (ValidationStatus::Failed, Some(err), None),
    };
    let duration_ms = start.elapsed().as_millis();
    cases.push(ValidationCase {
        name: "video_validation".to_string(),
        status,
        duration_ms,
        details,
    });

    let (passed, failed) = cases.iter().fold((0, 0), |acc, case| match case.status {
        ValidationStatus::Passed => (acc.0 + 1, acc.1),
        ValidationStatus::Failed => (acc.0, acc.1 + 1),
    });

    ValidationReport {
        generated_at: chrono_stamp(),
        total: cases.len(),
        passed,
        failed,
        cases,
        video: video_report,
    }
}
