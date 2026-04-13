use cocoa::base::{id, nil};
use mozui::{NativeSheet, NativeSheetHandle, Window, px, size};
use objc::{msg_send, sel, sel_impl};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_SHEET_IDENTIFIER: AtomicUsize = AtomicUsize::new(1);

thread_local! {
    static SHEET_HANDLES: RefCell<HashMap<usize, NativeSheetHandle>> = RefCell::new(HashMap::new());
}

/// Configuration for a sheet window.
pub struct SheetConfig {
    pub width: f64,
    pub height: f64,
}

impl Default for SheetConfig {
    fn default() -> Self {
        Self {
            width: 400.0,
            height: 300.0,
        }
    }
}

/// Show a modal sheet attached to the window.
///
/// `content_view` is an `NSView` to display in the sheet.
/// Pass `nil` for an empty sheet.
/// Returns the sheet `NSWindow`.
pub fn show_sheet(window: &Window, content_view: id, config: SheetConfig) -> id {
    let identifier = format!(
        "mozui-native.sheet.{}",
        NEXT_SHEET_IDENTIFIER.fetch_add(1, Ordering::Relaxed)
    );

    let Some(handle) = window.show_native_sheet(
        NativeSheet::new(size(px(config.width as f32), px(config.height as f32)))
            .host_identifier(identifier.clone()),
    ) else {
        return nil;
    };

    if content_view != nil {
        let _ = window.set_native_host_content(&identifier, content_view.cast());
    }

    let Some(host_view) = window
        .raw_native_host_view_ptr(&identifier)
        .map(|view| view as id)
    else {
        return nil;
    };
    let sheet_window: id = unsafe { msg_send![host_view, window] };
    if sheet_window != nil {
        SHEET_HANDLES.with(|handles| {
            handles.borrow_mut().insert(sheet_window as usize, handle);
        });
    }
    sheet_window
}

/// End a sheet that was previously shown.
pub fn end_sheet(window: &Window, sheet_window: id) {
    if sheet_window == nil {
        return;
    }

    let handle =
        SHEET_HANDLES.with(|handles| handles.borrow_mut().remove(&(sheet_window as usize)));
    if let Some(handle) = handle {
        let _ = window.close_native_sheet(handle);
        return;
    }

    unsafe {
        let _: () = msg_send![sheet_window, close];
    }
}
