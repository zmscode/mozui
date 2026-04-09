use crate::{Element, InteractionMap};
use mozui_events::CursorStyle;
use mozui_layout::LayoutEngine;
use mozui_renderer::{DrawCommand, DrawList};
use mozui_style::{Color, Corners, Fill, Point, Rect, Theme};
use mozui_text::FontSystem;
use taffy::prelude::*;

const HANDLE_SIZE: f32 = 1.0;
const HANDLE_HIT_AREA: f32 = 8.0;
const PANEL_MIN_SIZE: f32 = 100.0;

/// Axis for resizable panel layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeAxis {
    Horizontal,
    Vertical,
}

/// A single panel within a resizable group.
pub struct ResizablePanel {
    children: Vec<Box<dyn Element>>,
    initial_size: Option<f32>,
    min_size: f32,
    max_size: f32,
}

pub fn resizable_panel() -> ResizablePanel {
    ResizablePanel {
        children: Vec::new(),
        initial_size: None,
        min_size: PANEL_MIN_SIZE,
        max_size: f32::MAX,
    }
}

impl ResizablePanel {
    pub fn child(mut self, element: impl Element + 'static) -> Self {
        self.children.push(Box::new(element));
        self
    }

    pub fn children(mut self, elements: impl IntoIterator<Item = Box<dyn Element>>) -> Self {
        self.children.extend(elements);
        self
    }

    pub fn size(mut self, size: f32) -> Self {
        self.initial_size = Some(size);
        self
    }

    pub fn min_size(mut self, size: f32) -> Self {
        self.min_size = size;
        self
    }

    pub fn max_size(mut self, size: f32) -> Self {
        self.max_size = size;
        self
    }
}

/// A group of resizable panels with drag handles between them.
pub struct ResizablePanelGroup {
    axis: ResizeAxis,
    panels: Vec<ResizablePanel>,
    sizes: Vec<f32>,
    on_resize: Option<Box<dyn Fn(Vec<f32>, &mut dyn std::any::Any)>>,
    handle_color: Color,
    handle_hover_color: Color,
}

pub fn h_resizable(theme: &Theme) -> ResizablePanelGroup {
    ResizablePanelGroup {
        axis: ResizeAxis::Horizontal,
        panels: Vec::new(),
        sizes: Vec::new(),
        on_resize: None,
        handle_color: theme.border,
        handle_hover_color: theme.primary,
    }
}

pub fn v_resizable(theme: &Theme) -> ResizablePanelGroup {
    ResizablePanelGroup {
        axis: ResizeAxis::Vertical,
        panels: Vec::new(),
        sizes: Vec::new(),
        on_resize: None,
        handle_color: theme.border,
        handle_hover_color: theme.primary,
    }
}

impl ResizablePanelGroup {
    pub fn panel(mut self, panel: ResizablePanel) -> Self {
        self.panels.push(panel);
        self
    }

    pub fn sizes(mut self, sizes: Vec<f32>) -> Self {
        self.sizes = sizes;
        self
    }

    pub fn on_resize(
        mut self,
        f: impl Fn(Vec<f32>, &mut dyn std::any::Any) + 'static,
    ) -> Self {
        self.on_resize = Some(Box::new(f));
        self
    }

    fn has_valid_sizes(&self) -> bool {
        self.sizes.len() == self.panels.len() && self.sizes.iter().all(|s| *s > 0.0)
    }
}

