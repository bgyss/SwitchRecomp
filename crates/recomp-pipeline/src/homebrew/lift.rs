use crate::homebrew::module::{ModuleJson, MODULE_SCHEMA_VERSION};
use crate::input::{Function, Module, Op};
use serde_json;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct LiftOptions {
    pub module_json_path: PathBuf,
    pub out_dir: PathBuf,
    pub entry_name: String,
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

    let mut verified_segments = 0_u64;
    for module in &module_json.modules {
        for segment in &module.segments {
            let segment_path = resolve_segment_path(base_dir, &segment.output_path);
            if !segment_path.exists() {
                return Err(format!(
                    "segment file not found: {}",
                    segment_path.display()
                ));
            }
            verified_segments += 1;
        }
    }
    if verified_segments == 0 {
        warnings.push("homebrew module contains no segments".to_string());
    }

    ensure_dir(&out_dir)?;

    let lifted = Module {
        arch: "aarch64".to_string(),
        functions: vec![Function {
            name: options.entry_name,
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

fn resolve_segment_path(base_dir: &Path, output_path: &str) -> PathBuf {
    let path = Path::new(output_path);
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
