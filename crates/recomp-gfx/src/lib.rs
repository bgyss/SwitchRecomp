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
}
