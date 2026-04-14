//! Data model for the spreadsheet component, backed by IronCalc.

use ironcalc_base::{UserModel, expressions::types::Area, types::HorizontalAlignment};
use mozui::{AppContext as _, Context, Entity, EventEmitter, Hsla, TextAlign, Window, hsla};

use crate::input::{InputEvent, InputState};

// ── Color helpers ─────────────────────────────────────────────────────────────

/// Convert `Hsla` (h, s, l in 0.0..1.0) to `"#RRGGBB"` for IronCalc.
fn hsla_to_hex(c: Hsla) -> String {
    let h = c.h * 360.0;
    let s = c.s;
    let l = c.l;
    let cap = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = cap * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - cap / 2.0;
    let (r1, g1, b1) = if h < 60.0 {
        (cap, x, 0.0_f32)
    } else if h < 120.0 {
        (x, cap, 0.0)
    } else if h < 180.0 {
        (0.0, cap, x)
    } else if h < 240.0 {
        (0.0, x, cap)
    } else if h < 300.0 {
        (x, 0.0, cap)
    } else {
        (cap, 0.0, x)
    };
    let r = ((r1 + m) * 255.0).round() as u8;
    let g = ((g1 + m) * 255.0).round() as u8;
    let b = ((b1 + m) * 255.0).round() as u8;
    format!("#{:02X}{:02X}{:02X}", r, g, b)
}

/// Parse `"#RRGGBB"` hex string from IronCalc into `Hsla`.
fn hex_to_hsla(hex: &str) -> Option<Hsla> {
    let hex = hex.trim_start_matches('#');
    if hex.len() < 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()? as f32 / 255.0;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()? as f32 / 255.0;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()? as f32 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    let d = max - min;
    if d < 1e-6 {
        return Some(hsla(0.0, 0.0, l, 1.0));
    }
    let s = d / (1.0 - (2.0 * l - 1.0).abs());
    let h_raw = if (max - r).abs() < 1e-6 {
        (g - b) / d
    } else if (max - g).abs() < 1e-6 {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    } / 6.0;
    let h = if h_raw < 0.0 { h_raw + 1.0 } else { h_raw };
    Some(hsla(h, s, l, 1.0))
}

// ── Number format ─────────────────────────────────────────────────────────────

/// How a numeric cell value should be displayed.
/// Stored as an IronCalc `num_fmt` string — participates in undo/redo and serialisation.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum NumberFormat {
    #[default]
    Auto,
    Integer,
    Decimal(u8),
    Currency,
    Percent,
}

impl NumberFormat {
    pub fn to_ironcalc(&self) -> String {
        match self {
            NumberFormat::Auto => String::new(),
            NumberFormat::Integer => "0".to_string(),
            NumberFormat::Decimal(p) => format!("0.{}", "0".repeat(*p as usize)),
            NumberFormat::Currency => "$#,##0.00".to_string(),
            NumberFormat::Percent => "0.0%".to_string(),
        }
    }

    pub fn from_ironcalc(s: &str) -> Self {
        match s {
            "" | "General" | "general" => NumberFormat::Auto,
            "0" => NumberFormat::Integer,
            s if s.starts_with('$') => NumberFormat::Currency,
            s if s.ends_with('%') => NumberFormat::Percent,
            s if s.starts_with("0.") && s[2..].chars().all(|c| c == '0') => {
                NumberFormat::Decimal((s.len() - 2) as u8)
            }
            _ => NumberFormat::Auto,
        }
    }
}

// ── Cell format ───────────────────────────────────────────────────────────────

/// Visual formatting for a cell.
///
/// Written via IronCalc's `update_range_style` (participates in undo/redo, serialises with the
/// workbook). Read back via `get_cell_style`.
#[derive(Debug, Clone, Default)]
pub struct CellFormat {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub text_color: Option<Hsla>,
    pub bg_color: Option<Hsla>,
    pub align: TextAlign,
    pub number_format: NumberFormat,
}

