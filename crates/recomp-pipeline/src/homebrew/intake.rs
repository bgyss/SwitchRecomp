use crate::homebrew::module::{
    BssInfo, ModuleBuild, ModuleJson, ModuleSegment, OffsetInfo, MODULE_SCHEMA_VERSION,
};
use crate::homebrew::nro::{parse_nro, NroModule};
use crate::homebrew::nso::{extract_segments, parse_nso, NsoModule, NsoSegmentKind};
use crate::homebrew::romfs::{list_romfs_entries, RomfsEntry};
use crate::homebrew::util::hex_bytes;
use crate::output::{GeneratedFile, InputSummary};
use crate::provenance::{InputFormat, ProvenanceManifest};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const INTAKE_SCHEMA_VERSION: &str = "1";

#[derive(Debug)]
pub struct IntakeOptions {
    pub module_path: PathBuf,
    pub nso_paths: Vec<PathBuf>,
    pub provenance_path: PathBuf,
    pub out_dir: PathBuf,
}

#[derive(Debug)]
pub struct IntakeReport {
    pub out_dir: PathBuf,
    pub module_json_path: PathBuf,
    pub manifest_path: PathBuf,
    pub files_written: Vec<PathBuf>,
}

#[derive(Debug, serde::Serialize)]
struct IntakeManifest {
    schema_version: String,
    tool: ToolInfo,
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

pub fn intake_homebrew(options: IntakeOptions) -> Result<IntakeReport, String> {
    let module_path = absolute_path(&options.module_path)?;
    let nso_paths = options
        .nso_paths
        .iter()
        .map(|path| absolute_path(path))
        .collect::<Result<Vec<_>, _>>()?;
    let provenance_path = absolute_path(&options.provenance_path)?;
    let out_dir = absolute_path(&options.out_dir)?;

    let provenance_src =
        fs::read_to_string(&provenance_path).map_err(|err| format!("read provenance: {err}"))?;
    let provenance = ProvenanceManifest::parse(&provenance_src)?;
    let validation = provenance.validate(&provenance_path, &provenance_src)?;

    enforce_homebrew_formats(&validation.inputs)?;
    ensure_input_present(&validation.inputs, &module_path)?;
    for nso_path in &nso_paths {
        ensure_input_present(&validation.inputs, nso_path)?;
    }

    let nro = parse_nro(&module_path)?;
    let nso_modules = nso_paths
        .iter()
        .map(|path| parse_nso(path))
        .collect::<Result<Vec<_>, _>>()?;

    fs::create_dir_all(&out_dir)
        .map_err(|err| format!("create out dir {}: {err}", out_dir.display()))?;
    let segments_dir = out_dir.join("segments");
    let assets_dir = out_dir.join("assets");
    fs::create_dir_all(&segments_dir).map_err(|err| format!("create segments dir: {err}"))?;
    fs::create_dir_all(&assets_dir).map_err(|err| format!("create assets dir: {err}"))?;

    let (nro_build, mut generated_files, mut files_written) =
        write_nro_segments(&nro, &segments_dir)?;
    let mut module_builds = vec![nro_build];

    for nso in &nso_modules {
        let (build, segment_files, segment_written) = write_nso_segments(nso, &segments_dir)?;
        module_builds.push(build);
        generated_files.extend(segment_files);
        files_written.extend(segment_written);
    }

    let mut assets = Vec::new();
    let (asset_files, asset_written) = extract_assets(&nro, &out_dir, &assets_dir, &mut assets)?;
    generated_files.extend(asset_files);
    files_written.extend(asset_written);

    module_builds.sort_by(|a, b| a.name.cmp(&b.name));
    let module_json = ModuleJson {
        schema_version: MODULE_SCHEMA_VERSION.to_string(),
        module_type: "homebrew".to_string(),
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

    let manifest = IntakeManifest {
        schema_version: INTAKE_SCHEMA_VERSION.to_string(),
        tool: ToolInfo {
            name: "recomp-pipeline".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
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

    Ok(IntakeReport {
        out_dir,
        module_json_path,
        manifest_path,
        files_written,
    })
}

fn write_nro_segments(
    module: &NroModule,
    segments_dir: &Path,
) -> Result<(ModuleBuild, Vec<GeneratedFile>, Vec<PathBuf>), String> {
    let module_name = module
        .path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("nro")
        .to_string();
    let module_dir = segments_dir.join(&module_name);
    fs::create_dir_all(&module_dir).map_err(|err| format!("create module dir: {err}"))?;

    let bytes = fs::read(&module.path)
        .map_err(|err| format!("read NRO {}: {err}", module.path.display()))?;
    let mut segments = Vec::new();
    let mut generated = Vec::new();
    let mut written = Vec::new();

    for segment in &module.segments {
        let start = segment.file_offset as usize;
        let end = start
            .checked_add(segment.size as usize)
            .ok_or_else(|| "segment offset overflow".to_string())?;
        if end > bytes.len() {
            return Err(format!("NRO segment out of range: {}..{}", start, end));
        }
        let data = &bytes[start..end];
        let file_name = format!("{}.bin", segment.name);
        let output_rel = format!("segments/{module_name}/{file_name}");
        let output_path = module_dir.join(&file_name);
        fs::write(&output_path, data).map_err(|err| format!("write segment {file_name}: {err}"))?;
        written.push(output_path.clone());
        generated.push(GeneratedFile {
            path: output_rel.clone(),
            sha256: sha256_bytes(data),
            size: data.len() as u64,
        });
        segments.push(ModuleSegment {
            name: segment.name.clone(),
            file_offset: segment.file_offset as u64,
            file_size: segment.size as u64,
            memory_offset: segment.memory_offset as u64,
            memory_size: segment.size as u64,
            permissions: segment.permissions.as_str().to_string(),
            compressed: None,
            output_path: output_rel,
        });
    }

    let input_sha256 = sha256_path(&module.path)?;
    let input_size = bytes.len() as u64;
    let bss_offset = module
        .segments
        .iter()
        .find(|segment| segment.name == "data")
        .map(|segment| segment.memory_offset as u64 + segment.size as u64)
        .unwrap_or(0);

    let build = ModuleBuild {
        name: module_name,
        format: "nro".to_string(),
        input_path: module.path.clone(),
        input_sha256,
        input_size,
        build_id: module.build_id_hex(),
        segments,
        bss: BssInfo {
            size: module.bss_size as u64,
            memory_offset: bss_offset,
        },
        embedded: None,
        dynstr: None,
        dynsym: None,
    };

    Ok((build, generated, written))
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

    let build = ModuleBuild {
        name: module_name,
        format: "nso".to_string(),
        input_path: module.path.clone(),
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

fn extract_assets(
    module: &NroModule,
    root_dir: &Path,
    assets_dir: &Path,
    records: &mut Vec<AssetRecord>,
) -> Result<(Vec<GeneratedFile>, Vec<PathBuf>), String> {
    let Some(assets) = module.assets.clone() else {
        return Ok((Vec::new(), Vec::new()));
    };
    let bytes = fs::read(&module.path)
        .map_err(|err| format!("read NRO {}: {err}", module.path.display()))?;
    let mut generated = Vec::new();
    let mut written = Vec::new();

    if assets.icon.size > 0 {
        let (path, info) = extract_asset(
            &bytes,
            &assets,
            assets.icon,
            root_dir,
            assets_dir,
            "icon.bin",
            "icon",
        )?;
        records.push(info);
        generated.push(path.generated_file);
        written.push(path.path);
    }

    if assets.nacp.size > 0 {
        let (path, info) = extract_asset(
            &bytes,
            &assets,
            assets.nacp,
            root_dir,
            assets_dir,
            "control.nacp",
            "nacp",
        )?;
        if info.size != 0x4000 {
            return Err(format!(
                "NACP size mismatch: expected 0x4000, got {}",
                info.size
            ));
        }
        records.push(info);
        generated.push(path.generated_file);
        written.push(path.path);
    }

    if assets.romfs.size > 0 {
        let romfs_dir = assets_dir.join("romfs");
        fs::create_dir_all(&romfs_dir).map_err(|err| format!("create romfs dir: {err}"))?;
        let (romfs_bytes, romfs_base_offset) = extract_asset_bytes(&bytes, &assets, assets.romfs)?;
        let entries = list_romfs_entries(&romfs_bytes)?;
        let (generated_entries, written_entries) = write_romfs_entries(
            &romfs_bytes,
            &entries,
            romfs_base_offset,
            root_dir,
            &romfs_dir,
            "romfs",
            records,
        )?;
        generated.extend(generated_entries);
        written.extend(written_entries);
    }

    Ok((generated, written))
}

struct AssetWrite {
    path: PathBuf,
    generated_file: GeneratedFile,
}

fn extract_asset(
    bytes: &[u8],
    assets: &crate::homebrew::nro::NroAssetHeader,
    section: crate::homebrew::nro::NroAssetSection,
    root_dir: &Path,
    out_dir: &Path,
    file_name: &str,
    kind: &str,
) -> Result<(AssetWrite, AssetRecord), String> {
    let start = assets
        .base_offset
        .checked_add(section.offset)
        .ok_or_else(|| "asset offset overflow".to_string())? as usize;
    let end = start
        .checked_add(section.size as usize)
        .ok_or_else(|| "asset size overflow".to_string())?;
    if end > bytes.len() {
        return Err(format!(
            "asset out of range: {}..{} (len={})",
            start,
            end,
            bytes.len()
        ));
    }
    let data = &bytes[start..end];
    let out_path = out_dir.join(file_name);
    fs::write(&out_path, data).map_err(|err| format!("write asset {file_name}: {err}"))?;

    let rel = out_path
        .strip_prefix(root_dir)
        .unwrap_or(&out_path)
        .to_string_lossy()
        .replace('\\', "/");

    let generated_file = GeneratedFile {
        path: rel.clone(),
        sha256: sha256_bytes(data),
        size: data.len() as u64,
    };
    let record = AssetRecord {
        kind: kind.to_string(),
        path: rel,
        sha256: generated_file.sha256.clone(),
        size: generated_file.size,
        source_offset: section.offset,
        source_size: section.size,
    };

    Ok((
        AssetWrite {
            path: out_path,
            generated_file,
        },
        record,
    ))
}

fn extract_asset_bytes(
    bytes: &[u8],
    assets: &crate::homebrew::nro::NroAssetHeader,
    section: crate::homebrew::nro::NroAssetSection,
) -> Result<(Vec<u8>, u64), String> {
    let start = assets
        .base_offset
        .checked_add(section.offset)
        .ok_or_else(|| "asset offset overflow".to_string())? as usize;
    let end = start
        .checked_add(section.size as usize)
        .ok_or_else(|| "asset size overflow".to_string())?;
    if end > bytes.len() {
        return Err(format!(
            "asset out of range: {}..{} (len={})",
            start,
            end,
            bytes.len()
        ));
    }
    Ok((bytes[start..end].to_vec(), start as u64))
}

fn write_romfs_entries(
    romfs_bytes: &[u8],
    entries: &[RomfsEntry],
    romfs_base_offset: u64,
    root_dir: &Path,
    romfs_dir: &Path,
    kind: &str,
    records: &mut Vec<AssetRecord>,
) -> Result<(Vec<GeneratedFile>, Vec<PathBuf>), String> {
    let mut generated = Vec::new();
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
        let generated_file = GeneratedFile {
            path: rel.clone(),
            sha256: sha256_bytes(data),
            size: data.len() as u64,
        };
        let record = AssetRecord {
            kind: kind.to_string(),
            path: rel,
            sha256: generated_file.sha256.clone(),
            size: generated_file.size,
            source_offset: romfs_base_offset
                .checked_add(entry.data_offset)
                .ok_or_else(|| "romfs source offset overflow".to_string())?,
            source_size: entry.data_size,
        };
        records.push(record);
        generated.push(generated_file);
        written.push(out_path);
    }

    Ok((generated, written))
}

fn ensure_input_present(
    inputs: &[crate::provenance::ValidatedInput],
    path: &Path,
) -> Result<(), String> {
    if inputs.iter().any(|input| input.path == path) {
        Ok(())
    } else {
        Err(format!(
            "input {} not listed in provenance metadata",
            path.display()
        ))
    }
}

fn enforce_homebrew_formats(inputs: &[crate::provenance::ValidatedInput]) -> Result<(), String> {
    let mut disallowed = BTreeMap::new();
    for input in inputs {
        match input.format {
            InputFormat::Nro0 | InputFormat::Nso0 => {}
            other => {
                disallowed
                    .entry(other.as_str())
                    .or_insert_with(Vec::new)
                    .push(input.path.clone());
            }
        }
    }
    if disallowed.is_empty() {
        return Ok(());
    }
    let mut message = String::from("disallowed input formats for homebrew intake:\n");
    for (format, paths) in disallowed {
        for path in paths {
            message.push_str(&format!("- {format}: {}\n", path.display()));
        }
    }
    Err(message)
}

fn segment_name(kind: NsoSegmentKind) -> &'static str {
    match kind {
        NsoSegmentKind::Text => "text",
        NsoSegmentKind::Rodata => "rodata",
        NsoSegmentKind::Data => "data",
    }
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
