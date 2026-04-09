use crate::{Element, LayoutContext, PaintContext};
use mozui_events::CursorStyle;
use mozui_layout::LayoutId;
use mozui_renderer::DrawCommand;
use mozui_style::{Color, Corners, Fill, Point, Rect, Theme};
use std::rc::Rc;
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
    child_ids: Vec<LayoutId>,
}

pub fn resizable_panel() -> ResizablePanel {
    ResizablePanel {
        children: Vec::new(),
        initial_size: None,
        min_size: PANEL_MIN_SIZE,
        max_size: f32::MAX,
        child_ids: Vec::new(),
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
    on_resize: Option<Rc<dyn Fn(Vec<f32>, &mut dyn std::any::Any)>>,
    handle_color: Color,
    handle_hover_color: Color,
    layout_id: LayoutId,
    handle_ids: Vec<LayoutId>,
    panel_ids: Vec<LayoutId>,
}

pub fn h_resizable(theme: &Theme) -> ResizablePanelGroup {
    ResizablePanelGroup {
        axis: ResizeAxis::Horizontal,
        panels: Vec::new(),
        sizes: Vec::new(),
        on_resize: None,
        handle_color: theme.border,
        handle_hover_color: theme.primary,
        layout_id: LayoutId::NONE,
        handle_ids: Vec::new(),
        panel_ids: Vec::new(),
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
        layout_id: LayoutId::NONE,
        handle_ids: Vec::new(),
        panel_ids: Vec::new(),
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

    pub fn on_resize(mut self, f: impl Fn(Vec<f32>, &mut dyn std::any::Any) + 'static) -> Self {
        self.on_resize = Some(Rc::new(f));
        self
    }

    fn has_valid_sizes(&self) -> bool {
        self.sizes.len() == self.panels.len() && self.sizes.iter().all(|s| *s > 0.0)
    }
}

impl Element for ResizablePanelGroup {
    fn debug_info(&self) -> Option<mozui_devtools::ElementInfo> {
        Some(mozui_devtools::ElementInfo {
            type_name: "ResizablePanelGroup",
            layout_id: self.layout_id,
            properties: vec![],
        })
    }

    fn layout(&mut self, cx: &mut LayoutContext) -> LayoutId {
        let n = self.panels.len();
        let has_sizes = self.has_valid_sizes();
        let mut children = Vec::new();
        self.handle_ids.clear();
        self.panel_ids.clear();

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
                let handle_id = cx.new_leaf(handle_style);
                self.handle_ids.push(handle_id);
                children.push(handle_id);
            }

            self.panels[i].child_ids.clear();
            for j in 0..self.panels[i].children.len() {
                let child_id = self.panels[i].children[j].layout(cx);
                self.panels[i].child_ids.push(child_id);
            }

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
            let panel_id = cx.new_with_children(panel_style, &panel.child_ids);
            self.panel_ids.push(panel_id);
            children.push(panel_id);
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
        self.layout_id = cx.new_with_children(container_style, &children);
        self.layout_id
    }

    fn paint(&mut self, bounds: Rect, cx: &mut PaintContext) {
        let n = self.panels.len();
        if n == 0 {
            return;
        }

        // Collect actual rendered sizes for drag calculations.
        let mut actual_sizes = Vec::with_capacity(n);
        for i in 0..n {
            let panel_bounds = cx.bounds(self.panel_ids[i]);
            actual_sizes.push(match self.axis {
                ResizeAxis::Horizontal => panel_bounds.size.width,
                ResizeAxis::Vertical => panel_bounds.size.height,
            });
        }

        let mut handle_idx = 0;
        for i in 0..n {
            // Handle
            if i > 0 {
                let handle_bounds = cx.bounds(self.handle_ids[handle_idx]);
                handle_idx += 1;

                let hit_bounds = match self.axis {
                    ResizeAxis::Horizontal => Rect::new(
                        handle_bounds.origin.x - HANDLE_HIT_AREA / 2.0,
                        handle_bounds.origin.y,
                        HANDLE_SIZE + HANDLE_HIT_AREA,
                        handle_bounds.size.height,
                    ),
                    ResizeAxis::Vertical => Rect::new(
                        handle_bounds.origin.x,
                        handle_bounds.origin.y - HANDLE_HIT_AREA / 2.0,
                        handle_bounds.size.width,
                        HANDLE_SIZE + HANDLE_HIT_AREA,
                    ),
                };

                let hovered = cx.interactions.is_hovered(hit_bounds);
                cx.draw_list.push(DrawCommand::Rect {
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
                cx.interactions.register_cursor_region(hit_bounds, cursor);
                cx.interactions.register_hover_region(hit_bounds);

                if let Some(ref on_resize) = self.on_resize {
                    let h = on_resize.clone();
                    let panel_idx = i;
                    let axis = self.axis;
                    let sizes = actual_sizes.clone();
                    let mins: Vec<f32> = self.panels.iter().map(|p| p.min_size).collect();
                    let maxs: Vec<f32> = self.panels.iter().map(|p| p.max_size).collect();
                    let origin = match axis {
                        ResizeAxis::Horizontal => bounds.origin.x,
                        ResizeAxis::Vertical => bounds.origin.y,
                    };

                    cx.interactions.register_drag_handler(
                        hit_bounds,
                        Rc::new(move |pos: Point, cx: &mut dyn std::any::Any| {
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

                            let new_left = (mouse - left_origin).clamp(mins[li], maxs[li]);
                            let delta = new_left - ns[li];
                            let new_right =
                                (ns[panel_idx] - delta).clamp(mins[panel_idx], maxs[panel_idx]);
                            let actual = ns[panel_idx] - new_right;
                            ns[li] += actual;
                            ns[panel_idx] = new_right;

                            h(ns, cx);
                        }),
                    );
                }
            }

            // Panel container
            let panel_bounds = cx.bounds(self.panel_ids[i]);
            cx.draw_list.push_clip(panel_bounds);

            for j in 0..self.panels[i].children.len() {
                let child_bounds = cx.bounds(self.panels[i].child_ids[j]);
                self.panels[i].children[j].paint(child_bounds, cx);
            }

            cx.draw_list.pop_clip();
        }
    }
}
