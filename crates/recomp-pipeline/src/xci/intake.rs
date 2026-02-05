use crate::homebrew::nso;
use crate::homebrew::romfs::list_romfs_entries;
use crate::input::{Function, Module, ModuleSegment, ModuleSegmentPermissions, Op};
use crate::output::{GeneratedFile, InputSummary};
use crate::provenance::{InputFormat, ProvenanceManifest, ProvenanceValidation};
use crate::xci::external::{ExternalXciExtractor, XciToolPreference};
use crate::xci::mock::MockXciExtractor;
use crate::xci::types::{XciExtractRequest, XciExtractor, XciFile, XciProgram};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Component, Path, PathBuf};

const INTAKE_SCHEMA_VERSION: &str = "1";
const MANIFEST_SELF_PATH: &str = "manifest.json";
const MANIFEST_SELF_SHA_PLACEHOLDER: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Debug)]
pub struct XciIntakeOptions {
    pub xci_path: PathBuf,
    pub keys_path: PathBuf,
    pub config_path: Option<PathBuf>,
    pub provenance_path: PathBuf,
    pub out_dir: PathBuf,
    pub assets_dir: PathBuf,
    pub tool_preference: XciToolPreference,
    pub tool_path: Option<PathBuf>,
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
    pub assets_root: Option<String>,
    #[serde(default)]
    pub assets: Vec<IntakeAssetSummary>,
    #[serde(default)]
    pub generated_files: Vec<IntakeGeneratedFile>,
}

#[derive(Debug, Deserialize)]
pub struct IntakeProgramSummary {
    #[serde(default)]
    pub name: Option<String>,
    pub title_id: String,
    pub version: String,
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

pub fn intake_xci(options: XciIntakeOptions) -> Result<IntakeReport, String> {
    let manifest_src = fs::read_to_string(&options.provenance_path).map_err(|err| {
        format!(
            "read provenance {}: {err}",
            options.provenance_path.display()
        )
    })?;
    let manifest = ProvenanceManifest::parse(&manifest_src)?;
    let validation = manifest.validate(&options.provenance_path, &manifest_src)?;

    ensure_input_present(&validation, &options.xci_path, InputFormat::Xci)?;
    ensure_input_present(&validation, &options.keys_path, InputFormat::Keyset)?;

    let extract_request = XciExtractRequest {
        xci_path: options.xci_path.clone(),
        keys_path: options.keys_path.clone(),
    };

    let external =
        ExternalXciExtractor::detect(options.tool_preference, options.tool_path.as_deref())?;
    let (extractor, used_mock): (Box<dyn XciExtractor>, bool) = match external {
        Some(extractor) => (Box::new(extractor), false),
        None => (Box::new(MockXciExtractor::new()), true),
    };

    let extract = extractor.extract(&extract_request)?;
    let selection = match options.config_path.as_ref() {
        Some(path) => Some(load_selection_config(path)?),
        None => None,
    };
    let program = select_program(extract.programs, selection.as_ref())?;

    write_intake_outputs(&options, &validation, &program, used_mock)
}

#[derive(Debug, Default, Deserialize)]
struct XciSelectionConfig {
    #[serde(default)]
    program_title_id: Option<String>,
    #[serde(default)]
    program_version: Option<String>,
    #[serde(default)]
    program_content_type: Option<String>,
}

fn load_selection_config(path: &Path) -> Result<XciSelectionConfig, String> {
    let src = fs::read_to_string(path)
        .map_err(|err| format!("read xci intake config {}: {err}", path.display()))?;
    toml::from_str(&src).map_err(|err| format!("parse xci intake config: {err}"))
}

#[derive(Debug, Serialize)]
struct IntakeManifest {
    schema_version: String,
    tool: ToolInfo,
    program: ProgramRecord,
    assets_root: String,
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
    version: String,
    content_type: String,
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

fn write_intake_outputs(
    options: &XciIntakeOptions,
    validation: &ProvenanceValidation,
    program: &XciProgram,
    used_mock: bool,
) -> Result<IntakeReport, String> {
    ensure_separate_roots(&options.out_dir, &options.assets_dir)?;
    fs::create_dir_all(&options.out_dir)
        .map_err(|err| format!("create out dir {}: {err}", options.out_dir.display()))?;
    fs::create_dir_all(&options.assets_dir)
        .map_err(|err| format!("create assets dir {}: {err}", options.assets_dir.display()))?;

    let exefs_dir = options.out_dir.join("exefs");
    let segments_root = options.out_dir.join("segments");
    let segments_dir = segments_root.join("main");
    let romfs_dir = options.assets_dir.join("romfs");

    fs::create_dir_all(&exefs_dir)
        .map_err(|err| format!("create exefs dir {}: {err}", exefs_dir.display()))?;
    fs::create_dir_all(&segments_dir)
        .map_err(|err| format!("create segments dir {}: {err}", segments_dir.display()))?;
    fs::create_dir_all(&romfs_dir)
        .map_err(|err| format!("create romfs dir {}: {err}", romfs_dir.display()))?;

    let mut generated_files = Vec::new();
    let mut files_written = Vec::new();

    let mut exefs_records = Vec::new();
    for file in &program.exefs_files {
        let out_path = exefs_dir.join(&file.name);
        fs::write(&out_path, &file.data)
            .map_err(|err| format!("write exefs {}: {err}", out_path.display()))?;
        files_written.push(out_path.clone());
        let rel_path = path_rel(&out_path, &options.out_dir);
        let sha = sha256_bytes(&file.data);
        exefs_records.push(ExeFsEntryRecord {
            name: file.name.clone(),
            path: rel_path.clone(),
            sha256: sha.clone(),
            size: file.data.len() as u64,
        });
        generated_files.push(GeneratedFile {
            path: rel_path,
            sha256: sha,
            size: file.data.len() as u64,
        });
    }

    let main_nso_name = select_main_nso(program)?;
    let main_path = exefs_dir.join(&main_nso_name);
    let (segment_records, module_segments) = write_segments_from_main(
        &main_path,
        &segments_dir,
        &options.out_dir,
        &mut generated_files,
        &mut files_written,
    )?;

    let mut assets = Vec::new();
    let romfs_entries = extract_romfs_entries(program)?;
    for entry in romfs_entries {
        let rel_path = validate_romfs_path(&entry.name)?;
        let out_path = romfs_dir.join(&rel_path);
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("create romfs dir {}: {err}", parent.display()))?;
        }
        fs::write(&out_path, &entry.data)
            .map_err(|err| format!("write romfs entry {}: {err}", out_path.display()))?;
        files_written.push(out_path.clone());
        let rel_asset_path = path_rel(&out_path, &options.assets_dir);
        assets.push(AssetRecord {
            kind: "romfs".to_string(),
            path: rel_asset_path,
            sha256: sha256_bytes(&entry.data),
            size: entry.data.len() as u64,
        });
    }

