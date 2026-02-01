use std::collections::HashSet;

const ROMFS_HEADER_SIZE: u64 = 0x50;
const ROMFS_INVALID_OFFSET: u32 = 0xFFFF_FFFF;

#[derive(Debug, Clone)]
pub struct RomfsEntry {
    pub path: String,
    pub data_offset: u64,
    pub data_size: u64,
}

#[derive(Debug, Clone)]
struct RomfsHeader {
    dir_table_off: u64,
    dir_table_size: u64,
    file_table_off: u64,
    file_table_size: u64,
    file_data_off: u64,
}

#[derive(Debug, Clone)]
struct DirEntry {
    sibling: u32,
    child_dir: u32,
    child_file: u32,
    name: String,
}

#[derive(Debug, Clone)]
struct FileEntry {
    sibling: u32,
    data_off: u64,
    data_size: u64,
    name: String,
}

pub fn list_romfs_entries(bytes: &[u8]) -> Result<Vec<RomfsEntry>, String> {
    let header = parse_header(bytes)?;
    let mut entries = Vec::new();
    let mut dir_stack = Vec::new();
    let mut visited_dirs = HashSet::new();
    let mut visited_files = HashSet::new();
    let mut seen_paths = HashSet::new();

    dir_stack.push((0u32, Vec::<String>::new()));

    while let Some((dir_off, parent_path)) = dir_stack.pop() {
        if !visited_dirs.insert(dir_off) {
            return Err(format!("romfs directory loop at offset {dir_off}"));
        }
        let dir = read_dir_entry(bytes, &header, dir_off)?;
        let mut current_path = parent_path.clone();
        if !dir.name.is_empty() {
            current_path.push(dir.name);
        }

        if dir.child_file != ROMFS_INVALID_OFFSET {
            extract_files(
                bytes,
                &header,
                dir.child_file,
                &current_path,
                &mut visited_files,
                &mut seen_paths,
                &mut entries,
            )?;
        }

        if dir.sibling != ROMFS_INVALID_OFFSET {
            dir_stack.push((dir.sibling, parent_path));
        }
        if dir.child_dir != ROMFS_INVALID_OFFSET {
            dir_stack.push((dir.child_dir, current_path));
        }
    }

    Ok(entries)
}

fn extract_files(
    bytes: &[u8],
    header: &RomfsHeader,
    start_off: u32,
    dir_components: &[String],
    visited_files: &mut HashSet<u32>,
    seen_paths: &mut HashSet<String>,
    entries: &mut Vec<RomfsEntry>,
) -> Result<(), String> {
    let mut file_off = start_off;
    while file_off != ROMFS_INVALID_OFFSET {
        if !visited_files.insert(file_off) {
            return Err(format!("romfs file loop at offset {file_off}"));
        }
        let file = read_file_entry(bytes, header, file_off)?;
        if file.name.is_empty() {
            return Err(format!("romfs file entry {file_off} has empty name"));
        }
        let mut components = dir_components.to_vec();
        components.push(file.name);
        let path = components.join("/");
        if !seen_paths.insert(path.clone()) {
            return Err(format!("romfs duplicate path {path}"));
        }

        let data_offset = header
            .file_data_off
            .checked_add(file.data_off)
            .ok_or_else(|| "romfs data offset overflow".to_string())?;
        let data_end = data_offset
            .checked_add(file.data_size)
            .ok_or_else(|| "romfs data size overflow".to_string())?;
        if data_end > bytes.len() as u64 {
            return Err(format!(
                "romfs file data out of range: {}..{} (len={})",
                data_offset,
                data_end,
                bytes.len()
            ));
        }

        entries.push(RomfsEntry {
            path,
            data_offset,
            data_size: file.data_size,
        });

        file_off = file.sibling;
    }

    Ok(())
}

fn parse_header(bytes: &[u8]) -> Result<RomfsHeader, String> {
    if bytes.len() < ROMFS_HEADER_SIZE as usize {
        return Err(format!("romfs image too small: {} bytes", bytes.len()));
    }
    let header_size = read_u64(bytes, 0x0, "header_size")?;
    if header_size != ROMFS_HEADER_SIZE {
        return Err(format!(
            "romfs header size mismatch: expected 0x50, got 0x{header_size:x}"
        ));
    }

    let dir_table_off = read_u64(bytes, 0x18, "dir_table_off")?;
    let dir_table_size = read_u64(bytes, 0x20, "dir_table_size")?;
    let file_table_off = read_u64(bytes, 0x38, "file_table_off")?;
    let file_table_size = read_u64(bytes, 0x40, "file_table_size")?;
    let file_data_off = read_u64(bytes, 0x48, "file_data_off")?;

    validate_range(bytes, dir_table_off, dir_table_size, "dir_table")?;
    validate_range(bytes, file_table_off, file_table_size, "file_table")?;
    if file_data_off > bytes.len() as u64 {
        return Err(format!(
            "romfs file data offset out of range: {file_data_off} (len={})",
            bytes.len()
        ));
    }

    Ok(RomfsHeader {
        dir_table_off,
        dir_table_size,
        file_table_off,
        file_table_size,
        file_data_off,
    })
}

