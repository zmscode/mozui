use crate::{Element, LayoutContext, PaintContext};
use mozui_icons::{IconName, IconWeight};
use mozui_layout::LayoutId;
use mozui_renderer::DrawCommand;
use mozui_style::{Color, Corners, Fill, Rect, Theme};
use taffy::prelude::*;

const INDENT: f32 = 20.0;
const ROW_HEIGHT: f32 = 28.0;
const ICON_SIZE: f32 = 16.0;
const CHEVRON_SIZE: f32 = 14.0;
const GAP: f32 = 6.0;
const FONT_SIZE: f32 = 13.0;

/// A node in a tree view.
pub struct TreeNode {
    label: String,
    icon: Option<IconName>,
    expanded: bool,
    selected: bool,
    children: Vec<TreeNode>,
    on_toggle: Option<Box<dyn Fn(&mut dyn std::any::Any)>>,
    on_click: Option<Box<dyn Fn(&mut dyn std::any::Any)>>,
}

pub fn tree_node(label: impl Into<String>) -> TreeNode {
    TreeNode {
        label: label.into(),
        icon: None,
        expanded: false,
        selected: false,
        children: Vec::new(),
        on_toggle: None,
        on_click: None,
    }
}

impl TreeNode {
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn child(mut self, node: TreeNode) -> Self {
        self.children.push(node);
        self
    }

    pub fn children(mut self, nodes: Vec<TreeNode>) -> Self {
        self.children = nodes;
        self
    }

    pub fn on_toggle(mut self, f: impl Fn(&mut dyn std::any::Any) + 'static) -> Self {
        self.on_toggle = Some(Box::new(f));
        self
    }

    pub fn on_click(mut self, f: impl Fn(&mut dyn std::any::Any) + 'static) -> Self {
        self.on_click = Some(Box::new(f));
        self
    }

    fn is_branch(&self) -> bool {
        !self.children.is_empty()
    }

    /// Count visible rows (self + expanded children recursively).
    fn visible_count(&self) -> usize {
        let mut count = 1;
        if self.expanded {
            for child in &self.children {
                count += child.visible_count();
            }
        }
        count
    }
}

/// A tree view component displaying hierarchical data.
pub struct TreeView {
    layout_id: LayoutId,
    /// Flat list of row LayoutIds in visible order
    row_ids: Vec<LayoutId>,

    roots: Vec<TreeNode>,
    width: f32,
    fg: Color,
    muted_fg: Color,
    selected_bg: Color,
    selected_fg: Color,
    hover_bg: Color,
    corner_radius: f32,
}

pub fn tree_view(theme: &Theme) -> TreeView {
    TreeView {
        layout_id: LayoutId::NONE,
        row_ids: Vec::new(),
        roots: Vec::new(),
        width: 240.0,
        fg: theme.foreground,
        muted_fg: theme.muted_foreground,
        selected_bg: theme.secondary,
        selected_fg: theme.foreground,
        hover_bg: theme.secondary_hover,
        corner_radius: theme.radius_sm,
    }
}

impl TreeView {
    pub fn root(mut self, node: TreeNode) -> Self {
        self.roots.push(node);
        self
    }

    pub fn roots(mut self, nodes: Vec<TreeNode>) -> Self {
        self.roots = nodes;
        self
    }

    pub fn width(mut self, w: f32) -> Self {
        self.width = w;
        self
    }

    fn total_visible(&self) -> usize {
        self.roots.iter().map(|r| r.visible_count()).sum()
    }
}

fn layout_node(
    node: &TreeNode,
    depth: usize,
    cx: &mut LayoutContext,
    rows: &mut Vec<LayoutId>,
) {
    let left_pad = depth as f32 * INDENT;
    let row_id = cx.new_leaf(Style {
        size: Size {
            width: percent(1.0),
            height: length(ROW_HEIGHT),
        },
        padding: taffy::Rect {
            left: length(left_pad),
            right: length(4.0),
            top: zero(),
            bottom: zero(),
        },
        ..Default::default()
    });
    rows.push(row_id);

    if node.expanded {
        for child in &node.children {
            layout_node(child, depth + 1, cx, rows);
        }
    }
}

impl Element for TreeView {
    fn debug_info(&self) -> Option<mozui_devtools::ElementInfo> {
        Some(mozui_devtools::ElementInfo {
            type_name: "TreeView",
            layout_id: self.layout_id,
            properties: vec![],
        })
    }

