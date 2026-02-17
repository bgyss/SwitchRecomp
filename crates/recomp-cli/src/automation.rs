use recomp_pipeline::homebrew::{
    intake_homebrew, lift_homebrew, IntakeOptions, LiftMode, LiftOptions,
};
use recomp_pipeline::xci::{intake_xci, XciIntakeOptions, XciToolPreference};
use recomp_pipeline::{run_pipeline, PipelineOptions};
use recomp_validation::{
    hash_audio_file, hash_frames_dir, run_video_suite, write_hash_list, CaptureVideoConfig,
    HashFormat,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

const AUTOMATION_SCHEMA_VERSION: &str = "1";
const RUN_MANIFEST_SCHEMA_VERSION: &str = "1";

#[derive(Debug, Deserialize, Clone)]
pub struct AutomationConfig {
    pub schema_version: String,
    pub inputs: InputsConfig,
    pub outputs: OutputsConfig,
    pub reference: ReferenceConfig,
    pub capture: CaptureConfig,
    pub commands: CommandConfig,
    #[serde(default)]
    pub tools: ToolsConfig,
    #[serde(default)]
    pub analysis: AnalysisConfig,
    #[serde(default)]
    pub policy: PolicyConfig,
    #[serde(default)]
    pub run: RunConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct InputsConfig {
    pub mode: InputMode,
    #[serde(default)]
    pub module_json: Option<PathBuf>,
    #[serde(default)]
    pub nro: Option<PathBuf>,
    #[serde(default)]
    pub nso: Vec<PathBuf>,
    #[serde(default)]
    pub xci: Option<PathBuf>,
    #[serde(default)]
    pub keys: Option<PathBuf>,
    pub provenance: PathBuf,
    pub config: PathBuf,
    #[serde(default)]
    pub runtime_path: Option<PathBuf>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum InputMode {
    Homebrew,
    Xci,
    Lifted,
}

#[derive(Debug, Deserialize, Clone)]
pub struct OutputsConfig {
    pub work_root: PathBuf,
    #[serde(default)]
    pub intake_dir: Option<PathBuf>,
    #[serde(default)]
    pub lift_dir: Option<PathBuf>,
    #[serde(default)]
    pub build_dir: Option<PathBuf>,
    #[serde(default)]
    pub assets_dir: Option<PathBuf>,
    #[serde(default)]
    pub validation_dir: Option<PathBuf>,
    #[serde(default)]
    pub log_dir: Option<PathBuf>,
    #[serde(default)]
    pub run_manifest: Option<PathBuf>,
    #[serde(default)]
    pub lifted_module_json: Option<PathBuf>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ReferenceConfig {
    pub reference_video_toml: PathBuf,
    pub capture_video_toml: PathBuf,
    #[serde(default)]
    pub validation_config_toml: Option<PathBuf>,
    #[serde(default)]
    pub input_script_toml: Option<PathBuf>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CaptureConfig {
    pub video_path: PathBuf,
    pub frames_dir: PathBuf,
    #[serde(default)]
    pub audio_file: Option<PathBuf>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CommandConfig {
    pub build: Vec<String>,
    pub run: Vec<String>,
    pub capture: Vec<String>,
    pub extract_frames: Vec<String>,
    #[serde(default)]
    pub extract_audio: Option<Vec<String>>,
    #[serde(default)]
    pub lift: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct ToolsConfig {
    #[serde(default)]
    pub xci_tool: Option<AutomationXciTool>,
    #[serde(default)]
    pub xci_tool_path: Option<PathBuf>,
    #[serde(default)]
    pub ffmpeg_path: Option<PathBuf>,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum AutomationXciTool {
    Auto,
    Hactool,
    Hactoolnet,
    Mock,
}

impl From<AutomationXciTool> for XciToolPreference {
    fn from(value: AutomationXciTool) -> Self {
        match value {
            AutomationXciTool::Auto => XciToolPreference::Auto,
            AutomationXciTool::Hactool => XciToolPreference::Hactool,
            AutomationXciTool::Hactoolnet => XciToolPreference::Hactoolnet,
            AutomationXciTool::Mock => XciToolPreference::Mock,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct RunConfig {
    #[serde(default = "default_resume")]
    pub resume: bool,
    #[serde(default)]
    pub lift_entry: Option<String>,
    #[serde(default)]
    pub lift_mode: Option<LiftModeConfig>,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            resume: default_resume(),
            lift_entry: None,
            lift_mode: None,
        }
    }
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct AnalysisConfig {
    #[serde(default)]
    pub command: Option<Vec<String>>,
    #[serde(default)]
    pub expected_outputs: Vec<PathBuf>,
    #[serde(default)]
    pub name_map_json: Option<PathBuf>,
    #[serde(default)]
    pub runtime_trace_manifest: Option<PathBuf>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct PolicyConfig {
    #[serde(default)]
    pub requires_approval: bool,
    #[serde(default)]
    pub max_cost_usd: Option<f64>,
    #[serde(default)]
    pub max_runtime_minutes: Option<u64>,
    #[serde(default)]
    pub execution_mode: Option<ExecutionMode>,
    #[serde(default)]
    pub redaction_profile: Option<String>,
    #[serde(default)]
    pub allowed_models: Vec<String>,
    #[serde(default)]
    pub run_windows: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionMode {
    Local,
    Cloud,
    Hybrid,
}

impl std::fmt::Display for ExecutionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionMode::Local => f.write_str("local"),
            ExecutionMode::Cloud => f.write_str("cloud"),
            ExecutionMode::Hybrid => f.write_str("hybrid"),
        }
    }
}

impl Default for ExecutionMode {
    fn default() -> Self {
        Self::Local
    }
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum LiftModeConfig {
    Stub,
    Decode,
}

impl From<LiftModeConfig> for LiftMode {
    fn from(value: LiftModeConfig) -> Self {
        match value {
            LiftModeConfig::Stub => LiftMode::Stub,
            LiftModeConfig::Decode => LiftMode::Decode,
        }
    }
}

fn default_resume() -> bool {
    true
}

#[derive(Debug)]
struct ResolvedPaths {
    repo_root: PathBuf,
    config_dir: PathBuf,
    work_root: PathBuf,
    intake_dir: PathBuf,
    lift_dir: PathBuf,
    build_dir: PathBuf,
    assets_dir: PathBuf,
    validation_dir: PathBuf,
    log_dir: PathBuf,
    run_manifest: PathBuf,
    lifted_module_json: PathBuf,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RunManifest {
    pub schema_version: String,
    #[serde(default)]
    pub run_id: String,
    #[serde(default)]
    pub execution_mode: ExecutionMode,
    #[serde(default)]
    pub host_fingerprint: String,
    #[serde(default)]
    pub tool_versions: ToolVersions,
    pub input_fingerprint: String,
    pub inputs: Vec<RunInput>,
    pub steps: Vec<RunStep>,
    pub artifacts: Vec<RunArtifact>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation_report: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RunInput {
    pub name: String,
    pub path: String,
    pub sha256: String,
    pub size: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RunStep {
    pub name: String,
    pub status: StepStatus,
    pub duration_ms: u128,
    #[serde(default)]
    pub stage_attempt: u32,
    #[serde(default)]
    pub cache_hit: bool,
    #[serde(default)]
    pub cache_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stdout_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stderr_path: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outputs: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    Succeeded,
    Failed,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RunArtifact {
    pub path: String,
    pub sha256: String,
    pub size: u64,
    pub role: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolVersions {
    pub recomp_cli: String,
    pub rustc: Option<String>,
    pub ffmpeg: Option<String>,
    pub xci_tool: Option<String>,
}

impl Default for ToolVersions {
    fn default() -> Self {
        Self {
            recomp_cli: env!("CARGO_PKG_VERSION").to_string(),
            rustc: None,
            ffmpeg: None,
            xci_tool: None,
        }
    }
}

#[derive(Debug)]
struct RunState {
    manifest: RunManifest,
    artifacts: BTreeMap<String, RunArtifact>,
    previous_steps: HashMap<String, RunStep>,
    attempts: HashMap<String, u32>,
    cache_valid: bool,
}

pub fn run_automation(config_path: &Path) -> Result<RunManifest, String> {
    let config_path = fs::canonicalize(config_path)
        .map_err(|err| format!("resolve automation config {}: {err}", config_path.display()))?;
    let config_src = fs::read_to_string(&config_path)
        .map_err(|err| format!("read automation config {}: {err}", config_path.display()))?;
    let mut config: AutomationConfig =
        toml::from_str(&config_src).map_err(|err| format!("invalid automation config: {err}"))?;
    let config_dir = config_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    config.resolve_paths(&config_dir);
    config.validate()?;

    let paths = ResolvedPaths::new(&config, config_dir.clone())?;
    fs::create_dir_all(&paths.work_root)
        .map_err(|err| format!("create work root {}: {err}", paths.work_root.display()))?;
    fs::create_dir_all(&paths.log_dir)
        .map_err(|err| format!("create log dir {}: {err}", paths.log_dir.display()))?;
    fs::create_dir_all(&paths.validation_dir).map_err(|err| {
        format!(
            "create validation dir {}: {err}",
            paths.validation_dir.display()
        )
    })?;

    let inputs = gather_inputs(&config, &config_path, &paths)?;
    let input_fingerprint = fingerprint_inputs(&inputs);
    let execution_mode = config.policy.execution_mode.unwrap_or_default();
    let tool_versions = gather_tool_versions(&config);
    let host_fingerprint = fingerprint_host();
    let run_id = derive_run_id(&input_fingerprint, execution_mode, &paths.work_root);

    let previous_manifest = if config.run.resume && paths.run_manifest.exists() {
        Some(load_run_manifest(&paths.run_manifest)?)
    } else {
        None
    };

    let mut artifacts = BTreeMap::new();
    let mut previous_steps = HashMap::new();
    let mut attempts = HashMap::new();
    if let Some(previous) = &previous_manifest {
        if previous.input_fingerprint == input_fingerprint {
            for artifact in &previous.artifacts {
                artifacts.insert(artifact.path.clone(), artifact.clone());
            }
            for step in &previous.steps {
                previous_steps.insert(step.name.clone(), step.clone());
                attempts.insert(step.name.clone(), step.stage_attempt);
            }
        }
    }

    let mut state = RunState {
        manifest: RunManifest {
            schema_version: RUN_MANIFEST_SCHEMA_VERSION.to_string(),
            run_id,
            execution_mode,
            host_fingerprint,
            tool_versions,
            input_fingerprint: input_fingerprint.clone(),
            inputs,
            steps: Vec::new(),
            artifacts: Vec::new(),
            validation_report: None,
        },
        artifacts,
        previous_steps,
        attempts,
        cache_valid: config.run.resume,
    };

    let mut module_json_path = match config.inputs.mode {
        InputMode::Lifted => config
            .inputs
            .module_json
            .clone()
            .ok_or_else(|| "inputs.module_json is required for mode=lifted".to_string())?,
        _ => paths.intake_dir.join("module.json"),
    };

    if matches!(config.inputs.mode, InputMode::Homebrew | InputMode::Xci) {
        run_cached_step("intake", &paths, &config, &mut state, None, |state| {
            let outcome =
                match config.inputs.mode {
                    InputMode::Homebrew => {
                        let report = intake_homebrew(IntakeOptions {
                            module_path: config.inputs.nro.clone().ok_or_else(|| {
                                "inputs.nro is required for mode=homebrew".to_string()
                            })?,
                            nso_paths: config.inputs.nso.clone(),
                            provenance_path: config.inputs.provenance.clone(),
                            out_dir: paths.intake_dir.clone(),
                        })
                        .map_err(|err| format!("homebrew intake failed: {err}"))?;
                        module_json_path = report.module_json_path.clone();
                        let mut outputs = Vec::new();
                        for path in report.files_written {
                            outputs.push(record_artifact(state, &paths, &path, "intake_output")?);
                        }
                        StepOutcome {
                            status: StepStatus::Succeeded,
                            stdout: format!("homebrew intake wrote {} files", outputs.len()),
                            stderr: String::new(),
                            outputs,
                        }
                    }
                    InputMode::Xci => {
                        let report =
                            intake_xci(XciIntakeOptions {
                                xci_path: config.inputs.xci.clone().ok_or_else(|| {
                                    "inputs.xci is required for mode=xci".to_string()
                                })?,
                                keys_path: config.inputs.keys.clone().ok_or_else(|| {
                                    "inputs.keys is required for mode=xci".to_string()
                                })?,
                                config_path: None,
                                provenance_path: config.inputs.provenance.clone(),
                                out_dir: paths.intake_dir.clone(),
                                assets_dir: paths.assets_dir.clone(),
                                tool_preference: config
                                    .tools
                                    .xci_tool
                                    .unwrap_or(AutomationXciTool::Auto)
                                    .into(),
                                tool_path: config.tools.xci_tool_path.clone(),
                            })
                            .map_err(|err| format!("xci intake failed: {err}"))?;
                        module_json_path = report.module_json_path.clone();
                        let mut outputs = Vec::new();
                        for path in report.files_written {
                            outputs.push(record_artifact(state, &paths, &path, "intake_output")?);
                        }
                        StepOutcome {
                            status: StepStatus::Succeeded,
                            stdout: format!("xci intake wrote {} files", outputs.len()),
                            stderr: String::new(),
                            outputs,
                        }
                    }
                    InputMode::Lifted => {
                        return Err("intake step not valid for mode=lifted".to_string());
                    }
                };
            Ok(outcome)
        })?;
    }

    let has_analysis_contract = config.analysis.command.is_some()
        || !config.analysis.expected_outputs.is_empty()
        || config.analysis.name_map_json.is_some()
        || config.analysis.runtime_trace_manifest.is_some();
    if has_analysis_contract {
        let analysis_command = config.analysis.command.clone();
        run_cached_step(
            "analysis",
            &paths,
            &config,
            &mut state,
            analysis_command.clone(),
            |state| {
                let (stdout, stderr) = match &analysis_command {
                    Some(command) => run_command(command, &paths, &config)?,
                    None => (
                        "analysis command not configured; validating analysis contracts only"
                            .to_string(),
                        String::new(),
                    ),
                };
                let mut outputs = Vec::new();
                for output in &config.analysis.expected_outputs {
                    if !output.exists() {
                        return Err(format!(
                            "analysis expected output not found: {}",
                            output.display()
                        ));
                    }
                    outputs.push(record_artifact(state, &paths, output, "analysis_output")?);
                }
                if let Some(path) = &config.analysis.name_map_json {
                    if !path.exists() {
                        return Err(format!(
                            "analysis name_map_json not found: {}",
                            path.display()
                        ));
                    }
                    outputs.push(record_artifact(state, &paths, path, "analysis_name_map")?);
                }
                if let Some(path) = &config.analysis.runtime_trace_manifest {
                    if !path.exists() {
                        return Err(format!(
                            "analysis runtime_trace_manifest not found: {}",
                            path.display()
                        ));
                    }
                    outputs.push(record_artifact(
                        state,
                        &paths,
                        path,
                        "analysis_runtime_trace_manifest",
                    )?);
                }
                Ok(StepOutcome {
                    status: StepStatus::Succeeded,
                    stdout,
                    stderr,
                    outputs,
                })
            },
        )?;
    }

    if matches!(config.inputs.mode, InputMode::Homebrew | InputMode::Xci) {
        run_cached_step(
            "lift",
            &paths,
            &config,
            &mut state,
            None,
            |state| match config.inputs.mode {
                InputMode::Homebrew => {
                    let report = lift_homebrew(LiftOptions {
                        module_json_path: module_json_path.clone(),
                        out_dir: paths.lift_dir.clone(),
                        entry_name: config
                            .run
                            .lift_entry
                            .clone()
                            .unwrap_or_else(|| "entry".to_string()),
                        mode: config
                            .run
                            .lift_mode
                            .unwrap_or(LiftModeConfig::Decode)
                            .into(),
                    })
                    .map_err(|err| format!("homebrew lift failed: {err}"))?;
                    module_json_path = report.module_json_path.clone();
                    let output =
                        record_artifact(state, &paths, &report.module_json_path, "lifted_module")?;
                    Ok(StepOutcome {
                        status: StepStatus::Succeeded,
                        stdout: format!(
                            "lifted module emitted {} functions",
                            report.functions_emitted
                        ),
                        stderr: report.warnings.join("\n"),
                        outputs: vec![output],
                    })
                }
                InputMode::Xci => {
                    let lift_command = config
                        .commands
                        .lift
                        .clone()
                        .ok_or_else(|| "commands.lift is required for mode=xci".to_string())?;
                    let (stdout, stderr) = run_command(&lift_command, &paths, &config)?;
                    let output_path = paths.lifted_module_json.clone();
                    if !output_path.exists() {
                        return Err(format!(
                            "lifted module not found at {}",
                            output_path.display()
                        ));
                    }
                    module_json_path = output_path.clone();
                    let output = record_artifact(state, &paths, &output_path, "lifted_module")?;
                    Ok(StepOutcome {
                        status: StepStatus::Succeeded,
                        stdout,
                        stderr,
                        outputs: vec![output],
                    })
                }
                InputMode::Lifted => unreachable!(),
            },
        )?;
    }

    run_cached_step("pipeline", &paths, &config, &mut state, None, |state| {
        let runtime_path = config
            .inputs
            .runtime_path
            .clone()
            .unwrap_or_else(|| paths.repo_root.join("crates/recomp-runtime"));
        let report = run_pipeline(PipelineOptions {
            module_path: module_json_path.clone(),
            config_path: config.inputs.config.clone(),
            provenance_path: config.inputs.provenance.clone(),
            out_dir: paths.build_dir.clone(),
            runtime_path,
        })
        .map_err(|err| format!("pipeline failed: {err}"))?;
        let mut outputs = Vec::new();
        for path in report.files_written {
            outputs.push(record_artifact(state, &paths, &path, "pipeline_output")?);
        }
        Ok(StepOutcome {
            status: StepStatus::Succeeded,
            stdout: format!("pipeline wrote {} files", outputs.len()),
            stderr: String::new(),
            outputs,
        })
    })?;

    run_cached_step(
        "build",
        &paths,
        &config,
        &mut state,
        Some(config.commands.build.clone()),
        |_state| {
            let (stdout, stderr) = run_command(&config.commands.build, &paths, &config)?;
            Ok(StepOutcome {
                status: StepStatus::Succeeded,
                stdout,
                stderr,
                outputs: Vec::new(),
            })
        },
    )?;

    run_cached_step(
        "run",
        &paths,
        &config,
        &mut state,
        Some(config.commands.run.clone()),
        |_state| {
            let (stdout, stderr) = run_command(&config.commands.run, &paths, &config)?;
            Ok(StepOutcome {
                status: StepStatus::Succeeded,
                stdout,
                stderr,
                outputs: Vec::new(),
            })
        },
    )?;

    run_cached_step(
        "capture",
        &paths,
        &config,
        &mut state,
        Some(config.commands.capture.clone()),
        |state| {
            let (stdout, stderr) = run_command(&config.commands.capture, &paths, &config)?;
            let mut outputs = Vec::new();
            if config.capture.video_path.exists() {
                outputs.push(record_artifact(
                    state,
                    &paths,
                    &config.capture.video_path,
                    "capture_video",
                )?);
            }
            Ok(StepOutcome {
                status: StepStatus::Succeeded,
                stdout,
                stderr,
                outputs,
            })
        },
    )?;

    run_cached_step(
        "extract_frames",
        &paths,
        &config,
        &mut state,
        Some(config.commands.extract_frames.clone()),
        |_state| {
            let (stdout, stderr) = run_command(&config.commands.extract_frames, &paths, &config)?;
            Ok(StepOutcome {
                status: StepStatus::Succeeded,
                stdout,
                stderr,
                outputs: Vec::new(),
            })
        },
    )?;

    if let Some(audio_file) = config.capture.audio_file.clone() {
        let command = config.commands.extract_audio.clone().ok_or_else(|| {
            "commands.extract_audio is required when capture.audio_file is set".to_string()
        })?;
        run_cached_step(
            "extract_audio",
            &paths,
            &config,
            &mut state,
            Some(command.clone()),
            |state| {
                let (stdout, stderr) = run_command(&command, &paths, &config)?;
                let mut outputs = Vec::new();
                if audio_file.exists() {
                    outputs.push(record_artifact(
                        state,
                        &paths,
                        &audio_file,
                        "capture_audio",
                    )?);
                }
                Ok(StepOutcome {
                    status: StepStatus::Succeeded,
                    stdout,
                    stderr,
                    outputs,
                })
            },
        )?;
    }

    let capture_config_src =
        fs::read_to_string(&config.reference.capture_video_toml).map_err(|err| {
            format!(
                "read capture config {}: {err}",
                config.reference.capture_video_toml.display()
            )
        })?;
    let capture_config: CaptureVideoConfig = toml::from_str(&capture_config_src)
        .map_err(|err| format!("invalid capture config: {err}"))?;
    let capture_config_dir = config
        .reference
        .capture_video_toml
        .parent()
        .unwrap_or_else(|| Path::new("."));
    let capture_video_path = resolve_path(capture_config_dir, &capture_config.video.path);
    if capture_video_path != config.capture.video_path {
        return Err(format!(
            "capture video path mismatch: config {}, capture_video.toml {}",
            config.capture.video_path.display(),
            capture_video_path.display()
        ));
    }

    if capture_config.hashes.frames.format != HashFormat::List {
        return Err("capture hashes.frames must use format=list".to_string());
    }
    let frames_hash_path = resolve_path(capture_config_dir, &capture_config.hashes.frames.path);
    run_cached_step("hash_frames", &paths, &config, &mut state, None, |state| {
        let hashes = hash_frames_dir(&config.capture.frames_dir)
            .map_err(|err| format!("hash frames failed: {err}"))?;
        write_hash_list(&frames_hash_path, &hashes)
            .map_err(|err| format!("write frame hashes: {err}"))?;
        let output = record_artifact(state, &paths, &frames_hash_path, "frame_hashes")?;
        Ok(StepOutcome {
            status: StepStatus::Succeeded,
            stdout: format!("frame hashes written ({})", hashes.len()),
            stderr: String::new(),
            outputs: vec![output],
        })
    })?;

    if let Some(audio_hash) = &capture_config.hashes.audio {
        if audio_hash.format != HashFormat::List {
            return Err("capture hashes.audio must use format=list".to_string());
        }
        let audio_file = config
            .capture
            .audio_file
            .clone()
            .ok_or_else(|| "capture.audio_file is required for audio hashing".to_string())?;
        let audio_hash_path = resolve_path(capture_config_dir, &audio_hash.path);
        run_cached_step("hash_audio", &paths, &config, &mut state, None, |state| {
            let hashes =
                hash_audio_file(&audio_file).map_err(|err| format!("hash audio failed: {err}"))?;
            write_hash_list(&audio_hash_path, &hashes)
                .map_err(|err| format!("write audio hashes: {err}"))?;
            let output = record_artifact(state, &paths, &audio_hash_path, "audio_hashes")?;
            Ok(StepOutcome {
                status: StepStatus::Succeeded,
                stdout: format!("audio hashes written ({})", hashes.len()),
                stderr: String::new(),
                outputs: vec![output],
            })
        })?;
    }

    run_cached_step("validate", &paths, &config, &mut state, None, |state| {
        let report = run_video_suite(
            &config.reference.reference_video_toml,
            &config.reference.capture_video_toml,
            config.reference.validation_config_toml.as_deref(),
        );
        let report_dir = &paths.validation_dir;
        recomp_validation::write_report(report_dir, &report)
            .map_err(|err| format!("write validation report: {err}"))?;
        let report_path = report_dir.join("validation-report.json");
        let output = record_artifact(state, &paths, &report_path, "validation_report")?;
        state.manifest.validation_report = Some(output.clone());
        let status = if report.failed > 0 {
            StepStatus::Failed
        } else {
            StepStatus::Succeeded
        };
        Ok(StepOutcome {
            status,
            stdout: format!(
                "validation status: {}",
                if report.failed > 0 {
                    "failed"
                } else {
                    "passed"
                }
            ),
            stderr: if report.failed > 0 {
                format!("validation failed: {} cases", report.failed)
            } else {
                String::new()
            },
            outputs: vec![output],
        })
    })?;

    finalize_manifest(&mut state);
    write_run_manifest(&paths.run_manifest, &state.manifest)?;

    Ok(state.manifest)
}

impl AutomationConfig {
    fn resolve_paths(&mut self, base_dir: &Path) {
        self.inputs.provenance = resolve_path(base_dir, &self.inputs.provenance);
        self.inputs.config = resolve_path(base_dir, &self.inputs.config);
        if let Some(path) = &self.inputs.module_json {
            self.inputs.module_json = Some(resolve_path(base_dir, path));
        }
        if let Some(path) = &self.inputs.nro {
            self.inputs.nro = Some(resolve_path(base_dir, path));
        }
        if let Some(path) = &self.inputs.xci {
            self.inputs.xci = Some(resolve_path(base_dir, path));
        }
        if let Some(path) = &self.inputs.keys {
            self.inputs.keys = Some(resolve_path(base_dir, path));
        }
        if let Some(path) = &self.inputs.runtime_path {
            self.inputs.runtime_path = Some(resolve_path(base_dir, path));
        }
        for path in &mut self.inputs.nso {
            *path = resolve_path(base_dir, path);
        }

        self.outputs.work_root = resolve_path(base_dir, &self.outputs.work_root);
        if let Some(path) = &self.outputs.intake_dir {
            self.outputs.intake_dir = Some(resolve_path(base_dir, path));
        }
        if let Some(path) = &self.outputs.lift_dir {
            self.outputs.lift_dir = Some(resolve_path(base_dir, path));
        }
        if let Some(path) = &self.outputs.build_dir {
            self.outputs.build_dir = Some(resolve_path(base_dir, path));
        }
        if let Some(path) = &self.outputs.assets_dir {
            self.outputs.assets_dir = Some(resolve_path(base_dir, path));
        }
        if let Some(path) = &self.outputs.validation_dir {
            self.outputs.validation_dir = Some(resolve_path(base_dir, path));
        }
        if let Some(path) = &self.outputs.log_dir {
            self.outputs.log_dir = Some(resolve_path(base_dir, path));
        }
        if let Some(path) = &self.outputs.run_manifest {
            self.outputs.run_manifest = Some(resolve_path(base_dir, path));
        }
        if let Some(path) = &self.outputs.lifted_module_json {
            self.outputs.lifted_module_json = Some(resolve_path(base_dir, path));
        }

        self.reference.reference_video_toml =
            resolve_path(base_dir, &self.reference.reference_video_toml);
        self.reference.capture_video_toml =
            resolve_path(base_dir, &self.reference.capture_video_toml);
        if let Some(path) = &self.reference.validation_config_toml {
            self.reference.validation_config_toml = Some(resolve_path(base_dir, path));
        }
        if let Some(path) = &self.reference.input_script_toml {
            self.reference.input_script_toml = Some(resolve_path(base_dir, path));
        }

        self.capture.video_path = resolve_path(base_dir, &self.capture.video_path);
        self.capture.frames_dir = resolve_path(base_dir, &self.capture.frames_dir);
        if let Some(path) = &self.capture.audio_file {
            self.capture.audio_file = Some(resolve_path(base_dir, path));
        }
        for path in &mut self.analysis.expected_outputs {
            *path = resolve_path(base_dir, path);
        }
        if let Some(path) = &self.analysis.name_map_json {
            self.analysis.name_map_json = Some(resolve_path(base_dir, path));
        }
        if let Some(path) = &self.analysis.runtime_trace_manifest {
            self.analysis.runtime_trace_manifest = Some(resolve_path(base_dir, path));
        }

        if let Some(path) = &self.tools.xci_tool_path {
            self.tools.xci_tool_path = Some(resolve_path(base_dir, path));
        }
        if let Some(path) = &self.tools.ffmpeg_path {
            self.tools.ffmpeg_path = Some(resolve_path(base_dir, path));
        }
    }

    fn validate(&self) -> Result<(), String> {
        if self.schema_version != AUTOMATION_SCHEMA_VERSION {
            return Err(format!(
                "unsupported automation schema version: {}",
                self.schema_version
            ));
        }
        if self.commands.build.is_empty()
            || self.commands.run.is_empty()
            || self.commands.capture.is_empty()
            || self.commands.extract_frames.is_empty()
        {
            return Err("commands.build/run/capture/extract_frames must be non-empty".to_string());
        }
        if !self.inputs.provenance.exists() {
            return Err(format!(
                "provenance path not found: {}",
                self.inputs.provenance.display()
            ));
        }
        if !self.inputs.config.exists() {
            return Err(format!(
                "config path not found: {}",
                self.inputs.config.display()
            ));
        }
        match self.inputs.mode {
            InputMode::Homebrew => {
                let Some(nro) = &self.inputs.nro else {
                    return Err("inputs.nro is required for mode=homebrew".to_string());
                };
                if !nro.exists() {
                    return Err(format!("homebrew NRO not found: {}", nro.display()));
                }
                for path in &self.inputs.nso {
                    if !path.exists() {
                        return Err(format!("homebrew NSO not found: {}", path.display()));
                    }
                }
            }
            InputMode::Xci => {
                let Some(xci) = &self.inputs.xci else {
                    return Err("inputs.xci is required for mode=xci".to_string());
                };
                if !xci.exists() {
                    return Err(format!("xci not found: {}", xci.display()));
                }
                let Some(keys) = &self.inputs.keys else {
                    return Err("inputs.keys is required for mode=xci".to_string());
                };
                if !keys.exists() {
                    return Err(format!("keys not found: {}", keys.display()));
                }
                if self.commands.lift.is_none() {
                    return Err("commands.lift is required for mode=xci".to_string());
                }
            }
            InputMode::Lifted => {
                let Some(module_json) = &self.inputs.module_json else {
                    return Err("inputs.module_json is required for mode=lifted".to_string());
                };
                if !module_json.exists() {
                    return Err(format!("module.json not found: {}", module_json.display()));
                }
            }
        }
        if !self.reference.reference_video_toml.exists() {
            return Err(format!(
                "reference video config not found: {}",
                self.reference.reference_video_toml.display()
            ));
        }
        if !self.reference.capture_video_toml.exists() {
            return Err(format!(
                "capture video config not found: {}",
                self.reference.capture_video_toml.display()
            ));
        }
        if let Some(path) = &self.reference.validation_config_toml {
            if !path.exists() {
                return Err(format!("validation config not found: {}", path.display()));
            }
        }
        if let Some(path) = &self.reference.input_script_toml {
            if !path.exists() {
                return Err(format!("input script not found: {}", path.display()));
            }
        }
        if let Some(runtime_path) = &self.inputs.runtime_path {
            if !runtime_path.exists() {
                return Err(format!(
                    "runtime path not found: {}",
                    runtime_path.display()
                ));
            }
        }
        if self.capture.audio_file.is_some() && self.commands.extract_audio.is_none() {
            return Err(
                "commands.extract_audio is required when capture.audio_file is set".to_string(),
            );
        }
        if let Some(command) = &self.analysis.command {
            if command.is_empty() {
                return Err("analysis.command must be non-empty when set".to_string());
            }
        }
        if self.analysis.command.is_none()
            && (self.analysis.name_map_json.is_some()
                || self.analysis.runtime_trace_manifest.is_some())
            && self.analysis.expected_outputs.is_empty()
        {
            let name_map_missing = self
                .analysis
                .name_map_json
                .as_ref()
                .map(|path| !path.exists())
                .unwrap_or(false);
            let trace_missing = self
                .analysis
                .runtime_trace_manifest
                .as_ref()
                .map(|path| !path.exists())
                .unwrap_or(false);
            if name_map_missing || trace_missing {
                return Err(
                    "analysis contract paths must exist when analysis.command is not configured"
                        .to_string(),
                );
            }
        }
        Ok(())
    }
}

impl ResolvedPaths {
    fn new(config: &AutomationConfig, config_dir: PathBuf) -> Result<Self, String> {
        let repo_root = repo_root();
        let work_root = config.outputs.work_root.clone();
        let intake_dir = config
            .outputs
            .intake_dir
            .clone()
            .unwrap_or_else(|| work_root.join("intake"));
        let lift_dir = config
            .outputs
            .lift_dir
            .clone()
            .unwrap_or_else(|| work_root.join("lift"));
        let build_dir = config
            .outputs
            .build_dir
            .clone()
            .unwrap_or_else(|| work_root.join("build"));
        let assets_dir = config
            .outputs
            .assets_dir
            .clone()
            .unwrap_or_else(|| work_root.join("assets"));
        let validation_dir = config
            .outputs
            .validation_dir
            .clone()
            .unwrap_or_else(|| work_root.join("validation"));
        let log_dir = config
            .outputs
            .log_dir
            .clone()
            .unwrap_or_else(|| work_root.join("logs"));
        let run_manifest = config
            .outputs
            .run_manifest
            .clone()
            .unwrap_or_else(|| work_root.join("run-manifest.json"));
        let lifted_module_json = config
            .outputs
            .lifted_module_json
            .clone()
            .unwrap_or_else(|| lift_dir.join("module.json"));

        Ok(Self {
            repo_root,
            config_dir,
            work_root,
            intake_dir,
            lift_dir,
            build_dir,
            assets_dir,
            validation_dir,
            log_dir,
            run_manifest,
            lifted_module_json,
        })
    }
}

fn run_cached_step<F>(
    name: &str,
    paths: &ResolvedPaths,
    config: &AutomationConfig,
    state: &mut RunState,
    command: Option<Vec<String>>,
    action: F,
) -> Result<(), String>
where
    F: FnOnce(&mut RunState) -> Result<StepOutcome, String>,
{
    let cache_key = stage_cache_key(
        name,
        &state.manifest.input_fingerprint,
        &command,
        state,
        config,
    );
    if state.cache_valid {
        if let Some(previous) = state.previous_steps.get(name) {
            if previous.status == StepStatus::Succeeded
                && outputs_exist(paths, previous)
                && previous.cache_key == cache_key
            {
                let mut cached = previous.clone();
                cached.cache_hit = true;
                cached.command = command.clone();
                state.manifest.steps.push(cached);
                return Ok(());
            }
        }
        state.cache_valid = false;
    }

    let stage_attempt = state.attempts.get(name).copied().unwrap_or(0) + 1;
    state.attempts.insert(name.to_string(), stage_attempt);

    let start = Instant::now();
    let outcome = action(state);
    let duration_ms = start.elapsed().as_millis();

    match outcome {
        Ok(outcome) => {
            let (stdout_path, stderr_path) =
                write_step_logs(paths, name, &outcome.stdout, &outcome.stderr)?;
            let mut outputs = outcome.outputs;
            if let Some(stdout) = &stdout_path {
                outputs.push(record_artifact(state, paths, stdout, "log_stdout")?);
            }
            if let Some(stderr) = &stderr_path {
                outputs.push(record_artifact(state, paths, stderr, "log_stderr")?);
            }
            let step = RunStep {
                name: name.to_string(),
                status: outcome.status,
                duration_ms,
                stage_attempt,
                cache_hit: false,
                cache_key,
                command,
                stdout_path: stdout_path.map(|path| format_path(paths, &path)),
                stderr_path: stderr_path.map(|path| format_path(paths, &path)),
                outputs,
                notes: if outcome.status == StepStatus::Failed {
                    Some(outcome.stderr.clone())
                } else {
                    None
                },
            };
            state.manifest.steps.push(step);
            finalize_manifest(state);
            write_run_manifest(&paths.run_manifest, &state.manifest)?;
            if outcome.status == StepStatus::Failed {
                Err(outcome.stderr)
            } else {
                Ok(())
            }
        }
        Err(err) => {
            let (stdout_path, stderr_path) = write_step_logs(paths, name, "", &err)?;
            let mut outputs = Vec::new();
            if let Some(stdout) = &stdout_path {
                outputs.push(record_artifact(state, paths, stdout, "log_stdout")?);
            }
            if let Some(stderr) = &stderr_path {
                outputs.push(record_artifact(state, paths, stderr, "log_stderr")?);
            }
            let step = RunStep {
                name: name.to_string(),
                status: StepStatus::Failed,
                duration_ms,
                stage_attempt,
                cache_hit: false,
                cache_key,
                command,
                stdout_path: stdout_path.map(|path| format_path(paths, &path)),
                stderr_path: stderr_path.map(|path| format_path(paths, &path)),
                outputs,
                notes: Some(err.clone()),
            };
            state.manifest.steps.push(step);
            finalize_manifest(state);
            write_run_manifest(&paths.run_manifest, &state.manifest)?;
            Err(err)
        }
    }
}

struct StepOutcome {
    status: StepStatus,
    stdout: String,
    stderr: String,
    outputs: Vec<String>,
}

fn run_command(
    argv: &[String],
    paths: &ResolvedPaths,
    config: &AutomationConfig,
) -> Result<(String, String), String> {
    let (program, args) = argv
        .split_first()
        .ok_or_else(|| "command argv is empty".to_string())?;
    let mut cmd = Command::new(program);
    cmd.args(args);
    cmd.current_dir(&paths.repo_root);
    for (key, value) in command_env(paths, config) {
        cmd.env(key, value);
    }
    let output = cmd
        .output()
        .map_err(|err| format!("run command failed: {err}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if output.status.success() {
        Ok((stdout, stderr))
    } else {
        Err(format!(
            "command failed ({}): {}",
            output.status.code().unwrap_or(-1),
            stderr.trim()
        ))
    }
}

fn command_env(paths: &ResolvedPaths, config: &AutomationConfig) -> BTreeMap<String, String> {
    let mut env = BTreeMap::new();
    env.insert(
        "RECOMP_WORK_ROOT".to_string(),
        paths.work_root.display().to_string(),
    );
    env.insert(
        "RECOMP_INTAKE_DIR".to_string(),
        paths.intake_dir.display().to_string(),
    );
    env.insert(
        "RECOMP_LIFT_DIR".to_string(),
        paths.lift_dir.display().to_string(),
    );
    env.insert(
        "RECOMP_BUILD_DIR".to_string(),
        paths.build_dir.display().to_string(),
    );
    env.insert(
        "RECOMP_ASSETS_DIR".to_string(),
        paths.assets_dir.display().to_string(),
    );
    env.insert(
        "RECOMP_REFERENCE_VIDEO_TOML".to_string(),
        config.reference.reference_video_toml.display().to_string(),
    );
    env.insert(
        "RECOMP_CAPTURE_VIDEO_TOML".to_string(),
        config.reference.capture_video_toml.display().to_string(),
    );
    env.insert(
        "RECOMP_CAPTURE_VIDEO".to_string(),
        config.capture.video_path.display().to_string(),
    );
    env.insert(
        "RECOMP_CAPTURE_FRAMES_DIR".to_string(),
        config.capture.frames_dir.display().to_string(),
    );
    if let Some(audio_file) = &config.capture.audio_file {
        env.insert(
            "RECOMP_CAPTURE_AUDIO_FILE".to_string(),
            audio_file.display().to_string(),
        );
    }
    env.insert(
        "RECOMP_VALIDATION_DIR".to_string(),
        paths.validation_dir.display().to_string(),
    );
    env.insert(
        "RECOMP_RUN_MANIFEST".to_string(),
        paths.run_manifest.display().to_string(),
    );
    env.insert(
        "RECOMP_LIFTED_MODULE_JSON".to_string(),
        paths.lifted_module_json.display().to_string(),
    );
    env.insert(
        "RECOMP_EXECUTION_MODE".to_string(),
        config.policy.execution_mode.unwrap_or_default().to_string(),
    );
    env.insert(
        "RECOMP_POLICY_REQUIRES_APPROVAL".to_string(),
        config.policy.requires_approval.to_string(),
    );
    if let Some(max_cost_usd) = config.policy.max_cost_usd {
        env.insert(
            "RECOMP_POLICY_MAX_COST_USD".to_string(),
            max_cost_usd.to_string(),
        );
    }
    if let Some(max_runtime_minutes) = config.policy.max_runtime_minutes {
        env.insert(
            "RECOMP_POLICY_MAX_RUNTIME_MINUTES".to_string(),
            max_runtime_minutes.to_string(),
        );
    }
    if let Some(name_map) = &config.analysis.name_map_json {
        env.insert(
            "RECOMP_ANALYSIS_NAME_MAP_JSON".to_string(),
            name_map.display().to_string(),
        );
    }
    if let Some(trace_manifest) = &config.analysis.runtime_trace_manifest {
        env.insert(
            "RECOMP_ANALYSIS_RUNTIME_TRACE_MANIFEST".to_string(),
            trace_manifest.display().to_string(),
        );
    }
    if let Some(profile) = &config.policy.redaction_profile {
        env.insert(
            "RECOMP_POLICY_REDACTION_PROFILE".to_string(),
            profile.clone(),
        );
    }
    if !config.policy.allowed_models.is_empty() {
        env.insert(
            "RECOMP_POLICY_ALLOWED_MODELS".to_string(),
            config.policy.allowed_models.join(","),
        );
    }
    if !config.policy.run_windows.is_empty() {
        env.insert(
            "RECOMP_POLICY_RUN_WINDOWS".to_string(),
            config.policy.run_windows.join(","),
        );
    }
    if let Some(validation) = &config.reference.validation_config_toml {
        env.insert(
            "RECOMP_VALIDATION_CONFIG_TOML".to_string(),
            validation.display().to_string(),
        );
    }
    if let Some(input_script) = &config.reference.input_script_toml {
        env.insert(
            "RECOMP_INPUT_SCRIPT_TOML".to_string(),
            input_script.display().to_string(),
        );
    }
    env
}

fn write_step_logs(
    paths: &ResolvedPaths,
    name: &str,
    stdout: &str,
    stderr: &str,
) -> Result<(Option<PathBuf>, Option<PathBuf>), String> {
    let stdout_path = paths.log_dir.join(format!("{name}.stdout.log"));
    let stderr_path = paths.log_dir.join(format!("{name}.stderr.log"));
    fs::write(&stdout_path, stdout)
        .map_err(|err| format!("write stdout log {}: {err}", stdout_path.display()))?;
    fs::write(&stderr_path, stderr)
        .map_err(|err| format!("write stderr log {}: {err}", stderr_path.display()))?;
    Ok((Some(stdout_path), Some(stderr_path)))
}

fn record_artifact(
    state: &mut RunState,
    paths: &ResolvedPaths,
    path: &Path,
    role: &str,
) -> Result<String, String> {
    let (sha256, size) = hash_file(path)?;
    let stored_path = format_path(paths, path);
    state.artifacts.insert(
        stored_path.clone(),
        RunArtifact {
            path: stored_path.clone(),
            sha256,
            size,
            role: role.to_string(),
        },
    );
    Ok(stored_path)
}

fn finalize_manifest(state: &mut RunState) {
    state.manifest.artifacts = state
        .artifacts
        .values()
        .cloned()
        .collect::<Vec<RunArtifact>>();
    state.manifest.artifacts.sort_by(|a, b| a.path.cmp(&b.path));
}

fn format_path(paths: &ResolvedPaths, path: &Path) -> String {
    if let Ok(relative) = path.strip_prefix(&paths.config_dir) {
        return relative.to_string_lossy().to_string();
    }
    path.to_string_lossy().to_string()
}

fn outputs_exist(paths: &ResolvedPaths, step: &RunStep) -> bool {
    if step.outputs.is_empty() {
        return true;
    }
    step.outputs.iter().all(|stored| {
        let path = resolve_path(&paths.config_dir, Path::new(stored));
        path.exists()
    })
}

fn write_run_manifest(path: &Path, manifest: &RunManifest) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("create manifest dir {}: {err}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(manifest).map_err(|err| err.to_string())?;
    fs::write(path, json).map_err(|err| format!("write run manifest {}: {err}", path.display()))?;
    Ok(())
}

fn load_run_manifest(path: &Path) -> Result<RunManifest, String> {
    let src = fs::read_to_string(path)
        .map_err(|err| format!("read run manifest {}: {err}", path.display()))?;
    serde_json::from_str(&src).map_err(|err| format!("invalid run manifest: {err}"))
}

fn gather_inputs(
    config: &AutomationConfig,
    config_path: &Path,
    paths: &ResolvedPaths,
) -> Result<Vec<RunInput>, String> {
    let mut inputs = vec![
        run_input("automation_config", config_path)?,
        run_input("provenance", &config.inputs.provenance)?,
        run_input("title_config", &config.inputs.config)?,
        run_input("reference_video", &config.reference.reference_video_toml)?,
        run_input("capture_video", &config.reference.capture_video_toml)?,
    ];
    if let Some(validation) = &config.reference.validation_config_toml {
        inputs.push(run_input("validation_config", validation)?);
    }
    if let Some(input_script) = &config.reference.input_script_toml {
        inputs.push(run_input("input_script", input_script)?);
    }
    if let Some(path) = &config.analysis.name_map_json {
        if path.exists() {
            inputs.push(run_input("analysis_name_map_json", path)?);
        }
    }
    if let Some(path) = &config.analysis.runtime_trace_manifest {
        if path.exists() {
            inputs.push(run_input("analysis_runtime_trace_manifest", path)?);
        }
    }
    if let Some(path) = &config.inputs.module_json {
        inputs.push(run_input("module_json", path)?);
    }
    if let Some(path) = &config.inputs.nro {
        inputs.push(run_input("homebrew_nro", path)?);
    }
    if let Some(path) = &config.inputs.xci {
        inputs.push(run_input("xci", path)?);
    }
    if let Some(path) = &config.inputs.keys {
        inputs.push(run_input("keyset", path)?);
    }
    for (index, path) in config.inputs.nso.iter().enumerate() {
        inputs.push(run_input(&format!("homebrew_nso_{index}"), path)?);
    }
    if let Some(runtime_path) = &config.inputs.runtime_path {
        let cargo_toml = runtime_path.join("Cargo.toml");
        if cargo_toml.exists() {
            inputs.push(run_input("runtime_cargo", &cargo_toml)?);
        }
    } else {
        let default_runtime = paths.repo_root.join("crates/recomp-runtime/Cargo.toml");
        if default_runtime.exists() {
            inputs.push(run_input("runtime_cargo", &default_runtime)?);
        }
    }
    inputs.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(inputs)
}

fn run_input(name: &str, path: &Path) -> Result<RunInput, String> {
    let (sha256, size) = hash_file(path)?;
    Ok(RunInput {
        name: name.to_string(),
        path: path.to_string_lossy().to_string(),
        sha256,
        size,
    })
}

fn hash_file(path: &Path) -> Result<(String, u64), String> {
    let bytes = fs::read(path).map_err(|err| format!("read {}: {err}", path.display()))?;
    let size = bytes.len() as u64;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let digest = hasher.finalize();
    Ok((format!("{:x}", digest), size))
}

fn fingerprint_inputs(inputs: &[RunInput]) -> String {
    let mut hasher = Sha256::new();
    for input in inputs {
        if input.name == "automation_config" {
            continue;
        }
        hasher.update(input.name.as_bytes());
        hasher.update(b":");
        hasher.update(input.sha256.as_bytes());
        hasher.update(b":");
        hasher.update(input.size.to_string().as_bytes());
        hasher.update(b"\n");
    }
    let digest = hasher.finalize();
    format!("{:x}", digest)
}

fn stage_cache_key(
    name: &str,
    input_fingerprint: &str,
    command: &Option<Vec<String>>,
    state: &RunState,
    config: &AutomationConfig,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(name.as_bytes());
    hasher.update(b"\n");
    hasher.update(input_fingerprint.as_bytes());
    hasher.update(b"\n");
    hasher.update(state.manifest.execution_mode.to_string().as_bytes());
    hasher.update(b"\n");
    hasher.update(state.manifest.tool_versions.recomp_cli.as_bytes());
    hasher.update(b"\n");
    if let Some(rustc) = &state.manifest.tool_versions.rustc {
        hasher.update(rustc.as_bytes());
    }
    hasher.update(b"\n");
    if let Some(ffmpeg) = &state.manifest.tool_versions.ffmpeg {
        hasher.update(ffmpeg.as_bytes());
    }
    hasher.update(b"\n");
    if let Some(xci_tool) = &state.manifest.tool_versions.xci_tool {
        hasher.update(xci_tool.as_bytes());
    }
    hasher.update(b"\n");
    match command {
        Some(argv) => {
            for arg in argv {
                hasher.update(arg.as_bytes());
                hasher.update(&[0]);
            }
        }
        None => hasher.update(b"<none>"),
    }
    hasher.update(b"\n");
    hasher.update(stage_config_signature(name, config).as_bytes());
    let digest = hasher.finalize();
    format!("{:x}", digest)
}

fn stage_config_signature(name: &str, config: &AutomationConfig) -> String {
    match name {
        "intake" => format!(
            "mode={:?};nro={};nso={:?};xci={};keys={};xci_tool={:?};xci_tool_path={}",
            config.inputs.mode,
            config
                .inputs
                .nro
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            config
                .inputs
                .nso
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>(),
            config
                .inputs
                .xci
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            config
                .inputs
                .keys
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            config.tools.xci_tool,
            config
                .tools
                .xci_tool_path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_default()
        ),
        "analysis" => format!(
            "command={:?};expected_outputs={:?};name_map_json={};runtime_trace_manifest={}",
            config.analysis.command,
            config
                .analysis
                .expected_outputs
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>(),
            config
                .analysis
                .name_map_json
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            config
                .analysis
                .runtime_trace_manifest
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_default()
        ),
        "lift" => format!(
            "entry={};mode={:?};lift_command={:?}",
            config
                .run
                .lift_entry
                .clone()
                .unwrap_or_else(|| "entry".to_string()),
            config.run.lift_mode.unwrap_or(LiftModeConfig::Decode),
            config.commands.lift
        ),
        "pipeline" => format!(
            "runtime_path={};title_config={};provenance={}",
            config
                .inputs
                .runtime_path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "default-runtime".to_string()),
            config.inputs.config.display(),
            config.inputs.provenance.display()
        ),
        "build" => format!("build={:?}", config.commands.build),
        "run" => format!("run={:?}", config.commands.run),
        "capture" => format!(
            "capture={:?};video_path={};frames_dir={};audio_file={}",
            config.commands.capture,
            config.capture.video_path.display(),
            config.capture.frames_dir.display(),
            config
                .capture
                .audio_file
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_default()
        ),
        "extract_frames" => format!("extract_frames={:?}", config.commands.extract_frames),
        "extract_audio" => format!("extract_audio={:?}", config.commands.extract_audio),
        "hash_frames" => format!(
            "capture_video_toml={};frames_dir={}",
            config.reference.capture_video_toml.display(),
            config.capture.frames_dir.display()
        ),
        "hash_audio" => format!(
            "capture_video_toml={};audio_file={}",
            config.reference.capture_video_toml.display(),
            config
                .capture
                .audio_file
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_default()
        ),
        "validate" => format!(
            "reference={};capture={};validation={}",
            config.reference.reference_video_toml.display(),
            config.reference.capture_video_toml.display(),
            config
                .reference
                .validation_config_toml
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_default()
        ),
        _ => String::new(),
    }
}

fn gather_tool_versions(config: &AutomationConfig) -> ToolVersions {
    let rustc = Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
            } else {
                None
            }
        });
    ToolVersions {
        recomp_cli: env!("CARGO_PKG_VERSION").to_string(),
        rustc,
        ffmpeg: config
            .tools
            .ffmpeg_path
            .as_ref()
            .map(|path| path.display().to_string()),
        xci_tool: config.tools.xci_tool.map(|tool| match tool {
            AutomationXciTool::Auto => "auto".to_string(),
            AutomationXciTool::Hactool => "hactool".to_string(),
            AutomationXciTool::Hactoolnet => "hactoolnet".to_string(),
            AutomationXciTool::Mock => "mock".to_string(),
        }),
    }
}

fn fingerprint_host() -> String {
    let host = std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(host.as_bytes());
    hasher.update(b"\n");
    hasher.update(std::env::consts::OS.as_bytes());
    hasher.update(b"\n");
    hasher.update(std::env::consts::ARCH.as_bytes());
    let digest = hasher.finalize();
    format!("{:x}", digest)
}

fn derive_run_id(
    input_fingerprint: &str,
    execution_mode: ExecutionMode,
    work_root: &Path,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input_fingerprint.as_bytes());
    hasher.update(b"\n");
    hasher.update(execution_mode.to_string().as_bytes());
    hasher.update(b"\n");
    hasher.update(work_root.display().to_string().as_bytes());
    let digest = hasher.finalize();
    let full = format!("{:x}", digest);
    format!("run-{}", &full[..16])
}

fn resolve_path(base_dir: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_dir.join(path)
    }
}

fn repo_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(|path| path.parent())
        .unwrap_or(&manifest_dir)
        .to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn automation_runs_with_lifted_module() {
        let repo_root = repo_root();
        let temp = tempdir().expect("tempdir");
        let work_root = temp.path().join("work");
        let capture_dir = temp.path().join("capture");
        let frames_dir = capture_dir.join("frames");
        fs::create_dir_all(&frames_dir).expect("frames dir");

        let frame_a = frames_dir.join("00000001.png");
        let frame_b = frames_dir.join("00000002.png");
        fs::write(&frame_a, b"frame-one").expect("write frame a");
        fs::write(&frame_b, b"frame-two").expect("write frame b");

        let reference_hashes = hash_frames_dir(&frames_dir).expect("hash frames");
        let reference_hash_path = temp.path().join("reference_frames.hashes");
        write_hash_list(&reference_hash_path, &reference_hashes).expect("write ref hashes");

        let capture_hash_path = capture_dir.join("frames.hashes");
        let capture_video_path = capture_dir.join("capture.mp4");
        fs::write(&capture_video_path, b"").expect("write capture video");

        let reference_toml = format!(
            r#"schema_version = "2"

[video]
path = "reference.mp4"
width = 1280
height = 720
fps = 30.0

[timeline]
start = "00:00:00.000"
end = "00:00:00.067"

[hashes.frames]
format = "list"
path = "{}"
"#,
            reference_hash_path.display()
        );
        let capture_toml = format!(
            r#"schema_version = "1"

[video]
path = "{}"
width = 1280
height = 720
fps = 30.0

[hashes.frames]
format = "list"
path = "{}"
"#,
            capture_video_path.display(),
            capture_hash_path.display()
        );
        let reference_path = temp.path().join("reference_video.toml");
        let capture_path = temp.path().join("capture_video.toml");
        fs::write(&reference_path, reference_toml).expect("write reference config");
        fs::write(&capture_path, capture_toml).expect("write capture config");

        let automation_path = temp.path().join("automation.toml");
        let automation_toml = format!(
            r#"schema_version = "1"

[inputs]
mode = "lifted"
module_json = "{}"
provenance = "{}"
config = "{}"
runtime_path = "{}"

[outputs]
work_root = "{}"

[reference]
reference_video_toml = "{}"
capture_video_toml = "{}"

[capture]
video_path = "{}"
frames_dir = "{}"

[commands]
build = ["/usr/bin/true"]
run = ["/usr/bin/true"]
capture = ["/usr/bin/true"]
extract_frames = ["/usr/bin/true"]
"#,
            repo_root.join("samples/minimal/module.json").display(),
            repo_root.join("samples/minimal/provenance.toml").display(),
            repo_root.join("samples/minimal/title.toml").display(),
            repo_root.join("crates/recomp-runtime").display(),
            work_root.display(),
            reference_path.display(),
            capture_path.display(),
            capture_video_path.display(),
            frames_dir.display()
        );
        fs::write(&automation_path, &automation_toml).expect("write automation config");

        let manifest = run_automation(&automation_path).expect("run automation");
        assert_eq!(manifest.input_fingerprint.len(), 64);
        assert!(manifest.run_id.starts_with("run-"));
        assert_eq!(manifest.execution_mode, ExecutionMode::Local);
        assert_eq!(manifest.host_fingerprint.len(), 64);
        assert!(manifest.steps.iter().any(|step| step.name == "pipeline"));
        assert!(manifest.steps.iter().all(|step| !step.cache_hit));
        assert!(manifest.steps.iter().all(|step| step.stage_attempt >= 1));
        assert!(paths_exist(&manifest, temp.path()));

        let manifest_again = run_automation(&automation_path).expect("run automation again");
        assert_eq!(manifest.input_fingerprint, manifest_again.input_fingerprint);
        let cache_misses: Vec<_> = manifest_again
            .steps
            .iter()
            .filter(|step| !step.cache_hit)
            .map(|step| step.name.clone())
            .collect();
        assert!(
            cache_misses.is_empty(),
            "expected all cache hits, misses: {cache_misses:?}"
        );
    }

    #[test]
    fn automation_command_change_invalidates_dependent_stages_only() {
        let repo_root = repo_root();
        let temp = tempdir().expect("tempdir");
        let work_root = temp.path().join("work");
        let capture_dir = temp.path().join("capture");
        let frames_dir = capture_dir.join("frames");
        fs::create_dir_all(&frames_dir).expect("frames dir");

        let frame_a = frames_dir.join("00000001.png");
        let frame_b = frames_dir.join("00000002.png");
        fs::write(&frame_a, b"frame-one").expect("write frame a");
        fs::write(&frame_b, b"frame-two").expect("write frame b");

        let reference_hashes = hash_frames_dir(&frames_dir).expect("hash frames");
        let reference_hash_path = temp.path().join("reference_frames.hashes");
        write_hash_list(&reference_hash_path, &reference_hashes).expect("write ref hashes");

        let capture_hash_path = capture_dir.join("frames.hashes");
        let capture_video_path = capture_dir.join("capture.mp4");
        fs::write(&capture_video_path, b"").expect("write capture video");

        let reference_toml = format!(
            r#"schema_version = "2"

[video]
path = "reference.mp4"
width = 1280
height = 720
fps = 30.0

[timeline]
start = "00:00:00.000"
end = "00:00:00.067"

[hashes.frames]
format = "list"
path = "{}"
"#,
            reference_hash_path.display()
        );
        let capture_toml = format!(
            r#"schema_version = "1"

[video]
path = "{}"
width = 1280
height = 720
fps = 30.0

[hashes.frames]
format = "list"
path = "{}"
"#,
            capture_video_path.display(),
            capture_hash_path.display()
        );
        let reference_path = temp.path().join("reference_video.toml");
        let capture_path = temp.path().join("capture_video.toml");
        fs::write(&reference_path, reference_toml).expect("write reference config");
        fs::write(&capture_path, capture_toml).expect("write capture config");

        let automation_path = temp.path().join("automation.toml");
        let automation_toml = format!(
            r#"schema_version = "1"

[inputs]
mode = "lifted"
module_json = "{}"
provenance = "{}"
config = "{}"
runtime_path = "{}"

[outputs]
work_root = "{}"

[reference]
reference_video_toml = "{}"
capture_video_toml = "{}"

[capture]
video_path = "{}"
frames_dir = "{}"

[commands]
build = ["/usr/bin/true"]
run = ["/usr/bin/true"]
capture = ["/usr/bin/true"]
extract_frames = ["/usr/bin/true"]
"#,
            repo_root.join("samples/minimal/module.json").display(),
            repo_root.join("samples/minimal/provenance.toml").display(),
            repo_root.join("samples/minimal/title.toml").display(),
            repo_root.join("crates/recomp-runtime").display(),
            work_root.display(),
            reference_path.display(),
            capture_path.display(),
            capture_video_path.display(),
            frames_dir.display()
        );
        fs::write(&automation_path, &automation_toml).expect("write automation config");

        let manifest_first = run_automation(&automation_path).expect("first automation run");
        assert!(manifest_first.steps.iter().all(|step| !step.cache_hit));

        let updated = automation_toml.replace(
            "run = [\"/usr/bin/true\"]",
            "run = [\"/usr/bin/printf\", \"\"]",
        );
        fs::write(&automation_path, updated).expect("update automation config");

        let manifest_second = run_automation(&automation_path).expect("second automation run");
        let mut by_name = HashMap::new();
        for step in &manifest_second.steps {
            by_name.insert(step.name.clone(), step.cache_hit);
        }
        assert_eq!(by_name.get("pipeline"), Some(&true));
        assert_eq!(by_name.get("build"), Some(&true));
        assert_eq!(by_name.get("run"), Some(&false));
        assert_eq!(by_name.get("capture"), Some(&false));
        assert_eq!(by_name.get("extract_frames"), Some(&false));
        assert_eq!(by_name.get("hash_frames"), Some(&false));
        assert_eq!(by_name.get("validate"), Some(&false));
    }

    #[test]
    fn automation_failure_then_resume_reuses_upstream_steps() {
        let repo_root = repo_root();
        let temp = tempdir().expect("tempdir");
        let work_root = temp.path().join("work");
        let capture_dir = temp.path().join("capture");
        let frames_dir = capture_dir.join("frames");
        fs::create_dir_all(&frames_dir).expect("frames dir");

        let frame_a = frames_dir.join("00000001.png");
        let frame_b = frames_dir.join("00000002.png");
        fs::write(&frame_a, b"frame-one").expect("write frame a");
        fs::write(&frame_b, b"frame-two").expect("write frame b");

        let reference_hashes = hash_frames_dir(&frames_dir).expect("hash frames");
        let reference_hash_path = temp.path().join("reference_frames.hashes");
        write_hash_list(&reference_hash_path, &reference_hashes).expect("write ref hashes");

        let capture_hash_path = capture_dir.join("frames.hashes");
        let capture_video_path = capture_dir.join("capture.mp4");
        fs::write(&capture_video_path, b"").expect("write capture video");

        let reference_toml = format!(
            r#"schema_version = "2"

[video]
path = "reference.mp4"
width = 1280
height = 720
fps = 30.0

[timeline]
start = "00:00:00.000"
end = "00:00:00.067"

[hashes.frames]
format = "list"
path = "{}"
"#,
            reference_hash_path.display()
        );
        let capture_toml = format!(
            r#"schema_version = "1"

[video]
path = "{}"
width = 1280
height = 720
fps = 30.0

[hashes.frames]
format = "list"
path = "{}"
"#,
            capture_video_path.display(),
            capture_hash_path.display()
        );
        let reference_path = temp.path().join("reference_video.toml");
        let capture_path = temp.path().join("capture_video.toml");
        fs::write(&reference_path, reference_toml).expect("write reference config");
        fs::write(&capture_path, capture_toml).expect("write capture config");

        let automation_path = temp.path().join("automation.toml");
        let failing_automation_toml = format!(
            r#"schema_version = "1"

[inputs]
mode = "lifted"
module_json = "{}"
provenance = "{}"
config = "{}"
runtime_path = "{}"

[outputs]
work_root = "{}"

[reference]
reference_video_toml = "{}"
capture_video_toml = "{}"

[capture]
video_path = "{}"
frames_dir = "{}"

[commands]
build = ["/usr/bin/true"]
run = ["/usr/bin/false"]
capture = ["/usr/bin/true"]
extract_frames = ["/usr/bin/true"]
"#,
            repo_root.join("samples/minimal/module.json").display(),
            repo_root.join("samples/minimal/provenance.toml").display(),
            repo_root.join("samples/minimal/title.toml").display(),
            repo_root.join("crates/recomp-runtime").display(),
            work_root.display(),
            reference_path.display(),
            capture_path.display(),
            capture_video_path.display(),
            frames_dir.display()
        );
        fs::write(&automation_path, &failing_automation_toml).expect("write failing config");

        let err = run_automation(&automation_path).expect_err("first run should fail");
        assert!(err.contains("command failed"), "unexpected error: {err}");

        let failed_manifest =
            load_run_manifest(&work_root.join("run-manifest.json")).expect("load failed manifest");
        assert_eq!(
            failed_manifest
                .steps
                .last()
                .expect("at least one step")
                .name,
            "run"
        );
        assert_eq!(
            failed_manifest
                .steps
                .last()
                .expect("at least one step")
                .status,
            StepStatus::Failed
        );

        let fixed = failing_automation_toml
            .replace("run = [\"/usr/bin/false\"]", "run = [\"/usr/bin/true\"]");
        fs::write(&automation_path, fixed).expect("write fixed config");

        let resumed_manifest = run_automation(&automation_path).expect("resume run");
        let mut by_name = HashMap::new();
        for step in &resumed_manifest.steps {
            by_name.insert(step.name.clone(), step.cache_hit);
        }
        assert_eq!(by_name.get("pipeline"), Some(&true));
        assert_eq!(by_name.get("build"), Some(&true));
        assert_eq!(by_name.get("run"), Some(&false));
        assert_eq!(by_name.get("capture"), Some(&false));
        assert_eq!(by_name.get("extract_frames"), Some(&false));
        assert_eq!(by_name.get("hash_frames"), Some(&false));
        assert_eq!(by_name.get("validate"), Some(&false));
    }

    fn paths_exist(manifest: &RunManifest, base: &Path) -> bool {
        for artifact in &manifest.artifacts {
            let path = resolve_path(base, Path::new(&artifact.path));
            if !path.exists() {
                return false;
            }
        }
        true
    }
}
