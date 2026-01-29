use crate::pipeline::{ensure_dir, RustFunction, RustProgram};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
pub struct BuildManifest {
    pub title: String,
    pub abi_version: String,
    pub module_sha256: String,
    pub config_sha256: String,
    pub generated_files: Vec<String>,
}

pub fn emit_project(
    out_dir: &Path,
    runtime_rel: &Path,
    program: &RustProgram,
    manifest: &BuildManifest,
) -> Result<Vec<PathBuf>, String> {
    ensure_dir(out_dir).map_err(|err| err.to_string())?;

    let mut written = Vec::new();
    let cargo_toml = emit_cargo_toml(program, runtime_rel);
    let cargo_path = out_dir.join("Cargo.toml");
    fs::write(&cargo_path, cargo_toml).map_err(|err| err.to_string())?;
    written.push(cargo_path);

    let src_dir = out_dir.join("src");
    ensure_dir(&src_dir).map_err(|err| err.to_string())?;
    let main_rs = emit_main_rs(program);
    let main_path = src_dir.join("main.rs");
    fs::write(&main_path, main_rs).map_err(|err| err.to_string())?;
    written.push(main_path);

    let manifest_path = out_dir.join("manifest.json");
    let manifest_json =
        serde_json::to_string_pretty(manifest).map_err(|err| err.to_string())?;
    fs::write(&manifest_path, manifest_json).map_err(|err| err.to_string())?;
    written.push(manifest_path);

    Ok(written)
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
    out.push_str("    recomp_runtime::init();\n");
    out.push_str(&format!("    {}()?;\n", program.entry));
    out.push_str("    Ok(())\n}
\n");

    for function in &program.functions {
        out.push_str(&emit_function(function));
        out.push_str("\n");
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
        out.push_str("\n");
    }
    for line in &function.lines {
        out.push_str("    ");
        out.push_str(line);
        out.push_str("\n");
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
