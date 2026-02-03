use crate::config::{PerformanceMode, StubBehavior, TitleConfig};
use crate::homebrew::ModuleJson;
use crate::input::{Block, Function, Module, Op, Terminator};
use crate::memory::MemoryLayoutDescriptor;
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
    let module = match parse_module_source(&module_src)? {
        ModuleSource::Lifted(module) => {
            module.validate_arch().map_err(PipelineError::Module)?;
            module
        }
        ModuleSource::Homebrew(module_json) => {
            return Err(PipelineError::Module(format!(
                "homebrew module.json detected (schema_version={}, module_type={}). Run the lifter to produce a lifted module.json before translation.",
                module_json.schema_version, module_json.module_type
            )));
        }
    };

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
        memory_layout: program.memory_layout.clone(),
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

#[derive(Debug)]
enum ModuleSource {
    Lifted(Module),
    Homebrew(ModuleJson),
}

fn parse_module_source(module_src: &str) -> Result<ModuleSource, PipelineError> {
    let value: serde_json::Value = serde_json::from_str(module_src)
        .map_err(|err| PipelineError::Module(format!("invalid module JSON: {err}")))?;
    if looks_like_homebrew_module(&value) {
        let module_json: ModuleJson = serde_json::from_value(value)
            .map_err(|err| PipelineError::Module(format!("invalid homebrew module JSON: {err}")))?;
        return Ok(ModuleSource::Homebrew(module_json));
    }
    let module: Module = serde_json::from_value(value)
        .map_err(|err| PipelineError::Module(format!("invalid module JSON: {err}")))?;
    Ok(ModuleSource::Lifted(module))
}

fn looks_like_homebrew_module(value: &serde_json::Value) -> bool {
    value
        .get("schema_version")
        .and_then(|value| value.as_str())
        .is_some()
        && value
            .get("module_type")
            .and_then(|value| value.as_str())
            .is_some()
        && value
            .get("modules")
            .and_then(|value| value.as_array())
            .is_some()
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
        memory_layout: config.memory_layout.clone(),
    })
}

