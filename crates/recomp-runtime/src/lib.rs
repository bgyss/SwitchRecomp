use std::fmt;

mod audio;
mod boot;
mod homebrew;
mod input;
mod memory;

pub const ABI_VERSION: &str = "0.1.0";

pub use audio::{AudioBackend, AudioBuffer, AudioError, StubAudioBackend};
pub use boot::{
    BootAssets, BootContext, BootError, BootPlan, BootStep, BootTrace, ServiceCallSpec,
};
pub use homebrew::{
    entrypoint_shim, DeterministicClock, InputEvent, InputQueue, LoaderConfig, LoaderConfigBuilder,
    LoaderConfigEntry, LoaderConfigKey, NroEntrypoint, RuntimeManifest, ServiceStub, NRO_ENTRY_X1,
};
pub use input::{InputBackend, InputFrame, StubInputBackend};
pub use memory::{
    init_memory, recomp_mem_load_u16, recomp_mem_load_u32, recomp_mem_load_u64, recomp_mem_load_u8,
    recomp_mem_store_u16, recomp_mem_store_u32, recomp_mem_store_u64, recomp_mem_store_u8,
    MemoryInitSegment, MemoryLayout, MemoryLayoutError, MemoryPermissions, MemoryRegionSpec,
    MemoryStatus, MemoryZeroSegment,
};
pub use recomp_gfx::{
    CommandStream, FrameDescriptor, GraphicsBackend, GraphicsError, GraphicsPresenter, StubBackend,
    StubPresenter,
};
pub use recomp_services::{
    register_stubbed_services, stub_handler, ServiceAccessControl, ServiceCall, ServiceError,
    ServiceLogger, ServiceRegistry, ServiceStubSpec, StubBehavior,
};
pub use recomp_timing::Scheduler;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerformanceMode {
    Handheld,
    Docked,
}

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub performance_mode: PerformanceMode,
}

impl RuntimeConfig {
    pub fn new(performance_mode: PerformanceMode) -> Self {
        Self { performance_mode }
    }
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            performance_mode: PerformanceMode::Handheld,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("stubbed syscall: {name}")]
    StubbedSyscall { name: String },
    #[error("missing loader config key: {key:?}")]
    MissingLoaderConfigKey { key: LoaderConfigKey },
    #[error("runtime manifest serialization failed: {message}")]
    ManifestSerialize { message: String },
    #[error("io error: {message}")]
    Io { message: String },
    #[error("memory layout error: {0}")]
    MemoryLayout(#[from] MemoryLayoutError),
    #[error("memory access error {status:?} at {address:#x} size {size}")]
    MemoryAccess {
        status: MemoryStatus,
        address: u64,
        size: usize,
    },
}

pub type RuntimeResult<T> = Result<T, RuntimeError>;

pub fn abi_version() -> &'static str {
    ABI_VERSION
}

pub fn init(config: &RuntimeConfig) {
    println!(
        "[recomp-runtime] init performance_mode={:?}",
        config.performance_mode
    );
}

pub fn init_default_memory(layout: MemoryLayout) -> RuntimeResult<()> {
    init_memory(layout)?;
    Ok(())
}

pub fn apply_memory_image(
    init_segments: &[MemoryInitSegment],
    zero_segments: &[MemoryZeroSegment],
) -> RuntimeResult<()> {
    memory::apply_memory_image(init_segments, zero_segments).map_err(|status| {
        RuntimeError::MemoryAccess {
            status,
            address: 0,
            size: 0,
        }
    })
}

pub fn mem_load_u8(address: u64) -> RuntimeResult<u64> {
    memory::mem_load_u8(address).map_err(|status| RuntimeError::MemoryAccess {
        status,
        address,
        size: 1,
    })
}

pub fn mem_load_u16(address: u64) -> RuntimeResult<u64> {
    memory::mem_load_u16(address).map_err(|status| RuntimeError::MemoryAccess {
        status,
        address,
        size: 2,
    })
}

