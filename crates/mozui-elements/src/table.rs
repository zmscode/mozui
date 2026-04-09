use crate::{Element, InteractionMap};
use mozui_icons::{IconName, IconWeight};
use mozui_layout::LayoutEngine;
use mozui_renderer::{Border, DrawCommand, DrawList};
use mozui_style::{Color, Corners, Fill, Theme};
use mozui_text::FontSystem;
use taffy::prelude::*;

/// Column sort direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

/// Column definition for a table.
pub struct TableColumn {
    pub key: String,
    pub label: String,
    pub width: ColumnWidth,
    pub sortable: bool,
}

/// How a column's width is determined.
#[derive(Debug, Clone, Copy)]
pub enum ColumnWidth {
    /// Fixed width in logical pixels.
    Fixed(f32),
    /// Flexible — takes remaining space proportional to weight.
    Flex(f32),
}

pub fn table_column(key: impl Into<String>, label: impl Into<String>) -> TableColumn {
    TableColumn {
        key: key.into(),
        label: label.into(),
        width: ColumnWidth::Flex(1.0),
        sortable: false,
    }
}

impl TableColumn {
    pub fn fixed(mut self, width: f32) -> Self {
        self.width = ColumnWidth::Fixed(width);
        self
    }

    pub fn flex(mut self, weight: f32) -> Self {
        self.width = ColumnWidth::Flex(weight);
        self
    }

    pub fn sortable(mut self, sortable: bool) -> Self {
        self.sortable = sortable;
        self
    }
}

/// A row of data — maps column keys to cell values.
pub struct TableRow {
    pub cells: Vec<String>,
    pub selected: bool,
}

pub fn table_row(cells: Vec<impl Into<String>>) -> TableRow {
    TableRow {
        cells: cells.into_iter().map(|c| c.into()).collect(),
        selected: false,
    }
}

impl TableRow {
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}

/// A data table with columns, rows, sorting, and row selection.
///
/// ```rust,ignore
/// table(&theme)
///     .columns(vec![
///         table_column("name", "Name").flex(2.0).sortable(true),
///         table_column("email", "Email").flex(3.0),
///         table_column("role", "Role").fixed(100.0),
///     ])
///     .rows(data.iter().map(|d| table_row(vec![&d.name, &d.email, &d.role])))
///     .sort_by("name", SortDirection::Ascending)
///     .on_sort(|key, dir, cx| { /* handle sort change */ })
///     .on_row_click(|row_index, cx| { /* handle row click */ })
/// ```
pub struct Table {
    columns: Vec<TableColumn>,
    rows: Vec<TableRow>,
    sort_key: Option<String>,
    sort_dir: SortDirection,
    on_sort: Option<Box<dyn Fn(&str, SortDirection, &mut dyn std::any::Any)>>,
    on_row_click: Option<Box<dyn Fn(usize, &mut dyn std::any::Any)>>,
    striped: bool,
    // Theme colors
    header_bg: Color,
    header_fg: Color,
    row_bg: Color,
    row_alt_bg: Color,
    row_hover_bg: Color,
    row_selected_bg: Color,
    fg: Color,
    border_color: Color,
    corner_radius: f32,
    font_size: f32,
    header_font_size: f32,
    row_height: f32,
    header_height: f32,
    cell_px: f32,
}

pub fn table(theme: &Theme) -> Table {
    Table {
        columns: Vec::new(),
        rows: Vec::new(),
        sort_key: None,
        sort_dir: SortDirection::Ascending,
        on_sort: None,
        on_row_click: None,
        striped: true,
        header_bg: theme.secondary,
        header_fg: theme.foreground,
        row_bg: Color::TRANSPARENT,
        row_alt_bg: theme.secondary.with_alpha(0.3),
        row_hover_bg: theme.secondary,
        row_selected_bg: theme.primary.with_alpha(0.15),
        fg: theme.foreground,
        border_color: theme.border,
        corner_radius: theme.radius_md,
        font_size: theme.font_size_sm,
        header_font_size: theme.font_size_xs,
        row_height: 40.0,
        header_height: 36.0,
        cell_px: 12.0,
    }
}