impl CellFormat {
    pub fn header() -> Self {
        Self {
            bold: true,
            align: TextAlign::Center,
            ..Default::default()
        }
    }
    pub fn currency() -> Self {
        Self {
            align: TextAlign::Right,
            number_format: NumberFormat::Currency,
            ..Default::default()
        }
    }
    pub fn percent() -> Self {
        Self {
            align: TextAlign::Right,
            number_format: NumberFormat::Percent,
            ..Default::default()
        }
    }
    pub fn bold() -> Self {
        Self {
            bold: true,
            ..Default::default()
        }
    }
    pub fn right() -> Self {
        Self {
            align: TextAlign::Right,
            ..Default::default()
        }
    }
}

// ── Events ────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub enum SpreadsheetEvent {
    CellChanged {
        row: usize,
        col: usize,
    },
    EditingStarted {
        row: usize,
        col: usize,
    },
    EditingCommitted {
        row: usize,
        col: usize,
    },
    EditingCancelled,
    /// Emitted whenever the undo/redo availability changes.
    UndoStateChanged,
    /// Emitted when sheets are added, deleted, renamed, or the active sheet changes.
    SheetChanged,
}

// ── SpreadsheetData ───────────────────────────────────────────────────────────

/// Single-workbook spreadsheet state backed by an IronCalc [`UserModel`].
///
/// - Formula evaluation, undo/redo, multi-sheet, and style storage are all delegated to IronCalc.
/// - Formatting (bold, colour, alignment, number format) is written via `update_range_style` and
///   read back via `get_cell_style` — no separate formatting HashMap.
/// - Editing state (`editing_cell`, `input`) drives the inline cell editor.
pub struct SpreadsheetData {
    model: UserModel<'static>,
    pub rows: usize,
    pub cols: usize,
    pub editing_cell: Option<(usize, usize)>,
    pub input: Entity<InputState>,
    _input_sub: mozui::Subscription,
}

impl EventEmitter<SpreadsheetEvent> for SpreadsheetData {}

impl SpreadsheetData {
    pub fn new(rows: usize, cols: usize, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let model = UserModel::new_empty("Sheet1", "en", "UTC", "en")
            .expect("failed to create IronCalc model");

        let input = cx.new(|cx| InputState::new(window, cx));

        let sub = cx.subscribe(
            &input,
            |this: &mut Self, _input, event: &InputEvent, cx| match event {
                InputEvent::PressEnter { .. } | InputEvent::Blur => {
                    this.commit_edit(cx);
                }
                _ => {}
            },
        );

        Self {
            model,
            rows,
            cols,
            editing_cell: None,
            input,
            _input_sub: sub,
        }
    }

    // ── Sheet management ─────────────────────────────────────────────────────

    /// Index of the currently active sheet.
    pub fn active_sheet(&self) -> u32 {
        self.model.get_selected_sheet()
    }

    /// Names of all visible (non-hidden) sheets, in order.
    pub fn sheet_names(&self) -> Vec<String> {
        self.model
            .get_worksheets_properties()
            .into_iter()
            .filter(|s| s.state != "hidden")
            .map(|s| s.name)
            .collect()
    }

    /// Returns `(sheet_index, name)` for every visible sheet.
    ///
    /// The `sheet_index` is the position in IronCalc's full worksheets array
    /// (including hidden sheets) — this is the value all IronCalc APIs expect.
    pub fn visible_sheets(&self) -> Vec<(u32, String)> {
        self.model
            .get_worksheets_properties()
            .into_iter()
            .enumerate()
            .filter(|(_, s)| s.state != "hidden")
            .map(|(i, s)| (i as u32, s.name))
            .collect()
    }

    pub fn add_sheet(&mut self, cx: &mut Context<Self>) {
        let current = self.model.get_selected_sheet();
        let _ = self.model.new_sheet();
        // Stay on the current sheet — don't jump to the new empty one.
        let _ = self.model.set_selected_sheet(current);
        cx.emit(SpreadsheetEvent::SheetChanged);
        cx.notify();
    }

