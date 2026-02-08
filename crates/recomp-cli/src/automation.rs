use recomp_pipeline::homebrew::{
    intake_homebrew, lift_homebrew, IntakeOptions, LiftMode, LiftOptions,
};
use recomp_pipeline::xci::{intake_xci, XciIntakeOptions, XciToolPreference};
use recomp_pipeline::{run_pipeline, PipelineOptions};
use recomp_validation::{
    hash_audio_file, hash_frames_dir, run_video_suite, write_hash_list, CaptureVideoConfig,
    HashFormat, ReferenceVideoConfig, Timecode,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

const AUTOMATION_SCHEMA_VERSION_V1: &str = "1";
const AUTOMATION_SCHEMA_VERSION_V2: &str = "2";
const RUN_MANIFEST_SCHEMA_VERSION: &str = "2";
const ATTEMPT_MANIFEST_SCHEMA_VERSION: &str = "1";
const RUN_SUMMARY_SCHEMA_VERSION: &str = "1";
const STRATEGY_CATALOG_SCHEMA_VERSION: &str = "1";

const DEFAULT_MAX_RETRIES: usize = 5;
const DEFAULT_MAX_RUNTIME_MINUTES: u64 = 120;
const DEFAULT_AUDIO_RATE: u32 = 48_000;

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
    pub run: RunConfig,
    #[serde(default, rename = "loop")]
    pub loop_config: LoopConfig,
    #[serde(default)]
    pub gates: GatesConfig,
    #[serde(default)]
    pub agent: AgentConfig,
    #[serde(default)]
    pub cloud: CloudConfig,
    #[serde(default)]
    pub scenes: Vec<SceneConfig>,
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
    #[serde(default)]
    pub ghidra: Option<GhidraConfig>,
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

#[derive(Debug, Deserialize, Clone, Default)]
pub struct GhidraConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub headless_path: Option<PathBuf>,
    #[serde(default)]
    pub project_root: Option<PathBuf>,
    #[serde(default)]
    pub project_name: Option<String>,
    #[serde(default)]
    pub script_path: Option<PathBuf>,
    #[serde(default)]
    pub pre_script: Option<String>,
    #[serde(default)]
    pub post_script: Option<String>,
    #[serde(default)]
    pub target_binary: Option<PathBuf>,
    #[serde(default)]
    pub language_id: Option<String>,
    #[serde(default)]
    pub analysis_timeout_sec: Option<u64>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct RunConfig {
    #[serde(default = "default_resume")]
    pub resume: bool,
    #[serde(default)]
    pub lift_entry: Option<String>,
    #[serde(default)]
    pub lift_mode: Option<LiftModeConfig>,
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

#[derive(Debug, Deserialize, Clone)]
pub struct LoopConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_max_retries")]
    pub max_retries: usize,
    #[serde(default = "default_max_runtime_minutes")]
    pub max_runtime_minutes: u64,
    #[serde(default = "default_strategy_order")]
    pub strategy_order: Vec<String>,
    #[serde(default = "default_stop_on_first_pass")]
    pub stop_on_first_pass: bool,
    #[serde(default)]
    pub strategy_catalog_toml: Option<PathBuf>,
}

impl Default for LoopConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_retries: default_max_retries(),
            max_runtime_minutes: default_max_runtime_minutes(),
            strategy_order: default_strategy_order(),
            stop_on_first_pass: default_stop_on_first_pass(),
            strategy_catalog_toml: None,
        }
    }
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct GatesConfig {
    #[serde(default)]
    pub hash: HashGateConfig,
    #[serde(default)]
    pub perceptual: PerceptualGateConfig,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct HashGateConfig {
    #[serde(default)]
    pub frame_match_ratio_min: Option<f32>,
    #[serde(default)]
    pub audio_match_ratio_min: Option<f32>,
    #[serde(default)]
    pub max_drift_frames: Option<i32>,
    #[serde(default)]
    pub max_audio_drift_chunks: Option<i32>,
    #[serde(default)]
    pub max_dropped_frames: Option<usize>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PerceptualGateConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_ssim_min")]
    pub ssim_min: f32,
    #[serde(default = "default_psnr_min")]
    pub psnr_min: f32,
    #[serde(default = "default_vmaf_min")]
    pub vmaf_min: f32,
    #[serde(default = "default_audio_lufs_delta_max")]
    pub audio_lufs_delta_max: f32,
    #[serde(default = "default_audio_peak_delta_max")]
    pub audio_peak_delta_max: f32,
    #[serde(default)]
    pub require_vmaf: bool,
    #[serde(default = "default_audio_rate")]
    pub audio_rate: u32,
    #[serde(default)]
    pub offset_seconds: f64,
}

impl Default for PerceptualGateConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            ssim_min: default_ssim_min(),
            psnr_min: default_psnr_min(),
            vmaf_min: default_vmaf_min(),
            audio_lufs_delta_max: default_audio_lufs_delta_max(),
            audio_peak_delta_max: default_audio_peak_delta_max(),
            require_vmaf: false,
            audio_rate: default_audio_rate(),
            offset_seconds: 0.0,
        }
    }
}

#[derive(Debug, Deserialize, Clone, Default, Serialize)]
pub struct AgentConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub reasoning_effort: Option<String>,
    #[serde(default)]
    pub max_cost_usd: Option<f64>,
    #[serde(default)]
    pub approval_mode: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct CloudConfig {
    #[serde(default)]
    pub mode: CloudMode,
    #[serde(default)]
    pub artifact_uri: Option<String>,
    #[serde(default)]
    pub queue_name: Option<String>,
    #[serde(default)]
    pub state_machine_arn: Option<String>,
}