impl Table {
    pub fn columns(mut self, columns: Vec<TableColumn>) -> Self {
        self.columns = columns;
        self
    }

    pub fn rows(mut self, rows: impl IntoIterator<Item = TableRow>) -> Self {
        self.rows = rows.into_iter().collect();
        self
    }

    pub fn sort_by(mut self, key: impl Into<String>, dir: SortDirection) -> Self {
        self.sort_key = Some(key.into());
        self.sort_dir = dir;
        self
    }

    pub fn on_sort(
        mut self,
        handler: impl Fn(&str, SortDirection, &mut dyn std::any::Any) + 'static,
    ) -> Self {
        self.on_sort = Some(Box::new(handler));
        self
    }

    pub fn on_row_click(
        mut self,
        handler: impl Fn(usize, &mut dyn std::any::Any) + 'static,
    ) -> Self {
        self.on_row_click = Some(Box::new(handler));
        self
    }

    pub fn striped(mut self, striped: bool) -> Self {
        self.striped = striped;
        self
    }

    pub fn row_height(mut self, height: f32) -> Self {
        self.row_height = height;
        self
    }

    /// Resolve column widths given the total available width.
    #[allow(dead_code)]
    fn resolve_widths(&self, total_width: f32) -> Vec<f32> {
        let fixed_total: f32 = self
            .columns
            .iter()
            .filter_map(|c| match c.width {
                ColumnWidth::Fixed(w) => Some(w),
                _ => None,
            })
            .sum();
        let flex_total: f32 = self
            .columns
            .iter()
            .filter_map(|c| match c.width {
                ColumnWidth::Flex(w) => Some(w),
                _ => None,
            })
            .sum();

        let remaining = (total_width - fixed_total - self.cell_px * 2.0 * self.columns.len() as f32).max(0.0);

        self.columns
            .iter()
            .map(|c| match c.width {
                ColumnWidth::Fixed(w) => w,
                ColumnWidth::Flex(weight) => {
                    if flex_total > 0.0 {
                        remaining * weight / flex_total
                    } else {
                        remaining / self.columns.len() as f32
                    }
                }
            })
            .collect()
    }
}

