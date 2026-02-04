use crate::output::{GeneratedFile, InputSummary};
use crate::provenance::{InputFormat, ProvenanceManifest, ProvenanceValidation};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

const INTAKE_SCHEMA_VERSION: &str = "1";
const XCI_MAGIC: &[u8; 4] = b"XCI0";
const XCI_HEADER_SIZE: usize = 0x20;
const XCI_ENTRY_SIZE: usize = 0x40;
const NCA_MAGIC: &[u8; 4] = b"NCA3";
const PFS0_MAGIC: &[u8; 4] = b"PFS0";
const MANIFEST_SELF_PATH: &str = "manifest.json";
const MANIFEST_SELF_SHA_PLACEHOLDER: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Debug)]
pub struct IntakeOptions {
    pub xci_path: PathBuf,
    pub keys_path: PathBuf,
    pub provenance_path: PathBuf,
    pub out_dir: PathBuf,
    pub program: ProgramSelection,
}

#[derive(Debug, Clone)]
pub enum ProgramSelection {
    TitleId(String),
    Name(String),
}

#[derive(Debug)]
pub struct IntakeReport {
    pub out_dir: PathBuf,
    pub module_json_path: PathBuf,
    pub manifest_path: PathBuf,
    pub files_written: Vec<PathBuf>,
}

#[derive(Debug, Serialize)]
struct IntakeManifest {
    schema_version: String,
    tool: ToolInfo,
    program: ProgramRecord,
    assets: Vec<AssetRecord>,
    inputs: Vec<InputSummary>,
    manifest_self_hash_basis: String,
    generated_files: Vec<GeneratedFile>,
}

#[derive(Debug, Serialize)]
struct ToolInfo {
    name: String,
    version: String,
}

#[derive(Debug, Serialize)]
struct ProgramRecord {
    name: String,
    title_id: String,
    version: u32,
    content_type: String,
    nca_offset: u64,
    nca_size: u64,
    exefs_entries: Vec<ExeFsEntryRecord>,
    segments: Vec<SegmentRecord>,
}

#[derive(Debug, Serialize, Clone)]
struct ExeFsEntryRecord {
    name: String,
    path: String,
    sha256: String,
    size: u64,
}

#[derive(Debug, Serialize, Clone)]
struct SegmentRecord {
    name: String,
    kind: String,
    permissions: String,
    memory_offset: u32,
    size: u32,
    path: String,
    sha256: String,
    file_size: u64,
}

#[derive(Debug, Serialize, Clone)]
struct AssetRecord {
    kind: String,
    path: String,
    sha256: String,
    size: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContentType {
    Program,
    Control,
    Meta,
    Data,
    Unknown(u32),
}

impl ContentType {
    fn from_u32(value: u32) -> Self {
        match value {
            0 => ContentType::Program,
            1 => ContentType::Control,
            2 => ContentType::Meta,
            3 => ContentType::Data,
            other => ContentType::Unknown(other),
        }
    }

