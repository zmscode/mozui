use crate::{Element, InteractionMap};
use mozui_events;
use mozui_layout::LayoutEngine;
use mozui_renderer::{Border, DrawCommand, DrawList};
use mozui_style::{Color, Corners, Fill, Rect};
use mozui_text::FontSystem;
use taffy::prelude::*;

/// Persistent state for a text input, stored in a signal.
#[derive(Clone, Debug)]
pub struct TextInputState {
    pub value: String,
    pub cursor: usize,
    pub focused: bool,
}

impl TextInputState {
    pub fn new() -> Self {
        Self {
            value: String::new(),
            cursor: 0,
            focused: false,
        }
    }

    pub fn with_value(value: impl Into<String>) -> Self {
        let v: String = value.into();
        let len = v.chars().count();
        Self {
            value: v,
            cursor: len,
            focused: false,
        }
    }
}

type StateUpdater = Box<dyn Fn(Box<dyn FnOnce(&mut TextInputState)>, &mut dyn std::any::Any)>;

pub struct TextInput {
    // Current snapshot (read from signal by user)
    current: TextInputState,

    // Callback to push state changes back to the signal
    on_change: Option<StateUpdater>,

    // Visual
    background: Fill,
    corner_radii: Corners,
    border_width: f32,
    border_color: Color,
    focus_border_color: Color,
    text_color: Color,
    placeholder_color: Color,
    font_size: f32,
    placeholder: String,

    // Layout
    taffy_style: Style,
}

pub fn text_input(current: TextInputState) -> TextInput {
    TextInput {
        current,
        on_change: None,
        background: Fill::Solid(Color::hex("#313244")),
        corner_radii: Corners::uniform(6.0),
        border_width: 1.0,
        border_color: Color::hex("#45475a"),
        focus_border_color: Color::hex("#89b4fa"),
        text_color: Color::hex("#cdd6f4"),
        placeholder_color: Color::hex("#6c7086"),
        font_size: 14.0,
        placeholder: String::new(),
        taffy_style: Style {
            size: Size {
                width: length(240.0),
                height: length(36.0),
            },
            padding: taffy::prelude::Rect {
                left: length(10.0),
                right: length(10.0),
                top: length(0.0),
                bottom: length(0.0),
            },
            ..Default::default()
        },
    }
}

impl TextInput {
    /// Set the callback for state changes. This is how the input pushes updates back to signals.
    /// The callback receives a mutation function and the context (as &mut dyn Any).
    pub fn on_change(
        mut self,
        handler: impl Fn(Box<dyn FnOnce(&mut TextInputState)>, &mut dyn std::any::Any) + 'static,
    ) -> Self {
        self.on_change = Some(Box::new(handler));
        self
    }

    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = text.into();
        self
    }

    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    pub fn w(mut self, width: f32) -> Self {
        self.taffy_style.size.width = length(width);
        self
    }

    pub fn h(mut self, height: f32) -> Self {
        self.taffy_style.size.height = length(height);
        self
    }

    pub fn bg(mut self, color: Color) -> Self {
        self.background = Fill::Solid(color);
        self
    }

    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = color;
        self
    }

    pub fn border_color(mut self, color: Color) -> Self {
        self.border_color = color;
        self
    }

    pub fn focus_border_color(mut self, color: Color) -> Self {
        self.focus_border_color = color;
        self
    }

    pub fn rounded(mut self, radius: f32) -> Self {
        self.corner_radii = Corners::uniform(radius);
        self
    }
}

impl Element for TextInput {
    fn layout(&self, engine: &mut LayoutEngine, _font_system: &FontSystem) -> taffy::NodeId {
        engine.new_leaf(self.taffy_style.clone())
    }