    fn layout(&mut self, cx: &mut LayoutContext) -> LayoutId {
        self.row_ids.clear();
        let row_count = self.total_visible();
        self.row_ids.reserve(row_count);

        for root in &self.roots {
            layout_node(root, 0, cx, &mut self.row_ids);
        }

        self.layout_id = cx.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                size: Size {
                    width: length(self.width),
                    height: auto(),
                },
                ..Default::default()
            },
            &self.row_ids,
        );
        self.layout_id
    }

    fn paint(&mut self, _bounds: Rect, cx: &mut PaintContext) {
        let mut row_idx = 0;
        for root in &self.roots {
            self.paint_node(root, 0, &mut row_idx, cx);
        }
    }
}

impl TreeView {
    fn paint_node(
        &self,
        node: &TreeNode,
        depth: usize,
        row_idx: &mut usize,
        cx: &mut PaintContext,
    ) {
        let row_bounds = cx.bounds(self.row_ids[*row_idx]);
        *row_idx += 1;

        let left_pad = depth as f32 * INDENT;
        // Content area starts after indent padding
        let content_x = row_bounds.origin.x + left_pad;
        let content_w = row_bounds.size.width - left_pad;
        let content_bounds = Rect::new(content_x, row_bounds.origin.y, content_w, row_bounds.size.height);
        let hovered = cx.interactions.is_hovered(content_bounds);

        // Row background (selected or hover) — drawn over content area only
        if node.selected {
            cx.draw_list.push(DrawCommand::Rect {
                bounds: content_bounds,
                background: Fill::Solid(self.selected_bg),
                corner_radii: Corners::uniform(self.corner_radius),
                border: None,
                shadow: None,
            });
        } else if hovered {
            cx.draw_list.push(DrawCommand::Rect {
                bounds: content_bounds,
                background: Fill::Solid(self.hover_bg),
                corner_radii: Corners::uniform(self.corner_radius),
                border: None,
                shadow: None,
            });
        }

        let fg = if node.selected {
            self.selected_fg
        } else {
            self.fg
        };
        let mut x = content_x + 4.0;
        let cy = row_bounds.origin.y + (ROW_HEIGHT - ICON_SIZE) / 2.0;

        // Chevron for branches
        if node.is_branch() {
            let chevron_icon = if node.expanded {
                IconName::CaretDown
            } else {
                IconName::CaretRight
            };
            cx.draw_list.push(DrawCommand::Icon {
                name: chevron_icon,
                weight: IconWeight::Bold,
                bounds: Rect::new(x, cy, CHEVRON_SIZE, CHEVRON_SIZE),
                color: if node.selected { fg } else { self.muted_fg },
                size_px: CHEVRON_SIZE,
            });
        }
        x += CHEVRON_SIZE + GAP;

        // Optional icon
        if let Some(icon) = node.icon {
            cx.draw_list.push(DrawCommand::Icon {
                name: icon,
                weight: IconWeight::Regular,
                bounds: Rect::new(x, cy, ICON_SIZE, ICON_SIZE),
                color: if node.selected { fg } else { self.muted_fg },
                size_px: ICON_SIZE,
            });
            x += ICON_SIZE + GAP;
        }

        // Label
        cx.draw_list.push(DrawCommand::Text {
            text: node.label.clone(),
            bounds: Rect::new(x, row_bounds.origin.y, content_w - (x - content_x), ROW_HEIGHT),
            font_size: FONT_SIZE,
            color: fg,
            weight: if node.selected { 600 } else { 400 },
            italic: false,
        });

        // Click handler — register on content area, not full row
        if node.is_branch() {
            if let Some(ref on_toggle) = node.on_toggle {
                let ptr = on_toggle.as_ref() as *const dyn Fn(&mut dyn std::any::Any);
                cx.interactions
                    .register_click(content_bounds, Box::new(move |cx| unsafe { (*ptr)(cx) }));
            }
        } else if let Some(ref on_click) = node.on_click {
            let ptr = on_click.as_ref() as *const dyn Fn(&mut dyn std::any::Any);
            cx.interactions.register_click(content_bounds, Box::new(move |cx| unsafe { (*ptr)(cx) }));
        }

        cx.interactions.register_hover_region(content_bounds);

        // Recurse into expanded children
        if node.expanded {
            for child in &node.children {
                self.paint_node(child, depth + 1, row_idx, cx);
            }
        }
    }
}
