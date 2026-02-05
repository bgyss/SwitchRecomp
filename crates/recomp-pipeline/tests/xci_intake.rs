use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use recomp_pipeline::xci::{
    check_intake_manifest, intake_xci, XciIntakeOptions, XciToolPreference,
};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write;
        let _ = write!(&mut out, "{byte:02x}");
    }
    out
}

fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_u64(bytes: &mut [u8], offset: usize, value: u64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn align_up(value: usize, align: usize) -> usize {
    value.div_ceil(align) * align
}

fn build_romfs_image() -> Vec<u8> {
    let file_root = b"HELLO";
    let file_nested = b"NESTED";
    let nested_dir = "data";
    let root_name = "";

    let root_entry_size = align_up(0x18 + root_name.len(), 4);
    let nested_entry_off = root_entry_size as u32;
    let nested_entry_size = align_up(0x18 + nested_dir.len(), 4);
    let dir_table_size = root_entry_size + nested_entry_size;

    let file_root_name = "hello.txt";
    let file_nested_name = "nested.bin";
    let file_root_entry_size = align_up(0x20 + file_root_name.len(), 4);
    let file_nested_off = file_root_entry_size as u32;
    let file_nested_entry_size = align_up(0x20 + file_nested_name.len(), 4);
    let file_table_size = file_root_entry_size + file_nested_entry_size;

    let file_root_data_off = 0u64;
    let file_nested_data_off = align_up(file_root.len(), 0x10) as u64;
    let mut file_data = Vec::new();
    file_data.extend_from_slice(file_root);
    let padding = align_up(file_data.len(), 0x10) - file_data.len();
    file_data.extend(std::iter::repeat_n(0u8, padding));
    file_data.extend_from_slice(file_nested);

    let mut dir_table = Vec::new();
    push_dir_entry(
        &mut dir_table,
        0xFFFF_FFFF,
        0xFFFF_FFFF,
        nested_entry_off,
        0,
        0xFFFF_FFFF,
        root_name,
    );
    push_dir_entry(
        &mut dir_table,
        0,
        0xFFFF_FFFF,
        0xFFFF_FFFF,
        file_nested_off,
        0xFFFF_FFFF,
        nested_dir,
    );

    let mut file_table = Vec::new();
    push_file_entry(
        &mut file_table,
        0,
        0xFFFF_FFFF,
        file_root_data_off,
        file_root.len() as u64,
        0xFFFF_FFFF,
        file_root_name,
    );
    push_file_entry(
        &mut file_table,
        nested_entry_off,
        0xFFFF_FFFF,
        file_nested_data_off,
        file_nested.len() as u64,
        0xFFFF_FFFF,
        file_nested_name,
    );

    let header_size = 0x50usize;
    let dir_table_off = align_up(header_size, 0x10);
    let file_table_off = align_up(dir_table_off + dir_table_size, 0x10);
    let file_data_off = align_up(file_table_off + file_table_size, 0x10);
    let total_size = file_data_off + file_data.len();

    let mut image = vec![0u8; total_size];
    write_u64(&mut image, 0x0, 0x50);
    write_u64(&mut image, 0x8, dir_table_off as u64);
    write_u64(&mut image, 0x10, 0);
    write_u64(&mut image, 0x18, dir_table_off as u64);
    write_u64(&mut image, 0x20, dir_table_size as u64);
    write_u64(&mut image, 0x28, file_table_off as u64);
    write_u64(&mut image, 0x30, 0);
    write_u64(&mut image, 0x38, file_table_off as u64);
    write_u64(&mut image, 0x40, file_table_size as u64);
    write_u64(&mut image, 0x48, file_data_off as u64);

    image[dir_table_off..dir_table_off + dir_table_size].copy_from_slice(&dir_table);
    image[file_table_off..file_table_off + file_table_size].copy_from_slice(&file_table);
    image[file_data_off..file_data_off + file_data.len()].copy_from_slice(&file_data);

    image
}

fn push_dir_entry(
    buf: &mut Vec<u8>,
    parent: u32,
    sibling: u32,
    child_dir: u32,
    child_file: u32,
    next_hash: u32,
    name: &str,
) -> u32 {
    let offset = buf.len() as u32;
    buf.extend_from_slice(&parent.to_le_bytes());
    buf.extend_from_slice(&sibling.to_le_bytes());
    buf.extend_from_slice(&child_dir.to_le_bytes());
    buf.extend_from_slice(&child_file.to_le_bytes());
    buf.extend_from_slice(&next_hash.to_le_bytes());
    buf.extend_from_slice(&(name.len() as u32).to_le_bytes());
    buf.extend_from_slice(name.as_bytes());
    while buf.len() % 4 != 0 {
        buf.push(0);
    }
    offset
}

fn push_file_entry(
    buf: &mut Vec<u8>,
    parent: u32,
    sibling: u32,
    data_off: u64,
    data_size: u64,
    next_hash: u32,
    name: &str,
) -> u32 {
    let offset = buf.len() as u32;
    buf.extend_from_slice(&parent.to_le_bytes());
    buf.extend_from_slice(&sibling.to_le_bytes());
    buf.extend_from_slice(&data_off.to_le_bytes());
    buf.extend_from_slice(&data_size.to_le_bytes());
    buf.extend_from_slice(&next_hash.to_le_bytes());
    buf.extend_from_slice(&(name.len() as u32).to_le_bytes());
    buf.extend_from_slice(name.as_bytes());
    while buf.len() % 4 != 0 {
        buf.push(0);
    }
    offset
}

fn build_nso() -> Vec<u8> {
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
        "schema_version = \"1\"\n\n[title]\nname = \"Test\"\ntitle_id = \"0100000000000000\"\nversion = \"1.0.0\"\nregion = \"US\"\n\n[collection]\ndevice = \"Switch\"\ncollected_at = \"2026-02-03\"\n\n[collection.tool]\nname = \"collector\"\nversion = \"0.1\"\n\n{}",
        inputs
    );
    fs::write(path, toml).expect("write provenance");
}

