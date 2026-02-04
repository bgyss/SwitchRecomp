use recomp_pipeline::xci::{intake_xci, IntakeOptions, ProgramSelection, ToolKind};
use sha2::{Digest, Sha256};
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

#[test]
fn xci_intake_emits_outputs() {
    let temp = tempfile::tempdir().expect("tempdir");
    let xci_path = temp.path().join("sample.xci");
    let keys_path = temp.path().join("prod.keys");
    let provenance_path = temp.path().join("provenance.toml");
    let out_dir = temp.path().join("out");

    let title_id = 0x0100000000000000u64;
    let xci_bytes = build_xci(vec![build_program_entry("program.nca", title_id, 1)]);
    fs::write(&xci_path, &xci_bytes).expect("write xci");
    fs::write(&keys_path, "header_key = 0123456789abcdef0123456789abcdef").expect("write keys");

    write_provenance(&provenance_path, &xci_path, &keys_path);

    let report = intake_xci(IntakeOptions {
        xci_path: xci_path.clone(),
        keys_path: keys_path.clone(),
        provenance_path: provenance_path.clone(),
        out_dir: out_dir.clone(),
        program: ProgramSelection::TitleId(format!("{title_id:x}")),
        tool_path: None,
        tool_kind: ToolKind::Auto,
        title_keys_path: None,
    })
    .expect("intake");

    assert!(report.module_json_path.exists());
    assert!(report.manifest_path.exists());
    assert!(out_dir.join("intake/exefs/main").exists());
    assert!(out_dir.join("intake/segments/main-text.bin").exists());
    assert!(out_dir.join("assets/romfs.bin").exists());

    let manifest_src = fs::read_to_string(out_dir.join("manifest.json")).expect("manifest");
    let manifest: serde_json::Value = serde_json::from_str(&manifest_src).expect("parse manifest");
    let generated = manifest
        .get("generated_files")
        .and_then(|value| value.as_array())
        .expect("generated_files");
    assert!(generated.iter().any(|entry| {
        entry.get("path").and_then(|value| value.as_str()) == Some("assets/romfs.bin")
    }));
}

#[test]
fn xci_intake_rejects_invalid_keyset() {
    let temp = tempfile::tempdir().expect("tempdir");
    let xci_path = temp.path().join("sample.xci");
    let keys_path = temp.path().join("prod.keys");
    let provenance_path = temp.path().join("provenance.toml");
    let out_dir = temp.path().join("out");

    let title_id = 0x0100000000000000u64;
    let xci_bytes = build_xci(vec![build_program_entry("program.nca", title_id, 1)]);
    fs::write(&xci_path, &xci_bytes).expect("write xci");
    fs::write(&keys_path, "invalid = nothex").expect("write keys");

    write_provenance(&provenance_path, &xci_path, &keys_path);

    let err = intake_xci(IntakeOptions {
        xci_path,
        keys_path,
        provenance_path,
        out_dir,
        program: ProgramSelection::TitleId(format!("{title_id:x}")),
        tool_path: None,
        tool_kind: ToolKind::Auto,
        title_keys_path: None,
    })
    .expect_err("intake should fail");

    assert!(err.contains("keyset"));
}

#[test]
fn xci_intake_rejects_ambiguous_program_selection() {
    let temp = tempfile::tempdir().expect("tempdir");
    let xci_path = temp.path().join("sample.xci");
    let keys_path = temp.path().join("prod.keys");
    let provenance_path = temp.path().join("provenance.toml");
    let out_dir = temp.path().join("out");

    let title_id = 0x0100000000000000u64;
    let xci_bytes = build_xci(vec![
        build_program_entry("program-a.nca", title_id, 1),
        build_program_entry("program-b.nca", title_id, 2),
    ]);
    fs::write(&xci_path, &xci_bytes).expect("write xci");
    fs::write(&keys_path, "header_key = 0123456789abcdef0123456789abcdef").expect("write keys");

    write_provenance(&provenance_path, &xci_path, &keys_path);

    let err = intake_xci(IntakeOptions {
        xci_path,
        keys_path,
        provenance_path,
        out_dir,
        program: ProgramSelection::TitleId(format!("{title_id:x}")),
        tool_path: None,
        tool_kind: ToolKind::Auto,
        title_keys_path: None,
    })
    .expect_err("intake should fail");

    assert!(err.contains("ambiguous"));
}

