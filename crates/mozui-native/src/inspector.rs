use cocoa::base::{id, nil};
use mozui::{NativeInspectorHost, Window, px};
use objc::{msg_send, sel, sel_impl};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::cell::RefCell;
use std::collections::HashMap;

const INSPECTOR_IDENTIFIER: &str = "mozui-native.inspector";

thread_local! {
    static INSPECTOR_VISIBILITY: RefCell<HashMap<usize, bool>> = RefCell::new(HashMap::new());
}

/// Configuration for an inspector panel.
pub struct InspectorConfig {
    /// Width of the inspector panel.
    pub width: f64,
    /// Whether the inspector is initially visible.
    pub is_visible: bool,
}

impl Default for InspectorConfig {
    fn default() -> Self {
        Self {
            width: 260.0,
            is_visible: false,
        }
    }
}

/// Install an inspector panel host using `mozui` core native-window APIs.
///
/// The returned `id` is the hosted inspector view container. Existing AppKit
/// integration code may treat it as the content view to populate.
pub fn install_inspector(window: &Window, config: InspectorConfig) -> id {
    if !window.install_native_inspector_host(
        NativeInspectorHost::new(INSPECTOR_IDENTIFIER, px(config.width as f32))
            .min_width(px(config.width as f32))
            .max_width(px((config.width * 1.5) as f32))
            .visible(config.is_visible),
    ) {
        return nil;
    }

    INSPECTOR_VISIBILITY.with(|state| {
        state
            .borrow_mut()
            .insert(window_key(window), config.is_visible);
    });

    window
        .raw_native_host_view_ptr(INSPECTOR_IDENTIFIER)
        .map(|view| view as id)
        .unwrap_or(nil)
}

/// Toggle the inspector panel visibility.
pub fn toggle_inspector(inspector_item: id) {
    let Some((window_key, visible)) = inspector_state(inspector_item) else {
        return;
    };

    let next_visible = !visible;
    let ns_window = unsafe {
        let window: id = msg_send![inspector_item, window];
        window
    };
    if ns_window == nil {
        return;
    }

    if set_visibility_for_window(ns_window, next_visible) {
        INSPECTOR_VISIBILITY.with(|state| {
            state.borrow_mut().insert(window_key, next_visible);
        });
    }
}

/// Set inspector visibility.
pub fn set_inspector_visible(inspector_item: id, visible: bool) {
    let Some((window_key, _)) = inspector_state(inspector_item) else {
        return;
    };

    let ns_window = unsafe {
        let window: id = msg_send![inspector_item, window];
        window
    };
    if ns_window == nil {
        return;
    }

    if set_visibility_for_window(ns_window, visible) {
        INSPECTOR_VISIBILITY.with(|state| {
            state.borrow_mut().insert(window_key, visible);
        });
    }
}

fn inspector_state(inspector_item: id) -> Option<(usize, bool)> {
    if inspector_item == nil {
        return None;
    }

    let ns_window = unsafe {
        let window: id = msg_send![inspector_item, window];
        window
    };
    if ns_window == nil {
        return None;
    }

    let key = ns_window as usize;
    let visible =
        INSPECTOR_VISIBILITY.with(|state| state.borrow().get(&key).copied().unwrap_or(false));
    Some((key, visible))
}

fn set_visibility_for_window(ns_window: id, visible: bool) -> bool {
    unsafe {
        let split_vc: id = msg_send![ns_window, contentViewController];
        if split_vc == nil {
            return false;
        }
        let split_items: id = msg_send![split_vc, splitViewItems];
        let count: usize = msg_send![split_items, count];
        if count == 0 {
            return false;
        }
        let item: id = msg_send![split_items, objectAtIndex: count - 1];
        let _: () = msg_send![item, setCollapsed: !visible];
    }
    true
}

fn window_key(window: &Window) -> usize {
    unsafe {
        let host_view = get_raw_ns_view(window);
        if host_view == nil {
            return 0;
        }
        let ns_window: id = msg_send![host_view, window];
        ns_window as usize
    }
}

fn get_raw_ns_view(window: &Window) -> id {
    let handle = HasWindowHandle::window_handle(window).expect("window handle unavailable");
    match handle.as_raw() {
        RawWindowHandle::AppKit(h) => h.ns_view.as_ptr() as id,
        _ => unreachable!("expected AppKit window handle on macOS"),
    }
}
