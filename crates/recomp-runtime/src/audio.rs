#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioBuffer {
    pub frames: u32,
    pub channels: u16,
    pub sample_rate: u32,
}

impl AudioBuffer {
    pub fn new(frames: u32, channels: u16, sample_rate: u32) -> Self {
        Self {
            frames,
            channels,
            sample_rate,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    #[error("unsupported audio buffer")]
    Unsupported,
}

pub trait AudioBackend {
    fn submit(&mut self, buffer: &AudioBuffer) -> Result<(), AudioError>;
}

#[derive(Debug, Default)]
pub struct StubAudioBackend {
    pub submitted: Vec<AudioBuffer>,
}

impl AudioBackend for StubAudioBackend {
    fn submit(&mut self, buffer: &AudioBuffer) -> Result<(), AudioError> {
        self.submitted.push(buffer.clone());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_audio_backend_records_buffers() {
        let mut backend = StubAudioBackend::default();
        let buffer = AudioBuffer::new(128, 2, 48_000);
        backend.submit(&buffer).expect("submit");
        assert_eq!(backend.submitted.len(), 1);
        assert_eq!(backend.submitted[0], buffer);
    }
}
