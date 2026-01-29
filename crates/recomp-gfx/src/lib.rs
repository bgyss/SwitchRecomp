#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandStream {
    pub words: Vec<u32>,
}

impl CommandStream {
    pub fn new(words: Vec<u32>) -> Self {
        Self { words }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum GraphicsError {
    #[error("unsupported command stream")]
    Unsupported,
}

pub trait GraphicsBackend {
    fn submit(&mut self, stream: &CommandStream) -> Result<(), GraphicsError>;
}

pub fn checksum_stream(stream: &CommandStream) -> u64 {
    let mut hash = 1469598103934665603u64;
    for word in &stream.words {
        hash ^= *word as u64;
        hash = hash.wrapping_mul(1099511628211);
    }
    hash
}

#[derive(Debug, Default)]
pub struct StubBackend {
    pub submitted: Vec<CommandStream>,
}

impl GraphicsBackend for StubBackend {
    fn submit(&mut self, stream: &CommandStream) -> Result<(), GraphicsError> {
        self.submitted.push(stream.clone());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_backend_records_stream() {
        let mut backend = StubBackend::default();
        let stream = CommandStream::new(vec![1, 2, 3]);
        backend.submit(&stream).expect("submit ok");
        assert_eq!(backend.submitted.len(), 1);
        assert_eq!(backend.submitted[0].words, vec![1, 2, 3]);
    }

    #[test]
    fn checksum_is_stable() {
        let stream = CommandStream::new(vec![10, 20, 30]);
        let first = checksum_stream(&stream);
        let second = checksum_stream(&stream);
        assert_eq!(first, second);
    }
}
