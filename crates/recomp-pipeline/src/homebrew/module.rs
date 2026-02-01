use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const MODULE_SCHEMA_VERSION: &str = "1";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModuleJson {
    pub schema_version: String,
    pub module_type: String,
    pub modules: Vec<ModuleBuild>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModuleBuild {
    pub name: String,
    pub format: String,
    pub input_path: PathBuf,
    pub input_sha256: String,
    pub input_size: u64,
    pub build_id: String,
    pub segments: Vec<ModuleSegment>,
    pub bss: BssInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedded: Option<OffsetInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynstr: Option<OffsetInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynsym: Option<OffsetInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModuleSegment {
    pub name: String,
    pub file_offset: u64,
    pub file_size: u64,
    pub memory_offset: u64,
    pub memory_size: u64,
    pub permissions: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compressed: Option<bool>,
    pub output_path: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BssInfo {
    pub size: u64,
    pub memory_offset: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OffsetInfo {
    pub offset: u64,
    pub size: u64,
}

#[derive(Debug)]
pub struct ModuleWriteReport {
    pub module_json_path: PathBuf,
    pub segment_paths: Vec<PathBuf>,
}
