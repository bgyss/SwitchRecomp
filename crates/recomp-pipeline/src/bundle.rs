use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

const BUNDLE_SCHEMA_VERSION: &str = "1";
const BUNDLE_MANIFEST_SELF_PATH: &str = "metadata/bundle-manifest.json";
const BUNDLE_MANIFEST_SELF_SHA_PLACEHOLDER: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Debug)]
pub struct PackageOptions {
    pub project_dir: PathBuf,
    pub provenance_path: PathBuf,
    pub out_dir: PathBuf,
    pub assets_dir: Option<PathBuf>,
}

#[derive(Debug)]
pub struct BundleReport {
    pub out_dir: PathBuf,
    pub manifest_path: PathBuf,
    pub files_written: Vec<PathBuf>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BundleManifest {
    pub schema_version: String,
    pub files: Vec<BundleFile>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BundleFile {
    pub path: String,
    pub sha256: String,
    pub size: u64,
}

pub fn package_bundle(options: PackageOptions) -> Result<BundleReport, String> {
    let out_dir = ensure_dir(&options.out_dir)?;
    let code_dir = out_dir.join("code");
    let assets_dir = out_dir.join("assets");
    let metadata_dir = out_dir.join("metadata");
    ensure_dir(&code_dir)?;
    ensure_dir(&assets_dir)?;
    ensure_dir(&metadata_dir)?;

    let mut written = Vec::new();
    copy_project(&options.project_dir, &code_dir, &mut written)?;
    let provenance_dest = metadata_dir.join("provenance.toml");
    fs::copy(&options.provenance_path, &provenance_dest)
        .map_err(|err| format!("copy provenance: {err}"))?;
    written.push(provenance_dest);

    if let Some(source_assets) = options.assets_dir.as_ref() {
        copy_dir(source_assets, &assets_dir, &mut written)?;
    } else {
        let placeholder = assets_dir.join("README.txt");
        fs::write(
            &placeholder,
            "Place user-supplied assets in this directory. Do not distribute proprietary data.",
        )
        .map_err(|err| format!("write assets placeholder: {err}"))?;
        written.push(placeholder);
    }

    let files = collect_bundle_files(&out_dir)?;
    let (_manifest, manifest_json) = build_bundle_manifest(files)?;
    let manifest_path = metadata_dir.join("bundle-manifest.json");
    fs::write(&manifest_path, manifest_json).map_err(|err| err.to_string())?;
    written.push(manifest_path.clone());

    Ok(BundleReport {
        out_dir,
        manifest_path,
        files_written: written,
    })
}

fn ensure_dir(path: &Path) -> Result<PathBuf, String> {
    fs::create_dir_all(path).map_err(|err| format!("create dir {}: {err}", path.display()))?;
    Ok(path.to_path_buf())
}

fn copy_project(
    project_dir: &Path,
    code_dir: &Path,
    written: &mut Vec<PathBuf>,
) -> Result<(), String> {
    let cargo_toml = project_dir.join("Cargo.toml");
    let manifest_json = project_dir.join("manifest.json");
    let src_dir = project_dir.join("src");

    copy_file(&cargo_toml, &code_dir.join("Cargo.toml"), written)?;
    copy_file(&manifest_json, &code_dir.join("manifest.json"), written)?;
    copy_dir(&src_dir, &code_dir.join("src"), written)?;
    Ok(())
}

fn copy_file(src: &Path, dest: &Path, written: &mut Vec<PathBuf>) -> Result<(), String> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("create dir {}: {err}", parent.display()))?;
    }
    fs::copy(src, dest).map_err(|err| format!("copy {}: {err}", src.display()))?;
    written.push(dest.to_path_buf());
    Ok(())
}

fn copy_dir(src: &Path, dest: &Path, written: &mut Vec<PathBuf>) -> Result<(), String> {
    fs::create_dir_all(dest).map_err(|err| format!("create dir {}: {err}", dest.display()))?;
    for entry in fs::read_dir(src).map_err(|err| format!("read dir {}: {err}", src.display()))? {
        let entry = entry.map_err(|err| err.to_string())?;
        let path = entry.path();
        let target = dest.join(entry.file_name());
        if path.is_dir() {
            copy_dir(&path, &target, written)?;
        } else {
            copy_file(&path, &target, written)?;
        }
    }
    Ok(())
}