    let module_json = Module {
        arch: "aarch64".to_string(),
        segments: module_segments,
        functions: vec![Function {
            name: "entry".to_string(),
            ops: vec![Op::Ret],
            blocks: Vec::new(),
        }],
    };
    let module_json_path = options.out_dir.join("module.json");
    let module_json_src =
        serde_json::to_string_pretty(&module_json).map_err(|err| err.to_string())?;
    fs::write(&module_json_path, &module_json_src)
        .map_err(|err| format!("write module.json: {err}"))?;
    files_written.push(module_json_path.clone());
    generated_files.push(GeneratedFile {
        path: "module.json".to_string(),
        sha256: sha256_bytes(module_json_src.as_bytes()),
        size: module_json_src.len() as u64,
    });

    generated_files.sort_by(|a, b| a.path.cmp(&b.path));
    assets.sort_by(|a, b| a.path.cmp(&b.path));

    let tool_info = ToolInfo {
        name: "recomp-pipeline".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        xci_tool: Some(build_tool_info(options, used_mock)),
    };

    let inputs = validation
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

    let program_record = ProgramRecord {
        name: format!("program-{}", program.title_id),
        title_id: program.title_id.clone(),
        version: program.version.clone(),
        content_type: program.content_type.clone(),
        nca_sha256: Some(sha256_bytes(&program.nca_bytes)),
        exefs_entries: exefs_records,
        segments: segment_records,
    };

