use recomp_validation::{run_artifact_validation, ArtifactIndex};
use std::fs;
#[test]
fn artifact_validation_runs_xci_manifest_check() {
    let temp = tempfile::tempdir().expect("tempdir");
    let out_dir = temp.path().join("out");
    let manifest_dir = temp.path().join("xci");
    let generated_path = manifest_dir.join("intake/exefs/main");
    fs::create_dir_all(generated_path.parent().expect("parent")).expect("create dirs");
    fs::write(&generated_path, b"main").expect("write generated file");

    let manifest_path = manifest_dir.join("manifest.json");
    let manifest = serde_json::json!({
        "schema_version": "1",
        "program": {
            "name": "test",
            "title_id": "0100000000000000",
            "version": "1.0.0",
            "content_type": "program"
        },
        "assets": [],
        "generated_files": [
            {"path": "manifest.json", "sha256": "00", "size": 0},
            {"path": "intake/exefs/main", "sha256": "00", "size": 4}
        ]
    });
    fs::create_dir_all(&manifest_dir).expect("create manifest dir");
    fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .expect("write manifest");

    let index = ArtifactIndex {
        label: Some("test".to_string()),
        xci_intake_manifest: Some(manifest_path),
        pipeline_manifest: None,
        run_manifest: None,
        reference_config: None,
        capture_config: None,
        validation_config: None,
        out_dir: Some(out_dir.clone()),
    };

    let report = run_artifact_validation(&index);
    assert_eq!(report.failed, 0);
    assert_eq!(report.passed, 1);
    assert_eq!(report.cases.len(), 1);
    assert_eq!(report.cases[0].name, "xci_intake_manifest");
}

#[test]
fn artifact_validation_accepts_run_manifest() {
    let temp = tempfile::tempdir().expect("tempdir");
    let out_dir = temp.path().join("out");
    let run_manifest_path = temp.path().join("run-manifest.json");
    let run_manifest = serde_json::json!({
        "schema_version": "1",
        "run_id": "run-0000000000000000",
        "execution_mode": "local",
        "host_fingerprint": "0".repeat(64),
        "tool_versions": {"recomp_cli": "0.1.0", "rustc": null, "ffmpeg": null, "xci_tool": null},
        "input_fingerprint": "1".repeat(64),
        "inputs": [],
        "steps": [],
        "artifacts": []
    });
    fs::write(
        &run_manifest_path,
        serde_json::to_string_pretty(&run_manifest).expect("serialize run manifest"),
    )
    .expect("write run manifest");

    let index = ArtifactIndex {
        label: Some("run-manifest-test".to_string()),
        xci_intake_manifest: None,
        pipeline_manifest: None,
        run_manifest: Some(run_manifest_path),
        reference_config: None,
        capture_config: None,
        validation_config: None,
        out_dir: Some(out_dir),
    };

    let report = run_artifact_validation(&index);
    assert_eq!(report.failed, 0);
    assert_eq!(report.passed, 1);
    assert_eq!(report.cases.len(), 1);
    assert_eq!(report.cases[0].name, "run_manifest");
}