    pub fn delete_sheet(&mut self, sheet_index: u32, cx: &mut Context<Self>) {
        let _ = self.model.delete_sheet(sheet_index);
        cx.emit(SpreadsheetEvent::SheetChanged);
        cx.notify();
    }

    pub fn rename_sheet(&mut self, sheet_index: u32, name: &str, cx: &mut Context<Self>) {
        let _ = self.model.rename_sheet(sheet_index, name);
        cx.emit(SpreadsheetEvent::SheetChanged);
        cx.notify();
    }

    pub fn set_active_sheet(&mut self, sheet_index: u32, cx: &mut Context<Self>) {
        let _ = self.model.set_selected_sheet(sheet_index);
        cx.emit(SpreadsheetEvent::SheetChanged);
        cx.notify();
    }

    // ── Selection (backed by IronCalc) ──────────────────────────────────────

    /// Tell IronCalc the active cell (anchor). Also collapses the range to that cell.
    pub fn set_selected_cell(&mut self, row: usize, col: usize) {
        let _ = self
            .model
            .set_selected_cell((row + 1) as i32, (col + 1) as i32);
    }

    /// Extend the selection from the anchor to `(row, col)`.
    ///
    /// The anchor must already be set via [`set_selected_cell`].
    pub fn extend_selection_to(&mut self, row: usize, col: usize) {
        let view = self.model.get_selected_view();
        let _ = self.model.set_selected_range(
            view.row,
            view.column,
            (row + 1) as i32,
            (col + 1) as i32,
        );
    }

    /// Read back the current range from IronCalc as 0-based
    /// `(start_row, start_col, end_row, end_col)`.
    ///
    /// Returns `None` when the range is a single cell (no multi-cell selection).
    pub fn selected_range(&self) -> Option<(usize, usize, usize, usize)> {
        let view = self.model.get_selected_view();
        let [r0, c0, r1, c1] = view.range;
        if r0 == r1 && c0 == c1 {
            return None;
        }
        Some((
            (r0 - 1) as usize,
            (c0 - 1) as usize,
            (r1 - 1) as usize,
            (c1 - 1) as usize,
        ))
    }

    // ── Clipboard ────────────────────────────────────────────────────────────

    /// Copy selected range to a TSV string suitable for the system clipboard.
    pub fn copy_selection_as_tsv(&self) -> String {
        let view = self.model.get_selected_view();
        let [r0, c0, r1, c1] = view.range;
        let sheet = self.active_sheet();
        let mut lines = Vec::new();
        for row in r0..=r1 {
            let mut cells = Vec::new();
            for col in c0..=c1 {
                let val = self
                    .model
                    .get_formatted_cell_value(sheet, row, col)
                    .unwrap_or_default();
                cells.push(val);
            }
            lines.push(cells.join("\t"));
        }
        lines.join("\n")
    }

    /// Cut: copy TSV then clear the selected cells.
    pub fn cut_selection_as_tsv(&mut self, cx: &mut Context<Self>) -> String {
        let tsv = self.copy_selection_as_tsv();
        let view = self.model.get_selected_view();
        let [r0, c0, r1, c1] = view.range;
        let sheet = self.active_sheet();
        for row in r0..=r1 {
            for col in c0..=c1 {
                let _ = self.model.set_user_input(sheet, row, col, "");
            }
        }
        self.model.evaluate();
        cx.emit(SpreadsheetEvent::UndoStateChanged);
        cx.notify();
        tsv
    }

    /// Paste a TSV/CSV string at the currently selected cell.
    pub fn paste_tsv(&mut self, tsv: &str, cx: &mut Context<Self>) {
        let view = self.model.get_selected_view();
        let area = Area {
            sheet: self.active_sheet(),
            row: view.row,
            column: view.column,
            width: 1,
            height: 1,
        };
        let _ = self.model.paste_csv_string(&area, tsv);
        self.model.evaluate();
        cx.emit(SpreadsheetEvent::UndoStateChanged);
        cx.notify();
    }

    // ── Freeze panes ─────────────────────────────────────────────────────────

