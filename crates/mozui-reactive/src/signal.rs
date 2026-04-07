use std::any::Any;
use std::marker::PhantomData;

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
    dirty: bool,
}

impl SignalStore {
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            dirty: false,
        }
    }

    /// Get or create a signal at a given hook index.
    /// On first call with this index, creates with `initial`.
    /// On subsequent calls, returns the existing signal (ignoring `initial`).
    pub fn get_or_create<T: 'static>(&mut self, index: usize, initial: T) -> (Signal<T>, SetSignal<T>) {
        if index >= self.slots.len() {
            self.slots.push(Box::new(initial));
        }
        (
            Signal { id: index, _marker: PhantomData },
            SetSignal { id: index, _marker: PhantomData },
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
        *self.slots[signal.id]
            .downcast_mut::<T>()
            .expect("signal type mismatch") = value;
        self.dirty = true;
    }

    /// Update a signal's value in place and mark dirty.
    pub fn update<T: 'static>(&mut self, signal: SetSignal<T>, f: impl FnOnce(&mut T)) {
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
}