    fn as_str(self) -> String {
        match self {
            ContentType::Program => "program".to_string(),
            ContentType::Control => "control".to_string(),
            ContentType::Meta => "meta".to_string(),
            ContentType::Data => "data".to_string(),
            ContentType::Unknown(value) => format!("unknown({value})"),
        }
    }
}

#[derive(Debug)]
struct XciEntry {
    name: String,
    offset: u64,
    size: u64,
    title_id: u64,
    content_type: ContentType,
}

#[derive(Debug)]
struct NcaHeader {
    title_id: u64,
    version: u32,
    content_type: ContentType,
    exefs_offset: u64,
    exefs_size: u64,
    romfs_offset: u64,
    romfs_size: u64,
}

#[derive(Debug)]
struct ExeFsEntry {
    name: String,
    data: Vec<u8>,
}

pub fn intake_xci(options: IntakeOptions) -> Result<IntakeReport, String> {
    let xci_path = absolute_path(&options.xci_path)?;
    let keys_path = absolute_path(&options.keys_path)?;
    let provenance_path = absolute_path(&options.provenance_path)?;
    let out_dir = absolute_path(&options.out_dir)?;

    let provenance_src =
        fs::read_to_string(&provenance_path).map_err(|err| format!("read provenance: {err}"))?;
    let provenance = ProvenanceManifest::parse(&provenance_src)?;
    let validation = provenance.validate(&provenance_path, &provenance_src)?;
    enforce_xci_formats(&validation.inputs)?;
    ensure_input_present(&validation.inputs, &xci_path)?;
    ensure_input_present(&validation.inputs, &keys_path)?;

    validate_keyset(&keys_path)?;

    let xci_bytes = fs::read(&xci_path).map_err(|err| format!("read XCI: {err}"))?;
    let entries = parse_xci_entries(&xci_bytes)?;
    let program_entry = select_program_entry(&entries, &options.program)?;

    let nca_bytes = slice_entry(&xci_bytes, program_entry)?;
    let nca_header = parse_nca_header(nca_bytes)?;
    if nca_header.content_type != ContentType::Program {
        return Err(format!(
            "selected NCA is not program content ({})",
            nca_header.content_type.as_str()
        ));
    }

    let exefs_bytes = slice_region(nca_bytes, nca_header.exefs_offset, nca_header.exefs_size)?;
    let romfs_bytes = slice_region(nca_bytes, nca_header.romfs_offset, nca_header.romfs_size)?;

    let exefs_entries = parse_pfs0(exefs_bytes)?;

    fs::create_dir_all(&out_dir)
        .map_err(|err| format!("create out dir {}: {err}", out_dir.display()))?;
    let intake_dir = out_dir.join("intake");
    let exefs_dir = intake_dir.join("exefs");
    let segments_dir = intake_dir.join("segments");
    let assets_dir = out_dir.join("assets");
    fs::create_dir_all(&exefs_dir).map_err(|err| format!("create exefs dir: {err}"))?;
    fs::create_dir_all(&segments_dir).map_err(|err| format!("create segments dir: {err}"))?;
    fs::create_dir_all(&assets_dir).map_err(|err| format!("create assets dir: {err}"))?;

    let mut files_written = Vec::new();
    let mut generated_files = Vec::new();

    let mut exefs_records = Vec::new();
    for entry in &exefs_entries {
        let output_path = exefs_dir.join(&entry.name);
        fs::write(&output_path, &entry.data)
            .map_err(|err| format!("write exefs {}: {err}", output_path.display()))?;
        files_written.push(output_path.clone());
        let rel_path = path_rel(&output_path, &out_dir);
        exefs_records.push(ExeFsEntryRecord {
            name: entry.name.clone(),
            path: rel_path.clone(),
            sha256: sha256_bytes(&entry.data),
            size: entry.data.len() as u64,
        });
        generated_files.push(GeneratedFile {
            path: rel_path,
            sha256: sha256_bytes(&entry.data),
            size: entry.data.len() as u64,
        });
    }

    let mut segment_records = Vec::new();
    if exefs_entries.iter().any(|entry| entry.name == "main") {
        let main_path = exefs_dir.join("main");
        let nso_module = crate::homebrew::nso::parse_nso(&main_path)?;
        let segments = crate::homebrew::nso::extract_segments(&nso_module)?;
        for segment in segments {
            let kind = segment.segment.kind;
            let kind_label = match kind {
                crate::homebrew::nso::NsoSegmentKind::Text => "text",
                crate::homebrew::nso::NsoSegmentKind::Rodata => "rodata",
                crate::homebrew::nso::NsoSegmentKind::Data => "data",
            };
            let name = format!("main-{kind_label}");
            let output_path = segments_dir.join(format!("{name}.bin"));
            fs::write(&output_path, &segment.data)
                .map_err(|err| format!("write segment {}: {err}", output_path.display()))?;
            files_written.push(output_path.clone());
            let rel_path = path_rel(&output_path, &out_dir);
            let permissions = segment.segment.permissions.as_str();
            let sha = sha256_bytes(&segment.data);
            segment_records.push(SegmentRecord {
                name: name.clone(),
                kind: kind_label.to_string(),
                permissions: permissions.to_string(),
                memory_offset: segment.segment.memory_offset,
                size: segment.segment.size,
                path: rel_path.clone(),
                sha256: sha.clone(),
                file_size: segment.data.len() as u64,
            });
            generated_files.push(GeneratedFile {
                path: rel_path,
                sha256: sha,
                size: segment.data.len() as u64,
            });
        }
    } else {
        return Err("ExeFS missing main NSO entry".to_string());
    }

    let romfs_path = assets_dir.join("romfs.bin");
    fs::write(&romfs_path, romfs_bytes)
        .map_err(|err| format!("write romfs {}: {err}", romfs_path.display()))?;
    files_written.push(romfs_path.clone());
    let romfs_rel = path_rel(&romfs_path, &out_dir);
    let romfs_hash = sha256_bytes(romfs_bytes);
    generated_files.push(GeneratedFile {
        path: romfs_rel.clone(),
        sha256: romfs_hash.clone(),
        size: romfs_bytes.len() as u64,
    });

    let module_json_path = out_dir.join("module.json");
    let module_json = serde_json::to_string_pretty(&ProgramModuleJson {
        schema_version: INTAKE_SCHEMA_VERSION.to_string(),
        module_type: "xci".to_string(),
        title_id: format!("{:#x}", nca_header.title_id),
        program: ProgramRecord {
            name: program_entry.name.clone(),
            title_id: format!("{:#x}", nca_header.title_id),
            version: nca_header.version,
            content_type: nca_header.content_type.as_str(),
            nca_offset: program_entry.offset,
            nca_size: program_entry.size,
            exefs_entries: exefs_records.clone(),
            segments: segment_records.clone(),
        },
        romfs: Some(RomfsRecord {
            path: romfs_rel.clone(),
            sha256: romfs_hash.clone(),
            size: romfs_bytes.len() as u64,
        }),
    })
    .map_err(|err| format!("serialize module.json: {err}"))?;
    fs::write(&module_json_path, &module_json)
        .map_err(|err| format!("write module.json: {err}"))?;
    files_written.push(module_json_path.clone());
    let module_rel = path_rel(&module_json_path, &out_dir);
    generated_files.push(GeneratedFile {
        path: module_rel.clone(),
        sha256: sha256_bytes(module_json.as_bytes()),
        size: module_json.len() as u64,
    });

    let manifest_path = out_dir.join("manifest.json");
    let manifest = IntakeManifest {
        schema_version: INTAKE_SCHEMA_VERSION.to_string(),
        tool: ToolInfo {
            name: "recomp-cli".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
        program: ProgramRecord {
            name: program_entry.name.clone(),
            title_id: format!("{:#x}", nca_header.title_id),
            version: nca_header.version,
            content_type: nca_header.content_type.as_str(),
            nca_offset: program_entry.offset,
            nca_size: program_entry.size,
            exefs_entries: exefs_records,
            segments: segment_records,
        },
        assets: vec![AssetRecord {
            kind: "romfs".to_string(),
            path: romfs_rel,
            sha256: romfs_hash,
            size: romfs_bytes.len() as u64,
        }],
        inputs: input_summaries(&validation),
        manifest_self_hash_basis: "generated_files_self_placeholder".to_string(),
        generated_files,
    };

    let (_manifest, manifest_json) = build_manifest_json(manifest)?;
    fs::write(&manifest_path, manifest_json)
        .map_err(|err| format!("write manifest.json: {err}"))?;
    files_written.push(manifest_path.clone());

    Ok(IntakeReport {
        out_dir,
        module_json_path,
        manifest_path,
        files_written,
    })
}

#[derive(Debug, Serialize)]
struct ProgramModuleJson {
    schema_version: String,
    module_type: String,
    title_id: String,
    program: ProgramRecord,
    #[serde(skip_serializing_if = "Option::is_none")]
    romfs: Option<RomfsRecord>,
}

#[derive(Debug, Serialize)]
struct RomfsRecord {
    path: String,
    sha256: String,
    size: u64,
}

fn build_manifest_json(mut manifest: IntakeManifest) -> Result<(IntakeManifest, String), String> {
    if manifest
        .generated_files
        .iter()
        .any(|file| file.path == MANIFEST_SELF_PATH)
    {
        return Err("intake manifest already present in generated files".to_string());
    }

    manifest.generated_files.push(GeneratedFile {
        path: MANIFEST_SELF_PATH.to_string(),
        sha256: MANIFEST_SELF_SHA_PLACEHOLDER.to_string(),
        size: 0,
    });

    let manifest_json = serde_json::to_string_pretty(&manifest).map_err(|err| err.to_string())?;
    let final_hash = sha256_bytes(manifest_json.as_bytes());
    let final_size = manifest_json.len() as u64;

    for entry in &mut manifest.generated_files {
        if entry.path == MANIFEST_SELF_PATH {
            entry.sha256 = final_hash.clone();
            entry.size = final_size;
        }
    }

    let final_json = serde_json::to_string_pretty(&manifest).map_err(|err| err.to_string())?;
    Ok((manifest, final_json))
}

fn input_summaries(validation: &ProvenanceValidation) -> Vec<InputSummary> {
    validation
        .inputs
        .iter()
        .map(|input| InputSummary {
            path: input.path.clone(),
            format: input.format.as_str().to_string(),
            sha256: input.sha256.clone(),
            size: input.size,
            role: input.role.clone(),
        })
        .collect()
}

fn parse_xci_entries(bytes: &[u8]) -> Result<Vec<XciEntry>, String> {
    if bytes.len() < XCI_HEADER_SIZE {
        return Err(format!("XCI too small: {} bytes", bytes.len()));
    }
    if &bytes[0..4] != XCI_MAGIC {
        return Err("XCI magic mismatch".to_string());
    }
    let entry_count = read_u32(bytes, 0x8)? as usize;
    let entry_size = read_u32(bytes, 0xC)? as usize;
    if entry_size < XCI_ENTRY_SIZE {
        return Err(format!("XCI entry size too small: {entry_size}"));
    }

    let mut entries = Vec::new();
    let mut offset = XCI_HEADER_SIZE;
    for _ in 0..entry_count {
        let entry_bytes = read_bytes(bytes, offset, entry_size)?;
        let name_raw = &entry_bytes[0..32];
        let name = String::from_utf8(
            name_raw
                .iter()
                .cloned()
                .take_while(|byte| *byte != 0)
                .collect::<Vec<u8>>(),
        )
        .map_err(|_| "invalid XCI entry name".to_string())?;
        let entry_offset = read_u64(entry_bytes, 32)?;
        let size = read_u64(entry_bytes, 40)?;
        let title_id = read_u64(entry_bytes, 48)?;
        let content_type = ContentType::from_u32(read_u32(entry_bytes, 60)?);

        if entry_offset
            .checked_add(size)
            .ok_or_else(|| "XCI entry offset overflow".to_string())?
            > bytes.len() as u64
        {
            return Err(format!("XCI entry {name} exceeds file bounds"));
        }
        entries.push(XciEntry {
            name,
            offset: entry_offset,
            size,
            title_id,
            content_type,
        });
        offset = offset
            .checked_add(entry_size)
            .ok_or_else(|| "XCI entry table overflow".to_string())?;
    }
    if entries.is_empty() {
        return Err("XCI contains no entries".to_string());
    }
    Ok(entries)
}

fn select_program_entry<'a>(
    entries: &'a [XciEntry],
    selection: &ProgramSelection,
) -> Result<&'a XciEntry, String> {
    let mut candidates: Vec<&XciEntry> = entries
        .iter()
        .filter(|entry| entry.content_type == ContentType::Program)
        .collect();
    if candidates.is_empty() {
        return Err("XCI contains no program NCA entries".to_string());
    }