    pub fn set_frozen_rows(&mut self, count: usize, cx: &mut Context<Self>) {
        let sheet = self.active_sheet();
        let _ = self.model.set_frozen_rows_count(sheet, count as i32);
        cx.notify();
    }

    pub fn set_frozen_cols(&mut self, count: usize, cx: &mut Context<Self>) {
        let sheet = self.active_sheet();
        let _ = self.model.set_frozen_columns_count(sheet, count as i32);
        cx.notify();
    }

    pub fn frozen_rows(&self) -> usize {
        let sheet = self.active_sheet();
        self.model
            .get_frozen_rows_count(sheet)
            .unwrap_or(0)
            .max(0) as usize
    }

    pub fn frozen_cols(&self) -> usize {
        let sheet = self.active_sheet();
        self.model
            .get_frozen_columns_count(sheet)
            .unwrap_or(0)
            .max(0) as usize
    }

    // ── Grid lines ──────────────────────────────────────────────────────────

    pub fn set_show_grid_lines(&mut self, show: bool, cx: &mut Context<Self>) {
        let sheet = self.active_sheet();
        let _ = self.model.set_show_grid_lines(sheet, show);
        cx.notify();
    }

    pub fn show_grid_lines(&self) -> bool {
        let sheet = self.active_sheet();
        self.model.get_show_grid_lines(sheet).unwrap_or(true)
    }

    // ── Serialization ───────────────────────────────────────────────────────

    pub fn save(&self) -> Vec<u8> {
        self.model.to_bytes()
    }

    pub fn load(
        bytes: &[u8],
        rows: usize,
        cols: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let model =
            UserModel::from_bytes(bytes, "en").expect("failed to deserialize IronCalc model");

        let input = cx.new(|cx| InputState::new(window, cx));
        let sub = cx.subscribe(
            &input,
            |this: &mut Self, _input, event: &InputEvent, cx| match event {
                InputEvent::PressEnter { .. } | InputEvent::Blur => {
                    this.commit_edit(cx);
                }
                _ => {}
            },
        );

        Self {
            model,
            rows,
            cols,
            editing_cell: None,
            input,
            _input_sub: sub,
        }
    }

    // ── Undo / Redo ──────────────────────────────────────────────────────────

    pub fn can_undo(&self) -> bool {
        self.model.can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.model.can_redo()
    }

    pub fn undo(&mut self, cx: &mut Context<Self>) {
        let _ = self.model.undo();
        cx.emit(SpreadsheetEvent::UndoStateChanged);
        cx.notify();
    }

    pub fn redo(&mut self, cx: &mut Context<Self>) {
        let _ = self.model.redo();
        cx.emit(SpreadsheetEvent::UndoStateChanged);
        cx.notify();
    }

    /// Snapshot the current model state and discard all undo history.
    ///
    /// Call after programmatic data population so that Cmd+Z only captures user edits,
    /// not the initial data setup.
    pub fn snapshot_clear_history(&mut self) {
        let bytes = self.model.to_bytes();
        if let Ok(fresh) = UserModel::from_bytes(&bytes, "en") {
            self.model = fresh;
        }
    }

    // ── Data ─────────────────────────────────────────────────────────────────

    /// Set a cell via raw user input (plain values or `=FORMULA` strings).
    /// Automatically evaluates after write.
    pub fn set_cell(&mut self, row: usize, col: usize, value: impl AsRef<str>) {
        let sheet = self.active_sheet();
        let _ =
            self.model
                .set_user_input(sheet, (row + 1) as i32, (col + 1) as i32, value.as_ref());
        self.model.evaluate();
    }

    /// Formatted display string for a cell (IronCalc applies the stored `num_fmt`).
    pub fn display_value(&self, row: usize, col: usize) -> String {
        let sheet = self.active_sheet();
        self.model
            .get_formatted_cell_value(sheet, (row + 1) as i32, (col + 1) as i32)
            .unwrap_or_default()
    }

    /// Raw cell content — formula string if formula, otherwise the value as entered.
    pub fn raw_value(&self, row: usize, col: usize) -> String {
        let sheet = self.active_sheet();
        self.model
            .get_cell_content(sheet, (row + 1) as i32, (col + 1) as i32)
            .unwrap_or_default()
    }

