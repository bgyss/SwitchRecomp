use std::fmt;

pub fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, String> {
    let end = offset
        .checked_add(4)
        .ok_or_else(|| "offset overflow".to_string())?;
    if end > bytes.len() {
        return Err(format!(
            "read_u32 out of range: offset={offset} len={}",
            bytes.len()
        ));
    }
    Ok(u32::from_le_bytes(
        bytes[offset..end].try_into().expect("slice length"),
    ))
}

pub fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, String> {
    let end = offset
        .checked_add(8)
        .ok_or_else(|| "offset overflow".to_string())?;
    if end > bytes.len() {
        return Err(format!(
            "read_u64 out of range: offset={offset} len={}",
            bytes.len()
        ));
    }
    Ok(u64::from_le_bytes(
        bytes[offset..end].try_into().expect("slice length"),
    ))
}

pub fn read_bytes(bytes: &[u8], offset: usize, size: usize) -> Result<&[u8], String> {
    let end = offset
        .checked_add(size)
        .ok_or_else(|| "offset overflow".to_string())?;
    if end > bytes.len() {
        return Err(format!(
            "read_bytes out of range: offset={offset} size={size} len={}",
            bytes.len()
        ));
    }
    Ok(&bytes[offset..end])
}

pub fn hex_bytes(bytes: &[u8]) -> String {
    use std::fmt::Write;

    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(&mut out, "{byte:02x}");
    }
    out
}

pub fn find_magic(bytes: &[u8], magic: u32, search_len: usize) -> Option<usize> {
    let target = magic.to_le_bytes();
    let len = bytes.len().min(search_len);
    bytes
        .windows(4)
        .take(len.saturating_sub(3))
        .position(|window| window == target)
}

#[derive(Debug)]
pub struct ParseError {
    pub context: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.context)
    }
}

impl std::error::Error for ParseError {}

pub fn parse_error(context: impl Into<String>) -> ParseError {
    ParseError {
        context: context.into(),
    }
}
