use cocoa::base::{id, nil};
use cocoa::foundation::NSString as CocoaNSString;
use mozui::Window;
use objc::{class, msg_send, sel, sel_impl};
use raw_window_handle::HasWindowHandle;

/// Window-level appearance and behavior configuration.
///
/// Provides native macOS window modifiers that map to AppKit's NSWindow
/// properties, inspired by SwiftUI's view modifiers.

/// Set the window's title.
pub fn set_title(window: &Window, title: &str) {
    unsafe {
        let ns_window = get_ns_window(window);
        let ns_title = CocoaNSString::alloc(nil).init_str(title);
        let _: () = msg_send![ns_window, setTitle: ns_title];
    }
}

/// Set the window's subtitle (macOS 11+).
pub fn set_subtitle(window: &Window, subtitle: &str) {
    unsafe {
        let ns_window = get_ns_window(window);
        let ns_subtitle = CocoaNSString::alloc(nil).init_str(subtitle);
        let _: () = msg_send![ns_window, setSubtitle: ns_subtitle];
    }
}

/// Window title visibility options.
pub enum TitleVisibility {
    Visible,
    Hidden,
}

/// Set the window title visibility.
pub fn set_title_visibility(window: &Window, visibility: TitleVisibility) {
    unsafe {
        let ns_window = get_ns_window(window);
        let value: isize = match visibility {
            TitleVisibility::Visible => 0,
            TitleVisibility::Hidden => 1,
        };
        let _: () = msg_send![ns_window, setTitleVisibility: value];
    }
}

/// Window toolbar style options.
pub enum WindowToolbarStyle {
    /// Standard toolbar appearance.
    Automatic,
    /// Expanded toolbar with larger items.
    Expanded,
    /// Compact, unified toolbar/titlebar.
    Unified,
    /// Compact unified toolbar.
    UnifiedCompact,
}

/// Set the window's toolbar style (macOS 11+).
pub fn set_toolbar_style(window: &Window, style: WindowToolbarStyle) {
    unsafe {
        let ns_window = get_ns_window(window);
        let value: isize = match style {
            WindowToolbarStyle::Automatic => 0,
            WindowToolbarStyle::Expanded => 1,
            WindowToolbarStyle::Unified => 2,
            WindowToolbarStyle::UnifiedCompact => 3,
        };
        let _: () = msg_send![ns_window, setToolbarStyle: value];
    }
}

/// Set the window's represented URL (shown in title bar with file icon).
pub fn set_represented_filename(window: &Window, path: &str) {
    unsafe {
        let ns_window = get_ns_window(window);
        let ns_path = CocoaNSString::alloc(nil).init_str(path);
        let _: () = msg_send![ns_window, setRepresentedFilename: ns_path];
    }
}

/// Tab behavior when multiple windows are created.
pub enum WindowTabbingMode {
    Automatic,
    Preferred,
    Disallowed,
}

/// Set window tabbing mode (macOS 10.12+).
pub fn set_tabbing_mode(window: &Window, mode: WindowTabbingMode) {
    unsafe {
        let ns_window = get_ns_window(window);
        let value: isize = match mode {
            WindowTabbingMode::Automatic => 0,
            WindowTabbingMode::Preferred => 1,
            WindowTabbingMode::Disallowed => 2,
        };
        let _: () = msg_send![ns_window, setTabbingMode: value];
    }
}

/// Window collection behavior flags.
pub enum CollectionBehavior {
    /// Can join all spaces.
    CanJoinAllSpaces,
    /// Moves to active space.
    MoveToActiveSpace,
    /// Full-screen primary window.
    FullScreenPrimary,
    /// Full-screen auxiliary window.
    FullScreenAuxiliary,
    /// Allows tiling.
    FullScreenAllowsTiling,
}

/// Set window collection behavior.
pub fn set_collection_behavior(window: &Window, behavior: CollectionBehavior) {
    unsafe {
        let ns_window = get_ns_window(window);
        let value: usize = match behavior {
            CollectionBehavior::CanJoinAllSpaces => 1 << 0,
            CollectionBehavior::MoveToActiveSpace => 1 << 1,
            CollectionBehavior::FullScreenPrimary => 1 << 7,
            CollectionBehavior::FullScreenAuxiliary => 1 << 8,
            CollectionBehavior::FullScreenAllowsTiling => 1 << 11,
        };
        let _: () = msg_send![ns_window, setCollectionBehavior: value];
    }
}

/// Set the window's minimum size.
pub fn set_min_size(window: &Window, width: f64, height: f64) {
    unsafe {
        let ns_window = get_ns_window(window);
        let size = cocoa::foundation::NSSize::new(width, height);
        let _: () = msg_send![ns_window, setMinSize: size];
    }
}

/// Set the window's maximum size.
pub fn set_max_size(window: &Window, width: f64, height: f64) {
    unsafe {
        let ns_window = get_ns_window(window);
        let size = cocoa::foundation::NSSize::new(width, height);
        let _: () = msg_send![ns_window, setMaxSize: size];
    }
}

/// Set window opacity (0.0 = transparent, 1.0 = opaque).
pub fn set_opacity(window: &Window, opacity: f64) {
    unsafe {
        let ns_window = get_ns_window(window);
        let _: () = msg_send![ns_window, setAlphaValue: opacity];
    }
}

/// Set whether the window has a shadow.
pub fn set_has_shadow(window: &Window, has_shadow: bool) {
    unsafe {
        let ns_window = get_ns_window(window);
        let _: () = msg_send![ns_window, setHasShadow: has_shadow];
    }
}

/// Set whether the window is movable by dragging its background.
pub fn set_movable_by_background(window: &Window, movable: bool) {
    unsafe {
        let ns_window = get_ns_window(window);
        let _: () = msg_send![ns_window, setMovableByWindowBackground: movable];
    }
}

/// Enter native full screen mode.
pub fn toggle_full_screen(window: &Window) {
    unsafe {
        let ns_window = get_ns_window(window);
        let _: () = msg_send![ns_window, toggleFullScreen: nil];
    }
}

/// Set the window's appearance (light/dark/auto).
pub enum WindowAppearance {
    /// Inherit from system.
    Inherit,
    /// Light (Aqua) appearance.
    Light,
    /// Dark appearance.
    Dark,
}

/// Set the window's appearance.
pub fn set_appearance(window: &Window, appearance: WindowAppearance) {
    unsafe {
        let ns_window = get_ns_window(window);
        match appearance {
            WindowAppearance::Inherit => {
                let _: () = msg_send![ns_window, setAppearance: nil];
            }
            WindowAppearance::Light => {
                let name = CocoaNSString::alloc(nil).init_str("NSAppearanceNameAqua");
                let app: id = msg_send![class!(NSAppearance), appearanceNamed: name];
                let _: () = msg_send![ns_window, setAppearance: app];
            }
            WindowAppearance::Dark => {
                let name = CocoaNSString::alloc(nil).init_str("NSAppearanceNameDarkAqua");
                let app: id = msg_send![class!(NSAppearance), appearanceNamed: name];
                let _: () = msg_send![ns_window, setAppearance: app];
            }
        }
    }
}

fn get_ns_window(window: &Window) -> id {
    let handle = HasWindowHandle::window_handle(window).expect("window handle unavailable");
    match handle.as_raw() {
        raw_window_handle::RawWindowHandle::AppKit(h) => unsafe {
            let ns_view = h.ns_view.as_ptr() as id;
            msg_send![ns_view, window]
        },
        _ => unreachable!("expected AppKit window handle on macOS"),
    }
}
