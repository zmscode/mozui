use mozui_elements::ScrollOffset;
use mozui_events::WindowId;
use mozui_executor::{Executor, TimerId, TimerManager};
use mozui_platform::WindowOptions;
use mozui_reactive::{SetSignal, Signal, SignalStore};
use mozui_style::Theme;
use mozui_style::animation::{Animated, Lerp, SpringHandle, Transition};
use std::cell::Cell;
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::time::Duration;

use crate::app::RootBuilder;

/// A pending request to open a new window.
pub(crate) struct WindowRequest {
    pub options: WindowOptions,
    pub root_builder: RootBuilder,
}

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
    /// Pending window-open requests (drained by the app loop).
    pub(crate) window_requests: Vec<WindowRequest>,
    /// Pending window-close requests.
    pub(crate) close_requests: Vec<WindowId>,
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
            window_requests: Vec::new(),
            close_requests: Vec::new(),
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

    /// Create or retrieve a persistent scroll state for a scrollable container.
    /// Includes momentum physics — scroll continues with deceleration after input stops.
    /// Call this once per scrollable element, in a stable order (like `use_signal`).
    pub fn use_scroll(&mut self) -> ScrollOffset {
        let flag = self.animations_active.clone();
        let (signal, _) = self.use_signal(ScrollOffset::new().with_animation_flag(flag));
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

    /// Cache a computed value that only recomputes when dependencies change.
    ///
    /// ```rust,ignore
    /// let sorted_items = cx.use_memo(&(items.len(), sort_key), || {
    ///     let mut sorted = items.clone();
    ///     sorted.sort_by_key(|i| i.name.clone());
    ///     sorted
    /// });
    /// ```
    pub fn use_memo<D: Hash, T: 'static>(&mut self, deps: &D, compute: impl FnOnce() -> T) -> &T {
        let deps_hash = hash_deps(deps);
        let idx = self.hook_index;
        self.hook_index += 1;

        if idx >= self.signals.slot_count() {
            // First render — compute and store
            let value = compute();
            let (signal, _) = self
                .signals
                .get_or_create::<(u64, T)>(idx, (deps_hash, value));
            return &self.signals.get(signal).1;
        }

        // Subsequent render — check if deps changed
        let (signal, set_signal) = self.signals.get_existing::<(u64, T)>(idx);
        let existing_hash = self.signals.get(signal).0;
        if existing_hash != deps_hash {
            let new_value = compute();
            self.signals.set(set_signal, (deps_hash, new_value));
        }
        &self.signals.get(signal).1
    }

    /// Run a side-effect when dependencies change. The effect runs synchronously
    /// during the render call when deps differ from the previous render.
    ///
    /// ```rust,ignore
    /// cx.use_effect(&selected_id, || {
    ///     tracing::info!("Selection changed to: {}", selected_id);
    /// });
    /// ```
    pub fn use_effect<D: Hash>(&mut self, deps: &D, effect: impl FnOnce()) {
        let deps_hash = hash_deps(deps);
        let (signal, set_signal) = self
            .signals
            .get_or_create::<u64>(self.hook_index, deps_hash);
        self.hook_index += 1;

        let existing = *self.signals.get(signal);
        if existing != deps_hash {
            self.signals.set(set_signal, deps_hash);
            effect();
        }
    }

    /// Open a new window with the given options and root builder.
    /// The window is created on the next frame.
    pub fn open_window(
        &mut self,
        options: WindowOptions,
        root: impl Fn(&mut Context) -> Box<dyn mozui_elements::Element> + 'static,
    ) {
        self.window_requests.push(WindowRequest {
            options,
            root_builder: Box::new(root),
        });
    }

    /// Close a window by its ID.
    pub fn close_window(&mut self, id: WindowId) {
        self.close_requests.push(id);
    }

    /// Get the shared animation flag. Pass this to `Animated::new()` when creating
    /// animations outside of `use_animated()` (e.g. in dynamically-sized collections).
    pub fn animation_flag(&self) -> Rc<Cell<bool>> {
        self.animations_active.clone()
    }

    /// Returns true if any animations are currently in progress.
    /// The app loop uses this to request continuous redraws.
    pub fn has_active_animations(&self) -> bool {
        self.animations_active.get()
    }

    /// Return a summary of all signal slots for devtools display.
    pub fn signal_summary(&self) -> Vec<(usize, &'static str)> {
        self.signals.signal_summary()
    }

    /// Clear the animation-active flag at the start of each frame.
    pub fn clear_animation_flag(&mut self) {
        self.animations_active.set(false);
    }
}

/// Hash a dependency value to a u64 for memo/effect comparisons.
fn hash_deps<D: Hash>(deps: &D) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    deps.hash(&mut hasher);
    hasher.finish()
}
