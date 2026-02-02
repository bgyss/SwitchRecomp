use crate::pipeline::{ensure_dir, FunctionBody, RustFunction, RustProgram, RustTerminator};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

const BUILD_MANIFEST_SELF_PATH: &str = "manifest.json";
const BUILD_MANIFEST_SELF_SHA_PLACEHOLDER: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Debug, Serialize, Clone)]
pub struct BuildManifest {
    pub title: String,
    pub abi_version: String,
    pub module_sha256: String,
    pub config_sha256: String,
    pub provenance_sha256: String,
    pub inputs: Vec<InputSummary>,
    pub manifest_self_hash_basis: String,
    pub generated_files: Vec<GeneratedFile>,
}

#[derive(Debug, Serialize, Clone)]
pub struct InputSummary {
    pub path: PathBuf,
    pub format: String,
    pub sha256: String,
    pub size: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct GeneratedFile {
    pub path: String,
    pub sha256: String,
    pub size: u64,
}

pub fn emit_project(
    out_dir: &Path,
    runtime_rel: &Path,
    program: &RustProgram,
    manifest: &BuildManifest,
) -> Result<(Vec<PathBuf>, BuildManifest), String> {
    ensure_dir(out_dir).map_err(|err| err.to_string())?;

    let mut written = Vec::new();
    let mut generated_files = Vec::new();
    let cargo_toml = emit_cargo_toml(program, runtime_rel);
    let cargo_path = out_dir.join("Cargo.toml");
    fs::write(&cargo_path, &cargo_toml).map_err(|err| err.to_string())?;
    written.push(cargo_path);
    generated_files.push(GeneratedFile {
        path: "Cargo.toml".to_string(),
        sha256: sha256_bytes(cargo_toml.as_bytes()),
        size: cargo_toml.len() as u64,
    });

    let src_dir = out_dir.join("src");
    ensure_dir(&src_dir).map_err(|err| err.to_string())?;
    let main_rs = emit_main_rs(program);
    let main_path = src_dir.join("main.rs");
    fs::write(&main_path, &main_rs).map_err(|err| err.to_string())?;
    written.push(main_path);
    generated_files.push(GeneratedFile {
        path: "src/main.rs".to_string(),
        sha256: sha256_bytes(main_rs.as_bytes()),
        size: main_rs.len() as u64,
    });

    let manifest_path = out_dir.join("manifest.json");
    let mut updated_manifest = manifest.clone();
    updated_manifest.generated_files = generated_files;
    let (updated_manifest, manifest_json) = build_manifest_json(updated_manifest)?;
    fs::write(&manifest_path, manifest_json).map_err(|err| err.to_string())?;
    written.push(manifest_path);

    Ok((written, updated_manifest))
}

fn build_manifest_json(mut manifest: BuildManifest) -> Result<(BuildManifest, String), String> {
    if manifest
        .generated_files
        .iter()
        .any(|file| file.path == BUILD_MANIFEST_SELF_PATH)
    {
        return Err("build manifest already present in generated files".to_string());
    }

    manifest.manifest_self_hash_basis = "generated_files_self_placeholder".to_string();
    manifest.generated_files.push(GeneratedFile {
        path: BUILD_MANIFEST_SELF_PATH.to_string(),
        sha256: BUILD_MANIFEST_SELF_SHA_PLACEHOLDER.to_string(),
        size: 0,
    });
    manifest.generated_files.sort_by(|a, b| a.path.cmp(&b.path));

    let size = stabilize_manifest_size(&mut manifest)?;
    let self_hash = manifest_self_hash(&manifest)?;
    let self_entry = find_manifest_self_entry_mut(&mut manifest)?;
    self_entry.size = size;
    self_entry.sha256 = self_hash;

    let manifest_json = serde_json::to_string_pretty(&manifest).map_err(|err| err.to_string())?;
    let final_size = manifest_json.len() as u64;
    if final_size != size {
        return Err(format!(
            "build manifest size mismatch: expected {size}, got {final_size}"
        ));
    }

    Ok((manifest, manifest_json))
}

fn manifest_self_hash(manifest: &BuildManifest) -> Result<String, String> {
    let mut normalized = manifest.clone();
    let self_entry = find_manifest_self_entry_mut(&mut normalized)?;
    self_entry.sha256 = BUILD_MANIFEST_SELF_SHA_PLACEHOLDER.to_string();
    let manifest_json = serde_json::to_string_pretty(&normalized).map_err(|err| err.to_string())?;
    Ok(sha256_bytes(manifest_json.as_bytes()))
}

fn stabilize_manifest_size(manifest: &mut BuildManifest) -> Result<u64, String> {
    let mut size = 0_u64;
    for _ in 0..5 {
        let self_entry = find_manifest_self_entry_mut(manifest)?;
        self_entry.sha256 = BUILD_MANIFEST_SELF_SHA_PLACEHOLDER.to_string();
        self_entry.size = size;
        let manifest_json =
            serde_json::to_string_pretty(manifest).map_err(|err| err.to_string())?;
        let new_size = manifest_json.len() as u64;
        if new_size == size {
            return Ok(size);
        }
        size = new_size;
    }
    Err("build manifest size did not stabilize".to_string())
}

fn find_manifest_self_entry_mut(
    manifest: &mut BuildManifest,
) -> Result<&mut GeneratedFile, String> {
    manifest
        .generated_files
        .iter_mut()
        .find(|entry| entry.path == BUILD_MANIFEST_SELF_PATH)
        .ok_or_else(|| "build manifest self entry not found".to_string())
}

fn emit_cargo_toml(program: &RustProgram, runtime_rel: &Path) -> String {
    format!(
        "[package]\nname = \"recomp-{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nrecomp-runtime = {{ path = \"{runtime}\" }}\n",
        name = sanitize_name(&program.title),
        runtime = runtime_rel.display()
    )
}

fn emit_main_rs(program: &RustProgram) -> String {
    let mut out = String::new();
    out.push_str("use recomp_runtime;\n\n");
    out.push_str("fn main() -> Result<(), recomp_runtime::RuntimeError> {\n");
    out.push_str(&format!(
        "    println!(\"recomp target: {}\");\n",
        program.title
    ));
    out.push_str(&format!(
        "    println!(\"abi version: {}\");\n",
        program.abi_version
    ));
    out.push_str("    let runtime_config = recomp_runtime::RuntimeConfig::new(");
    out.push_str(match program.performance_mode {
        crate::config::PerformanceMode::Handheld => "recomp_runtime::PerformanceMode::Handheld",
        crate::config::PerformanceMode::Docked => "recomp_runtime::PerformanceMode::Docked",
    });
    out.push_str(");\n");
    out.push_str("    recomp_runtime::init(&runtime_config);\n");
    out.push_str(&format!("    {}()?;\n", program.entry));
    out.push_str(
        "    Ok(())\n}
\n",
    );

    for function in &program.functions {
        out.push_str(&emit_function(function));
        out.push('\n');
    }

    out
}

fn emit_function(function: &RustFunction) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "fn {}() -> Result<(), recomp_runtime::RuntimeError> {{\n",
        function.name
    ));
    for reg in &function.regs {
        out.push_str(&format!("    let mut {reg}: i64 = 0;\n"));
    }
    if function.needs_flags {
        out.push_str("    let mut flag_n = false;\n");
        out.push_str("    let mut flag_z = false;\n");
        out.push_str("    let mut flag_c = false;\n");
        out.push_str("    let mut flag_v = false;\n");
    }
    if !function.regs.is_empty() || function.needs_flags {
        out.push('\n');
    }
    match &function.body {
        FunctionBody::Linear(lines) => {
            for line in lines {
                out.push_str("    ");
                out.push_str(line);
                out.push('\n');
            }
            if lines
                .last()
                .map(|line| !line.trim_start().starts_with("return"))
                .unwrap_or(true)
            {
                out.push_str("    Ok(())\n");
            }
        }
        FunctionBody::Blocks(blocks) => {
            let entry = blocks
                .first()
                .map(|block| block.label.as_str())
                .unwrap_or("entry");
            out.push_str(&format!("    let mut block_label = \"{entry}\";\n"));
            out.push_str("    loop {\n");
            out.push_str("        match block_label {\n");
            for block in blocks {
                out.push_str(&format!("            \"{}\" => {{\n", block.label));
                for line in &block.lines {
                    out.push_str("                ");
                    out.push_str(line);
                    out.push('\n');
                }
                emit_block_terminator(&mut out, &block.terminator);
                out.push_str("            },\n");
            }
            out.push_str("            _ => {\n");
            out.push_str("                panic!(\"unknown block label: {}\", block_label);\n");
            out.push_str("            }\n");
            out.push_str("        }\n");
            out.push_str("    }\n");
        }
    }
    out.push_str("}\n");
    out
}