    // ── Formatting ───────────────────────────────────────────────────────────

    fn cell_area(&self, row: usize, col: usize) -> Area {
        Area {
            sheet: self.active_sheet(),
            row: (row + 1) as i32,
            column: (col + 1) as i32,
            width: 1,
            height: 1,
        }
    }

    /// Apply a [`CellFormat`] to a cell via IronCalc's style system.
    /// All changes are recorded in the undo history.
    pub fn set_format(&mut self, row: usize, col: usize, fmt: CellFormat) {
        let area = self.cell_area(row, col);
        let _ =
            self.model
                .update_range_style(&area, "font.b", if fmt.bold { "true" } else { "false" });
        let _ = self.model.update_range_style(
            &area,
            "font.i",
            if fmt.italic { "true" } else { "false" },
        );
        if let Some(c) = fmt.text_color {
            let _ = self
                .model
                .update_range_style(&area, "font.color", &hsla_to_hex(c));
        }
        if let Some(c) = fmt.bg_color {
            let _ = self
                .model
                .update_range_style(&area, "fill.bg_color", &hsla_to_hex(c));
        }
        let align_str = match fmt.align {
            TextAlign::Right => "right",
            TextAlign::Center => "center",
            _ => "general",
        };
        let _ = self
            .model
            .update_range_style(&area, "alignment.horizontal", align_str);
        let num_fmt = fmt.number_format.to_ironcalc();
        let _ = self.model.update_range_style(&area, "num_fmt", &num_fmt);
    }

    /// Read formatting for a cell from IronCalc's style system.
    pub fn format(&self, row: usize, col: usize) -> CellFormat {
        let sheet = self.active_sheet();
        let Ok(style) = self
            .model
            .get_cell_style(sheet, (row + 1) as i32, (col + 1) as i32)
        else {
            return CellFormat::default();
        };

        let align = match style.alignment.as_ref().map(|a| &a.horizontal) {
            Some(HorizontalAlignment::Right) => TextAlign::Right,
            Some(HorizontalAlignment::Center) => TextAlign::Center,
            _ => TextAlign::Left,
        };

        CellFormat {
            bold: style.font.b,
            italic: style.font.i,
            underline: style.font.u,
            // Filter out the workbook-default black — it's just "no colour set".
            text_color: style
                .font
                .color
                .as_deref()
                .filter(|c| !c.is_empty() && *c != "#000000")
                .and_then(hex_to_hsla),
            bg_color: style
                .fill
                .bg_color
                .as_deref()
                .filter(|c| !c.is_empty())
                .and_then(hex_to_hsla),
            align,
            number_format: NumberFormat::from_ironcalc(&style.num_fmt),
        }
    }

    pub fn toggle_bold(&mut self, row: usize, col: usize, cx: &mut Context<Self>) {
        let bold = !self.format(row, col).bold;
        let area = self.cell_area(row, col);
        let _ = self
            .model
            .update_range_style(&area, "font.b", if bold { "true" } else { "false" });
        cx.emit(SpreadsheetEvent::UndoStateChanged);
        cx.notify();
    }

    pub fn toggle_italic(&mut self, row: usize, col: usize, cx: &mut Context<Self>) {
        let italic = !self.format(row, col).italic;
        let area = self.cell_area(row, col);
        let _ =
            self.model
                .update_range_style(&area, "font.i", if italic { "true" } else { "false" });
        cx.emit(SpreadsheetEvent::UndoStateChanged);
        cx.notify();
    }

    pub fn set_align(&mut self, row: usize, col: usize, align: TextAlign, cx: &mut Context<Self>) {
        let align_str = match align {
            TextAlign::Right => "right",
            TextAlign::Center => "center",
            _ => "general",
        };
        let area = self.cell_area(row, col);
        let _ = self
            .model
            .update_range_style(&area, "alignment.horizontal", align_str);
        cx.emit(SpreadsheetEvent::UndoStateChanged);
        cx.notify();
    }