#[test]
#[cfg(unix)]
fn xci_intake_uses_external_tool() {
    let temp = tempfile::tempdir().expect("tempdir");
    let xci_path = temp.path().join("real.xci");
    let keys_path = temp.path().join("prod.keys");
    let provenance_path = temp.path().join("provenance.toml");
    let out_dir = temp.path().join("out");
    let fake_tool = temp.path().join("fake-hactool.sh");
    let fake_nca = temp.path().join("program.nca");
    let fake_nso = temp.path().join("main.nso");

    fs::write(&xci_path, b"REALX").expect("write xci");
    fs::write(&keys_path, "header_key = 0123456789abcdef0123456789abcdef").expect("write keys");
    fs::write(&fake_nca, b"NCA3FAKE").expect("write nca");
    fs::write(&fake_nso, build_nso()).expect("write nso");

    std::env::set_var("RECOMP_FAKE_NCA", &fake_nca);
    std::env::set_var("RECOMP_FAKE_NSO", &fake_nso);

    let script = r#"#!/bin/sh
set -e
outdir=""
exefsdir=""
romfs=""
info=0
while [ $# -gt 0 ]; do
  case "$1" in
    --outdir) outdir="$2"; shift 2;;
    --outdir=*) outdir="${1#--outdir=}"; shift;;
    --exefsdir) exefsdir="$2"; shift 2;;
    --exefsdir=*) exefsdir="${1#--exefsdir=}"; shift;;
    --romfs) romfs="$2"; shift 2;;
    --romfs=*) romfs="${1#--romfs=}"; shift;;
    -i) info=1; shift;;
    *) shift;;
  esac
done

if [ "$info" = "1" ]; then
  echo "Title ID: 0100000000000000"
  echo "Content Type: Program"
  echo "Version: 1"
  exit 0
fi

if [ -n "$outdir" ]; then
  mkdir -p "$outdir/secure"
  cp "$RECOMP_FAKE_NCA" "$outdir/secure/program.nca"
  exit 0
fi

if [ -n "$exefsdir" ]; then
  mkdir -p "$exefsdir"
  cp "$RECOMP_FAKE_NSO" "$exefsdir/main"
  printf "NPDM" > "$exefsdir/main.npdm"
  if [ -n "$romfs" ]; then
    printf "ROMFS" > "$romfs"
  fi
  exit 0
fi

exit 0
"#;

    fs::write(&fake_tool, script).expect("write fake tool");
    let mut perms = fs::metadata(&fake_tool).expect("metadata").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&fake_tool, perms).expect("chmod");

    write_provenance(&provenance_path, &xci_path, &keys_path);

    let report = intake_xci(IntakeOptions {
        xci_path,
        keys_path,
        provenance_path,
        out_dir: out_dir.clone(),
        program: ProgramSelection::TitleId("0100000000000000".to_string()),
        tool_path: Some(fake_tool),
        tool_kind: ToolKind::Hactool,
        title_keys_path: None,
    })
    .expect("intake");

    assert!(report.module_json_path.exists());
    assert!(out_dir.join("intake/exefs/main").exists());
    assert!(out_dir.join("intake/segments/main-text.bin").exists());
    assert!(out_dir.join("assets/romfs.bin").exists());

    let manifest_src = fs::read_to_string(out_dir.join("manifest.json")).expect("manifest");
    let manifest: serde_json::Value = serde_json::from_str(&manifest_src).expect("parse manifest");
    assert!(manifest
        .get("tool")
        .and_then(|tool| tool.get("xci_tool"))
        .is_some());
}

