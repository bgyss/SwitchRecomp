use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct XciFile {
    pub name: String,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct XciProgram {
    pub title_id: String,
    pub content_type: String,
    pub version: String,
    pub nca_bytes: Vec<u8>,
    pub exefs_files: Vec<XciFile>,
    pub nso_files: Vec<XciFile>,
}

#[derive(Debug, Clone)]
pub struct XciExtractResult {
    pub programs: Vec<XciProgram>,
    pub romfs_image: Option<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct XciExtractRequest {
    pub xci_path: PathBuf,
    pub keys_path: PathBuf,
}

pub trait XciExtractor {
    fn extract(&self, request: &XciExtractRequest) -> Result<XciExtractResult, String>;
}
