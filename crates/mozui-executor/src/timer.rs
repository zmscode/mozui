use std::rc::Rc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimerId(usize);

enum TimerKind {
    Once(Option<Box<dyn FnOnce(&mut dyn std::any::Any)>>),
    Repeating {
        callback: Rc<dyn Fn(&mut dyn std::any::Any)>,
        interval: Duration,
    },
}

struct TimerEntry {
    id: TimerId,
    deadline: Instant,
    kind: TimerKind,
}

/// Manages scheduled timers (one-shot and repeating).
pub struct TimerManager {
    timers: Vec<TimerEntry>,
    next_id: usize,
}

impl TimerManager {
    pub fn new() -> Self {
        Self {
            timers: Vec::new(),
            next_id: 0,
        }
    }

    /// Schedule a one-shot callback after a duration.
    pub fn set_timeout(
        &mut self,
        duration: Duration,
        callback: impl FnOnce(&mut dyn std::any::Any) + 'static,
    ) -> TimerId {
        let id = TimerId(self.next_id);
        self.next_id += 1;
        self.timers.push(TimerEntry {
            id,
            deadline: Instant::now() + duration,
            kind: TimerKind::Once(Some(Box::new(callback))),
        });
        id
    }

    /// Schedule a repeating callback at an interval.
    pub fn set_interval(
        &mut self,
        interval: Duration,
        callback: impl Fn(&mut dyn std::any::Any) + 'static,
    ) -> TimerId {
        let id = TimerId(self.next_id);
        self.next_id += 1;
        self.timers.push(TimerEntry {
            id,
            deadline: Instant::now() + interval,
            kind: TimerKind::Repeating {
                callback: Rc::new(callback),
                interval,
            },
        });
        id
    }

    /// Cancel a timer.
    pub fn cancel(&mut self, id: TimerId) {
        self.timers.retain(|t| t.id != id);
    }

    /// Fire all expired timers. Returns true if any fired.
    pub fn fire_expired(&mut self, cx: &mut dyn std::any::Any) -> bool {
        let now = Instant::now();
        let mut fired = false;
        let mut to_reschedule = Vec::new();

        let mut i = 0;
        while i < self.timers.len() {
            if self.timers[i].deadline <= now {
                let mut entry = self.timers.swap_remove(i);
                fired = true;

                match &mut entry.kind {
                    TimerKind::Once(callback) => {
                        if let Some(cb) = callback.take() {
                            cb(cx);
                        }
                    }
                    TimerKind::Repeating { callback, interval } => {
                        callback(cx);
                        to_reschedule.push(TimerEntry {
                            id: entry.id,
                            deadline: now + *interval,
                            kind: TimerKind::Repeating {
                                callback: callback.clone(),
                                interval: *interval,
                            },
                        });
                    }
                }
            } else {
                i += 1;
            }
        }

        self.timers.extend(to_reschedule);
        fired
    }

    /// Get time until the next timer fires (for event loop sleep).
    pub fn time_until_next(&self) -> Option<Duration> {
        let now = Instant::now();
        self.timers
            .iter()
            .map(|t| t.deadline.saturating_duration_since(now))
            .min()
    }

    /// Check if there are any active timers.
    pub fn has_timers(&self) -> bool {
        !self.timers.is_empty()
    }
}
