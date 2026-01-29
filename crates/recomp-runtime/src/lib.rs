use std::fmt;

pub const ABI_VERSION: &str = "0.1.0";

pub use recomp_gfx::{CommandStream, GraphicsBackend, GraphicsError, StubBackend};
pub use recomp_services::{
    stub_handler, ServiceAccessControl, ServiceCall, ServiceError, ServiceLogger, ServiceRegistry,
    StubBehavior,
};
pub use recomp_timing::Scheduler;

#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("stubbed syscall: {name}")]
    StubbedSyscall { name: String },
}

pub type RuntimeResult<T> = Result<T, RuntimeError>;

pub fn abi_version() -> &'static str {
    ABI_VERSION
}

pub fn init() {
    // Placeholder for future runtime init work (logging, timing, etc.).
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
        match err {
            RuntimeError::StubbedSyscall { name } => {
                assert_eq!(name, "svc_test");
            }
        }
    }
}