    match selection {
        ProgramSelection::TitleId(title_id) => {
            let wanted = parse_title_id(title_id)?;
            candidates.retain(|entry| entry.title_id == wanted);
        }
        ProgramSelection::Name(name) => {
            candidates.retain(|entry| entry.name == *name);
        }
    }

    if candidates.is_empty() {
        return Err("program NCA selection matched no entries".to_string());
    }
    if candidates.len() > 1 {
        return Err(format!(
            "program NCA selection is ambiguous: {} candidates",
            candidates.len()
        ));
    }
    Ok(candidates[0])
}

fn parse_title_id(raw: &str) -> Result<u64, String> {
    let trimmed = raw.trim();
    let trimmed = trimmed.strip_prefix("0x").unwrap_or(trimmed);
    let value = u64::from_str_radix(trimmed, 16);
    value.map_err(|err| format!("invalid title id '{raw}': {err}"))
}

fn slice_entry<'a>(xci: &'a [u8], entry: &XciEntry) -> Result<&'a [u8], String> {
    slice_region(xci, entry.offset, entry.size)
}

fn parse_nca_header(bytes: &[u8]) -> Result<NcaHeader, String> {
    if bytes.len() < 0x40 {
        return Err(format!("NCA too small: {} bytes", bytes.len()));
    }
    if &bytes[0..4] != NCA_MAGIC {
        return Err("NCA magic mismatch".to_string());
    }
    let content_type = ContentType::from_u32(read_u32(bytes, 0x8)?);
    let title_id = read_u64(bytes, 0x10)?;
    let version = read_u32(bytes, 0x18)?;
    let exefs_offset = read_u64(bytes, 0x20)?;
    let exefs_size = read_u64(bytes, 0x28)?;
    let romfs_offset = read_u64(bytes, 0x30)?;
    let romfs_size = read_u64(bytes, 0x38)?;

    if exefs_offset == 0 || exefs_size == 0 {
        return Err("NCA missing ExeFS region".to_string());
    }

    Ok(NcaHeader {
        title_id,
        version,
        content_type,
        exefs_offset,
        exefs_size,
        romfs_offset,
        romfs_size,
    })
}

