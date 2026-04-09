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
        #[cfg(not(target_arch = "wasm32"))]
        let fs = ct::FontSystem::new();

        #[cfg(target_arch = "wasm32")]
        let fs = {
            let locale = "en-US".to_string();
            let db = ct::fontdb::Database::new();
            let mut fs = ct::FontSystem::new_with_locale_and_db(locale, db);
            fs.db_mut().load_font_data(include_bytes!("../fonts/Inter-Regular.ttf").to_vec());
            fs.db_mut().load_font_data(include_bytes!("../fonts/Inter-Bold.ttf").to_vec());
            fs
        };

        Self {
            inner: RefCell::new(fs),
        }
    }

    /// Borrow the inner cosmic-text FontSystem mutably.
    /// Needed for shaping and buffer operations.
    pub fn borrow_mut(&self) -> std::cell::RefMut<'_, ct::FontSystem> {
        self.inner.borrow_mut()
    }
}
