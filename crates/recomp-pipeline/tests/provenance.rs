use recomp_pipeline::provenance::{detect_format, ProvenanceManifest};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("formats")
        .join(name)
}

#[test]
fn detect_supported_formats() {
    let cases = vec![
        ("program.nca", "nca"),
        ("exefs.pfs0", "exefs"),
        ("main.nso", "nso0"),
        ("homebrew.nro", "nro0"),
        ("plugins.nrr", "nrr0"),
        ("main.npdm", "npdm"),
        ("sample.xci", "xci"),
        ("sample.keys", "keyset"),
    ];

    for (file, expected) in cases {
        let path = fixture_path(file);
        let detected = detect_format(&path).expect("format detected");
        assert_eq!(detected.as_str(), expected);
    }
}

#[test]
fn provenance_validation_logs_inputs() {
    let nca_path = fixture_path("program.nca");
    let nso_path = fixture_path("main.nso");

    let nca_hash = sha256_path(&nca_path);
    let nso_hash = sha256_path(&nso_path);
    let nca_size = fs::metadata(&nca_path).expect("metadata").len();
    let nso_size = fs::metadata(&nso_path).expect("metadata").len();

    let provenance = format!(
        "schema_version = \"1\"\n\n[title]\nname = \"Fixture\"\ntitle_id = \"0100000000000000\"\nversion = \"1.0.0\"\nregion = \"US\"\n\n[collection]\ndevice = \"fixture\"\ncollected_at = \"2026-01-30\"\n\n[collection.tool]\nname = \"fixture\"\nversion = \"1.0\"\n\n[[inputs]]\npath = \"{}\"\nformat = \"nca\"\nsha256 = \"{}\"\nsize = {}\nrole = \"program_nca\"\n\n[[inputs]]\npath = \"{}\"\nformat = \"nso0\"\nsha256 = \"{}\"\nsize = {}\nrole = \"main_executable\"\n",
        nca_path.display(),
        nca_hash,
        nca_size,
        nso_path.display(),
        nso_hash,
        nso_size
    );
    let manifest = ProvenanceManifest::parse(&provenance).expect("parse");
    let validation = manifest
        .validate(Path::new("fixtures/manifest.toml"), &provenance)
        .expect("validate");

    assert_eq!(validation.inputs.len(), 2);
    assert_eq!(validation.inputs[0].format.as_str(), "nca");
    assert_eq!(validation.inputs[1].format.as_str(), "nso0");
}

fn sha256_path(path: &Path) -> String {
    let bytes = fs::read(path).expect("read file");
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    format!("{:x}", digest)
}
