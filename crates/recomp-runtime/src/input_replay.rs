use crate::{InputBackend, InputEvent, InputFrame};
use serde::Deserialize;
use std::collections::HashSet;

const INPUT_SCRIPT_SCHEMA_VERSION: &str = "1";

#[derive(Debug, Deserialize, Clone)]
pub struct InputScript {
    pub schema_version: String,
    pub metadata: InputMetadata,
    pub events: Vec<InputScriptEvent>,
    #[serde(default)]
    pub markers: Vec<InputScriptMarker>,
}

impl InputScript {
    pub fn parse(toml_src: &str) -> Result<Self, String> {
        let script: InputScript =
            toml::from_str(toml_src).map_err(|err| format!("invalid input script: {err}"))?;
        script.validate()?;
        Ok(script)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != INPUT_SCRIPT_SCHEMA_VERSION {
            return Err(format!(
                "unsupported input script schema version: {}",
                self.schema_version
            ));
        }
        if self.metadata.title.trim().is_empty()
            || self.metadata.controller.trim().is_empty()
            || self.metadata.timing_mode == TimingMode::Unspecified
        {
            return Err("input script metadata is incomplete".to_string());
        }
        if self.events.is_empty() {
            return Err("input script events list is empty".to_string());
        }

        for (index, event) in self.events.iter().enumerate() {
            let label = format!("event[{index}]");
            validate_time_fields(
                &label,
                self.metadata.timing_mode,
                event.time_ms,
                event.frame,
            )?;
        }

        let mut names = HashSet::new();
        for (index, marker) in self.markers.iter().enumerate() {
            let label = format!("marker[{index}]");
            if marker.name.trim().is_empty() {
                return Err(format!("{label} name is empty"));
            }
            if !names.insert(marker.name.as_str()) {
                return Err(format!("{label} name is duplicated"));
            }
            validate_time_fields(
                &label,
                self.metadata.timing_mode,
                marker.time_ms,
                marker.frame,
            )?;
        }

        Ok(())
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct InputMetadata {
    pub title: String,
    pub controller: String,
    pub timing_mode: TimingMode,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub recorded_at: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TimingMode {
    #[serde(rename = "ms")]
    Milliseconds,
    Frames,
    #[serde(other)]
    Unspecified,
}

#[derive(Debug, Deserialize, Clone)]
pub struct InputScriptEvent {
    #[serde(default)]
    pub time_ms: Option<u64>,
    #[serde(default)]
    pub frame: Option<u64>,
    pub control: u32,
    pub value: i32,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct InputScriptMarker {
    pub name: String,
    #[serde(default)]
    pub time_ms: Option<u64>,
    #[serde(default)]
    pub frame: Option<u64>,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputMarker {
    pub name: String,
    pub time: u64,
    pub note: Option<String>,
}

#[derive(Debug, Clone)]
pub struct InputPlayback {
    timing_mode: TimingMode,
    frames: Vec<InputFrame>,
    markers: Vec<InputMarker>,
    cursor: usize,
}

impl InputPlayback {
    pub fn from_script(script: InputScript) -> Result<Self, String> {
        script.validate()?;

        let timing_mode = script.metadata.timing_mode;
        let mut sequenced = Vec::with_capacity(script.events.len());
        for (index, event) in script.events.into_iter().enumerate() {
            let time = match timing_mode {
                TimingMode::Milliseconds => event.time_ms.expect("validated"),
                TimingMode::Frames => event.frame.expect("validated"),
                TimingMode::Unspecified => {
                    return Err("input script timing mode is unspecified".to_string())
                }
            };
            let input_event = InputEvent {
                time,
                code: event.control,
                value: event.value,
            };
            sequenced.push(SequencedEvent {
                time,
                index,
                event: input_event,
            });
        }

        sequenced.sort_by(|a, b| a.time.cmp(&b.time).then_with(|| a.index.cmp(&b.index)));

        let mut frames: Vec<InputFrame> = Vec::new();
        for item in sequenced {
            if let Some(frame) = frames.last_mut() {
                if frame.time == item.time {
                    frame.events.push(item.event);
                    continue;
                }
            }
            frames.push(InputFrame::new(item.time, vec![item.event]));
        }

        let mut markers: Vec<SequencedMarker> = script
            .markers
            .into_iter()
            .enumerate()
            .map(|(index, marker)| {
                let time = match timing_mode {
                    TimingMode::Milliseconds => marker.time_ms.expect("validated"),
                    TimingMode::Frames => marker.frame.expect("validated"),
                    TimingMode::Unspecified => 0,
                };
                SequencedMarker {
                    time,
                    index,
                    marker: InputMarker {
                        name: marker.name,
                        time,
                        note: marker.note,
                    },
                }
            })
            .collect();

        markers.sort_by(|a, b| a.time.cmp(&b.time).then_with(|| a.index.cmp(&b.index)));

        Ok(Self {
            timing_mode,
            frames,
            markers: markers.into_iter().map(|entry| entry.marker).collect(),
            cursor: 0,
        })
    }

    pub fn timing_mode(&self) -> TimingMode {
        self.timing_mode
    }

    pub fn frames(&self) -> &[InputFrame] {
        &self.frames
    }

    pub fn markers(&self) -> &[InputMarker] {
        &self.markers
    }

    pub fn reset(&mut self) {
        self.cursor = 0;
    }

    pub fn seek(&mut self, time: u64) {
        let mut index = 0;
        while index < self.frames.len() && self.frames[index].time < time {
            index += 1;
        }
        self.cursor = index;
    }

    pub fn is_finished(&self) -> bool {
        self.cursor >= self.frames.len()
    }

    pub fn feed_until<B: InputBackend>(&mut self, backend: &mut B, time: u64) -> usize {
        let mut pushed = 0;
        while self.cursor < self.frames.len() && self.frames[self.cursor].time <= time {
            backend.push_frame(self.frames[self.cursor].clone());
            self.cursor += 1;
            pushed += 1;
        }
        pushed
    }
}

#[derive(Debug)]
struct SequencedEvent {
    time: u64,
    index: usize,
    event: InputEvent,
}

#[derive(Debug)]
struct SequencedMarker {
    time: u64,
    index: usize,
    marker: InputMarker,
}

fn validate_time_fields(
    label: &str,
    timing_mode: TimingMode,
    time_ms: Option<u64>,
    frame: Option<u64>,
) -> Result<(), String> {
    match timing_mode {
        TimingMode::Milliseconds => {
            if time_ms.is_none() {
                return Err(format!("{label} missing time_ms for timing_mode=ms"));
            }
            if frame.is_some() {
                return Err(format!("{label} frame is not valid for timing_mode=ms"));
            }
        }
        TimingMode::Frames => {
            if frame.is_none() {
                return Err(format!("{label} missing frame for timing_mode=frames"));
            }
            if time_ms.is_some() {
                return Err(format!(
                    "{label} time_ms is not valid for timing_mode=frames"
                ));
            }
        }
        TimingMode::Unspecified => {
            return Err(format!("{label} timing_mode is unspecified"));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn playback_orders_events_deterministically() {
        let toml_src = r#"
            schema_version = "1"

            [metadata]
            title = "Replay"
            controller = "pro_controller"
            timing_mode = "ms"

            [[events]]
            time_ms = 20
            control = 10
            value = 1

            [[events]]
            time_ms = 10
            control = 20
            value = 1

            [[events]]
            time_ms = 10
            control = 30
            value = 0
        "#;

        let script = InputScript::parse(toml_src).expect("parse script");
        let playback = InputPlayback::from_script(script).expect("build playback");
        let frames = playback.frames();
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].time, 10);
        assert_eq!(frames[0].events.len(), 2);
        assert_eq!(frames[0].events[0].code, 20);
        assert_eq!(frames[0].events[1].code, 30);
        assert_eq!(frames[1].time, 20);
    }

    #[test]
    fn playback_sorts_markers_by_time() {
        let toml_src = r#"
            schema_version = "1"

            [metadata]
            title = "Replay"
            controller = "pro_controller"
            timing_mode = "ms"

            [[events]]
            time_ms = 0
            control = 1
            value = 1

            [[markers]]
            name = "late"
            time_ms = 300

            [[markers]]
            name = "boot"
            time_ms = 0

            [[markers]]
            name = "mid"
            time_ms = 150
        "#;

        let script = InputScript::parse(toml_src).expect("parse script");
        let playback = InputPlayback::from_script(script).expect("build playback");
        let markers = playback.markers();
        assert_eq!(markers.len(), 3);
        assert_eq!(markers[0].name, "boot");
        assert_eq!(markers[0].time, 0);
        assert_eq!(markers[1].name, "mid");
        assert_eq!(markers[1].time, 150);
        assert_eq!(markers[2].name, "late");
        assert_eq!(markers[2].time, 300);
    }
}
