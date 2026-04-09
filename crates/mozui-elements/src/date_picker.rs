use crate::{Element, InteractionMap};
use mozui_icons::{IconName, IconWeight};
use mozui_layout::LayoutEngine;
use mozui_renderer::{Border, DrawCommand, DrawList};
use mozui_style::{Color, Corners, Fill, Theme};
use mozui_text::FontSystem;
use taffy::prelude::*;
use time::{Date, Month};

// ── Public types ─────────────────────────────────────────────────

/// What the user has selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateSelection {
    Single(Option<Date>),
    Range(Option<Date>, Option<Date>),
}

impl DateSelection {
    pub fn single(&self) -> Option<Date> {
        match self {
            Self::Single(d) => *d,
            Self::Range(d, _) => *d,
        }
    }

    fn format(&self) -> String {
        match self {
            Self::Single(Some(d)) => format_date(*d),
            Self::Range(Some(a), Some(b)) => {
                format!("{} – {}", format_date(*a), format_date(*b))
            }
            Self::Range(Some(a), None) => format!("{}  –", format_date(*a)),
            _ => String::new(),
        }
    }

    fn is_active(&self, date: Date) -> bool {
        match self {
            Self::Single(Some(d)) => *d == date,
            Self::Range(Some(a), Some(b)) => *a == date || *b == date,
            Self::Range(Some(a), None) => *a == date,
            _ => false,
        }
    }

    fn is_in_range(&self, date: Date) -> bool {
        match self {
            Self::Range(Some(a), Some(b)) => {
                let (start, end) = if a <= b { (a, b) } else { (b, a) };
                date > *start && date < *end
            }
            _ => false,
        }
    }
}

fn format_date(d: Date) -> String {
    format!("{:04}/{:02}/{:02}", d.year(), d.month() as u8, d.day())
}

/// Disabled date matching.
pub enum DisabledMatcher {
    /// Disable specific weekdays (0 = Monday .. 6 = Sunday).
    Weekdays(Vec<u8>),
    /// Disable dates before this date.
    Before(Date),
    /// Disable dates after this date.
    After(Date),
    /// Disable dates outside this range (inclusive).
    Range(Date, Date),
    /// Custom predicate.
    Custom(Box<dyn Fn(Date) -> bool>),
}

impl DisabledMatcher {
    fn is_disabled(&self, date: Date) -> bool {
        match self {
            Self::Weekdays(days) => days.contains(&(date.weekday().number_days_from_monday())),
            Self::Before(d) => date < *d,
            Self::After(d) => date > *d,
            Self::Range(a, b) => date < *a || date > *b,
            Self::Custom(f) => f(date),
        }
    }
}

// ── Calendar ─────────────────────────────────────────────────────

const CELL_SIZE: f32 = 36.0;
const HEADER_HEIGHT: f32 = 40.0;
const NAV_ICON_SIZE: f32 = 16.0;
const GRID_COLS: usize = 7;
const GRID_ROWS: usize = 6;
const GAP: f32 = 2.0;
const PAD: f32 = 16.0;
const WEEKDAY_FONT_SIZE: f32 = 11.0;
const DAY_FONT_SIZE: f32 = 13.0;
/// Day cells use half-size radius for a circular/pill shape.
const DAY_RADIUS: f32 = CELL_SIZE / 2.0;

static WEEKDAY_LABELS: [&str; 7] = ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"];

/// A standalone calendar grid component.
///
/// Shows a month grid with navigation, day selection, and optional disabled dates.
/// Can be used standalone or inside a `DatePicker`.
pub struct Calendar {
    selection: DateSelection,
    view_year: i32,
    view_month: Month,
    today: Option<Date>,
    disabled: Vec<DisabledMatcher>,
    on_select: Option<Box<dyn Fn(Date, &mut dyn std::any::Any)>>,
    on_nav: Option<Box<dyn Fn(i32, Month, &mut dyn std::any::Any)>>,
    // Theme colors
    bg: Color,
    fg: Color,
    muted_fg: Color,
    primary: Color,
    primary_fg: Color,
    border_color: Color,
    hover_bg: Color,
    corner_radius: f32,
    font_size: f32,
}

