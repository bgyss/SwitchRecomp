use crate::homebrew::module::{BssInfo, ModuleBuild, ModuleJson, ModuleSegment, OffsetInfo};
use crate::homebrew::nso::{extract_segments, parse_nso, NsoModule, NsoSegmentKind};
use crate::homebrew::romfs::{list_romfs_entries, RomfsEntry};
use crate::output::{GeneratedFile, InputSummary};
use crate::provenance::{InputFormat, ProvenanceManifest};
use crate::xci::mock::MockXciExtractor;
use crate::xci::types::{XciExtractRequest, XciExtractResult, XciExtractor, XciProgram};
use pathdiff::diff_paths;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

const INTAKE_SCHEMA_VERSION: &str = "1";
const MODULE_SCHEMA_VERSION: &str = "1";

#[derive(Debug)]
pub struct XciIntakeOptions {
    pub xci_path: PathBuf,
    pub keys_path: PathBuf,
    pub config_path: Option<PathBuf>,
    pub provenance_path: PathBuf,
    pub out_dir: PathBuf,
    pub assets_dir: PathBuf,
}

#[derive(Debug)]
pub struct XciIntakeReport {
    pub out_dir: PathBuf,
    pub assets_dir: PathBuf,
    pub module_json_path: PathBuf,
    pub manifest_path: PathBuf,
    pub files_written: Vec<PathBuf>,
}

#[derive(Debug, Deserialize, Default)]
struct RawXciConfig {
    #[serde(default)]
    program_title_id: Option<String>,
    #[serde(default)]
    program_version: Option<String>,
    #[serde(default)]
    program_content_type: Option<String>,
}

#[derive(Debug, Clone)]
struct XciSelection {
    title_id: Option<String>,
    version: Option<String>,
    content_type: Option<String>,
}

#[derive(Debug, serde::Serialize)]
struct IntakeManifest {
    schema_version: String,
    tool: ToolInfo,
    program: ProgramRecord,
    assets_root: String,
    modules: Vec<ModuleRecord>,
    assets: Vec<AssetRecord>,
    inputs: Vec<InputSummary>,
    generated_files: Vec<GeneratedFile>,
}

#[derive(Debug, serde::Serialize)]
struct ToolInfo {
    name: String,
    version: String,
}

#[derive(Debug, serde::Serialize)]
struct ProgramRecord {
    title_id: String,
    content_type: String,
    version: String,
    nca_sha256: String,
    nca_size: u64,
    nca_metadata_path: String,
}

#[derive(Debug, serde::Serialize)]
struct ModuleRecord {
    name: String,
    format: String,
    build_id: String,
    module_json_path: String,
}

#[derive(Debug, serde::Serialize, Clone)]
struct AssetRecord {
    kind: String,
    path: String,
    sha256: String,
    size: u64,
    source_offset: u64,
    source_size: u64,
}

pub fn intake_xci(options: XciIntakeOptions) -> Result<XciIntakeReport, String> {
    let extractor = MockXciExtractor::new();
    intake_xci_with_extractor(options, &extractor)
}

