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

    pub fn validate(&self) -> Result<(), String> {
        if self.regions.is_empty() {
            return Err("memory layout must define at least one region".to_string());
        }

        let mut ranges = Vec::with_capacity(self.regions.len());
        for region in &self.regions {
            if region.size == 0 {
                return Err(format!("memory region {} has zero size", region.name));
            }
            let end = region
                .base
                .checked_add(region.size)
                .ok_or_else(|| format!("memory region {} overflows address space", region.name))?;
            ranges.push((region.name.as_str(), region.base, end));
        }

        ranges.sort_by(|a, b| a.1.cmp(&b.1));
        for window in ranges.windows(2) {
            let left = window[0];
            let right = window[1];
            if left.2 > right.1 {
                return Err(format!("memory regions {} and {} overlap", left.0, right.0));
            }
        }

        Ok(())
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

#[derive(Debug, Serialize, Clone)]
pub struct MemoryImageDescriptor {
    pub init_segments: Vec<MemoryInitSegmentDescriptor>,
    pub zero_segments: Vec<MemoryZeroSegmentDescriptor>,
}

impl MemoryImageDescriptor {
    pub fn empty() -> Self {
        Self {
            init_segments: Vec::new(),
            zero_segments: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.init_segments.is_empty() && self.zero_segments.is_empty()
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct MemoryInitSegmentDescriptor {
    pub name: String,
    pub base: u64,
    pub size: u64,
    pub init_path: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct MemoryZeroSegmentDescriptor {
    pub name: String,
    pub base: u64,
    pub size: u64,
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
