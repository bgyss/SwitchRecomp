use crate::xci::types::{XciExtractRequest, XciExtractResult, XciExtractor, XciFile, XciProgram};
use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use serde::Deserialize;
use std::fs;

const MOCK_SCHEMA_VERSION: &str = "1";

#[derive(Debug, Deserialize)]
struct MockXciImage {
    schema_version: String,
    programs: Vec<MockProgram>,
    #[serde(default)]
    romfs: Option<MockRomfs>,
}

#[derive(Debug, Deserialize)]
struct MockProgram {
    title_id: String,
    content_type: String,
    version: String,
    nca: MockBlob,
    exefs: Vec<MockFile>,
    #[serde(default)]
    nso: Vec<MockFile>,
}

#[derive(Debug, Deserialize)]
struct MockRomfs {
    image_b64: String,
}

#[derive(Debug, Deserialize)]
struct MockFile {
    name: String,
    data_b64: String,
}

#[derive(Debug, Deserialize)]
struct MockBlob {
    data_b64: String,
}

#[derive(Debug, Default)]
pub struct MockXciExtractor;

impl MockXciExtractor {
    pub fn new() -> Self {
        Self
    }
}

impl XciExtractor for MockXciExtractor {
    fn extract(&self, request: &XciExtractRequest) -> Result<XciExtractResult, String> {
        let payload = fs::read_to_string(&request.xci_path)
            .map_err(|err| format!("read mock xci {}: {err}", request.xci_path.display()))?;
        let image: MockXciImage =
            serde_json::from_str(&payload).map_err(|err| format!("parse mock xci: {err}"))?;
        if image.schema_version != MOCK_SCHEMA_VERSION {
            return Err(format!(
                "unsupported mock xci schema version: {}",
                image.schema_version
            ));
        }

        let mut programs = Vec::new();
        for program in image.programs {
            let nca_bytes = decode_b64("nca", &program.nca.data_b64)?;
            let exefs_files = decode_files(&program.exefs)?;
            let nso_files = decode_files(&program.nso)?;
            let romfs_image = match &image.romfs {
                Some(romfs) => Some(decode_b64("romfs", &romfs.image_b64)?),
                None => None,
            };
            programs.push(XciProgram {
                title_id: program.title_id,
                content_type: program.content_type,
                version: program.version,
                nca_bytes,
                exefs_files,
                nso_files,
                romfs_image,
                romfs_entries: Vec::new(),
            });
        }
        Ok(XciExtractResult { programs })
    }
}

fn decode_files(files: &[MockFile]) -> Result<Vec<XciFile>, String> {
    let mut out = Vec::new();
    for file in files {
        let data = decode_b64(&file.name, &file.data_b64)?;
        out.push(XciFile {
            name: file.name.clone(),
            data,
        });
    }
    Ok(out)
}

fn decode_b64(label: &str, payload: &str) -> Result<Vec<u8>, String> {
    STANDARD
        .decode(payload)
        .map_err(|err| format!("invalid base64 for {label}: {err}"))
}