pub fn intake_xci_with_extractor(
    options: XciIntakeOptions,
    extractor: &dyn XciExtractor,
) -> Result<XciIntakeReport, String> {
    let xci_path = absolute_path(&options.xci_path)?;
    let keys_path = absolute_path(&options.keys_path)?;
    let provenance_path = absolute_path(&options.provenance_path)?;
    let out_dir = absolute_path(&options.out_dir)?;
    let assets_dir = absolute_path(&options.assets_dir)?;

    let config = match &options.config_path {
        Some(path) => {
            let config_path = absolute_path(path)?;
            let config_src = fs::read_to_string(&config_path)
                .map_err(|err| format!("read config {}: {err}", config_path.display()))?;
            parse_config(&config_src)?
        }
        None => RawXciConfig::default(),
    };

    ensure_separate_outputs(&out_dir, &assets_dir)?;

    let provenance_src =
        fs::read_to_string(&provenance_path).map_err(|err| format!("read provenance: {err}"))?;
    let provenance = ProvenanceManifest::parse(&provenance_src)?;
    let validation = provenance.validate(&provenance_path, &provenance_src)?;

    ensure_input_present(&validation.inputs, &xci_path, InputFormat::Xci)?;
    ensure_input_present(&validation.inputs, &keys_path, InputFormat::Keyset)?;

    let extract_request = XciExtractRequest {
        xci_path: xci_path.clone(),
        keys_path: keys_path.clone(),
    };
    let extraction = extractor.extract(&extract_request)?;
    let mut selection = XciSelection {
        title_id: config.program_title_id,
        version: config.program_version,
        content_type: config.program_content_type,
    };
    if selection.content_type.is_none() {
        selection.content_type = Some("program".to_string());
    }
    let program = select_program(&extraction, &selection)?;

    fs::create_dir_all(&out_dir)
        .map_err(|err| format!("create out dir {}: {err}", out_dir.display()))?;
    fs::create_dir_all(&assets_dir)
        .map_err(|err| format!("create assets dir {}: {err}", assets_dir.display()))?;

    let exefs_dir = out_dir.join("exefs");
    let segments_dir = out_dir.join("segments");
    let nca_dir = out_dir.join("nca");
    fs::create_dir_all(&exefs_dir).map_err(|err| format!("create exefs dir: {err}"))?;
    fs::create_dir_all(&segments_dir).map_err(|err| format!("create segments dir: {err}"))?;
    fs::create_dir_all(&nca_dir).map_err(|err| format!("create nca dir: {err}"))?;

    let mut generated_files = Vec::new();
    let mut files_written = Vec::new();

    let mut exefs_index = BTreeMap::new();
    for file in &program.exefs_files {
        let name = sanitize_name(&file.name)?;
        let out_path = exefs_dir.join(&name);
        fs::write(&out_path, &file.data).map_err(|err| format!("write exefs {name}: {err}"))?;
        files_written.push(out_path.clone());
        let rel_path = format!("exefs/{name}");
        generated_files.push(GeneratedFile {
            path: rel_path.clone(),
            sha256: sha256_bytes(&file.data),
            size: file.data.len() as u64,
        });
        exefs_index.insert(name, out_path);
    }

    let mut module_builds = Vec::new();
    let mut module_files = Vec::new();
    for nso in &program.nso_files {
        let name = sanitize_name(&nso.name)?;
        let Some(nso_path) = exefs_index.get(&name) else {
            return Err(format!("NSO {name} is not present in ExeFS output"));
        };
        let module = parse_nso(nso_path)?;
        let (build, generated, written) = write_nso_segments(&module, &segments_dir)?;
        module_builds.push(build);
        module_files.extend(generated);
        files_written.extend(written);
    }

    generated_files.extend(module_files);

    module_builds.sort_by(|a, b| a.name.cmp(&b.name));
    let module_json = ModuleJson {
        schema_version: MODULE_SCHEMA_VERSION.to_string(),
        module_type: "xci".to_string(),
        modules: module_builds,
    };
    let module_json_path = out_dir.join("module.json");
    let module_json_src =
        serde_json::to_string_pretty(&module_json).map_err(|err| err.to_string())?;
    fs::write(&module_json_path, module_json_src.as_bytes())
        .map_err(|err| format!("write module.json: {err}"))?;
    files_written.push(module_json_path.clone());
    generated_files.push(GeneratedFile {
        path: "module.json".to_string(),
        sha256: sha256_bytes(module_json_src.as_bytes()),
        size: module_json_src.len() as u64,
    });

    let nca_path = nca_dir.join("program.json");
    let nca_metadata = ProgramRecord {
        title_id: program.title_id.clone(),
        content_type: program.content_type.clone(),
        version: program.version.clone(),
        nca_sha256: sha256_bytes(&program.nca_bytes),
        nca_size: program.nca_bytes.len() as u64,
        nca_metadata_path: "nca/program.json".to_string(),
    };
    let nca_src = serde_json::to_string_pretty(&nca_metadata).map_err(|err| err.to_string())?;
    fs::write(&nca_path, nca_src.as_bytes()).map_err(|err| format!("write nca metadata: {err}"))?;
    files_written.push(nca_path);
    generated_files.push(GeneratedFile {
        path: "nca/program.json".to_string(),
        sha256: sha256_bytes(nca_src.as_bytes()),
        size: nca_src.len() as u64,
    });

    let mut assets = Vec::new();
    if let Some(romfs_image) = extraction.romfs_image.clone() {
        let romfs_root = assets_dir.join("romfs");
        fs::create_dir_all(&romfs_root).map_err(|err| format!("create romfs dir: {err}"))?;
        let entries = list_romfs_entries(&romfs_image)?;
        let asset_written = write_romfs_entries(
            &romfs_image,
            &entries,
            &romfs_root,
            &assets_dir,
            "romfs",
            &mut assets,
        )?;
        files_written.extend(asset_written);
    }

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

    let module_records = module_json
        .modules
        .iter()
        .map(|module| ModuleRecord {
            name: module.name.clone(),
            format: module.format.clone(),
            build_id: module.build_id.clone(),
            module_json_path: "module.json".to_string(),
        })
        .collect::<Vec<_>>();

    assets.sort_by(|a, b| a.path.cmp(&b.path));
    generated_files.sort_by(|a, b| a.path.cmp(&b.path));

    let assets_root = diff_paths(&assets_dir, &out_dir)
        .unwrap_or_else(|| assets_dir.clone())
        .to_string_lossy()
        .replace('\\', "/");

    let manifest = IntakeManifest {
        schema_version: INTAKE_SCHEMA_VERSION.to_string(),
        tool: ToolInfo {
            name: "recomp-pipeline".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
        program: nca_metadata,
        assets_root,
        modules: module_records,
        assets,
        inputs,
        generated_files,
    };

    let manifest_path = out_dir.join("manifest.json");
    let manifest_src = serde_json::to_string_pretty(&manifest).map_err(|err| err.to_string())?;
    fs::write(&manifest_path, manifest_src.as_bytes())
        .map_err(|err| format!("write manifest.json: {err}"))?;
    files_written.push(manifest_path.clone());

    Ok(XciIntakeReport {
        out_dir,
        assets_dir,
        module_json_path,
        manifest_path,
        files_written,
    })
}

fn parse_config(src: &str) -> Result<RawXciConfig, String> {
    toml::from_str(src).map_err(|err| format!("invalid xci intake config: {err}"))
}

fn select_program<'a>(
    extraction: &'a XciExtractResult,
    selection: &XciSelection,
) -> Result<&'a XciProgram, String> {
    let mut candidates = Vec::new();
    for program in &extraction.programs {
        if let Some(title_id) = &selection.title_id {
            if &program.title_id != title_id {
                continue;
            }
        }
        if let Some(version) = &selection.version {
            if &program.version != version {
                continue;
            }
        }
        if let Some(content_type) = &selection.content_type {
            if &program.content_type != content_type {
                continue;
            }
        }
        candidates.push(program);
    }

    if candidates.is_empty() {
        return Err(format!(
            "no Program NCA matches selection. available: {}",
            format_programs(extraction.programs.iter())
        ));
    }
    if candidates.len() > 1 {
        return Err(format!(
            "ambiguous Program NCA selection. Provide program_title_id/program_version to disambiguate. available: {}",
            format_programs(candidates.iter().copied())
        ));
    }

    Ok(candidates[0])
}

