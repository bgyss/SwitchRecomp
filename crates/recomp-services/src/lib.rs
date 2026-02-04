use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct ServiceCall {
    pub client: String,
    pub service: String,
    pub args: Vec<i64>,
}

#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error("service not found: {0}")]
    NotFound(String),
    #[error("access denied: {0}")]
    AccessDenied(String),
    #[error("stubbed service: {0}")]
    Stubbed(String),
}

pub type ServiceResult<T> = Result<T, ServiceError>;

type ServiceHandler = Box<dyn Fn(&ServiceCall) -> ServiceResult<()> + Send + Sync>;

#[derive(Default)]
pub struct ServiceRegistry {
    handlers: BTreeMap<String, ServiceHandler>,
}

impl ServiceRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<F>(&mut self, name: &str, handler: F)
    where
        F: Fn(&ServiceCall) -> ServiceResult<()> + Send + Sync + 'static,
    {
        self.handlers.insert(name.to_string(), Box::new(handler));
    }

    pub fn call(&self, call: &ServiceCall) -> ServiceResult<()> {
        let handler = self
            .handlers
            .get(&call.service)
            .ok_or_else(|| ServiceError::NotFound(call.service.clone()))?;
        handler(call)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StubBehavior {
    Log,
    Noop,
    Panic,
}

pub fn stub_handler(
    behavior: StubBehavior,
) -> impl Fn(&ServiceCall) -> ServiceResult<()> + Send + Sync {
    move |call: &ServiceCall| match behavior {
        StubBehavior::Log => {
            println!(
                "[recomp-services] stub {service} args={:?}",
                call.args,
                service = call.service
            );
            Ok(())
        }
        StubBehavior::Noop => Ok(()),
        StubBehavior::Panic => Err(ServiceError::Stubbed(call.service.clone())),
    }
}

#[derive(Debug, Default)]
pub struct ServiceAccessControl {
    allowed: BTreeSet<String>,
}

impl ServiceAccessControl {
    pub fn from_allowed(names: impl IntoIterator<Item = String>) -> Self {
        Self {
            allowed: names.into_iter().collect(),
        }
    }

    pub fn check(&self, call: &ServiceCall) -> ServiceResult<()> {
        if self.allowed.is_empty() {
            return Ok(());
        }
        if self.allowed.contains(&call.service) {
            Ok(())
        } else {
            Err(ServiceError::AccessDenied(call.service.clone()))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceLogEntry {
    pub id: u64,
    pub client: String,
    pub service: String,
    pub args: Vec<i64>,
}

pub trait ServiceLogSink: Send + Sync {
    fn record(&self, entry: ServiceLogEntry);
}

#[derive(Debug, Default)]
pub struct StdoutLogSink;

impl ServiceLogSink for StdoutLogSink {
    fn record(&self, entry: ServiceLogEntry) {
        println!(
            "[recomp-services] #{id} client={} service={} args={:?}",
            entry.client,
            entry.service,
            entry.args,
            id = entry.id
        );
    }
}

pub struct ServiceLogger {
    counter: AtomicU64,
    sink: Arc<dyn ServiceLogSink>,
}

impl Default for ServiceLogger {
    fn default() -> Self {
        Self::new(Arc::new(StdoutLogSink))
    }
}

impl ServiceLogger {
    pub fn new<S>(sink: Arc<S>) -> Self
    where
        S: ServiceLogSink + 'static,
    {
        Self {
            counter: AtomicU64::new(0),
            sink,
        }
    }

    pub fn next_id(&self) -> u64 {
        self.counter.fetch_add(1, Ordering::SeqCst)
    }

    pub fn log_call(&self, call: &ServiceCall) {
        let entry = ServiceLogEntry {
            id: self.next_id(),
            client: call.client.clone(),
            service: call.service.clone(),
            args: call.args.clone(),
        };
        self.sink.record(entry);
    }
}

pub struct ServiceDispatcher {
    registry: ServiceRegistry,
    access: ServiceAccessControl,
    logger: ServiceLogger,
}

impl ServiceDispatcher {
    pub fn new(
        registry: ServiceRegistry,
        access: ServiceAccessControl,
        logger: ServiceLogger,
    ) -> Self {
        Self {
            registry,
            access,
            logger,
        }
    }

    pub fn dispatch(&self, call: &ServiceCall) -> ServiceResult<()> {
        self.access.check(call)?;
        self.logger.log_call(call);
        self.registry.call(call)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[test]
    fn registry_calls_handler() {
        let mut registry = ServiceRegistry::new();
        registry.register("svc_test", |_| Ok(()));

        let call = ServiceCall {
            client: "demo".to_string(),
            service: "svc_test".to_string(),
            args: vec![1, 2],
        };

        assert!(registry.call(&call).is_ok());
    }

    #[test]
    fn access_control_denies_unknown() {
        let access = ServiceAccessControl::from_allowed(vec!["svc_ok".to_string()]);
        let call = ServiceCall {
            client: "demo".to_string(),
            service: "svc_bad".to_string(),
            args: vec![],
        };

        let err = access.check(&call).unwrap_err();
        assert!(matches!(err, ServiceError::AccessDenied(_)));
    }

    #[test]
    fn stub_panic_returns_error() {
        let handler = stub_handler(StubBehavior::Panic);
        let call = ServiceCall {
            client: "demo".to_string(),
            service: "svc_stub".to_string(),
            args: vec![],
        };
        let err = handler(&call).unwrap_err();
        assert!(matches!(err, ServiceError::Stubbed(_)));
    }

    #[test]
    fn dispatcher_checks_access_and_logs() {
        let mut registry = ServiceRegistry::new();
        registry.register("svc_ok", |_| Ok(()));
        let access = ServiceAccessControl::from_allowed(vec!["svc_ok".to_string()]);
        let dispatcher = ServiceDispatcher::new(registry, access, ServiceLogger::default());
        let call = ServiceCall {
            client: "demo".to_string(),
            service: "svc_ok".to_string(),
            args: vec![42],
        };

        assert!(dispatcher.dispatch(&call).is_ok());
    }

    #[test]
    fn dispatcher_emits_structured_log_entry() {
        #[derive(Default)]
        struct TestLogSink {
            entries: Mutex<Vec<ServiceLogEntry>>,
        }

        impl TestLogSink {
            fn entries(&self) -> Vec<ServiceLogEntry> {
                self.entries.lock().unwrap().clone()
            }
        }

        impl ServiceLogSink for TestLogSink {
            fn record(&self, entry: ServiceLogEntry) {
                self.entries.lock().unwrap().push(entry);
            }
        }

        let sink = Arc::new(TestLogSink::default());
        let logger = ServiceLogger::new(Arc::clone(&sink));
        let mut registry = ServiceRegistry::new();
        registry.register("svc_ok", |_| Ok(()));
        let access = ServiceAccessControl::from_allowed(vec!["svc_ok".to_string()]);
        let dispatcher = ServiceDispatcher::new(registry, access, logger);
        let call = ServiceCall {
            client: "demo".to_string(),
            service: "svc_ok".to_string(),
            args: vec![7, 8, 9],
        };

        assert!(dispatcher.dispatch(&call).is_ok());

        let entries = sink.entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0],
            ServiceLogEntry {
                id: 0,
                client: "demo".to_string(),
                service: "svc_ok".to_string(),
                args: vec![7, 8, 9],
            }
        );
    }
}
