use crate::pipeline::{ensure_dir, RustFunction, RustProgram};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Clone)]
pub struct BuildManifest {
    pub title: String,
    pub abi_version: String,
    pub module_sha256: String,
    pub config_sha256: String,
    pub provenance_sha256: String,
    pub inputs: Vec<InputSummary>,
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
    let manifest_json =
        serde_json::to_string_pretty(&updated_manifest).map_err(|err| err.to_string())?;
    fs::write(&manifest_path, manifest_json).map_err(|err| err.to_string())?;
    written.push(manifest_path);

    Ok((written, updated_manifest))
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
    if !function.regs.is_empty() {
        out.push('\n');
    }
    for line in &function.lines {
        out.push_str("    ");
        out.push_str(line);
        out.push('\n');
    }
    if function
        .lines
        .last()
        .map(|line| !line.trim_start().starts_with("return"))
        .unwrap_or(true)
    {
        out.push_str("    Ok(())\n");
    }
    out.push_str("}\n");
    out
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
