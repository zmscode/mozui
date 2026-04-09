use crate::context::Context;
use crate::keybindings::KeybindingRegistry;
use mozui_devtools::DevtoolsState;
use mozui_elements::{
    DeferredEntry, DeferredPosition, Element, InteractionMap, LayoutCache, LayoutContext,
    PaintContext, ResolvedDeferred, compute_anchored_position,
};
use mozui_events::{PlatformEvent, WindowId};
use mozui_layout::LayoutEngine;
use mozui_platform::{WindowOptions, create_platform};
use mozui_renderer::{DrawList, Renderer};
use mozui_style::{Size, Theme};
use std::collections::HashMap;
use web_time::Instant;
use taffy::prelude::*;

pub type ActionHandler = Box<dyn Fn(&dyn crate::Action, &mut Context)>;
pub type RootBuilder = Box<dyn Fn(&mut Context) -> Box<dyn Element>>;

/// Per-window rendering and interaction state.
struct WindowState {
    window: Box<dyn mozui_platform::PlatformWindow>,
    renderer: Renderer,
    layout_engine: LayoutEngine,
    layout_cache: LayoutCache,
    interactions: InteractionMap,
    draw_list: DrawList,
    element_tree: Option<Box<dyn Element>>,
    root_builder: RootBuilder,
    needs_render: bool,
    needs_layout: bool,
    last_size: mozui_style::Size,
    last_scale: f32,
    bg_color: mozui_style::Color,
    frame_count: u64,
    devtools: DevtoolsState,
    signal_summary_cache: Vec<(usize, &'static str)>,
}

impl WindowState {
    fn new(
        window: Box<dyn mozui_platform::PlatformWindow>,
        renderer: Renderer,
        root_builder: RootBuilder,
        bg_color: mozui_style::Color,
    ) -> Self {
        let last_size = window.content_size();
        let last_scale = window.scale_factor();
        Self {
            window,
            renderer,
            layout_engine: LayoutEngine::new(),
            layout_cache: LayoutCache::new(),
            interactions: InteractionMap::new(),
            draw_list: DrawList::new(),
            element_tree: None,
            root_builder,
            needs_render: true,
            needs_layout: true,
            last_size,
            last_scale,
            bg_color,
            frame_count: 0,
            devtools: DevtoolsState::new(),
            signal_summary_cache: Vec::new(),
        }
    }

    fn rebuild(&mut self, _cx: &mut Context) {
        // Mark for re-render. The actual tree rebuild happens in render()
        // so deferred elements always get fresh children.
        self.needs_layout = true;
        self.needs_render = true;
    }

