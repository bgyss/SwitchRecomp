# DKCR HD Boot Path (Scaffold)

This document sketches a first-level boot path using the new runtime stubs. The goal is to
capture the minimal service, graphics, audio, and input wiring needed to reach the first
playable level without bundling proprietary assets.

## Boot Flow Summary
- Mount external RomFS at `game-data/dkcr-hd/romfs`.
- Initialize stub services (SM, FS, VI, HID, audio) for early boot.
- Submit placeholder graphics commands and present frames.
- Submit placeholder audio buffers.
- Queue deterministic input frames.

## Runtime Stub Shape
The runtime exposes a small boot scaffold that records steps and uses stub backends for
services, graphics, audio, and input.

```rust
use recomp_runtime::{
    AudioBuffer, BootContext, BootPlan, FrameDescriptor, InputEvent, InputFrame,
    ServiceCallSpec, ServiceStubSpec, StubBehavior,
};
use recomp_runtime::{CommandStream};

let mut boot = BootContext::new("DKCR HD Sample")
    .with_assets_root("game-data/dkcr-hd/romfs");

boot.register_service_stubs(&[
    ServiceStubSpec::new("svc_sm", StubBehavior::Log),
    ServiceStubSpec::new("svc_fs", StubBehavior::Log),
    ServiceStubSpec::new("svc_vi", StubBehavior::Log),
    ServiceStubSpec::new("svc_hid", StubBehavior::Log),
    ServiceStubSpec::new("svc_audout", StubBehavior::Log),
]);

let plan = BootPlan::new()
    .service_call(ServiceCallSpec::new("svc_sm", vec![]))
    .service_call(ServiceCallSpec::new("svc_fs", vec![]))
    .gfx_stream(CommandStream::new(vec![0xdead_beef]))
    .present(FrameDescriptor::new(1, 1280, 720))
    .audio(AudioBuffer::new(256, 2, 48_000))
    .input(InputFrame::new(0, vec![InputEvent { time: 0, code: 1, value: 1 }]));

let trace = boot.run_plan(&plan).expect("boot plan");
println!("boot steps: {}", trace.steps().len());
```

## Notes
- `samples/dkcr-hd/title.toml` contains stub mappings and the RomFS path.
- `samples/dkcr-hd/patches/first-level.toml` records placeholder patches for the first level.
- Replace stub service calls with real implementations as the pipeline matures.
