use crate::context::Context;
use crate::keybindings::KeybindingRegistry;
use mozui_elements::{Element, InteractionMap};
use mozui_events::PlatformEvent;
use mozui_layout::LayoutEngine;
use mozui_platform::{WindowOptions, create_platform};
use mozui_renderer::{DrawList, Renderer};
use mozui_style::Theme;
use taffy::prelude::*;

pub type ActionHandler = Box<dyn Fn(&dyn crate::Action, &mut Context)>;

pub struct App {
    theme: Theme,
    window_options: WindowOptions,
    keybindings: KeybindingRegistry,
    action_handler: Option<ActionHandler>,
}

impl App {
    pub fn new() -> Self {
        Self {
            theme: Theme::dark(),
            window_options: WindowOptions::default(),
            keybindings: KeybindingRegistry::new(),
            action_handler: None,
        }
    }

    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    pub fn window(mut self, options: WindowOptions) -> Self {
        self.window_options = options;
        self
    }

    /// Configure keybindings.
    pub fn keybindings(mut self, f: impl FnOnce(&mut KeybindingRegistry)) -> Self {
        f(&mut self.keybindings);
        self
    }

    /// Set a global action handler that fires when a keybinding matches.
    pub fn on_action(mut self, handler: impl Fn(&dyn crate::Action, &mut Context) + 'static) -> Self {
        self.action_handler = Some(Box::new(handler));
        self
    }

    pub fn run<F>(self, root: F) -> !
    where
        F: Fn(&mut Context) -> Box<dyn Element> + 'static,
    {
        let mut platform = create_platform();
        let window = platform.open_window(self.window_options);

        let mut renderer = Renderer::new(window.as_ref());
        let mut cx = Context::new(self.theme.clone());
        let mut layout_engine = LayoutEngine::new();
        let bg_color = self.theme.background;

        let mut needs_render = true;
        let mut interactions = InteractionMap::new();
        let keybindings = self.keybindings;
        let action_handler = self.action_handler;

        // Build initial element tree
        cx.reset_hooks();
        let mut element_tree: Box<dyn Element> = root(&mut cx);

        // Shared rebuild logic: after element tree changes, immediately
        // re-layout and repaint so interaction handlers point to live closures.
        let mut draw_list = DrawList::new();

        let rebuild_interactions =
            |element_tree: &Box<dyn Element>,
             layout_engine: &mut LayoutEngine,
             renderer: &Renderer,
             interactions: &mut InteractionMap,
             draw_list: &mut DrawList,
             window: &dyn mozui_platform::PlatformWindow| {
                let size = window.content_size();
                layout_engine.clear();
                let root_node = element_tree.layout(layout_engine, renderer.font_system());
                layout_engine.compute_layout(
                    root_node,
                    taffy::prelude::Size {
                        width: AvailableSpace::Definite(size.width),
                        height: AvailableSpace::Definite(size.height),
                    },
                );
                let layouts = layout_engine.collect_layouts(root_node);
                draw_list.clear();
                interactions.clear();
                let mut index = 0;
                element_tree.paint(
                    &layouts,
                    &mut index,
                    draw_list,
                    interactions,
                    renderer.font_system(),
                );
            };

        platform.run(Box::new(move |event| {
            match event {
                PlatformEvent::RedrawRequested => {
                    if needs_render {
                        let size = window.content_size();
                        let scale = window.scale_factor();

                        rebuild_interactions(
                            &element_tree,
                            &mut layout_engine,
                            &renderer,
                            &mut interactions,
                            &mut draw_list,
                            window.as_ref(),
                        );

                        renderer.render(bg_color, &draw_list, size, scale);
                        needs_render = false;
                    }
                }
                PlatformEvent::MouseUp {
                    button: mozui_events::MouseButton::Left,
                    position,
                    ..
                } => {
                    if interactions.dispatch_click(position, &mut cx) {
                        cx.clear_dirty();
                        cx.reset_hooks();
                        element_tree = root(&mut cx);
                        // Immediately rebuild interactions so handlers point to live tree
                        rebuild_interactions(
                            &element_tree,
                            &mut layout_engine,
                            &renderer,
                            &mut interactions,
                            &mut draw_list,
                            window.as_ref(),
                        );
                        needs_render = true;
                        window.request_redraw();
                    }
                }
                PlatformEvent::KeyDown { key, modifiers, .. } => {
                    let mut handled = false;

                    // 1. Check keybindings first
                    if let Some(action) = keybindings.match_key(key, modifiers, &[]) {
                        if let Some(ref handler) = action_handler {
                            let action_clone = action.boxed_clone();
                            handler(action_clone.as_ref(), &mut cx);
                            handled = true;
                        }
                    }

                    // 2. Tab cycles focus
                    if !handled && key == mozui_events::Key::Tab {
                        if interactions.cycle_focus(modifiers.shift, &mut cx) {
                            handled = true;
                        }
                    }

                    // 3. Dispatch to focused element + global key handlers
                    if !handled {
                        interactions.dispatch_key(key, modifiers, &mut cx);
                    }

                    // Rebuild if anything changed
                    if cx.is_dirty() || handled {
                        cx.clear_dirty();
                        cx.reset_hooks();
                        element_tree = root(&mut cx);
                        rebuild_interactions(
                            &element_tree,
                            &mut layout_engine,
                            &renderer,
                            &mut interactions,
                            &mut draw_list,
                            window.as_ref(),
                        );
                        needs_render = true;
                        window.request_redraw();
                    }
                }
                PlatformEvent::MouseMove { position, .. } => {
                    mozui_platform::set_cursor_style(interactions.cursor_at(position));
                }
                PlatformEvent::WindowResize { size } => {
                    let scale = window.scale_factor();
                    renderer.resize(size, scale);
                    cx.reset_hooks();
                    element_tree = root(&mut cx);
                    needs_render = true;
                    window.request_redraw();
                }
                PlatformEvent::ScaleFactorChanged { scale } => {
                    let size = window.content_size();
                    renderer.resize(size, scale);
                    needs_render = true;
                    window.request_redraw();
                }
                PlatformEvent::WindowCloseRequested => {
                    std::process::exit(0);
                }
                _ => {
                    tracing::trace!(?event, "platform event");
                }
            }
        }))
    }
}
