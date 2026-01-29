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
}
