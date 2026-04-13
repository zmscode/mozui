#[cfg(any(target_os = "macos", target_os = "ios"))]
mod native_view;

// Window-level APIs (toolbar, sidebar, breadcrumb, view, menus)
#[cfg(target_os = "macos")]
mod alert;
#[cfg(target_os = "macos")]
mod breadcrumb;
#[cfg(target_os = "macos")]
mod menu;
#[cfg(target_os = "macos")]
mod popover;
#[cfg(target_os = "macos")]
mod search;
#[cfg(target_os = "macos")]
mod sidebar;
#[cfg(target_os = "macos")]
mod toolbar;
#[cfg(target_os = "macos")]
mod view;

// Element-based components (NSView subviews positioned by layout engine)
#[cfg(target_os = "macos")]
mod button;
#[cfg(target_os = "macos")]
mod color_picker;
#[cfg(target_os = "macos")]
mod glass_effect;
#[cfg(target_os = "macos")]
mod picker;
#[cfg(target_os = "macos")]
mod progress;
#[cfg(target_os = "macos")]
mod slider;
#[cfg(any(target_os = "macos", target_os = "ios"))]
mod switch;
#[cfg(target_os = "macos")]
mod symbol;
#[cfg(target_os = "macos")]
mod table;
#[cfg(target_os = "macos")]
mod text_field;
#[cfg(target_os = "macos")]
mod visual_effect;

// System integration
#[cfg(target_os = "macos")]
mod date_picker;
#[cfg(target_os = "macos")]
mod file_dialog;
#[cfg(target_os = "macos")]
mod inspector;
#[cfg(target_os = "macos")]
mod stepper;
#[cfg(target_os = "macos")]
mod tab_view;

// Advanced patterns
#[cfg(target_os = "macos")]
mod drag_drop;
#[cfg(target_os = "macos")]
mod share;
#[cfg(target_os = "macos")]
mod sheet;

// Re-exports
#[cfg(any(target_os = "macos", target_os = "ios"))]
pub use native_view::NativeViewState;

#[cfg(target_os = "macos")]
pub use alert::*;
#[cfg(target_os = "macos")]
pub use breadcrumb::*;
#[cfg(target_os = "macos")]
pub use menu::*;
#[cfg(target_os = "macos")]
pub use popover::*;
#[cfg(target_os = "macos")]
pub use search::*;
#[cfg(target_os = "macos")]
pub use sidebar::*;
#[cfg(target_os = "macos")]
pub use toolbar::*;
#[cfg(target_os = "macos")]
pub use view::*;

#[cfg(target_os = "macos")]
pub use button::*;
#[cfg(target_os = "macos")]
pub use color_picker::*;
#[cfg(target_os = "macos")]
pub use glass_effect::*;
#[cfg(target_os = "macos")]
pub use picker::*;
#[cfg(target_os = "macos")]
pub use progress::*;
#[cfg(target_os = "macos")]
pub use slider::*;
#[cfg(any(target_os = "macos", target_os = "ios"))]
pub use switch::*;
#[cfg(target_os = "macos")]
pub use symbol::*;
#[cfg(target_os = "macos")]
pub use table::*;
#[cfg(target_os = "macos")]
pub use text_field::*;
#[cfg(target_os = "macos")]
pub use visual_effect::*;

#[cfg(target_os = "macos")]
pub use date_picker::*;
#[cfg(target_os = "macos")]
pub use file_dialog::*;
#[cfg(target_os = "macos")]
pub use inspector::*;
#[cfg(target_os = "macos")]
pub use stepper::*;
#[cfg(target_os = "macos")]
pub use tab_view::*;

#[cfg(target_os = "macos")]
pub use drag_drop::*;
#[cfg(target_os = "macos")]
pub use share::*;
#[cfg(target_os = "macos")]
pub use sheet::*;
