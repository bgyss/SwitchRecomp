use crate::homebrew::module::{ModuleJson, ModuleSegment, MODULE_SCHEMA_VERSION};
use crate::input::{Function, Module, Op};
use serde_json;
use std::fs;
use std::path::{Path, PathBuf};

const MAX_DECODE_INSTRUCTIONS: usize = 50_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiftMode {
    Stub,
    Decode,
}

#[derive(Debug)]
pub struct LiftOptions {
    pub module_json_path: PathBuf,
    pub out_dir: PathBuf,
    pub entry_name: String,
    pub mode: LiftMode,
}

#[derive(Debug)]
pub struct LiftReport {
    pub module_json_path: PathBuf,
    pub functions_emitted: usize,
    pub warnings: Vec<String>,
}

pub fn lift_homebrew(options: LiftOptions) -> Result<LiftReport, String> {
    if options.entry_name.trim().is_empty() {
        return Err("entry name must be non-empty".to_string());
    }

    let module_json_path = absolute_path(&options.module_json_path)?;
    let out_dir = absolute_path(&options.out_dir)?;

    let module_src = fs::read_to_string(&module_json_path).map_err(|err| err.to_string())?;
    let module_json: ModuleJson =
        serde_json::from_str(&module_src).map_err(|err| err.to_string())?;

    validate_homebrew_module(&module_json)?;

    let base_dir = module_json_path
        .parent()
        .ok_or_else(|| "homebrew module.json has no parent directory".to_string())?;

    match options.mode {
        LiftMode::Stub => lift_stub(&options, &module_json, base_dir, &out_dir),
        LiftMode::Decode => lift_decode(&options, &module_json, base_dir, &out_dir),
    }
}

fn validate_homebrew_module(module_json: &ModuleJson) -> Result<(), String> {
    if module_json.schema_version != MODULE_SCHEMA_VERSION {
        return Err(format!(
            "unsupported homebrew module schema version: {}",
            module_json.schema_version
        ));
    }
    if module_json.module_type != "homebrew" {
        return Err(format!(
            "unsupported module type for homebrew lifter: {}",
            module_json.module_type
        ));
    }
    if module_json.modules.is_empty() {
        return Err("homebrew module list is empty".to_string());
    }
    Ok(())
}

fn lift_stub(
    options: &LiftOptions,
    module_json: &ModuleJson,
    base_dir: &Path,
    out_dir: &Path,
) -> Result<LiftReport, String> {
    let mut warnings = Vec::new();

    if module_json.modules.len() > 1 {
        warnings.push(format!(
            "homebrew lifter emitted a stub entry for {} modules without decoding instructions",
            module_json.modules.len()
        ));
    } else {
        warnings
            .push("homebrew lifter emitted a stub entry without decoding instructions".to_string());
    }

    let segments = collect_segments(module_json, base_dir)?;
    if segments.is_empty() {
        warnings.push("homebrew module contains no segments".to_string());
    }

    ensure_dir(out_dir)?;

    let lifted = Module {
        arch: "aarch64".to_string(),
        functions: vec![Function {
            name: options.entry_name.clone(),
            ops: vec![Op::Ret],
        }],
    };

    let output_path = out_dir.join("module.json");
    let output_json = serde_json::to_string_pretty(&lifted).map_err(|err| err.to_string())?;
    fs::write(&output_path, output_json).map_err(|err| err.to_string())?;

    Ok(LiftReport {
        module_json_path: output_path,
        functions_emitted: 1,
        warnings,
    })
}

fn lift_decode(
    options: &LiftOptions,
    module_json: &ModuleJson,
    base_dir: &Path,
    out_dir: &Path,
) -> Result<LiftReport, String> {
    let mut warnings = Vec::new();
    if module_json.modules.len() > 1 {
        warnings.push(format!(
            "homebrew lifter decoding only the first text segment across {} modules",
            module_json.modules.len()
        ));
    }

    let segments = collect_segments(module_json, base_dir)?;
    let text_segment = find_text_segment(&segments, &mut warnings)?;
    let mut ops = decode_text_segment(&text_segment, &mut warnings)?;

    if ops.is_empty() {
        warnings.push("decoded zero instructions from text segment".to_string());
    }
    if ops.last().map(|op| !matches!(op, Op::Ret)).unwrap_or(true) {
        warnings.push("decoded block does not end with ret; adding ret".to_string());
        ops.push(Op::Ret);
    }

    ensure_dir(out_dir)?;

    let lifted = Module {
        arch: "aarch64".to_string(),
        functions: vec![Function {
            name: options.entry_name.clone(),
            ops,
        }],
    };

    let output_path = out_dir.join("module.json");
    let output_json = serde_json::to_string_pretty(&lifted).map_err(|err| err.to_string())?;
    fs::write(&output_path, output_json).map_err(|err| err.to_string())?;

    Ok(LiftReport {
        module_json_path: output_path,
        functions_emitted: 1,
        warnings,
    })
}