fn read_dir_entry(bytes: &[u8], header: &RomfsHeader, dir_off: u32) -> Result<DirEntry, String> {
    let dir_off = dir_off as u64;
    if dir_off >= header.dir_table_size {
        return Err(format!(
            "romfs dir offset out of range: {dir_off} (size={})",
            header.dir_table_size
        ));
    }
    let entry_off = header
        .dir_table_off
        .checked_add(dir_off)
        .ok_or_else(|| "romfs dir offset overflow".to_string())?;
    let entry_off = entry_off as usize;
    let sibling = read_u32(bytes, entry_off + 0x4, "dir_sibling")?;
    let child_dir = read_u32(bytes, entry_off + 0x8, "dir_child_dir")?;
    let child_file = read_u32(bytes, entry_off + 0xC, "dir_child_file")?;
    let name_len = read_u32(bytes, entry_off + 0x14, "dir_name_len")? as usize;
    let name_off = entry_off + 0x18;
    let name = read_name(bytes, name_off, name_len, "dir")?;

    Ok(DirEntry {
        sibling,
        child_dir,
        child_file,
        name,
    })
}

fn read_file_entry(bytes: &[u8], header: &RomfsHeader, file_off: u32) -> Result<FileEntry, String> {
    let file_off = file_off as u64;
    if file_off >= header.file_table_size {
        return Err(format!(
            "romfs file offset out of range: {file_off} (size={})",
            header.file_table_size
        ));
    }
    let entry_off = header
        .file_table_off
        .checked_add(file_off)
        .ok_or_else(|| "romfs file offset overflow".to_string())?;
    let entry_off = entry_off as usize;
    let sibling = read_u32(bytes, entry_off + 0x4, "file_sibling")?;
    let data_off = read_u64(bytes, entry_off + 0x8, "file_data_off")?;
    let data_size = read_u64(bytes, entry_off + 0x10, "file_data_size")?;
    let name_len = read_u32(bytes, entry_off + 0x1C, "file_name_len")? as usize;
    let name_off = entry_off + 0x20;
    let name = read_name(bytes, name_off, name_len, "file")?;

    Ok(FileEntry {
        sibling,
        data_off,
        data_size,
        name,
    })
}

fn read_name(bytes: &[u8], offset: usize, len: usize, kind: &str) -> Result<String, String> {
    if len == 0 {
        return Ok(String::new());
    }
    let end = offset
        .checked_add(len)
        .ok_or_else(|| format!("romfs {kind} name overflow"))?;
    if end > bytes.len() {
        return Err(format!(
            "romfs {kind} name out of range: {}..{} (len={})",
            offset,
            end,
            bytes.len()
        ));
    }
    let name_bytes = &bytes[offset..end];
    let terminator = name_bytes.iter().position(|b| *b == 0).unwrap_or(len);
    let name_bytes = &name_bytes[..terminator];
    let name = String::from_utf8(name_bytes.to_vec())
        .map_err(|_| format!("romfs {kind} name is not valid UTF-8"))?;
    if name.contains('/') || name.contains('\\') {
        return Err(format!("romfs {kind} name has path separator: {name}"));
    }
    if name == "." || name == ".." {
        return Err(format!("romfs {kind} name is invalid: {name}"));
    }
    Ok(name)
}

fn read_u32(bytes: &[u8], offset: usize, label: &str) -> Result<u32, String> {
    let end = offset + 4;
    if end > bytes.len() {
        return Err(format!(
            "romfs read {label} out of range: {}..{} (len={})",
            offset,
            end,
            bytes.len()
        ));
    }
    let mut buf = [0u8; 4];
    buf.copy_from_slice(&bytes[offset..end]);
    Ok(u32::from_le_bytes(buf))
}

fn read_u64(bytes: &[u8], offset: usize, label: &str) -> Result<u64, String> {
    let end = offset + 8;
    if end > bytes.len() {
        return Err(format!(
            "romfs read {label} out of range: {}..{} (len={})",
            offset,
            end,
            bytes.len()
        ));
    }
    let mut buf = [0u8; 8];
    buf.copy_from_slice(&bytes[offset..end]);
    Ok(u64::from_le_bytes(buf))
}

fn validate_range(bytes: &[u8], offset: u64, size: u64, label: &str) -> Result<(), String> {
    let end = offset
        .checked_add(size)
        .ok_or_else(|| format!("romfs {label} range overflow"))?;
    if end > bytes.len() as u64 {
        return Err(format!(
            "romfs {label} out of range: {}..{} (len={})",
            offset,
            end,
            bytes.len()
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::list_romfs_entries;

    fn align_up(value: usize, align: usize) -> usize {
        (value + align - 1) / align * align
    }

    fn write_u64(bytes: &mut [u8], offset: usize, value: u64) {
        bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
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
        file_data.extend(std::iter::repeat(0u8).take(padding));
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

        assert_eq!(dir_table.len(), dir_table_size);

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

        assert_eq!(file_table.len(), file_table_size);

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

    #[test]
    fn list_romfs_entries_emits_paths() {
        let image = build_romfs_image();
        let mut entries = list_romfs_entries(&image).expect("list entries");
        entries.sort_by(|a, b| a.path.cmp(&b.path));
        let paths = entries
            .iter()
            .map(|entry| entry.path.as_str())
            .collect::<Vec<_>>();
        assert_eq!(paths, vec!["data/nested.bin", "hello.txt"]);
    }
}
