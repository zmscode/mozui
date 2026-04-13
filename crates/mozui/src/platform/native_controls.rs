//! Platform-native control contracts shared by mozui backends.

use crate::{Bounds, Pixels};
use std::ffi::c_void;

/// Opaque state stored by native-backed elements across frames.
pub struct NativeControlState {
    view: *mut c_void,
    target: *mut c_void,
    cleanup: unsafe fn(*mut c_void, *mut c_void),
}

impl Default for NativeControlState {
    fn default() -> Self {
        Self {
            view: std::ptr::null_mut(),
            target: std::ptr::null_mut(),
            cleanup: noop_cleanup,
        }
    }
}

impl Drop for NativeControlState {
    fn drop(&mut self) {
        unsafe { (self.cleanup)(self.view, self.target) }
    }
}

unsafe impl Send for NativeControlState {}

unsafe fn noop_cleanup(_view: *mut c_void, _target: *mut c_void) {}

impl NativeControlState {
    /// Create a new native control state from raw platform handles.
    pub fn new(
        view: *mut c_void,
        target: *mut c_void,
        cleanup: unsafe fn(*mut c_void, *mut c_void),
    ) -> Self {
        Self {
            view,
            target,
            cleanup,
        }
    }

    /// Returns the native view pointer for this control.
    pub fn view(&self) -> *mut c_void {
        self.view
    }

    /// Returns true if a platform view has been created.
    pub fn is_initialized(&self) -> bool {
        !self.view.is_null()
    }

    /// Replace the target pointer for this control.
    pub fn set_target(&mut self, target: *mut c_void) {
        self.target = target;
    }

    /// Returns the target pointer for this control.
    pub fn target(&self) -> *mut c_void {
        self.target
    }
}

/// Native button styling shared across backends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonStyle {
    /// Standard rounded push button.
    Rounded,
    /// Filled / accented button.
    Filled,
    /// Inline toolbar-style button.
    Inline,
    /// Borderless icon button.
    Borderless,
}

/// Native progress indicator styling shared across backends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProgressStyle {
    /// Horizontal progress bar.
    #[default]
    Bar,
    /// Spinner / indeterminate activity indicator.
    Spinning,
}

/// Native text field styling shared across backends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextFieldStyle {
    /// Standard editable text field.
    #[default]
    Plain,
    /// Native search field appearance.
    Search,
}

/// Per-frame button configuration.
pub struct ButtonConfig<'a> {
    /// Title rendered by the native control.
    pub title: &'a str,
    /// Whether the button accepts interaction.
    pub enabled: bool,
    /// Platform style variant to apply.
    pub style: ButtonStyle,
    /// Callback invoked when the button fires its action.
    pub on_click: Option<Box<dyn Fn()>>,
}

/// Per-frame switch configuration.
pub struct SwitchConfig {
    /// Whether the switch is on.
    pub checked: bool,
    /// Whether the switch accepts interaction.
    pub enabled: bool,
    /// Callback invoked after the switch toggles.
    pub on_change: Option<Box<dyn Fn(bool)>>,
}

/// Per-frame slider configuration.
pub struct SliderConfig {
    /// Minimum slider value.
    pub min: f64,
    /// Maximum slider value.
    pub max: f64,
    /// Current slider value.
    pub value: f64,
    /// Whether the slider accepts interaction.
    pub enabled: bool,
    /// Callback invoked after the slider value changes.
    pub on_change: Option<Box<dyn Fn(f64)>>,
}

/// Per-frame progress configuration.
pub struct ProgressConfig {
    /// Current value for determinate progress, or `None` for indeterminate mode.
    pub value: Option<f64>,
    /// Minimum progress value.
    pub min: f64,
    /// Maximum progress value.
    pub max: f64,
    /// Platform style variant to apply.
    pub style: ProgressStyle,
}

/// Per-frame text field configuration.
pub struct TextFieldConfig<'a> {
    /// Current text value.
    pub value: &'a str,
    /// Placeholder text shown while empty.
    pub placeholder: Option<&'a str>,
    /// Whether the field accepts interaction.
    pub enabled: bool,
    /// Whether the field can be edited.
    pub editable: bool,
    /// Whether the field allows text selection.
    pub selectable: bool,
    /// Whether the field uses a bezel/border.
    pub bezeled: bool,
    /// Optional system font size override.
    pub font_size: Option<f64>,
    /// Whether to render a secure/password field.
    pub secure: bool,
    /// Platform style variant to apply.
    pub style: TextFieldStyle,
    /// Callback invoked as the text changes.
    pub on_change: Option<Box<dyn Fn(String)>>,
    /// Callback invoked when the field submits/commits.
    pub on_submit: Option<Box<dyn Fn(String)>>,
}

/// Platform-native controls interface implemented by backend window systems.
pub trait PlatformNativeControls {
    /// Update or create a native button.
    fn update_button(
        &self,
        _state: &mut NativeControlState,
        _parent: *mut c_void,
        _bounds: Bounds<Pixels>,
        _scale: f32,
        _config: ButtonConfig<'_>,
    ) {
    }

    /// Update or create a native switch.
    fn update_switch(
        &self,
        _state: &mut NativeControlState,
        _parent: *mut c_void,
        _bounds: Bounds<Pixels>,
        _scale: f32,
        _config: SwitchConfig,
    ) {
    }

    /// Update or create a native slider.
    fn update_slider(
        &self,
        _state: &mut NativeControlState,
        _parent: *mut c_void,
        _bounds: Bounds<Pixels>,
        _scale: f32,
        _config: SliderConfig,
    ) {
    }

    /// Update or create a native progress indicator.
    fn update_progress(
        &self,
        _state: &mut NativeControlState,
        _parent: *mut c_void,
        _bounds: Bounds<Pixels>,
        _scale: f32,
        _config: ProgressConfig,
    ) {
    }

    /// Update or create a native text field.
    fn update_text_field(
        &self,
        _state: &mut NativeControlState,
        _parent: *mut c_void,
        _bounds: Bounds<Pixels>,
        _scale: f32,
        _config: TextFieldConfig<'_>,
    ) {
    }
}
