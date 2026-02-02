use recomp_pipeline::config::TitleConfig;
use recomp_pipeline::input::Module;
use recomp_pipeline::{run_pipeline, PipelineOptions};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;

const MODULE_JSON: &str = r#"{
  "arch": "aarch64",
  "functions": [
    {
      "name": "entry",
      "ops": [
        { "op": "const_i64", "dst": "x0", "imm": 7 },
        { "op": "const_i64", "dst": "x1", "imm": 35 },
        { "op": "add_i64", "dst": "x2", "lhs": "x0", "rhs": "x1" },
        { "op": "syscall", "name": "svc_log", "args": ["x2"] },
        { "op": "ret" }
      ]
    }
  ]
}"#;

const CONFIG_TOML: &str = r#"
title = "Minimal Sample"
entry = "entry"
abi_version = "0.1.0"

[stubs]
svc_log = "log"
"#;

#[test]
fn parse_title_config() {
    let config = TitleConfig::parse(CONFIG_TOML).expect("config parses");
    assert_eq!(config.title, "Minimal Sample");
    assert_eq!(config.entry, "entry");
    assert_eq!(config.abi_version, "0.1.0");
    assert!(config.stubs.contains_key("svc_log"));
}

#[test]
fn module_validation_rejects_unknown_arch() {
    let module: Module =
        serde_json::from_str(r#"{"arch":"mips","functions":[]}"#).expect("module parses");
    let err = module.validate_arch().unwrap_err();
    assert!(err.contains("unsupported arch"));
}

#[test]
fn pipeline_emits_project() {
    let temp = tempfile::tempdir().expect("tempdir");
    let module_path = temp.path().join("module.json");
    let config_path = temp.path().join("title.toml");
    let provenance_path = temp.path().join("provenance.toml");
    let nso_path = temp.path().join("main.nso");
    let out_dir = temp.path().join("out");
    let runtime_path = PathBuf::from("../crates/recomp-runtime");

    fs::write(&module_path, MODULE_JSON).expect("write module");
    fs::write(&config_path, CONFIG_TOML).expect("write config");
    fs::write(&nso_path, b"NSO0").expect("write nso");

    let module_hash = sha256_hex(MODULE_JSON.as_bytes());
    let nso_hash = sha256_hex(b"NSO0");
    let provenance = format!(
        "schema_version = \"1\"\n\n[title]\nname = \"Minimal Sample\"\ntitle_id = \"0100000000000000\"\nversion = \"1.0.0\"\nregion = \"US\"\n\n[collection]\ndevice = \"demo\"\ncollected_at = \"2026-01-30\"\n\n[collection.tool]\nname = \"manual\"\nversion = \"1.0\"\n\n[[inputs]]\npath = \"module.json\"\nformat = \"lifted_json\"\nsha256 = \"{module_hash}\"\nsize = {module_size}\nrole = \"lifted_module\"\n\n[[inputs]]\npath = \"main.nso\"\nformat = \"nso0\"\nsha256 = \"{nso_hash}\"\nsize = 4\nrole = \"main_executable\"\n",
        module_hash = module_hash,
        module_size = MODULE_JSON.len()
    );
    fs::write(&provenance_path, provenance).expect("write provenance");

    let report = run_pipeline(PipelineOptions {
        module_path,
        config_path,
        provenance_path,
        out_dir: out_dir.clone(),
        runtime_path,
    })
    .expect("pipeline runs");

    let cargo_toml = out_dir.join("Cargo.toml");
    let main_rs = out_dir.join("src/main.rs");
    let manifest = out_dir.join("manifest.json");
    assert!(cargo_toml.exists(), "Cargo.toml emitted");
    assert!(main_rs.exists(), "main.rs emitted");
    assert!(manifest.exists(), "manifest.json emitted");

    let main_src = fs::read_to_string(main_rs).expect("read main.rs");
    assert!(main_src.contains("svc_log"));
    let manifest_src = fs::read_to_string(manifest).expect("read manifest.json");
    let manifest_json: serde_json::Value =
        serde_json::from_str(&manifest_src).expect("parse manifest.json");
    assert!(manifest_json.get("module_sha256").is_some());
    assert!(manifest_json.get("memory_layout").is_some());
    let regions = manifest_json
        .get("memory_layout")
        .and_then(|value| value.get("regions"))
        .and_then(|value| value.as_array())
        .expect("memory_layout.regions array");
    assert_eq!(regions.len(), 5);
    assert_eq!(
        manifest_json
            .get("manifest_self_hash_basis")
            .and_then(|value| value.as_str()),
        Some("generated_files_self_placeholder")
    );
    let generated_files = manifest_json
        .get("generated_files")
        .and_then(|value| value.as_array())
        .expect("generated_files array");
    assert!(generated_files.iter().any(|entry| {
        entry.get("path").and_then(|value| value.as_str()) == Some("manifest.json")
    }));
    assert_eq!(report.files_written.len(), 3);
    assert_eq!(report.detected_inputs.len(), 2);
}

#[test]
fn pipeline_lowers_load_store_ops() {
    let temp = tempfile::tempdir().expect("tempdir");
    let module_path = temp.path().join("module.json");
    let config_path = temp.path().join("title.toml");
    let provenance_path = temp.path().join("provenance.toml");
    let nso_path = temp.path().join("main.nso");
    let out_dir = temp.path().join("out");
    let runtime_path = PathBuf::from("../crates/recomp-runtime");

    let module_json = r#"{
  "arch": "aarch64",
  "functions": [
    {
      "name": "entry",
      "ops": [
        { "op": "const_i64", "dst": "x0", "imm": 4096 },
        { "op": "const_i64", "dst": "x1", "imm": 123 },
        { "op": "store_i32", "src": "x1", "addr": "x0", "offset": 0 },
        { "op": "load_i32", "dst": "x2", "addr": "x0", "offset": 0 },
        { "op": "ret" }
      ]
    }
  ]
}"#;

    fs::write(&module_path, module_json).expect("write module");
    fs::write(&config_path, CONFIG_TOML).expect("write config");
    fs::write(&nso_path, b"NSO0").expect("write nso");

    let module_hash = sha256_hex(module_json.as_bytes());
    let nso_hash = sha256_hex(b"NSO0");
    let provenance = format!(
        "schema_version = \"1\"\n\n[title]\nname = \"Minimal Sample\"\ntitle_id = \"0100000000000000\"\nversion = \"1.0.0\"\nregion = \"US\"\n\n[collection]\ndevice = \"demo\"\ncollected_at = \"2026-01-30\"\n\n[collection.tool]\nname = \"manual\"\nversion = \"1.0\"\n\n[[inputs]]\npath = \"module.json\"\nformat = \"lifted_json\"\nsha256 = \"{module_hash}\"\nsize = {module_size}\nrole = \"lifted_module\"\n\n[[inputs]]\npath = \"main.nso\"\nformat = \"nso0\"\nsha256 = \"{nso_hash}\"\nsize = 4\nrole = \"main_executable\"\n",
        module_hash = module_hash,
        module_size = module_json.len()
    );
    fs::write(&provenance_path, provenance).expect("write provenance");

    run_pipeline(PipelineOptions {
        module_path,
        config_path,
        provenance_path,
        out_dir: out_dir.clone(),
        runtime_path,
    })
    .expect("pipeline runs");

    let main_rs = out_dir.join("src/main.rs");
    let main_src = fs::read_to_string(main_rs).expect("read main.rs");
    assert!(main_src.contains("mem_store_u32"));
    assert!(main_src.contains("mem_load_u32"));
}

#[test]
fn pipeline_rejects_homebrew_module_json() {
    let temp = tempfile::tempdir().expect("tempdir");
    let module_path = temp.path().join("module.json");
    let config_path = temp.path().join("title.toml");
    let provenance_path = temp.path().join("provenance.toml");
    let out_dir = temp.path().join("out");
    let runtime_path = PathBuf::from("../crates/recomp-runtime");

    let homebrew_module = r#"{
  "schema_version": "1",
  "module_type": "homebrew",
  "modules": [
    {
      "name": "sample",
      "format": "nro",
      "input_path": "module.nro",
      "input_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "input_size": 4,
      "build_id": "deadbeef",
      "segments": [],
      "bss": { "size": 0, "memory_offset": 0 }
    }
  ]
}"#;

    fs::write(&module_path, homebrew_module).expect("write module");
    let err = run_pipeline(PipelineOptions {
        module_path,
        config_path,
        provenance_path,
        out_dir,
        runtime_path,
    })
    .expect_err("pipeline rejects homebrew module.json");

    let message = err.to_string();
    assert!(message.contains("homebrew module.json detected"));
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    format!("{:x}", digest)
}
