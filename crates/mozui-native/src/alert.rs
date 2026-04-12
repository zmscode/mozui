use cocoa::base::{id, nil};
use cocoa::foundation::NSString as CocoaNSString;
use mozui::Window;
use objc::{class, msg_send, sel, sel_impl};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

/// Alert style (icon displayed).
pub enum AlertStyle {
    /// Informational (app icon).
    Informational,
    /// Warning (caution icon).
    Warning,
    /// Critical (stop icon).
    Critical,
}

/// A button to display in the alert.
pub struct AlertButton {
    pub title: String,
}

/// Result of showing an alert — which button was pressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertResponse {
    /// First button (rightmost, default). Index 0.
    First,
    /// Second button. Index 1.
    Second,
    /// Third button. Index 2.
    Third,
}

/// Configuration for a native alert dialog.
pub struct AlertConfig {
    /// Main message text.
    pub message: String,
    /// Informative/secondary text.
    pub informative: Option<String>,
    /// Alert style.
    pub style: AlertStyle,
    /// Buttons (first = default/rightmost, added in order).
    pub buttons: Vec<AlertButton>,
}

impl AlertConfig {
    /// Create a simple informational alert with an OK button.
    pub fn info(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            informative: None,
            style: AlertStyle::Informational,
            buttons: vec![AlertButton { title: "OK".into() }],
        }
    }

    /// Create a confirmation alert with OK and Cancel buttons.
    pub fn confirm(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            informative: None,
            style: AlertStyle::Informational,
            buttons: vec![
                AlertButton { title: "OK".into() },
                AlertButton {
                    title: "Cancel".into(),
                },
            ],
        }
    }

    /// Create a destructive confirmation with a warning style.
    pub fn destructive(message: impl Into<String>, action: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            informative: None,
            style: AlertStyle::Critical,
            buttons: vec![
                AlertButton {
                    title: action.into(),
                },
                AlertButton {
                    title: "Cancel".into(),
                },
            ],
        }
    }

    /// Set informative text.
    pub fn with_informative(mut self, text: impl Into<String>) -> Self {
        self.informative = Some(text.into());
        self
    }
}

/// Show a modal alert attached to the window. Returns which button was pressed.
pub fn show_alert(window: &Window, config: AlertConfig) -> AlertResponse {
    let ns_view = get_raw_ns_view(window);
    unsafe {
        let ns_window: id = msg_send![ns_view, window];

        let alert: id = msg_send![class!(NSAlert), alloc];
        let alert: id = msg_send![alert, init];

        let ns_message = CocoaNSString::alloc(nil).init_str(&config.message);
        let _: () = msg_send![alert, setMessageText: ns_message];

        if let Some(info) = &config.informative {
            let ns_info = CocoaNSString::alloc(nil).init_str(info);
            let _: () = msg_send![alert, setInformativeText: ns_info];
        }

        let style: isize = match config.style {
            AlertStyle::Informational => 1,
            AlertStyle::Warning => 0,
            AlertStyle::Critical => 2,
        };
        let _: () = msg_send![alert, setAlertStyle: style];

        for button in &config.buttons {
            let ns_title = CocoaNSString::alloc(nil).init_str(&button.title);
            let _: () = msg_send![alert, addButtonWithTitle: ns_title];
        }

        // Run as sheet on window
        let response: isize = msg_send![alert, runModalForWindow: ns_window];

        // NSAlertFirstButtonReturn = 1000
        match response - 1000 {
            0 => AlertResponse::First,
            1 => AlertResponse::Second,
            _ => AlertResponse::Third,
        }
    }
}

/// Show a standalone modal alert (not attached to a window).
pub fn show_modal_alert(config: AlertConfig) -> AlertResponse {
    unsafe {
        let alert: id = msg_send![class!(NSAlert), alloc];
        let alert: id = msg_send![alert, init];

        let ns_message = CocoaNSString::alloc(nil).init_str(&config.message);
        let _: () = msg_send![alert, setMessageText: ns_message];

        if let Some(info) = &config.informative {
            let ns_info = CocoaNSString::alloc(nil).init_str(info);
            let _: () = msg_send![alert, setInformativeText: ns_info];
        }

        let style: isize = match config.style {
            AlertStyle::Informational => 1,
            AlertStyle::Warning => 0,
            AlertStyle::Critical => 2,
        };
        let _: () = msg_send![alert, setAlertStyle: style];

        for button in &config.buttons {
            let ns_title = CocoaNSString::alloc(nil).init_str(&button.title);
            let _: () = msg_send![alert, addButtonWithTitle: ns_title];
        }

        let response: isize = msg_send![alert, runModal];

        match response - 1000 {
            0 => AlertResponse::First,
            1 => AlertResponse::Second,
            _ => AlertResponse::Third,
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
