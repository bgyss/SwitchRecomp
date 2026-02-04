use crate::homebrew::{InputEvent, InputQueue};

#[derive(Debug, Clone)]
pub struct InputFrame {
    pub time: u64,
    pub events: Vec<InputEvent>,
}

impl InputFrame {
    pub fn new(time: u64, events: Vec<InputEvent>) -> Self {
        Self { time, events }
    }
}

pub trait InputBackend {
    fn push_frame(&mut self, frame: InputFrame);
    fn drain_ready(&mut self, time: u64) -> Vec<InputEvent>;
}

#[derive(Debug, Default)]
pub struct StubInputBackend {
    queue: InputQueue,
    pub pushed: Vec<InputFrame>,
}

impl StubInputBackend {
    pub fn pending(&self) -> usize {
        self.queue.pending()
    }
}

impl InputBackend for StubInputBackend {
    fn push_frame(&mut self, frame: InputFrame) {
        for event in &frame.events {
            self.queue.push(InputEvent {
                time: event.time,
                code: event.code,
                value: event.value,
            });
        }
        self.pushed.push(frame);
    }

    fn drain_ready(&mut self, time: u64) -> Vec<InputEvent> {
        self.queue.drain_ready(time)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_input_backend_records_frames_and_events() {
        let mut backend = StubInputBackend::default();
        backend.push_frame(InputFrame::new(
            0,
            vec![InputEvent {
                time: 1,
                code: 10,
                value: 1,
            }],
        ));
        assert_eq!(backend.pushed.len(), 1);
        assert_eq!(backend.pending(), 1);
        let ready = backend.drain_ready(1);
        assert_eq!(ready.len(), 1);
        assert_eq!(backend.pending(), 0);
    }
}
