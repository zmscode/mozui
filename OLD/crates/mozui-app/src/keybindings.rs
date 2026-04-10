use crate::actions::Action;
use mozui_events::{Key, Modifiers};
use std::collections::HashMap;

/// A key combination (key + modifiers).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyCombo {
    pub key: Key,
    pub modifiers: Modifiers,
}

impl KeyCombo {
    pub fn new(key: Key, modifiers: Modifiers) -> Self {
        Self { key, modifiers }
    }

    /// Parse a key combo string like "cmd-s", "ctrl-shift-z", "escape".
    /// `cmd` maps to `meta` (Cmd on macOS, Ctrl on other platforms in future).
    pub fn parse(s: &str) -> Result<Self, String> {
        let parts: Vec<&str> = s.split('-').collect();
        if parts.is_empty() {
            return Err("empty key combo".into());
        }

        let mut modifiers = Modifiers::default();
        let key_part = parts.last().unwrap();

        for &part in &parts[..parts.len() - 1] {
            match part.to_lowercase().as_str() {
                "cmd" | "meta" | "super" => modifiers.meta = true,
                "ctrl" | "control" => modifiers.ctrl = true,
                "alt" | "option" => modifiers.alt = true,
                "shift" => modifiers.shift = true,
                other => return Err(format!("unknown modifier: {other}")),
            }
        }

        let key = parse_key(key_part)?;
        Ok(Self { key, modifiers })
    }

    /// Check if this combo matches a key event.
    pub fn matches(&self, key: Key, modifiers: Modifiers) -> bool {
        self.key == key && self.modifiers == modifiers
    }
}

fn parse_key(s: &str) -> Result<Key, String> {
    match s.to_lowercase().as_str() {
        "enter" | "return" => Ok(Key::Enter),
        "escape" | "esc" => Ok(Key::Escape),
        "tab" => Ok(Key::Tab),
        "backspace" => Ok(Key::Backspace),
        "delete" | "del" => Ok(Key::Delete),
        "space" => Ok(Key::Space),
        "up" | "arrowup" => Ok(Key::ArrowUp),
        "down" | "arrowdown" => Ok(Key::ArrowDown),
        "left" | "arrowleft" => Ok(Key::ArrowLeft),
        "right" | "arrowright" => Ok(Key::ArrowRight),
        "home" => Ok(Key::Home),
        "end" => Ok(Key::End),
        "pageup" => Ok(Key::PageUp),
        "pagedown" => Ok(Key::PageDown),
        "f1" => Ok(Key::F1),
        "f2" => Ok(Key::F2),
        "f3" => Ok(Key::F3),
        "f4" => Ok(Key::F4),
        "f5" => Ok(Key::F5),
        "f6" => Ok(Key::F6),
        "f7" => Ok(Key::F7),
        "f8" => Ok(Key::F8),
        "f9" => Ok(Key::F9),
        "f10" => Ok(Key::F10),
        "f11" => Ok(Key::F11),
        "f12" => Ok(Key::F12),
        s if s.len() == 1 => {
            let ch = s.chars().next().unwrap();
            Ok(Key::Character(ch))
        }
        other => Err(format!("unknown key: {other}")),
    }
}

struct Keybinding {
    combo: KeyCombo,
    action: Box<dyn Action>,
}

/// Stores keybindings organized by context.
pub struct KeybindingRegistry {
    global: Vec<Keybinding>,
    contextual: HashMap<String, Vec<Keybinding>>,
}

impl KeybindingRegistry {
    pub fn new() -> Self {
        Self {
            global: Vec::new(),
            contextual: HashMap::new(),
        }
    }

    /// Bind a key combo to an action globally.
    pub fn bind(&mut self, combo: &str, action: impl Action + 'static) {
        match KeyCombo::parse(combo) {
            Ok(kc) => self.global.push(Keybinding {
                combo: kc,
                action: Box::new(action),
            }),
            Err(e) => tracing::warn!("invalid keybinding {combo:?}: {e}"),
        }
    }

    /// Bind a key combo to an action within a named context.
    pub fn bind_in(&mut self, context: &str, combo: &str, action: impl Action + 'static) {
        match KeyCombo::parse(combo) {
            Ok(kc) => {
                self.contextual
                    .entry(context.to_string())
                    .or_default()
                    .push(Keybinding {
                        combo: kc,
                        action: Box::new(action),
                    });
            }
            Err(e) => tracing::warn!("invalid keybinding {combo:?} in context {context:?}: {e}"),
        }
    }

    /// Look up a matching action for the given key event.
    /// Checks contextual bindings first (if a context is active), then global.
    pub fn match_key(
        &self,
        key: Key,
        modifiers: Modifiers,
        active_contexts: &[&str],
    ) -> Option<&dyn Action> {
        // Check contextual bindings (most specific first)
        for ctx in active_contexts.iter().rev() {
            if let Some(bindings) = self.contextual.get(*ctx) {
                for binding in bindings {
                    if binding.combo.matches(key, modifiers) {
                        return Some(binding.action.as_ref());
                    }
                }
            }
        }

        // Check global bindings
        for binding in &self.global {
            if binding.combo.matches(key, modifiers) {
                return Some(binding.action.as_ref());
            }
        }

        None
    }
}

/// Builder for configuring keybindings fluently.
pub struct KeybindingBuilder<'a> {
    registry: &'a mut KeybindingRegistry,
    context: Option<String>,
}

impl<'a> KeybindingBuilder<'a> {
    pub fn new(registry: &'a mut KeybindingRegistry) -> Self {
        Self {
            registry,
            context: None,
        }
    }

    /// Set the context for subsequent bindings.
    pub fn context(mut self, name: &str) -> Self {
        self.context = Some(name.to_string());
        self
    }

    /// Bind a key combo to an action.
    pub fn bind(self, combo: &str, action: impl Action + 'static) -> Self {
        if let Some(ref ctx) = self.context {
            self.registry.bind_in(ctx, combo, action);
        } else {
            self.registry.bind(combo, action);
        }
        self
    }
}
