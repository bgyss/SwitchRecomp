use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct MemoryLayoutDescriptor {
    pub regions: Vec<MemoryRegionDescriptor>,
}

impl MemoryLayoutDescriptor {
    pub fn minimal_default() -> Self {
        Self {
            regions: vec![
                MemoryRegionDescriptor::new(
                    "code",
                    0x1000_0000,
                    0x0001_0000,
                    MemoryPermissionsDescriptor::new(true, false, true),
                ),
                MemoryRegionDescriptor::new(
                    "rodata",
                    0x1001_0000,
                    0x0001_0000,
                    MemoryPermissionsDescriptor::new(true, false, false),
                ),
                MemoryRegionDescriptor::new(
                    "data",
                    0x1002_0000,
                    0x0001_0000,
                    MemoryPermissionsDescriptor::new(true, true, false),
                ),
                MemoryRegionDescriptor::new(
                    "heap",
                    0x2000_0000,
                    0x0004_0000,
                    MemoryPermissionsDescriptor::new(true, true, false),
                ),
                MemoryRegionDescriptor::new(
                    "stack",
                    0x3000_0000,
                    0x0004_0000,
                    MemoryPermissionsDescriptor::new(true, true, false),
                ),
            ],
        }
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct MemoryRegionDescriptor {
    pub name: String,
    pub base: u64,
    pub size: u64,
    pub permissions: MemoryPermissionsDescriptor,
}

impl MemoryRegionDescriptor {
    pub fn new(
        name: impl Into<String>,
        base: u64,
        size: u64,
        permissions: MemoryPermissionsDescriptor,
    ) -> Self {
        Self {
            name: name.into(),
            base,
            size,
            permissions,
        }
    }
}

#[derive(Debug, Serialize, Clone, Copy)]
pub struct MemoryPermissionsDescriptor {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
}

impl MemoryPermissionsDescriptor {
    pub fn new(read: bool, write: bool, execute: bool) -> Self {
        Self {
            read,
            write,
            execute,
        }
    }
}