fn collect_bundle_files(root: &Path) -> Result<Vec<BundleFile>, String> {
    let mut files = Vec::new();
    collect_recursive(root, root, &mut files)?;
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

pub fn bundle_manifest_self_hash(manifest: &BundleManifest) -> Result<String, String> {
    let mut normalized = manifest.clone();
    let self_entry = find_self_entry_mut(&mut normalized)?;
    self_entry.sha256 = BUNDLE_MANIFEST_SELF_SHA_PLACEHOLDER.to_string();
    let manifest_json = serde_json::to_string_pretty(&normalized).map_err(|err| err.to_string())?;
    Ok(sha256_bytes(manifest_json.as_bytes()))
}

fn build_bundle_manifest(files: Vec<BundleFile>) -> Result<(BundleManifest, String), String> {
    let mut files = files;
    if files
        .iter()
        .any(|file| file.path == BUNDLE_MANIFEST_SELF_PATH)
    {
        return Err("bundle manifest already present in file list".to_string());
    }

    files.push(BundleFile {
        path: BUNDLE_MANIFEST_SELF_PATH.to_string(),
        sha256: BUNDLE_MANIFEST_SELF_SHA_PLACEHOLDER.to_string(),
        size: 0,
    });
    files.sort_by(|a, b| a.path.cmp(&b.path));

    let mut manifest = BundleManifest {
        schema_version: BUNDLE_SCHEMA_VERSION.to_string(),
        files,
    };

    let size = stabilize_manifest_size(&mut manifest)?;
    let self_hash = bundle_manifest_self_hash(&manifest)?;
    let self_entry = find_self_entry_mut(&mut manifest)?;
    self_entry.size = size;
    self_entry.sha256 = self_hash;

    let manifest_json = serde_json::to_string_pretty(&manifest).map_err(|err| err.to_string())?;
    let final_size = manifest_json.len() as u64;
    if final_size != size {
        return Err(format!(
            "bundle manifest size mismatch: expected {size}, got {final_size}"
        ));
    }

    Ok((manifest, manifest_json))
}

fn stabilize_manifest_size(manifest: &mut BundleManifest) -> Result<u64, String> {
    let mut size = 0_u64;
    for _ in 0..5 {
        let self_entry = find_self_entry_mut(manifest)?;
        self_entry.sha256 = BUNDLE_MANIFEST_SELF_SHA_PLACEHOLDER.to_string();
        self_entry.size = size;
        let manifest_json =
            serde_json::to_string_pretty(manifest).map_err(|err| err.to_string())?;
        let new_size = manifest_json.len() as u64;
        if new_size == size {
            return Ok(size);
        }
        size = new_size;
    }
    Err("bundle manifest size did not stabilize".to_string())
}

fn find_self_entry_mut(manifest: &mut BundleManifest) -> Result<&mut BundleFile, String> {
    manifest
        .files
        .iter_mut()
        .find(|entry| entry.path == BUNDLE_MANIFEST_SELF_PATH)
        .ok_or_else(|| "bundle manifest self entry not found".to_string())
}

fn collect_recursive(root: &Path, dir: &Path, files: &mut Vec<BundleFile>) -> Result<(), String> {
    for entry in fs::read_dir(dir).map_err(|err| format!("read dir {}: {err}", dir.display()))? {
        let entry = entry.map_err(|err| err.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            collect_recursive(root, &path, files)?;
        } else {
            let bytes = fs::read(&path).map_err(|err| err.to_string())?;
            let sha256 = sha256_bytes(&bytes);
            let size = bytes.len() as u64;
            let rel = path
                .strip_prefix(root)
                .map_err(|_| "strip prefix failed".to_string())?
                .to_string_lossy()
                .replace('\\', "/");
            files.push(BundleFile {
                path: rel,
                sha256,
                size,
            });
        }
    }
    Ok(())
}

fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    format!("{:x}", digest)
}
