use std::path::Path;
use std::sync::Arc;

use cosmic_text::{FontSystem, fontdb::Database};
use objc2_core_text::{CTFont, CTFontUIFontType};

use crate::PlatformTextSystem;

const IOS_FONT_DIRS: &[&str] = &[
    "/System/Library/Fonts",
    "/System/Library/Fonts/Core",
    "/System/Library/Fonts/CoreUI",
    "/System/Library/Fonts/LanguageSupport",
];

pub(crate) fn make_text_system() -> Arc<dyn PlatformTextSystem> {
    let system_font_fallback = resolve_system_font_family();
    let mut db = Database::new();
    for dir in IOS_FONT_DIRS {
        let path = Path::new(dir);
        if path.exists() {
            db.load_fonts_dir(path);
        }
    }

    let font_system = FontSystem::new_with_locale_and_db("en-US".to_string(), db);
    let text_system = Arc::new(
        crate::platform::wgpu::CosmicTextSystem::new_with_font_system(
            &system_font_fallback,
            font_system,
        ),
    );

    log::info!("iOS text system using system UI font family: {system_font_fallback}");
    text_system
}

fn resolve_system_font_family() -> String {
    unsafe {
        CTFont::new_ui_font_for_language(CTFontUIFontType::System, 0.0, None)
            .map(|font| font.family_name().to_string())
            .unwrap_or_else(|| {
                log::warn!(
                    "failed to resolve iOS system UI font via CoreText; falling back to Helvetica"
                );
                "Helvetica".to_string()
            })
    }
}
