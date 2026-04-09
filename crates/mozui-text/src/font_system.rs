use cosmic_text as ct;
use std::cell::RefCell;

/// Wraps cosmic-text's FontSystem with interior mutability.
/// cosmic-text requires `&mut` for shaping/buffer operations, but mozui's
/// Element trait passes `&FontSystem`. RefCell bridges this cleanly.
pub struct FontSystem {
    inner: RefCell<ct::FontSystem>,
}

impl FontSystem {
    pub fn new() -> Self {
        Self {
            inner: RefCell::new(ct::FontSystem::new()),
        }
    }

    /// Borrow the inner cosmic-text FontSystem mutably.
    /// Needed for shaping and buffer operations.
    pub fn borrow_mut(&self) -> std::cell::RefMut<'_, ct::FontSystem> {
        self.inner.borrow_mut()
    }
}