pub fn calendar(theme: &Theme) -> Calendar {
    let now = time::OffsetDateTime::now_utc();
    Calendar {
        selection: DateSelection::Single(None),
        view_year: now.year(),
        view_month: now.month(),
        today: Date::from_calendar_date(now.year(), now.month(), now.day()).ok(),
        disabled: Vec::new(),
        on_select: None,
        on_nav: None,
        bg: theme.popover,
        fg: theme.foreground,
        muted_fg: theme.muted_foreground,
        primary: theme.primary,
        primary_fg: theme.primary_foreground,
        border_color: theme.border,
        hover_bg: theme.secondary,
        corner_radius: theme.radius_md,
        font_size: DAY_FONT_SIZE,
    }
}

impl Calendar {
    pub fn selection(mut self, sel: DateSelection) -> Self {
        self.selection = sel;
        self
    }

    pub fn view(mut self, year: i32, month: Month) -> Self {
        self.view_year = year;
        self.view_month = month;
        self
    }

    pub fn today(mut self, date: Date) -> Self {
        self.today = Some(date);
        self
    }

    pub fn disabled(mut self, matcher: DisabledMatcher) -> Self {
        self.disabled.push(matcher);
        self
    }

    pub fn on_select(mut self, f: impl Fn(Date, &mut dyn std::any::Any) + 'static) -> Self {
        self.on_select = Some(Box::new(f));
        self
    }

    /// Called when prev/next month buttons are clicked. Args: (year, month, cx).
    pub fn on_nav(mut self, f: impl Fn(i32, Month, &mut dyn std::any::Any) + 'static) -> Self {
        self.on_nav = Some(Box::new(f));
        self
    }

    fn is_disabled(&self, date: Date) -> bool {
        self.disabled.iter().any(|m| m.is_disabled(date))
    }

    /// Get the 42 day cells (6 weeks × 7 days) for the current view.
    fn grid_dates(&self) -> Vec<Date> {
        let first = Date::from_calendar_date(self.view_year, self.view_month, 1).unwrap();
        // Monday = 0, Sunday = 6
        let offset = first.weekday().number_days_from_monday() as i64;
        let start = first - time::Duration::days(offset);
        (0..42).map(|i| start + time::Duration::days(i)).collect()
    }

    fn prev_month(&self) -> (i32, Month) {
        if self.view_month == Month::January {
            (self.view_year - 1, Month::December)
        } else {
            (self.view_year, self.view_month.previous())
        }
    }

    fn next_month(&self) -> (i32, Month) {
        if self.view_month == Month::December {
            (self.view_year + 1, Month::January)
        } else {
            (self.view_year, self.view_month.next())
        }
    }

    fn month_name(&self) -> &'static str {
        match self.view_month {
            Month::January => "January",
            Month::February => "February",
            Month::March => "March",
            Month::April => "April",
            Month::May => "May",
            Month::June => "June",
            Month::July => "July",
            Month::August => "August",
            Month::September => "September",
            Month::October => "October",
            Month::November => "November",
            Month::December => "December",
        }
    }
}

// ── Calendar layout + paint ──────────────────────────────────────

/// Measure text and return bounds centered within the given cell rect.
fn centered_text_bounds(
    text: &str,
    font_size: f32,
    cell: mozui_style::Rect,
    font_system: &FontSystem,
) -> mozui_style::Rect {
    let style = mozui_text::TextStyle {
        font_size,
        ..Default::default()
    };
    let measured = mozui_text::measure_text(text, &style, None, font_system);
    let x = cell.origin.x + (cell.size.width - measured.width) / 2.0;
    mozui_style::Rect::new(x, cell.origin.y, measured.width, cell.size.height)
}