struct ProgramEntry {
    name: String,
    title_id: u64,
    version: u32,
    nca_bytes: Vec<u8>,
}

fn build_program_entry(name: &str, title_id: u64, version: u32) -> ProgramEntry {
    let nso = build_nso();
    let exefs = build_pfs0(&[("main", &nso), ("main.npdm", b"NPDM")]);
    let romfs = b"ROMFS".to_vec();
    let nca_bytes = build_nca(title_id, version, &exefs, &romfs);
    ProgramEntry {
        name: name.to_string(),
        title_id,
        version,
        nca_bytes,
    }
}

fn build_xci(entries: Vec<ProgramEntry>) -> Vec<u8> {
    let header_size = 0x20;
    let entry_size = 0x40;
    let table_size = entry_size * entries.len();
    let data_offset = header_size + table_size;
    let total_size: usize = data_offset
        + entries
            .iter()
            .map(|entry| entry.nca_bytes.len())
            .sum::<usize>();
    let mut bytes = vec![0u8; total_size];
    bytes[0..4].copy_from_slice(b"XCI0");
    write_u32(&mut bytes, 0x8, entries.len() as u32);
    write_u32(&mut bytes, 0xC, entry_size as u32);

    let mut cursor = data_offset;
    for (index, entry) in entries.into_iter().enumerate() {
        let entry_offset = header_size + index * entry_size;
        let name_bytes = entry.name.as_bytes();
        let name_len = name_bytes.len().min(32);
        bytes[entry_offset..entry_offset + name_len].copy_from_slice(&name_bytes[..name_len]);
        write_u64(&mut bytes, entry_offset + 32, cursor as u64);
        write_u64(&mut bytes, entry_offset + 40, entry.nca_bytes.len() as u64);
        write_u64(&mut bytes, entry_offset + 48, entry.title_id);
        write_u32(&mut bytes, entry_offset + 56, entry.version);
        write_u32(&mut bytes, entry_offset + 60, 0); // program content
        bytes[cursor..cursor + entry.nca_bytes.len()].copy_from_slice(&entry.nca_bytes);
        cursor += entry.nca_bytes.len();
    }

    bytes
}

fn build_nca(title_id: u64, version: u32, exefs: &[u8], romfs: &[u8]) -> Vec<u8> {
    let header_size = 0x40;
    let exefs_offset = header_size as u64;
    let exefs_size = exefs.len() as u64;
    let romfs_offset = exefs_offset + exefs_size;
    let romfs_size = romfs.len() as u64;
    let mut bytes = vec![0u8; header_size + exefs.len() + romfs.len()];
    bytes[0..4].copy_from_slice(b"NCA3");
    write_u32(&mut bytes, 0x8, 0); // program
    write_u64(&mut bytes, 0x10, title_id);
    write_u32(&mut bytes, 0x18, version);
    write_u64(&mut bytes, 0x20, exefs_offset);
    write_u64(&mut bytes, 0x28, exefs_size);
    write_u64(&mut bytes, 0x30, romfs_offset);
    write_u64(&mut bytes, 0x38, romfs_size);
    bytes[header_size..header_size + exefs.len()].copy_from_slice(exefs);
    let romfs_start = header_size + exefs.len();
    bytes[romfs_start..romfs_start + romfs.len()].copy_from_slice(romfs);
    bytes
}

