use std::collections::VecDeque;
use std::time::Instant;

/// Record of a single signal mutation.
#[derive(Debug, Clone)]
pub struct SignalMutation {
    pub slot_id: usize,
    pub type_name: &'static str,
    pub old_debug: String,
    pub new_debug: String,
    pub frame: u64,
    pub timestamp: Instant,
}

/// Ring buffer of recent signal mutations.
pub struct SignalLog {
    mutations: VecDeque<SignalMutation>,
    capacity: usize,
    pub current_frame: u64,
}

impl SignalLog {
    pub fn new(capacity: usize) -> Self {
        Self {
            mutations: VecDeque::with_capacity(capacity),
            capacity,
            current_frame: 0,
        }
    }

    pub fn push(&mut self, mutation: SignalMutation) {
        if self.mutations.len() >= self.capacity {
            self.mutations.pop_front();
        }
        self.mutations.push_back(mutation);
    }

    pub fn mutations_this_frame(&self, frame: u64) -> impl Iterator<Item = &SignalMutation> {
        self.mutations.iter().filter(move |m| m.frame == frame)
    }

    pub fn iter(&self) -> impl Iterator<Item = &SignalMutation> {
        self.mutations.iter()
    }

    pub fn clear(&mut self) {
        self.mutations.clear();
    }

    pub fn len(&self) -> usize {
        self.mutations.len()
    }

    pub fn is_empty(&self) -> bool {
        self.mutations.is_empty()
    }
}

/// Snapshot of a single signal's current state.
#[derive(Debug, Clone)]
pub struct SignalSnapshot {
    pub slot_id: usize,
    pub type_name: &'static str,
    pub value_debug: String,
    pub dirty: bool,
}
