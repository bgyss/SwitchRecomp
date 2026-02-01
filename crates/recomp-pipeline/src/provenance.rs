use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

const SCHEMA_VERSION: &str = "1";

#[derive(Debug, Deserialize, Clone)]
pub struct ProvenanceManifest {
    pub schema_version: String,
    pub title: TitleInfo,
    pub collection: CollectionInfo,
    pub inputs: Vec<InputRecord>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TitleInfo {
    pub name: String,
    pub title_id: String,
    pub version: String,
    pub region: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CollectionInfo {
    pub device: String,
    pub tool: ToolInfo,
    #[serde(default)]
    pub decryption_tool: Option<ToolInfo>,
    #[serde(default)]
    pub notes: Option<String>,
    pub collected_at: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ToolInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct InputRecord {
    pub path: PathBuf,
    pub sha256: String,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default)]
    pub format: Option<InputFormatHint>,
    #[serde(default)]
    pub role: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputFormatHint {
    Nca,
    Exefs,
    Nso0,
    Nro0,
    Nrr0,
    Npdm,
    LiftedJson,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputFormat {
    Nca,
    Exefs,
    Nso0,
    Nro0,
    Nrr0,
    Npdm,
    LiftedJson,
}

impl InputFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            InputFormat::Nca => "nca",
            InputFormat::Exefs => "exefs",
            InputFormat::Nso0 => "nso0",
            InputFormat::Nro0 => "nro0",
            InputFormat::Nrr0 => "nrr0",
            InputFormat::Npdm => "npdm",
            InputFormat::LiftedJson => "lifted_json",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ValidatedInput {
    pub path: PathBuf,
    pub format: InputFormat,
    pub sha256: String,
    pub size: u64,
    pub role: Option<String>,
}

#[derive(Debug)]
pub struct ProvenanceValidation {
    pub manifest: ProvenanceManifest,
    pub manifest_sha256: String,
    pub inputs: Vec<ValidatedInput>,
}

impl ProvenanceManifest {
    pub fn parse(toml_src: &str) -> Result<Self, String> {
        toml::from_str(toml_src).map_err(|err| format!("invalid provenance metadata: {err}"))
    }

    pub fn validate(
        &self,
        manifest_path: &Path,
        manifest_src: &str,
    ) -> Result<ProvenanceValidation, String> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(format!(
                "unsupported provenance schema version: {}",
                self.schema_version
            ));
        }
        if self.inputs.is_empty() {
            return Err("provenance inputs list is empty".to_string());
        }
        if self.title.name.trim().is_empty()
            || self.title.title_id.trim().is_empty()
            || self.title.version.trim().is_empty()
            || self.title.region.trim().is_empty()
        {
            return Err("provenance title metadata is incomplete".to_string());
        }
        if self.collection.device.trim().is_empty()
            || self.collection.tool.name.trim().is_empty()
            || self.collection.tool.version.trim().is_empty()
            || self.collection.collected_at.trim().is_empty()
        {
            return Err("provenance collection metadata is incomplete".to_string());
        }

        let base_dir = manifest_path
            .parent()
            .ok_or_else(|| "provenance metadata has no parent directory".to_string())?;

        let mut inputs = Vec::new();
        for record in &self.inputs {
            let resolved = if record.path.is_absolute() {
                record.path.clone()
            } else {
                base_dir.join(&record.path)
            };
            let size = fs::metadata(&resolved)
                .map_err(|err| format!("provenance input missing: {} ({err})", resolved.display()))?
                .len();
            if let Some(expected_size) = record.size {
                if expected_size != size {
                    return Err(format!(
                        "provenance size mismatch for {}: expected {}, found {}",
                        resolved.display(),
                        expected_size,
                        size
                    ));
                }
            }
            if !is_hex_sha256(&record.sha256) {
                return Err(format!(
                    "provenance sha256 is invalid for {}",
                    resolved.display()
                ));
            }
            let actual_hash = sha256_path(&resolved)
                .map_err(|err| format!("hashing failed for {}: {err}", resolved.display()))?;
            if actual_hash != record.sha256 {
                return Err(format!(
                    "provenance sha256 mismatch for {}",
                    resolved.display()
                ));
            }
            let detected = detect_format(&resolved)?;
            if let Some(hint) = record.format {
                let expected = match hint {
                    InputFormatHint::Nca => InputFormat::Nca,
                    InputFormatHint::Exefs => InputFormat::Exefs,
                    InputFormatHint::Nso0 => InputFormat::Nso0,
                    InputFormatHint::Nro0 => InputFormat::Nro0,
                    InputFormatHint::Nrr0 => InputFormat::Nrr0,
                    InputFormatHint::Npdm => InputFormat::Npdm,
                    InputFormatHint::LiftedJson => InputFormat::LiftedJson,
                };
                if expected != detected {
                    return Err(format!(
                        "provenance format mismatch for {}: expected {}, detected {}",
                        resolved.display(),
                        expected.as_str(),
                        detected.as_str()
                    ));
                }
            }

            inputs.push(ValidatedInput {
                path: resolved,
                format: detected,
                sha256: actual_hash,
                size,
                role: record.role.clone(),
            });
        }

        Ok(ProvenanceValidation {
            manifest: ProvenanceManifest {
                schema_version: self.schema_version.clone(),
                title: TitleInfo {
                    name: self.title.name.clone(),
                    title_id: self.title.title_id.clone(),
                    version: self.title.version.clone(),
                    region: self.title.region.clone(),
                },
                collection: CollectionInfo {
                    device: self.collection.device.clone(),
                    tool: ToolInfo {
                        name: self.collection.tool.name.clone(),
                        version: self.collection.tool.version.clone(),
                    },
                    decryption_tool: self.collection.decryption_tool.as_ref().map(|tool| {
                        ToolInfo {
                            name: tool.name.clone(),
                            version: tool.version.clone(),
                        }
                    }),
                    notes: self.collection.notes.clone(),
                    collected_at: self.collection.collected_at.clone(),
                },
                inputs: self.inputs.clone(),
            },
            manifest_sha256: sha256_bytes(manifest_src.as_bytes()),
            inputs,
        })
    }
}

pub fn detect_format(path: &Path) -> Result<InputFormat, String> {
    if let Some(ext) = path.extension().and_then(|value| value.to_str()) {
        if ext.eq_ignore_ascii_case("json") {
            return Ok(InputFormat::LiftedJson);
        }
    }

    let bytes =
        fs::read(path).map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    if bytes.len() < 4 {
        return Err(format!(
            "unsupported input format (too small: {} bytes) for {}",
            bytes.len(),
            path.display()
        ));
    }
    let magic = &bytes[0..4];
    match magic {
        b"NCA3" | b"NCA2" => Ok(InputFormat::Nca),
        b"PFS0" => Ok(InputFormat::Exefs),
        b"NSO0" => Ok(InputFormat::Nso0),
        b"NRO0" => Ok(InputFormat::Nro0),
        b"NRR0" => Ok(InputFormat::Nrr0),
        b"META" | b"NPDM" => Ok(InputFormat::Npdm),
        _ => {
            if bytes.len() >= 0x14 && &bytes[0x10..0x14] == b"NRO0" {
                return Ok(InputFormat::Nro0);
            }
            Err(format!(
                "unsupported input format (magic {:?}) for {}",
                magic,
                path.display()
            ))
        }
    }
}

fn sha256_path(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|err| err.to_string())?;
    Ok(sha256_bytes(&bytes))
}

fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    format!("{:x}", digest)
}

fn is_hex_sha256(value: &str) -> bool {
    if value.len() != 64 {
        return false;
    }
    value
        .bytes()
        .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}
