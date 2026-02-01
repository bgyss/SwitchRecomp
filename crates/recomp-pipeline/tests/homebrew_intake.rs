use recomp_pipeline::homebrew::{intake_homebrew, IntakeOptions};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_u64(bytes: &mut [u8], offset: usize, value: u64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn build_nro(path: &Path, with_assets: bool) -> Vec<u8> {
    let header_size = 0x80usize;
    let text = b"TEXT";
    let rodata = b"RODT";
    let data = b"DATA";
    let text_off = header_size as u32;
    let ro_off = text_off + text.len() as u32;
    let data_off = ro_off + rodata.len() as u32;

    let nro_size = header_size + text.len() + rodata.len() + data.len();
    let mut bytes = vec![0u8; nro_size];

    bytes[0x10..0x14].copy_from_slice(b"NRO0");
    write_u32(&mut bytes, 0x18, nro_size as u32);
    write_u32(&mut bytes, 0x20, text_off);
    write_u32(&mut bytes, 0x24, text.len() as u32);
    write_u32(&mut bytes, 0x28, ro_off);
    write_u32(&mut bytes, 0x2C, rodata.len() as u32);
    write_u32(&mut bytes, 0x30, data_off);
    write_u32(&mut bytes, 0x34, data.len() as u32);
    write_u32(&mut bytes, 0x38, 0x20);

    let build_id = [0xABu8; 0x20];
    bytes[0x40..0x60].copy_from_slice(&build_id);

    bytes[text_off as usize..text_off as usize + text.len()].copy_from_slice(text);
    bytes[ro_off as usize..ro_off as usize + rodata.len()].copy_from_slice(rodata);
    bytes[data_off as usize..data_off as usize + data.len()].copy_from_slice(data);

    if with_assets {
        let asset_base = bytes.len();
        let icon = b"ICON";
        let nacp = vec![0x11u8; 0x4000];
        let romfs = b"ROMFS";
        let asset_header_size = 0x38usize;
        let icon_offset = asset_header_size as u64;
        let nacp_offset = icon_offset + icon.len() as u64;
        let romfs_offset = nacp_offset + nacp.len() as u64;
        let total = asset_base + asset_header_size + icon.len() + nacp.len() + romfs.len();
        bytes.resize(total, 0u8);

        bytes[asset_base..asset_base + 4].copy_from_slice(b"ASET");
        write_u64(&mut bytes, asset_base + 0x8, icon_offset);
        write_u64(&mut bytes, asset_base + 0x10, icon.len() as u64);
        write_u64(&mut bytes, asset_base + 0x18, nacp_offset);
        write_u64(&mut bytes, asset_base + 0x20, nacp.len() as u64);
        write_u64(&mut bytes, asset_base + 0x28, romfs_offset);
        write_u64(&mut bytes, asset_base + 0x30, romfs.len() as u64);

        let icon_start = asset_base + icon_offset as usize;
        bytes[icon_start..icon_start + icon.len()].copy_from_slice(icon);
        let nacp_start = asset_base + nacp_offset as usize;
        bytes[nacp_start..nacp_start + nacp.len()].copy_from_slice(&nacp);
        let romfs_start = asset_base + romfs_offset as usize;
        bytes[romfs_start..romfs_start + romfs.len()].copy_from_slice(romfs);
    }

    fs::write(path, &bytes).expect("write NRO");
    bytes
}

fn build_nso(path: &Path) -> Vec<u8> {
    let header_size = 0x100usize;
    let text = b"TEXTDATA";
    let rodata = b"RO";
    let data = b"DATA";
    let compressed_text = lz4_flex::block::compress(text);

    let text_off = header_size as u32;
    let ro_off = text_off + compressed_text.len() as u32;
    let data_off = ro_off + rodata.len() as u32;
    let total = header_size + compressed_text.len() + rodata.len() + data.len();
    let mut bytes = vec![0u8; total];

    bytes[0x0..0x4].copy_from_slice(b"NSO0");
    write_u32(&mut bytes, 0x8, 0x1);
    write_u32(&mut bytes, 0x10, text_off);
    write_u32(&mut bytes, 0x14, 0);
    write_u32(&mut bytes, 0x18, text.len() as u32);
    write_u32(&mut bytes, 0x20, ro_off);
    write_u32(&mut bytes, 0x24, 0x1000);
    write_u32(&mut bytes, 0x28, rodata.len() as u32);
    write_u32(&mut bytes, 0x30, data_off);
    write_u32(&mut bytes, 0x34, 0x2000);
    write_u32(&mut bytes, 0x38, data.len() as u32);
    write_u32(&mut bytes, 0x3C, 0x40);

    let module_id = [0xCDu8; 0x20];
    bytes[0x40..0x60].copy_from_slice(&module_id);
    write_u32(&mut bytes, 0x60, compressed_text.len() as u32);
    write_u32(&mut bytes, 0x64, rodata.len() as u32);
    write_u32(&mut bytes, 0x68, data.len() as u32);

    bytes[text_off as usize..text_off as usize + compressed_text.len()]
        .copy_from_slice(&compressed_text);
    let ro_start = ro_off as usize;
    bytes[ro_start..ro_start + rodata.len()].copy_from_slice(rodata);
    let data_start = data_off as usize;
    bytes[data_start..data_start + data.len()].copy_from_slice(data);

    fs::write(path, &bytes).expect("write NSO");
    bytes
}

fn write_provenance(path: &Path, entries: Vec<(PathBuf, &str, &[u8])>) {
    let mut inputs = String::new();
    for (entry_path, format, bytes) in entries {
        let sha = sha256_hex(bytes);
        let size = bytes.len();
        inputs.push_str(&format!(
            "[[inputs]]\npath = \"{}\"\nsha256 = \"{}\"\nsize = {}\nformat = \"{}\"\n\n",
            entry_path.display(),
            sha,
            size,
            format
        ));
    }

    let toml = format!(
        "schema_version = \"1\"\n\n[title]\nname = \"Test\"\ntitle_id = \"0100000000000000\"\nversion = \"1.0.0\"\nregion = \"US\"\n\n[collection]\ndevice = \"Switch\"\ncollected_at = \"2024-01-01\"\n\n[collection.tool]\nname = \"collector\"\nversion = \"0.1\"\n\n{}",
        inputs
    );
    fs::write(path, toml).expect("write provenance");
}

#[test]
fn intake_homebrew_extracts_assets_and_segments() {
    let dir = tempdir().expect("tempdir");
    let nro_path = dir.path().join("main.nro");
    let nro_bytes = build_nro(&nro_path, true);
    let provenance_path = dir.path().join("provenance.toml");
    write_provenance(
        &provenance_path,
        vec![(nro_path.clone(), "nro0", &nro_bytes)],
    );

    let out_dir = dir.path().join("out");
    let report = intake_homebrew(IntakeOptions {
        module_path: nro_path,
        nso_paths: Vec::new(),
        provenance_path,
        out_dir: out_dir.clone(),
    })
    .expect("intake homebrew");

    assert!(report.module_json_path.exists());
    assert!(report.manifest_path.exists());
    assert!(out_dir.join("segments/main/text.bin").exists());
    assert!(out_dir.join("assets/icon.bin").exists());
    assert!(out_dir.join("assets/control.nacp").exists());
    assert!(out_dir.join("assets/romfs/romfs.bin").exists());

    let manifest = fs::read_to_string(report.manifest_path).expect("read manifest");
    assert!(manifest.contains("control.nacp"));
    assert!(manifest.contains("romfs/romfs.bin"));
}

#[test]
fn intake_homebrew_handles_nso_segments() {
    let dir = tempdir().expect("tempdir");
    let nro_path = dir.path().join("main.nro");
    let nro_bytes = build_nro(&nro_path, false);
    let nso_path = dir.path().join("mod.nso");
    let nso_bytes = build_nso(&nso_path);

    let provenance_path = dir.path().join("provenance.toml");
    write_provenance(
        &provenance_path,
        vec![
            (nro_path.clone(), "nro0", &nro_bytes),
            (nso_path.clone(), "nso0", &nso_bytes),
        ],
    );

    let out_dir = dir.path().join("out");
    let report = intake_homebrew(IntakeOptions {
        module_path: nro_path.clone(),
        nso_paths: vec![nso_path.clone()],
        provenance_path,
        out_dir: out_dir.clone(),
    })
    .expect("intake homebrew");

    let nso_text = fs::read(out_dir.join("segments/mod/text.bin")).expect("read text");
    let nso_data = fs::read(out_dir.join("segments/mod/data.bin")).expect("read data");
    assert_eq!(nso_text, b"TEXTDATA");
    assert_eq!(nso_data, b"DATA");

    let module_json = fs::read_to_string(report.module_json_path).expect("read module.json");
    assert!(module_json.contains("\"format\": \"nso\""));
}