    /// Run the three-phase layout → prepaint → paint pipeline.
    fn rebuild_interactions(&mut self) {
        let frame_start = Instant::now();
        let size = self.window.content_size();
        let window_size = Size::new(size.width, size.height);

        // Take the element tree out so we can mutably borrow it
        // alongside the layout engine / draw list / interactions.
        let mut tree = self.element_tree.take().unwrap();

        // Phase 1: Layout (collects deferred entries)
        // Periodic full GC every 120 frames to prevent unbounded node growth
        let layout_start = Instant::now();
        self.frame_count += 1;
        if self.frame_count % 120 == 0 {
            self.layout_engine.clear();
            self.layout_cache.clear();
        } else {
            self.layout_engine.begin_frame();
        }
        self.layout_cache.begin_frame();
        let mut deferred = Vec::new();
        let root_id = {
            let mut lcx = LayoutContext::new(
                &mut self.layout_engine,
                self.renderer.font_system(),
                &mut deferred,
                &mut self.layout_cache,
            );
            tree.layout(&mut lcx)
        };

        self.layout_engine.compute_layout(
            root_id,
            taffy::prelude::Size {
                width: AvailableSpace::Definite(size.width),
                height: AvailableSpace::Definite(size.height),
            },
            self.renderer.font_system(),
        );

        // Inject devtools overlays
        if self.devtools.perf_overlay_active {
            deferred.push(DeferredEntry {
                element: crate::devtools::perf_overlay(&self.devtools.timings),
                position: DeferredPosition::Overlay,
            });
        }
        if self.devtools.signal_debugger_active {
            deferred.push(DeferredEntry {
                element: crate::devtools::signal_panel::signal_panel(
                    &self.devtools.signal_log,
                    &self.signal_summary_cache,
                ),
                position: DeferredPosition::Overlay,
            });
        }
        if self.devtools.inspector_active {
            deferred.push(DeferredEntry {
                element: crate::devtools::inspector::inspector_overlay(&self.devtools.inspector),
                position: DeferredPosition::Overlay,
            });
        }

        // Resolve deferred elements (independent layout trees)
        let mut resolved_deferred = resolve_deferred(
            deferred,
            &self.layout_engine,
            window_size,
            self.renderer.font_system(),
        );
        let layout_us = layout_start.elapsed().as_micros() as u64;

        // Phase 2 & 3: Prepaint + Paint (main tree)
        let paint_start = Instant::now();
        self.draw_list.clear();
        self.interactions.clear();
        let root_bounds = {
            let cl = self.layout_engine.bounds(root_id);
            mozui_style::Rect::new(cl.x, cl.y, cl.width, cl.height)
        };
        {
            let mut debug_tree = Vec::new();
            let mut pcx = PaintContext::new(
                &self.layout_engine,
                &mut self.draw_list,
                &mut self.interactions,
                self.renderer.font_system(),
                window_size,
            );
            if self.devtools.inspector_active {
                pcx.debug_collector = Some(&mut debug_tree);
            }
            tree.prepaint(root_bounds, &mut pcx);
            tree.paint(root_bounds, &mut pcx);

            // Update inspector element tree and do hit-testing
            if self.devtools.inspector_active {
                pcx.debug_collector = None;
                let mouse = self.interactions.mouse_position();
                self.devtools.inspector.tree = debug_tree;
                self.devtools.inspector.hovered = self
                    .devtools
                    .inspector
                    .tree
                    .iter()
                    .rposition(|e| {
                        mouse.x >= e.x
                            && mouse.x <= e.x + e.width
                            && mouse.y >= e.y
                            && mouse.y <= e.y + e.height
                    });
            }
        }

        // Paint deferred elements on top of the main tree
        paint_deferred(
            &mut resolved_deferred,
            &mut self.draw_list,
            &mut self.interactions,
            self.renderer.font_system(),
            window_size,
        );
        let paint_us = paint_start.elapsed().as_micros() as u64;

        // Collect frame stats (render_us filled in after GPU render)
        let draw_call_count = self.draw_list.len() as u32;
        let element_count = self.interactions.entry_count() as u32;
        let node_count = self.layout_engine.node_count() as u32;
        let total_us = frame_start.elapsed().as_micros() as u64;

        self.devtools.timings.push(mozui_devtools::FrameTiming {
            layout_us,
            paint_us,
            render_us: 0, // filled in after renderer.render()
            total_us,
            draw_call_count,
            element_count,
            node_count,
        });

        // Put the tree back
        self.element_tree = Some(tree);
    }

    fn render(&mut self, cx: &mut Context) {
        if !self.needs_render {
            return;
        }

        // Always rebuild the element tree so deferred elements (Sheet, Dialog,
        // Notification) get fresh children. They use std::mem::take to move
        // children into their overlay, so the tree can't be reused across frames.
        cx.reset_hooks();
        self.element_tree = Some((self.root_builder)(cx));

        // Drain mutation buffer and push to signal log
        {
            let mut buf = cx.mutation_buffer.borrow_mut();
            for (slot_id, type_name) in buf.drain(..) {
                self.devtools.signal_log.push(mozui_devtools::SignalMutation {
                    slot_id,
                    type_name,
                    old_debug: String::new(),
                    new_debug: String::new(),
                    frame: self.frame_count,
                    timestamp: Instant::now(),
                });
            }
        }
        self.devtools.signal_log.current_frame = self.frame_count;

        // Capture signal summary for devtools before layout
        if self.devtools.signal_debugger_active {
            self.signal_summary_cache = cx.signal_summary();
        }

        let size = self.window.content_size();
        let scale = self.window.scale_factor();

        self.rebuild_interactions();

        self.renderer
            .render(self.bg_color, &self.draw_list, size, scale);
        self.needs_render = false;
        self.needs_layout = false;
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

    /// Run the application (native targets).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn run<F>(self, root: F) -> !
    where
        F: Fn(&mut Context) -> Box<dyn Element> + 'static,
    {
        let mut platform = create_platform();
        mozui_platform::install_services(platform.as_ref());
        let (main_id, window) = platform.open_window(self.window_options);
        let renderer = Renderer::new(window.as_ref());
        Self::run_event_loop(platform, main_id, window, renderer, root, self.theme, self.keybindings, self.action_handler)
    }

    /// Run the application (WASM target).
    ///
    /// GPU initialization is async on the web, so this method is async.
    /// Call it from a `wasm_bindgen_futures::spawn_local` block.
    #[cfg(target_arch = "wasm32")]
    pub async fn start<F>(self, root: F)
    where
        F: Fn(&mut Context) -> Box<dyn Element> + 'static,
    {
        use wasm_bindgen::JsCast;

        use mozui_platform::Platform;

        let mut platform = mozui_platform::web::WebPlatform::new();
        mozui_platform::install_services(&platform);
        let (main_id, window): (WindowId, Box<dyn mozui_platform::PlatformWindow>) =
            platform.open_window(self.window_options);

        // Get the canvas element that open_window created/found
        let canvas: web_sys::HtmlCanvasElement = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id("mozui-canvas"))
            .and_then(|el| el.dyn_into().ok())
            .expect("mozui-canvas element not found in DOM");

        // Async GPU initialization
        let (gpu, surface) = mozui_renderer::GpuContext::new_with_canvas(canvas).await;
        let size = window.content_size();
        let scale = window.scale_factor();
        let renderer = Renderer::from_gpu(gpu, surface, size, scale);

        // Build event callback and start the RAF loop (returns normally)
        let callback = Self::build_event_callback(
            main_id, window, renderer, root, self.theme, self.keybindings, self.action_handler,
        );
        platform.start_event_loop(callback);
    }