fn format_programs<'a>(programs: impl IntoIterator<Item = &'a XciProgram>) -> String {
    let mut out = Vec::new();
    for program in programs {
        out.push(format!(
            "{} {} {}",
            program.title_id, program.content_type, program.version
        ));
    }
    out.join(", ")
}

fn ensure_input_present(
    inputs: &[crate::provenance::ValidatedInput],
    path: &Path,
    format: InputFormat,
) -> Result<(), String> {
    if inputs
        .iter()
        .any(|input| input.path == path && input.format == format)
    {
        Ok(())
    } else {
        Err(format!(
            "input {} with format {} not listed in provenance metadata",
            path.display(),
            format.as_str()
        ))
    }
}

fn ensure_separate_outputs(out_dir: &Path, assets_dir: &Path) -> Result<(), String> {
    let normalized_out = normalize_path(out_dir);
    let normalized_assets = normalize_path(assets_dir);
    if normalized_out == normalized_assets {
        return Err("assets_dir must be separate from out_dir".to_string());
    }
    if is_within(&normalized_assets, &normalized_out) {
        return Err("assets_dir must not be inside out_dir".to_string());
    }
    if is_within(&normalized_out, &normalized_assets) {
        return Err("out_dir must not be inside assets_dir".to_string());
    }
    Ok(())
}

fn is_within(path: &Path, base: &Path) -> bool {
    path.starts_with(base)
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            Component::Prefix(prefix) => out.push(prefix.as_os_str()),
            Component::RootDir => out.push(Component::RootDir.as_os_str()),
            Component::Normal(value) => out.push(value),
        }
    }
    out
}