fn translate_function(
    function: &Function,
    config: &TitleConfig,
) -> Result<RustFunction, PipelineError> {
    if !function.blocks.is_empty() {
        translate_block_function(function, config)
    } else if !function.ops.is_empty() {
        translate_linear_function(function, config)
    } else {
        Err(PipelineError::Module(format!(
            "function {} has no ops or blocks",
            function.name
        )))
    }
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

fn translate_linear_function(
    function: &Function,
    config: &TitleConfig,
) -> Result<RustFunction, PipelineError> {
    let mut regs = Vec::new();
    let mut lines = Vec::new();
    let mut needs_flags = false;

    for op in &function.ops {
        translate_op(op, config, &mut regs, &mut lines, &mut needs_flags)?;
    }

    Ok(RustFunction {
        name: function.name.clone(),
        regs,
        needs_flags,
        body: FunctionBody::Linear(lines),
    })
}

fn translate_block_function(
    function: &Function,
    config: &TitleConfig,
) -> Result<RustFunction, PipelineError> {
    let mut regs = Vec::new();
    let mut blocks = Vec::new();
    let mut needs_flags = false;

    for block in &function.blocks {
        let mut lines = Vec::new();
        for op in &block.ops {
            translate_op(op, config, &mut regs, &mut lines, &mut needs_flags)?;
        }
        let terminator = translate_terminator(block, &mut needs_flags)?;
        blocks.push(RustBlock {
            label: block.label.clone(),
            lines,
            terminator,
        });
    }

    Ok(RustFunction {
        name: function.name.clone(),
        regs,
        needs_flags,
        body: FunctionBody::Blocks(blocks),
    })
}

fn translate_op(
    op: &Op,
    config: &TitleConfig,
    regs: &mut Vec<String>,
    lines: &mut Vec<String>,
    needs_flags: &mut bool,
) -> Result<(), PipelineError> {
    match op {
        Op::ConstI64 { dst, imm } => {
            track_reg(regs, dst);
            lines.push(format!("{dst} = {imm};"));
        }
        Op::AddI64 { dst, lhs, rhs } => {
            track_reg(regs, dst);
            track_reg(regs, lhs);
            track_reg(regs, rhs);
            lines.push(format!("{dst} = {lhs} + {rhs};"));
        }
        Op::MovI64 { dst, src } => {
            track_reg(regs, dst);
            track_reg(regs, src);
            lines.push(format!("{dst} = {src};"));
        }
        Op::SubI64 { dst, lhs, rhs } => {
            track_reg(regs, dst);
            track_reg(regs, lhs);
            track_reg(regs, rhs);
            lines.push(format!("{dst} = {lhs} - {rhs};"));
        }
        Op::AndI64 { dst, lhs, rhs } => {
            track_reg(regs, dst);
            track_reg(regs, lhs);
            track_reg(regs, rhs);
            lines.push(format!("{dst} = {lhs} & {rhs};"));
        }
        Op::OrI64 { dst, lhs, rhs } => {
            track_reg(regs, dst);
            track_reg(regs, lhs);
            track_reg(regs, rhs);
            lines.push(format!("{dst} = {lhs} | {rhs};"));
        }
        Op::XorI64 { dst, lhs, rhs } => {
            track_reg(regs, dst);
            track_reg(regs, lhs);
            track_reg(regs, rhs);
            lines.push(format!("{dst} = {lhs} ^ {rhs};"));
        }
        Op::CmpI64 { lhs, rhs } => {
            track_reg(regs, lhs);
            track_reg(regs, rhs);
            *needs_flags = true;
            lines.push(format!(
                "let (__recomp_cmp_res, __recomp_cmp_overflow) = {lhs}.overflowing_sub({rhs});"
            ));
            lines.push(format!(
                "let (_, __recomp_cmp_borrow) = ({lhs} as u64).overflowing_sub({rhs} as u64);"
            ));
            lines.push("flag_n = __recomp_cmp_res < 0;".to_string());
            lines.push("flag_z = __recomp_cmp_res == 0;".to_string());
            lines.push("flag_c = !__recomp_cmp_borrow;".to_string());
            lines.push("flag_v = __recomp_cmp_overflow;".to_string());
        }
        Op::CmnI64 { lhs, rhs } => {
            track_reg(regs, lhs);
            track_reg(regs, rhs);
            *needs_flags = true;
            lines.push(format!(
                "let (__recomp_cmn_res, __recomp_cmn_overflow) = {lhs}.overflowing_add({rhs});"
            ));
            lines.push(format!(
                "let (_, __recomp_cmn_carry) = ({lhs} as u64).overflowing_add({rhs} as u64);"
            ));
            lines.push("flag_n = __recomp_cmn_res < 0;".to_string());
            lines.push("flag_z = __recomp_cmn_res == 0;".to_string());
            lines.push("flag_c = __recomp_cmn_carry;".to_string());
            lines.push("flag_v = __recomp_cmn_overflow;".to_string());
        }
        Op::TestI64 { lhs, rhs } => {
            track_reg(regs, lhs);
            track_reg(regs, rhs);
            *needs_flags = true;
            lines.push(format!("let __recomp_tst_res = {lhs} & {rhs};"));
            lines.push("flag_n = __recomp_tst_res < 0;".to_string());
            lines.push("flag_z = __recomp_tst_res == 0;".to_string());
            lines.push("flag_c = false;".to_string());
            lines.push("flag_v = false;".to_string());
        }
        Op::LslI64 { dst, lhs, rhs } => {
            track_reg(regs, dst);
            track_reg(regs, lhs);
            track_reg(regs, rhs);
            lines.push(format!(
                "{dst} = (({lhs} as u64) << (({rhs} as u64) & 63)) as i64;"
            ));
        }
        Op::LsrI64 { dst, lhs, rhs } => {
            track_reg(regs, dst);
            track_reg(regs, lhs);
            track_reg(regs, rhs);
            lines.push(format!(
                "{dst} = (({lhs} as u64) >> (({rhs} as u64) & 63)) as i64;"
            ));
        }
        Op::AsrI64 { dst, lhs, rhs } => {
            track_reg(regs, dst);
            track_reg(regs, lhs);
            track_reg(regs, rhs);
            lines.push(format!("{dst} = {lhs} >> (({rhs} as u64) & 63);"));
        }
        Op::PcRel { dst, pc, offset } => {
            track_reg(regs, dst);
            lines.push(format!("{dst} = {pc} + {offset};"));
        }
        Op::LoadI8 { dst, addr, offset } => {
            track_reg(regs, dst);
            track_reg(regs, addr);
            emit_load(lines, dst, addr, *offset, "mem_load_u8");
        }
        Op::LoadI16 { dst, addr, offset } => {
            track_reg(regs, dst);
            track_reg(regs, addr);
            emit_load(lines, dst, addr, *offset, "mem_load_u16");
        }
        Op::LoadI32 { dst, addr, offset } => {
            track_reg(regs, dst);
            track_reg(regs, addr);
            emit_load(lines, dst, addr, *offset, "mem_load_u32");
        }
        Op::LoadI64 { dst, addr, offset } => {
            track_reg(regs, dst);
            track_reg(regs, addr);
            emit_load(lines, dst, addr, *offset, "mem_load_u64");
        }
        Op::StoreI8 { src, addr, offset } => {
            track_reg(regs, src);
            track_reg(regs, addr);
            emit_store(lines, src, addr, *offset, "mem_store_u8");
        }
        Op::StoreI16 { src, addr, offset } => {
            track_reg(regs, src);
            track_reg(regs, addr);
            emit_store(lines, src, addr, *offset, "mem_store_u16");
        }
        Op::StoreI32 { src, addr, offset } => {
            track_reg(regs, src);
            track_reg(regs, addr);
            emit_store(lines, src, addr, *offset, "mem_store_u32");
        }
        Op::StoreI64 { src, addr, offset } => {
            track_reg(regs, src);
            track_reg(regs, addr);
            emit_store(lines, src, addr, *offset, "mem_store_u64");
        }
        Op::Br { target } => {
            lines.push(format!(
                "panic!({});",
                rust_string_literal(&format!("control-flow op in linear IR: br to {target}"))
            ));
        }
        Op::BrCond { cond, .. } => {
            *needs_flags = true;
            lines.push(format!(
                "panic!({});",
                rust_string_literal(&format!("control-flow op in linear IR: br_cond {cond}"))
            ));
        }
        Op::Call { target } => {
            let call_line = render_call_line(target);
            lines.push(call_line);
        }
        Op::Syscall { name, args } => {
            for arg in args {
                track_reg(regs, arg);
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
    Ok(())
}

fn translate_terminator(
    block: &Block,
    needs_flags: &mut bool,
) -> Result<RustTerminator, PipelineError> {
    match &block.terminator {
        Terminator::Br { target } => Ok(RustTerminator::Br {
            target: target.clone(),
        }),
        Terminator::BrCond {
            cond,
            then,
            else_target,
        } => {
            *needs_flags = true;
            let cond_expr = render_cond_expr(cond);
            Ok(RustTerminator::BrCond {
                cond_expr,
                cond: cond.clone(),
                then_label: then.clone(),
                else_label: else_target.clone(),
            })
        }
        Terminator::Call { target, next } => {
            let call_line = render_call_line(target);
            Ok(RustTerminator::Call {
                call_line,
                next: next.clone(),
            })
        }
        Terminator::BrIndirect { reg } => Ok(RustTerminator::BrIndirect { reg: reg.clone() }),
        Terminator::Ret => Ok(RustTerminator::Ret),
    }
}

fn render_call_line(target: &str) -> String {
    if is_rust_ident(target) {
        format!("{target}()?;")
    } else {
        format!(
            "panic!({});",
            rust_string_literal(&format!("unsupported call target: {target}"))
        )
    }
}

fn emit_load(lines: &mut Vec<String>, dst: &str, addr: &str, offset: i64, helper: &str) {
    let address_expr = format!("({addr} as u64).wrapping_add({offset} as u64)");
    lines.push(format!("let __recomp_addr = {address_expr};"));
    lines.push(format!(
        "let __recomp_value = recomp_runtime::{helper}(__recomp_addr)?;"
    ));
    lines.push(format!("{dst} = __recomp_value as i64;"));
}

fn emit_store(lines: &mut Vec<String>, src: &str, addr: &str, offset: i64, helper: &str) {
    let address_expr = format!("({addr} as u64).wrapping_add({offset} as u64)");
    lines.push(format!("let __recomp_addr = {address_expr};"));
    lines.push(format!(
        "recomp_runtime::{helper}(__recomp_addr, {src} as u64)?;"
    ));
}

fn render_cond_expr(cond: &str) -> Option<String> {
    let expr = match cond {
        "eq" => "flag_z",
        "ne" => "!flag_z",
        "cs" | "hs" => "flag_c",
        "cc" | "lo" => "!flag_c",
        "mi" => "flag_n",
        "pl" => "!flag_n",
        "vs" => "flag_v",
        "vc" => "!flag_v",
        "hi" => "flag_c && !flag_z",
        "ls" => "!flag_c || flag_z",
        "ge" => "flag_n == flag_v",
        "lt" => "flag_n != flag_v",
        "gt" => "!flag_z && (flag_n == flag_v)",
        "le" => "flag_z || (flag_n != flag_v)",
        "al" => "true",
        _ => return None,
    };
    Some(expr.to_string())
}

fn is_rust_ident(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first == '_' || first.is_ascii_alphabetic()) {
        return false;
    }
    chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn rust_string_literal(value: &str) -> String {
    format!("{value:?}")
}

#[derive(Debug)]
pub struct RustFunction {
    pub name: String,
    pub regs: Vec<String>,
    pub needs_flags: bool,
    pub body: FunctionBody,
}

#[derive(Debug)]
pub enum FunctionBody {
    Linear(Vec<String>),
    Blocks(Vec<RustBlock>),
}

#[derive(Debug)]
pub struct RustBlock {
    pub label: String,
    pub lines: Vec<String>,
    pub terminator: RustTerminator,
}

#[derive(Debug)]
pub enum RustTerminator {
    Br {
        target: String,
    },
    BrCond {
        cond_expr: Option<String>,
        cond: String,
        then_label: String,
        else_label: String,
    },
    Call {
        call_line: String,
        next: String,
    },
    BrIndirect {
        reg: String,
    },
    Ret,
}

#[derive(Debug)]
pub struct RustProgram {
    pub title: String,
    pub abi_version: String,
    pub entry: String,
    pub functions: Vec<RustFunction>,
    pub performance_mode: PerformanceMode,
    pub memory_layout: MemoryLayoutDescriptor,
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
