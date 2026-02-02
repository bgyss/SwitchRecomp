use crate::RuntimeError;
use recomp_services::StubBehavior;
use serde::Serialize;
use std::collections::BTreeSet;

pub const NRO_ENTRY_X1: u64 = 0xFFFF_FFFF_FFFF_FFFF;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
#[repr(u32)]
pub enum LoaderConfigKey {
    EndOfList,
    MainThreadHandle,
    AppletType,
    Argv,
    OverrideHeap,
    AllocPages,
    LockRegion,
}

impl LoaderConfigKey {
    pub fn supported_keys() -> Vec<LoaderConfigKey> {
        vec![
            LoaderConfigKey::EndOfList,
            LoaderConfigKey::MainThreadHandle,
            LoaderConfigKey::AppletType,
            LoaderConfigKey::Argv,
            LoaderConfigKey::OverrideHeap,
            LoaderConfigKey::AllocPages,
            LoaderConfigKey::LockRegion,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct LoaderConfigEntry {
    pub key: LoaderConfigKey,
    pub flags: u32,
    pub values: [u64; 2],
}

impl LoaderConfigEntry {
    pub fn new(key: LoaderConfigKey, value0: u64, value1: u64, flags: u32) -> Self {
        Self {
            key,
            flags,
            values: [value0, value1],
        }
    }
}

#[derive(Debug, Clone)]
pub struct LoaderConfig {
    entries: Vec<LoaderConfigEntry>,
}

impl LoaderConfig {
    pub fn entries(&self) -> &[LoaderConfigEntry] {
        &self.entries
    }

    pub fn entry_ptr(&self) -> *const LoaderConfigEntry {
        self.entries.as_ptr()
    }

    pub fn provided_keys(&self) -> Vec<LoaderConfigKey> {
        let present: BTreeSet<LoaderConfigKey> =
            self.entries.iter().map(|entry| entry.key).collect();
        LoaderConfigKey::supported_keys()
            .into_iter()
            .filter(|key| present.contains(key))
            .collect()
    }
}

#[derive(Debug, Default)]
pub struct LoaderConfigBuilder {
    entries: Vec<LoaderConfigEntry>,
}

impl LoaderConfigBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn main_thread_handle(mut self, handle: u64) -> Self {
        self.entries.push(LoaderConfigEntry::new(
            LoaderConfigKey::MainThreadHandle,
            handle,
            0,
            0,
        ));
        self
    }

    pub fn applet_type(mut self, applet_type: u64) -> Self {
        self.entries.push(LoaderConfigEntry::new(
            LoaderConfigKey::AppletType,
            applet_type,
            0,
            0,
        ));
        self
    }

    pub fn argv(mut self, argv_ptr: u64) -> Self {
        self.entries.push(LoaderConfigEntry::new(
            LoaderConfigKey::Argv,
            argv_ptr,
            0,
            0,
        ));
        self
    }

    pub fn override_heap(mut self, heap_ptr: u64) -> Self {
        self.entries.push(LoaderConfigEntry::new(
            LoaderConfigKey::OverrideHeap,
            heap_ptr,
            0,
            0,
        ));
        self
    }

    pub fn alloc_pages(mut self, page_count: u64) -> Self {
        self.entries.push(LoaderConfigEntry::new(
            LoaderConfigKey::AllocPages,
            page_count,
            0,
            0,
        ));
        self
    }

    pub fn lock_region(mut self, region_ptr: u64) -> Self {
        self.entries.push(LoaderConfigEntry::new(
            LoaderConfigKey::LockRegion,
            region_ptr,
            0,
            0,
        ));
        self
    }

    pub fn build(mut self) -> Result<LoaderConfig, RuntimeError> {
        let present: BTreeSet<LoaderConfigKey> =
            self.entries.iter().map(|entry| entry.key).collect();
        if !present.contains(&LoaderConfigKey::MainThreadHandle) {
            return Err(RuntimeError::MissingLoaderConfigKey {
                key: LoaderConfigKey::MainThreadHandle,
            });
        }
        if !present.contains(&LoaderConfigKey::AppletType) {
            return Err(RuntimeError::MissingLoaderConfigKey {
                key: LoaderConfigKey::AppletType,
            });
        }

        self.entries
            .retain(|entry| entry.key != LoaderConfigKey::EndOfList);
        self.entries
            .push(LoaderConfigEntry::new(LoaderConfigKey::EndOfList, 0, 0, 0));

        Ok(LoaderConfig {
            entries: self.entries,
        })
    }
}

pub type NroEntrypoint = unsafe extern "C" fn(*const LoaderConfigEntry, u64) -> i32;

pub fn entrypoint_shim(entry: NroEntrypoint, loader_config: &LoaderConfig) -> i32 {
    unsafe { entry(loader_config.entry_ptr(), NRO_ENTRY_X1) }
}

#[derive(Debug, Clone)]
pub struct ServiceStub {
    pub name: String,
    pub behavior: StubBehavior,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeManifest {
    pub abi_version: String,
    pub loader_config: LoaderConfigManifest,
    pub services: Vec<ServiceStubManifest>,
    pub determinism: DeterminismManifest,
}

#[derive(Debug, Clone, Serialize)]
pub struct LoaderConfigManifest {
    pub supported_keys: Vec<LoaderConfigKey>,
    pub provided_keys: Vec<LoaderConfigKey>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ServiceStubManifest {
    pub name: String,
    pub behavior: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeterminismManifest {
    pub time_source: String,
    pub input_source: String,
}

impl RuntimeManifest {
    pub fn new(
        abi_version: &str,
        loader_config: &LoaderConfig,
        service_stubs: &[ServiceStub],
    ) -> Self {
        let services = service_stubs
            .iter()
            .map(|stub| ServiceStubManifest {
                name: stub.name.clone(),
                behavior: format!("{:?}", stub.behavior).to_ascii_lowercase(),
            })
            .collect();
        Self {
            abi_version: abi_version.to_string(),
            loader_config: LoaderConfigManifest {
                supported_keys: LoaderConfigKey::supported_keys(),
                provided_keys: loader_config.provided_keys(),
            },
            services,
            determinism: DeterminismManifest {
                time_source: "deterministic".to_string(),
                input_source: "deterministic".to_string(),
            },
        }
    }

    pub fn to_json(&self) -> Result<String, RuntimeError> {
        serde_json::to_string_pretty(self).map_err(|err| RuntimeError::ManifestSerialize {
            message: err.to_string(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputEvent {
    pub time: u64,
    pub code: u32,
    pub value: i32,
}

#[derive(Debug, Default)]
pub struct DeterministicClock {
    time: u64,
}

impl DeterministicClock {
    pub fn new(start: u64) -> Self {
        Self { time: start }
    }

    pub fn now(&self) -> u64 {
        self.time
    }

    pub fn advance(&mut self, delta: u64) -> u64 {
        self.time = self.time.saturating_add(delta);
        self.time
    }

    pub fn set(&mut self, time: u64) {
        self.time = time;
    }
}

#[derive(Debug, Default)]
pub struct InputQueue {
    next_id: u64,
    pending: Vec<QueuedInput>,
}

#[derive(Debug, Clone)]
struct QueuedInput {
    id: u64,
    event: InputEvent,
}

impl InputQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, event: InputEvent) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.pending.push(QueuedInput { id, event });
        id
    }

    pub fn drain_ready(&mut self, time: u64) -> Vec<InputEvent> {
        self.pending.sort_by(|a, b| {
            a.event
                .time
                .cmp(&b.event.time)
                .then_with(|| a.id.cmp(&b.id))
        });

        let mut ready = Vec::new();
        let mut remaining = Vec::new();
        for queued in self.pending.drain(..) {
            if queued.event.time <= time {
                ready.push(queued.event);
            } else {
                remaining.push(queued);
            }
        }
        self.pending = remaining;
        ready
    }

    pub fn pending(&self) -> usize {
        self.pending.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_requires_main_thread_handle() {
        let err = LoaderConfigBuilder::new()
            .applet_type(1)
            .build()
            .unwrap_err();
        assert!(matches!(
            err,
            RuntimeError::MissingLoaderConfigKey {
                key: LoaderConfigKey::MainThreadHandle
            }
        ));
    }

    #[test]
    fn builder_requires_applet_type() {
        let err = LoaderConfigBuilder::new()
            .main_thread_handle(5)
            .build()
            .unwrap_err();
        assert!(matches!(
            err,
            RuntimeError::MissingLoaderConfigKey {
                key: LoaderConfigKey::AppletType
            }
        ));
    }

    #[test]
    fn builder_appends_end_of_list_last() {
        let config = LoaderConfigBuilder::new()
            .main_thread_handle(2)
            .applet_type(3)
            .argv(99)
            .build()
            .expect("build loader config");
        let entries = config.entries();
        assert_eq!(entries.last().unwrap().key, LoaderConfigKey::EndOfList);
    }

    #[test]
    fn entrypoint_shim_passes_expected_registers() {
        use std::sync::{Mutex, OnceLock};

        #[derive(Default)]
        struct Seen {
            x1: u64,
            ptr: usize,
        }

        static SEEN: OnceLock<Mutex<Seen>> = OnceLock::new();
        unsafe extern "C" fn probe(entry: *const LoaderConfigEntry, x1: u64) -> i32 {
            let seen = SEEN.get_or_init(|| Mutex::new(Seen::default()));
            let mut guard = seen.lock().expect("lock");
            guard.x1 = x1;
            guard.ptr = entry as usize;
            0
        }

        let config = LoaderConfigBuilder::new()
            .main_thread_handle(9)
            .applet_type(1)
            .build()
            .expect("build loader config");
        let expected_ptr = config.entry_ptr() as usize;

        let result = entrypoint_shim(probe, &config);
        assert_eq!(result, 0);
        let seen = SEEN.get_or_init(|| Mutex::new(Seen::default()));
        let guard = seen.lock().expect("lock");
        assert_eq!(guard.x1, NRO_ENTRY_X1);
        assert_eq!(guard.ptr, expected_ptr);
    }

    #[test]
    fn manifest_includes_provided_keys() {
        let config = LoaderConfigBuilder::new()
            .main_thread_handle(1)
            .applet_type(2)
            .argv(3)
            .build()
            .expect("config");
        let manifest = RuntimeManifest::new(
            "0.1.0",
            &config,
            &[ServiceStub {
                name: "svc_stub".to_string(),
                behavior: StubBehavior::Panic,
            }],
        );
        let json = manifest.to_json().expect("serialize manifest");
        assert!(json.contains("provided_keys"));
        assert!(json.contains("svc_stub"));
    }

    #[test]
    fn deterministic_clock_advances() {
        let mut clock = DeterministicClock::new(10);
        assert_eq!(clock.now(), 10);
        clock.advance(5);
        assert_eq!(clock.now(), 15);
    }

    #[test]
    fn input_queue_is_deterministic() {
        let mut queue = InputQueue::new();
        queue.push(InputEvent {
            time: 5,
            code: 1,
            value: 0,
        });
        queue.push(InputEvent {
            time: 3,
            code: 2,
            value: 1,
        });
        queue.push(InputEvent {
            time: 5,
            code: 3,
            value: 2,
        });
        let ready = queue.drain_ready(5);
        let codes: Vec<u32> = ready.into_iter().map(|event| event.code).collect();
        assert_eq!(codes, vec![2, 1, 3]);
        assert_eq!(queue.pending(), 0);
    }
}
