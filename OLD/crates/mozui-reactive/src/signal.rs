use std::any::Any;
use std::marker::PhantomData;

/// Callback invoked on each signal mutation: (slot_id, type_name).
pub type MutationCallback = Box<dyn Fn(usize, &'static str)>;

/// Opaque handle for reading a signal value.
#[derive(Debug)]
pub struct Signal<T> {
    id: usize,
    _marker: PhantomData<T>,
}

impl<T> Clone for Signal<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for Signal<T> {}

/// Opaque handle for writing a signal value.
#[derive(Debug)]
pub struct SetSignal<T> {
    id: usize,
    _marker: PhantomData<T>,
}

impl<T> Clone for SetSignal<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for SetSignal<T> {}

/// Central store for all signal values.
pub struct SignalStore {
    slots: Vec<Box<dyn Any>>,
    type_names: Vec<&'static str>,
    dirty: bool,
    mutation_callback: Option<MutationCallback>,
}

impl SignalStore {
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            type_names: Vec::new(),
            dirty: false,
            mutation_callback: None,
        }
    }

    /// Install a callback that fires on every `set()` / `update()`.
    pub fn set_mutation_callback(&mut self, cb: MutationCallback) {
        self.mutation_callback = Some(cb);
    }

    /// Get or create a signal at a given hook index.
    /// On first call with this index, creates with `initial`.
    /// On subsequent calls, returns the existing signal (ignoring `initial`).
    pub fn get_or_create<T: 'static>(
        &mut self,
        index: usize,
        initial: T,
    ) -> (Signal<T>, SetSignal<T>) {
        if index >= self.slots.len() {
            self.slots.push(Box::new(initial));
            self.type_names.push(std::any::type_name::<T>());
        }
        (
            Signal {
                id: index,
                _marker: PhantomData,
            },
            SetSignal {
                id: index,
                _marker: PhantomData,
            },
        )
    }

    /// Read the current value of a signal.
    pub fn get<T: 'static>(&self, signal: Signal<T>) -> &T {
        self.slots[signal.id]
            .downcast_ref::<T>()
            .expect("signal type mismatch")
    }

    /// Set a signal's value and mark the store dirty.
    pub fn set<T: 'static>(&mut self, signal: SetSignal<T>, value: T) {
        if let Some(ref cb) = self.mutation_callback {
            let type_name = self
                .type_names
                .get(signal.id)
                .copied()
                .unwrap_or("unknown");
            cb(signal.id, type_name);
        }
        *self.slots[signal.id]
            .downcast_mut::<T>()
            .expect("signal type mismatch") = value;
        self.dirty = true;
    }

    /// Update a signal's value in place and mark dirty.
    pub fn update<T: 'static>(&mut self, signal: SetSignal<T>, f: impl FnOnce(&mut T)) {
        if let Some(ref cb) = self.mutation_callback {
            let type_name = self
                .type_names
                .get(signal.id)
                .copied()
                .unwrap_or("unknown");
            cb(signal.id, type_name);
        }
        let val = self.slots[signal.id]
            .downcast_mut::<T>()
            .expect("signal type mismatch");
        f(val);
        self.dirty = true;
    }

    /// Check if any signal has been modified since last clear.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Clear the dirty flag.
    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    /// Number of allocated slots.
    pub fn slot_count(&self) -> usize {
        self.slots.len()
    }

    /// Get the type name for a given slot.
    pub fn type_name(&self, slot_id: usize) -> &'static str {
        self.type_names
            .get(slot_id)
            .copied()
            .unwrap_or("unknown")
    }

    /// Return a summary of all signal slots: (slot_id, type_name).
    pub fn signal_summary(&self) -> Vec<(usize, &'static str)> {
        self.type_names
            .iter()
            .enumerate()
            .map(|(i, tn)| (i, *tn))
            .collect()
    }

    /// Get signal handles for an existing slot (panics if slot doesn't exist).
    pub fn get_existing<T: 'static>(&self, index: usize) -> (Signal<T>, SetSignal<T>) {
        assert!(index < self.slots.len(), "slot does not exist");
        (
            Signal {
                id: index,
                _marker: PhantomData,
            },
            SetSignal {
                id: index,
                _marker: PhantomData,
            },
        )
    }
}