fn parse_pfs0(bytes: &[u8]) -> Result<Vec<ExeFsEntry>, String> {
    if bytes.len() < 0x10 {
        return Err("PFS0 too small".to_string());
    }
    if &bytes[0..4] != PFS0_MAGIC {
        return Err("ExeFS magic mismatch".to_string());
    }
    let file_count = read_u32(bytes, 0x4)? as usize;
    let string_table_size = read_u32(bytes, 0x8)? as usize;
    let entry_table_offset = 0x10;
    let entry_size = 0x18;
    let string_table_offset = entry_table_offset + entry_size * file_count;
    let data_offset = string_table_offset + string_table_size;

    let mut entries = Vec::new();
    for index in 0..file_count {
        let entry_offset = entry_table_offset + index * entry_size;
        let offset = read_u64(bytes, entry_offset)? as usize;
        let size = read_u64(bytes, entry_offset + 0x8)? as usize;
        let name_offset = read_u32(bytes, entry_offset + 0x10)? as usize;
        let name = read_c_string(bytes, string_table_offset + name_offset)?;
        let start = data_offset + offset;
        let end = start
            .checked_add(size)
            .ok_or_else(|| "ExeFS entry offset overflow".to_string())?;
        if end > bytes.len() {
            return Err(format!("ExeFS entry {name} out of range"));
        }
        entries.push(ExeFsEntry {
            name,
            data: bytes[start..end].to_vec(),
        });
    }
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(entries)
}

