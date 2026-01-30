use recomp_pipeline::bundle::{package_bundle, BundleManifest, PackageOptions};
use sha2::{Digest, Sha256};
use std::fs;

#[test]
fn bundle_manifest_checksums_match_contents() {
    let temp = tempfile::tempdir().expect("tempdir");
    let project_dir = temp.path().join("project");
    let provenance_path = temp.path().join("provenance.toml");
    let out_dir = temp.path().join("bundle");

    fs::create_dir_all(project_dir.join("src")).expect("create project");
    fs::write(project_dir.join("Cargo.toml"), "name = \"demo\"").expect("write Cargo.toml");
    fs::write(project_dir.join("src/main.rs"), "fn main() {}").expect("write main.rs");
    fs::write(project_dir.join("manifest.json"), "{}").expect("write manifest.json");
    fs::write(&provenance_path, "schema_version = \"1\"").expect("write provenance");

    let report = package_bundle(PackageOptions {
        project_dir: project_dir.clone(),
        provenance_path: provenance_path.clone(),
        out_dir: out_dir.clone(),
        assets_dir: None,
    })
    .expect("package bundle");

    let manifest_src = fs::read_to_string(&report.manifest_path).expect("read manifest");
    let manifest: BundleManifest = serde_json::from_str(&manifest_src).expect("parse manifest");

    for entry in manifest.files {
        let path = out_dir.join(entry.path);
        let bytes = fs::read(&path).expect("read file");
        let sha = sha256_bytes(&bytes);
        assert_eq!(
            sha,
            entry.sha256,
            "checksum mismatch for {}",
            path.display()
        );
        assert_eq!(
            bytes.len() as u64,
            entry.size,
            "size mismatch for {}",
            path.display()
        );
    }
}

fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    format!("{:x}", digest)
}
