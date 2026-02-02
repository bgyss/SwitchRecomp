use crate::homebrew::util::{hex_bytes, parse_error, read_bytes, read_u32};
use lz4_flex::block::decompress;
use std::fs;
use std::path::{Path, PathBuf};

const NSO_MAGIC: u32 = 0x304F534E; // "NSO0"
const NSO_HEADER_SIZE: usize = 0x100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NsoSegmentKind {
    Text,
    Rodata,
    Data,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NsoSegmentPermissions {
    Rx,
    R,
    Rw,
}

impl NsoSegmentPermissions {
    pub fn as_str(self) -> &'static str {
        match self {
            NsoSegmentPermissions::Rx => "r-x",
            NsoSegmentPermissions::R => "r--",
            NsoSegmentPermissions::Rw => "rw-",
        }
    }
}

#[derive(Debug, Clone)]
pub struct NsoSegment {
    pub kind: NsoSegmentKind,
    pub file_offset: u32,
    pub memory_offset: u32,
    pub size: u32,
    pub file_size: u32,
    pub compressed: bool,
    pub permissions: NsoSegmentPermissions,
}

#[derive(Debug, Clone)]
pub struct NsoModule {
    pub path: PathBuf,
    pub size: u64,
    pub segments: Vec<NsoSegment>,
    pub bss_size: u32,
    pub module_id: [u8; 32],
    pub embedded_offset: u32,
    pub embedded_size: u32,
    pub dynstr_offset: u32,
    pub dynstr_size: u32,
    pub dynsym_offset: u32,
    pub dynsym_size: u32,
}

impl NsoModule {
    pub fn module_id_hex(&self) -> String {
        hex_bytes(&self.module_id)
    }
}

#[derive(Debug, Clone)]
pub struct NsoSegmentData {
    pub segment: NsoSegment,
    pub data: Vec<u8>,
}

pub fn parse_nso(path: &Path) -> Result<NsoModule, String> {
    let bytes = fs::read(path).map_err(|err| format!("read NSO {}: {err}", path.display()))?;
    if bytes.len() < NSO_HEADER_SIZE {
        return Err(format!("NSO too small: {} bytes", bytes.len()));
    }
    let magic = read_u32(&bytes, 0).map_err(|err| err.to_string())?;
    if magic != NSO_MAGIC {
        return Err(format!("NSO magic mismatch: {magic:#x}"));
    }

    let flags = read_u32(&bytes, 0x8).map_err(|err| err.to_string())?;
    let text = read_segment(&bytes, 0x10, NsoSegmentKind::Text)?;
    let rodata = read_segment(&bytes, 0x20, NsoSegmentKind::Rodata)?;
    let data = read_segment(&bytes, 0x30, NsoSegmentKind::Data)?;

    let file_sizes = [
        read_u32(&bytes, 0x60).map_err(|err| err.to_string())?,
        read_u32(&bytes, 0x64).map_err(|err| err.to_string())?,
        read_u32(&bytes, 0x68).map_err(|err| err.to_string())?,
    ];

    let mut segments = vec![text, rodata, data];
    for (segment, file_size) in segments.iter_mut().zip(file_sizes) {
        segment.file_size = file_size;
        segment.compressed = match segment.kind {
            NsoSegmentKind::Text => flags & 0x1 != 0,
            NsoSegmentKind::Rodata => flags & 0x2 != 0,
            NsoSegmentKind::Data => flags & 0x4 != 0,
        };
    }

    let bss_size = read_u32(&bytes, 0x3C).map_err(|err| err.to_string())?;
    let embedded_offset = read_u32(&bytes, 0x70).map_err(|err| err.to_string())?;
    let embedded_size = read_u32(&bytes, 0x74).map_err(|err| err.to_string())?;
    let dynstr_offset = read_u32(&bytes, 0x78).map_err(|err| err.to_string())?;
    let dynstr_size = read_u32(&bytes, 0x7C).map_err(|err| err.to_string())?;
    let dynsym_offset = read_u32(&bytes, 0x80).map_err(|err| err.to_string())?;
    let dynsym_size = read_u32(&bytes, 0x84).map_err(|err| err.to_string())?;

    let module_id = read_bytes(&bytes, 0x40, 0x20)
        .map_err(|err| err.to_string())?
        .try_into()
        .map_err(|_| "NSO module id length mismatch".to_string())?;

    Ok(NsoModule {
        path: path.to_path_buf(),
        size: bytes.len() as u64,
        segments,
        bss_size,
        module_id,
        embedded_offset,
        embedded_size,
        dynstr_offset,
        dynstr_size,
        dynsym_offset,
        dynsym_size,
    })
}

pub fn extract_segments(module: &NsoModule) -> Result<Vec<NsoSegmentData>, String> {
    let bytes = fs::read(&module.path)
        .map_err(|err| format!("read NSO {}: {err}", module.path.display()))?;
    let mut out = Vec::new();
    for segment in &module.segments {
        let start = segment.file_offset as usize;
        let file_size = segment.file_size as usize;
        let end = start
            .checked_add(file_size)
            .ok_or_else(|| "segment offset overflow".to_string())?;
        if end > bytes.len() {
            return Err(format!(
                "NSO segment out of range: {}..{} for {}",
                start,
                end,
                module.path.display()
            ));
        }
        let data = &bytes[start..end];
        let decoded = if segment.compressed {
            decompress(data, segment.size as usize)
                .map_err(|err| parse_error(format!("lz4 decode failed: {err}")))
                .map_err(|err| err.to_string())?
        } else {
            data.to_vec()
        };
        if decoded.len() != segment.size as usize {
            return Err(format!(
                "NSO segment size mismatch: expected {}, got {}",
                segment.size,
                decoded.len()
            ));
        }
        out.push(NsoSegmentData {
            segment: segment.clone(),
            data: decoded,
        });
    }
    Ok(out)
}

fn read_segment(bytes: &[u8], offset: usize, kind: NsoSegmentKind) -> Result<NsoSegment, String> {
    let file_offset = read_u32(bytes, offset).map_err(|err| err.to_string())?;
    let memory_offset = read_u32(bytes, offset + 0x4).map_err(|err| err.to_string())?;
    let size = read_u32(bytes, offset + 0x8).map_err(|err| err.to_string())?;
    let permissions = match kind {
        NsoSegmentKind::Text => NsoSegmentPermissions::Rx,
        NsoSegmentKind::Rodata => NsoSegmentPermissions::R,
        NsoSegmentKind::Data => NsoSegmentPermissions::Rw,
    };

    Ok(NsoSegment {
        kind,
        file_offset,
        memory_offset,
        size,
        file_size: 0,
        compressed: false,
        permissions,
    })
}
