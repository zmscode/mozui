use cocoa::base::{id, nil};
use cocoa::foundation::NSString as CocoaNSString;
use mozui::Window;
use objc::{class, msg_send, sel, sel_impl};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

/// Configuration for an open file dialog.
pub struct OpenDialogConfig {
    pub title: Option<String>,
    pub message: Option<String>,
    pub allowed_types: Vec<String>,
    pub allows_directories: bool,
    pub allows_multiple: bool,
}

impl Default for OpenDialogConfig {
    fn default() -> Self {
        Self {
            title: None,
            message: None,
            allowed_types: Vec::new(),
            allows_directories: false,
            allows_multiple: false,
        }
    }
}

impl OpenDialogConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    pub fn allowed_types(mut self, types: Vec<String>) -> Self {
        self.allowed_types = types;
        self
    }

    pub fn allows_directories(mut self, allows: bool) -> Self {
        self.allows_directories = allows;
        self
    }

    pub fn allows_multiple(mut self, allows: bool) -> Self {
        self.allows_multiple = allows;
        self
    }
}

/// Configuration for a save file dialog.
pub struct SaveDialogConfig {
    pub title: Option<String>,
    pub message: Option<String>,
    pub allowed_types: Vec<String>,
    pub name_field_label: Option<String>,
    pub default_name: Option<String>,
}

impl Default for SaveDialogConfig {
    fn default() -> Self {
        Self {
            title: None,
            message: None,
            allowed_types: Vec::new(),
            name_field_label: None,
            default_name: None,
        }
    }
}

impl SaveDialogConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    pub fn allowed_types(mut self, types: Vec<String>) -> Self {
        self.allowed_types = types;
        self
    }

    pub fn default_name(mut self, name: impl Into<String>) -> Self {
        self.default_name = Some(name.into());
        self
    }
}

/// Show a native open panel. Returns selected file paths, or empty if cancelled.
pub fn show_open_dialog(window: &Window, config: OpenDialogConfig) -> Vec<String> {
    let ns_view = get_raw_ns_view(window);
    unsafe {
        let _ns_window: id = msg_send![ns_view, window];

        let panel: id = msg_send![class!(NSOpenPanel), openPanel];

        let _: () = msg_send![panel, setCanChooseFiles: !config.allows_directories];
        let _: () = msg_send![panel, setCanChooseDirectories: config.allows_directories];
        let _: () = msg_send![panel, setAllowsMultipleSelection: config.allows_multiple];

        if let Some(title) = &config.title {
            let ns_title = CocoaNSString::alloc(nil).init_str(title);
            let _: () = msg_send![panel, setTitle: ns_title];
        }

        if let Some(message) = &config.message {
            let ns_msg = CocoaNSString::alloc(nil).init_str(message);
            let _: () = msg_send![panel, setMessage: ns_msg];
        }

        if !config.allowed_types.is_empty() {
            let type_ids: Vec<id> = config
                .allowed_types
                .iter()
                .map(|t| {
                    let ns_type = CocoaNSString::alloc(nil).init_str(t);
                    let ut_type: id = msg_send![class!(UTType), typeWithFilenameExtension: ns_type];
                    ut_type
                })
                .collect();
            let types_arr: id = msg_send![
                class!(NSArray),
                arrayWithObjects: type_ids.as_ptr()
                count: type_ids.len()
            ];
            let _: () = msg_send![panel, setAllowedContentTypes: types_arr];
        }

        // Run as sheet
        let response: isize = msg_send![panel, runModal];

        // NSModalResponseOK = 1
        if response == 1 {
            let urls: id = msg_send![panel, URLs];
            let count: usize = msg_send![urls, count];
            let mut paths = Vec::with_capacity(count);
            for i in 0..count {
                let url: id = msg_send![urls, objectAtIndex: i];
                let path: id = msg_send![url, path];
                let utf8: *const i8 = msg_send![path, UTF8String];
                let s = std::ffi::CStr::from_ptr(utf8)
                    .to_str()
                    .unwrap_or("")
                    .to_string();
                paths.push(s);
            }
            paths
        } else {
            Vec::new()
        }
    }
}

/// Show a native save panel. Returns the chosen file path, or `None` if cancelled.
pub fn show_save_dialog(window: &Window, config: SaveDialogConfig) -> Option<String> {
    let ns_view = get_raw_ns_view(window);
    unsafe {
        let _ns_window: id = msg_send![ns_view, window];

        let panel: id = msg_send![class!(NSSavePanel), savePanel];

        if let Some(title) = &config.title {
            let ns_title = CocoaNSString::alloc(nil).init_str(title);
            let _: () = msg_send![panel, setTitle: ns_title];
        }

        if let Some(message) = &config.message {
            let ns_msg = CocoaNSString::alloc(nil).init_str(message);
            let _: () = msg_send![panel, setMessage: ns_msg];
        }

        if let Some(name) = &config.default_name {
            let ns_name = CocoaNSString::alloc(nil).init_str(name);
            let _: () = msg_send![panel, setNameFieldStringValue: ns_name];
        }

        if !config.allowed_types.is_empty() {
            let type_ids: Vec<id> = config
                .allowed_types
                .iter()
                .map(|t| {
                    let ns_type = CocoaNSString::alloc(nil).init_str(t);
                    let ut_type: id = msg_send![class!(UTType), typeWithFilenameExtension: ns_type];
                    ut_type
                })
                .collect();
            let types_arr: id = msg_send![
                class!(NSArray),
                arrayWithObjects: type_ids.as_ptr()
                count: type_ids.len()
            ];
            let _: () = msg_send![panel, setAllowedContentTypes: types_arr];
        }

        let response: isize = msg_send![panel, runModal];

        if response == 1 {
            let url: id = msg_send![panel, URL];
            let path: id = msg_send![url, path];
            let utf8: *const i8 = msg_send![path, UTF8String];
            let s = std::ffi::CStr::from_ptr(utf8)
                .to_str()
                .unwrap_or("")
                .to_string();
            Some(s)
        } else {
            None
        }
    }
}

fn get_raw_ns_view(window: &Window) -> id {
    let handle = HasWindowHandle::window_handle(window).expect("window handle unavailable");
    match handle.as_raw() {
        RawWindowHandle::AppKit(h) => h.ns_view.as_ptr() as id,
        _ => unreachable!("expected AppKit window handle on macOS"),
    }
}