#[derive(Debug, Clone)]
struct SegmentInfo {
    name: String,
    permissions: String,
    memory_offset: u64,
    file_size: u64,
    path: PathBuf,
}

fn collect_segments(module_json: &ModuleJson, base_dir: &Path) -> Result<Vec<SegmentInfo>, String> {
    let mut segments = Vec::new();
    for module in &module_json.modules {
        for segment in &module.segments {
            let segment_path = resolve_segment_path(base_dir, segment);
            if !segment_path.exists() {
                return Err(format!(
                    "segment file not found: {}",
                    segment_path.display()
                ));
            }
            segments.push(SegmentInfo {
                name: segment.name.clone(),
                permissions: segment.permissions.clone(),
                memory_offset: segment.memory_offset,
                file_size: segment.file_size,
                path: segment_path,
            });
        }
    }
    Ok(segments)
}

fn find_text_segment(
    segments: &[SegmentInfo],
    warnings: &mut Vec<String>,
) -> Result<SegmentInfo, String> {
    let mut candidates = segments
        .iter()
        .filter(|segment| segment.name == "text" || segment.permissions.contains('x'));
    let first = candidates
        .next()
        .cloned()
        .ok_or_else(|| "no executable text segment found in homebrew module".to_string())?;
    if candidates.next().is_some() {
        warnings.push("multiple executable segments found; using the first".to_string());
    }
    Ok(first)
}

fn decode_text_segment(
    segment: &SegmentInfo,
    warnings: &mut Vec<String>,
) -> Result<Vec<Op>, String> {
    let bytes = fs::read(&segment.path).map_err(|err| err.to_string())?;
    if bytes.len() as u64 != segment.file_size {
        warnings.push(format!(
            "text segment size mismatch: manifest {} bytes, file {} bytes",
            segment.file_size,
            bytes.len()
        ));
    }
    if bytes.len() % 4 != 0 {
        return Err(format!(
            "text segment size is not 4-byte aligned: {} bytes",
            bytes.len()
        ));
    }

    let mut ops = Vec::new();
    let mut reg_values = [None; 32];
    let mut temp_counter = 0_u32;

    for (index, chunk) in bytes.chunks(4).enumerate() {
        if index >= MAX_DECODE_INSTRUCTIONS {
            return Err(format!(
                "decode limit exceeded ({} instructions)",
                MAX_DECODE_INSTRUCTIONS
            ));
        }

        let word = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        let pc = segment.memory_offset + (index as u64 * 4);

        match decode_instruction(word, pc)? {
            Decoded::Nop => {}
            Decoded::Ret => {
                ops.push(Op::Ret);
                break;
            }
            Decoded::MovWide {
                dst,
                kind,
                imm,
                shift,
            } => {
                let shift_bits = (shift as u32) * 16;
                let imm_value = (imm as u64) << shift_bits;
                let value = match kind {
                    MovWideKind::MovZ => imm_value,
                    MovWideKind::MovN => !imm_value,
                    MovWideKind::MovK => {
                        let prev = reg_values[dst as usize].ok_or_else(|| {
                            format!("movk requires prior value for x{} at 0x{:x}", dst, pc)
                        })?;
                        let mask = !(0xFFFF_u64 << shift_bits);
                        (prev as u64 & mask) | imm_value
                    }
                };
                reg_values[dst as usize] = Some(value as i64);
                ops.push(Op::ConstI64 {
                    dst: reg_name(dst),
                    imm: value as i64,
                });
            }
            Decoded::AddImm { dst, src, imm } => {
                let temp = format!("imm{}", temp_counter);
                temp_counter += 1;
                ops.push(Op::ConstI64 {
                    dst: temp.clone(),
                    imm: imm as i64,
                });
                ops.push(Op::AddI64 {
                    dst: reg_name(dst),
                    lhs: reg_name(src),
                    rhs: temp,
                });
                let next = reg_values[src as usize].map(|value| value.wrapping_add(imm as i64));
                reg_values[dst as usize] = next;
            }
            Decoded::AddReg { dst, lhs, rhs } => {
                ops.push(Op::AddI64 {
                    dst: reg_name(dst),
                    lhs: reg_name(lhs),
                    rhs: reg_name(rhs),
                });
                let next = match (reg_values[lhs as usize], reg_values[rhs as usize]) {
                    (Some(a), Some(b)) => Some(a.wrapping_add(b)),
                    _ => None,
                };
                reg_values[dst as usize] = next;
            }
        }
    }

    Ok(ops)
}