impl Element for Calendar {
    fn layout(&self, engine: &mut LayoutEngine, _font_system: &FontSystem) -> taffy::NodeId {
        let grid_width = CELL_SIZE * GRID_COLS as f32 + GAP * (GRID_COLS - 1) as f32;
        let total_width = grid_width + PAD * 2.0;

        // Navigation header: [<] [Month Year] [>]
        let prev_btn = engine.new_leaf(Style {
            size: Size {
                width: length(HEADER_HEIGHT),
                height: length(HEADER_HEIGHT),
            },
            ..Default::default()
        });
        let title = engine.new_leaf(Style {
            flex_grow: 1.0,
            size: Size {
                width: auto(),
                height: length(HEADER_HEIGHT),
            },
            ..Default::default()
        });
        let next_btn = engine.new_leaf(Style {
            size: Size {
                width: length(HEADER_HEIGHT),
                height: length(HEADER_HEIGHT),
            },
            ..Default::default()
        });
        let header = engine.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::Center),
                size: Size {
                    width: length(total_width),
                    height: auto(),
                },
                ..Default::default()
            },
            &[prev_btn, title, next_btn],
        );

        // Weekday labels row
        let mut weekday_cells = Vec::new();
        for _ in 0..GRID_COLS {
            weekday_cells.push(engine.new_leaf(Style {
                size: Size {
                    width: length(CELL_SIZE),
                    height: length(CELL_SIZE * 0.75),
                },
                ..Default::default()
            }));
        }
        let weekday_row = engine.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                gap: Size {
                    width: length(GAP),
                    height: zero(),
                },
                padding: taffy::Rect {
                    left: length(PAD),
                    right: length(PAD),
                    top: zero(),
                    bottom: zero(),
                },
                ..Default::default()
            },
            &weekday_cells,
        );

        // Day grid: 6 rows × 7 cells
        let mut rows = Vec::new();
        for _ in 0..GRID_ROWS {
            let mut cells = Vec::new();
            for _ in 0..GRID_COLS {
                cells.push(engine.new_leaf(Style {
                    size: Size {
                        width: length(CELL_SIZE),
                        height: length(CELL_SIZE),
                    },
                    ..Default::default()
                }));
            }
            rows.push(engine.new_with_children(
                Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    gap: Size {
                        width: length(GAP),
                        height: zero(),
                    },
                    padding: taffy::Rect {
                        left: length(PAD),
                        right: length(PAD),
                        top: zero(),
                        bottom: zero(),
                    },
                    ..Default::default()
                },
                &cells,
            ));
        }

        let mut children = vec![header, weekday_row];
        children.extend(rows);

        engine.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                gap: Size {
                    width: zero(),
                    height: length(GAP),
                },
                padding: taffy::Rect {
                    left: zero(),
                    right: zero(),
                    top: length(PAD),
                    bottom: length(PAD),
                },
                ..Default::default()
            },
            &children,
        )
    }

    fn paint(
        &self,
        layouts: &[mozui_layout::ComputedLayout],
        index: &mut usize,
        draw_list: &mut DrawList,
        interactions: &mut InteractionMap,
        font_system: &FontSystem,
    ) {
        let dates = self.grid_dates();

        // Container
        let container = layouts[*index];
        *index += 1;
        let container_bounds =
            mozui_style::Rect::new(container.x, container.y, container.width, container.height);
        draw_list.push(DrawCommand::Rect {
            bounds: container_bounds,
            background: Fill::Solid(self.bg),
            corner_radii: Corners::uniform(self.corner_radius),
            border: Some(Border {
                width: 1.0,
                color: self.border_color,
            }),
            shadow: None,
        });

        // ── Header ───────────────────────────────────────────────
        let _header = layouts[*index];
        *index += 1;

        // Prev button
        let prev_layout = layouts[*index];
        *index += 1;
        let prev_bounds = mozui_style::Rect::new(
            prev_layout.x,
            prev_layout.y,
            prev_layout.width,
            prev_layout.height,
        );
        let prev_hovered = interactions.is_hovered(prev_bounds);
        if prev_hovered {
            draw_list.push(DrawCommand::Rect {
                bounds: prev_bounds,
                background: Fill::Solid(self.hover_bg),
                corner_radii: Corners::uniform(HEADER_HEIGHT / 2.0),
                border: None,
                shadow: None,
            });
        }
        draw_list.push(DrawCommand::Icon {
            name: IconName::CaretLeft,
            weight: IconWeight::Bold,
            bounds: mozui_style::Rect::new(
                prev_layout.x + (HEADER_HEIGHT - NAV_ICON_SIZE) / 2.0,
                prev_layout.y + (HEADER_HEIGHT - NAV_ICON_SIZE) / 2.0,
                NAV_ICON_SIZE,
                NAV_ICON_SIZE,
            ),
            color: self.fg,
            size_px: NAV_ICON_SIZE,
        });
        if let Some(ref nav) = self.on_nav {
            let (y, m) = self.prev_month();
            let nav_ptr = nav.as_ref() as *const dyn Fn(i32, Month, &mut dyn std::any::Any);
            interactions.register_click(
                prev_bounds,
                Box::new(move |cx| unsafe { (*nav_ptr)(y, m, cx) }),
            );
        }

        // Title (Month Year) — centered
        let title_layout = layouts[*index];
        *index += 1;
        let title_text = format!("{} {}", self.month_name(), self.view_year);
        let title_rect = mozui_style::Rect::new(
            title_layout.x,
            title_layout.y,
            title_layout.width,
            title_layout.height,
        );
        let title_bounds = centered_text_bounds(&title_text, 14.0, title_rect, font_system);
        draw_list.push(DrawCommand::Text {
            text: title_text,
            bounds: title_bounds,
            font_size: 14.0,
            color: self.fg,
            weight: 600,
            italic: false,
        });

        // Next button
        let next_layout = layouts[*index];
        *index += 1;
        let next_bounds = mozui_style::Rect::new(
            next_layout.x,
            next_layout.y,
            next_layout.width,
            next_layout.height,
        );
        let next_hovered = interactions.is_hovered(next_bounds);
        if next_hovered {
            draw_list.push(DrawCommand::Rect {
                bounds: next_bounds,
                background: Fill::Solid(self.hover_bg),
                corner_radii: Corners::uniform(HEADER_HEIGHT / 2.0),
                border: None,
                shadow: None,
            });
        }
        draw_list.push(DrawCommand::Icon {
            name: IconName::CaretRight,
            weight: IconWeight::Bold,
            bounds: mozui_style::Rect::new(
                next_layout.x + (HEADER_HEIGHT - NAV_ICON_SIZE) / 2.0,
                next_layout.y + (HEADER_HEIGHT - NAV_ICON_SIZE) / 2.0,
                NAV_ICON_SIZE,
                NAV_ICON_SIZE,
            ),
            color: self.fg,
            size_px: NAV_ICON_SIZE,
        });
        if let Some(ref nav) = self.on_nav {
            let (y, m) = self.next_month();
            let nav_ptr = nav.as_ref() as *const dyn Fn(i32, Month, &mut dyn std::any::Any);
            interactions.register_click(
                next_bounds,
                Box::new(move |cx| unsafe { (*nav_ptr)(y, m, cx) }),
            );
        }

        // Register hover regions for nav buttons
        interactions.register_hover_region(prev_bounds);
        interactions.register_hover_region(next_bounds);

        // ── Weekday labels ───────────────────────────────────────
        let _weekday_row = layouts[*index];
        *index += 1;
        for i in 0..GRID_COLS {
            let cell = layouts[*index];
            *index += 1;
            let cell_rect = mozui_style::Rect::new(cell.x, cell.y, cell.width, cell.height);
            let text_bounds =
                centered_text_bounds(WEEKDAY_LABELS[i], WEEKDAY_FONT_SIZE, cell_rect, font_system);
            draw_list.push(DrawCommand::Text {
                text: WEEKDAY_LABELS[i].to_string(),
                bounds: text_bounds,
                font_size: WEEKDAY_FONT_SIZE,
                color: self.muted_fg,
                weight: 500,
                italic: false,
            });
        }

        // ── Day cells ────────────────────────────────────────────
        for row in 0..GRID_ROWS {
            let _row_layout = layouts[*index];
            *index += 1;

            for col in 0..GRID_COLS {
                let cell = layouts[*index];
                *index += 1;

                let day_idx = row * GRID_COLS + col;
                let date = dates[day_idx];
                let in_current_month = date.month() == self.view_month;
                let is_today = self.today == Some(date);
                let is_active = self.selection.is_active(date);
                let is_in_range = self.selection.is_in_range(date);
                let is_disabled = self.is_disabled(date);

                let cell_bounds = mozui_style::Rect::new(cell.x, cell.y, cell.width, cell.height);
                let hovered =
                    !is_disabled && in_current_month && interactions.is_hovered(cell_bounds);

                // Background
                if is_active {
                    draw_list.push(DrawCommand::Rect {
                        bounds: cell_bounds,
                        background: Fill::Solid(self.primary),
                        corner_radii: Corners::uniform(DAY_RADIUS),
                        border: None,
                        shadow: None,
                    });
                } else if is_in_range {
                    draw_list.push(DrawCommand::Rect {
                        bounds: cell_bounds,
                        background: Fill::Solid(self.primary.with_alpha(0.15)),
                        corner_radii: Corners::uniform(DAY_RADIUS),
                        border: None,
                        shadow: None,
                    });
                } else if hovered {
                    draw_list.push(DrawCommand::Rect {
                        bounds: cell_bounds,
                        background: Fill::Solid(self.hover_bg),
                        corner_radii: Corners::uniform(DAY_RADIUS),
                        border: None,
                        shadow: None,
                    });
                } else if is_today && !is_active {
                    draw_list.push(DrawCommand::Rect {
                        bounds: cell_bounds,
                        background: Fill::Solid(Color::TRANSPARENT),
                        corner_radii: Corners::uniform(DAY_RADIUS),
                        border: Some(Border {
                            width: 1.0,
                            color: self.primary,
                        }),
                        shadow: None,
                    });
                }

                // Text color
                let text_color = if is_active {
                    self.primary_fg
                } else if is_disabled {
                    self.muted_fg.with_alpha(0.3)
                } else if !in_current_month {
                    self.muted_fg.with_alpha(0.4)
                } else {
                    self.fg
                };

                // Day number (centered in cell)
                let day_text = date.day().to_string();
                let day_bounds =
                    centered_text_bounds(&day_text, self.font_size, cell_bounds, font_system);
                draw_list.push(DrawCommand::Text {
                    text: day_text,
                    bounds: day_bounds,
                    font_size: self.font_size,
                    color: text_color,
                    weight: if is_active || is_today { 600 } else { 400 },
                    italic: false,
                });

                // Click handler
                if !is_disabled && in_current_month {
                    if let Some(ref on_select) = self.on_select {
                        let ptr = on_select.as_ref() as *const dyn Fn(Date, &mut dyn std::any::Any);
                        interactions.register_click(
                            cell_bounds,
                            Box::new(move |cx| unsafe { (*ptr)(date, cx) }),
                        );
                    }
                    interactions.register_hover_region(cell_bounds);
                }
            }
        }
    }
}

