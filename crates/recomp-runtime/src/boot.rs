use crate::audio::{AudioBackend, AudioBuffer, AudioError, StubAudioBackend};
use crate::input::{InputBackend, InputFrame, StubInputBackend};
use crate::Runtime;
use recomp_gfx::{
    CommandStream, FrameDescriptor, GraphicsBackend, GraphicsError, GraphicsPresenter, StubBackend,
    StubPresenter,
};
use recomp_services::{register_stubbed_services, ServiceCall, ServiceError, ServiceStubSpec};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootStep {
    pub stage: String,
    pub detail: String,
}

#[derive(Debug, Default, Clone)]
pub struct BootTrace {
    steps: Vec<BootStep>,
}

impl BootTrace {
    pub fn record(&mut self, stage: impl Into<String>, detail: impl Into<String>) {
        self.steps.push(BootStep {
            stage: stage.into(),
            detail: detail.into(),
        });
    }

    pub fn steps(&self) -> &[BootStep] {
        &self.steps
    }
}

#[derive(Debug, Clone)]
pub struct BootAssets {
    pub romfs_root: PathBuf,
}

impl Default for BootAssets {
    fn default() -> Self {
        Self {
            romfs_root: PathBuf::from("game-data/romfs"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ServiceCallSpec {
    pub service: String,
    pub args: Vec<i64>,
}

impl ServiceCallSpec {
    pub fn new(service: impl Into<String>, args: Vec<i64>) -> Self {
        Self {
            service: service.into(),
            args,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct BootPlan {
    pub service_calls: Vec<ServiceCallSpec>,
    pub gfx_streams: Vec<CommandStream>,
    pub present_frames: Vec<FrameDescriptor>,
    pub audio_buffers: Vec<AudioBuffer>,
    pub input_frames: Vec<InputFrame>,
}

impl BootPlan {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn service_call(mut self, call: ServiceCallSpec) -> Self {
        self.service_calls.push(call);
        self
    }

    pub fn gfx_stream(mut self, stream: CommandStream) -> Self {
        self.gfx_streams.push(stream);
        self
    }

    pub fn present(mut self, frame: FrameDescriptor) -> Self {
        self.present_frames.push(frame);
        self
    }

    pub fn audio(mut self, buffer: AudioBuffer) -> Self {
        self.audio_buffers.push(buffer);
        self
    }

    pub fn input(mut self, frame: InputFrame) -> Self {
        self.input_frames.push(frame);
        self
    }
}

#[derive(Debug, thiserror::Error)]
pub enum BootError {
    #[error("service error: {0}")]
    Service(#[from] ServiceError),
    #[error("graphics error: {0}")]
    Graphics(#[from] GraphicsError),
    #[error("audio error: {0}")]
    Audio(#[from] AudioError),
}

pub struct BootContext {
    pub title: String,
    pub assets: BootAssets,
    pub runtime: Runtime,
    pub gfx: StubBackend,
    pub presenter: StubPresenter,
    pub audio: StubAudioBackend,
    pub input: StubInputBackend,
    pub trace: BootTrace,
}

impl BootContext {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            assets: BootAssets::default(),
            runtime: Runtime::new(),
            gfx: StubBackend::default(),
            presenter: StubPresenter::default(),
            audio: StubAudioBackend::default(),
            input: StubInputBackend::default(),
            trace: BootTrace::default(),
        }
    }

    pub fn with_assets_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.assets.romfs_root = root.into();
        self
    }

    pub fn register_service_stubs(&mut self, stubs: &[ServiceStubSpec]) {
        register_stubbed_services(&mut self.runtime.services, stubs);
        self.trace
            .record("services.register", format!("count={}", stubs.len()));
    }

    pub fn run_plan(&mut self, plan: &BootPlan) -> Result<BootTrace, BootError> {
        self.trace
            .record("boot.start", format!("title={}", self.title));
        self.trace
            .record("assets.romfs", self.assets.romfs_root.display().to_string());

        for call in &plan.service_calls {
            let call = ServiceCall {
                client: "boot".to_string(),
                service: call.service.clone(),
                args: call.args.clone(),
            };
            self.runtime.dispatch_service(&call)?;
            self.trace.record("service.call", call.service);
        }

        for stream in &plan.gfx_streams {
            self.gfx.submit(stream)?;
            self.trace
                .record("gfx.submit", format!("words={}", stream.words.len()));
        }

        for frame in &plan.present_frames {
            self.presenter.present(frame)?;
            self.trace
                .record("gfx.present", format!("frame={}", frame.frame_id));
        }

        for buffer in &plan.audio_buffers {
            self.audio.submit(buffer)?;
            self.trace
                .record("audio.submit", format!("frames={}", buffer.frames));
        }

        for frame in &plan.input_frames {
            self.input.push_frame(frame.clone());
            self.trace
                .record("input.frame", format!("events={}", frame.events.len()));
        }

        Ok(self.trace.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InputEvent;
    use recomp_services::StubBehavior;

    #[test]
    fn boot_context_runs_plan_and_records() {
        let mut context =
            BootContext::new("DKCR HD Sample").with_assets_root("game-data/dkcr-hd/romfs");
        context.register_service_stubs(&[
            ServiceStubSpec::new("svc_sm", StubBehavior::Noop),
            ServiceStubSpec::new("svc_fs", StubBehavior::Noop),
        ]);

        let plan = BootPlan::new()
            .service_call(ServiceCallSpec::new("svc_sm", vec![]))
            .service_call(ServiceCallSpec::new("svc_fs", vec![1]))
            .gfx_stream(CommandStream::new(vec![1, 2, 3]))
            .present(FrameDescriptor::new(1, 1280, 720))
            .audio(AudioBuffer::new(256, 2, 48_000))
            .input(InputFrame::new(
                0,
                vec![InputEvent {
                    time: 0,
                    code: 1,
                    value: 1,
                }],
            ));

        let trace = context.run_plan(&plan).expect("boot plan");
        assert!(trace.steps().len() >= 7);
        assert_eq!(context.gfx.submitted.len(), 1);
        assert_eq!(context.presenter.presented.len(), 1);
        assert_eq!(context.audio.submitted.len(), 1);
        assert_eq!(context.input.pending(), 1);
    }
}