fn read_c_string(bytes: &[u8], offset: usize) -> Result<String, String> {
    if offset >= bytes.len() {
        return Err("string table offset out of range".to_string());
    }
    let end = bytes[offset..]
        .iter()
        .position(|byte| *byte == 0)
        .map(|idx| offset + idx)
        .unwrap_or(bytes.len());
    String::from_utf8(bytes[offset..end].to_vec())
        .map_err(|_| "invalid string table entry".to_string())
}

fn slice_region(bytes: &[u8], offset: u64, size: u64) -> Result<&[u8], String> {
    let start = offset as usize;
    let end = start
        .checked_add(size as usize)
        .ok_or_else(|| "slice overflow".to_string())?;
    if end > bytes.len() {
        return Err("slice out of range".to_string());
    }
    Ok(&bytes[start..end])
}

fn validate_keyset(path: &Path) -> Result<(), String> {
    let contents =
        fs::read_to_string(path).map_err(|err| format!("read keyset {}: {err}", path.display()))?;
    let mut saw_key = false;
    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (name, value) = match line.split_once('=') {
            Some((name, value)) => (name.trim(), value.trim()),
            None => continue,
        };
        if name.is_empty() || value.is_empty() {
            continue;
        }
        if value.chars().all(|ch| ch.is_ascii_hexdigit())
            && (value.len() == 32 || value.len() == 64)
        {
            saw_key = true;
            break;
        }
    }
    if !saw_key {
        return Err("keyset missing hex key entries".to_string());
    }
    Ok(())
}