pub fn mem_load_u32(address: u64) -> RuntimeResult<u64> {
    memory::mem_load_u32(address).map_err(|status| RuntimeError::MemoryAccess {
        status,
        address,
        size: 4,
    })
}

pub fn mem_load_u64(address: u64) -> RuntimeResult<u64> {
    memory::mem_load_u64(address).map_err(|status| RuntimeError::MemoryAccess {
        status,
        address,
        size: 8,
    })
}

pub fn mem_store_u8(address: u64, value: u64) -> RuntimeResult<()> {
    memory::mem_store_u8(address, value).map_err(|status| RuntimeError::MemoryAccess {
        status,
        address,
        size: 1,
    })
}

pub fn mem_store_u16(address: u64, value: u64) -> RuntimeResult<()> {
    memory::mem_store_u16(address, value).map_err(|status| RuntimeError::MemoryAccess {
        status,
        address,
        size: 2,
    })
}

pub fn mem_store_u32(address: u64, value: u64) -> RuntimeResult<()> {
    memory::mem_store_u32(address, value).map_err(|status| RuntimeError::MemoryAccess {
        status,
        address,
        size: 4,
    })
}

pub fn mem_store_u64(address: u64, value: u64) -> RuntimeResult<()> {
    memory::mem_store_u64(address, value).map_err(|status| RuntimeError::MemoryAccess {
        status,
        address,
        size: 8,
    })
}

pub fn syscall_log(name: &str, args: &[i64]) -> RuntimeResult<()> {
    println!("[recomp-runtime] syscall {name} args={}", ArgsDisplay(args));
    Ok(())
}

pub fn syscall_noop(_name: &str, _args: &[i64]) -> RuntimeResult<()> {
    Ok(())
}

pub fn syscall_panic(name: &str, _args: &[i64]) -> RuntimeResult<()> {
    Err(RuntimeError::StubbedSyscall {
        name: name.to_string(),
    })
}

pub struct Runtime {
    pub scheduler: Scheduler,
    pub services: ServiceRegistry,
    pub access: ServiceAccessControl,
    pub logger: ServiceLogger,
}

impl Runtime {
    pub fn new() -> Self {
        Self {
            scheduler: Scheduler::new(),
            services: ServiceRegistry::new(),
            access: ServiceAccessControl::default(),
            logger: ServiceLogger::default(),
        }
    }

    pub fn dispatch_service(&self, call: &ServiceCall) -> Result<(), ServiceError> {
        self.access.check(call)?;
        self.logger.log_call(call);
        self.services.call(call)
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

struct ArgsDisplay<'a>(&'a [i64]);

impl fmt::Display for ArgsDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[")?;
        for (idx, value) in self.0.iter().enumerate() {
            if idx > 0 {
                f.write_str(", ")?;
            }
            write!(f, "{value}")?;
        }
        f.write_str("]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_syscall_is_ok() {
        assert!(syscall_noop("svc_test", &[]).is_ok());
    }

    #[test]
    fn panic_syscall_returns_error() {
        let err = syscall_panic("svc_test", &[]).unwrap_err();
        assert!(matches!(
            err,
            RuntimeError::StubbedSyscall { name } if name == "svc_test"
        ));
    }

    #[test]
    fn dispatch_service_respects_access_control() {
        let mut runtime = Runtime::new();
        runtime.services.register("svc_ok", |_| Ok(()));
        runtime.access = ServiceAccessControl::from_allowed(vec!["svc_ok".to_string()]);

        let ok_call = ServiceCall {
            client: "demo".to_string(),
            service: "svc_ok".to_string(),
            args: vec![],
        };
        assert!(runtime.dispatch_service(&ok_call).is_ok());

        let bad_call = ServiceCall {
            client: "demo".to_string(),
            service: "svc_bad".to_string(),
            args: vec![],
        };
        let err = runtime.dispatch_service(&bad_call).unwrap_err();
        assert!(matches!(err, ServiceError::AccessDenied(_)));
    }

    #[test]
    fn runtime_config_defaults_to_handheld() {
        let config = RuntimeConfig::default();
        assert!(matches!(config.performance_mode, PerformanceMode::Handheld));
    }
}