impl Element for Table {
    fn layout(&self, engine: &mut LayoutEngine, _font_system: &FontSystem) -> taffy::NodeId {
        // We use a simple approach: one row node per row (header + data rows)
        // Each row is a flex row with cells sized according to column widths.
        // Since we don't know the total width at layout time, use percentage-based sizing.

        let mut row_nodes = Vec::new();

        // Header row
        let mut header_cells = Vec::new();
        for col in &self.columns {
            let cell_style = match col.width {
                ColumnWidth::Fixed(w) => Style {
                    size: taffy::Size {
                        width: length(w + self.cell_px * 2.0),
                        height: length(self.header_height),
                    },
                    padding: taffy::Rect {
                        left: length(self.cell_px),
                        right: length(self.cell_px),
                        top: zero(),
                        bottom: zero(),
                    },
                    ..Default::default()
                },
                ColumnWidth::Flex(weight) => Style {
                    flex_grow: weight,
                    flex_shrink: 1.0,
                    size: taffy::Size {
                        width: auto(),
                        height: length(self.header_height),
                    },
                    padding: taffy::Rect {
                        left: length(self.cell_px),
                        right: length(self.cell_px),
                        top: zero(),
                        bottom: zero(),
                    },
                    ..Default::default()
                },
            };
            header_cells.push(engine.new_leaf(cell_style));
        }
        row_nodes.push(engine.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::Center),
                ..Default::default()
            },
            &header_cells,
        ));

        // Data rows
        for _row in &self.rows {
            let mut cells = Vec::new();
            for col in &self.columns {
                let cell_style = match col.width {
                    ColumnWidth::Fixed(w) => Style {
                        size: taffy::Size {
                            width: length(w + self.cell_px * 2.0),
                            height: length(self.row_height),
                        },
                        padding: taffy::Rect {
                            left: length(self.cell_px),
                            right: length(self.cell_px),
                            top: zero(),
                            bottom: zero(),
                        },
                        ..Default::default()
                    },
                    ColumnWidth::Flex(weight) => Style {
                        flex_grow: weight,
                        flex_shrink: 1.0,
                        size: taffy::Size {
                            width: auto(),
                            height: length(self.row_height),
                        },
                        padding: taffy::Rect {
                            left: length(self.cell_px),
                            right: length(self.cell_px),
                            top: zero(),
                            bottom: zero(),
                        },
                        ..Default::default()
                    },
                };
                cells.push(engine.new_leaf(cell_style));
            }
            row_nodes.push(engine.new_with_children(
                Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: Some(AlignItems::Center),
                    ..Default::default()
                },
                &cells,
            ));
        }

        engine.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            &row_nodes,
        )
    }

    fn paint(
        &self,
        layouts: &[mozui_layout::ComputedLayout],
        index: &mut usize,
        draw_list: &mut DrawList,
        interactions: &mut InteractionMap,
        _font_system: &FontSystem,
    ) {
        // Outer container
        let outer = layouts[*index];
        *index += 1;
        let outer_bounds =
            mozui_style::Rect::new(outer.x, outer.y, outer.width, outer.height);

        // Table border/background
        draw_list.push(DrawCommand::Rect {
            bounds: outer_bounds,
            background: Fill::Solid(Color::TRANSPARENT),
            corner_radii: Corners::uniform(self.corner_radius),
            border: Some(Border {
                width: 1.0,
                color: self.border_color,
            }),
            shadow: None,
        });

        // ── Header row ──
        let header_row = layouts[*index];
        *index += 1;
        let header_bounds = mozui_style::Rect::new(
            header_row.x,
            header_row.y,
            header_row.width,
            header_row.height,
        );

        // Header background
        draw_list.push(DrawCommand::Rect {
            bounds: header_bounds,
            background: Fill::Solid(self.header_bg),
            corner_radii: Corners {
                top_left: self.corner_radius,
                top_right: self.corner_radius,
                bottom_left: 0.0,
                bottom_right: 0.0,
            },
            border: None,
            shadow: None,
        });

        // Header cells
        for col in self.columns.iter() {
            let cell = layouts[*index];
            *index += 1;
            let cell_bounds =
                mozui_style::Rect::new(cell.x, cell.y, cell.width, cell.height);

            let is_sorted = self
                .sort_key
                .as_ref()
                .map_or(false, |k| *k == col.key);

            // Header label
            let label_weight = if is_sorted { 700 } else { 600 };
            draw_list.push(DrawCommand::Text {
                text: col.label.clone(),
                bounds: mozui_style::Rect::new(
                    cell.x + self.cell_px,
                    cell.y,
                    cell.width - self.cell_px * 2.0 - if col.sortable { 16.0 } else { 0.0 },
                    cell.height,
                ),
                font_size: self.header_font_size,
                color: self.header_fg,
                weight: label_weight,
                italic: false,
            });

            // Sort indicator
            if col.sortable && is_sorted {
                let icon = match self.sort_dir {
                    SortDirection::Ascending => IconName::ArrowUp,
                    SortDirection::Descending => IconName::ArrowDown,
                };
                draw_list.push(DrawCommand::Icon {
                    name: icon,
                    weight: IconWeight::Bold,
                    bounds: mozui_style::Rect::new(
                        cell.x + cell.width - self.cell_px - 12.0,
                        cell.y + (cell.height - 12.0) / 2.0,
                        12.0,
                        12.0,
                    ),
                    color: self.header_fg,
                    size_px: 12.0,
                });
            }

            // Sort click handler
            if col.sortable {
                if let Some(ref handler) = self.on_sort {
                    let key = col.key.clone();
                    let current_dir = if is_sorted {
                        self.sort_dir
                    } else {
                        SortDirection::Ascending
                    };
                    let next_dir = if is_sorted {
                        match current_dir {
                            SortDirection::Ascending => SortDirection::Descending,
                            SortDirection::Descending => SortDirection::Ascending,
                        }
                    } else {
                        SortDirection::Ascending
                    };
                    let handler_ptr = handler.as_ref()
                        as *const dyn Fn(&str, SortDirection, &mut dyn std::any::Any);
                    interactions.register_click(
                        cell_bounds,
                        Box::new(move |cx| unsafe {
                            (*handler_ptr)(&key, next_dir, cx);
                        }),
                    );
                }
            }
        }

        // Header bottom border
        draw_list.push(DrawCommand::Rect {
            bounds: mozui_style::Rect::new(
                header_row.x,
                header_row.y + header_row.height - 1.0,
                header_row.width,
                1.0,
            ),
            background: Fill::Solid(self.border_color),
            corner_radii: Corners::ZERO,
            border: None,
            shadow: None,
        });

        // ── Data rows ──
        for (row_idx, row) in self.rows.iter().enumerate() {
            let row_layout = layouts[*index];
            *index += 1;
            let row_bounds = mozui_style::Rect::new(
                row_layout.x,
                row_layout.y,
                row_layout.width,
                row_layout.height,
            );

            let hovered = interactions.is_hovered(row_bounds);
            let is_last = row_idx == self.rows.len() - 1;

            // Row background
            let bg = if row.selected {
                self.row_selected_bg
            } else if hovered {
                self.row_hover_bg
            } else if self.striped && row_idx % 2 == 1 {
                self.row_alt_bg
            } else {
                self.row_bg
            };

            if bg.a > 0.0 {
                let corners = if is_last {
                    Corners {
                        top_left: 0.0,
                        top_right: 0.0,
                        bottom_left: self.corner_radius,
                        bottom_right: self.corner_radius,
                    }
                } else {
                    Corners::ZERO
                };
                draw_list.push(DrawCommand::Rect {
                    bounds: row_bounds,
                    background: Fill::Solid(bg),
                    corner_radii: corners,
                    border: None,
                    shadow: None,
                });
            }

            // Row cells
            for col_idx in 0..self.columns.len() {
                let cell = layouts[*index];
                *index += 1;

                let text = row.cells.get(col_idx).cloned().unwrap_or_default();
                draw_list.push(DrawCommand::Text {
                    text,
                    bounds: mozui_style::Rect::new(
                        cell.x + self.cell_px,
                        cell.y,
                        cell.width - self.cell_px * 2.0,
                        cell.height,
                    ),
                    font_size: self.font_size,
                    color: self.fg,
                    weight: 400,
                    italic: false,
                });
            }

            // Row separator
            if !is_last {
                draw_list.push(DrawCommand::Rect {
                    bounds: mozui_style::Rect::new(
                        row_layout.x,
                        row_layout.y + row_layout.height - 0.5,
                        row_layout.width,
                        0.5,
                    ),
                    background: Fill::Solid(self.border_color.with_alpha(0.5)),
                    corner_radii: Corners::ZERO,
                    border: None,
                    shadow: None,
                });
            }

            // Row click handler
            if let Some(ref handler) = self.on_row_click {
                let handler_ptr =
                    handler.as_ref() as *const dyn Fn(usize, &mut dyn std::any::Any);
                let idx = row_idx;
                interactions.register_click(
                    row_bounds,
                    Box::new(move |cx| unsafe {
                        (*handler_ptr)(idx, cx);
                    }),
                );
            }
        }
    }
}
