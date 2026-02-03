use crate::xci::types::{XciExtractRequest, XciExtractResult, XciExtractor, XciFile, XciProgram};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XciToolKind {
    Hactool,
    Hactoolnet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XciToolPreference {
    Auto,
    Hactool,
    Hactoolnet,
    Mock,
}

impl XciToolPreference {
    pub fn from_env() -> Option<Self> {
        let value = env::var("RECOMP_XCI_TOOL").ok()?;
        match value.to_ascii_lowercase().as_str() {
            "auto" => Some(Self::Auto),
            "hactool" => Some(Self::Hactool),
            "hactoolnet" => Some(Self::Hactoolnet),
            "mock" => Some(Self::Mock),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
struct XciTool {
    path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ExternalXciExtractor {
    tool: XciTool,
}

impl ExternalXciExtractor {
    pub fn detect(
        preference: XciToolPreference,
        tool_path: Option<&Path>,
    ) -> Result<Option<Self>, String> {
        let env_pref = XciToolPreference::from_env().unwrap_or(preference);
        if matches!(env_pref, XciToolPreference::Mock) {
            return Ok(None);
        }

        let env_path = env::var_os("RECOMP_XCI_TOOL_PATH").map(PathBuf::from);
        let path_override = tool_path.map(PathBuf::from).or(env_path);
        let tool = match env_pref {
            XciToolPreference::Auto => detect_tool(path_override)?,
            XciToolPreference::Hactool => detect_specific(XciToolKind::Hactool, path_override)?,
            XciToolPreference::Hactoolnet => {
                detect_specific(XciToolKind::Hactoolnet, path_override)?
            }
            XciToolPreference::Mock => None,
        };

        Ok(tool.map(|tool| Self { tool }))
    }

    fn run(&self, args: &[&str]) -> Result<String, String> {
        let output = Command::new(&self.tool.path)
            .args(args)
            .output()
            .map_err(|err| format!("failed to run {}: {err}", self.tool.path.display()))?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        if output.status.success() {
            Ok(stdout)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!(
                "{} failed (status={}): {}{}",
                self.tool.path.display(),
                output.status,
                stderr,
                if stdout.is_empty() {
                    String::new()
                } else {
                    format!("\nstdout:\n{stdout}")
                }
            ))
        }
    }

    fn extract_xci(&self, request: &XciExtractRequest, out_dir: &Path) -> Result<(), String> {
        let args = [
            "-k",
            request.keys_path.to_str().ok_or("keys path invalid")?,
            "--intype=xci",
            "--outdir",
            out_dir.to_str().ok_or("xci out dir invalid")?,
            request.xci_path.to_str().ok_or("xci path invalid")?,
        ];
        self.run(&args)?;
        Ok(())
    }

    fn list_titles(&self, request: &XciExtractRequest) -> Option<Vec<ProgramMetadata>> {
        let args = [
            "-k",
            request.keys_path.to_str()?,
            "--intype=xci",
            "--listtitles",
            request.xci_path.to_str()?,
        ];
        let output = self.run(&args).ok()?;
        Some(parse_title_listing(&output))
    }

    fn extract_nca(
        &self,
        request: &XciExtractRequest,
        nca_path: &Path,
        exefs: &Path,
        romfs: &Path,
    ) -> Result<(), String> {
        let args = [
            "-k",
            request.keys_path.to_str().ok_or("keys path invalid")?,
            "--intype=nca",
            "--exefsdir",
            exefs.to_str().ok_or("exefs dir invalid")?,
            "--romfsdir",
            romfs.to_str().ok_or("romfs dir invalid")?,
            nca_path.to_str().ok_or("nca path invalid")?,
        ];
        self.run(&args)?;
        Ok(())
    }
}

impl XciExtractor for ExternalXciExtractor {
    fn extract(&self, request: &XciExtractRequest) -> Result<XciExtractResult, String> {
        let temp = tempfile::tempdir().map_err(|err| format!("create temp dir: {err}"))?;
        let xci_out = temp.path().join("xci");
        fs::create_dir_all(&xci_out)
            .map_err(|err| format!("create xci dir {}: {err}", xci_out.display()))?;

        self.extract_xci(request, &xci_out)?;

        let mut nca_files = Vec::new();
        collect_nca_files(&xci_out, &mut nca_files)?;
        if nca_files.is_empty() {
            return Err("no NCA files extracted from XCI".to_string());
        }

        let metadata = self.list_titles(request).unwrap_or_default();
        let mut programs = Vec::new();
        let mut matched = Vec::new();

        for meta in &metadata {
            if let Some(content_id) = &meta.content_id {
                if let Some(path) = find_nca_by_content_id(&nca_files, content_id) {
                    matched.push(path.clone());
                    programs.push(build_program(self, request, path, meta)?);
                }
            }
        }

        if programs.is_empty() {
            for (index, nca_path) in nca_files.iter().enumerate() {
                let meta = ProgramMetadata {
                    title_id: "unknown".to_string(),
                    content_type: "program".to_string(),
                    version: format!("unknown-{index}"),
                    content_id: None,
                };
                programs.push(build_program(self, request, nca_path.clone(), &meta)?);
            }
        } else {
            for nca_path in &nca_files {
                if matched.iter().any(|path| path == nca_path) {
                    continue;
                }
            }
        }

        Ok(XciExtractResult { programs })
    }
}

fn build_program(
    extractor: &ExternalXciExtractor,
    request: &XciExtractRequest,
    nca_path: PathBuf,
    meta: &ProgramMetadata,
) -> Result<XciProgram, String> {
    let temp = tempfile::tempdir().map_err(|err| format!("create temp dir: {err}"))?;
    let exefs_dir = temp.path().join("exefs");
    let romfs_dir = temp.path().join("romfs");
    fs::create_dir_all(&exefs_dir)
        .map_err(|err| format!("create exefs dir {}: {err}", exefs_dir.display()))?;
    fs::create_dir_all(&romfs_dir)
        .map_err(|err| format!("create romfs dir {}: {err}", romfs_dir.display()))?;

    extractor.extract_nca(request, &nca_path, &exefs_dir, &romfs_dir)?;

    let mut exefs_files = Vec::new();
    let mut nso_files = Vec::new();
    for entry in fs::read_dir(&exefs_dir)
        .map_err(|err| format!("read exefs dir {}: {err}", exefs_dir.display()))?
    {
        let entry = entry.map_err(|err| format!("read exefs entry: {err}"))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "invalid exefs file name".to_string())?;
        let data = fs::read(&path).map_err(|err| format!("read exefs file: {err}"))?;
        let file = XciFile {
            name: name.clone(),
            data: data.clone(),
        };
        if is_nso_name(&name) {
            nso_files.push(file.clone());
        }
        exefs_files.push(file);
    }

    exefs_files.sort_by(|a, b| a.name.cmp(&b.name));
    nso_files.sort_by(|a, b| a.name.cmp(&b.name));

    let romfs_entries = collect_romfs_entries(&romfs_dir)?;

    Ok(XciProgram {
        title_id: meta.title_id.clone(),
        content_type: meta.content_type.clone(),
        version: meta.version.clone(),
        nca_bytes: fs::read(&nca_path).map_err(|err| format!("read NCA: {err}"))?,
        exefs_files,
        nso_files,
        romfs_image: None,
        romfs_entries,
    })
}

fn is_nso_name(name: &str) -> bool {
    if name == "main" {
        return true;
    }
    if name.ends_with(".nso") {
        return true;
    }
    !name.contains('.') && name != "main.npdm"
}

fn collect_romfs_entries(root: &Path) -> Result<Vec<XciFile>, String> {
    let mut entries = Vec::new();
    collect_romfs_entries_recursive(root, root, &mut entries)?;
    Ok(entries)
}

fn collect_romfs_entries_recursive(
    root: &Path,
    current: &Path,
    entries: &mut Vec<XciFile>,
) -> Result<(), String> {
    let dir_entries = match fs::read_dir(current) {
        Ok(entries) => entries,
        Err(_) => return Ok(()),
    };
    for entry in dir_entries {
        let entry = entry.map_err(|err| format!("read romfs entry: {err}"))?;
        let path = entry.path();
        if path.is_dir() {
            collect_romfs_entries_recursive(root, &path, entries)?;
            continue;
        }
        let rel = path
            .strip_prefix(root)
            .map_err(|_| "romfs entry outside root".to_string())?;
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        let data =
            fs::read(&path).map_err(|err| format!("read romfs file {}: {err}", path.display()))?;
        entries.push(XciFile {
            name: rel_str,
            data,
        });
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct ProgramMetadata {
    title_id: String,
    content_type: String,
    version: String,
    content_id: Option<String>,
}

fn parse_title_listing(output: &str) -> Vec<ProgramMetadata> {
    let mut out = Vec::new();
    let mut current = ProgramMetadata {
        title_id: String::new(),
        content_type: "program".to_string(),
        version: "unknown".to_string(),
        content_id: None,
    };

    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !current.title_id.is_empty() {
                out.push(current.clone());
                current = ProgramMetadata {
                    title_id: String::new(),
                    content_type: "program".to_string(),
                    version: "unknown".to_string(),
                    content_id: None,
                };
            }
            continue;
        }
        let lower = trimmed.to_ascii_lowercase();
        if lower.starts_with("title id") {
            if !current.title_id.is_empty() {
                out.push(current.clone());
            }
            current = ProgramMetadata {
                title_id: after_colon(trimmed),
                content_type: "program".to_string(),
                version: "unknown".to_string(),
                content_id: None,
            };
        } else if lower.starts_with("content type") {
            current.content_type = after_colon(trimmed).to_ascii_lowercase();
        } else if lower.starts_with("version") {
            current.version = after_colon(trimmed);
        } else if lower.starts_with("content id") {
            current.content_id = Some(after_colon(trimmed).to_ascii_lowercase());
        }
    }

    if !current.title_id.is_empty() {
        out.push(current);
    }

    out
}

fn after_colon(line: &str) -> String {
    line.split_once(':')
        .map(|(_, value)| value.trim())
        .unwrap_or("")
        .to_string()
}

fn collect_nca_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(dir).map_err(|err| format!("read dir {}: {err}", dir.display()))? {
        let entry = entry.map_err(|err| format!("read entry: {err}"))?;
        let path = entry.path();
        if path.is_dir() {
            collect_nca_files(&path, out)?;
            continue;
        }
        if path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("nca"))
        {
            out.push(path);
        }
    }
    out.sort();
    Ok(())
}

fn find_nca_by_content_id(ncas: &[PathBuf], content_id: &str) -> Option<PathBuf> {
    let target = content_id.to_ascii_lowercase();
    for path in ncas {
        let stem = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if stem == target {
            return Some(path.clone());
        }
    }
    None
}

fn detect_tool(path_override: Option<PathBuf>) -> Result<Option<XciTool>, String> {
    if let Some(path) = path_override {
        return Ok(Some(infer_tool_kind(path)?));
    }
    if let Some(path) = find_on_path("hactoolnet") {
        return Ok(Some(XciTool { path }));
    }
    if let Some(path) = find_on_path("hactool") {
        return Ok(Some(XciTool { path }));
    }
    Ok(None)
}

fn detect_specific(
    kind: XciToolKind,
    path_override: Option<PathBuf>,
) -> Result<Option<XciTool>, String> {
    let path = if let Some(path) = path_override {
        path
    } else {
        let name = match kind {
            XciToolKind::Hactool => "hactool",
            XciToolKind::Hactoolnet => "hactoolnet",
        };
        match find_on_path(name) {
            Some(path) => path,
            None => return Err(format!("{} not found on PATH", name)),
        }
    };
    Ok(Some(XciTool { path }))
}

fn infer_tool_kind(path: PathBuf) -> Result<XciTool, String> {
    if !path.is_file() {
        return Err(format!("xci tool path is not a file: {}", path.display()));
    }
    Ok(XciTool { path })
}

fn find_on_path(name: &str) -> Option<PathBuf> {
    let path_var = env::var_os("PATH")?;
    for dir in env::split_paths(&path_var) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
        #[cfg(windows)]
        {
            let candidate = dir.join(format!("{name}.exe"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}