impl Element for ResizablePanelGroup {
    fn layout(&self, engine: &mut LayoutEngine, font_system: &FontSystem) -> taffy::NodeId {
        let n = self.panels.len();
        let has_sizes = self.has_valid_sizes();
        let mut children = Vec::new();

        for i in 0..n {
            if i > 0 {
                let handle_style = match self.axis {
                    ResizeAxis::Horizontal => Style {
                        size: Size {
                            width: length(HANDLE_SIZE),
                            height: percent(1.0),
                        },
                        flex_shrink: 0.0,
                        ..Default::default()
                    },
                    ResizeAxis::Vertical => Style {
                        size: Size {
                            width: percent(1.0),
                            height: length(HANDLE_SIZE),
                        },
                        flex_shrink: 0.0,
                        ..Default::default()
                    },
                };
                children.push(engine.new_leaf(handle_style));
            }

            let panel_children: Vec<_> = self.panels[i]
                .children
                .iter()
                .map(|c| c.layout(engine, font_system))
                .collect();

            let panel = &self.panels[i];

            let (flex_basis_val, flex_grow_val) = if has_sizes {
                (length(self.sizes[i]), 0.0)
            } else if let Some(init) = panel.initial_size {
                (length(init), 0.0)
            } else {
                (auto(), 1.0)
            };

            let panel_style = match self.axis {
                ResizeAxis::Horizontal => Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    flex_basis: flex_basis_val,
                    flex_grow: flex_grow_val,
                    flex_shrink: 1.0,
                    min_size: Size {
                        width: length(panel.min_size),
                        height: auto(),
                    },
                    max_size: Size {
                        width: if panel.max_size < f32::MAX {
                            length(panel.max_size)
                        } else {
                            auto()
                        },
                        height: auto(),
                    },
                    size: Size {
                        width: auto(),
                        height: percent(1.0),
                    },
                    overflow: taffy::Point {
                        x: taffy::Overflow::Hidden,
                        y: taffy::Overflow::Hidden,
                    },
                    ..Default::default()
                },
                ResizeAxis::Vertical => Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    flex_basis: flex_basis_val,
                    flex_grow: flex_grow_val,
                    flex_shrink: 1.0,
                    min_size: Size {
                        width: auto(),
                        height: length(panel.min_size),
                    },
                    max_size: Size {
                        width: auto(),
                        height: if panel.max_size < f32::MAX {
                            length(panel.max_size)
                        } else {
                            auto()
                        },
                    },
                    size: Size {
                        width: percent(1.0),
                        height: auto(),
                    },
                    overflow: taffy::Point {
                        x: taffy::Overflow::Hidden,
                        y: taffy::Overflow::Hidden,
                    },
                    ..Default::default()
                },
            };
            children.push(engine.new_with_children(panel_style, &panel_children));
        }

        let container_style = match self.axis {
            ResizeAxis::Horizontal => Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                size: Size {
                    width: percent(1.0),
                    height: percent(1.0),
                },
                ..Default::default()
            },
            ResizeAxis::Vertical => Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                size: Size {
                    width: percent(1.0),
                    height: percent(1.0),
                },
                ..Default::default()
            },
        };
        engine.new_with_children(container_style, &children)
    }

    fn paint(
        &self,
        layouts: &[mozui_layout::ComputedLayout],
        index: &mut usize,
        draw_list: &mut DrawList,
        interactions: &mut InteractionMap,
        font_system: &FontSystem,
    ) {
        let n = self.panels.len();
        if n == 0 {
            return;
        }

        let container = layouts[*index];
        *index += 1;

        // We need actual rendered sizes for drag calculations.
        // Pre-scan: the layout array is in pre-order. For each panel, its layout
        // node is at a known position relative to handles. We peek ahead to read
        // panel layout widths/heights before painting.
        let mut peek = *index;
        let mut actual_sizes = Vec::with_capacity(n);
        for i in 0..n {
            if i > 0 {
                peek += 1; // handle leaf
            }
            let pl = layouts[peek];
            actual_sizes.push(match self.axis {
                ResizeAxis::Horizontal => pl.width,
                ResizeAxis::Vertical => pl.height,
            });
            peek += 1; // panel container node
            // We don't know how many children follow, but we don't need to skip them
            // because we only need the panel container nodes which are at predictable offsets.
        }

        for i in 0..n {
            // Handle
            if i > 0 {
                let hl = layouts[*index];
                *index += 1;

                let handle_bounds = Rect::new(hl.x, hl.y, hl.width, hl.height);
                let hit_bounds = match self.axis {
                    ResizeAxis::Horizontal => Rect::new(
                        hl.x - HANDLE_HIT_AREA / 2.0,
                        hl.y,
                        HANDLE_SIZE + HANDLE_HIT_AREA,
                        hl.height,
                    ),
                    ResizeAxis::Vertical => Rect::new(
                        hl.x,
                        hl.y - HANDLE_HIT_AREA / 2.0,
                        hl.width,
                        HANDLE_SIZE + HANDLE_HIT_AREA,
                    ),
                };

                let hovered = interactions.is_hovered(hit_bounds);
                draw_list.push(DrawCommand::Rect {
                    bounds: handle_bounds,
                    background: Fill::Solid(if hovered {
                        self.handle_hover_color
                    } else {
                        self.handle_color
                    }),
                    corner_radii: Corners::uniform(0.0),
                    border: None,
                    shadow: None,
                });

                let cursor = match self.axis {
                    ResizeAxis::Horizontal => CursorStyle::ResizeEW,
                    ResizeAxis::Vertical => CursorStyle::ResizeNS,
                };
                interactions.register_cursor_region(hit_bounds, cursor);
                interactions.register_hover_region(hit_bounds);

                if let Some(ref on_resize) = self.on_resize {
                    let ptr =
                        on_resize.as_ref() as *const dyn Fn(Vec<f32>, &mut dyn std::any::Any);
                    let panel_idx = i;
                    let axis = self.axis;
                    let sizes = actual_sizes.clone();
                    let mins: Vec<f32> = self.panels.iter().map(|p| p.min_size).collect();
                    let maxs: Vec<f32> = self.panels.iter().map(|p| p.max_size).collect();
                    let origin = match axis {
                        ResizeAxis::Horizontal => container.x,
                        ResizeAxis::Vertical => container.y,
                    };

                    interactions.register_drag_handler(
                        hit_bounds,
                        Box::new(move |pos: Point, cx| {
                            let mouse = match axis {
                                ResizeAxis::Horizontal => pos.x,
                                ResizeAxis::Vertical => pos.y,
                            };
                            let mut ns = sizes.clone();
                            let li = panel_idx - 1;

                            let mut left_origin = origin;
                            for k in 0..li {
                                left_origin += ns[k] + HANDLE_SIZE;
                            }

                            let new_left =
                                (mouse - left_origin).clamp(mins[li], maxs[li]);
                            let delta = new_left - ns[li];
                            let new_right =
                                (ns[panel_idx] - delta).clamp(mins[panel_idx], maxs[panel_idx]);
                            let actual = ns[panel_idx] - new_right;
                            ns[li] += actual;
                            ns[panel_idx] = new_right;

                            unsafe { (*ptr)(ns, cx) };
                        }),
                    );
                }
            }

            // Panel container
            let pl = layouts[*index];
            *index += 1;
            let panel_bounds = Rect::new(pl.x, pl.y, pl.width, pl.height);
            draw_list.push_clip(panel_bounds);

            for child in &self.panels[i].children {
                child.paint(layouts, index, draw_list, interactions, font_system);
            }

            draw_list.pop_clip();
        }
    }
}