fn build_mock_xci_json(nso: &[u8], romfs: &[u8]) -> String {
    let nca_bytes = b"NCA3";
    let program = serde_json::json!({
        "title_id": "0100000000000000",
        "content_type": "program",
        "version": "1.0.0",
        "nca": { "data_b64": STANDARD.encode(nca_bytes) },
        "exefs": [
            { "name": "main", "data_b64": STANDARD.encode(nso) },
            { "name": "main.npdm", "data_b64": STANDARD.encode(b"NPDM") }
        ],
        "nso": [
            { "name": "main", "data_b64": STANDARD.encode(nso) }
        ]
    });
    let image = serde_json::json!({
        "schema_version": "1",
        "programs": [program],
        "romfs": { "image_b64": STANDARD.encode(romfs) }
    });
    serde_json::to_string(&image).expect("serialize mock xci")
}

#[test]
fn intake_xci_emits_manifest_and_assets() {
    let dir = tempdir().expect("tempdir");
    let xci_path = dir.path().join("sample.xci");
    let keys_path = dir.path().join("title.keys");
    fs::write(&keys_path, b"DUMMYKEYS").expect("write keys");

    let nso_bytes = build_nso();
    let romfs_bytes = build_romfs_image();
    let xci_json = build_mock_xci_json(&nso_bytes, &romfs_bytes);
    fs::write(&xci_path, xci_json.as_bytes()).expect("write xci");

    let provenance_path = dir.path().join("provenance.toml");
    write_provenance(
        &provenance_path,
        vec![
            (xci_path.clone(), "xci", xci_json.as_bytes()),
            (keys_path.clone(), "keyset", b"DUMMYKEYS"),
        ],
    );

    let out_dir = dir.path().join("out");
    let assets_dir = dir.path().join("assets");
    let report = intake_xci(XciIntakeOptions {
        xci_path,
        keys_path,
        config_path: None,
        provenance_path,
        out_dir: out_dir.clone(),
        assets_dir: assets_dir.clone(),
        tool_preference: XciToolPreference::Mock,
        tool_path: None,
    })
    .expect("intake xci");

    assert!(report.module_json_path.exists());
    assert!(report.manifest_path.exists());
    assert!(out_dir.join("exefs/main").exists());
    assert!(out_dir.join("segments/main/text.bin").exists());
    assert!(assets_dir.join("romfs/hello.txt").exists());

    let manifest_src = fs::read_to_string(report.manifest_path).expect("read manifest");
    let manifest: serde_json::Value = serde_json::from_str(&manifest_src).expect("parse manifest");
    let assets_root = manifest
        .get("assets_root")
        .and_then(|value| value.as_str())
        .expect("assets_root string");
    assert!(assets_root.contains("assets"));

    let check = check_intake_manifest(&out_dir.join("manifest.json")).expect("check manifest");
    assert!(check.missing_files.is_empty());
}