fn emit_block_terminator(out: &mut String, terminator: &RustTerminator) {
    match terminator {
        RustTerminator::Br { target } => {
            out.push_str(&format!("                block_label = \"{target}\";\n"));
            out.push_str("                continue;\n");
        }
        RustTerminator::BrCond {
            cond_expr,
            cond,
            then_label,
            else_label,
        } => {
            if let Some(expr) = cond_expr {
                out.push_str(&format!("                if {expr} {{\n"));
                out.push_str(&format!(
                    "                    block_label = \"{then_label}\";\n"
                ));
                out.push_str("                } else {\n");
                out.push_str(&format!(
                    "                    block_label = \"{else_label}\";\n"
                ));
                out.push_str("                }\n");
                out.push_str("                continue;\n");
            } else {
                out.push_str(&format!(
                    "                panic!(\"unsupported condition: {cond}\");\n"
                ));
            }
        }
        RustTerminator::Call { call_line, next } => {
            out.push_str("                ");
            out.push_str(call_line);
            out.push('\n');
            out.push_str(&format!("                block_label = \"{next}\";\n"));
            out.push_str("                continue;\n");
        }
        RustTerminator::BrIndirect { reg } => {
            out.push_str(&format!(
                "                panic!(\"indirect branch via {reg} is not supported\");\n"
            ));
        }
        RustTerminator::Ret => {
            out.push_str("                return Ok(());\n");
        }
    }
}

fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|ch| match ch {
            'a'..='z' | '0'..='9' => ch,
            'A'..='Z' => ch.to_ascii_lowercase(),
            _ => '-',
        })
        .collect()
}

fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    format!("{:x}", digest)
}
