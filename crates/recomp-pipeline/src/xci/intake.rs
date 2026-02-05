use crate::output::{GeneratedFile, InputSummary};
use crate::provenance::{InputFormat, ProvenanceManifest, ProvenanceValidation};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const INTAKE_SCHEMA_VERSION: &str = "1";
const XCI_MAGIC: &[u8; 4] = b"XCI0";
const XCI_HEADER_SIZE: usize = 0x20;
const XCI_ENTRY_SIZE: usize = 0x40;
const NCA_MAGIC: &[u8; 4] = b"NCA3";
const PFS0_MAGIC: &[u8; 4] = b"PFS0";
const MANIFEST_SELF_PATH: &str = "manifest.json";
const MANIFEST_SELF_SHA_PLACEHOLDER: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolKind {
    Auto,
    Hactool,
    HactoolNet,
}

impl ToolKind {
    fn as_str(self) -> &'static str {
        match self {
            ToolKind::Auto => "auto",
            ToolKind::Hactool => "hactool",
            ToolKind::HactoolNet => "hactoolnet",
        }
    }
}

#[derive(Debug)]
pub struct IntakeOptions {
    pub xci_path: PathBuf,
    pub keys_path: PathBuf,
    pub provenance_path: PathBuf,
    pub out_dir: PathBuf,
    pub program: ProgramSelection,
    pub tool_path: Option<PathBuf>,
    pub tool_kind: ToolKind,
    pub title_keys_path: Option<PathBuf>,
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

#[derive(Debug, Deserialize)]
pub struct IntakeManifestSummary {
    pub schema_version: String,
    pub program: IntakeProgramSummary,
    #[serde(default)]
    pub assets: Vec<IntakeAssetSummary>,
    #[serde(default)]
    pub generated_files: Vec<IntakeGeneratedFile>,
}

#[derive(Debug, Deserialize)]
pub struct IntakeProgramSummary {
    pub name: String,
    pub title_id: String,
    pub version: u32,
    pub content_type: String,
}

#[derive(Debug, Deserialize)]
pub struct IntakeAssetSummary {
    pub path: String,
    pub sha256: String,
    pub size: u64,
    pub kind: String,
}

#[derive(Debug, Deserialize)]
pub struct IntakeGeneratedFile {
    pub path: String,
    pub sha256: String,
    pub size: u64,
}

#[derive(Debug)]
pub struct IntakeManifestCheck {
    pub manifest: IntakeManifestSummary,
    pub missing_files: Vec<String>,
}

pub fn read_intake_manifest(path: &Path) -> Result<IntakeManifestSummary, String> {
    let text = fs::read_to_string(path)
        .map_err(|err| format!("read intake manifest {}: {err}", path.display()))?;
    serde_json::from_str(&text).map_err(|err| format!("parse intake manifest json: {err}"))
}

pub fn check_intake_manifest(path: &Path) -> Result<IntakeManifestCheck, String> {
    let manifest = read_intake_manifest(path)?;
    if manifest.schema_version != INTAKE_SCHEMA_VERSION {
        return Err(format!(
            "unsupported intake manifest schema version: {}",
            manifest.schema_version
        ));
    }
    if manifest.program.title_id.trim().is_empty() {
        return Err("intake manifest missing program title_id".to_string());
    }

    let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
    let mut missing = Vec::new();
    for entry in &manifest.generated_files {
        let resolved = base_dir.join(&entry.path);
        if !resolved.exists() {
            missing.push(entry.path.clone());
        }
    }

    Ok(IntakeManifestCheck {
        manifest,
        missing_files: missing,
    })
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
    #[serde(skip_serializing_if = "Option::is_none")]
    xci_tool: Option<ExternalToolInfo>,
}

#[derive(Debug, Serialize)]
struct ExternalToolInfo {
    kind: String,
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
struct ProgramRecord {
    name: String,
    title_id: String,
    version: u32,
    content_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    nca_offset: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    nca_size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    nca_sha256: Option<String>,
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

