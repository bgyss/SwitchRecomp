use crate::config::{PerformanceMode, StubBehavior, TitleConfig};
use crate::input::{Function, Module, Op};
use crate::output::{emit_project, BuildManifest, GeneratedFile, InputSummary};
use crate::provenance::{ProvenanceManifest, ValidatedInput};
use pathdiff::diff_paths;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct PipelineOptions {
    pub module_path: PathBuf,
    pub config_path: PathBuf,
    pub provenance_path: PathBuf,
    pub out_dir: PathBuf,
    pub runtime_path: PathBuf,
}

#[derive(Debug)]
pub struct PipelineReport {
    pub out_dir: PathBuf,
    pub files_written: Vec<PathBuf>,
    pub detected_inputs: Vec<ValidatedInput>,
}

#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("config error: {0}")]
    Config(String),
    #[error("module error: {0}")]
    Module(String),
    #[error("provenance error: {0}")]
    Provenance(String),
    #[error("emit error: {0}")]
    Emit(String),
}

pub fn run_pipeline(options: PipelineOptions) -> Result<PipelineReport, PipelineError> {
    let module_path = absolute_path(&options.module_path)?;
    let config_path = absolute_path(&options.config_path)?;
    let provenance_path = absolute_path(&options.provenance_path)?;
    let out_dir = absolute_path(&options.out_dir)?;
    let runtime_path = absolute_path(&options.runtime_path)?;

    let module_src = fs::read_to_string(&module_path)?;
    let module: Module = serde_json::from_str(&module_src)
        .map_err(|err| PipelineError::Module(format!("invalid module JSON: {err}")))?;
    module.validate_arch().map_err(PipelineError::Module)?;

    let config_src = fs::read_to_string(&config_path)?;
    let config = TitleConfig::parse(&config_src).map_err(PipelineError::Config)?;

    let provenance_src = fs::read_to_string(&provenance_path)?;
    let provenance =
        ProvenanceManifest::parse(&provenance_src).map_err(PipelineError::Provenance)?;
    let provenance_validation = provenance
        .validate(&provenance_path, &provenance_src)
        .map_err(PipelineError::Provenance)?;
    if !provenance_validation
        .inputs
        .iter()
        .any(|input| input.path == module_path)
    {
        return Err(PipelineError::Provenance(format!(
            "module input is not listed in provenance metadata: {}",
            module_path.display()
        )));
    }

    let program = translate_module(&module, &config)?;

    let module_hash = sha256_hex(&module_src);
    let config_hash = sha256_hex(&config_src);
    let provenance_hash = provenance_validation.manifest_sha256.clone();

    let runtime_rel = diff_paths(&runtime_path, &out_dir).unwrap_or(runtime_path.clone());
    let inputs = provenance_validation
        .inputs
        .iter()
        .map(|input| InputSummary {
            path: input.path.clone(),
            format: input.format.as_str().to_string(),
            sha256: input.sha256.clone(),
            size: input.size,
            role: input.role.clone(),
        })
        .collect::<Vec<_>>();
    let manifest = BuildManifest {
        title: program.title.clone(),
        abi_version: program.abi_version.clone(),
        module_sha256: module_hash,
        config_sha256: config_hash,
        provenance_sha256: provenance_hash,
        inputs,
        manifest_self_hash_basis: String::new(),
        generated_files: Vec::<GeneratedFile>::new(),
    };

    let (files_written, _manifest) =
        emit_project(&out_dir, &runtime_rel, &program, &manifest).map_err(PipelineError::Emit)?;

    Ok(PipelineReport {
        out_dir,
        files_written,
        detected_inputs: provenance_validation.inputs,
    })
}

fn translate_module(module: &Module, config: &TitleConfig) -> Result<RustProgram, PipelineError> {
    let entry = config.entry.clone();
    let mut functions = Vec::new();
    for function in &module.functions {
        functions.push(translate_function(function, config)?);
    }

    Ok(RustProgram {
        title: config.title.clone(),
        abi_version: config.abi_version.clone(),
        entry,
        functions,
        performance_mode: config.runtime.performance_mode,
    })
}

fn translate_function(
    function: &Function,
    config: &TitleConfig,
) -> Result<RustFunction, PipelineError> {
    let mut regs = Vec::new();
    let mut lines = Vec::new();

    for op in &function.ops {
        match op {
            Op::ConstI64 { dst, imm } => {
                track_reg(&mut regs, dst);
                lines.push(format!("{dst} = {imm};"));
            }
            Op::AddI64 { dst, lhs, rhs } => {
                track_reg(&mut regs, dst);
                track_reg(&mut regs, lhs);
                track_reg(&mut regs, rhs);
                lines.push(format!("{dst} = {lhs} + {rhs};"));
            }
            Op::Syscall { name, args } => {
                for arg in args {
                    track_reg(&mut regs, arg);
                }
                let behavior = config
                    .stubs
                    .get(name)
                    .copied()
                    .unwrap_or(StubBehavior::Panic);
                let call = render_syscall(name, behavior, args);
                lines.push(call);
            }
            Op::Ret => {
                lines.push("return Ok(());".to_string());
            }
        }
    }

    Ok(RustFunction {
        name: function.name.clone(),
        regs,
        lines,
    })
}

fn track_reg(regs: &mut Vec<String>, name: &str) {
    if !regs.iter().any(|item| item == name) {
        regs.push(name.to_string());
    }
}

fn render_syscall(name: &str, behavior: StubBehavior, args: &[String]) -> String {
    let args_list = args.join(", ");
    match behavior {
        StubBehavior::Log => {
            format!("recomp_runtime::syscall_log(\"{name}\", &[{args_list}])?;")
        }
        StubBehavior::Noop => {
            format!("recomp_runtime::syscall_noop(\"{name}\", &[{args_list}])?;")
        }
        StubBehavior::Panic => {
            format!("recomp_runtime::syscall_panic(\"{name}\", &[{args_list}])?;")
        }
    }
}

#[derive(Debug)]
pub struct RustFunction {
    pub name: String,
    pub regs: Vec<String>,
    pub lines: Vec<String>,
}

#[derive(Debug)]
pub struct RustProgram {
    pub title: String,
    pub abi_version: String,
    pub entry: String,
    pub functions: Vec<RustFunction>,
    pub performance_mode: PerformanceMode,
}

impl RustProgram {
    pub fn entry_function(&self) -> Option<&RustFunction> {
        self.functions.iter().find(|func| func.name == self.entry)
    }
}

pub fn ensure_dir(path: &Path) -> Result<(), PipelineError> {
    fs::create_dir_all(path).map_err(PipelineError::Io)
}

fn absolute_path(path: &Path) -> Result<PathBuf, PipelineError> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(std::env::current_dir()?.join(path))
    }
}

fn sha256_hex(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    let digest = hasher.finalize();
    format!("{:x}", digest)
}
