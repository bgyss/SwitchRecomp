use crate::homebrew::util::{find_magic, hex_bytes, read_bytes, read_u32, read_u64};
use std::fs;
use std::path::{Path, PathBuf};

const NRO_MAGIC: u32 = 0x304F524E; // "NRO0"
const NRO_HEADER_MAGIC_OFFSET: usize = 0x10;
const NRO_HEADER_MIN_SIZE: usize = 0x80;
const ASSET_MAGIC: u32 = 0x54455341; // "ASET"

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NroSegmentPermissions {
    Rx,
    R,
    Rw,
}

impl NroSegmentPermissions {
    pub fn as_str(self) -> &'static str {
        match self {
            NroSegmentPermissions::Rx => "r-x",
            NroSegmentPermissions::R => "r--",
            NroSegmentPermissions::Rw => "rw-",
        }
    }
}

#[derive(Debug, Clone)]
pub struct NroSegment {
    pub name: String,
    pub file_offset: u32,
    pub size: u32,
    pub memory_offset: u32,
    pub permissions: NroSegmentPermissions,
}

#[derive(Debug, Clone, Copy)]
pub struct NroAssetSection {
    pub offset: u64,
    pub size: u64,
}

#[derive(Debug, Clone)]
pub struct NroAssetHeader {
    pub icon: NroAssetSection,
    pub nacp: NroAssetSection,
    pub romfs: NroAssetSection,
    pub base_offset: u64,
}

#[derive(Debug, Clone)]
pub struct NroModule {
    pub path: PathBuf,
    pub size: u32,
    pub segments: Vec<NroSegment>,
    pub bss_size: u32,
    pub build_id: [u8; 0x20],
    pub assets: Option<NroAssetHeader>,
}

impl NroModule {
    pub fn build_id_hex(&self) -> String {
        hex_bytes(&self.build_id)
    }
}

pub fn parse_nro(path: &Path) -> Result<NroModule, String> {
    let bytes = fs::read(path).map_err(|err| format!("read NRO {}: {err}", path.display()))?;
    let magic_offset =
        find_magic(&bytes, NRO_MAGIC, 0x80).ok_or_else(|| "NRO magic not found".to_string())?;
    let header_start = magic_offset
        .checked_sub(NRO_HEADER_MAGIC_OFFSET)
        .ok_or_else(|| "NRO header offset underflow".to_string())?;
    if bytes.len() < header_start + NRO_HEADER_MIN_SIZE {
        return Err("NRO header truncated".to_string());
    }

    let size = read_u32(&bytes, header_start + 0x18).map_err(|err| err.to_string())?;
    let text_mem_offset = read_u32(&bytes, header_start + 0x20).map_err(|err| err.to_string())?;
    let text_size = read_u32(&bytes, header_start + 0x24).map_err(|err| err.to_string())?;
    let ro_mem_offset = read_u32(&bytes, header_start + 0x28).map_err(|err| err.to_string())?;
    let ro_size = read_u32(&bytes, header_start + 0x2C).map_err(|err| err.to_string())?;
    let data_mem_offset = read_u32(&bytes, header_start + 0x30).map_err(|err| err.to_string())?;
    let data_size = read_u32(&bytes, header_start + 0x34).map_err(|err| err.to_string())?;
    let bss_size = read_u32(&bytes, header_start + 0x38).map_err(|err| err.to_string())?;
    let build_id = read_bytes(&bytes, header_start + 0x40, 0x20)
        .map_err(|err| err.to_string())?
        .try_into()
        .map_err(|_| "NRO build id length mismatch".to_string())?;

    let segments = match parse_segments_libnx(&bytes, magic_offset) {
        Some(mut segments) => {
            if segments.iter().all(|seg| {
                let end = seg.file_offset.saturating_add(seg.size);
                end as usize <= bytes.len()
            }) {
                segments[0].memory_offset = text_mem_offset;
                segments[1].memory_offset = ro_mem_offset;
                segments[2].memory_offset = data_mem_offset;
                segments
            } else {
                synthesize_segments(
                    header_start,
                    text_mem_offset,
                    text_size,
                    ro_mem_offset,
                    ro_size,
                    data_mem_offset,
                    data_size,
                )
            }
        }
        None => synthesize_segments(
            header_start,
            text_mem_offset,
            text_size,
            ro_mem_offset,
            ro_size,
            data_mem_offset,
            data_size,
        ),
    };

    let assets = parse_assets(&bytes, size as usize);

    Ok(NroModule {
        path: path.to_path_buf(),
        size,
        segments,
        bss_size,
        build_id,
        assets,
    })
}

fn parse_segments_libnx(bytes: &[u8], magic_offset: usize) -> Option<Vec<NroSegment>> {
    let mut segments = Vec::new();
    let offsets = [0x10, 0x18, 0x20];
    let names = ["text", "rodata", "data"];
    let perms = [
        NroSegmentPermissions::Rx,
        NroSegmentPermissions::R,
        NroSegmentPermissions::Rw,
    ];
    for ((offset, name), perm) in offsets.iter().zip(names.iter()).zip(perms.iter()) {
        let file_offset = read_u32(bytes, magic_offset + offset).ok()?;
        let size = read_u32(bytes, magic_offset + offset + 0x4).ok()?;
        segments.push(NroSegment {
            name: name.to_string(),
            file_offset,
            size,
            memory_offset: 0,
            permissions: *perm,
        });
    }
    Some(segments)
}

fn synthesize_segments(
    header_start: usize,
    text_mem_offset: u32,
    text_size: u32,
    ro_mem_offset: u32,
    ro_size: u32,
    data_mem_offset: u32,
    data_size: u32,
) -> Vec<NroSegment> {
    let text_file_offset = (header_start + NRO_HEADER_MIN_SIZE) as u32;
    let ro_file_offset = text_file_offset.saturating_add(text_size);
    let data_file_offset = ro_file_offset.saturating_add(ro_size);

    vec![
        NroSegment {
            name: "text".to_string(),
            file_offset: text_file_offset,
            size: text_size,
            memory_offset: text_mem_offset,
            permissions: NroSegmentPermissions::Rx,
        },
        NroSegment {
            name: "rodata".to_string(),
            file_offset: ro_file_offset,
            size: ro_size,
            memory_offset: ro_mem_offset,
            permissions: NroSegmentPermissions::R,
        },
        NroSegment {
            name: "data".to_string(),
            file_offset: data_file_offset,
            size: data_size,
            memory_offset: data_mem_offset,
            permissions: NroSegmentPermissions::Rw,
        },
    ]
}

fn parse_assets(bytes: &[u8], offset: usize) -> Option<NroAssetHeader> {
    if offset + 0x38 > bytes.len() {
        return None;
    }
    let magic = read_u32(bytes, offset).ok()?;
    if magic != ASSET_MAGIC {
        return None;
    }
    let icon_offset = read_u64(bytes, offset + 0x8).ok()?;
    let icon_size = read_u64(bytes, offset + 0x10).ok()?;
    let nacp_offset = read_u64(bytes, offset + 0x18).ok()?;
    let nacp_size = read_u64(bytes, offset + 0x20).ok()?;
    let romfs_offset = read_u64(bytes, offset + 0x28).ok()?;
    let romfs_size = read_u64(bytes, offset + 0x30).ok()?;

    Some(NroAssetHeader {
        icon: NroAssetSection {
            offset: icon_offset,
            size: icon_size,
        },
        nacp: NroAssetSection {
            offset: nacp_offset,
            size: nacp_size,
        },
        romfs: NroAssetSection {
            offset: romfs_offset,
            size: romfs_size,
        },
        base_offset: offset as u64,
    })
}
