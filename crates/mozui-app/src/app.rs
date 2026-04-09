use crate::context::Context;
use crate::keybindings::KeybindingRegistry;
use mozui_elements::{Element, InteractionMap};
use mozui_events::{PlatformEvent, WindowId};
use mozui_layout::LayoutEngine;
use mozui_platform::{WindowOptions, create_platform};
use mozui_renderer::{DrawList, Renderer};
use mozui_style::Theme;
use std::collections::HashMap;
use taffy::prelude::*;

pub type ActionHandler = Box<dyn Fn(&dyn crate::Action, &mut Context)>;
pub type RootBuilder = Box<dyn Fn(&mut Context) -> Box<dyn Element>>;

/// Per-window rendering and interaction state.
struct WindowState {
    window: Box<dyn mozui_platform::PlatformWindow>,
    renderer: Renderer,
    layout_engine: LayoutEngine,
    interactions: InteractionMap,
    draw_list: DrawList,
    cached_layouts: Vec<mozui_layout::ComputedLayout>,
    element_tree: Option<Box<dyn Element>>,
    root_builder: RootBuilder,
    needs_render: bool,
    needs_layout: bool,
    last_size: mozui_style::Size,
    last_scale: f32,
    bg_color: mozui_style::Color,
}

impl WindowState {
    fn new(
        window: Box<dyn mozui_platform::PlatformWindow>,
        root_builder: RootBuilder,
        bg_color: mozui_style::Color,
    ) -> Self {
        let renderer = Renderer::new(window.as_ref());
        let last_size = window.content_size();
        let last_scale = window.scale_factor();
        Self {
            window,
            renderer,
            layout_engine: LayoutEngine::new(),
            interactions: InteractionMap::new(),
            draw_list: DrawList::new(),
            cached_layouts: Vec::new(),
            element_tree: None,
            root_builder,
            needs_render: true,
            needs_layout: true,
            last_size,
            last_scale,
            bg_color,
        }
    }

    fn rebuild(&mut self, cx: &mut Context) {
        cx.reset_hooks();
        self.element_tree = Some((self.root_builder)(cx));
        self.needs_layout = true;
        self.needs_render = true;
    }

    fn rebuild_interactions(&mut self) {
        let element_tree = self.element_tree.as_ref().unwrap();
        let size = self.window.content_size();
        self.layout_engine.clear();
        let root_node = element_tree.layout(&mut self.layout_engine, self.renderer.font_system());
        self.layout_engine.compute_layout(
            root_node,
            taffy::prelude::Size {
                width: AvailableSpace::Definite(size.width),
                height: AvailableSpace::Definite(size.height),
            },
        );
        self.cached_layouts = self.layout_engine.collect_layouts(root_node);
        self.draw_list.clear();
        self.interactions.clear();
        let mut index = 0;
        element_tree.paint(
            &self.cached_layouts,
            &mut index,
            &mut self.draw_list,
            &mut self.interactions,
            self.renderer.font_system(),
        );
    }

    fn repaint_only(&mut self) {
        let element_tree = self.element_tree.as_ref().unwrap();
        self.draw_list.clear();
        self.interactions.clear();
        let mut index = 0;
        element_tree.paint(
            &self.cached_layouts,
            &mut index,
            &mut self.draw_list,
            &mut self.interactions,
            self.renderer.font_system(),
        );
    }