    /// Build the event callback that drives the app each frame.
    fn build_event_callback<F>(
        main_id: WindowId,
        window: Box<dyn mozui_platform::PlatformWindow>,
        renderer: Renderer,
        root: F,
        theme: Theme,
        keybindings: KeybindingRegistry,
        action_handler: Option<ActionHandler>,
    ) -> mozui_platform::EventCallback
    where
        F: Fn(&mut Context) -> Box<dyn Element> + 'static,
    {
        let mut cx = Context::new(theme.clone());

        // Set up clipboard access via platform services
        cx.set_clipboard(
            || mozui_platform::clipboard_read(),
            |text| mozui_platform::clipboard_write(text),
        );

        let bg_color = theme.background;

        // Create main window state
        let mut windows: HashMap<WindowId, WindowState> = HashMap::new();
        let mut ws = WindowState::new(window, renderer, Box::new(root), bg_color);
        ws.rebuild(&mut cx);
        windows.insert(main_id, ws);

        // Track whether shared state changed (requires all-window rebuild)
        let mut global_dirty = false;
        // Counter for dynamically opened windows (starts after MAIN = 0)
        #[allow(unused_mut, unused_variables)]
        let mut next_window_id: u64 = 1;

        Box::new(move |window_id, event| {
            match event {
                PlatformEvent::RedrawRequested => {
                    if window_id == WindowId::MAIN {
                        // Process pending window-open requests (native only)
                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            let reqs = std::mem::take(&mut cx.window_requests);
                            for req in reqs {
                                let new_window = mozui_platform::create_window(req.options);
                                let new_id = WindowId(next_window_id);
                                next_window_id += 1;
                                let new_renderer = Renderer::new(new_window.as_ref());
                                let mut new_ws =
                                    WindowState::new(new_window, new_renderer, req.root_builder, bg_color);
                                new_ws.rebuild(&mut cx);
                                windows.insert(new_id, new_ws);
                            }
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
                        #[cfg(not(target_arch = "wasm32"))]
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

                    // Force re-render when devtools overlays are active
                    if ws.devtools.any_active() {
                        ws.needs_render = true;
                    }

                    ws.render(&mut cx);

                    if cx.has_active_animations() || ws.devtools.any_active() {
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
        })
    }

    /// Run the event loop (native path). Calls Platform::run which never returns.
    #[cfg(not(target_arch = "wasm32"))]
    fn run_event_loop<F>(
        mut platform: Box<dyn mozui_platform::Platform>,
        main_id: WindowId,
        window: Box<dyn mozui_platform::PlatformWindow>,
        renderer: Renderer,
        root: F,
        theme: Theme,
        keybindings: KeybindingRegistry,
        action_handler: Option<ActionHandler>,
    ) -> !
    where
        F: Fn(&mut Context) -> Box<dyn Element> + 'static,
    {
        let callback = Self::build_event_callback(main_id, window, renderer, root, theme, keybindings, action_handler);
        platform.run(callback)
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
        PlatformEvent::MouseDown {
            button: mozui_events::MouseButton::Right,
            position,
            ..
        } => {
            if ws.interactions.dispatch_right_click(position, cx) {
                cx.clear_dirty();
                ws.rebuild(cx);
                ws.rebuild_interactions();
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

            // Devtools toggles: Cmd+Shift+O / Cmd+Shift+I / Cmd+Shift+S
            if modifiers.meta && modifiers.shift {
                match key {
                    mozui_events::Key::Character('o') | mozui_events::Key::Character('O') => {
                        ws.devtools.perf_overlay_active = !ws.devtools.perf_overlay_active;
                        ws.needs_render = true;
                        ws.window.request_redraw();
                        handled = true;
                    }
                    mozui_events::Key::Character('i') | mozui_events::Key::Character('I') => {
                        ws.devtools.inspector_active = !ws.devtools.inspector_active;
                        ws.needs_render = true;
                        ws.window.request_redraw();
                        handled = true;
                    }
                    mozui_events::Key::Character('d') | mozui_events::Key::Character('D') => {
                        ws.devtools.signal_debugger_active = !ws.devtools.signal_debugger_active;
                        ws.needs_render = true;
                        ws.window.request_redraw();
                        handled = true;
                    }
                    _ => {}
                }
            }

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

// ── Deferred element resolution ───────────────────────────────────

/// Lay out and solve each deferred element in its own independent layout tree.
fn resolve_deferred(
    deferred: Vec<DeferredEntry>,
    main_engine: &LayoutEngine,
    window_size: Size,
    font_system: &mozui_text::FontSystem,
) -> Vec<ResolvedDeferred> {
    let available = taffy::prelude::Size {
        width: AvailableSpace::Definite(window_size.width),
        height: AvailableSpace::Definite(window_size.height),
    };

    let mut resolved = Vec::with_capacity(deferred.len());

    for entry in deferred {
        let mut sub_engine = LayoutEngine::new();
        let mut element = entry.element;

        // Layout the deferred element in its own engine
        let root_id = {
            let mut sub_deferred = Vec::new();
            let mut sub_cache = LayoutCache::new();
            let mut sub_lcx = LayoutContext::new(&mut sub_engine, font_system, &mut sub_deferred, &mut sub_cache);
            element.layout(&mut sub_lcx)
            // TODO: support nested deferred (sub_deferred)
        };

        // Solve layout
        sub_engine.compute_layout(root_id, available, font_system);

        let anchor_bounds = match &entry.position {
            DeferredPosition::Anchored {
                anchor_id,
                placement,
                gap,
            } => {
                // Get anchor bounds from the main tree
                let ab = main_engine.bounds(*anchor_id);
                let anchor_rect =
                    mozui_style::Rect::new(ab.x, ab.y, ab.width, ab.height);

                // Get content size from the sub-engine solve
                let content = sub_engine.bounds(root_id);
                let content_size = Size::new(content.width, content.height);

                // Compute position relative to anchor
                let pos = compute_anchored_position(
                    anchor_rect,
                    content_size,
                    *placement,
                    *gap,
                    window_size,
                );

                // Re-resolve bounds with the computed offset
                sub_engine.resolve_with_offset(root_id, pos.x, pos.y);

                Some(anchor_rect)
            }
            DeferredPosition::Overlay => {
                // Overlays position themselves via their own layout.
                // No offset needed — the sub-engine root fills the window.
                None
            }
        };

        resolved.push(ResolvedDeferred {
            element,
            engine: sub_engine,
            root_id,
            anchor_bounds,
        });
    }

    resolved
}

/// Paint all resolved deferred elements on top of the main tree.
fn paint_deferred(
    resolved: &mut Vec<ResolvedDeferred>,
    draw_list: &mut DrawList,
    interactions: &mut InteractionMap,
    font_system: &mozui_text::FontSystem,
    window_size: Size,
) {
    for entry in resolved.iter_mut() {
        let root_bounds = {
            let cl = entry.engine.bounds(entry.root_id);
            mozui_style::Rect::new(cl.x, cl.y, cl.width, cl.height)
        };
        let mut pcx = PaintContext::new(
            &entry.engine,
            draw_list,
            interactions,
            font_system,
            window_size,
        );
        pcx.anchor_bounds = entry.anchor_bounds;
        entry.element.prepaint(root_bounds, &mut pcx);
        entry.element.paint(root_bounds, &mut pcx);
    }
}