#[test]
fn intake_xci_rejects_ambiguous_program() {
    let dir = tempdir().expect("tempdir");
    let xci_path = dir.path().join("sample.xci");
    let keys_path = dir.path().join("title.keys");
    fs::write(&keys_path, b"DUMMYKEYS").expect("write keys");

    let nso_bytes = build_nso();
    let program_one = serde_json::json!({
        "title_id": "0100000000000000",
        "content_type": "program",
        "version": "1.0.0",
        "nca": { "data_b64": STANDARD.encode(b"NCA3") },
        "exefs": [{ "name": "main", "data_b64": STANDARD.encode(&nso_bytes) }],
        "nso": [{ "name": "main", "data_b64": STANDARD.encode(&nso_bytes) }]
    });
    let program_two = serde_json::json!({
        "title_id": "0100000000000001",
        "content_type": "program",
        "version": "1.0.0",
        "nca": { "data_b64": STANDARD.encode(b"NCA3") },
        "exefs": [{ "name": "main", "data_b64": STANDARD.encode(&nso_bytes) }],
        "nso": [{ "name": "main", "data_b64": STANDARD.encode(&nso_bytes) }]
    });
    let image = serde_json::json!({
        "schema_version": "1",
        "programs": [program_one, program_two]
    });
    let xci_json = serde_json::to_string(&image).expect("serialize mock xci");
    fs::write(&xci_path, xci_json.as_bytes()).expect("write xci");

    let provenance_path = dir.path().join("provenance.toml");
    write_provenance(
        &provenance_path,
        vec![
            (xci_path.clone(), "xci", xci_json.as_bytes()),
            (keys_path.clone(), "keyset", b"DUMMYKEYS"),
        ],
    );

    let out_dir = dir.path().join("out");
    let assets_dir = dir.path().join("assets");
    let err = intake_xci(XciIntakeOptions {
        xci_path,
        keys_path,
        config_path: None,
        provenance_path,
        out_dir,
        assets_dir,
        tool_preference: XciToolPreference::Mock,
        tool_path: None,
    })
    .expect_err("ambiguous program should fail");
    assert!(err.contains("ambiguous Program NCA selection"));
}

#[test]
fn intake_xci_rejects_nested_assets_dir() {
    let dir = tempdir().expect("tempdir");
    let xci_path = dir.path().join("sample.xci");
    let keys_path = dir.path().join("title.keys");
    fs::write(&keys_path, b"DUMMYKEYS").expect("write keys");

    let xci_json = build_mock_xci_json(&build_nso(), &build_romfs_image());
    fs::write(&xci_path, xci_json.as_bytes()).expect("write xci");

    let provenance_path = dir.path().join("provenance.toml");
    write_provenance(
        &provenance_path,
        vec![
            (xci_path.clone(), "xci", xci_json.as_bytes()),
            (keys_path.clone(), "keyset", b"DUMMYKEYS"),
        ],
    );

    let out_dir = dir.path().join("out");
    let assets_dir = out_dir.join("assets");
    let err = intake_xci(XciIntakeOptions {
        xci_path,
        keys_path,
        config_path: None,
        provenance_path,
        out_dir,
        assets_dir,
        tool_preference: XciToolPreference::Mock,
        tool_path: None,
    })
    .expect_err("nested assets_dir should fail");
    assert!(err.contains("assets_dir must not be inside out_dir"));
}
