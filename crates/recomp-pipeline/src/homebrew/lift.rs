use crate::homebrew::module::{ModuleJson, ModuleSegment, MODULE_SCHEMA_VERSION};
use crate::input::{Block, Function, Module, Op, Terminator};
use serde_json;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

const MAX_BLOCK_INSTRUCTIONS: usize = 10_000;
const MAX_FUNCTION_INSTRUCTIONS: usize = 200_000;

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
        segments: Vec::new(),
        functions: vec![Function {
            name: options.entry_name.clone(),
            ops: vec![Op::Ret],
            blocks: Vec::new(),
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
    let text = load_text_segment(&text_segment, &mut warnings)?;

    let mut functions = Vec::new();
    let mut pending = BTreeSet::new();
    let mut seen = BTreeSet::new();
    let mut name_table = FunctionNames::new();
    name_table.seed_entry(text.base_addr, options.entry_name.clone());

    pending.insert(text.base_addr);

    while let Some(func_addr) = pop_first(&mut pending) {
        if !text.contains(func_addr) {
            warnings.push(format!(
                "function entry 0x{func_addr:x} is outside text segment"
            ));
            continue;
        }
        if seen.contains(&func_addr) {
            continue;
        }
        seen.insert(func_addr);

        let func_name = name_table.name_for(func_addr);
        let mut state = DecodeState::new();
        let (blocks, call_targets) =
            decode_function(&text, func_addr, &mut state, &mut warnings, &mut name_table)?;

        for target in call_targets {
            if !text.contains(target) {
                warnings.push(format!("call target 0x{target:x} is outside text segment"));
                continue;
            }
            pending.insert(target);
        }

        functions.push(Function {
            name: func_name,
            ops: Vec::new(),
            blocks,
        });
    }

    if functions.is_empty() {
        warnings.push("decoded zero functions from text segment".to_string());
    }

    ensure_dir(out_dir)?;

    let lifted = Module {
        arch: "aarch64".to_string(),
        segments: Vec::new(),
        functions,
    };

    let output_path = out_dir.join("module.json");
    let output_json = serde_json::to_string_pretty(&lifted).map_err(|err| err.to_string())?;
    fs::write(&output_path, output_json).map_err(|err| err.to_string())?;

    Ok(LiftReport {
        module_json_path: output_path,
        functions_emitted: lifted.functions.len(),
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

#[derive(Debug, Clone)]
struct TextSegment {
    bytes: Vec<u8>,
    base_addr: u64,
}

impl TextSegment {
    fn contains(&self, addr: u64) -> bool {
        let end = self.base_addr + self.bytes.len() as u64;
        addr >= self.base_addr && addr < end
    }

    fn read_word(&self, addr: u64) -> Result<u32, String> {
        if !self.contains(addr) {
            return Err(format!("address 0x{addr:x} outside text segment"));
        }
        let offset = addr - self.base_addr;
        if offset % 4 != 0 {
            return Err(format!("unaligned instruction address 0x{addr:x}"));
        }
        let idx = offset as usize;
        if idx + 4 > self.bytes.len() {
            return Err(format!("instruction read overrun at 0x{addr:x}"));
        }
        Ok(u32::from_le_bytes([
            self.bytes[idx],
            self.bytes[idx + 1],
            self.bytes[idx + 2],
            self.bytes[idx + 3],
        ]))
    }
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

fn load_text_segment(
    segment: &SegmentInfo,
    warnings: &mut Vec<String>,
) -> Result<TextSegment, String> {
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
    Ok(TextSegment {
        bytes,
        base_addr: segment.memory_offset,
    })
}

#[derive(Debug, Default)]
struct DecodeState {
    temp_counter: u32,
    reg_values: [Option<i64>; 32],
}

impl DecodeState {
    fn new() -> Self {
        Self {
            temp_counter: 0,
            reg_values: [None; 32],
        }
    }

    fn next_temp(&mut self, prefix: &str) -> String {
        let name = format!("{prefix}{}", self.temp_counter);
        self.temp_counter += 1;
        name
    }
}

struct FunctionNames {
    names: BTreeMap<u64, String>,
}

impl FunctionNames {
    fn new() -> Self {
        Self {
            names: BTreeMap::new(),
        }
    }

    fn name_for(&mut self, addr: u64) -> String {
        if let Some(name) = self.names.get(&addr) {
            return name.clone();
        }
        let name = format!("fn_{addr:016x}");
        self.names.insert(addr, name.clone());
        name
    }

    fn seed_entry(&mut self, addr: u64, name: String) {
        self.names.insert(addr, name);
    }
}

fn decode_function(
    text: &TextSegment,
    entry: u64,
    state: &mut DecodeState,
    warnings: &mut Vec<String>,
    names: &mut FunctionNames,
) -> Result<(Vec<Block>, Vec<u64>), String> {
    let mut blocks = BTreeMap::new();
    let mut pending = BTreeSet::new();
    let mut call_targets = BTreeSet::new();
    let mut decoded_count = 0_usize;

    pending.insert(entry);

    while let Some(addr) = pop_first(&mut pending) {
        if blocks.contains_key(&addr) {
            continue;
        }

        let (block, block_targets, block_calls, block_insts) =
            decode_block(text, addr, state, warnings, names)?;
        decoded_count += block_insts;
        if decoded_count > MAX_FUNCTION_INSTRUCTIONS {
            return Err(format!(
                "function decode limit exceeded ({} instructions)",
                MAX_FUNCTION_INSTRUCTIONS
            ));
        }

        for target in block_targets {
            pending.insert(target);
        }
        for target in block_calls {
            call_targets.insert(target);
        }
        blocks.insert(addr, block);
    }

    Ok((
        blocks.into_values().collect(),
        call_targets.into_iter().collect(),
    ))
}

fn decode_block(
    text: &TextSegment,
    start: u64,
    state: &mut DecodeState,
    warnings: &mut Vec<String>,
    names: &mut FunctionNames,
) -> Result<(Block, Vec<u64>, Vec<u64>, usize), String> {
    if start % 4 != 0 {
        return Err(format!("unaligned block start 0x{start:x}"));
    }
    if !text.contains(start) {
        return Err(format!("block start 0x{start:x} outside text segment"));
    }

    let mut ops = Vec::new();
    let mut pc = start;
    let mut inst_count = 0_usize;

    loop {
        if inst_count >= MAX_BLOCK_INSTRUCTIONS {
            return Err(format!(
                "block decode limit exceeded ({} instructions)",
                MAX_BLOCK_INSTRUCTIONS
            ));
        }
        let word = text.read_word(pc)?;
        let decoded = decode_one(word, pc, state, &mut ops, warnings)?;
        inst_count += 1;

        match decoded {
            DecodedOutcome::Continue => {
                pc = pc.wrapping_add(4);
                continue;
            }
            DecodedOutcome::Terminate(term) => {
                let (terminator, block_targets, call_targets) = lower_terminator(term, names);
                let block = Block {
                    label: block_label(start),
                    start,
                    ops,
                    terminator,
                };
                return Ok((block, block_targets, call_targets, inst_count));
            }
        }
    }
}

fn lower_terminator(term: TermInfo, names: &mut FunctionNames) -> (Terminator, Vec<u64>, Vec<u64>) {
    match term {
        TermInfo::Br { target } => (
            Terminator::Br {
                target: block_label(target),
            },
            vec![target],
            Vec::new(),
        ),
        TermInfo::BrCond {
            cond,
            then_target,
            else_target,
        } => (
            Terminator::BrCond {
                cond,
                then: block_label(then_target),
                else_target: block_label(else_target),
            },
            vec![then_target, else_target],
            Vec::new(),
        ),
        TermInfo::Call { target, next } => {
            let name = names.name_for(target);
            (
                Terminator::Call {
                    target: name,
                    next: block_label(next),
                },
                vec![next],
                vec![target],
            )
        }
        TermInfo::BrIndirect { reg } => (
            Terminator::BrIndirect { reg: reg_name(reg) },
            Vec::new(),
            Vec::new(),
        ),
        TermInfo::Ret => (Terminator::Ret, Vec::new(), Vec::new()),
    }
}

#[derive(Debug)]
enum DecodedOutcome {
    Continue,
    Terminate(TermInfo),
}

#[derive(Debug)]
enum TermInfo {
    Br {
        target: u64,
    },
    BrCond {
        cond: String,
        then_target: u64,
        else_target: u64,
    },
    Call {
        target: u64,
        next: u64,
    },
    BrIndirect {
        reg: u8,
    },
    Ret,
}

fn decode_one(
    word: u32,
    pc: u64,
    state: &mut DecodeState,
    ops: &mut Vec<Op>,
    warnings: &mut Vec<String>,
) -> Result<DecodedOutcome, String> {
    if word == 0xD503201F {
        return Ok(DecodedOutcome::Continue);
    }
    if is_ret(word) {
        return Ok(DecodedOutcome::Terminate(TermInfo::Ret));
    }
    if let Some(reg) = decode_br_register(word) {
        return Ok(DecodedOutcome::Terminate(TermInfo::BrIndirect { reg }));
    }
    if let Some(target) = decode_branch_imm(word, pc) {
        return Ok(DecodedOutcome::Terminate(TermInfo::Br { target }));
    }
    if let Some(target) = decode_branch_link(word, pc) {
        return Ok(DecodedOutcome::Terminate(TermInfo::Call {
            target,
            next: pc.wrapping_add(4),
        }));
    }
    if let Some((cond, target)) = decode_branch_cond(word, pc)? {
        return Ok(DecodedOutcome::Terminate(TermInfo::BrCond {
            cond,
            then_target: target,
            else_target: pc.wrapping_add(4),
        }));
    }

    if let Some((is_nz, reg, target)) = decode_cbz(word, pc)? {
        let zero = state.next_temp("imm");
        ops.push(Op::ConstI64 {
            dst: zero.clone(),
            imm: 0,
        });
        ops.push(Op::CmpI64 {
            lhs: reg_name(reg),
            rhs: zero,
        });
        let cond = if is_nz { "ne" } else { "eq" };
        return Ok(DecodedOutcome::Terminate(TermInfo::BrCond {
            cond: cond.to_string(),
            then_target: target,
            else_target: pc.wrapping_add(4),
        }));
    }

    if let Some((is_nz, reg, bit, target)) = decode_tbz(word, pc)? {
        let tmp_shift = state.next_temp("tmp");
        let tmp_mask = state.next_temp("tmp");
        let tmp_one = state.next_temp("imm");
        let tmp_zero = state.next_temp("imm");
        ops.push(Op::ConstI64 {
            dst: tmp_one.clone(),
            imm: 1,
        });
        ops.push(Op::ConstI64 {
            dst: tmp_zero.clone(),
            imm: 0,
        });
        ops.push(Op::ConstI64 {
            dst: tmp_shift.clone(),
            imm: bit as i64,
        });
        ops.push(Op::LsrI64 {
            dst: tmp_mask.clone(),
            lhs: reg_name(reg),
            rhs: tmp_shift,
        });
        ops.push(Op::AndI64 {
            dst: tmp_mask.clone(),
            lhs: tmp_mask.clone(),
            rhs: tmp_one,
        });
        ops.push(Op::CmpI64 {
            lhs: tmp_mask,
            rhs: tmp_zero,
        });
        let cond = if is_nz { "ne" } else { "eq" };
        return Ok(DecodedOutcome::Terminate(TermInfo::BrCond {
            cond: cond.to_string(),
            then_target: target,
            else_target: pc.wrapping_add(4),
        }));
    }

    if let Some(decoded) = decode_mov_wide(word)? {
        emit_mov_wide(decoded, pc, state, ops)?;
        return Ok(DecodedOutcome::Continue);
    }

    if let Some(decoded) = decode_adr(word, pc)? {
        ops.push(decoded);
        return Ok(DecodedOutcome::Continue);
    }

    if let Some(decoded) = decode_add_sub_immediate(word)? {
        emit_add_sub_immediate(decoded, state, ops);
        return Ok(DecodedOutcome::Continue);
    }

    if let Some(decoded) = decode_add_sub_register(word)? {
        emit_add_sub_register(decoded, state, ops);
        return Ok(DecodedOutcome::Continue);
    }

    if let Some(decoded) = decode_logical_register(word)? {
        emit_logical_register(decoded, state, ops);
        return Ok(DecodedOutcome::Continue);
    }

    if let Some(decoded) = decode_load_store(word)? {
        emit_load_store(decoded, ops, warnings);
        return Ok(DecodedOutcome::Continue);
    }

    Err(format!(
        "unsupported instruction 0x{word:08x} at 0x{pc:016x}"
    ))
}

#[derive(Debug, Clone, Copy)]
struct MovWide {
    dst: u8,
    kind: MovWideKind,
    imm: u16,
    shift: u8,
    is_32: bool,
}

#[derive(Debug, Clone, Copy)]
enum MovWideKind {
    MovZ,
    MovN,
    MovK,
}

fn decode_mov_wide(word: u32) -> Result<Option<MovWide>, String> {
    let sf = (word >> 31) & 0x1;
    let opc = (word >> 29) & 0x3;
    let fixed = (word >> 23) & 0x3F;
    if fixed != 0b100101 {
        return Ok(None);
    }
    let hw = ((word >> 21) & 0x3) as u8;
    let imm16 = ((word >> 5) & 0xFFFF) as u16;
    let rd = (word & 0x1F) as u8;
    let kind = match opc {
        0b00 => MovWideKind::MovN,
        0b10 => MovWideKind::MovZ,
        0b11 => MovWideKind::MovK,
        _ => return Ok(None),
    };
    Ok(Some(MovWide {
        dst: rd,
        kind,
        imm: imm16,
        shift: hw,
        is_32: sf == 0,
    }))
}

fn emit_mov_wide(
    decoded: MovWide,
    pc: u64,
    state: &mut DecodeState,
    ops: &mut Vec<Op>,
) -> Result<(), String> {
    let shift_bits = (decoded.shift as u32) * 16;
    let imm_value = (decoded.imm as u64) << shift_bits;
    let value = match decoded.kind {
        MovWideKind::MovZ => imm_value,
        MovWideKind::MovN => !imm_value,
        MovWideKind::MovK => {
            let prev = state.reg_values[decoded.dst as usize].ok_or_else(|| {
                format!(
                    "movk requires prior value for x{} at 0x{:x}",
                    decoded.dst, pc
                )
            })?;
            let mask = !(0xFFFF_u64 << shift_bits);
            (prev as u64 & mask) | imm_value
        }
    };
    state.reg_values[decoded.dst as usize] = Some(value as i64);
    ops.push(Op::ConstI64 {
        dst: reg_name(decoded.dst),
        imm: value as i64,
    });
    if decoded.is_32 {
        zero_extend_32(&reg_name(decoded.dst), state, ops);
    }
    Ok(())
}

fn decode_adr(word: u32, pc: u64) -> Result<Option<Op>, String> {
    let op = (word >> 31) & 0x1;
    let fixed = (word >> 24) & 0x1F;
    if fixed != 0b10000 {
        return Ok(None);
    }
    let immlo = (word >> 29) & 0x3;
    let immhi = (word >> 5) & 0x7FFFF;
    let rd = (word & 0x1F) as u8;
    let imm = ((immhi << 2) | immlo) as i64;
    let signed = sign_extend(imm, 21);
    if op == 0 {
        return Ok(Some(Op::PcRel {
            dst: reg_name(rd),
            pc: pc as i64,
            offset: signed,
        }));
    }
    let page = (pc as i64) & !0xFFF;
    let offset = signed << 12;
    Ok(Some(Op::PcRel {
        dst: reg_name(rd),
        pc: page,
        offset,
    }))
}

#[derive(Debug, Clone, Copy)]
struct AddSubImm {
    dst: u8,
    src: u8,
    imm: u64,
    is_sub: bool,
    set_flags: bool,
    is_32: bool,
}

fn decode_add_sub_immediate(word: u32) -> Result<Option<AddSubImm>, String> {
    let sf = (word >> 31) & 0x1;
    let op = (word >> 30) & 0x1;
    let s = (word >> 29) & 0x1;
    let opcode = (word >> 24) & 0x1F;
    if opcode != 0b10001 {
        return Ok(None);
    }
    let shift = (word >> 22) & 0x3;
    if shift > 1 {
        return Ok(None);
    }
    let imm12 = (word >> 10) & 0xFFF;
    let rn = ((word >> 5) & 0x1F) as u8;
    let rd = (word & 0x1F) as u8;
    let imm = (imm12 as u64) << (shift * 12);
    Ok(Some(AddSubImm {
        dst: rd,
        src: rn,
        imm,
        is_sub: op == 1,
        set_flags: s == 1,
        is_32: sf == 0,
    }))
}

fn emit_add_sub_immediate(decoded: AddSubImm, state: &mut DecodeState, ops: &mut Vec<Op>) {
    let temp = state.next_temp("imm");
    ops.push(Op::ConstI64 {
        dst: temp.clone(),
        imm: decoded.imm as i64,
    });
    if decoded.set_flags && decoded.dst == 31 {
        if decoded.is_sub {
            ops.push(Op::CmpI64 {
                lhs: reg_name(decoded.src),
                rhs: temp,
            });
        } else {
            ops.push(Op::CmnI64 {
                lhs: reg_name(decoded.src),
                rhs: temp,
            });
        }
        return;
    }
    if decoded.is_sub {
        ops.push(Op::SubI64 {
            dst: reg_name(decoded.dst),
            lhs: reg_name(decoded.src),
            rhs: temp,
        });
    } else {
        ops.push(Op::AddI64 {
            dst: reg_name(decoded.dst),
            lhs: reg_name(decoded.src),
            rhs: temp,
        });
    }
    if decoded.is_32 {
        zero_extend_32(&reg_name(decoded.dst), state, ops);
    }
}

#[derive(Debug, Clone, Copy)]
struct AddSubReg {
    dst: u8,
    lhs: u8,
    rhs: u8,
    is_sub: bool,
    set_flags: bool,
    is_32: bool,
}

fn decode_add_sub_register(word: u32) -> Result<Option<AddSubReg>, String> {
    let sf = (word >> 31) & 0x1;
    let op = (word >> 30) & 0x1;
    let s = (word >> 29) & 0x1;
    let opcode = (word >> 24) & 0x1F;
    if opcode != 0b01011 {
        return Ok(None);
    }
    let shift = (word >> 22) & 0x3;
    let imm6 = (word >> 10) & 0x3F;
    if shift != 0 || imm6 != 0 {
        return Ok(None);
    }
    let rm = ((word >> 16) & 0x1F) as u8;
    let rn = ((word >> 5) & 0x1F) as u8;
    let rd = (word & 0x1F) as u8;
    Ok(Some(AddSubReg {
        dst: rd,
        lhs: rn,
        rhs: rm,
        is_sub: op == 1,
        set_flags: s == 1,
        is_32: sf == 0,
    }))
}

fn emit_add_sub_register(decoded: AddSubReg, state: &mut DecodeState, ops: &mut Vec<Op>) {
    if decoded.set_flags && decoded.dst == 31 {
        if decoded.is_sub {
            ops.push(Op::CmpI64 {
                lhs: reg_name(decoded.lhs),
                rhs: reg_name(decoded.rhs),
            });
        } else {
            ops.push(Op::CmnI64 {
                lhs: reg_name(decoded.lhs),
                rhs: reg_name(decoded.rhs),
            });
        }
        return;
    }
    if decoded.is_sub {
        ops.push(Op::SubI64 {
            dst: reg_name(decoded.dst),
            lhs: reg_name(decoded.lhs),
            rhs: reg_name(decoded.rhs),
        });
    } else {
        ops.push(Op::AddI64 {
            dst: reg_name(decoded.dst),
            lhs: reg_name(decoded.lhs),
            rhs: reg_name(decoded.rhs),
        });
    }
    if decoded.is_32 {
        zero_extend_32(&reg_name(decoded.dst), state, ops);
    }
}

#[derive(Debug, Clone, Copy)]
struct LogicalReg {
    dst: u8,
    lhs: u8,
    rhs: u8,
    opc: u8,
    is_32: bool,
}

fn decode_logical_register(word: u32) -> Result<Option<LogicalReg>, String> {
    let sf = (word >> 31) & 0x1;
    let opc = ((word >> 29) & 0x3) as u8;
    let fixed = (word >> 24) & 0x1F;
    let n = (word >> 21) & 0x1;
    if fixed != 0b01010 || n != 0 {
        return Ok(None);
    }
    let shift = (word >> 22) & 0x3;
    let imm6 = (word >> 10) & 0x3F;
    if shift != 0 || imm6 != 0 {
        return Ok(None);
    }
    let rm = ((word >> 16) & 0x1F) as u8;
    let rn = ((word >> 5) & 0x1F) as u8;
    let rd = (word & 0x1F) as u8;
    Ok(Some(LogicalReg {
        dst: rd,
        lhs: rn,
        rhs: rm,
        opc,
        is_32: sf == 0,
    }))
}

fn emit_logical_register(decoded: LogicalReg, state: &mut DecodeState, ops: &mut Vec<Op>) {
    let dst = reg_name(decoded.dst);
    let lhs = reg_name(decoded.lhs);
    let rhs = reg_name(decoded.rhs);

    match decoded.opc {
        0b00 => {
            ops.push(Op::AndI64 {
                dst: dst.clone(),
                lhs,
                rhs,
            });
            if decoded.is_32 {
                zero_extend_32(&dst, state, ops);
            }
        }
        0b01 => {
            if decoded.lhs == 31 {
                ops.push(Op::MovI64 {
                    dst: dst.clone(),
                    src: rhs,
                });
            } else {
                ops.push(Op::OrI64 {
                    dst: dst.clone(),
                    lhs,
                    rhs,
                });
            }
            if decoded.is_32 {
                zero_extend_32(&dst, state, ops);
            }
        }
        0b10 => {
            ops.push(Op::XorI64 {
                dst: dst.clone(),
                lhs,
                rhs,
            });
            if decoded.is_32 {
                zero_extend_32(&dst, state, ops);
            }
        }
        0b11 => {
            if decoded.dst == 31 {
                ops.push(Op::TestI64 { lhs, rhs });
            } else {
                ops.push(Op::AndI64 {
                    dst: dst.clone(),
                    lhs,
                    rhs,
                });
                ops.push(Op::TestI64 {
                    lhs: dst.clone(),
                    rhs: dst.clone(),
                });
                if decoded.is_32 {
                    zero_extend_32(&dst, state, ops);
                }
            }
        }
        _ => {}
    }
}

#[derive(Debug, Clone, Copy)]
struct LoadStore {
    is_load: bool,
    size: u8,
    base: u8,
    rt: u8,
    offset: u64,
}

fn decode_load_store(word: u32) -> Result<Option<LoadStore>, String> {
    if (word & 0x3B000000) != 0x39000000 {
        return Ok(None);
    }
    let size = ((word >> 30) & 0x3) as u8;
    let is_load = ((word >> 22) & 0x1) == 1;
    let imm12 = (word >> 10) & 0xFFF;
    let rn = ((word >> 5) & 0x1F) as u8;
    let rt = (word & 0x1F) as u8;
    let offset = (imm12 as u64) << size;
    Ok(Some(LoadStore {
        is_load,
        size,
        base: rn,
        rt,
        offset,
    }))
}

fn emit_load_store(decoded: LoadStore, ops: &mut Vec<Op>, warnings: &mut Vec<String>) {
    let offset = decoded.offset as i64;
    let base = reg_name(decoded.base);
    let reg = reg_name(decoded.rt);
    let op = match (decoded.is_load, decoded.size) {
        (true, 0) => Op::LoadI8 {
            dst: reg,
            addr: base,
            offset,
        },
        (true, 1) => Op::LoadI16 {
            dst: reg,
            addr: base,
            offset,
        },
        (true, 2) => Op::LoadI32 {
            dst: reg,
            addr: base,
            offset,
        },
        (true, 3) => Op::LoadI64 {
            dst: reg,
            addr: base,
            offset,
        },
        (false, 0) => Op::StoreI8 {
            src: reg,
            addr: base,
            offset,
        },
        (false, 1) => Op::StoreI16 {
            src: reg,
            addr: base,
            offset,
        },
        (false, 2) => Op::StoreI32 {
            src: reg,
            addr: base,
            offset,
        },
        (false, 3) => Op::StoreI64 {
            src: reg,
            addr: base,
            offset,
        },
        _ => {
            warnings.push("unsupported load/store size".to_string());
            return;
        }
    };
    ops.push(op);
}

fn decode_branch_imm(word: u32, pc: u64) -> Option<u64> {
    if (word & 0xFC000000) != 0x14000000 {
        return None;
    }
    let imm26 = (word & 0x03FFFFFF) as i64;
    let offset = sign_extend(imm26 << 2, 28);
    Some((pc as i64 + offset) as u64)
}

fn decode_branch_link(word: u32, pc: u64) -> Option<u64> {
    if (word & 0xFC000000) != 0x94000000 {
        return None;
    }
    let imm26 = (word & 0x03FFFFFF) as i64;
    let offset = sign_extend(imm26 << 2, 28);
    Some((pc as i64 + offset) as u64)
}

fn decode_branch_cond(word: u32, pc: u64) -> Result<Option<(String, u64)>, String> {
    if (word & 0xFF000010) != 0x54000000 {
        return Ok(None);
    }
    let cond = (word & 0xF) as u8;
    let cond_str = cond_code(cond).ok_or_else(|| format!("unsupported condition {cond}"))?;
    let imm19 = ((word >> 5) & 0x7FFFF) as i64;
    let offset = sign_extend(imm19 << 2, 21);
    Ok(Some((cond_str.to_string(), (pc as i64 + offset) as u64)))
}

fn decode_cbz(word: u32, pc: u64) -> Result<Option<(bool, u8, u64)>, String> {
    let top = word & 0x7F000000;
    let is_cbz = top == 0x34000000 || top == 0xB4000000;
    let is_cbnz = top == 0x35000000 || top == 0xB5000000;
    if !is_cbz && !is_cbnz {
        return Ok(None);
    }
    let imm19 = ((word >> 5) & 0x7FFFF) as i64;
    let offset = sign_extend(imm19 << 2, 21);
    let rt = (word & 0x1F) as u8;
    Ok(Some((is_cbnz, rt, (pc as i64 + offset) as u64)))
}

fn decode_tbz(word: u32, pc: u64) -> Result<Option<(bool, u8, u8, u64)>, String> {
    let top = word & 0x7F000000;
    let is_tbz = top == 0x36000000 || top == 0xB6000000;
    let is_tbnz = top == 0x37000000 || top == 0xB7000000;
    if !is_tbz && !is_tbnz {
        return Ok(None);
    }
    let b5 = ((word >> 31) & 0x1) as u8;
    let b40 = ((word >> 19) & 0x1F) as u8;
    let bit = (b5 << 5) | b40;
    let imm14 = ((word >> 5) & 0x3FFF) as i64;
    let offset = sign_extend(imm14 << 2, 16);
    let rt = (word & 0x1F) as u8;
    Ok(Some((is_tbnz, rt, bit, (pc as i64 + offset) as u64)))
}

fn decode_br_register(word: u32) -> Option<u8> {
    if (word & 0xFFFFFC1F) != 0xD61F0000 {
        return None;
    }
    Some(((word >> 5) & 0x1F) as u8)
}

fn is_ret(word: u32) -> bool {
    (word & 0xFFFFFC1F) == 0xD65F0000
}

fn cond_code(code: u8) -> Option<&'static str> {
    match code {
        0x0 => Some("eq"),
        0x1 => Some("ne"),
        0x2 => Some("cs"),
        0x3 => Some("cc"),
        0x4 => Some("mi"),
        0x5 => Some("pl"),
        0x6 => Some("vs"),
        0x7 => Some("vc"),
        0x8 => Some("hi"),
        0x9 => Some("ls"),
        0xA => Some("ge"),
        0xB => Some("lt"),
        0xC => Some("gt"),
        0xD => Some("le"),
        0xE => Some("al"),
        _ => None,
    }
}

fn sign_extend(value: i64, bits: u8) -> i64 {
    let shift = 64 - bits as i64;
    (value << shift) >> shift
}

fn zero_extend_32(dst: &str, state: &mut DecodeState, ops: &mut Vec<Op>) {
    let mask = state.next_temp("imm");
    ops.push(Op::ConstI64 {
        dst: mask.clone(),
        imm: 0xFFFF_FFFF,
    });
    ops.push(Op::AndI64 {
        dst: dst.to_string(),
        lhs: dst.to_string(),
        rhs: mask,
    });
}

fn reg_name(reg: u8) -> String {
    format!("x{reg}")
}

fn block_label(addr: u64) -> String {
    format!("bb_{addr:016x}")
}

fn pop_first(set: &mut BTreeSet<u64>) -> Option<u64> {
    let first = set.iter().next().copied();
    if let Some(value) = first {
        set.remove(&value);
    }
    first
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
