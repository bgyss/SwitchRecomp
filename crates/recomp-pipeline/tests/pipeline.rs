use recomp_pipeline::config::TitleConfig;
use recomp_pipeline::input::Module;
use recomp_pipeline::{run_pipeline, PipelineOptions};
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
    let module: Module = serde_json::from_str(r#"{"arch":"mips","functions":[]}"#)
    .expect("module parses");
    let err = module.validate_arch().unwrap_err();
    assert!(err.contains("unsupported arch"));
}

#[test]
fn pipeline_emits_project() {
    let temp = tempfile::tempdir().expect("tempdir");
    let module_path = temp.path().join("module.json");
    let config_path = temp.path().join("title.toml");
    let out_dir = temp.path().join("out");
    let runtime_path = PathBuf::from("../crates/recomp-runtime");

    fs::write(&module_path, MODULE_JSON).expect("write module");
    fs::write(&config_path, CONFIG_TOML).expect("write config");

    let report = run_pipeline(PipelineOptions {
        module_path,
        config_path,
        out_dir: out_dir.clone(),
        runtime_path,
    })
    .expect("pipeline runs");

    let cargo_toml = out_dir.join("Cargo.toml");
    let main_rs = out_dir.join("src/main.rs");
    assert!(cargo_toml.exists(), "Cargo.toml emitted");
    assert!(main_rs.exists(), "main.rs emitted");

    let main_src = fs::read_to_string(main_rs).expect("read main.rs");
    assert!(main_src.contains("svc_log"));
    assert_eq!(report.files_written.len(), 2);
}
