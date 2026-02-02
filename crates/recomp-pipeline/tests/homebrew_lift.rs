use recomp_pipeline::homebrew::{lift_homebrew, LiftOptions};
use serde_json::Value;
use std::fs;

#[test]
fn homebrew_lift_emits_stub_module() {
    let temp = tempfile::tempdir().expect("tempdir");
    let base_dir = temp.path();
    let segment_dir = base_dir.join("segments/sample");
    fs::create_dir_all(&segment_dir).expect("segment dir");
    let segment_path = segment_dir.join("text.bin");
    fs::write(&segment_path, [0_u8; 16]).expect("segment data");

    let module_json = format!(
        r#"{{
  "schema_version": "1",
  "module_type": "homebrew",
  "modules": [
    {{
      "name": "sample",
      "format": "nro",
      "input_path": "module.nro",
      "input_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "input_size": 16,
      "build_id": "deadbeef",
      "segments": [
        {{
          "name": "text",
          "file_offset": 0,
          "file_size": 16,
          "memory_offset": 0,
          "memory_size": 16,
          "permissions": "r-x",
          "output_path": "{}"
        }}
      ],
      "bss": {{ "size": 0, "memory_offset": 0 }}
    }}
  ]
}}"#,
        segment_path.strip_prefix(base_dir).unwrap().display()
    );

    let module_path = base_dir.join("module.json");
    fs::write(&module_path, module_json).expect("write module.json");

    let out_dir = base_dir.join("lifted");
    let report = lift_homebrew(LiftOptions {
        module_json_path: module_path,
        out_dir: out_dir.clone(),
        entry_name: "entry".to_string(),
    })
    .expect("lift homebrew");

    assert_eq!(report.functions_emitted, 1);
    assert!(!report.warnings.is_empty());

    let lifted_path = out_dir.join("module.json");
    let lifted_src = fs::read_to_string(lifted_path).expect("read lifted module");
    let lifted_json: Value = serde_json::from_str(&lifted_src).expect("parse lifted module");
    assert_eq!(
        lifted_json.get("arch").and_then(|v| v.as_str()),
        Some("aarch64")
    );
    let functions = lifted_json
        .get("functions")
        .and_then(|v| v.as_array())
        .expect("functions");
    assert_eq!(functions.len(), 1);
    assert_eq!(
        functions[0].get("name").and_then(|v| v.as_str()),
        Some("entry")
    );
    let ops = functions[0]
        .get("ops")
        .and_then(|v| v.as_array())
        .expect("ops");
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].get("op").and_then(|v| v.as_str()), Some("ret"));
}