    fn render(&mut self) {
        if !self.needs_render {
            return;
        }
        let size = self.window.content_size();
        let scale = self.window.scale_factor();

        if self.needs_layout {
            self.rebuild_interactions();
            self.needs_layout = false;
        } else {
            self.repaint_only();
        }

        self.renderer
            .render(self.bg_color, &self.draw_list, size, scale);
        self.needs_render = false;
    }
}

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
    pub fn on_action(
        mut self,
        handler: impl Fn(&dyn crate::Action, &mut Context) + 'static,
    ) -> Self {
        self.action_handler = Some(Box::new(handler));
        self
    }

    pub fn run<F>(self, root: F) -> !
    where
        F: Fn(&mut Context) -> Box<dyn Element> + 'static,
    {
        let mut platform = create_platform();
        let (main_id, window) = platform.open_window(self.window_options);

        let mut cx = Context::new(self.theme.clone());

        // Set up clipboard access
        cx.set_clipboard(
            || mozui_platform::clipboard_read(),
            |text| mozui_platform::clipboard_write(text),
        );

        let bg_color = self.theme.background;

        // Create main window state
        let mut windows: HashMap<WindowId, WindowState> = HashMap::new();
        let mut ws = WindowState::new(window, Box::new(root), bg_color);
        ws.rebuild(&mut cx);
        windows.insert(main_id, ws);

        let keybindings = self.keybindings;
        let action_handler = self.action_handler;

        // Track whether shared state changed (requires all-window rebuild)
        let mut global_dirty = false;
        // Counter for dynamically opened windows (starts after MAIN = 0)
        let mut next_window_id: u64 = 1;

        platform.run(Box::new(move |window_id, event| {
            match event {
                PlatformEvent::RedrawRequested => {
                    // Handle global state changes first (timers, async, signals, animations)
                    // We only check on the main window's redraw to avoid duplicate work.
                    if window_id == WindowId::MAIN {
                        // Process pending window-open requests
                        let reqs = std::mem::take(&mut cx.window_requests);
                        for req in reqs {
                            let new_window = mozui_platform::create_window(req.options);
                            let new_id = WindowId(next_window_id);
                            next_window_id += 1;
                            let mut new_ws =
                                WindowState::new(new_window, req.root_builder, bg_color);
                            new_ws.rebuild(&mut cx);
                            windows.insert(new_id, new_ws);
                        }

                        // Process pending window-close requests
                        let close_reqs = std::mem::take(&mut cx.close_requests);
                        for id in close_reqs {
                            if let Some(mut ws) = windows.remove(&id) {
                                ws.window.close();
                            }
                        }

                        // Fire expired timers
                        let mut timers =
                            std::mem::replace(&mut cx.timers, mozui_executor::TimerManager::new());
                        if timers.fire_expired(&mut cx) {
                            global_dirty = true;
                        }
                        cx.timers = timers;

                        // Poll async tasks
                        if cx.executor.poll_ready() {
                            global_dirty = true;
                        }

                        // Check animations
                        let animations_running = cx.has_active_animations();
                        cx.clear_animation_flag();

                        if cx.is_dirty() || animations_running {
                            global_dirty = true;
                        }

                        if global_dirty {
                            cx.clear_dirty();
                            for w in windows.values_mut() {
                                w.rebuild(&mut cx);
                            }
                            global_dirty = false;
                        }
                    }

                    let Some(ws) = windows.get_mut(&window_id) else {
                        return;
                    };

                    if !ws.window.is_visible() {
                        if window_id == WindowId::MAIN {
                            std::process::exit(0);
                        }
                        return;
                    }

                    // Detect window resize
                    let current_size = ws.window.content_size();
                    let current_scale = ws.window.scale_factor();
                    if current_size != ws.last_size || current_scale != ws.last_scale {
                        ws.renderer.resize(current_size, current_scale);
                        ws.last_size = current_size;
                        ws.last_scale = current_scale;
                        ws.rebuild(&mut cx);
                    }

                    ws.render();

                    if cx.has_active_animations() {
                        ws.window.request_redraw();
                    }
                }
                _ => {
                    let Some(ws) = windows.get_mut(&window_id) else {
                        return;
                    };
                    handle_input_event(ws, event, &mut cx, &keybindings, &action_handler);
                }
            }
        }))
    }
}