fn write_nso_segments(
    module: &NsoModule,
    segments_dir: &Path,
) -> Result<(ModuleBuild, Vec<GeneratedFile>, Vec<PathBuf>), String> {
    let module_name = module
        .path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("nso")
        .to_string();
    let module_dir = segments_dir.join(&module_name);
    fs::create_dir_all(&module_dir).map_err(|err| format!("create module dir: {err}"))?;

    let segment_data = extract_segments(module)?;
    let mut segments = Vec::new();
    let mut generated = Vec::new();
    let mut written = Vec::new();

    for entry in segment_data {
        let file_name = format!("{}.bin", segment_name(entry.segment.kind));
        let output_rel = format!("segments/{module_name}/{file_name}");
        let output_path = module_dir.join(&file_name);
        fs::write(&output_path, &entry.data)
            .map_err(|err| format!("write NSO segment {file_name}: {err}"))?;
        written.push(output_path.clone());
        generated.push(GeneratedFile {
            path: output_rel.clone(),
            sha256: sha256_bytes(&entry.data),
            size: entry.data.len() as u64,
        });
        segments.push(ModuleSegment {
            name: segment_name(entry.segment.kind).to_string(),
            file_offset: entry.segment.file_offset as u64,
            file_size: entry.segment.file_size as u64,
            memory_offset: entry.segment.memory_offset as u64,
            memory_size: entry.segment.size as u64,
            permissions: entry.segment.permissions.as_str().to_string(),
            compressed: Some(entry.segment.compressed),
            output_path: output_rel,
        });
    }

    let input_sha256 = sha256_path(&module.path)?;
    let bss_offset = module
        .segments
        .iter()
        .find(|segment| segment.kind == NsoSegmentKind::Data)
        .map(|segment| segment.memory_offset as u64 + segment.size as u64)
        .unwrap_or(0);

    let input_name = module
        .path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("nso");
    let build = ModuleBuild {
        name: module_name,
        format: "nso".to_string(),
        input_path: PathBuf::from(format!("exefs/{input_name}")),
        input_sha256,
        input_size: module.size,
        build_id: module.module_id_hex(),
        segments,
        bss: BssInfo {
            size: module.bss_size as u64,
            memory_offset: bss_offset,
        },
        embedded: Some(OffsetInfo {
            offset: module.embedded_offset as u64,
            size: module.embedded_size as u64,
        }),
        dynstr: Some(OffsetInfo {
            offset: module.dynstr_offset as u64,
            size: module.dynstr_size as u64,
        }),
        dynsym: Some(OffsetInfo {
            offset: module.dynsym_offset as u64,
            size: module.dynsym_size as u64,
        }),
    };

    Ok((build, generated, written))
}

fn segment_name(kind: NsoSegmentKind) -> &'static str {
    match kind {
        NsoSegmentKind::Text => "text",
        NsoSegmentKind::Rodata => "rodata",
        NsoSegmentKind::Data => "data",
    }
}

fn write_romfs_entries(
    romfs_bytes: &[u8],
    entries: &[RomfsEntry],
    romfs_dir: &Path,
    root_dir: &Path,
    kind: &str,
    records: &mut Vec<AssetRecord>,
) -> Result<Vec<PathBuf>, String> {
    let mut written = Vec::new();
    for entry in entries {
        let rel_path = Path::new(&entry.path);
        if rel_path.is_absolute() {
            return Err(format!("romfs entry path is absolute: {}", entry.path));
        }
        for component in rel_path.components() {
            match component {
                std::path::Component::Normal(_) => {}
                _ => {
                    return Err(format!(
                        "romfs entry path contains invalid component: {}",
                        entry.path
                    ))
                }
            }
        }

        let out_path = romfs_dir.join(rel_path);
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("create romfs dir {}: {err}", parent.display()))?;
        }

        let start = entry.data_offset as usize;
        let end = start
            .checked_add(entry.data_size as usize)
            .ok_or_else(|| "romfs file size overflow".to_string())?;
        if end > romfs_bytes.len() {
            return Err(format!(
                "romfs entry out of range: {}..{} (len={})",
                start,
                end,
                romfs_bytes.len()
            ));
        }
        let data = &romfs_bytes[start..end];
        fs::write(&out_path, data)
            .map_err(|err| format!("write romfs entry {}: {err}", out_path.display()))?;

        let rel = out_path
            .strip_prefix(root_dir)
            .unwrap_or(&out_path)
            .to_string_lossy()
            .replace('\\', "/");
        let record = AssetRecord {
            kind: kind.to_string(),
            path: rel,
            sha256: sha256_bytes(data),
            size: data.len() as u64,
            source_offset: entry.data_offset,
            source_size: entry.data_size,
        };
        records.push(record);
        written.push(out_path);
    }

    Ok(written)
}

fn sanitize_name(name: &str) -> Result<String, String> {
    if name.is_empty() {
        return Err("empty file name in ExeFS".to_string());
    }
    let path = Path::new(name);
    if path.components().count() != 1 {
        return Err(format!("ExeFS file name has path separators: {name}"));
    }
    Ok(name.to_string())
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

fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    hex_bytes(&digest)
}

fn sha256_path(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|err| err.to_string())?;
    Ok(sha256_bytes(&bytes))
}

fn hex_bytes(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write;
        let _ = write!(&mut out, "{byte:02x}");
    }
    out
}
