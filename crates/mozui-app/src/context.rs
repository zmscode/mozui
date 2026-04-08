use mozui_elements::ScrollOffset;
use mozui_executor::{Executor, TimerId, TimerManager};
use mozui_reactive::{SetSignal, Signal, SignalStore};
use mozui_style::Theme;
use mozui_style::animation::{Animated, Lerp, SpringHandle, Transition};
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

pub struct Context {
    theme: Theme,
    signals: SignalStore,
    hook_index: usize,
    clipboard_read_fn: Option<Box<dyn Fn() -> Option<String>>>,
    clipboard_write_fn: Option<Box<dyn Fn(&str)>>,
    pub(crate) executor: Executor,
    pub(crate) timers: TimerManager,
    /// Shared flag set by animations when they're still in progress.
    animations_active: Rc<Cell<bool>>,
}

impl Context {
    pub fn new(theme: Theme) -> Self {
        Self {
            theme,
            signals: SignalStore::new(),
            hook_index: 0,
            clipboard_read_fn: None,
            clipboard_write_fn: None,
            executor: Executor::new(),
            timers: TimerManager::new(),
            animations_active: Rc::new(Cell::new(false)),
        }
    }

    /// Set clipboard functions (called by App during setup).
    pub fn set_clipboard(
        &mut self,
        read: impl Fn() -> Option<String> + 'static,
        write: impl Fn(&str) + 'static,
    ) {
        self.clipboard_read_fn = Some(Box::new(read));
        self.clipboard_write_fn = Some(Box::new(write));
    }

    /// Read text from the system clipboard.
    pub fn clipboard_read(&self) -> Option<String> {
        self.clipboard_read_fn.as_ref().and_then(|f| f())
    }

    /// Write text to the system clipboard.
    pub fn clipboard_write(&self, text: &str) {
        if let Some(ref f) = self.clipboard_write_fn {
            f(text);
        }
    }

    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    /// Reset the hook index before re-rendering.
    pub fn reset_hooks(&mut self) {
        self.hook_index = 0;
    }

    /// Create or retrieve a signal. On first render, creates a new signal.
    /// On subsequent renders, returns the existing signal at this hook position.
    pub fn use_signal<T: 'static>(&mut self, initial: T) -> (Signal<T>, SetSignal<T>) {
        let idx = self.hook_index;
        self.hook_index += 1;
        self.signals.get_or_create(idx, initial)
    }

    /// Read the current value of a signal.
    pub fn get<T: 'static>(&self, signal: Signal<T>) -> &T {
        self.signals.get(signal)
    }

    /// Set a signal's value (triggers re-render).
    pub fn set<T: 'static>(&mut self, signal: SetSignal<T>, value: T) {
        self.signals.set(signal, value);
    }

    /// Update a signal's value in place (triggers re-render).
    pub fn update<T: 'static>(&mut self, signal: SetSignal<T>, f: impl FnOnce(&mut T)) {
        self.signals.update(signal, f);
    }

    /// Check if any signal has changed.
    pub fn is_dirty(&self) -> bool {
        self.signals.is_dirty()
    }

    /// Clear dirty flag after re-render.
    pub fn clear_dirty(&mut self) {
        self.signals.clear_dirty();
    }

    /// Spawn an async task on the main thread.
    pub fn spawn(&mut self, future: impl std::future::Future<Output = ()> + 'static) {
        self.executor.spawn(future);
    }

    /// Schedule a one-shot callback after a duration.
    pub fn set_timeout(
        &mut self,
        duration: Duration,
        callback: impl FnOnce(&mut dyn std::any::Any) + 'static,
    ) -> TimerId {
        self.timers.set_timeout(duration, callback)
    }

    /// Schedule a repeating callback at an interval.
    pub fn set_interval(
        &mut self,
        interval: Duration,
        callback: impl Fn(&mut dyn std::any::Any) + 'static,
    ) -> TimerId {
        self.timers.set_interval(interval, callback)
    }

    /// Cancel a timer.
    pub fn cancel_timer(&mut self, id: TimerId) {
        self.timers.cancel(id);
    }

    /// Create or retrieve a persistent scroll offset for a scrollable container.
    /// Call this once per scrollable element, in a stable order (like `use_signal`).
    pub fn use_scroll(&mut self) -> ScrollOffset {
        let (signal, _) = self.use_signal(ScrollOffset::new());
        self.get(signal).clone()
    }

    /// Create or retrieve a persistent animated value.
    ///
    /// The value smoothly transitions when you call `.set()` on the handle.
    /// ```rust,ignore
    /// let opacity = cx.use_animated(1.0f32, Transition::new(theme.transition_normal));
    /// let current = opacity.get(); // smoothly interpolated
    /// opacity.set(0.0); // start fading out
    /// ```
    pub fn use_animated<T: Lerp + 'static>(
        &mut self,
        initial: T,
        transition: Transition,
    ) -> Animated<T> {
        let flag = self.animations_active.clone();
        let (signal, _) = self.use_signal(Animated::new(initial, transition, flag));
        self.get(signal).clone()
    }

    /// Create or retrieve a persistent spring animation.
    ///
    /// Springs provide physically-based motion with configurable stiffness/damping.
    /// ```rust,ignore
    /// let scale = cx.use_spring(1.0);
    /// let current = scale.get(); // current spring value
    /// scale.set(1.2); // spring toward new target
    /// ```
    pub fn use_spring(&mut self, initial: f32) -> SpringHandle {
        let flag = self.animations_active.clone();
        let (signal, _) = self.use_signal(SpringHandle::new(initial, flag));
        self.get(signal).clone()
    }

    /// Returns true if any animations are currently in progress.
    /// The app loop uses this to request continuous redraws.
    pub fn has_active_animations(&self) -> bool {
        self.animations_active.get()
    }

    /// Clear the animation-active flag at the start of each frame.
    pub fn clear_animation_flag(&mut self) {
        self.animations_active.set(false);
    }
}
