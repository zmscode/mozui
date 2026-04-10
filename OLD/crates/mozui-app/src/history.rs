use std::time::{Duration, Instant};

/// Trait for items that can be stored in the undo/redo history.
pub trait HistoryItem: Clone + PartialEq {
    fn version(&self) -> usize;
    fn set_version(&mut self, version: usize);
}

/// Generic undo/redo history stack.
///
/// Supports grouping (batching multiple changes into one undo step within a
/// time interval) and unique mode (deduplicates consecutive identical entries).
pub struct History<I: HistoryItem> {
    undos: Vec<Vec<I>>,
    redos: Vec<Vec<I>>,
    version: usize,
    max_undos: usize,
    last_changed_at: Option<Instant>,
    group_interval: Duration,
    grouping: bool,
    unique: bool,
}

impl<I: HistoryItem> History<I> {
    pub fn new() -> Self {
        Self {
            undos: Vec::new(),
            redos: Vec::new(),
            version: 0,
            max_undos: 100,
            last_changed_at: None,
            group_interval: Duration::from_millis(500),
            grouping: false,
            unique: false,
        }
    }

    /// Set maximum number of undo steps.
    pub fn max_undos(mut self, n: usize) -> Self {
        self.max_undos = n;
        self
    }

    /// Enable unique mode — consecutive duplicate items are collapsed.
    pub fn unique(mut self) -> Self {
        self.unique = true;
        self
    }

    /// Set the time interval for automatic grouping.
    /// Changes within this interval are batched into a single undo step.
    pub fn group_interval(mut self, duration: Duration) -> Self {
        self.group_interval = duration;
        self
    }

    /// Begin explicit grouping. All `push()` calls until `end_grouping()`
    /// are batched into a single undo step.
    pub fn start_grouping(&mut self) {
        self.grouping = true;
    }

    /// End explicit grouping.
    pub fn end_grouping(&mut self) {
        self.grouping = false;
    }

    /// Push a new item onto the undo stack.
    /// Clears the redo stack (new edit branch).
    pub fn push(&mut self, mut item: I) {
        self.redos.clear();
        self.version += 1;
        item.set_version(self.version);

        // Check if we should group with the previous entry
        let now = Instant::now();
        let should_group = self.grouping
            || self
                .last_changed_at
                .map_or(false, |t| now.duration_since(t) < self.group_interval);

        if should_group {
            if let Some(group) = self.undos.last_mut() {
                if self.unique {
                    if group.last() != Some(&item) {
                        group.push(item);
                    }
                } else {
                    group.push(item);
                }
                self.last_changed_at = Some(now);
                return;
            }
        }

        // New group
        if self.unique {
            // Check last item in previous group
            if let Some(prev_group) = self.undos.last() {
                if prev_group.last() == Some(&item) {
                    self.last_changed_at = Some(now);
                    return;
                }
            }
        }

        self.undos.push(vec![item]);
        self.last_changed_at = Some(now);

        // Enforce max size
        while self.undos.len() > self.max_undos {
            self.undos.remove(0);
        }
    }

    /// Undo the last change group. Returns the items to restore, if any.
    pub fn undo(&mut self) -> Option<Vec<I>> {
        let group = self.undos.pop()?;
        self.redos.push(group.clone());
        self.last_changed_at = None;
        Some(group)
    }

    /// Redo the last undone change group. Returns the items to apply, if any.
    pub fn redo(&mut self) -> Option<Vec<I>> {
        let group = self.redos.pop()?;
        self.undos.push(group.clone());
        self.last_changed_at = None;
        Some(group)
    }

    /// Get the undo stack.
    pub fn undos(&self) -> &[Vec<I>] {
        &self.undos
    }

    /// Get the redo stack.
    pub fn redos(&self) -> &[Vec<I>] {
        &self.redos
    }

    /// Clear all history.
    pub fn clear(&mut self) {
        self.undos.clear();
        self.redos.clear();
        self.last_changed_at = None;
    }

    /// Check if undo is available.
    pub fn can_undo(&self) -> bool {
        !self.undos.is_empty()
    }

    /// Check if redo is available.
    pub fn can_redo(&self) -> bool {
        !self.redos.is_empty()
    }
}

impl<I: HistoryItem> Default for History<I> {
    fn default() -> Self {
        Self::new()
    }
}
