use mozui_style::{Point, Size};

#[derive(Debug, Clone)]
pub enum PlatformEvent {
    MouseMove {
        position: Point,
        modifiers: Modifiers,
    },
    MouseDown {
        button: MouseButton,
        position: Point,
        modifiers: Modifiers,
    },
    MouseUp {
        button: MouseButton,
        position: Point,
        modifiers: Modifiers,
    },
    ScrollWheel {
        delta: ScrollDelta,
        position: Point,
        modifiers: Modifiers,
    },
    KeyDown {
        key: Key,
        modifiers: Modifiers,
        is_repeat: bool,
    },
    KeyUp {
        key: Key,
        modifiers: Modifiers,
    },
    WindowResize {
        size: Size,
    },
    WindowMove {
        position: Point,
    },
    WindowFocused,
    WindowBlurred,
    WindowCloseRequested,
    ScaleFactorChanged {
        scale: f32,
    },
    RedrawRequested,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScrollDelta {
    Lines(f32, f32),
    Pixels(f32, f32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Character(char),
    Enter,
    Escape,
    Tab,
    Backspace,
    Delete,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Home,
    End,
    PageUp,
    PageDown,
    Space,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorStyle {
    Arrow,
    Hand,
    Text,
    ResizeNS,
    ResizeEW,
    ResizeNESW,
    ResizeNWSE,
    Crosshair,
    NotAllowed,
}
