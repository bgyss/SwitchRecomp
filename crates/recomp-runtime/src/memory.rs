use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum MemoryStatus {
    Ok = 0,
    Unaligned = 1,
    OutOfBounds = 2,
    PermissionDenied = 3,
    Unmapped = 4,
    Uninitialized = 5,
    InvalidOutPtr = 6,
    Internal = 7,
}

impl MemoryStatus {
    pub fn is_ok(self) -> bool {
        matches!(self, MemoryStatus::Ok)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryPermissions {
    read: bool,
    write: bool,
    execute: bool,
}

impl MemoryPermissions {
    pub const fn new(read: bool, write: bool, execute: bool) -> Self {
        Self {
            read,
            write,
            execute,
        }
    }

    pub const fn read_only() -> Self {
        Self::new(true, false, false)
    }

    pub const fn read_write() -> Self {
        Self::new(true, true, false)
    }

    pub const fn read_execute() -> Self {
        Self::new(true, false, true)
    }

    pub fn allows_read(self) -> bool {
        self.read
    }

    pub fn allows_write(self) -> bool {
        self.write
    }

    pub fn allows_execute(self) -> bool {
        self.execute
    }
}

#[derive(Debug, Clone)]
pub struct MemoryRegionSpec {
    pub name: String,
    pub base: u64,
    pub size: u64,
    pub permissions: MemoryPermissions,
}

impl MemoryRegionSpec {
    pub fn new(
        name: impl Into<String>,
        base: u64,
        size: u64,
        permissions: MemoryPermissions,
    ) -> Self {
        Self {
            name: name.into(),
            base,
            size,
            permissions,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MemoryLayout {
    pub regions: Vec<MemoryRegionSpec>,
}

impl MemoryLayout {
    pub fn new(regions: Vec<MemoryRegionSpec>) -> Self {
        Self { regions }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MemoryLayoutError {
    #[error("memory region {name} has zero size")]
    ZeroSizedRegion { name: String },
    #[error("memory region {name} overflows address space")]
    RegionOverflow { name: String },
    #[error("memory regions {left} and {right} overlap")]
    RegionOverlap { left: String, right: String },
}

#[derive(Debug, Clone, Copy)]
enum AccessKind {
    Read,
    Write,
    #[allow(dead_code)]
    Execute,
}

#[derive(Debug)]
struct MemoryRegion {
    spec: MemoryRegionSpec,
    data: Vec<u8>,
}

#[derive(Debug)]
struct RuntimeMemory {
    regions: Vec<MemoryRegion>,
}

impl RuntimeMemory {
    fn new(layout: &MemoryLayout) -> Result<Self, MemoryLayoutError> {
        let mut regions = Vec::with_capacity(layout.regions.len());
        for spec in &layout.regions {
            if spec.size == 0 {
                return Err(MemoryLayoutError::ZeroSizedRegion {
                    name: spec.name.clone(),
                });
            }
            let end = spec.base.checked_add(spec.size).ok_or_else(|| {
                MemoryLayoutError::RegionOverflow {
                    name: spec.name.clone(),
                }
            })?;
            regions.push((spec.clone(), end));
        }

        regions.sort_by(|a, b| a.0.base.cmp(&b.0.base));
        for pair in regions.windows(2) {
            let left = &pair[0].0;
            let left_end = pair[0].1;
            let right = &pair[1].0;
            if left_end > right.base {
                return Err(MemoryLayoutError::RegionOverlap {
                    left: left.name.clone(),
                    right: right.name.clone(),
                });
            }
        }

        let mapped = regions
            .into_iter()
            .map(|(spec, _)| MemoryRegion {
                data: vec![0u8; spec.size as usize],
                spec,
            })
            .collect();

        Ok(Self { regions: mapped })
    }

    fn load(&self, address: u64, size: usize) -> Result<u64, MemoryStatus> {
        let region = self.resolve_region(address, size, AccessKind::Read)?;
        let offset = (address - region.spec.base) as usize;
        let mut value = 0u64;
        for i in 0..size {
            value |= (region.data[offset + i] as u64) << (i * 8);
        }
        Ok(value)
    }

    fn store(&mut self, address: u64, size: usize, value: u64) -> Result<(), MemoryStatus> {
        let region = self.resolve_region_mut(address, size, AccessKind::Write)?;
        let offset = (address - region.spec.base) as usize;
        for i in 0..size {
            region.data[offset + i] = ((value >> (i * 8)) & 0xFF) as u8;
        }
        Ok(())
    }

    fn resolve_region(
        &self,
        address: u64,
        size: usize,
        access: AccessKind,
    ) -> Result<&MemoryRegion, MemoryStatus> {
        let index = self.resolve_region_inner(address, size, access)?;
        Ok(&self.regions[index])
    }

    fn resolve_region_mut(
        &mut self,
        address: u64,
        size: usize,
        access: AccessKind,
    ) -> Result<&mut MemoryRegion, MemoryStatus> {
        let index = self.resolve_region_inner(address, size, access)?;
        Ok(&mut self.regions[index])
    }

    fn resolve_region_inner(
        &self,
        address: u64,
        size: usize,
        access: AccessKind,
    ) -> Result<usize, MemoryStatus> {
        if size == 0 {
            return Err(MemoryStatus::Internal);
        }
        let size_u64 = size as u64;
        if address % size_u64 != 0 {
            return Err(MemoryStatus::Unaligned);
        }
        let end = address
            .checked_add(size_u64)
            .ok_or(MemoryStatus::OutOfBounds)?;

        for (index, region) in self.regions.iter().enumerate() {
            let region_end = region
                .spec
                .base
                .checked_add(region.spec.size)
                .ok_or(MemoryStatus::OutOfBounds)?;
            if address < region.spec.base || address >= region_end {
                continue;
            }
            if end > region_end {
                return Err(MemoryStatus::OutOfBounds);
            }
            if !self.check_permissions(region, access) {
                return Err(MemoryStatus::PermissionDenied);
            }
            return Ok(index);
        }

        Err(MemoryStatus::Unmapped)
    }

    fn check_permissions(&self, region: &MemoryRegion, access: AccessKind) -> bool {
        match access {
            AccessKind::Read => region.spec.permissions.allows_read(),
            AccessKind::Write => region.spec.permissions.allows_write(),
            AccessKind::Execute => region.spec.permissions.allows_execute(),
        }
    }
}

static MEMORY: OnceLock<Mutex<RuntimeMemory>> = OnceLock::new();

pub fn init_memory(layout: MemoryLayout) -> Result<(), MemoryLayoutError> {
    let memory = RuntimeMemory::new(&layout)?;
    let _ = MEMORY.set(Mutex::new(memory));
    Ok(())
}

fn with_memory_mut<F>(mut f: F) -> MemoryStatus
where
    F: FnMut(&mut RuntimeMemory) -> Result<(), MemoryStatus>,
{
    let memory = match MEMORY.get() {
        Some(memory) => memory,
        None => return MemoryStatus::Uninitialized,
    };
    let mut guard = match memory.lock() {
        Ok(guard) => guard,
        Err(_) => return MemoryStatus::Internal,
    };
    match f(&mut guard) {
        Ok(()) => MemoryStatus::Ok,
        Err(err) => err,
    }
}

fn with_memory<F, T>(mut f: F) -> Result<T, MemoryStatus>
where
    F: FnMut(&RuntimeMemory) -> Result<T, MemoryStatus>,
{
    let memory = match MEMORY.get() {
        Some(memory) => memory,
        None => return Err(MemoryStatus::Uninitialized),
    };
    let guard = match memory.lock() {
        Ok(guard) => guard,
        Err(_) => return Err(MemoryStatus::Internal),
    };
    f(&guard)
}

#[no_mangle]
pub extern "C" fn recomp_mem_load_u8(address: u64, out: *mut u64) -> MemoryStatus {
    mem_load_raw(address, 1, out)
}

#[no_mangle]
pub extern "C" fn recomp_mem_load_u16(address: u64, out: *mut u64) -> MemoryStatus {
    mem_load_raw(address, 2, out)
}

#[no_mangle]
pub extern "C" fn recomp_mem_load_u32(address: u64, out: *mut u64) -> MemoryStatus {
    mem_load_raw(address, 4, out)
}

#[no_mangle]
pub extern "C" fn recomp_mem_load_u64(address: u64, out: *mut u64) -> MemoryStatus {
    mem_load_raw(address, 8, out)
}

#[no_mangle]
pub extern "C" fn recomp_mem_store_u8(address: u64, value: u64) -> MemoryStatus {
    mem_store_raw(address, 1, value)
}

#[no_mangle]
pub extern "C" fn recomp_mem_store_u16(address: u64, value: u64) -> MemoryStatus {
    mem_store_raw(address, 2, value)
}

#[no_mangle]
pub extern "C" fn recomp_mem_store_u32(address: u64, value: u64) -> MemoryStatus {
    mem_store_raw(address, 4, value)
}

#[no_mangle]
pub extern "C" fn recomp_mem_store_u64(address: u64, value: u64) -> MemoryStatus {
    mem_store_raw(address, 8, value)
}

fn mem_load_raw(address: u64, size: usize, out: *mut u64) -> MemoryStatus {
    if out.is_null() {
        return MemoryStatus::InvalidOutPtr;
    }
    match with_memory(|memory| memory.load(address, size)) {
        Ok(value) => unsafe {
            *out = value;
            MemoryStatus::Ok
        },
        Err(err) => err,
    }
}

fn mem_store_raw(address: u64, size: usize, value: u64) -> MemoryStatus {
    with_memory_mut(|memory| memory.store(address, size, value))
}

pub(crate) fn mem_load_u8(address: u64) -> Result<u64, MemoryStatus> {
    mem_load(address, 1)
}

pub(crate) fn mem_load_u16(address: u64) -> Result<u64, MemoryStatus> {
    mem_load(address, 2)
}

pub(crate) fn mem_load_u32(address: u64) -> Result<u64, MemoryStatus> {
    mem_load(address, 4)
}

pub(crate) fn mem_load_u64(address: u64) -> Result<u64, MemoryStatus> {
    mem_load(address, 8)
}

pub(crate) fn mem_store_u8(address: u64, value: u64) -> Result<(), MemoryStatus> {
    mem_store(address, 1, value)
}

pub(crate) fn mem_store_u16(address: u64, value: u64) -> Result<(), MemoryStatus> {
    mem_store(address, 2, value)
}

pub(crate) fn mem_store_u32(address: u64, value: u64) -> Result<(), MemoryStatus> {
    mem_store(address, 4, value)
}

pub(crate) fn mem_store_u64(address: u64, value: u64) -> Result<(), MemoryStatus> {
    mem_store(address, 8, value)
}

fn mem_load(address: u64, size: usize) -> Result<u64, MemoryStatus> {
    with_memory(|memory| memory.load(address, size))
}

fn mem_store(address: u64, size: usize, value: u64) -> Result<(), MemoryStatus> {
    let status = with_memory_mut(|memory| memory.store(address, size, value));
    if status.is_ok() {
        Ok(())
    } else {
        Err(status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_layout() -> MemoryLayout {
        MemoryLayout::new(vec![
            MemoryRegionSpec::new("data", 0x1000, 0x40, MemoryPermissions::read_write()),
            MemoryRegionSpec::new("ro", 0x2000, 0x40, MemoryPermissions::read_only()),
        ])
    }

    #[test]
    fn load_store_round_trip() {
        init_memory(test_layout()).expect("init memory");
        mem_store_u32(0x1004, 0xDEADBEEF).expect("store");
        let value = mem_load_u32(0x1004).expect("load");
        assert_eq!(value, 0xDEADBEEF);
    }

    #[test]
    fn unaligned_is_error() {
        init_memory(test_layout()).expect("init memory");
        let err = mem_load_u32(0x1002).unwrap_err();
        assert_eq!(err, MemoryStatus::Unaligned);
    }

    #[test]
    fn permission_denied_for_write() {
        init_memory(test_layout()).expect("init memory");
        let err = mem_store_u8(0x2000, 1).unwrap_err();
        assert_eq!(err, MemoryStatus::PermissionDenied);
    }
}