fn build_pfs0(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let header_size = 0x10;
    let entry_size = 0x18;
    let string_table: Vec<u8> = entries
        .iter()
        .flat_map(|(name, _)| name.as_bytes().iter().cloned().chain(std::iter::once(0)))
        .collect();
    let string_table_size = string_table.len();
    let entry_table_size = entry_size * entries.len();
    let data_offset = header_size + entry_table_size + string_table_size;
    let data_size: usize = entries.iter().map(|(_, data)| data.len()).sum();
    let mut bytes = vec![0u8; data_offset + data_size];
    bytes[0..4].copy_from_slice(b"PFS0");
    write_u32(&mut bytes, 0x4, entries.len() as u32);
    write_u32(&mut bytes, 0x8, string_table_size as u32);

    let mut name_offset = 0u32;
    let mut cursor = data_offset;
    for (index, (name, data)) in entries.iter().enumerate() {
        let entry_offset = header_size + index * entry_size;
        write_u64(&mut bytes, entry_offset, (cursor - data_offset) as u64);
        write_u64(&mut bytes, entry_offset + 0x8, data.len() as u64);
        write_u32(&mut bytes, entry_offset + 0x10, name_offset);
        let next_offset = name_offset + name.len() as u32 + 1;
        name_offset = next_offset;
        bytes[cursor..cursor + data.len()].copy_from_slice(data);
        cursor += data.len();
    }

    let string_table_offset = header_size + entry_table_size;
    bytes[string_table_offset..string_table_offset + string_table_size]
        .copy_from_slice(&string_table);

    bytes
}

fn build_nso() -> Vec<u8> {
    let header_size = 0x100;
    let text = vec![0x90u8; 0x10];
    let rodata = vec![0x42u8; 0x10];
    let data = vec![0x24u8; 0x10];
    let text_offset = header_size;
    let rodata_offset = text_offset + text.len();
    let data_offset = rodata_offset + rodata.len();
    let total_size = data_offset + data.len();
    let mut bytes = vec![0u8; total_size];
    bytes[0..4].copy_from_slice(b"NSO0");
    write_u32(&mut bytes, 0x10, text_offset as u32);
    write_u32(&mut bytes, 0x14, 0x1000);
    write_u32(&mut bytes, 0x18, text.len() as u32);
    write_u32(&mut bytes, 0x20, rodata_offset as u32);
    write_u32(&mut bytes, 0x24, 0x2000);
    write_u32(&mut bytes, 0x28, rodata.len() as u32);
    write_u32(&mut bytes, 0x30, data_offset as u32);
    write_u32(&mut bytes, 0x34, 0x3000);
    write_u32(&mut bytes, 0x38, data.len() as u32);
    write_u32(&mut bytes, 0x60, text.len() as u32);
    write_u32(&mut bytes, 0x64, rodata.len() as u32);
    write_u32(&mut bytes, 0x68, data.len() as u32);

    bytes[text_offset..text_offset + text.len()].copy_from_slice(&text);
    bytes[rodata_offset..rodata_offset + rodata.len()].copy_from_slice(&rodata);
    bytes[data_offset..data_offset + data.len()].copy_from_slice(&data);
    bytes
}

fn write_provenance(path: &Path, xci_path: &Path, keys_path: &Path) {
    let xci_hash = sha256_path(xci_path);
    let keys_hash = sha256_path(keys_path);
    let xci_size = fs::metadata(xci_path).expect("metadata").len();
    let keys_size = fs::metadata(keys_path).expect("metadata").len();

    let provenance = format!(
        "schema_version = \"1\"\n\n[title]\nname = \"Fixture\"\ntitle_id = \"0100000000000000\"\nversion = \"1.0.0\"\nregion = \"US\"\n\n[collection]\ndevice = \"fixture\"\ncollected_at = \"2026-02-04\"\n\n[collection.tool]\nname = \"fixture\"\nversion = \"1.0\"\n\n[[inputs]]\npath = \"{}\"\nformat = \"xci\"\nsha256 = \"{}\"\nsize = {}\nrole = \"xci\"\n\n[[inputs]]\npath = \"{}\"\nformat = \"keyset\"\nsha256 = \"{}\"\nsize = {}\nrole = \"keyset\"\n",
        xci_path.display(),
        xci_hash,
        xci_size,
        keys_path.display(),
        keys_hash,
        keys_size
    );
    fs::write(path, provenance).expect("write provenance");
}

fn sha256_path(path: &Path) -> String {
    let bytes = fs::read(path).expect("read file");
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    format!("{:x}", digest)
}

fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_u64(bytes: &mut [u8], offset: usize, value: u64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}
