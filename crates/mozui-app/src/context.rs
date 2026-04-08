use mozui_reactive::{Signal, SetSignal, SignalStore};
use mozui_style::Theme;

pub struct Context {
    theme: Theme,
    signals: SignalStore,
    hook_index: usize,
    clipboard_read_fn: Option<Box<dyn Fn() -> Option<String>>>,
    clipboard_write_fn: Option<Box<dyn Fn(&str)>>,
}

impl Context {
    pub fn new(theme: Theme) -> Self {
        Self {
            theme,
            signals: SignalStore::new(),
            hook_index: 0,
            clipboard_read_fn: None,
            clipboard_write_fn: None,
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
}
