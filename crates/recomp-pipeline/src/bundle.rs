use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

const BUNDLE_SCHEMA_VERSION: &str = "1";

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

#[derive(Debug, Serialize, Deserialize)]
pub struct BundleManifest {
    pub schema_version: String,
    pub files: Vec<BundleFile>,
}

#[derive(Debug, Serialize, Deserialize)]
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

    let manifest = BundleManifest {
        schema_version: BUNDLE_SCHEMA_VERSION.to_string(),
        files: collect_bundle_files(&out_dir)?,
    };
    let manifest_path = metadata_dir.join("bundle-manifest.json");
    let manifest_json = serde_json::to_string_pretty(&manifest).map_err(|err| err.to_string())?;
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
