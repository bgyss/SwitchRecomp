use crate::{chrono_stamp, run_case, run_video_suite, ValidationReport, ValidationStatus};
use recomp_pipeline::xci::check_intake_manifest;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Clone)]
pub struct ArtifactIndex {
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub xci_intake_manifest: Option<PathBuf>,
    #[serde(default)]
    pub pipeline_manifest: Option<PathBuf>,
    #[serde(default)]
    pub run_manifest: Option<PathBuf>,
    #[serde(default)]
    pub reference_config: Option<PathBuf>,
    #[serde(default)]
    pub capture_config: Option<PathBuf>,
    #[serde(default)]
    pub validation_config: Option<PathBuf>,
    #[serde(default)]
    pub out_dir: Option<PathBuf>,
}

pub fn load_artifact_index(path: &Path) -> Result<ArtifactIndex, String> {
    let text = fs::read_to_string(path)
        .map_err(|err| format!("read artifact index {}: {err}", path.display()))?;
    serde_json::from_str(&text).map_err(|err| format!("parse artifact index json: {err}"))
}

pub fn run_artifact_validation(index: &ArtifactIndex) -> ValidationReport {
    let mut cases = Vec::new();

    if let Some(path) = &index.xci_intake_manifest {
        cases.push(run_case("xci_intake_manifest", || {
            let check = check_intake_manifest(path)?;
            if !check.missing_files.is_empty() {
                return Err(format!(
                    "missing generated files: {}",
                    check.missing_files.join(", ")
                ));
            }
            Ok(())
        }));
    }

    if let Some(path) = &index.pipeline_manifest {
        cases.push(run_case("pipeline_manifest", || {
            let text = fs::read_to_string(path)
                .map_err(|err| format!("read pipeline manifest {}: {err}", path.display()))?;
            let _: serde_json::Value =
                serde_json::from_str(&text).map_err(|err| format!("parse manifest json: {err}"))?;
            Ok(())
        }));
    }

    if let Some(path) = &index.run_manifest {
        cases.push(run_case("run_manifest", || {
            let text = fs::read_to_string(path)
                .map_err(|err| format!("read run manifest {}: {err}", path.display()))?;
            let _: serde_json::Value = serde_json::from_str(&text)
                .map_err(|err| format!("parse run manifest json: {err}"))?;
            Ok(())
        }));
    }

    let wants_video = index.reference_config.is_some()
        || index.capture_config.is_some()
        || index.validation_config.is_some();
    let mut video = None;
    if wants_video {
        let reference = index.reference_config.clone();
        let capture = index.capture_config.clone();
        if let (Some(reference), Some(capture)) = (reference, capture) {
            let video_report =
                run_video_suite(&reference, &capture, index.validation_config.as_deref());
            cases.extend(video_report.cases);
            video = video_report.video;
        } else {
            cases.push(run_case("video_validation", || {
                Err("artifact index missing reference_config or capture_config".to_string())
            }));
        }
    }

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
        video,
    }
}
