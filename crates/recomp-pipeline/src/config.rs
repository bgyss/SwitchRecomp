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

#[derive(Debug, Deserialize)]
struct RawTitleConfig {
    title: String,
    entry: String,
    abi_version: String,
    #[serde(default)]
    stubs: BTreeMap<String, String>,
}

#[derive(Debug)]
pub struct TitleConfig {
    pub title: String,
    pub entry: String,
    pub abi_version: String,
    pub stubs: BTreeMap<String, StubBehavior>,
}

impl TitleConfig {
    pub fn parse(toml_src: &str) -> Result<Self, String> {
        let raw: RawTitleConfig = toml::from_str(toml_src)
            .map_err(|err| format!("invalid config: {err}"))?;
        let mut stubs = BTreeMap::new();
        for (name, behavior) in raw.stubs {
            let parsed = StubBehavior::from_str(&behavior)?;
            stubs.insert(name, parsed);
        }
        Ok(TitleConfig {
            title: raw.title,
            entry: raw.entry,
            abi_version: raw.abi_version,
            stubs,
        })
    }
}