    fn paint(
        &self,
        layouts: &[mozui_layout::ComputedLayout],
        index: &mut usize,
        draw_list: &mut DrawList,
        interactions: &mut InteractionMap,
        font_system: &FontSystem,
    ) {
        let layout = layouts[*index];
        *index += 1;

        let bounds = Rect::new(layout.x, layout.y, layout.width, layout.height);

        // Paint background with focus-aware border
        let border_color = if self.current.focused {
            self.focus_border_color
        } else {
            self.border_color
        };

        draw_list.push(DrawCommand::Rect {
            bounds,
            background: self.background.clone(),
            corner_radii: self.corner_radii,
            border: Some(Border {
                width: self.border_width,
                color: border_color,
            }),
                    shadow: None,
                });

        // Text area (inset by padding)
        let pad_left = 10.0f32;
        let pad_right = 10.0f32;
        let text_bounds = Rect::new(
            bounds.origin.x + pad_left,
            bounds.origin.y,
            bounds.size.width - pad_left - pad_right,
            bounds.size.height,
        );

        // Paint text or placeholder
        let display_text = if self.current.value.is_empty() {
            &self.placeholder
        } else {
            &self.current.value
        };
        let text_color = if self.current.value.is_empty() {
            self.placeholder_color
        } else {
            self.text_color
        };

        if !display_text.is_empty() {
            draw_list.push(DrawCommand::Text {
                text: display_text.clone(),
                bounds: text_bounds,
                font_size: self.font_size,
                color: text_color,
                weight: 400,
                italic: false,
            });
        }

        // Paint cursor caret when focused
        if self.current.focused {
            let cursor_char_idx = self.current.cursor.min(self.current.value.chars().count());
            let cursor_text: String = self.current.value.chars().take(cursor_char_idx).collect();

            let cursor_x = if cursor_text.is_empty() {
                text_bounds.origin.x
            } else {
                let style = mozui_text::TextStyle {
                    font_size: self.font_size,
                    ..Default::default()
                };
                let measured = mozui_text::measure_text(&cursor_text, &style, None, font_system);
                text_bounds.origin.x + measured.width
            };

            let caret_height = self.font_size + 4.0;
            let caret_y = bounds.origin.y + (bounds.size.height - caret_height) / 2.0;

            draw_list.push(DrawCommand::Rect {
                bounds: Rect::new(cursor_x, caret_y, 1.5, caret_height),
                background: Fill::Solid(self.text_color),
                corner_radii: Corners::ZERO,
                border: None,
                    shadow: None,
                });
        }

        // Register as focusable for click-to-focus and key input
        if let Some(ref on_change) = self.on_change {
            let handler_ptr = on_change.as_ref()
                as *const dyn Fn(Box<dyn FnOnce(&mut TextInputState)>, &mut dyn std::any::Any);

            // Focus handler
            let focus_handler: Box<dyn Fn(bool, &mut dyn std::any::Any)> =
                Box::new(move |focused, cx_any| unsafe {
                    (*handler_ptr)(
                        Box::new(move |s| {
                            s.focused = focused;
                            if focused {
                                s.cursor = s.value.chars().count();
                            }
                        }),
                        cx_any,
                    );
                });

            // Key handler
            let key_handler: Box<
                dyn Fn(mozui_events::Key, mozui_events::Modifiers, &mut dyn std::any::Any),
            > = Box::new(move |key, mods, cx_any| unsafe {
                (*handler_ptr)(
                    Box::new(move |s| {
                        handle_key_input(s, key, mods);
                    }),
                    cx_any,
                );
            });

            interactions.register_focusable(bounds, focus_handler, key_handler);
        }
    }
}

/// Handle a key press on a text input state.
fn handle_key_input(s: &mut TextInputState, key: mozui_events::Key, mods: mozui_events::Modifiers) {
    // Cmd+key shortcuts
    if mods.meta {
        match key {
            mozui_events::Key::Character('a') => {
                // Select all — move cursor to end (full selection deferred)
                s.cursor = s.value.chars().count();
            }
            mozui_events::Key::Character('c') => {
                // Copy all text to clipboard
                if !s.value.is_empty() {
                    mozui_platform::clipboard_write(&s.value);
                }
            }
            mozui_events::Key::Character('v') => {
                // Paste from clipboard
                if let Some(text) = mozui_platform::clipboard_read() {
                    let byte_pos = char_to_byte_pos(&s.value, s.cursor);
                    s.value.insert_str(byte_pos, &text);
                    s.cursor += text.chars().count();
                }
            }
            mozui_events::Key::Character('x') => {
                // Cut all text
                if !s.value.is_empty() {
                    mozui_platform::clipboard_write(&s.value);
                    s.value.clear();
                    s.cursor = 0;
                }
            }
            _ => {}
        }
        return;
    }

    match key {
        mozui_events::Key::Character(ch) => {
            let byte_pos = char_to_byte_pos(&s.value, s.cursor);
            s.value.insert(byte_pos, ch);
            s.cursor += 1;
        }
        mozui_events::Key::Space => {
            let byte_pos = char_to_byte_pos(&s.value, s.cursor);
            s.value.insert(byte_pos, ' ');
            s.cursor += 1;
        }
        mozui_events::Key::Backspace => {
            if s.cursor > 0 {
                let byte_pos = char_to_byte_pos(&s.value, s.cursor - 1);
                let next_byte = char_to_byte_pos(&s.value, s.cursor);
                s.value.drain(byte_pos..next_byte);
                s.cursor -= 1;
            }
        }
        mozui_events::Key::Delete => {
            let char_count = s.value.chars().count();
            if s.cursor < char_count {
                let byte_pos = char_to_byte_pos(&s.value, s.cursor);
                let next_byte = char_to_byte_pos(&s.value, s.cursor + 1);
                s.value.drain(byte_pos..next_byte);
            }
        }
        mozui_events::Key::ArrowLeft => {
            if s.cursor > 0 {
                s.cursor -= 1;
            }
        }
        mozui_events::Key::ArrowRight => {
            let char_count = s.value.chars().count();
            if s.cursor < char_count {
                s.cursor += 1;
            }
        }
        mozui_events::Key::Home => {
            s.cursor = 0;
        }
        mozui_events::Key::End => {
            s.cursor = s.value.chars().count();
        }
        _ => {}
    }
}

/// Convert a char index to a byte index in a string.
fn char_to_byte_pos(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(i, _)| i)
        .unwrap_or(s.len())
}