// ── DatePicker (trigger + calendar dropdown) ─────────────────────

const TRIGGER_HEIGHT: f32 = 34.0;
const TRIGGER_PX: f32 = 12.0;
const ICON_SIZE: f32 = 16.0;
const DROPDOWN_GAP: f32 = 4.0;

/// A date picker input with a calendar dropdown.
///
/// ```rust,ignore
/// date_picker(&theme)
///     .selection(DateSelection::Single(Some(selected_date)))
///     .placeholder("Pick a date")
///     .open(is_open)
///     .on_select(move |date, cx| { /* update signal */ })
///     .on_toggle(move |cx| { /* toggle open state */ })
///     .on_nav(move |year, month, cx| { /* update view month/year signals */ })
/// ```
pub struct DatePicker {
    calendar: Calendar,
    placeholder: String,
    open: bool,
    on_toggle: Option<Box<dyn Fn(&mut dyn std::any::Any)>>,
    // Theme colors (trigger)
    trigger_bg: Color,
    trigger_fg: Color,
    trigger_muted_fg: Color,
    trigger_border: Color,
    trigger_hover_bg: Color,
    trigger_focus_border: Color,
    corner_radius: f32,
    font_size: f32,
    min_width: f32,
}

pub fn date_picker(theme: &Theme) -> DatePicker {
    DatePicker {
        calendar: calendar(theme),
        placeholder: "Select date…".into(),
        open: false,
        on_toggle: None,
        trigger_bg: theme.background,
        trigger_fg: theme.foreground,
        trigger_muted_fg: theme.muted_foreground,
        trigger_border: theme.border,
        trigger_hover_bg: theme.secondary,
        trigger_focus_border: theme.primary,
        corner_radius: theme.radius_md,
        font_size: theme.font_size_sm,
        min_width: 200.0,
    }
}