    pub fn set_number_format(
        &mut self,
        row: usize,
        col: usize,
        number_format: NumberFormat,
        cx: &mut Context<Self>,
    ) {
        let num_fmt = number_format.to_ironcalc();
        let area = self.cell_area(row, col);
        let _ = self.model.update_range_style(&area, "num_fmt", &num_fmt);
        cx.emit(SpreadsheetEvent::UndoStateChanged);
        cx.notify();
    }

    pub fn toggle_underline(&mut self, row: usize, col: usize, cx: &mut Context<Self>) {
        let style = self
            .model
            .get_cell_style(self.active_sheet(), (row + 1) as i32, (col + 1) as i32)
            .ok();
        let underline = !style.map(|s| s.font.u).unwrap_or(false);
        let area = self.cell_area(row, col);
        let _ = self.model.update_range_style(
            &area,
            "font.u",
            if underline { "true" } else { "false" },
        );
        cx.emit(SpreadsheetEvent::UndoStateChanged);
        cx.notify();
    }

    pub fn is_underline(&self, row: usize, col: usize) -> bool {
        self.model
            .get_cell_style(self.active_sheet(), (row + 1) as i32, (col + 1) as i32)
            .map(|s| s.font.u)
            .unwrap_or(false)
    }

    /// Clear formatting from a range of cells (keeps contents).
    pub fn clear_formatting_range(
        &mut self,
        r0: usize,
        c0: usize,
        r1: usize,
        c1: usize,
        cx: &mut Context<Self>,
    ) {
        let area = Area {
            sheet: self.active_sheet(),
            row: (r0 + 1) as i32,
            column: (c0 + 1) as i32,
            width: (c1 - c0 + 1) as i32,
            height: (r1 - r0 + 1) as i32,
        };
        let _ = self.model.range_clear_formatting(&area);
        cx.emit(SpreadsheetEvent::UndoStateChanged);
        cx.notify();
    }

    /// Clear contents from a range of cells (keeps formatting).
    pub fn clear_contents_range(
        &mut self,
        r0: usize,
        c0: usize,
        r1: usize,
        c1: usize,
        cx: &mut Context<Self>,
    ) {
        let area = Area {
            sheet: self.active_sheet(),
            row: (r0 + 1) as i32,
            column: (c0 + 1) as i32,
            width: (c1 - c0 + 1) as i32,
            height: (r1 - r0 + 1) as i32,
        };
        let _ = self.model.range_clear_contents(&area);
        self.model.evaluate();
        cx.emit(SpreadsheetEvent::UndoStateChanged);
        cx.notify();
    }

    /// Clear everything (contents + formatting) from a range.
    pub fn clear_all_range(
        &mut self,
        r0: usize,
        c0: usize,
        r1: usize,
        c1: usize,
        cx: &mut Context<Self>,
    ) {
        let area = Area {
            sheet: self.active_sheet(),
            row: (r0 + 1) as i32,
            column: (c0 + 1) as i32,
            width: (c1 - c0 + 1) as i32,
            height: (r1 - r0 + 1) as i32,
        };
        let _ = self.model.range_clear_all(&area);
        self.model.evaluate();
        cx.emit(SpreadsheetEvent::UndoStateChanged);
        cx.notify();
    }

    // ── Sheet visibility ────────────────────────────────────────────────────

    pub fn hide_sheet(&mut self, sheet_index: u32, cx: &mut Context<Self>) {
        let _ = self.model.hide_sheet(sheet_index);
        cx.emit(SpreadsheetEvent::SheetChanged);
        cx.notify();
    }

    pub fn unhide_sheet(&mut self, sheet_index: u32, cx: &mut Context<Self>) {
        let _ = self.model.unhide_sheet(sheet_index);
        cx.emit(SpreadsheetEvent::SheetChanged);
        cx.notify();
    }

    /// Returns `(sheet_index, name)` for every hidden sheet.
    pub fn hidden_sheets(&self) -> Vec<(u32, String)> {
        self.model
            .get_worksheets_properties()
            .into_iter()
            .enumerate()
            .filter(|(_, s)| s.state == "hidden")
            .map(|(i, s)| (i as u32, s.name))
            .collect()
    }