#[derive(Debug, Clone, Copy)]
enum MovWideKind {
    MovZ,
    MovN,
    MovK,
}

#[derive(Debug, Clone, Copy)]
enum Decoded {
    Nop,
    Ret,
    MovWide {
        dst: u8,
        kind: MovWideKind,
        imm: u16,
        shift: u8,
    },
    AddImm {
        dst: u8,
        src: u8,
        imm: u64,
    },
    AddReg {
        dst: u8,
        lhs: u8,
        rhs: u8,
    },
}

fn decode_instruction(word: u32, pc: u64) -> Result<Decoded, String> {
    if word == 0xD503201F {
        return Ok(Decoded::Nop);
    }
    if is_ret(word) {
        return Ok(Decoded::Ret);
    }
    if let Some(decoded) = decode_mov_wide(word) {
        return Ok(decoded);
    }
    if let Some(decoded) = decode_add_immediate(word) {
        return Ok(decoded);
    }
    if let Some(decoded) = decode_add_register(word) {
        return Ok(decoded);
    }
    Err(format!(
        "unsupported instruction 0x{word:08x} at 0x{pc:016x}"
    ))
}

fn decode_mov_wide(word: u32) -> Option<Decoded> {
    let sf = (word >> 31) & 0x1;
    let opc = (word >> 29) & 0x3;
    let fixed = (word >> 23) & 0x3F;
    if sf != 1 || fixed != 0b100101 {
        return None;
    }
    let hw = ((word >> 21) & 0x3) as u8;
    let imm16 = ((word >> 5) & 0xFFFF) as u16;
    let rd = (word & 0x1F) as u8;
    let kind = match opc {
        0b00 => MovWideKind::MovN,
        0b10 => MovWideKind::MovZ,
        0b11 => MovWideKind::MovK,
        _ => return None,
    };
    Some(Decoded::MovWide {
        dst: rd,
        kind,
        imm: imm16,
        shift: hw,
    })
}

fn decode_add_immediate(word: u32) -> Option<Decoded> {
    let sf = (word >> 31) & 0x1;
    let op = (word >> 30) & 0x1;
    let s = (word >> 29) & 0x1;
    let opcode = (word >> 24) & 0x1F;
    if sf != 1 || op != 0 || s != 0 || opcode != 0b10001 {
        return None;
    }
    let shift = (word >> 22) & 0x3;
    if shift > 1 {
        return None;
    }
    let imm12 = (word >> 10) & 0xFFF;
    let rn = ((word >> 5) & 0x1F) as u8;
    let rd = (word & 0x1F) as u8;
    let imm = (imm12 as u64) << (shift * 12);
    Some(Decoded::AddImm {
        dst: rd,
        src: rn,
        imm,
    })
}

fn decode_add_register(word: u32) -> Option<Decoded> {
    let sf = (word >> 31) & 0x1;
    let op = (word >> 30) & 0x1;
    let s = (word >> 29) & 0x1;
    let opcode = (word >> 24) & 0x1F;
    if sf != 1 || op != 0 || s != 0 || opcode != 0b01011 {
        return None;
    }
    let shift = (word >> 22) & 0x3;
    let imm6 = (word >> 10) & 0x3F;
    if shift != 0 || imm6 != 0 {
        return None;
    }
    let rm = ((word >> 16) & 0x1F) as u8;
    let rn = ((word >> 5) & 0x1F) as u8;
    let rd = (word & 0x1F) as u8;
    Some(Decoded::AddReg {
        dst: rd,
        lhs: rn,
        rhs: rm,
    })
}

fn is_ret(word: u32) -> bool {
    (word & 0xFFFFFC1F) == 0xD65F0000
}

fn reg_name(reg: u8) -> String {
    format!("x{}", reg)
}

fn resolve_segment_path(base_dir: &Path, segment: &ModuleSegment) -> PathBuf {
    let path = Path::new(&segment.output_path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_dir.join(path)
    }
}

fn ensure_dir(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|err| err.to_string())
}

fn absolute_path(path: &Path) -> Result<PathBuf, String> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        std::env::current_dir()
            .map_err(|err| err.to_string())
            .map(|cwd| cwd.join(path))
    }
}
