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

/// Visual effect blur material. Maps to NSVisualEffectMaterial on macOS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VisualEffectMaterial {
    /// Title bar appearance.
    Titlebar,
    /// Selection highlight appearance.
    #[default]
    Selection,
    /// Menu appearance.
    Menu,
    /// Popover appearance.
    Popover,
    /// Sidebar appearance.
    Sidebar,
    /// Header view appearance.
    HeaderView,
    /// Sheet appearance.
    Sheet,
    /// Window background appearance.
    WindowBackground,
    /// HUD window appearance.
    HudWindow,
    /// Full-screen UI appearance.
    FullScreenUI,
    /// Tool tip appearance.
    ToolTip,
    /// Content background appearance.
    ContentBackground,
    /// Under-window background appearance.
    UnderWindowBackground,
    /// Under-page background appearance.
    UnderPageBackground,
}

/// Visual effect blending mode. Maps to NSVisualEffectBlendingMode on macOS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VisualEffectBlending {
    /// Blends with content behind the window.
    #[default]
    BehindWindow,
    /// Blends with content within the window.
    WithinWindow,
}

/// Visual effect active state. Maps to NSVisualEffectState on macOS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VisualEffectActiveState {
    /// Mirrors the window's active/inactive state.
    FollowsWindowActiveState,
    /// Always renders as active.
    #[default]
    Active,
    /// Always renders as inactive.
    Inactive,
}

/// Per-frame visual effect configuration.
pub struct VisualEffectConfig {
    /// Blur material to apply.
    pub material: VisualEffectMaterial,
    /// Blending mode for the blur.
    pub blending: VisualEffectBlending,
    /// Active state for the blur.
    pub active_state: VisualEffectActiveState,
    /// Whether the view renders in an emphasized (selected) state.
    pub is_emphasized: bool,
}

/// Glass effect style (macOS 26+). Falls back to NSVisualEffectView on older OS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GlassEffectStyle {
    /// Standard glass appearance.
    #[default]
    Regular,
    /// Clear / minimal glass appearance.
    Clear,
}

/// Per-frame glass effect configuration.
pub struct GlassEffectConfig {
    /// Glass style variant.
    pub style: GlassEffectStyle,
    /// Optional corner radius override.
    pub corner_radius: Option<f64>,
    /// RGBA tint color components, each 0.0–1.0.
    pub tint_color: Option<(f64, f64, f64, f64)>,
}

/// SF Symbol weight. Maps to NSFontWeight values on Apple platforms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SymbolWeight {
    /// Ultra-light weight.
    UltraLight,
    /// Thin weight.
    Thin,
    /// Light weight.
    Light,
    /// Regular weight (default).
    #[default]
    Regular,
    /// Medium weight.
    Medium,
    /// Semibold weight.
    Semibold,
    /// Bold weight.
    Bold,
    /// Heavy weight.
    Heavy,
    /// Black weight.
    Black,
}

impl SymbolWeight {
    /// Convert to the corresponding `NSFontWeight` float value.
    pub fn to_ns_weight(self) -> f64 {
        match self {
            Self::UltraLight => -0.8,
            Self::Thin => -0.6,
            Self::Light => -0.4,
            Self::Regular => 0.0,
            Self::Medium => 0.23,
            Self::Semibold => 0.3,
            Self::Bold => 0.4,
            Self::Heavy => 0.56,
            Self::Black => 0.62,
        }
    }
}

/// SF Symbol scale. Maps to NSImageSymbolScale on Apple platforms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SymbolScale {
    /// Small symbol scale.
    Small,
    /// Medium symbol scale (default).
    #[default]
    Medium,
    /// Large symbol scale.
    Large,
}

impl SymbolScale {
    /// Convert to the corresponding `NSImageSymbolScale` integer value.
    pub fn to_ns_scale(self) -> isize {
        match self {
            Self::Small => 1,
            Self::Medium => 2,
            Self::Large => 3,
        }
    }
}

/// Per-frame image / SF Symbol view configuration.
pub struct ImageViewConfig<'a> {
    /// SF Symbol name (e.g. "folder.fill").
    pub symbol_name: &'a str,
    /// Symbol rendering weight.
    pub weight: SymbolWeight,
    /// Symbol rendering scale.
    pub scale: SymbolScale,
    /// Point size; 0.0 uses system default.
    pub point_size: f64,
    /// Optional RGBA tint color.
    pub tint_color: Option<(f64, f64, f64, f64)>,
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

    /// Update or create a native visual effect (blur/vibrancy) view.
    fn update_visual_effect(
        &self,
        _state: &mut NativeControlState,
        _parent: *mut c_void,
        _bounds: Bounds<Pixels>,
        _scale: f32,
        _config: VisualEffectConfig,
    ) {
    }

    /// Update or create a native glass effect view (macOS 26+, NSVisualEffectView fallback).
    fn update_glass_effect(
        &self,
        _state: &mut NativeControlState,
        _parent: *mut c_void,
        _bounds: Bounds<Pixels>,
        _scale: f32,
        _config: GlassEffectConfig,
    ) {
    }

    /// Update or create a native SF Symbol image view.
    fn update_image_view(
        &self,
        _state: &mut NativeControlState,
        _parent: *mut c_void,
        _bounds: Bounds<Pixels>,
        _scale: f32,
        _config: ImageViewConfig<'_>,
    ) {
    }
}