    // ── Locale / timezone ───────────────────────────────────────────────────

    pub fn set_locale(&mut self, locale: &str) {
        let _ = self.model.set_locale(locale);
    }

    pub fn set_timezone(&mut self, tz: &str) {
        let _ = self.model.set_timezone(tz);
    }

    pub fn locale(&self) -> String {
        self.model.get_locale()
    }

    pub fn timezone(&self) -> String {
        self.model.get_timezone()
    }

    // ── Editing ──────────────────────────────────────────────────────────────

    pub fn start_editing(
        &mut self,
        row: usize,
        col: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if row >= self.rows || col >= self.cols {
            return;
        }
        let raw = self.raw_value(row, col);
        self.editing_cell = Some((row, col));
        self.input.update(cx, |state, cx| {
            state.set_value(raw, window, cx);
            state.focus(window, cx);
        });
        cx.emit(SpreadsheetEvent::EditingStarted { row, col });
        cx.notify();
    }

    pub fn commit_edit(&mut self, cx: &mut Context<Self>) {
        if let Some((row, col)) = self.editing_cell.take() {
            let raw = self.input.read(cx).value().to_string();
            let sheet = self.active_sheet();
            let _ = self
                .model
                .set_user_input(sheet, (row + 1) as i32, (col + 1) as i32, &raw);
            self.model.evaluate();
            cx.emit(SpreadsheetEvent::CellChanged { row, col });
            cx.emit(SpreadsheetEvent::EditingCommitted { row, col });
            cx.emit(SpreadsheetEvent::UndoStateChanged);
        }
        cx.notify();
    }

    pub fn cancel_edit(&mut self, cx: &mut Context<Self>) {
        self.editing_cell = None;
        cx.emit(SpreadsheetEvent::EditingCancelled);
        cx.notify();
    }

    // ── Row / column operations ──────────────────────────────────────────────

    /// Append a new row at the bottom of the grid.
    pub fn add_row(&mut self, cx: &mut Context<Self>) {
        let sheet = self.active_sheet();
        let _ = self.model.insert_rows(sheet, (self.rows + 1) as i32, 1);
        self.rows += 1;
        cx.emit(SpreadsheetEvent::UndoStateChanged);
        cx.notify();
    }

    /// Insert a new row before `before_row` (0-based).
    pub fn insert_row(&mut self, before_row: usize, cx: &mut Context<Self>) {
        let sheet = self.active_sheet();
        let _ = self.model.insert_rows(sheet, (before_row + 1) as i32, 1);
        self.rows += 1;
        cx.emit(SpreadsheetEvent::UndoStateChanged);
        cx.notify();
    }

    /// Delete the row at `row` (0-based).
    pub fn delete_row(&mut self, row: usize, cx: &mut Context<Self>) {
        if self.rows == 0 {
            return;
        }
        let sheet = self.active_sheet();
        let _ = self.model.delete_rows(sheet, (row + 1) as i32, 1);
        self.rows = self.rows.saturating_sub(1);
        cx.emit(SpreadsheetEvent::UndoStateChanged);
        cx.notify();
    }

    /// Insert a new column before `before_col` (0-based).
    pub fn insert_col(&mut self, before_col: usize, cx: &mut Context<Self>) {
        let sheet = self.active_sheet();
        let _ = self.model.insert_columns(sheet, (before_col + 1) as i32, 1);
        self.cols += 1;
        cx.emit(SpreadsheetEvent::UndoStateChanged);
        cx.notify();
    }

    /// Delete the column at `col` (0-based).
    pub fn delete_col(&mut self, col: usize, cx: &mut Context<Self>) {
        if self.cols == 0 {
            return;
        }
        let sheet = self.active_sheet();
        let _ = self.model.delete_columns(sheet, (col + 1) as i32, 1);
        self.cols = self.cols.saturating_sub(1);
        cx.emit(SpreadsheetEvent::UndoStateChanged);
        cx.notify();
    }
}