impl DatePicker {
    pub fn selection(mut self, sel: DateSelection) -> Self {
        self.calendar.selection = sel;
        self
    }

    pub fn view(mut self, year: i32, month: Month) -> Self {
        self.calendar.view_year = year;
        self.calendar.view_month = month;
        self
    }

    pub fn today(mut self, date: Date) -> Self {
        self.calendar.today = Some(date);
        self
    }

    pub fn disabled(mut self, matcher: DisabledMatcher) -> Self {
        self.calendar.disabled.push(matcher);
        self
    }

    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = text.into();
        self
    }

    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    pub fn min_width(mut self, w: f32) -> Self {
        self.min_width = w;
        self
    }

    pub fn on_select(mut self, f: impl Fn(Date, &mut dyn std::any::Any) + 'static) -> Self {
        self.calendar.on_select = Some(Box::new(f));
        self
    }

    pub fn on_nav(mut self, f: impl Fn(i32, Month, &mut dyn std::any::Any) + 'static) -> Self {
        self.calendar.on_nav = Some(Box::new(f));
        self
    }

    pub fn on_toggle(mut self, f: impl Fn(&mut dyn std::any::Any) + 'static) -> Self {
        self.on_toggle = Some(Box::new(f));
        self
    }

    fn display_text(&self) -> String {
        let text = self.calendar.selection.format();
        if text.is_empty() {
            self.placeholder.clone()
        } else {
            text
        }
    }

    fn has_value(&self) -> bool {
        !self.calendar.selection.format().is_empty()
    }
}

