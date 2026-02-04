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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandStreamReport {
    pub word_count: usize,
    pub checksum: u64,
}

pub fn checksum_stream(stream: &CommandStream) -> u64 {
    let mut hash = 1469598103934665603u64;
    for word in &stream.words {
        hash ^= *word as u64;
        hash = hash.wrapping_mul(1099511628211);
    }
    hash
}

pub fn report_stream(stream: &CommandStream) -> CommandStreamReport {
    CommandStreamReport {
        word_count: stream.words.len(),
        checksum: checksum_stream(stream),
    }
}

pub fn validate_stream(stream: &CommandStream) -> Result<CommandStreamReport, GraphicsError> {
    if stream.words.is_empty() {
        return Err(GraphicsError::Unsupported);
    }
    Ok(report_stream(stream))
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

    #[test]
    fn report_includes_word_count_and_checksum() {
        let stream = CommandStream::new(vec![4, 5, 6, 7]);
        let report = report_stream(&stream);
        assert_eq!(report.word_count, 4);
        assert_eq!(report.checksum, checksum_stream(&stream));
    }

    #[test]
    fn validation_rejects_empty_stream() {
        let stream = CommandStream::new(Vec::new());
        assert!(validate_stream(&stream).is_err());
    }
}