    fn from_label(value: &str) -> Option<Self> {
        let normalized = value.trim().to_ascii_lowercase();
        match normalized.as_str() {
            "program" => Some(ContentType::Program),
            "control" => Some(ContentType::Control),
            "meta" => Some(ContentType::Meta),
            "data" => Some(ContentType::Data),
            _ => None,
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

#[derive(Debug)]
struct OutputDirs {
    exefs_dir: PathBuf,
    segments_dir: PathBuf,
    assets_dir: PathBuf,
}

#[derive(Debug)]
struct ResolvedTool {
    path: PathBuf,
    kind: ToolKind,
    version: Option<String>,
}

#[derive(Debug)]
struct NcaInfo {
    path: PathBuf,
    name: String,
    title_id: u64,
    content_type: ContentType,
    version: u32,
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

    if is_fixture_xci(&xci_path)? {
        return intake_fixture_xci(&xci_path, &out_dir, &options.program, &validation);
    }

    intake_external_xci(
        &xci_path,
        &keys_path,
        options.title_keys_path.as_deref(),
        &out_dir,
        &options.program,
        options.tool_path.as_deref(),
        options.tool_kind,
        &validation,
    )
}

fn intake_fixture_xci(
    xci_path: &Path,
    out_dir: &Path,
    program: &ProgramSelection,
    validation: &ProvenanceValidation,
) -> Result<IntakeReport, String> {
    let xci_bytes = fs::read(xci_path).map_err(|err| format!("read XCI: {err}"))?;
    let entries = parse_xci_entries(&xci_bytes)?;
    let program_entry = select_program_entry(&entries, program)?;

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

    let dirs = prepare_output_dirs(out_dir)?;
    let mut files_written = Vec::new();
    let mut generated_files = Vec::new();

    let exefs_records = write_exefs_entries(
        &exefs_entries,
        &dirs.exefs_dir,
        out_dir,
        &mut generated_files,
        &mut files_written,
    )?;

    let segments = write_segments_from_main(
        &dirs.exefs_dir,
        &dirs.segments_dir,
        out_dir,
        &mut generated_files,
        &mut files_written,
    )?;

    let (romfs_record, romfs_generated) =
        write_romfs(romfs_bytes, &dirs.assets_dir, out_dir, &mut files_written)?;
    generated_files.push(romfs_generated);
    let romfs_asset = AssetRecord {
        kind: "romfs".to_string(),
        path: romfs_record.path.clone(),
        sha256: romfs_record.sha256.clone(),
        size: romfs_record.size,
    };

    let nca_hash = sha256_bytes(nca_bytes);
    let program_record = ProgramRecord {
        name: program_entry.name.clone(),
        title_id: format!("{:#x}", nca_header.title_id),
        version: nca_header.version,
        content_type: nca_header.content_type.as_str(),
        nca_offset: Some(program_entry.offset),
        nca_size: Some(program_entry.size),
        nca_sha256: Some(nca_hash),
        exefs_entries: exefs_records,
        segments,
    };

    let module_json_path = out_dir.join("module.json");
    let module_json = serde_json::to_string_pretty(&ProgramModuleJson {
        schema_version: INTAKE_SCHEMA_VERSION.to_string(),
        module_type: "xci".to_string(),
        title_id: format!("{:#x}", nca_header.title_id),
        program: program_record.clone(),
        romfs: Some(romfs_record),
    })
    .map_err(|err| format!("serialize module.json: {err}"))?;
    fs::write(&module_json_path, &module_json)
        .map_err(|err| format!("write module.json: {err}"))?;
    files_written.push(module_json_path.clone());
    let module_rel = path_rel(&module_json_path, out_dir);
    generated_files.push(GeneratedFile {
        path: module_rel.clone(),
        sha256: sha256_bytes(module_json.as_bytes()),
        size: module_json.len() as u64,
    });

    let tool = ToolInfo {
        name: "recomp-cli".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        xci_tool: None,
    };

    write_manifest_and_finish(
        out_dir,
        tool,
        program_record,
        vec![romfs_asset],
        input_summaries(validation),
        generated_files,
        files_written,
        module_json_path,
    )
}

#[allow(clippy::too_many_arguments)]
fn intake_external_xci(
    xci_path: &Path,
    keys_path: &Path,
    title_keys_path: Option<&Path>,
    out_dir: &Path,
    program: &ProgramSelection,
    tool_path: Option<&Path>,
    tool_kind: ToolKind,
    validation: &ProvenanceValidation,
) -> Result<IntakeReport, String> {
    let tool = resolve_tool(tool_path, tool_kind)?;
    let extract_dir = out_dir.join("intake/xci-extract");
    if extract_dir.exists() {
        fs::remove_dir_all(&extract_dir)
            .map_err(|err| format!("clear extract dir {}: {err}", extract_dir.display()))?;
    }
    fs::create_dir_all(&extract_dir)
        .map_err(|err| format!("create extract dir {}: {err}", extract_dir.display()))?;

    run_xci_extract(&tool, keys_path, title_keys_path, xci_path, &extract_dir)?;

    let nca_candidates = find_nca_files(&extract_dir)?;
    if nca_candidates.is_empty() {
        return Err("no NCA files found after XCI extraction".to_string());
    }

    let mut nca_infos = Vec::new();
    for candidate in nca_candidates {
        let info = query_nca_info(&tool, keys_path, title_keys_path, &candidate)?;
        nca_infos.push(info);
    }

    let program_nca = select_program_nca(&nca_infos, program)?;

    let nca_bytes = fs::read(&program_nca.path)
        .map_err(|err| format!("read program NCA {}: {err}", program_nca.path.display()))?;
    let nca_hash = sha256_bytes(&nca_bytes);
    let nca_size = nca_bytes.len() as u64;

    let dirs = prepare_output_dirs(out_dir)?;
    let romfs_path = dirs.assets_dir.join("romfs.bin");

    run_nca_extract(
        &tool,
        keys_path,
        title_keys_path,
        &program_nca.path,
        &dirs.exefs_dir,
        &romfs_path,
    )?;

    let mut files_written = Vec::new();
    let mut generated_files = Vec::new();

    let exefs_records = collect_exefs_entries(
        &dirs.exefs_dir,
        out_dir,
        &mut generated_files,
        &mut files_written,
    )?;

    let segments = write_segments_from_main(
        &dirs.exefs_dir,
        &dirs.segments_dir,
        out_dir,
        &mut generated_files,
        &mut files_written,
    )?;

    let (romfs_record, romfs_generated) =
        record_romfs_file(&romfs_path, out_dir, &mut files_written)?;
    generated_files.push(romfs_generated);
    let romfs_asset = AssetRecord {
        kind: "romfs".to_string(),
        path: romfs_record.path.clone(),
        sha256: romfs_record.sha256.clone(),
        size: romfs_record.size,
    };

    let program_record = ProgramRecord {
        name: program_nca.name.clone(),
        title_id: format!("{:#x}", program_nca.title_id),
        version: program_nca.version,
        content_type: program_nca.content_type.as_str(),
        nca_offset: None,
        nca_size: Some(nca_size),
        nca_sha256: Some(nca_hash),
        exefs_entries: exefs_records,
        segments,
    };

    let module_json_path = out_dir.join("module.json");
    let module_json = serde_json::to_string_pretty(&ProgramModuleJson {
        schema_version: INTAKE_SCHEMA_VERSION.to_string(),
        module_type: "xci".to_string(),
        title_id: format!("{:#x}", program_nca.title_id),
        program: program_record.clone(),
        romfs: Some(romfs_record.clone()),
    })
    .map_err(|err| format!("serialize module.json: {err}"))?;
    fs::write(&module_json_path, &module_json)
        .map_err(|err| format!("write module.json: {err}"))?;
    files_written.push(module_json_path.clone());
    let module_rel = path_rel(&module_json_path, out_dir);
    generated_files.push(GeneratedFile {
        path: module_rel.clone(),
        sha256: sha256_bytes(module_json.as_bytes()),
        size: module_json.len() as u64,
    });

    let tool_info = ToolInfo {
        name: "recomp-cli".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        xci_tool: Some(ExternalToolInfo {
            kind: tool.kind.as_str().to_string(),
            path: tool.path.display().to_string(),
            version: tool.version.clone(),
        }),
    };

    let report = write_manifest_and_finish(
        out_dir,
        tool_info,
        program_record,
        vec![romfs_asset],
        input_summaries(validation),
        generated_files,
        files_written,
        module_json_path,
    )?;

    let _ = fs::remove_dir_all(&extract_dir);

    Ok(report)
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

#[derive(Debug, Serialize, Clone)]
struct RomfsRecord {
    path: String,
    sha256: String,
    size: u64,
}

#[allow(clippy::too_many_arguments)]
fn write_manifest_and_finish(
    out_dir: &Path,
    tool: ToolInfo,
    program: ProgramRecord,
    assets: Vec<AssetRecord>,
    inputs: Vec<InputSummary>,
    generated_files: Vec<GeneratedFile>,
    mut files_written: Vec<PathBuf>,
    module_json_path: PathBuf,
) -> Result<IntakeReport, String> {
    let manifest_path = out_dir.join("manifest.json");
    let manifest = IntakeManifest {
        schema_version: INTAKE_SCHEMA_VERSION.to_string(),
        tool,
        program,
        assets,
        inputs,
        manifest_self_hash_basis: "generated_files_self_placeholder".to_string(),
        generated_files,
    };

    let (_manifest, manifest_json) = build_manifest_json(manifest)?;
    fs::write(&manifest_path, manifest_json)
        .map_err(|err| format!("write manifest.json: {err}"))?;
    files_written.push(manifest_path.clone());

    Ok(IntakeReport {
        out_dir: out_dir.to_path_buf(),
        module_json_path,
        manifest_path,
        files_written,
    })
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

fn prepare_output_dirs(out_dir: &Path) -> Result<OutputDirs, String> {
    fs::create_dir_all(out_dir)
        .map_err(|err| format!("create out dir {}: {err}", out_dir.display()))?;
    let intake_dir = out_dir.join("intake");
    let exefs_dir = intake_dir.join("exefs");
    let segments_dir = intake_dir.join("segments");
    let assets_dir = out_dir.join("assets");
    fs::create_dir_all(&exefs_dir).map_err(|err| format!("create exefs dir: {err}"))?;
    fs::create_dir_all(&segments_dir).map_err(|err| format!("create segments dir: {err}"))?;
    fs::create_dir_all(&assets_dir).map_err(|err| format!("create assets dir: {err}"))?;
    Ok(OutputDirs {
        exefs_dir,
        segments_dir,
        assets_dir,
    })
}

fn write_exefs_entries(
    entries: &[ExeFsEntry],
    exefs_dir: &Path,
    out_dir: &Path,
    generated_files: &mut Vec<GeneratedFile>,
    files_written: &mut Vec<PathBuf>,
) -> Result<Vec<ExeFsEntryRecord>, String> {
    let mut records = Vec::new();
    for entry in entries {
        let output_path = exefs_dir.join(&entry.name);
        fs::write(&output_path, &entry.data)
            .map_err(|err| format!("write exefs {}: {err}", output_path.display()))?;
        files_written.push(output_path.clone());
        let rel_path = path_rel(&output_path, out_dir);
        let sha = sha256_bytes(&entry.data);
        records.push(ExeFsEntryRecord {
            name: entry.name.clone(),
            path: rel_path.clone(),
            sha256: sha.clone(),
            size: entry.data.len() as u64,
        });
        generated_files.push(GeneratedFile {
            path: rel_path,
            sha256: sha,
            size: entry.data.len() as u64,
        });
    }
    Ok(records)
}

fn collect_exefs_entries(
    exefs_dir: &Path,
    out_dir: &Path,
    generated_files: &mut Vec<GeneratedFile>,
    files_written: &mut Vec<PathBuf>,
) -> Result<Vec<ExeFsEntryRecord>, String> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(exefs_dir)
        .map_err(|err| format!("read exefs dir {}: {err}", exefs_dir.display()))?
    {
        let entry = entry.map_err(|err| format!("read exefs entry: {err}"))?;
        if entry.file_type().map_err(|err| err.to_string())?.is_file() {
            entries.push(entry.path());
        }
    }
    entries.sort();

    let mut records = Vec::new();
    for path in entries {
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| "invalid exefs filename".to_string())?
            .to_string();
        let bytes =
            fs::read(&path).map_err(|err| format!("read exefs {}: {err}", path.display()))?;
        files_written.push(path.clone());
        let rel_path = path_rel(&path, out_dir);
        let sha = sha256_bytes(&bytes);
        records.push(ExeFsEntryRecord {
            name,
            path: rel_path.clone(),
            sha256: sha.clone(),
            size: bytes.len() as u64,
        });
        generated_files.push(GeneratedFile {
            path: rel_path,
            sha256: sha,
            size: bytes.len() as u64,
        });
    }

    Ok(records)
}

fn write_segments_from_main(
    exefs_dir: &Path,
    segments_dir: &Path,
    out_dir: &Path,
    generated_files: &mut Vec<GeneratedFile>,
    files_written: &mut Vec<PathBuf>,
) -> Result<Vec<SegmentRecord>, String> {
    let main_path = exefs_dir.join("main");
    if !main_path.exists() {
        return Err("ExeFS missing main NSO entry".to_string());
    }
    let nso_module = crate::homebrew::nso::parse_nso(&main_path)?;
    let segments = crate::homebrew::nso::extract_segments(&nso_module)?;

    let mut records = Vec::new();
    for segment in segments {
        let kind_label = match segment.segment.kind {
            crate::homebrew::nso::NsoSegmentKind::Text => "text",
            crate::homebrew::nso::NsoSegmentKind::Rodata => "rodata",
            crate::homebrew::nso::NsoSegmentKind::Data => "data",
        };
        let name = format!("main-{kind_label}");
        let output_path = segments_dir.join(format!("{name}.bin"));
        fs::write(&output_path, &segment.data)
            .map_err(|err| format!("write segment {}: {err}", output_path.display()))?;
        files_written.push(output_path.clone());
        let rel_path = path_rel(&output_path, out_dir);
        let sha = sha256_bytes(&segment.data);
        records.push(SegmentRecord {
            name: name.clone(),
            kind: kind_label.to_string(),
            permissions: segment.segment.permissions.as_str().to_string(),
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

    Ok(records)
}

fn write_romfs(
    romfs_bytes: &[u8],
    assets_dir: &Path,
    out_dir: &Path,
    files_written: &mut Vec<PathBuf>,
) -> Result<(RomfsRecord, GeneratedFile), String> {
    let romfs_path = assets_dir.join("romfs.bin");
    fs::write(&romfs_path, romfs_bytes)
        .map_err(|err| format!("write romfs {}: {err}", romfs_path.display()))?;
    files_written.push(romfs_path.clone());
    let romfs_rel = path_rel(&romfs_path, out_dir);
    let romfs_hash = sha256_bytes(romfs_bytes);
    let generated = GeneratedFile {
        path: romfs_rel.clone(),
        sha256: romfs_hash.clone(),
        size: romfs_bytes.len() as u64,
    };
    let record = RomfsRecord {
        path: romfs_rel,
        sha256: romfs_hash,
        size: romfs_bytes.len() as u64,
    };
    Ok((record, generated))
}

fn record_romfs_file(
    romfs_path: &Path,
    out_dir: &Path,
    files_written: &mut Vec<PathBuf>,
) -> Result<(RomfsRecord, GeneratedFile), String> {
    let romfs_bytes = fs::read(romfs_path)
        .map_err(|err| format!("read romfs {}: {err}", romfs_path.display()))?;
    files_written.push(romfs_path.to_path_buf());
    let romfs_rel = path_rel(romfs_path, out_dir);
    let romfs_hash = sha256_bytes(&romfs_bytes);
    let generated = GeneratedFile {
        path: romfs_rel.clone(),
        sha256: romfs_hash.clone(),
        size: romfs_bytes.len() as u64,
    };
    let record = RomfsRecord {
        path: romfs_rel,
        sha256: romfs_hash,
        size: romfs_bytes.len() as u64,
    };
    Ok((record, generated))
}

fn run_xci_extract(
    tool: &ResolvedTool,
    keys_path: &Path,
    title_keys_path: Option<&Path>,
    xci_path: &Path,
    out_dir: &Path,
) -> Result<(), String> {
    let mut args = Vec::new();
    push_input_type(tool, "xci", &mut args);
    args.push("--outdir".to_string());
    args.push(out_dir.display().to_string());
    args.push(xci_path.display().to_string());
    let output = run_tool(tool, keys_path, title_keys_path, &args)?;
    if !output.status.success() {
        return Err(format!(
            "hactool XCI extract failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}

fn run_nca_extract(
    tool: &ResolvedTool,
    keys_path: &Path,
    title_keys_path: Option<&Path>,
    nca_path: &Path,
    exefs_dir: &Path,
    romfs_path: &Path,
) -> Result<(), String> {
    let mut args = Vec::new();
    push_input_type(tool, "nca", &mut args);
    args.push("--exefsdir".to_string());
    args.push(exefs_dir.display().to_string());
    args.push("--romfs".to_string());
    args.push(romfs_path.display().to_string());
    args.push(nca_path.display().to_string());
    let output = run_tool(tool, keys_path, title_keys_path, &args)?;
    if !output.status.success() {
        return Err(format!(
            "hactool NCA extract failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}

fn query_nca_info(
    tool: &ResolvedTool,
    keys_path: &Path,
    title_keys_path: Option<&Path>,
    nca_path: &Path,
) -> Result<NcaInfo, String> {
    let mut args = Vec::new();
    push_input_type(tool, "nca", &mut args);
    args.push("-i".to_string());
    args.push(nca_path.display().to_string());
    let output = run_tool(tool, keys_path, title_keys_path, &args)?;
    if !output.status.success() {
        return Err(format!(
            "hactool NCA info failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_nca_info(nca_path, &stdout)
}

fn parse_nca_info(nca_path: &Path, output: &str) -> Result<NcaInfo, String> {
    let mut title_id = None;
    let mut content_type = None;
    let mut version = None;

    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let lower = trimmed.to_ascii_lowercase();
        if lower.starts_with("title id") {
            if let Some(value) = trimmed.split(':').nth(1) {
                title_id = Some(parse_hex_u64(value.trim())?);
            }
        } else if lower.starts_with("content type") {
            if let Some(value) = trimmed.split(':').nth(1) {
                content_type = ContentType::from_label(value.trim());
            }
        } else if lower.starts_with("version") {
            if let Some(value) = trimmed.split(':').nth(1) {
                version = Some(parse_u32_value(value.trim())?);
            }
        }
    }

    let title_id = title_id.ok_or_else(|| "missing Title ID in NCA info".to_string())?;
    let content_type =
        content_type.ok_or_else(|| "missing Content Type in NCA info".to_string())?;
    let version = version.ok_or_else(|| "missing Version in NCA info".to_string())?;
    let name = nca_path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "invalid NCA filename".to_string())?
        .to_string();

    Ok(NcaInfo {
        path: nca_path.to_path_buf(),
        name,
        title_id,
        content_type,
        version,
    })
}

fn select_program_nca<'a>(
    candidates: &'a [NcaInfo],
    selection: &ProgramSelection,
) -> Result<&'a NcaInfo, String> {
    let mut programs: Vec<&NcaInfo> = candidates
        .iter()
        .filter(|info| info.content_type == ContentType::Program)
        .collect();

    if programs.is_empty() {
        return Err("no program NCA entries found".to_string());
    }

    match selection {
        ProgramSelection::TitleId(title_id) => {
            let wanted = parse_hex_u64(title_id)?;
            programs.retain(|info| info.title_id == wanted);
        }
        ProgramSelection::Name(name) => {
            programs.retain(|info| info.name == *name);
        }
    }

    if programs.is_empty() {
        return Err("program NCA selection matched no entries".to_string());
    }
    if programs.len() > 1 {
        return Err(format!(
            "program NCA selection is ambiguous: {} candidates",
            programs.len()
        ));
    }

    Ok(programs[0])
}

fn resolve_tool(tool_path: Option<&Path>, tool_kind: ToolKind) -> Result<ResolvedTool, String> {
    let (path, kind) = match (tool_path, tool_kind) {
        (Some(path), ToolKind::Auto) => (path.to_path_buf(), infer_tool_kind(path)),
        (Some(path), kind) => (path.to_path_buf(), kind),
        (None, ToolKind::Auto) => find_auto_tool()?,
        (None, ToolKind::Hactool) => {
            let path =
                find_in_path("hactool").ok_or_else(|| "hactool not found in PATH".to_string())?;
            (path, ToolKind::Hactool)
        }
        (None, ToolKind::HactoolNet) => {
            let path = find_in_path("hactoolnet")
                .or_else(|| find_in_path("hactoolnet.exe"))
                .ok_or_else(|| "hactoolnet not found in PATH".to_string())?;
            (path, ToolKind::HactoolNet)
        }
    };

    let version = query_tool_version(&path);

    Ok(ResolvedTool {
        path,
        kind,
        version,
    })
}

fn find_auto_tool() -> Result<(PathBuf, ToolKind), String> {
    if let Some(path) = find_in_path("hactoolnet") {
        return Ok((path, ToolKind::HactoolNet));
    }
    if let Some(path) = find_in_path("hactoolnet.exe") {
        return Ok((path, ToolKind::HactoolNet));
    }
    if let Some(path) = find_in_path("hactool") {
        return Ok((path, ToolKind::Hactool));
    }
    Err("no hactool or hactoolnet found in PATH".to_string())
}

fn infer_tool_kind(path: &Path) -> ToolKind {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if file_name.contains("hactoolnet") {
        ToolKind::HactoolNet
    } else {
        ToolKind::Hactool
    }
}

fn query_tool_version(path: &Path) -> Option<String> {
    let output = Command::new(path).arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

fn run_tool(
    tool: &ResolvedTool,
    keys_path: &Path,
    title_keys_path: Option<&Path>,
    args: &[String],
) -> Result<Output, String> {
    let mut cmd = Command::new(&tool.path);
    match tool.kind {
        ToolKind::Hactool => {
            cmd.arg("-k").arg(keys_path);
        }
        ToolKind::HactoolNet => {
            cmd.arg("--keyset").arg(keys_path);
        }
        ToolKind::Auto => {}
    }
    if tool.kind == ToolKind::HactoolNet {
        if let Some(title_keys) = title_keys_path {
            cmd.arg("--titlekeys").arg(title_keys);
        }
    }
    for arg in args {
        cmd.arg(arg);
    }
    cmd.output()
        .map_err(|err| format!("failed to run {}: {err}", tool.path.display()))
}

fn push_input_type(tool: &ResolvedTool, kind: &str, args: &mut Vec<String>) {
    match tool.kind {
        ToolKind::Hactool => {
            args.push("-t".to_string());
            args.push(kind.to_string());
        }
        ToolKind::HactoolNet => {
            args.push("--intype".to_string());
            args.push(kind.to_string());
        }
        ToolKind::Auto => {}
    }
}

fn find_nca_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    collect_files(root, &mut out)?;
    out.retain(|path| path.extension().and_then(OsStr::to_str) == Some("nca"));
    out.sort();
    Ok(out)
}

fn collect_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(dir).map_err(|err| format!("read dir {}: {err}", dir.display()))? {
        let entry = entry.map_err(|err| format!("read dir entry: {err}"))?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|err| err.to_string())?;
        if file_type.is_dir() {
            collect_files(&path, out)?;
        } else if file_type.is_file() {
            out.push(path);
        }
    }
    Ok(())
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
            let wanted = parse_hex_u64(title_id)?;
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

fn is_fixture_xci(path: &Path) -> Result<bool, String> {
    let mut file = fs::File::open(path).map_err(|err| format!("read XCI: {err}"))?;
    let mut magic = [0u8; 4];
    use std::io::Read;
    file.read_exact(&mut magic)
        .map_err(|err| format!("read XCI header: {err}"))?;
    Ok(&magic == XCI_MAGIC)
}

fn parse_hex_u64(raw: &str) -> Result<u64, String> {
    let trimmed = raw.trim();
    let trimmed = trimmed.strip_prefix("0x").unwrap_or(trimmed);
    u64::from_str_radix(trimmed, 16).map_err(|err| format!("invalid hex value '{raw}': {err}"))
}

fn parse_u32_value(raw: &str) -> Result<u32, String> {
    let trimmed = raw.trim();
    if let Some(stripped) = trimmed.strip_prefix("0x") {
        return u32::from_str_radix(stripped, 16)
            .map_err(|err| format!("invalid value '{raw}': {err}"));
    }
    if trimmed
        .chars()
        .any(|ch| matches!(ch, 'a'..='f' | 'A'..='F'))
    {
        u32::from_str_radix(trimmed, 16).map_err(|err| format!("invalid value '{raw}': {err}"))
    } else {
        trimmed
            .parse::<u32>()
            .map_err(|err| format!("invalid value '{raw}': {err}"))
    }
}

fn slice_entry<'a>(xci: &'a [u8], entry: &XciEntry) -> Result<&'a [u8], String> {
    slice_region(xci, entry.offset, entry.size)
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

fn find_in_path(name: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    for entry in std::env::split_paths(&path_var) {
        let candidate = entry.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}