impl Element for DatePicker {
    fn layout(&self, engine: &mut LayoutEngine, font_system: &FontSystem) -> taffy::NodeId {
        let display_text = self.display_text();
        let text_style = mozui_text::TextStyle {
            font_size: self.font_size,
            color: self.trigger_fg,
            ..Default::default()
        };
        let measured = mozui_text::measure_text(&display_text, &text_style, None, font_system);

        // Trigger: [calendar icon] [text] [chevron]
        let icon_node = engine.new_leaf(Style {
            size: Size {
                width: length(ICON_SIZE),
                height: length(ICON_SIZE),
            },
            ..Default::default()
        });
        let text_node = engine.new_leaf(Style {
            size: Size {
                width: length(measured.width),
                height: length(measured.height),
            },
            flex_grow: 1.0,
            ..Default::default()
        });
        let chevron_node = engine.new_leaf(Style {
            size: Size {
                width: length(ICON_SIZE),
                height: length(ICON_SIZE),
            },
            ..Default::default()
        });
        let trigger = engine.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::Center),
                min_size: Size {
                    width: length(self.min_width),
                    height: length(TRIGGER_HEIGHT),
                },
                padding: taffy::Rect {
                    left: length(TRIGGER_PX),
                    right: length(TRIGGER_PX),
                    top: zero(),
                    bottom: zero(),
                },
                gap: Size {
                    width: length(8.0),
                    height: zero(),
                },
                ..Default::default()
            },
            &[icon_node, text_node, chevron_node],
        );

        if !self.open {
            return trigger;
        }

        // Calendar dropdown
        let cal_node = self.calendar.layout(engine, font_system);

        engine.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                gap: Size {
                    width: zero(),
                    height: length(DROPDOWN_GAP),
                },
                ..Default::default()
            },
            &[trigger, cal_node],
        )
    }

    fn paint(
        &self,
        layouts: &[mozui_layout::ComputedLayout],
        index: &mut usize,
        draw_list: &mut DrawList,
        interactions: &mut InteractionMap,
        font_system: &FontSystem,
    ) {
        if !self.open {
            self.paint_trigger(layouts, index, draw_list, interactions);
            return;
        }

        // Wrapper
        let _wrapper = layouts[*index];
        *index += 1;

        // Trigger
        self.paint_trigger(layouts, index, draw_list, interactions);

        // Calendar
        self.calendar
            .paint(layouts, index, draw_list, interactions, font_system);

        // Escape to close
        if let Some(ref toggle) = self.on_toggle {
            let toggle_ptr = toggle.as_ref() as *const dyn Fn(&mut dyn std::any::Any);
            interactions.register_key_handler(Box::new(move |key, _mods, cx| {
                if key == mozui_events::Key::Escape {
                    unsafe { (*toggle_ptr)(cx) };
                }
            }));
        }
    }
}