    let manifest = IntakeManifest {
        schema_version: INTAKE_SCHEMA_VERSION.to_string(),
        tool: tool_info,
        program: program_record,
        assets_root: options.assets_dir.display().to_string(),
        assets,
        inputs,
        manifest_self_hash_basis: "generated_files_self_placeholder".to_string(),
        generated_files,
    };

    let manifest_path = options.out_dir.join("manifest.json");
    let (_manifest, manifest_json) = build_manifest_json(manifest)?;
    fs::write(&manifest_path, manifest_json)
        .map_err(|err| format!("write manifest.json: {err}"))?;
    files_written.push(manifest_path.clone());

    Ok(IntakeReport {
        out_dir: options.out_dir.clone(),
        module_json_path: module_json_path.clone(),
        manifest_path,
        files_written,
    })
}

fn ensure_separate_roots(out_dir: &Path, assets_dir: &Path) -> Result<(), String> {
    let out = normalize_path(out_dir);
    let assets = normalize_path(assets_dir);
    if assets.starts_with(&out) {
        return Err("assets_dir must not be inside out_dir".to_string());
    }
    if out.starts_with(&assets) {
        return Err("out_dir must not be inside assets_dir".to_string());
    }
    Ok(())
}

fn normalize_path(path: &Path) -> PathBuf {
    if let Ok(canon) = path.canonicalize() {
        return canon;
    }
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    };
    let mut out = PathBuf::new();
    for comp in absolute.components() {
        match comp {
            Component::Prefix(prefix) => out.push(prefix.as_os_str()),
            Component::RootDir => out.push(Path::new(std::path::MAIN_SEPARATOR_STR)),
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            Component::Normal(part) => out.push(part),
        }
    }
    out
}

fn select_program(
    programs: Vec<XciProgram>,
    selection: Option<&XciSelectionConfig>,
) -> Result<XciProgram, String> {
    let mut programs = programs;
    let content_type = selection
        .and_then(|cfg| cfg.program_content_type.clone())
        .unwrap_or_else(|| "program".to_string());
    programs.retain(|program| program.content_type.eq_ignore_ascii_case(&content_type));

    if let Some(cfg) = selection {
        if let Some(title_id) = &cfg.program_title_id {
            programs.retain(|program| program.title_id.eq_ignore_ascii_case(title_id));
        }
        if let Some(version) = &cfg.program_version {
            programs.retain(|program| program.version == *version);
        }
    }

    if programs.is_empty() {
        return Err(format!(
            "no program content found in XCI (content_type={content_type})"
        ));
    }
    if programs.len() > 1 {
        return Err("ambiguous Program NCA selection".to_string());
    }
    Ok(programs.remove(0))
}

fn select_main_nso(program: &XciProgram) -> Result<String, String> {
    if let Some(main) = program.nso_files.iter().find(|file| file.name == "main") {
        return Ok(main.name.clone());
    }
    if let Some(first) = program.nso_files.first() {
        return Ok(first.name.clone());
    }
    if let Some(exefs) = program
        .exefs_files
        .iter()
        .find(|file| is_nso_name(&file.name))
    {
        return Ok(exefs.name.clone());
    }
    Err("program ExeFS missing main NSO".to_string())
}

fn is_nso_name(name: &str) -> bool {
    if name == "main" {
        return true;
    }
    if name.ends_with(".nso") {
        return true;
    }
    !name.contains('.') && name != "main.npdm"
}

