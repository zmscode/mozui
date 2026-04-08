use crate::{Element, InteractionMap};
use mozui_layout::LayoutEngine;
use mozui_renderer::{Border, DrawCommand, DrawList};
use mozui_style::{Color, Corners, Fill};
use mozui_text::FontSystem;
use taffy::prelude::*;

/// Display a keyboard shortcut with platform-appropriate symbols.
///
/// On macOS, modifiers render as symbols: ⌘ ⇧ ⌥ ⌃
/// On other platforms, modifiers render as text: Ctrl+Shift+Alt
pub struct Kbd {
    keys: Vec<String>,
    font_size: f32,
    color: Color,
    border_color: Color,
    bg_color: Color,
}

/// Parse a combo string like "cmd-s" or "ctrl-shift-z" into display tokens.
fn parse_combo_to_display(combo: &str) -> Vec<String> {
    let parts: Vec<&str> = combo.split('-').collect();
    let mut tokens = Vec::new();

    let is_macos = cfg!(target_os = "macos");

    for (i, part) in parts.iter().enumerate() {
        let is_last = i == parts.len() - 1;
        if is_last {
            // This is the key itself
            tokens.push(format_key(part));
        } else {
            // This is a modifier
            tokens.push(format_modifier(part, is_macos));
        }
    }

    tokens
}

fn format_modifier(m: &str, _macos: bool) -> String {
    // Use ASCII-friendly abbreviations that render reliably in any font.
    // macOS convention would be ⌘⇧⌥⌃ but those require specific font support.
    match m.to_lowercase().as_str() {
        "cmd" | "meta" | "super" => "Cmd+".to_string(),
        "ctrl" | "control" => "Ctrl+".to_string(),
        "shift" => "Shift+".to_string(),
        "alt" | "option" => "Alt+".to_string(),
        other => format!("{other}+"),
    }
}

fn format_key(k: &str) -> String {
    match k.to_lowercase().as_str() {
        "enter" | "return" => "\u{21A9}".to_string(),
        "escape" | "esc" => "Esc".to_string(),
        "tab" => "\u{21E5}".to_string(),
        "backspace" => "\u{232B}".to_string(),
        "delete" | "del" => "\u{2326}".to_string(),
        "space" => "\u{2423}".to_string(),
        "up" | "arrowup" => "\u{2191}".to_string(),
        "down" | "arrowdown" => "\u{2193}".to_string(),
        "left" | "arrowleft" => "\u{2190}".to_string(),
        "right" | "arrowright" => "\u{2192}".to_string(),
        s if s.len() == 1 => s.to_uppercase(),
        other => other.to_string(),
    }
}

pub fn kbd(combo: &str) -> Kbd {
    let keys = parse_combo_to_display(combo);
    Kbd {
        keys,
        font_size: 11.0,
        color: Color::new(1.0, 1.0, 1.0, 0.7),
        border_color: Color::new(1.0, 1.0, 1.0, 0.15),
        bg_color: Color::new(1.0, 1.0, 1.0, 0.06),
    }
}

impl Kbd {
    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    fn display_string(&self) -> String {
        // Modifiers already include trailing "+", so just concatenate.
        self.keys.join("")
    }
}

impl Element for Kbd {
    fn layout(&self, engine: &mut LayoutEngine, font_system: &FontSystem) -> taffy::NodeId {
        let display = self.display_string();
        let text_style = mozui_text::TextStyle {
            font_size: self.font_size,
            color: self.color,
            ..Default::default()
        };

        let measured = mozui_text::measure_text(&display, &text_style, None, font_system);

        let px = 5.0_f32;
        let py = 2.0_f32;

        engine.new_leaf(Style {
            size: Size {
                width: length(measured.width + px * 2.0),
                height: length(measured.height + py * 2.0),
            },
            ..Default::default()
        })
    }

    fn paint(
        &self,
        layouts: &[mozui_layout::ComputedLayout],
        index: &mut usize,
        draw_list: &mut DrawList,
        _interactions: &mut InteractionMap,
        _font_system: &FontSystem,
    ) {
        let layout = layouts[*index];
        *index += 1;

        let bounds = mozui_style::Rect::new(layout.x, layout.y, layout.width, layout.height);

        // Background with border
        draw_list.push(DrawCommand::Rect {
            bounds,
            background: Fill::Solid(self.bg_color),
            corner_radii: Corners::uniform(4.0),
            border: Some(Border {
                width: 1.0,
                color: self.border_color,
            }),
        });

        // Text centered within the badge
        let px = 5.0_f32;
        let py = 2.0_f32;
        let text_bounds = mozui_style::Rect::new(
            layout.x + px,
            layout.y + py,
            layout.width - px * 2.0,
            layout.height - py * 2.0,
        );

        draw_list.push(DrawCommand::Text {
            text: self.display_string(),
            bounds: text_bounds,
            font_size: self.font_size,
            color: self.color,
            weight: 400,
            italic: false,
        });
    }
}
