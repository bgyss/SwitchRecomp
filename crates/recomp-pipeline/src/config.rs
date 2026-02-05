use crate::memory::{MemoryLayoutDescriptor, MemoryPermissionsDescriptor, MemoryRegionDescriptor};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StubBehavior {
    Log,
    Noop,
    Panic,
}

impl FromStr for StubBehavior {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "log" => Ok(StubBehavior::Log),
            "noop" => Ok(StubBehavior::Noop),
            "panic" => Ok(StubBehavior::Panic),
            other => Err(format!("unknown stub behavior: {other}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerformanceMode {
    Handheld,
    Docked,
}

impl FromStr for PerformanceMode {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "handheld" => Ok(PerformanceMode::Handheld),
            "docked" => Ok(PerformanceMode::Docked),
            other => Err(format!("unknown performance mode: {other}")),
        }
    }
}

#[derive(Debug, Deserialize, Default)]
struct RawRuntimeConfig {
    #[serde(default)]
    performance_mode: Option<String>,
    #[serde(default)]
    memory_layout: Option<RawMemoryLayoutConfig>,
}

#[derive(Debug, Deserialize)]
struct RawMemoryLayoutConfig {
    regions: Vec<RawMemoryRegionConfig>,
}

#[derive(Debug, Deserialize)]
struct RawMemoryRegionConfig {
    name: String,
    base: u64,
    size: u64,
    permissions: RawMemoryPermissionsConfig,
}

#[derive(Debug, Deserialize)]
struct RawMemoryPermissionsConfig {
    read: bool,
    write: bool,
    execute: bool,
}

#[derive(Debug)]
pub struct RuntimeConfig {
    pub performance_mode: PerformanceMode,
}

#[derive(Debug, Deserialize)]
struct RawTitleConfig {
    title: String,
    entry: String,
    abi_version: String,
    #[serde(default)]
    stubs: BTreeMap<String, String>,
    #[serde(default)]
    runtime: Option<RawRuntimeConfig>,
}

#[derive(Debug)]
pub struct TitleConfig {
    pub title: String,
    pub entry: String,
    pub abi_version: String,
    pub stubs: BTreeMap<String, StubBehavior>,
    pub runtime: RuntimeConfig,
    pub memory_layout: MemoryLayoutDescriptor,
}

impl TitleConfig {
    pub fn parse(toml_src: &str) -> Result<Self, String> {
        let raw: RawTitleConfig =
            toml::from_str(toml_src).map_err(|err| format!("invalid config: {err}"))?;
        let mut stubs = BTreeMap::new();
        for (name, behavior) in raw.stubs {
            let parsed = StubBehavior::from_str(&behavior)?;
            stubs.insert(name, parsed);
        }
        let runtime = raw.runtime.unwrap_or_default();
        let runtime_mode = runtime
            .performance_mode
            .unwrap_or_else(|| "handheld".to_string());
        let performance_mode = PerformanceMode::from_str(&runtime_mode)?;
        let memory_layout = match runtime.memory_layout {
            Some(layout) => parse_memory_layout(layout)?,
            None => {
                let layout = MemoryLayoutDescriptor::minimal_default();
                layout.validate()?;
                layout
            }
        };
        Ok(TitleConfig {
            title: raw.title,
            entry: raw.entry,
            abi_version: raw.abi_version,
            stubs,
            runtime: RuntimeConfig { performance_mode },
            memory_layout,
        })
    }
}

fn parse_memory_layout(layout: RawMemoryLayoutConfig) -> Result<MemoryLayoutDescriptor, String> {
    let regions = layout
        .regions
        .into_iter()
        .map(|region| {
            MemoryRegionDescriptor::new(
                region.name,
                region.base,
                region.size,
                MemoryPermissionsDescriptor::new(
                    region.permissions.read,
                    region.permissions.write,
                    region.permissions.execute,
                ),
            )
        })
        .collect();
    let descriptor = MemoryLayoutDescriptor { regions };
    descriptor.validate()?;
    Ok(descriptor)
}