fn write_segments_from_main(
    main_path: &Path,
    segments_dir: &Path,
    out_dir: &Path,
    generated_files: &mut Vec<GeneratedFile>,
    files_written: &mut Vec<PathBuf>,
) -> Result<(Vec<SegmentRecord>, Vec<ModuleSegment>), String> {
    if !main_path.exists() {
        return Err(format!(
            "ExeFS missing main NSO entry: {}",
            main_path.display()
        ));
    }
    let nso_module = nso::parse_nso(main_path)?;
    let segments = nso::extract_segments(&nso_module)?;

    let mut records = Vec::new();
    let mut module_segments = Vec::new();
    for segment in segments {
        let kind_label = match segment.segment.kind {
            nso::NsoSegmentKind::Text => "text",
            nso::NsoSegmentKind::Rodata => "rodata",
            nso::NsoSegmentKind::Data => "data",
        };
        let output_path = segments_dir.join(format!("{kind_label}.bin"));
        fs::write(&output_path, &segment.data)
            .map_err(|err| format!("write segment {}: {err}", output_path.display()))?;
        files_written.push(output_path.clone());
        let rel_path = path_rel(&output_path, out_dir);
        let sha = sha256_bytes(&segment.data);
        records.push(SegmentRecord {
            name: format!("main-{kind_label}"),
            kind: kind_label.to_string(),
            permissions: segment.segment.permissions.as_str().to_string(),
            memory_offset: segment.segment.memory_offset,
            size: segment.segment.size,
            path: rel_path.clone(),
            sha256: sha.clone(),
            file_size: segment.data.len() as u64,
        });
        generated_files.push(GeneratedFile {
            path: rel_path.clone(),
            sha256: sha,
            size: segment.data.len() as u64,
        });
        module_segments.push(ModuleSegment {
            name: format!("main-{kind_label}"),
            base: segment.segment.memory_offset as u64,
            size: segment.segment.size as u64,
            permissions: map_permissions(segment.segment.permissions),
            init_path: Some(rel_path),
            init_size: Some(segment.data.len() as u64),
            zero_fill: false,
        });
    }

    Ok((records, module_segments))
}

fn map_permissions(perms: nso::NsoSegmentPermissions) -> ModuleSegmentPermissions {
    match perms {
        nso::NsoSegmentPermissions::Rx => ModuleSegmentPermissions {
            read: true,
            write: false,
            execute: true,
        },
        nso::NsoSegmentPermissions::R => ModuleSegmentPermissions {
            read: true,
            write: false,
            execute: false,
        },
        nso::NsoSegmentPermissions::Rw => ModuleSegmentPermissions {
            read: true,
            write: true,
            execute: false,
        },
    }
}

fn extract_romfs_entries(program: &XciProgram) -> Result<Vec<XciFile>, String> {
    if !program.romfs_entries.is_empty() {
        return Ok(program.romfs_entries.clone());
    }
    let Some(romfs_bytes) = &program.romfs_image else {
        return Ok(Vec::new());
    };
    let entries = list_romfs_entries(romfs_bytes)?;
    let mut out = Vec::new();
    for entry in entries {
        let start = entry.data_offset as usize;
        let end = start
            .checked_add(entry.data_size as usize)
            .ok_or_else(|| "romfs entry size overflow".to_string())?;
        if end > romfs_bytes.len() {
            return Err(format!(
                "romfs entry out of range: {}..{} (len={})",
                start,
                end,
                romfs_bytes.len()
            ));
        }
        out.push(XciFile {
            name: entry.path,
            data: romfs_bytes[start..end].to_vec(),
        });
    }
    Ok(out)
}

fn validate_romfs_path(path: &str) -> Result<PathBuf, String> {
    let rel = Path::new(path);
    if rel.is_absolute() {
        return Err(format!("romfs entry path is absolute: {path}"));
    }
    for component in rel.components() {
        match component {
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(format!("romfs entry path is invalid: {path}"));
            }
            _ => {}
        }
    }
    Ok(rel.to_path_buf())
}

fn ensure_input_present(
    validation: &ProvenanceValidation,
    path: &Path,
    format: InputFormat,
) -> Result<(), String> {
    let found = validation
        .inputs
        .iter()
        .any(|input| input.format == format && input.path == path);
    if found {
        Ok(())
    } else {
        Err(format!(
            "provenance missing required input: {} ({})",
            path.display(),
            format.as_str()
        ))
    }
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

fn build_tool_info(options: &XciIntakeOptions, used_mock: bool) -> ExternalToolInfo {
    let kind = if used_mock {
        "mock".to_string()
    } else {
        match options.tool_preference {
            XciToolPreference::Auto => "auto",
            XciToolPreference::Hactool => "hactool",
            XciToolPreference::Hactoolnet => "hactoolnet",
            XciToolPreference::Mock => "mock",
        }
        .to_string()
    };
    let path = options
        .tool_path
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "auto".to_string());
    ExternalToolInfo {
        kind,
        path,
        version: None,
    }
}

fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    format!("{:x}", digest)
}

fn path_rel(path: &Path, base: &Path) -> String {
    path.strip_prefix(base)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}
