use std::cmp::Ordering;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub time: u64,
    pub id: u64,
    pub label: String,
}

#[derive(Debug, Default)]
pub struct Scheduler {
    next_id: u64,
    events: Vec<Event>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn schedule(&mut self, time: u64, label: &str) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.events.push(Event {
            time,
            id,
            label: label.to_string(),
        });
        id
    }

    pub fn run_until<F>(&mut self, time: u64, mut handler: F)
    where
        F: FnMut(&Event),
    {
        self.events.sort_by(|a, b| match a.time.cmp(&b.time) {
            Ordering::Equal => a.id.cmp(&b.id),
            other => other,
        });

        let mut remaining = Vec::new();
        for event in self.events.drain(..) {
            if event.time <= time {
                handler(&event);
            } else {
                remaining.push(event);
            }
        }
        self.events = remaining;
    }

    pub fn pending(&self) -> usize {
        self.events.len()
    }
}

#[derive(Debug, Default)]
pub struct TraceRecorder {
    events: Vec<Event>,
}

impl TraceRecorder {
    pub fn record(&mut self, event: &Event) {
        self.events.push(event.clone());
    }

    pub fn snapshot(&self) -> Vec<Event> {
        self.events.clone()
    }
}

#[derive(Debug, Clone)]
pub struct TraceReplayer {
    events: Vec<Event>,
    cursor: usize,
}

impl TraceReplayer {
    pub fn new(events: Vec<Event>) -> Self {
        Self { events, cursor: 0 }
    }

    pub fn from_slice(events: &[Event]) -> Self {
        Self::new(events.to_vec())
    }

    pub fn replay_until<F>(&mut self, time: u64, mut handler: F)
    where
        F: FnMut(&Event),
    {
        while self.cursor < self.events.len() {
            let event = &self.events[self.cursor];
            if event.time <= time {
                handler(event);
                self.cursor += 1;
            } else {
                break;
            }
        }
    }

    pub fn remaining(&self) -> usize {
        self.events.len().saturating_sub(self.cursor)
    }

    pub fn reset(&mut self) {
        self.cursor = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scheduler_orders_by_time_then_id() {
        let mut scheduler = Scheduler::new();
        scheduler.schedule(10, "late");
        scheduler.schedule(5, "early");
        scheduler.schedule(5, "early2");

        let mut labels = Vec::new();
        scheduler.run_until(10, |event| labels.push(event.label.clone()));

        assert_eq!(labels, vec!["early", "early2", "late"]);
        assert_eq!(scheduler.pending(), 0);
    }

    #[test]
    fn recorder_produces_deterministic_trace() {
        let mut scheduler = Scheduler::new();
        scheduler.schedule(5, "alpha");
        scheduler.schedule(1, "beta");
        scheduler.schedule(5, "gamma");

        let mut recorder = TraceRecorder::default();
        scheduler.run_until(10, |event| recorder.record(event));
        let first = recorder.snapshot();

        let mut scheduler = Scheduler::new();
        scheduler.schedule(5, "alpha");
        scheduler.schedule(1, "beta");
        scheduler.schedule(5, "gamma");
        let mut recorder = TraceRecorder::default();
        scheduler.run_until(10, |event| recorder.record(event));
        let second = recorder.snapshot();

        assert_eq!(first, second);
    }

    #[test]
    fn replayer_replays_trace_in_chunks() {
        let mut scheduler = Scheduler::new();
        scheduler.schedule(2, "tick-1");
        scheduler.schedule(4, "tick-2");
        scheduler.schedule(4, "tick-3");

        let mut recorder = TraceRecorder::default();
        scheduler.run_until(10, |event| recorder.record(event));
        let trace = recorder.snapshot();

        let mut replayer = TraceReplayer::from_slice(&trace);
        let mut labels = Vec::new();

        replayer.replay_until(2, |event| labels.push(event.label.clone()));
        assert_eq!(labels, vec!["tick-1"]);
        assert_eq!(replayer.remaining(), 2);

        replayer.replay_until(4, |event| labels.push(event.label.clone()));
        assert_eq!(labels, vec!["tick-1", "tick-2", "tick-3"]);
        assert_eq!(replayer.remaining(), 0);
    }

    #[test]
    fn replayer_can_reset_and_replay() {
        let events = vec![
            Event {
                time: 1,
                id: 0,
                label: "alpha".to_string(),
            },
            Event {
                time: 3,
                id: 1,
                label: "beta".to_string(),
            },
        ];

        let mut replayer = TraceReplayer::new(events);
        let mut labels = Vec::new();
        replayer.replay_until(3, |event| labels.push(event.label.clone()));
        assert_eq!(labels, vec!["alpha", "beta"]);

        replayer.reset();
        labels.clear();
        replayer.replay_until(1, |event| labels.push(event.label.clone()));
        assert_eq!(labels, vec!["alpha"]);
        assert_eq!(replayer.remaining(), 1);
    }
}