/// Handle non-redraw input events for a specific window.
fn handle_input_event(
    ws: &mut WindowState,
    event: PlatformEvent,
    cx: &mut Context,
    keybindings: &KeybindingRegistry,
    action_handler: &Option<ActionHandler>,
) {
    match event {
        PlatformEvent::MouseDown {
            button: mozui_events::MouseButton::Left,
            position,
            ..
        } => {
            ws.interactions.set_mouse_pressed(position);
            if ws.interactions.dispatch_drag_start(position, cx) {
                if cx.is_dirty() {
                    cx.clear_dirty();
                    ws.rebuild(cx);
                    ws.rebuild_interactions();
                }
            } else if ws.interactions.dnd_mouse_down(position) {
                // DnD source found
            } else if ws.interactions.is_drag_region(position) {
                ws.window.begin_drag_move();
            }
            ws.needs_render = true;
            ws.window.request_redraw();
        }
        PlatformEvent::MouseUp {
            button: mozui_events::MouseButton::Left,
            position,
            ..
        } => {
            ws.interactions.set_mouse_released();
            ws.interactions.clear_active_drag();
            let dnd_dropped = ws.interactions.dnd_mouse_up(position, cx);
            if dnd_dropped {
                if cx.is_dirty() {
                    cx.clear_dirty();
                    ws.rebuild(cx);
                    ws.rebuild_interactions();
                }
            } else {
                ws.interactions.dnd_cancel();
                if ws.interactions.dispatch_click(position, cx) {
                    cx.clear_dirty();
                    ws.rebuild(cx);
                    ws.rebuild_interactions();
                }
            }
            ws.needs_render = true;
            ws.window.request_redraw();
        }
        PlatformEvent::KeyDown { key, modifiers, .. } => {
            let mut handled = false;

            if let Some(action) = keybindings.match_key(key, modifiers, &[]) {
                if let Some(handler) = action_handler {
                    let action_clone = action.boxed_clone();
                    handler(action_clone.as_ref(), cx);
                    handled = true;
                }
            }

            if !handled && key == mozui_events::Key::Tab {
                if ws.interactions.cycle_focus(modifiers.shift, cx) {
                    handled = true;
                }
            }

            if !handled {
                ws.interactions.dispatch_key(key, modifiers, cx);
            }

            if cx.is_dirty() || handled {
                cx.clear_dirty();
                ws.rebuild(cx);
                ws.rebuild_interactions();
                ws.needs_render = true;
                ws.window.request_redraw();
            }
        }
        PlatformEvent::ScrollWheel {
            delta, position, ..
        } => {
            let (dx, dy) = match delta {
                mozui_events::ScrollDelta::Pixels(dx, dy) => (dx, dy),
                mozui_events::ScrollDelta::Lines(dx, dy) => (dx * 20.0, dy * 20.0),
            };
            if ws.interactions.dispatch_scroll(position, dx, dy, cx) {
                ws.needs_render = true;
                ws.window.request_redraw();
            }
        }
        PlatformEvent::MouseMove { position, .. } => {
            let old_pos = ws.interactions.mouse_position();
            ws.interactions.set_mouse_position(position);
            mozui_platform::set_cursor_style(ws.interactions.cursor_at(position));

            if ws.interactions.is_mouse_pressed() {
                if ws.interactions.dispatch_drag_move(position, cx) {
                    if cx.is_dirty() {
                        cx.clear_dirty();
                        ws.rebuild(cx);
                        ws.rebuild_interactions();
                    }
                    ws.needs_render = true;
                    ws.window.request_redraw();
                }

                if ws.interactions.dnd_mouse_move(position) {
                    ws.needs_render = true;
                    ws.window.request_redraw();
                }
            }

            if old_pos != position
                && (ws.interactions.has_handler_at(position)
                    || ws.interactions.has_handler_at(old_pos)
                    || ws.interactions.is_mouse_pressed())
            {
                ws.needs_render = true;
                ws.window.request_redraw();
            }
        }
        PlatformEvent::WindowResize { size } => {
            let scale = ws.window.scale_factor();
            ws.renderer.resize(size, scale);
            ws.rebuild(cx);
            ws.window.request_redraw();
        }
        PlatformEvent::ScaleFactorChanged { scale } => {
            let size = ws.window.content_size();
            ws.renderer.resize(size, scale);
            ws.needs_layout = true;
            ws.needs_render = true;
            ws.window.request_redraw();
        }
        PlatformEvent::WindowCloseRequested => {
            std::process::exit(0);
        }
        _ => {
            tracing::trace!(?event, "platform event");
        }
    }
}