impl Default for CloudConfig {
    fn default() -> Self {
        Self {
            mode: CloudMode::Local,
            artifact_uri: None,
            queue_name: None,
            state_machine_arn: None,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CloudMode {
    Local,
    AwsHybrid,
}

impl Default for CloudMode {
    fn default() -> Self {
        Self::Local
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct SceneConfig {
    pub id: String,
    pub start: String,
    pub end: String,
    #[serde(default)]
    pub input_marker_start: Option<String>,
    #[serde(default)]
    pub input_marker_end: Option<String>,
    #[serde(default = "default_scene_weight")]
    pub weight: f32,
}

#[derive(Debug, Deserialize, Clone)]
struct StrategyCatalog {
    #[serde(default)]
    schema_version: Option<String>,
    #[serde(default)]
    strategy: Vec<StrategyCatalogEntry>,
}

#[derive(Debug, Deserialize, Clone)]
struct StrategyCatalogEntry {
    id: String,
    #[serde(default = "default_strategy_enabled")]
    enabled: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RunManifest {
    pub schema_version: String,
    pub input_fingerprint: String,
    #[serde(default)]
    pub inputs: Vec<RunInput>,
    #[serde(default)]
    pub steps: Vec<RunStep>,
    #[serde(default)]
    pub artifacts: Vec<RunArtifact>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation_report: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attempts: Vec<AttemptRecord>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub winning_attempt: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_status: Option<RunFinalStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategy_catalog: Option<String>,
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
pub struct AttemptRecord {
    pub attempt: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategy: Option<StrategyKind>,
    pub status: AttemptStatus,
    pub attempt_manifest: String,
    pub gate_results: String,
    pub triage: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AttemptManifest {
    pub schema_version: String,
    pub attempt: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategy: Option<StrategyKind>,
    pub status: AttemptStatus,
    pub started_at: String,
    pub duration_ms: u128,
    pub run_manifest: RunManifest,
    pub gate_results: GateResults,
    pub triage: TriageReport,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ghidra_evidence: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GateResults {
    pub schema_version: String,
    pub hash: HashGateResult,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub perceptual: Option<PerceptualGateResult>,
    pub status: AttemptStatus,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HashGateResult {
    pub passed: bool,
    pub failed_cases: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame_match_ratio: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame_drift_frames: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame_length_delta: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_match_ratio: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_drift_chunks: Option<i32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub failures: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drift_seconds_hint: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PerceptualGateResult {
    pub enabled: bool,
    pub passed: bool,
    pub weighted_score: f32,
    pub total_weight: f32,
    pub passed_weight: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failing_scene: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scenes: Vec<PerceptualSceneResult>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PerceptualSceneResult {
    pub id: String,
    pub weight: f32,
    pub passed: bool,
    pub summary_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssim_avg: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub psnr_avg: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vmaf_avg: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_lufs_delta: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_peak_delta: Option<f32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub failures: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TriageReport {
    pub schema_version: String,
    pub attempt: usize,
    pub status: AttemptStatus,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub categories: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub findings: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub suggested_actions: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_strategy: Option<StrategyKind>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RunSummary {
    pub schema_version: String,
    pub input_fingerprint: String,
    pub status: RunFinalStatus,
    pub attempts: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub winning_attempt: Option<usize>,
    pub duration_ms: u128,
    pub cloud: CloudConfig,
    pub agent: AgentConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AttemptStatus {
    Passed,
    Failed,
    NeedsReview,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RunFinalStatus {
    Passed,
    Failed,
    NeedsReview,
    Exhausted,
}

#[derive(Debug)]
struct RunState {
    manifest: RunManifest,
    artifacts: BTreeMap<String, RunArtifact>,
    previous_steps: HashMap<String, RunStep>,
    cache_valid: bool,
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
    attempts_root: PathBuf,
    run_summary: PathBuf,
}

#[derive(Debug)]
struct AttemptExecution {
    manifest: RunManifest,
    status: AttemptStatus,
    hash_gate: HashGateResult,
    triage: TriageReport,
    attempt_manifest_path: PathBuf,
    gate_results_path: PathBuf,
    triage_path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StrategyKind {
    CaptureAlignmentProfile,
    InputTimingVariant,
    ServiceStubProfileSwitch,
    PatchSetVariant,
    LiftModeVariant,
    RuntimeModeVariant,
}

impl StrategyKind {
    fn id(&self) -> &'static str {
        match self {
            Self::CaptureAlignmentProfile => "capture_alignment_profile",
            Self::InputTimingVariant => "input_timing_variant",
            Self::ServiceStubProfileSwitch => "service_stub_profile_switch",
            Self::PatchSetVariant => "patch_set_variant",
            Self::LiftModeVariant => "lift_mode_variant",
            Self::RuntimeModeVariant => "runtime_mode_variant",
        }
    }

    fn from_id(value: &str) -> Option<Self> {
        match value {
            "capture_alignment_profile" => Some(Self::CaptureAlignmentProfile),
            "input_timing_variant" => Some(Self::InputTimingVariant),
            "service_stub_profile_switch" => Some(Self::ServiceStubProfileSwitch),
            "patch_set_variant" => Some(Self::PatchSetVariant),
            "lift_mode_variant" => Some(Self::LiftModeVariant),
            "runtime_mode_variant" => Some(Self::RuntimeModeVariant),
            _ => None,
        }
    }

    fn min_stage(self) -> AttemptStage {
        match self {
            Self::CaptureAlignmentProfile => AttemptStage::ValidatePerceptual,
            Self::InputTimingVariant => AttemptStage::Run,
            Self::ServiceStubProfileSwitch => AttemptStage::Pipeline,
            Self::PatchSetVariant => AttemptStage::Pipeline,
            Self::LiftModeVariant => AttemptStage::Lift,
            Self::RuntimeModeVariant => AttemptStage::Run,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum AttemptStage {
    Intake,
    Ghidra,
    Lift,
    Pipeline,
    Build,
    Run,
    Capture,
    Hash,
    ValidateHash,
    ValidatePerceptual,
    Triage,
}

#[derive(Debug)]
struct MutationState {
    strategy_counts: HashMap<StrategyKind, usize>,
    perceptual_offset_seconds: f64,
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

    let paths = ResolvedPaths::new(&config, config_dir)?;
    fs::create_dir_all(&paths.work_root)
        .map_err(|err| format!("create work root {}: {err}", paths.work_root.display()))?;
    fs::create_dir_all(&paths.attempts_root).map_err(|err| {
        format!(
            "create attempts root {}: {err}",
            paths.attempts_root.display()
        )
    })?;

    let inputs = gather_inputs(&config, &config_path, &paths)?;
    let input_fingerprint = fingerprint_inputs(&inputs);

    if config.run.resume && paths.run_manifest.exists() {
        if let Ok(previous) = load_run_manifest(&paths.run_manifest) {
            if previous.input_fingerprint == input_fingerprint
                && previous.final_status == Some(RunFinalStatus::Passed)
                && manifest_outputs_exist(&paths, &previous)
            {
                return Ok(previous);
            }
        }
    }

    let strategy_order = resolve_strategy_order(&config)?;
    let max_attempts = if config.loop_config.enabled {
        1 + config.loop_config.max_retries
    } else {
        1
    };

    let started = Instant::now();
    let mut mutation_state = MutationState {
        strategy_counts: HashMap::new(),
        perceptual_offset_seconds: config.gates.perceptual.offset_seconds,
    };

    let mut attempts = Vec::new();
    let mut used_strategies = HashSet::new();
    let mut previous_attempt_manifest: Option<RunManifest> = None;
    let mut last_attempt: Option<AttemptExecution> = None;
    let mut current_config = config.clone();

    for attempt in 0..max_attempts {
        if config.loop_config.enabled
            && started.elapsed() > Duration::from_secs(config.loop_config.max_runtime_minutes * 60)
        {
            break;
        }

        let strategy = if attempt == 0 {
            None
        } else {
            select_next_strategy(
                &strategy_order,
                last_attempt.as_ref(),
                &used_strategies,
                &current_config,
            )
        };

        if attempt > 0 {
            let Some(strategy_kind) = strategy else {
                break;
            };
            apply_strategy(
                strategy_kind,
                &mut current_config,
                &paths,
                attempt,
                &mut mutation_state,
                last_attempt.as_ref(),
            )?;
            used_strategies.insert(strategy_kind);
        }

        let attempt_exec = run_single_attempt(
            &current_config,
            &paths,
            &input_fingerprint,
            attempt,
            strategy,
            previous_attempt_manifest.as_ref(),
            mutation_state.perceptual_offset_seconds,
        )?;

        attempts.push(AttemptRecord {
            attempt,
            strategy,
            status: attempt_exec.status,
            attempt_manifest: format_path(&paths, &attempt_exec.attempt_manifest_path),
            gate_results: format_path(&paths, &attempt_exec.gate_results_path),
            triage: format_path(&paths, &attempt_exec.triage_path),
        });

        previous_attempt_manifest = Some(attempt_exec.manifest.clone());
        let attempt_status = attempt_exec.status;
        last_attempt = Some(attempt_exec);

        if attempt_status == AttemptStatus::Passed && config.loop_config.stop_on_first_pass {
            break;
        }
    }

    let Some(last) = last_attempt else {
        return Err("automation produced no attempts".to_string());
    };

    let final_status = if attempts
        .iter()
        .any(|attempt| attempt.status == AttemptStatus::Passed)
    {
        RunFinalStatus::Passed
    } else if attempts
        .iter()
        .any(|attempt| attempt.status == AttemptStatus::NeedsReview)
    {
        if attempts.len() >= max_attempts {
            RunFinalStatus::NeedsReview
        } else {
            RunFinalStatus::Exhausted
        }
    } else if attempts.len() >= max_attempts {
        RunFinalStatus::Exhausted
    } else {
        RunFinalStatus::Failed
    };

    let winning_attempt = attempts
        .iter()
        .find(|attempt| attempt.status == AttemptStatus::Passed)
        .map(|attempt| attempt.attempt);

    let run_summary = RunSummary {
        schema_version: RUN_SUMMARY_SCHEMA_VERSION.to_string(),
        input_fingerprint: input_fingerprint.clone(),
        status: final_status,
        attempts: attempts.len(),
        winning_attempt,
        duration_ms: started.elapsed().as_millis(),
        cloud: current_config.cloud.clone(),
        agent: current_config.agent.clone(),
    };
    write_json(&paths.run_summary, &run_summary)?;

    let mut run_manifest = last.manifest;
    run_manifest.schema_version = RUN_MANIFEST_SCHEMA_VERSION.to_string();
    run_manifest.inputs = inputs;
    run_manifest.input_fingerprint = input_fingerprint;
    run_manifest.attempts = attempts;
    run_manifest.winning_attempt = winning_attempt;
    run_manifest.final_status = Some(final_status);
    run_manifest.run_summary = Some(format_path(&paths, &paths.run_summary));
    run_manifest.strategy_catalog = current_config
        .loop_config
        .strategy_catalog_toml
        .as_ref()
        .map(|path| format_path(&paths, path));

    write_run_manifest(&paths.run_manifest, &run_manifest)?;
    Ok(run_manifest)
}

fn run_single_attempt(
    config: &AutomationConfig,
    root_paths: &ResolvedPaths,
    input_fingerprint: &str,
    attempt: usize,
    strategy: Option<StrategyKind>,
    previous_manifest: Option<&RunManifest>,
    perceptual_offset_seconds: f64,
) -> Result<AttemptExecution, String> {
    let attempt_started_at = chrono_stamp();
    let attempt_started = Instant::now();
    let attempt_root = root_paths.attempts_root.join(format!("{attempt:03}"));
    let attempt_log_dir = attempt_root.join("logs");
    let attempt_validation_dir = attempt_root.join("validation");
    let attempt_manifest_path = attempt_root.join("attempt-manifest.json");
    let gate_results_path = attempt_root.join("gate-results.json");
    let triage_path = attempt_root.join("triage.json");

    fs::create_dir_all(&attempt_log_dir).map_err(|err| {
        format!(
            "create attempt log dir {}: {err}",
            attempt_log_dir.display()
        )
    })?;
    fs::create_dir_all(&attempt_validation_dir).map_err(|err| {
        format!(
            "create attempt validation dir {}: {err}",
            attempt_validation_dir.display()
        )
    })?;

    let paths = root_paths.clone_for_attempt(
        attempt_root.clone(),
        attempt_log_dir,
        attempt_validation_dir,
        attempt_root.join("run-manifest.json"),
    );

    let inputs = gather_inputs_from_config(config, &paths)?;

    let mut artifacts = BTreeMap::new();
    let mut previous_steps = HashMap::new();
    if let Some(previous) = previous_manifest {
        for artifact in &previous.artifacts {
            artifacts.insert(artifact.path.clone(), artifact.clone());
        }
        for step in &previous.steps {
            previous_steps.insert(step.name.clone(), step.clone());
        }
    }

    let mut state = RunState {
        manifest: RunManifest {
            schema_version: RUN_MANIFEST_SCHEMA_VERSION.to_string(),
            input_fingerprint: input_fingerprint.to_string(),
            inputs: inputs.clone(),
            steps: Vec::new(),
            artifacts: Vec::new(),
            validation_report: None,
            attempts: Vec::new(),
            winning_attempt: None,
            final_status: None,
            run_summary: None,
            strategy_catalog: None,
        },
        artifacts,
        previous_steps,
        cache_valid: previous_manifest.is_some() && config.run.resume,
    };

    let reuse_before_stage = strategy.map(StrategyKind::min_stage);

    let mut module_json_path = match config.inputs.mode {
        InputMode::Lifted => config
            .inputs
            .module_json
            .clone()
            .ok_or_else(|| "inputs.module_json is required for mode=lifted".to_string())?,
        _ => paths.intake_dir.join("module.json"),
    };

    let allow_cache = |stage: AttemptStage| -> bool {
        reuse_before_stage
            .map(|reuse_stage| stage < reuse_stage)
            .unwrap_or(false)
    };

    if matches!(config.inputs.mode, InputMode::Homebrew | InputMode::Xci) {
        run_cached_step(
            "intake",
            &paths,
            &mut state,
            None,
            allow_cache(AttemptStage::Intake),
            true,
            |state| {
                let outcome = match config.inputs.mode {
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
                        return Err("intake step not valid for mode=lifted".to_string())
                    }
                };
                Ok(outcome)
            },
        )?;
    }

    run_cached_step(
        "ghidra_analysis",
        &paths,
        &mut state,
        None,
        allow_cache(AttemptStage::Ghidra),
        true,
        |state| run_ghidra_stage(config, &paths, state, attempt),
    )?;

    if matches!(config.inputs.mode, InputMode::Homebrew | InputMode::Xci) {
        run_cached_step(
            "lift",
            &paths,
            &mut state,
            None,
            allow_cache(AttemptStage::Lift),
            true,
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
                    let (stdout, stderr) = run_command(&lift_command, &paths, config)?;
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

    run_cached_step(
        "pipeline",
        &paths,
        &mut state,
        None,
        allow_cache(AttemptStage::Pipeline),
        true,
        |state| {
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
        },
    )?;

    run_cached_step(
        "build",
        &paths,
        &mut state,
        Some(config.commands.build.clone()),
        allow_cache(AttemptStage::Build),
        true,
        |_state| {
            let (stdout, stderr) = run_command(&config.commands.build, &paths, config)?;
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
        &mut state,
        Some(config.commands.run.clone()),
        allow_cache(AttemptStage::Run),
        true,
        |_state| {
            let (stdout, stderr) = run_command(&config.commands.run, &paths, config)?;
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
        &mut state,
        Some(config.commands.capture.clone()),
        allow_cache(AttemptStage::Capture),
        true,
        |state| {
            let (stdout, stderr) = run_command(&config.commands.capture, &paths, config)?;
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
        &mut state,
        Some(config.commands.extract_frames.clone()),
        allow_cache(AttemptStage::Hash),
        true,
        |_state| {
            let (stdout, stderr) = run_command(&config.commands.extract_frames, &paths, config)?;
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
            &mut state,
            Some(command.clone()),
            allow_cache(AttemptStage::Hash),
            true,
            |state| {
                let (stdout, stderr) = run_command(&command, &paths, config)?;
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

    run_cached_step(
        "hash_frames",
        &paths,
        &mut state,
        None,
        allow_cache(AttemptStage::Hash),
        true,
        |state| {
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
        },
    )?;

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
        run_cached_step(
            "hash_audio",
            &paths,
            &mut state,
            None,
            allow_cache(AttemptStage::Hash),
            true,
            |state| {
                let hashes = hash_audio_file(&audio_file)
                    .map_err(|err| format!("hash audio failed: {err}"))?;
                write_hash_list(&audio_hash_path, &hashes)
                    .map_err(|err| format!("write audio hashes: {err}"))?;
                let output = record_artifact(state, &paths, &audio_hash_path, "audio_hashes")?;
                Ok(StepOutcome {
                    status: StepStatus::Succeeded,
                    stdout: format!("audio hashes written ({})", hashes.len()),
                    stderr: String::new(),
                    outputs: vec![output],
                })
            },
        )?;
    }

    let mut hash_gate = HashGateResult {
        passed: false,
        failed_cases: 1,
        frame_match_ratio: None,
        frame_drift_frames: None,
        frame_length_delta: None,
        audio_match_ratio: None,
        audio_drift_chunks: None,
        failures: vec!["hash validation did not run".to_string()],
        report_path: None,
        drift_seconds_hint: None,
    };

    run_cached_step(
        "validate_hash",
        &paths,
        &mut state,
        None,
        allow_cache(AttemptStage::ValidateHash),
        false,
        |state| {
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
            hash_gate = evaluate_hash_gate(&report, &config.gates.hash, Some(output));
            let status = if hash_gate.passed {
                StepStatus::Succeeded
            } else {
                StepStatus::Failed
            };
            Ok(StepOutcome {
                status,
                stdout: if hash_gate.passed {
                    "hash validation passed".to_string()
                } else {
                    "hash validation failed".to_string()
                },
                stderr: if hash_gate.passed {
                    String::new()
                } else {
                    hash_gate.failures.join("; ")
                },
                outputs: state
                    .manifest
                    .validation_report
                    .clone()
                    .map(|path| vec![path])
                    .unwrap_or_default(),
            })
        },
    )?;

    let mut perceptual_gate = None;
    run_cached_step(
        "validate_perceptual",
        &paths,
        &mut state,
        None,
        allow_cache(AttemptStage::ValidatePerceptual),
        false,
        |state| {
            if !config.gates.perceptual.enabled {
                perceptual_gate = None;
                return Ok(StepOutcome {
                    status: StepStatus::Succeeded,
                    stdout: "perceptual gate disabled".to_string(),
                    stderr: String::new(),
                    outputs: Vec::new(),
                });
            }
            let result = run_perceptual_gate(config, &paths, perceptual_offset_seconds)?;
            let summary_path = paths.validation_dir.join("perceptual-summary.json");
            write_json(&summary_path, &result)?;
            let output = record_artifact(state, &paths, &summary_path, "perceptual_summary")?;
            let status = if result.passed {
                StepStatus::Succeeded
            } else {
                StepStatus::Failed
            };
            perceptual_gate = Some(result);
            Ok(StepOutcome {
                status,
                stdout: "perceptual gate completed".to_string(),
                stderr: String::new(),
                outputs: vec![output],
            })
        },
    )?;

    let status = if hash_gate.passed {
        if let Some(perceptual) = &perceptual_gate {
            if perceptual.passed {
                AttemptStatus::Passed
            } else {
                AttemptStatus::NeedsReview
            }
        } else {
            AttemptStatus::Passed
        }
    } else {
        AttemptStatus::Failed
    };

    let triage = build_triage(
        attempt,
        status,
        &hash_gate,
        perceptual_gate.as_ref(),
        strategy,
    );

    run_cached_step(
        "triage",
        &paths,
        &mut state,
        None,
        allow_cache(AttemptStage::Triage),
        false,
        |_state| {
            Ok(StepOutcome {
                status: if status == AttemptStatus::Failed {
                    StepStatus::Failed
                } else {
                    StepStatus::Succeeded
                },
                stdout: "triage generated".to_string(),
                stderr: String::new(),
                outputs: Vec::new(),
            })
        },
    )?;

    finalize_manifest(&mut state);
    write_run_manifest(&paths.run_manifest, &state.manifest)?;

    let gate_results = GateResults {
        schema_version: ATTEMPT_MANIFEST_SCHEMA_VERSION.to_string(),
        hash: hash_gate.clone(),
        perceptual: perceptual_gate.clone(),
        status,
    };
    write_json(&gate_results_path, &gate_results)?;
    write_json(&triage_path, &triage)?;

    let ghidra_evidence = find_role_artifact(&state.manifest, "ghidra_evidence");
    let attempt_manifest = AttemptManifest {
        schema_version: ATTEMPT_MANIFEST_SCHEMA_VERSION.to_string(),
        attempt,
        strategy,
        status,
        started_at: attempt_started_at,
        duration_ms: attempt_started.elapsed().as_millis(),
        run_manifest: state.manifest.clone(),
        gate_results,
        triage: triage.clone(),
        ghidra_evidence,
    };
    write_json(&attempt_manifest_path, &attempt_manifest)?;

    Ok(AttemptExecution {
        manifest: state.manifest,
        status,
        hash_gate,
        triage,
        attempt_manifest_path,
        gate_results_path,
        triage_path,
    })
}

fn run_ghidra_stage(
    config: &AutomationConfig,
    paths: &ResolvedPaths,
    state: &mut RunState,
    attempt: usize,
) -> Result<StepOutcome, String> {
    let Some(ghidra) = &config.tools.ghidra else {
        return Ok(StepOutcome {
            status: StepStatus::Succeeded,
            stdout: "ghidra disabled".to_string(),
            stderr: String::new(),
            outputs: Vec::new(),
        });
    };
    if !ghidra.enabled {
        return Ok(StepOutcome {
            status: StepStatus::Succeeded,
            stdout: "ghidra disabled".to_string(),
            stderr: String::new(),
            outputs: Vec::new(),
        });
    }

    let target_binary = match &ghidra.target_binary {
        Some(path) => path.clone(),
        None => match config.inputs.mode {
            InputMode::Homebrew => config.inputs.nro.clone().ok_or_else(|| {
                "tools.ghidra.target_binary missing and inputs.nro unset".to_string()
            })?,
            InputMode::Xci => config.inputs.xci.clone().ok_or_else(|| {
                "tools.ghidra.target_binary missing and inputs.xci unset".to_string()
            })?,
            InputMode::Lifted => {
                return Ok(StepOutcome {
                    status: StepStatus::Succeeded,
                    stdout: "ghidra skipped for lifted mode".to_string(),
                    stderr: String::new(),
                    outputs: Vec::new(),
                })
            }
        },
    };

    if !target_binary.exists() {
        return Err(format!(
            "ghidra target binary not found: {}",
            target_binary.display()
        ));
    }

    let analysis_dir = paths.validation_dir.join("analysis");
    fs::create_dir_all(&analysis_dir).map_err(|err| {
        format!(
            "create ghidra analysis dir {}: {err}",
            analysis_dir.display()
        )
    })?;
    let evidence_path = analysis_dir.join("ghidra-evidence.json");

    let project_root = ghidra
        .project_root
        .clone()
        .unwrap_or_else(|| paths.work_root.join("ghidra-projects"));
    fs::create_dir_all(&project_root).map_err(|err| {
        format!(
            "create ghidra project root {}: {err}",
            project_root.display()
        )
    })?;
    let project_name = ghidra
        .project_name
        .clone()
        .unwrap_or_else(|| format!("recomp-attempt-{attempt:03}"));

    let mut command = vec![
        ghidra
            .headless_path
            .clone()
            .unwrap_or_else(|| PathBuf::from("ghidra-analyzeHeadless"))
            .display()
            .to_string(),
        project_root.display().to_string(),
        project_name,
        "-import".to_string(),
        target_binary.display().to_string(),
        "-overwrite".to_string(),
    ];

    if let Some(script_path) = &ghidra.script_path {
        command.push("-scriptPath".to_string());
        command.push(script_path.display().to_string());
    } else {
        let default_script_dir = paths.repo_root.join("scripts/ghidra");
        command.push("-scriptPath".to_string());
        command.push(default_script_dir.display().to_string());
    }

    if let Some(pre_script) = &ghidra.pre_script {
        command.push("-preScript".to_string());
        command.push(pre_script.clone());
    }

    let post_script = ghidra
        .post_script
        .clone()
        .unwrap_or_else(|| "ghidra_export_evidence.py".to_string());
    command.push("-postScript".to_string());
    command.push(post_script);
    command.push(evidence_path.display().to_string());

    if let Some(language_id) = &ghidra.language_id {
        command.push("-processor".to_string());
        command.push(language_id.clone());
    }

    if let Some(timeout) = ghidra.analysis_timeout_sec {
        command.push("-analysisTimeoutPerFile".to_string());
        command.push(timeout.to_string());
    }

    command.push("-deleteProject".to_string());

    let (stdout, stderr) = run_command(&command, paths, config)?;

    if !evidence_path.exists() {
        let fallback = serde_json::json!({
            "schema_version": "1",
            "note": "ghidra post script did not emit evidence; fallback generated",
            "target_binary": target_binary.display().to_string(),
        });
        write_json(&evidence_path, &fallback)?;
    }

    let output = record_artifact(state, paths, &evidence_path, "ghidra_evidence")?;

    Ok(StepOutcome {
        status: StepStatus::Succeeded,
        stdout,
        stderr,
        outputs: vec![output],
    })
}

fn run_perceptual_gate(
    config: &AutomationConfig,
    paths: &ResolvedPaths,
    offset_seconds: f64,
) -> Result<PerceptualGateResult, String> {
    let compare_script = paths
        .repo_root
        .join("skills/static-recomp-av-compare/scripts/compare_av.py");
    if !compare_script.exists() {
        return Err(format!(
            "perceptual compare script not found: {}",
            compare_script.display()
        ));
    }

    let reference_src =
        fs::read_to_string(&config.reference.reference_video_toml).map_err(|err| {
            format!(
                "read reference video config {}: {err}",
                config.reference.reference_video_toml.display()
            )
        })?;
    let capture_src = fs::read_to_string(&config.reference.capture_video_toml).map_err(|err| {
        format!(
            "read capture video config {}: {err}",
            config.reference.capture_video_toml.display()
        )
    })?;

    let reference_cfg: ReferenceVideoConfig = toml::from_str(&reference_src)
        .map_err(|err| format!("invalid reference video config: {err}"))?;
    let capture_cfg: CaptureVideoConfig = toml::from_str(&capture_src)
        .map_err(|err| format!("invalid capture video config: {err}"))?;

    let reference_dir = config
        .reference
        .reference_video_toml
        .parent()
        .unwrap_or_else(|| Path::new("."));
    let capture_dir = config
        .reference
        .capture_video_toml
        .parent()
        .unwrap_or_else(|| Path::new("."));

    let reference_video_path = resolve_path(reference_dir, &reference_cfg.video.path);
    let capture_video_path = resolve_path(capture_dir, &capture_cfg.video.path);

    let scenes = scene_windows(config, &reference_cfg)?;
    let mut scene_results = Vec::new();

    for scene in scenes {
        let scene_dir = paths.validation_dir.join("perceptual").join(&scene.id);
        fs::create_dir_all(&scene_dir)
            .map_err(|err| format!("create scene output dir {}: {err}", scene_dir.display()))?;

        let mut command = vec![
            "python3".to_string(),
            compare_script.display().to_string(),
            "--ref".to_string(),
            reference_video_path.display().to_string(),
            "--test".to_string(),
            capture_video_path.display().to_string(),
            "--out-dir".to_string(),
            scene_dir.display().to_string(),
            "--label".to_string(),
            scene.id.clone(),
            "--width".to_string(),
            reference_cfg.video.width.to_string(),
            "--height".to_string(),
            reference_cfg.video.height.to_string(),
            "--fps".to_string(),
            format!("{:.3}", reference_cfg.video.fps),
            "--audio-rate".to_string(),
            config.gates.perceptual.audio_rate.to_string(),
            "--offset".to_string(),
            format!("{:.6}", offset_seconds),
            "--trim-start".to_string(),
            format!("{:.6}", scene.start_seconds),
            "--duration".to_string(),
            format!("{:.6}", scene.duration_seconds),
        ];

        if !config.gates.perceptual.require_vmaf {
            command.push("--no-vmaf".to_string());
        }

        let _ = run_command(&command, paths, config)?;

        let summary_path = scene_dir.join("summary.json");
        let summary_src = fs::read_to_string(&summary_path)
            .map_err(|err| format!("read summary {}: {err}", summary_path.display()))?;
        let summary_json: serde_json::Value = serde_json::from_str(&summary_src)
            .map_err(|err| format!("invalid summary {}: {err}", summary_path.display()))?;

        let ssim_avg = json_f32(&summary_json, &["video", "ssim", "average"]);
        let psnr_avg = json_f32(&summary_json, &["video", "psnr", "average"]);
        let vmaf_avg = json_f32(&summary_json, &["video", "vmaf", "average"]);
        let ref_lufs = json_f32(&summary_json, &["audio", "reference", "integrated_lufs"]);
        let test_lufs = json_f32(&summary_json, &["audio", "test", "integrated_lufs"]);
        let ref_peak = json_f32(&summary_json, &["audio", "reference", "true_peak_dbtp"]);
        let test_peak = json_f32(&summary_json, &["audio", "test", "true_peak_dbtp"]);
        let audio_lufs_delta = match (ref_lufs, test_lufs) {
            (Some(a), Some(b)) => Some((a - b).abs()),
            _ => None,
        };
        let audio_peak_delta = match (ref_peak, test_peak) {
            (Some(a), Some(b)) => Some((a - b).abs()),
            _ => None,
        };

        let mut failures = Vec::new();
        if let Some(value) = ssim_avg {
            if value < config.gates.perceptual.ssim_min {
                failures.push(format!(
                    "ssim {:.4} below {:.4}",
                    value, config.gates.perceptual.ssim_min
                ));
            }
        } else {
            failures.push("missing ssim metric".to_string());
        }

        if let Some(value) = psnr_avg {
            if value < config.gates.perceptual.psnr_min {
                failures.push(format!(
                    "psnr {:.4} below {:.4}",
                    value, config.gates.perceptual.psnr_min
                ));
            }
        } else {
            failures.push("missing psnr metric".to_string());
        }

        if config.gates.perceptual.require_vmaf {
            if let Some(value) = vmaf_avg {
                if value < config.gates.perceptual.vmaf_min {
                    failures.push(format!(
                        "vmaf {:.4} below {:.4}",
                        value, config.gates.perceptual.vmaf_min
                    ));
                }
            } else {
                failures.push("missing vmaf metric".to_string());
            }
        }

        if let Some(value) = audio_lufs_delta {
            if value > config.gates.perceptual.audio_lufs_delta_max {
                failures.push(format!(
                    "audio lufs delta {:.4} above {:.4}",
                    value, config.gates.perceptual.audio_lufs_delta_max
                ));
            }
        }

        if let Some(value) = audio_peak_delta {
            if value > config.gates.perceptual.audio_peak_delta_max {
                failures.push(format!(
                    "audio peak delta {:.4} above {:.4}",
                    value, config.gates.perceptual.audio_peak_delta_max
                ));
            }
        }

        let passed = failures.is_empty();

        let scene_result = PerceptualSceneResult {
            id: scene.id,
            weight: scene.weight,
            passed,
            summary_path: summary_path.display().to_string(),
            ssim_avg,
            psnr_avg,
            vmaf_avg,
            audio_lufs_delta,
            audio_peak_delta,
            failures,
        };
        scene_results.push(scene_result);
    }

    let total_weight: f32 = scene_results
        .iter()
        .map(|scene| scene.weight)
        .sum::<f32>()
        .max(1.0);
    let passed_weight: f32 = scene_results
        .iter()
        .filter(|scene| scene.passed)
        .map(|scene| scene.weight)
        .sum();
    let weighted_score = passed_weight / total_weight;
    let passed = scene_results.iter().all(|scene| scene.passed);
    let failing_scene = scene_results
        .iter()
        .filter(|scene| !scene.passed)
        .max_by(|a, b| {
            a.weight
                .partial_cmp(&b.weight)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|scene| scene.id.clone());

    Ok(PerceptualGateResult {
        enabled: true,
        passed,
        weighted_score,
        total_weight,
        passed_weight,
        failing_scene,
        scenes: scene_results,
    })
}

#[derive(Debug)]
struct SceneWindow {
    id: String,
    start_seconds: f64,
    duration_seconds: f64,
    weight: f32,
}

fn scene_windows(
    config: &AutomationConfig,
    reference_cfg: &ReferenceVideoConfig,
) -> Result<Vec<SceneWindow>, String> {
    if config.scenes.is_empty() {
        let start = reference_cfg.timeline.start.seconds;
        let end = reference_cfg.timeline.end.seconds;
        return Ok(vec![SceneWindow {
            id: "default".to_string(),
            start_seconds: start,
            duration_seconds: (end - start).max(0.001),
            weight: 1.0,
        }]);
    }

    let mut scenes = Vec::new();
    for scene in &config.scenes {
        let start = Timecode::parse(&scene.start)?.seconds;
        let end = Timecode::parse(&scene.end)?.seconds;
        if end <= start {
            return Err(format!(
                "scene {} has end <= start ({} <= {})",
                scene.id, end, start
            ));
        }
        scenes.push(SceneWindow {
            id: scene.id.clone(),
            start_seconds: start,
            duration_seconds: end - start,
            weight: scene.weight.max(0.0),
        });
        let _ = (&scene.input_marker_start, &scene.input_marker_end);
    }
    Ok(scenes)
}

fn evaluate_hash_gate(
    report: &recomp_validation::ValidationReport,
    gate: &HashGateConfig,
    report_path: Option<String>,
) -> HashGateResult {
    let mut failures = Vec::new();
    let mut passed = report.failed == 0;

    let mut frame_match_ratio = None;
    let mut frame_drift_frames = None;
    let mut frame_length_delta = None;
    let mut audio_match_ratio = None;
    let mut audio_drift_chunks = None;
    let mut drift_seconds_hint = None;

    if let Some(video) = &report.video {
        frame_match_ratio = Some(video.frame_comparison.match_ratio);
        frame_drift_frames = Some(video.drift.frame_offset);
        frame_length_delta = Some(video.drift.length_delta_frames);
        audio_match_ratio = video
            .audio_comparison
            .as_ref()
            .map(|audio| audio.match_ratio);
        audio_drift_chunks = video.audio_comparison.as_ref().map(|audio| audio.offset);
        drift_seconds_hint = Some(video.drift.frame_offset_seconds);
        failures.extend(video.failures.clone());

        if let Some(min_ratio) = gate.frame_match_ratio_min {
            if video.frame_comparison.match_ratio < min_ratio {
                failures.push(format!(
                    "hash gate override: frame match {:.4} below {:.4}",
                    video.frame_comparison.match_ratio, min_ratio
                ));
                passed = false;
            }
        }

        if let Some(max_drift) = gate.max_drift_frames {
            if video.drift.frame_offset.abs() > max_drift {
                failures.push(format!(
                    "hash gate override: frame drift {} exceeds {}",
                    video.drift.frame_offset, max_drift
                ));
                passed = false;
            }
        }

        if let Some(max_drop) = gate.max_dropped_frames {
            if video.drift.length_delta_frames.unsigned_abs() as usize > max_drop {
                failures.push(format!(
                    "hash gate override: frame delta {} exceeds {}",
                    video.drift.length_delta_frames, max_drop
                ));
                passed = false;
            }
        }

        if let Some(min_audio) = gate.audio_match_ratio_min {
            if let Some(audio) = &video.audio_comparison {
                if audio.match_ratio < min_audio {
                    failures.push(format!(
                        "hash gate override: audio match {:.4} below {:.4}",
                        audio.match_ratio, min_audio
                    ));
                    passed = false;
                }
            }
        }

        if let Some(max_audio_drift) = gate.max_audio_drift_chunks {
            if let Some(audio) = &video.audio_comparison {
                if audio.offset.abs() > max_audio_drift {
                    failures.push(format!(
                        "hash gate override: audio drift {} exceeds {}",
                        audio.offset, max_audio_drift
                    ));
                    passed = false;
                }
            }
        }
    } else {
        failures.push("hash validation missing video report".to_string());
        passed = false;
    }

    HashGateResult {
        passed,
        failed_cases: report.failed,
        frame_match_ratio,
        frame_drift_frames,
        frame_length_delta,
        audio_match_ratio,
        audio_drift_chunks,
        failures,
        report_path,
        drift_seconds_hint,
    }
}

fn build_triage(
    attempt: usize,
    status: AttemptStatus,
    hash_gate: &HashGateResult,
    perceptual: Option<&PerceptualGateResult>,
    strategy: Option<StrategyKind>,
) -> TriageReport {
    let mut categories = Vec::new();
    let mut findings = Vec::new();
    let mut suggestions = Vec::new();

    if !hash_gate.passed {
        categories.push("hash_gate_failed".to_string());
        findings.extend(hash_gate.failures.clone());

        if let Some(drift) = hash_gate.frame_drift_frames {
            if drift.abs() > 0 {
                suggestions.push("input_timing_variant".to_string());
            }
        }
        suggestions.push("service_stub_profile_switch".to_string());
        suggestions.push("patch_set_variant".to_string());
    }

    if let Some(perceptual_gate) = perceptual {
        if !perceptual_gate.passed {
            categories.push("perceptual_gate_failed".to_string());
            if let Some(scene) = &perceptual_gate.failing_scene {
                findings.push(format!("highest weighted failing scene: {scene}"));
            }
            suggestions.push("capture_alignment_profile".to_string());
            suggestions.push("runtime_mode_variant".to_string());
        }
    }

    if status == AttemptStatus::Passed {
        categories.push("pass".to_string());
    }

    let next_strategy = suggestions
        .iter()
        .find_map(|candidate| StrategyKind::from_id(candidate));

    let mut suggested_actions = suggestions;
    if let Some(current) = strategy {
        suggested_actions.push(format!("previous strategy was {}", current.id()));
    }

    TriageReport {
        schema_version: ATTEMPT_MANIFEST_SCHEMA_VERSION.to_string(),
        attempt,
        status,
        categories,
        findings,
        suggested_actions,
        next_strategy,
    }
}

fn resolve_strategy_order(config: &AutomationConfig) -> Result<Vec<StrategyKind>, String> {
    let mut order = Vec::new();

    if let Some(catalog_path) = &config.loop_config.strategy_catalog_toml {
        let src = fs::read_to_string(catalog_path)
            .map_err(|err| format!("read strategy catalog {}: {err}", catalog_path.display()))?;
        let catalog: StrategyCatalog = toml::from_str(&src)
            .map_err(|err| format!("invalid strategy catalog {}: {err}", catalog_path.display()))?;
        if let Some(version) = catalog.schema_version {
            if version != STRATEGY_CATALOG_SCHEMA_VERSION {
                return Err(format!(
                    "unsupported strategy catalog schema version: {version}"
                ));
            }
        }
        for entry in catalog.strategy {
            if !entry.enabled {
                continue;
            }
            let strategy = StrategyKind::from_id(&entry.id)
                .ok_or_else(|| format!("unknown strategy id in catalog: {}", entry.id))?;
            order.push(strategy);
        }
    }

    if order.is_empty() {
        for id in &config.loop_config.strategy_order {
            let strategy =
                StrategyKind::from_id(id).ok_or_else(|| format!("unknown strategy id: {id}"))?;
            order.push(strategy);
        }
    }

    if order.is_empty() {
        for id in default_strategy_order() {
            let strategy = StrategyKind::from_id(&id)
                .ok_or_else(|| format!("unknown default strategy id: {id}"))?;
            order.push(strategy);
        }
    }

    Ok(order)
}

fn select_next_strategy(
    order: &[StrategyKind],
    last_attempt: Option<&AttemptExecution>,
    used_strategies: &HashSet<StrategyKind>,
    config: &AutomationConfig,
) -> Option<StrategyKind> {
    if let Some(last) = last_attempt {
        if let Some(next) = last.triage.next_strategy {
            if !used_strategies.contains(&next) && strategy_applicable(next, config) {
                return Some(next);
            }
        }
    }

    order.iter().copied().find(|strategy| {
        !used_strategies.contains(strategy) && strategy_applicable(*strategy, config)
    })
}

fn strategy_applicable(strategy: StrategyKind, config: &AutomationConfig) -> bool {
    match strategy {
        StrategyKind::CaptureAlignmentProfile => config.gates.perceptual.enabled,
        StrategyKind::InputTimingVariant => config.reference.input_script_toml.is_some(),
        StrategyKind::ServiceStubProfileSwitch => config.inputs.config.exists(),
        StrategyKind::PatchSetVariant => config.inputs.config.exists(),
        StrategyKind::LiftModeVariant => !matches!(config.inputs.mode, InputMode::Lifted),
        StrategyKind::RuntimeModeVariant => config.inputs.config.exists(),
    }
}

fn apply_strategy(
    strategy: StrategyKind,
    config: &mut AutomationConfig,
    paths: &ResolvedPaths,
    attempt: usize,
    mutation_state: &mut MutationState,
    last_attempt: Option<&AttemptExecution>,
) -> Result<(), String> {
    let count = mutation_state.strategy_counts.entry(strategy).or_insert(0);
    let variant = *count;
    *count += 1;

    let mutation_dir = paths
        .attempts_root
        .join(format!("{attempt:03}"))
        .join("mutations");
    fs::create_dir_all(&mutation_dir)
        .map_err(|err| format!("create mutation dir {}: {err}", mutation_dir.display()))?;

    match strategy {
        StrategyKind::CaptureAlignmentProfile => {
            if let Some(last) = last_attempt {
                if let Some(drift) = last.hash_gate.drift_seconds_hint {
                    mutation_state.perceptual_offset_seconds = drift;
                }
            }
        }
        StrategyKind::InputTimingVariant => {
            let Some(input_script) = config.reference.input_script_toml.clone() else {
                return Ok(());
            };
            let src = fs::read_to_string(&input_script)
                .map_err(|err| format!("read input script {}: {err}", input_script.display()))?;
            let mut value: toml::Value = toml::from_str(&src)
                .map_err(|err| format!("parse input script {}: {err}", input_script.display()))?;
            let shift_frames = match variant % 4 {
                0 => 1,
                1 => -1,
                2 => 2,
                _ => -2,
            };
            apply_input_shift(&mut value, shift_frames)?;
            let out_path = mutation_dir.join("input_script.toml");
            fs::write(
                &out_path,
                toml::to_string_pretty(&value).map_err(|err| err.to_string())?,
            )
            .map_err(|err| format!("write input script {}: {err}", out_path.display()))?;
            config.reference.input_script_toml = Some(out_path);
        }
        StrategyKind::ServiceStubProfileSwitch => {
            mutate_title_config(config, &mutation_dir, |title| {
                let profile = match variant % 3 {
                    0 => "strict",
                    1 => "log-heavy",
                    _ => "noop-safe",
                };
                let stubs = ensure_table(title, "stubs")?;
                let keys: Vec<String> = stubs.keys().cloned().collect();
                for key in keys {
                    let value = match profile {
                        "strict" => "log",
                        "log-heavy" => "log",
                        _ => {
                            if key.contains("nifm") || key.contains("bsd") || key.contains("socket")
                            {
                                "noop"
                            } else {
                                "log"
                            }
                        }
                    };
                    stubs.insert(key, toml::Value::String(value.to_string()));
                }
                Ok(())
            })?;
        }
        StrategyKind::PatchSetVariant => {
            mutate_title_and_patch_set(config, &mutation_dir, variant)?;
        }
        StrategyKind::LiftModeVariant => {
            config.run.lift_mode = Some(
                match config.run.lift_mode.unwrap_or(LiftModeConfig::Decode) {
                    LiftModeConfig::Decode => LiftModeConfig::Stub,
                    LiftModeConfig::Stub => LiftModeConfig::Decode,
                },
            );
        }
        StrategyKind::RuntimeModeVariant => {
            mutate_title_config(config, &mutation_dir, |title| {
                let runtime = ensure_table(title, "runtime")?;
                let next = match runtime
                    .get("performance_mode")
                    .and_then(|value| value.as_str())
                    .unwrap_or("handheld")
                {
                    "handheld" => "docked",
                    _ => "handheld",
                };
                runtime.insert(
                    "performance_mode".to_string(),
                    toml::Value::String(next.to_string()),
                );
                Ok(())
            })?;
        }
    }

    Ok(())
}

fn apply_input_shift(script: &mut toml::Value, shift_frames: i64) -> Result<(), String> {
    let root = script
        .as_table_mut()
        .ok_or_else(|| "input script root must be a table".to_string())?;

    let timing_mode = root
        .get("metadata")
        .and_then(|value| value.as_table())
        .and_then(|table| table.get("timing_mode"))
        .and_then(|value| value.as_str())
        .unwrap_or("ms")
        .to_string();

    let shift_ms = ((1000.0 / 60.0) * shift_frames as f64).round() as i64;

    if let Some(events) = root
        .get_mut("events")
        .and_then(|value| value.as_array_mut())
    {
        for event in events {
            let table = event
                .as_table_mut()
                .ok_or_else(|| "input event must be a table".to_string())?;
            if timing_mode == "frames" {
                shift_integer_field(table, "frame", shift_frames)?;
            } else {
                shift_integer_field(table, "time_ms", shift_ms)?;
            }
        }
    }

    if let Some(markers) = root
        .get_mut("markers")
        .and_then(|value| value.as_array_mut())
    {
        for marker in markers {
            let table = marker
                .as_table_mut()
                .ok_or_else(|| "input marker must be a table".to_string())?;
            if timing_mode == "frames" {
                shift_integer_field(table, "frame", shift_frames)?;
            } else {
                shift_integer_field(table, "time_ms", shift_ms)?;
            }
        }
    }

    Ok(())
}

fn shift_integer_field(
    table: &mut toml::map::Map<String, toml::Value>,
    key: &str,
    delta: i64,
) -> Result<(), String> {
    if let Some(value) = table.get_mut(key) {
        let current = match value {
            toml::Value::Integer(number) => *number,
            _ => return Err(format!("input field {key} must be integer")),
        };
        let next = (current + delta).max(0);
        *value = toml::Value::Integer(next);
    }
    Ok(())
}

fn mutate_title_config<F>(
    config: &mut AutomationConfig,
    mutation_dir: &Path,
    mutator: F,
) -> Result<(), String>
where
    F: FnOnce(&mut toml::map::Map<String, toml::Value>) -> Result<(), String>,
{
    let title_path = config.inputs.config.clone();
    let src = fs::read_to_string(&title_path)
        .map_err(|err| format!("read title config {}: {err}", title_path.display()))?;
    let mut value: toml::Value = toml::from_str(&src)
        .map_err(|err| format!("parse title config {}: {err}", title_path.display()))?;
    let table = value
        .as_table_mut()
        .ok_or_else(|| "title config root must be table".to_string())?;
    mutator(table)?;

    let out_path = mutation_dir.join("title.toml");
    fs::write(
        &out_path,
        toml::to_string_pretty(&value).map_err(|err| err.to_string())?,
    )
    .map_err(|err| format!("write mutated title config {}: {err}", out_path.display()))?;

    config.inputs.config = out_path;
    Ok(())
}

fn mutate_title_and_patch_set(
    config: &mut AutomationConfig,
    mutation_dir: &Path,
    variant: usize,
) -> Result<(), String> {
    let title_path = config.inputs.config.clone();
    let title_src = fs::read_to_string(&title_path)
        .map_err(|err| format!("read title config {}: {err}", title_path.display()))?;
    let mut title_value: toml::Value = toml::from_str(&title_src)
        .map_err(|err| format!("parse title config {}: {err}", title_path.display()))?;
    let title_table = title_value
        .as_table_mut()
        .ok_or_else(|| "title config root must be table".to_string())?;

    let patches_path = title_table
        .get("patches")
        .and_then(|value| value.as_table())
        .and_then(|table| table.get("patch_set"))
        .and_then(|value| value.as_str())
        .map(PathBuf::from)
        .map(|path| {
            if path.is_absolute() {
                path
            } else {
                title_path
                    .parent()
                    .unwrap_or_else(|| Path::new("."))
                    .join(path)
            }
        });

    let Some(patch_path) = patches_path else {
        return Ok(());
    };

    let patch_src = fs::read_to_string(&patch_path)
        .map_err(|err| format!("read patch set {}: {err}", patch_path.display()))?;
    let mut patch_value: toml::Value = toml::from_str(&patch_src)
        .map_err(|err| format!("parse patch set {}: {err}", patch_path.display()))?;

    let patches = patch_value
        .as_table_mut()
        .and_then(|table| table.get_mut("patches"))
        .and_then(|value| value.as_array_mut())
        .ok_or_else(|| "patch set missing [[patches]] array".to_string())?;

    for (index, patch) in patches.iter_mut().enumerate() {
        let Some(table) = patch.as_table_mut() else {
            continue;
        };
        let enabled = match variant % 3 {
            0 => index % 2 == 0,
            1 => {
                let kind = table
                    .get("kind")
                    .and_then(|value| value.as_str())
                    .unwrap_or("");
                !kind.contains("branch")
            }
            _ => true,
        };
        table.insert("enabled".to_string(), toml::Value::Boolean(enabled));
    }

    let out_patch_path = mutation_dir.join("patches.toml");
    fs::write(
        &out_patch_path,
        toml::to_string_pretty(&patch_value).map_err(|err| err.to_string())?,
    )
    .map_err(|err| format!("write patch set {}: {err}", out_patch_path.display()))?;

    let patches_table = ensure_table(title_table, "patches")?;
    patches_table.insert(
        "patch_set".to_string(),
        toml::Value::String(out_patch_path.display().to_string()),
    );

    let out_title_path = mutation_dir.join("title.toml");
    fs::write(
        &out_title_path,
        toml::to_string_pretty(&title_value).map_err(|err| err.to_string())?,
    )
    .map_err(|err| format!("write title config {}: {err}", out_title_path.display()))?;

    config.inputs.config = out_title_path;
    Ok(())
}

fn ensure_table<'a>(
    root: &'a mut toml::map::Map<String, toml::Value>,
    key: &str,
) -> Result<&'a mut toml::map::Map<String, toml::Value>, String> {
    if !root.contains_key(key) {
        root.insert(key.to_string(), toml::Value::Table(toml::map::Map::new()));
    }
    root.get_mut(key)
        .and_then(|value| value.as_table_mut())
        .ok_or_else(|| format!("field {key} is not a table"))
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

        if let Some(path) = &self.tools.xci_tool_path {
            self.tools.xci_tool_path = Some(resolve_path(base_dir, path));
        }
        if let Some(path) = &self.tools.ffmpeg_path {
            self.tools.ffmpeg_path = Some(resolve_path(base_dir, path));
        }
        if let Some(ghidra) = &mut self.tools.ghidra {
            if let Some(path) = &ghidra.headless_path {
                ghidra.headless_path = Some(resolve_path(base_dir, path));
            }
            if let Some(path) = &ghidra.project_root {
                ghidra.project_root = Some(resolve_path(base_dir, path));
            }
            if let Some(path) = &ghidra.script_path {
                ghidra.script_path = Some(resolve_path(base_dir, path));
            }
            if let Some(path) = &ghidra.target_binary {
                ghidra.target_binary = Some(resolve_path(base_dir, path));
            }
        }

        if let Some(path) = &self.loop_config.strategy_catalog_toml {
            self.loop_config.strategy_catalog_toml = Some(resolve_path(base_dir, path));
        }
    }

    fn validate(&self) -> Result<(), String> {
        if self.schema_version != AUTOMATION_SCHEMA_VERSION_V1
            && self.schema_version != AUTOMATION_SCHEMA_VERSION_V2
        {
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
        if let Some(path) = &self.loop_config.strategy_catalog_toml {
            if !path.exists() {
                return Err(format!("strategy catalog not found: {}", path.display()));
            }
        }
        for scene in &self.scenes {
            if scene.id.trim().is_empty() {
                return Err("scene id cannot be empty".to_string());
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
        let attempts_root = work_root.join("attempts");
        let run_summary = work_root.join("run-summary.json");

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
            attempts_root,
            run_summary,
        })
    }

    fn clone_for_attempt(
        &self,
        _attempt_root: PathBuf,
        attempt_log_dir: PathBuf,
        attempt_validation_dir: PathBuf,
        attempt_manifest_path: PathBuf,
    ) -> Self {
        Self {
            repo_root: self.repo_root.clone(),
            config_dir: self.config_dir.clone(),
            work_root: self.work_root.clone(),
            intake_dir: self.intake_dir.clone(),
            lift_dir: self.lift_dir.clone(),
            build_dir: self.build_dir.clone(),
            assets_dir: self.assets_dir.clone(),
            validation_dir: attempt_validation_dir,
            log_dir: attempt_log_dir,
            run_manifest: attempt_manifest_path,
            lifted_module_json: self.lifted_module_json.clone(),
            attempts_root: self.attempts_root.clone(),
            run_summary: self.run_summary.clone(),
        }
    }
}

fn run_cached_step<F>(
    name: &str,
    paths: &ResolvedPaths,
    state: &mut RunState,
    command: Option<Vec<String>>,
    allow_cached: bool,
    fail_on_failed_status: bool,
    action: F,
) -> Result<(), String>
where
    F: FnOnce(&mut RunState) -> Result<StepOutcome, String>,
{
    if !allow_cached {
        state.cache_valid = false;
    }

    if state.cache_valid {
        if let Some(previous) = state.previous_steps.get(name) {
            if previous.status == StepStatus::Succeeded && outputs_exist(paths, previous) {
                state.manifest.steps.push(previous.clone());
                return Ok(());
            }
        }
        state.cache_valid = false;
    }

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
            if outcome.status == StepStatus::Failed && fail_on_failed_status {
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
    env.insert(
        "RECOMP_CLOUD_MODE".to_string(),
        match config.cloud.mode {
            CloudMode::Local => "local".to_string(),
            CloudMode::AwsHybrid => "aws_hybrid".to_string(),
        },
    );
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
    fs::create_dir_all(&paths.log_dir)
        .map_err(|err| format!("create log dir {}: {err}", paths.log_dir.display()))?;
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

fn manifest_outputs_exist(paths: &ResolvedPaths, manifest: &RunManifest) -> bool {
    manifest.artifacts.iter().all(|artifact| {
        let path = resolve_path(&paths.config_dir, Path::new(&artifact.path));
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
    let mut inputs = gather_inputs_from_config(config, paths)?;
    inputs.push(run_input("automation_config", config_path)?);
    inputs.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(inputs)
}

fn gather_inputs_from_config(
    config: &AutomationConfig,
    paths: &ResolvedPaths,
) -> Result<Vec<RunInput>, String> {
    let mut inputs = vec![
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
    if let Some(path) = &config.loop_config.strategy_catalog_toml {
        inputs.push(run_input("strategy_catalog", path)?);
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

fn json_f32(value: &serde_json::Value, path: &[&str]) -> Option<f32> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_f64().map(|number| number as f32)
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("create json dir {}: {err}", parent.display()))?;
    }
    let encoded = serde_json::to_string_pretty(value).map_err(|err| err.to_string())?;
    fs::write(path, encoded).map_err(|err| format!("write json {}: {err}", path.display()))
}

fn find_role_artifact(manifest: &RunManifest, role: &str) -> Option<String> {
    manifest
        .artifacts
        .iter()
        .find(|artifact| artifact.role == role)
        .map(|artifact| artifact.path.clone())
}

fn chrono_stamp() -> String {
    let now = std::time::SystemTime::now();
    let secs = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{secs}")
}

fn default_resume() -> bool {
    true
}

fn default_max_retries() -> usize {
    DEFAULT_MAX_RETRIES
}

fn default_max_runtime_minutes() -> u64 {
    DEFAULT_MAX_RUNTIME_MINUTES
}

fn default_stop_on_first_pass() -> bool {
    true
}

fn default_strategy_order() -> Vec<String> {
    vec![
        "capture_alignment_profile".to_string(),
        "input_timing_variant".to_string(),
        "service_stub_profile_switch".to_string(),
        "patch_set_variant".to_string(),
        "lift_mode_variant".to_string(),
        "runtime_mode_variant".to_string(),
    ]
}

fn default_scene_weight() -> f32 {
    1.0
}

fn default_ssim_min() -> f32 {
    0.95
}

fn default_psnr_min() -> f32 {
    35.0
}

fn default_vmaf_min() -> f32 {
    90.0
}

fn default_audio_lufs_delta_max() -> f32 {
    2.0
}

fn default_audio_peak_delta_max() -> f32 {
    2.0
}

fn default_audio_rate() -> u32 {
    DEFAULT_AUDIO_RATE
}

fn default_strategy_enabled() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn automation_runs_with_lifted_module_schema_v1() {
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
        fs::write(&automation_path, automation_toml).expect("write automation config");

        let manifest = run_automation(&automation_path).expect("run automation");
        assert_eq!(manifest.input_fingerprint.len(), 64);
        assert!(manifest.steps.iter().any(|step| step.name == "pipeline"));
        assert!(!manifest.attempts.is_empty());
        assert_eq!(manifest.final_status, Some(RunFinalStatus::Passed));
    }

    #[test]
    fn strategy_catalog_rejects_unknown_strategy() {
        let base = tempdir().expect("tempdir");
        let catalog = base.path().join("strategy-catalog.toml");
        fs::write(
            &catalog,
            r#"schema_version = "1"

[[strategy]]
id = "unknown"
enabled = true
"#,
        )
        .expect("write catalog");

        let config = AutomationConfig {
            schema_version: "2".to_string(),
            inputs: InputsConfig {
                mode: InputMode::Lifted,
                module_json: Some(PathBuf::from("/tmp/module.json")),
                nro: None,
                nso: Vec::new(),
                xci: None,
                keys: None,
                provenance: PathBuf::from("/tmp/provenance.toml"),
                config: PathBuf::from("/tmp/title.toml"),
                runtime_path: None,
            },
            outputs: OutputsConfig {
                work_root: PathBuf::from("/tmp/work"),
                intake_dir: None,
                lift_dir: None,
                build_dir: None,
                assets_dir: None,
                validation_dir: None,
                log_dir: None,
                run_manifest: None,
                lifted_module_json: None,
            },
            reference: ReferenceConfig {
                reference_video_toml: PathBuf::from("/tmp/ref.toml"),
                capture_video_toml: PathBuf::from("/tmp/cap.toml"),
                validation_config_toml: None,
                input_script_toml: None,
            },
            capture: CaptureConfig {
                video_path: PathBuf::from("/tmp/capture.mp4"),
                frames_dir: PathBuf::from("/tmp/frames"),
                audio_file: None,
            },
            commands: CommandConfig {
                build: vec!["/usr/bin/true".to_string()],
                run: vec!["/usr/bin/true".to_string()],
                capture: vec!["/usr/bin/true".to_string()],
                extract_frames: vec!["/usr/bin/true".to_string()],
                extract_audio: None,
                lift: None,
            },
            tools: ToolsConfig::default(),
            run: RunConfig::default(),
            loop_config: LoopConfig {
                enabled: true,
                max_retries: 1,
                max_runtime_minutes: 1,
                strategy_order: Vec::new(),
                stop_on_first_pass: true,
                strategy_catalog_toml: Some(catalog),
            },
            gates: GatesConfig::default(),
            agent: AgentConfig::default(),
            cloud: CloudConfig::default(),
            scenes: Vec::new(),
        };

        let err = resolve_strategy_order(&config).expect_err("expected unknown strategy error");
        assert!(err.contains("unknown strategy id"));
    }

    #[test]
    fn input_shift_updates_frame_events_and_markers() {
        let mut script: toml::Value = toml::from_str(
            r#"schema_version = "1"

[metadata]
title = "Test"
controller = "pad"
timing_mode = "frames"

[[events]]
frame = 10
control = 1
value = 1

[[markers]]
name = "m"
frame = 20
"#,
        )
        .expect("parse script");

        apply_input_shift(&mut script, 2).expect("shift script");

        let events = script
            .get("events")
            .and_then(|value| value.as_array())
            .expect("events");
        let event_frame = events[0]
            .get("frame")
            .and_then(|value| value.as_integer())
            .expect("frame");
        assert_eq!(event_frame, 12);

        let markers = script
            .get("markers")
            .and_then(|value| value.as_array())
            .expect("markers");
        let marker_frame = markers[0]
            .get("frame")
            .and_then(|value| value.as_integer())
            .expect("marker frame");
        assert_eq!(marker_frame, 22);
    }
}