fn enforce_xci_formats(inputs: &[crate::provenance::ValidatedInput]) -> Result<(), String> {
    let mut has_xci = false;
    let mut has_keys = false;
    for input in inputs {
        match input.format {
            InputFormat::Xci => has_xci = true,
            InputFormat::Keyset => has_keys = true,
            _ => {}
        }
    }
    if !has_xci {
        return Err("provenance inputs missing XCI entry".to_string());
    }
    if !has_keys {
        return Err("provenance inputs missing keyset entry".to_string());
    }
    Ok(())
}

fn ensure_input_present(
    inputs: &[crate::provenance::ValidatedInput],
    path: &Path,
) -> Result<(), String> {
    if inputs.iter().any(|input| input.path == path) {
        Ok(())
    } else {
        Err(format!("provenance inputs missing {}", path.display()))
    }
}

fn path_rel(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string()
}

fn absolute_path(path: &Path) -> Result<PathBuf, String> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(std::env::current_dir()
            .map_err(|err| err.to_string())?
            .join(path))
    }
}

fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    format!("{:x}", digest)
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, String> {
    let end = offset
        .checked_add(4)
        .ok_or_else(|| "offset overflow".to_string())?;
    if end > bytes.len() {
        return Err(format!(
            "read_u32 out of range: offset={offset} len={}",
            bytes.len()
        ));
    }
    Ok(u32::from_le_bytes(
        bytes[offset..end].try_into().expect("slice length"),
    ))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, String> {
    let end = offset
        .checked_add(8)
        .ok_or_else(|| "offset overflow".to_string())?;
    if end > bytes.len() {
        return Err(format!(
            "read_u64 out of range: offset={offset} len={}",
            bytes.len()
        ));
    }
    Ok(u64::from_le_bytes(
        bytes[offset..end].try_into().expect("slice length"),
    ))
}

fn read_bytes(bytes: &[u8], offset: usize, size: usize) -> Result<&[u8], String> {
    let end = offset
        .checked_add(size)
        .ok_or_else(|| "offset overflow".to_string())?;
    if end > bytes.len() {
        return Err(format!(
            "read_bytes out of range: offset={offset} size={size} len={}",
            bytes.len()
        ));
    }
    Ok(&bytes[offset..end])
}
