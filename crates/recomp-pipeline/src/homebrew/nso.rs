use crate::homebrew::util::{hex_bytes, parse_error, read_bytes, read_u32};
use lz4_flex::block::decompress;
use sha2::{Digest, Sha256};
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
    /// SHA-256 of the decompressed segment, when the header asks for it to be checked.
    pub expected_sha256: Option<[u8; 32]>,
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

    // Header layout: magic (0x0), version (0x4), reserved (0x8), flags (0xC).
    let flags = read_u32(&bytes, 0xC).map_err(|err| err.to_string())?;
    let text = read_segment(&bytes, 0x10, NsoSegmentKind::Text)?;
    let rodata = read_segment(&bytes, 0x20, NsoSegmentKind::Rodata)?;
    let data = read_segment(&bytes, 0x30, NsoSegmentKind::Data)?;

    let file_sizes = [
        read_u32(&bytes, 0x60).map_err(|err| err.to_string())?,
        read_u32(&bytes, 0x64).map_err(|err| err.to_string())?,
        read_u32(&bytes, 0x68).map_err(|err| err.to_string())?,
    ];

    // Flag bits 0..2 mark text, rodata, data as LZ4-compressed; bits 3..5
    // ask for the segment hash at 0xA0 + 0x20 * index to be checked.
    let mut segments = vec![text, rodata, data];
    for (index, (segment, file_size)) in segments.iter_mut().zip(file_sizes).enumerate() {
        segment.file_size = file_size;
        segment.compressed = flags & (1 << index) != 0;
        if flags & (8 << index) != 0 {
            let hash = read_bytes(&bytes, 0xA0 + 0x20 * index, 0x20)
                .map_err(|err| err.to_string())?
                .try_into()
                .map_err(|_| "NSO segment hash length mismatch".to_string())?;
            segment.expected_sha256 = Some(hash);
        }
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
        if let Some(expected) = segment.expected_sha256 {
            let mut hasher = Sha256::new();
            hasher.update(&decoded);
            if hasher.finalize().as_slice() != expected {
                return Err(format!(
                    "NSO {:?} segment does not match the SHA-256 in the header",
                    segment.kind
                ));
            }
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
        expected_sha256: None,
        permissions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal NSO with LZ4-compressed text and hash checking for all three
    /// segments. `corrupt_hash` flips one byte of the rodata hash.
    fn write_nso(dir: &Path, corrupt_hash: bool) -> PathBuf {
        let segments: [&[u8]; 3] = [b"TEXTTEXTTEXTTEXT", b"RODATA", b"DATA"];
        let stored = [
            lz4_flex::block::compress(segments[0]),
            segments[1].to_vec(),
            segments[2].to_vec(),
        ];
        let mut bytes = vec![0u8; NSO_HEADER_SIZE];
        bytes[0..4].copy_from_slice(b"NSO0");
        // Flags live at 0xC: bit 0 = text compressed, bits 3..5 = check hashes.
        bytes[0xC..0x10].copy_from_slice(&0x39u32.to_le_bytes());
        let mut offset = NSO_HEADER_SIZE as u32;
        for (index, (plain, blob)) in segments.iter().zip(&stored).enumerate() {
            let entry = 0x10 + index * 0x10;
            bytes[entry..entry + 4].copy_from_slice(&offset.to_le_bytes());
            bytes[entry + 4..entry + 8].copy_from_slice(&(index as u32 * 0x1000).to_le_bytes());
            bytes[entry + 8..entry + 12].copy_from_slice(&(plain.len() as u32).to_le_bytes());
            let size_entry = 0x60 + index * 4;
            bytes[size_entry..size_entry + 4].copy_from_slice(&(blob.len() as u32).to_le_bytes());
            let mut hasher = Sha256::new();
            hasher.update(plain);
            let hash_entry = 0xA0 + index * 0x20;
            bytes[hash_entry..hash_entry + 0x20].copy_from_slice(&hasher.finalize());
            offset += blob.len() as u32;
        }
        if corrupt_hash {
            bytes[0xC0] ^= 0xFF;
        }
        for blob in &stored {
            bytes.extend_from_slice(blob);
        }
        let path = dir.join("module.nso");
        fs::write(&path, bytes).expect("write nso");
        path
    }

    #[test]
    fn segments_are_decompressed_and_match_the_header_hashes() {
        let temp = tempfile::tempdir().expect("tempdir");
        let module = parse_nso(&write_nso(temp.path(), false)).expect("parse");
        assert!(module.segments[0].compressed);
        assert!(!module.segments[1].compressed);
        assert!(module.segments.iter().all(|s| s.expected_sha256.is_some()));

        let segments = extract_segments(&module).expect("extract");
        assert_eq!(segments[0].data, b"TEXTTEXTTEXTTEXT");
        assert_eq!(segments[1].data, b"RODATA");
        assert_eq!(segments[2].data, b"DATA");
    }

    #[test]
    fn segment_hash_mismatch_is_rejected() {
        let temp = tempfile::tempdir().expect("tempdir");
        let module = parse_nso(&write_nso(temp.path(), true)).expect("parse");
        let err = extract_segments(&module).unwrap_err();
        assert!(err.contains("does not match the SHA-256"), "{err}");
    }
}