impl DatePicker {
    fn paint_trigger(
        &self,
        layouts: &[mozui_layout::ComputedLayout],
        index: &mut usize,
        draw_list: &mut DrawList,
        interactions: &mut InteractionMap,
    ) {
        let trigger_layout = layouts[*index];
        *index += 1;
        let trigger_bounds = mozui_style::Rect::new(
            trigger_layout.x,
            trigger_layout.y,
            trigger_layout.width,
            trigger_layout.height,
        );

        let hovered = interactions.is_hovered(trigger_bounds);
        let active = interactions.is_active(trigger_bounds);

        let bg = if active {
            self.trigger_hover_bg
        } else if hovered {
            Color::new(
                self.trigger_bg.r * 0.95 + self.trigger_hover_bg.r * 0.05,
                self.trigger_bg.g * 0.95 + self.trigger_hover_bg.g * 0.05,
                self.trigger_bg.b * 0.95 + self.trigger_hover_bg.b * 0.05,
                1.0,
            )
        } else {
            self.trigger_bg
        };

        draw_list.push(DrawCommand::Rect {
            bounds: trigger_bounds,
            background: Fill::Solid(bg),
            corner_radii: Corners::uniform(self.corner_radius),
            border: Some(Border {
                width: 1.0,
                color: if self.open {
                    self.trigger_focus_border
                } else {
                    self.trigger_border
                },
            }),
            shadow: None,
        });

        // Calendar icon
        let icon_layout = layouts[*index];
        *index += 1;
        draw_list.push(DrawCommand::Icon {
            name: IconName::CalendarBlank,
            weight: IconWeight::Regular,
            bounds: mozui_style::Rect::new(
                icon_layout.x,
                icon_layout.y,
                icon_layout.width,
                icon_layout.height,
            ),
            color: self.trigger_muted_fg,
            size_px: ICON_SIZE,
        });

        // Text
        let text_layout = layouts[*index];
        *index += 1;
        let text_color = if self.has_value() {
            self.trigger_fg
        } else {
            self.trigger_muted_fg
        };
        draw_list.push(DrawCommand::Text {
            text: self.display_text(),
            bounds: mozui_style::Rect::new(
                text_layout.x,
                text_layout.y,
                text_layout.width,
                text_layout.height,
            ),
            font_size: self.font_size,
            color: text_color,
            weight: 400,
            italic: false,
        });

        // Chevron
        let chevron_layout = layouts[*index];
        *index += 1;
        draw_list.push(DrawCommand::Icon {
            name: if self.open {
                IconName::CaretUp
            } else {
                IconName::CaretDown
            },
            weight: IconWeight::Bold,
            bounds: mozui_style::Rect::new(
                chevron_layout.x,
                chevron_layout.y,
                chevron_layout.width,
                chevron_layout.height,
            ),
            color: self.trigger_muted_fg,
            size_px: ICON_SIZE,
        });

        // Click to toggle
        if let Some(ref toggle) = self.on_toggle {
            let toggle_ptr = toggle.as_ref() as *const dyn Fn(&mut dyn std::any::Any);
            interactions.register_click(
                trigger_bounds,
                Box::new(move |cx| unsafe { (*toggle_ptr)(cx) }),
            );
        }
    }
}
