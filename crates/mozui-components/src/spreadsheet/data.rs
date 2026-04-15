//! Data model for the spreadsheet component, backed by IronCalc.

use ironcalc_base::{
    UserModel,
    expressions::{lexer::util::get_tokens, token::TokenType, types::Area},
    types::HorizontalAlignment,
};
use log::warn;
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

// ── Formula range highlights ─────────────────────────────────────────────────

/// A cell range referenced by a formula, with a color for highlighting.
#[derive(Debug, Clone)]
pub struct FormulaHighlight {
    /// 0-based inclusive range: (start_row, start_col, end_row, end_col).
    pub r0: usize,
    pub c0: usize,
    pub r1: usize,
    pub c1: usize,
    /// Color for the highlight border/overlay.
    pub color: Hsla,
}

/// Palette of distinct colors for formula reference highlights.
const HIGHLIGHT_COLORS: &[Hsla] = &[
    Hsla { h: 0.6, s: 0.7, l: 0.55, a: 1.0 },   // blue
    Hsla { h: 0.0, s: 0.7, l: 0.55, a: 1.0 },    // red
    Hsla { h: 0.3, s: 0.7, l: 0.45, a: 1.0 },     // green
    Hsla { h: 0.08, s: 0.8, l: 0.55, a: 1.0 },   // orange
    Hsla { h: 0.75, s: 0.6, l: 0.55, a: 1.0 },   // purple
    Hsla { h: 0.5, s: 0.7, l: 0.45, a: 1.0 },    // teal
    Hsla { h: 0.95, s: 0.7, l: 0.55, a: 1.0 },   // pink
    Hsla { h: 0.15, s: 0.8, l: 0.5, a: 1.0 },    // yellow-green
];

/// Extract cell/range references from a formula using IronCalc's tokenizer.
/// Returns 0-based `(start_row, start_col, end_row, end_col)` for each reference.
/// Only returns same-sheet refs (sheet field is None).
pub fn parse_formula_refs(formula: &str) -> Vec<(usize, usize, usize, usize)> {
    if !formula.starts_with('=') {
        return Vec::new();
    }

    let tokens = get_tokens(&formula[1..]); // skip '='
    let mut refs = Vec::new();

    for mt in &tokens {
        match &mt.token {
            TokenType::Reference {
                sheet: None,
                row,
                column,
                ..
            } => {
                if *row > 0 && *column > 0 {
                    let r = (*row - 1) as usize;
                    let c = (*column - 1) as usize;
                    refs.push((r, c, r, c));
                }
            }
            TokenType::Range {
                sheet: None,
                left,
                right,
            } => {
                if left.row > 0 && left.column > 0 && right.row > 0 && right.column > 0 {
                    let r0 = (left.row - 1) as usize;
                    let c0 = (left.column - 1) as usize;
                    let r1 = (right.row - 1) as usize;
                    let c1 = (right.column - 1) as usize;
                    refs.push((r0.min(r1), c0.min(c1), r0.max(r1), c0.max(c1)));
                }
            }
            _ => {}
        }
    }

    refs
}

/// Assign a palette color to a cell range, reusing the same color for duplicate refs.
fn color_for_ref(
    r0: usize, c0: usize, r1: usize, c1: usize,
    seen: &mut Vec<(usize, usize, usize, usize)>,
) -> Hsla {
    let key = (r0, c0, r1, c1);
    let idx = if let Some(pos) = seen.iter().position(|k| *k == key) {
        pos
    } else {
        let pos = seen.len();
        seen.push(key);
        pos
    };
    HIGHLIGHT_COLORS[idx % HIGHLIGHT_COLORS.len()]
}

/// Build `FormulaHighlight` list from parsed refs.
/// Same cell/range reference gets the same color.
pub fn build_formula_highlights(formula: &str) -> Vec<FormulaHighlight> {
    let refs = parse_formula_refs(formula);
    let mut seen = Vec::new();
    refs.into_iter()
        .map(|(r0, c0, r1, c1)| FormulaHighlight {
            color: color_for_ref(r0, c0, r1, c1, &mut seen),
            r0, c0, r1, c1,
        })
        .collect()
}

/// Build full-coverage highlight ranges for a formula string.
/// Returns `(byte_range, Option<color>)` spans covering every byte.
/// Reference/range tokens get a color; everything else gets `None`.
/// Same cell/range reference gets the same color.
pub fn formula_bar_highlights(formula: &str) -> Vec<(std::ops::Range<usize>, Option<Hsla>)> {
    if !formula.starts_with('=') {
        return Vec::new();
    }

    let tokens = get_tokens(&formula[1..]);
    let refs = parse_formula_refs(formula);

    // Build color map from unique refs.
    let mut seen: Vec<(usize, usize, usize, usize)> = Vec::new();
    let mut ref_colors: Vec<Hsla> = Vec::new();
    for &(r0, c0, r1, c1) in &refs {
        ref_colors.push(color_for_ref(r0, c0, r1, c1, &mut seen));
    }

    // Collect reference token byte positions (offset +1 for leading '=').
    let mut ref_spans: Vec<(usize, usize, Hsla)> = Vec::new();
    let mut ref_idx = 0;
    for mt in &tokens {
        let is_ref = matches!(
            &mt.token,
            TokenType::Reference { sheet: None, .. } | TokenType::Range { sheet: None, .. }
        );
        if is_ref && ref_idx < ref_colors.len() {
            let start = (mt.start as usize) + 1;
            let end = (mt.end as usize) + 1;
            ref_spans.push((start, end, ref_colors[ref_idx]));
            ref_idx += 1;
        }
    }

    if ref_spans.is_empty() {
        return Vec::new();
    }

    // Build contiguous spans covering the full string.
    let len = formula.len();
    let mut result = Vec::new();
    let mut cursor = 0;

    for (start, end, color) in &ref_spans {
        let start = (*start).min(len);
        let end = (*end).min(len);
        if cursor < start {
            result.push((cursor..start, None));
        }
        result.push((start..end, Some(*color)));
        cursor = end;
    }
    if cursor < len {
        result.push((cursor..len, None));
    }

    result
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
    /// Font size in points (IronCalc default = 11). 0 means default.
    pub font_size: i32,
    pub wrap_text: bool,
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
    /// Highlighted cell ranges from the formula currently being edited.
    pub formula_highlights: Vec<FormulaHighlight>,
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
            formula_highlights: Vec::new(),
            _input_sub: sub,
        }
    }

    /// Recompute formula highlights from the given text (called when edit input changes).
    pub fn update_formula_highlights(&mut self, formula_text: &str) {
        self.formula_highlights = build_formula_highlights(formula_text);
    }

    /// Clear formula highlights (called when editing ends).
    pub fn clear_formula_highlights(&mut self) {
        self.formula_highlights.clear();
    }

    // ── Sheet management ─────────────────────────────────────────────────────

    /// Index of the currently active sheet.
    pub fn active_sheet(&self) -> u32 {
        self.model.get_selected_sheet()
    }

    /// Returns `(max_row, max_col)` — the extent of actual data in the active sheet.
    /// Values are 0-based and inclusive.
    pub fn data_extent(&self) -> (usize, usize) {
        let sheet = self.active_sheet();
        if let Ok(ws) = self.model.get_model().workbook.worksheet(sheet) {
            let dim = ws.dimension();
            ((dim.max_row - 1).max(0) as usize, (dim.max_column - 1).max(0) as usize)
        } else {
            (0, 0)
        }
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
    /// Returns `(sheet_index, name, tab_color)` for each visible sheet.
    pub fn visible_sheets(&self) -> Vec<(u32, String, Option<Hsla>)> {
        self.model
            .get_worksheets_properties()
            .into_iter()
            .enumerate()
            .filter(|(_, s)| s.state != "hidden")
            .map(|(i, s)| {
                let color = s.color.as_deref().and_then(hex_to_hsla);
                (i as u32, s.name, color)
            })
            .collect()
    }

    /// Set tab color for a sheet.
    pub fn set_sheet_color(&mut self, sheet_index: u32, color: &str, cx: &mut Context<Self>) {
        if let Err(e) = self.model.set_sheet_color(sheet_index, color) {
            warn!("spreadsheet: failed to set sheet color: {e}");
            return;
        }
        cx.emit(SpreadsheetEvent::SheetChanged);
        cx.notify();
    }

    pub fn add_sheet(&mut self, cx: &mut Context<Self>) {
        let current = self.model.get_selected_sheet();
        if let Err(e) = self.model.new_sheet() {
            warn!("spreadsheet: failed to add sheet: {e}");
            return;
        }
        // Stay on the current sheet — don't jump to the new empty one.
        let _ = self.model.set_selected_sheet(current);
        cx.emit(SpreadsheetEvent::SheetChanged);
        cx.notify();
    }

    pub fn delete_sheet(&mut self, sheet_index: u32, cx: &mut Context<Self>) {
        if let Err(e) = self.model.delete_sheet(sheet_index) {
            warn!("spreadsheet: failed to delete sheet {sheet_index}: {e}");
            return;
        }
        cx.emit(SpreadsheetEvent::SheetChanged);
        cx.notify();
    }

    pub fn rename_sheet(&mut self, sheet_index: u32, name: &str, cx: &mut Context<Self>) {
        if let Err(e) = self.model.rename_sheet(sheet_index, name) {
            warn!("spreadsheet: failed to rename sheet {sheet_index} to '{name}': {e}");
            return;
        }
        cx.emit(SpreadsheetEvent::SheetChanged);
        cx.notify();
    }

    pub fn set_active_sheet(&mut self, sheet_index: u32, cx: &mut Context<Self>) {
        if let Err(e) = self.model.set_selected_sheet(sheet_index) {
            warn!("spreadsheet: failed to set active sheet {sheet_index}: {e}");
            return;
        }
        cx.emit(SpreadsheetEvent::SheetChanged);
        cx.notify();
    }

    // ── Selection (backed by IronCalc) ──────────────────────────────────────

    /// Read back the active (anchor) cell from IronCalc as 0-based `(row, col)`.
    pub fn selected_cell(&self) -> (usize, usize) {
        let view = self.model.get_selected_view();
        ((view.row - 1) as usize, (view.column - 1) as usize)
    }

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
        // Normalize so (start_row, start_col) <= (end_row, end_col)
        // regardless of selection direction.
        let min_r = r0.min(r1);
        let max_r = r0.max(r1);
        let min_c = c0.min(c1);
        let max_c = c0.max(c1);
        Some((
            (min_r - 1) as usize,
            (min_c - 1) as usize,
            (max_r - 1) as usize,
            (max_c - 1) as usize,
        ))
    }

    // ── Clipboard ────────────────────────────────────────────────────────────

    /// Copy selected range to a TSV string suitable for the system clipboard.
    /// Uses raw cell content so formulas are preserved (e.g. `=SUM(A1:A5)`).
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
                    .get_cell_content(sheet, row, col)
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
        self.model.pause_evaluation();
        for row in r0..=r1 {
            for col in c0..=c1 {
                let _ = self.model.set_user_input(sheet, row, col, "");
            }
        }
        self.model.resume_evaluation();
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
        // Grow grid to fit pasted content.
        let line_count = tsv.lines().count();
        let col_count = tsv.lines().next().map_or(0, |l| l.split('\t').count());
        let paste_end_row = (view.row - 1) as usize + line_count;
        let paste_end_col = (view.column - 1) as usize + col_count;
        self.ensure_bounds(
            paste_end_row.saturating_sub(1),
            paste_end_col.saturating_sub(1),
        );
        if let Err(e) = self.model.paste_csv_string(&area, tsv) {
            warn!("spreadsheet: paste failed: {e}");
        }
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
        self.model.get_frozen_rows_count(sheet).unwrap_or(0).max(0) as usize
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
            formula_highlights: Vec::new(),
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

    // ── Evaluation batching ────────────────────────────────────────────────

    /// Pause formula evaluation. Call [`resume_evaluation`] when done with bulk writes.
    pub fn pause_evaluation(&mut self) {
        self.model.pause_evaluation();
    }

    /// Resume formula evaluation and recalculate all pending changes.
    pub fn resume_evaluation(&mut self) {
        self.model.resume_evaluation();
        self.model.evaluate();
    }

    // ── Data ─────────────────────────────────────────────────────────────────

    /// Grow grid boundaries if `(row, col)` falls outside current `rows`/`cols`.
    fn ensure_bounds(&mut self, row: usize, col: usize) {
        if row >= self.rows {
            self.rows = row + 1;
        }
        if col >= self.cols {
            self.cols = col + 1;
        }
    }

    /// Set a cell via raw user input (plain values or `=FORMULA` strings).
    /// Automatically evaluates after write. Grows grid if needed.
    pub fn set_cell(&mut self, row: usize, col: usize, value: impl AsRef<str>) {
        self.ensure_bounds(row, col);
        let sheet = self.active_sheet();
        let _ =
            self.model
                .set_user_input(sheet, (row + 1) as i32, (col + 1) as i32, value.as_ref());
        self.model.evaluate();
    }

    /// Set a cell without triggering evaluation. Use with [`pause_evaluation`] /
    /// [`resume_evaluation`] for bulk writes. Grows grid if needed.
    pub fn set_cell_quiet(&mut self, row: usize, col: usize, value: impl AsRef<str>) {
        self.ensure_bounds(row, col);
        let sheet = self.active_sheet();
        let _ =
            self.model
                .set_user_input(sheet, (row + 1) as i32, (col + 1) as i32, value.as_ref());
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

    fn range_area(&self, r0: usize, c0: usize, r1: usize, c1: usize) -> Area {
        let min_r = r0.min(r1);
        let max_r = r0.max(r1);
        let min_c = c0.min(c1);
        let max_c = c0.max(c1);
        Area {
            sheet: self.active_sheet(),
            row: (min_r + 1) as i32,
            column: (min_c + 1) as i32,
            width: (max_c - min_c + 1) as i32,
            height: (max_r - min_r + 1) as i32,
        }
    }

    /// Resolve the effective range for a formatting operation.
    /// If a multi-cell range is selected, use it. Otherwise fall back to the single cell.
    fn effective_range(&self, row: usize, col: usize) -> (usize, usize, usize, usize) {
        self.selected_range().unwrap_or((row, col, row, col))
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
            font_size: style.font.sz,
            wrap_text: style
                .alignment
                .as_ref()
                .map(|a| a.wrap_text)
                .unwrap_or(false),
        }
    }

    /// Toggle bold on the active cell or selected range.
    pub fn toggle_bold(&mut self, row: usize, col: usize, cx: &mut Context<Self>) {
        let bold = !self.format(row, col).bold;
        let (r0, c0, r1, c1) = self.effective_range(row, col);
        let area = self.range_area(r0, c0, r1, c1);
        let _ = self
            .model
            .update_range_style(&area, "font.b", if bold { "true" } else { "false" });
        cx.emit(SpreadsheetEvent::UndoStateChanged);
        cx.notify();
    }

    /// Toggle italic on the active cell or selected range.
    pub fn toggle_italic(&mut self, row: usize, col: usize, cx: &mut Context<Self>) {
        let italic = !self.format(row, col).italic;
        let (r0, c0, r1, c1) = self.effective_range(row, col);
        let area = self.range_area(r0, c0, r1, c1);
        let _ =
            self.model
                .update_range_style(&area, "font.i", if italic { "true" } else { "false" });
        cx.emit(SpreadsheetEvent::UndoStateChanged);
        cx.notify();
    }

    /// Set alignment on the active cell or selected range.
    pub fn set_align(&mut self, row: usize, col: usize, align: TextAlign, cx: &mut Context<Self>) {
        let align_str = match align {
            TextAlign::Right => "right",
            TextAlign::Center => "center",
            _ => "general",
        };
        let (r0, c0, r1, c1) = self.effective_range(row, col);
        let area = self.range_area(r0, c0, r1, c1);
        let _ = self
            .model
            .update_range_style(&area, "alignment.horizontal", align_str);
        cx.emit(SpreadsheetEvent::UndoStateChanged);
        cx.notify();
    }

    /// Set number format on the active cell or selected range.
    pub fn set_number_format(
        &mut self,
        row: usize,
        col: usize,
        number_format: NumberFormat,
        cx: &mut Context<Self>,
    ) {
        let num_fmt = number_format.to_ironcalc();
        let (r0, c0, r1, c1) = self.effective_range(row, col);
        let area = self.range_area(r0, c0, r1, c1);
        let _ = self.model.update_range_style(&area, "num_fmt", &num_fmt);
        cx.emit(SpreadsheetEvent::UndoStateChanged);
        cx.notify();
    }

    /// Toggle underline on the active cell or selected range.
    pub fn toggle_underline(&mut self, row: usize, col: usize, cx: &mut Context<Self>) {
        let style = self
            .model
            .get_cell_style(self.active_sheet(), (row + 1) as i32, (col + 1) as i32)
            .ok();
        let underline = !style.map(|s| s.font.u).unwrap_or(false);
        let (r0, c0, r1, c1) = self.effective_range(row, col);
        let area = self.range_area(r0, c0, r1, c1);
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

    /// Get font size for a cell (IronCalc default is 11).
    pub fn font_size(&self, row: usize, col: usize) -> i32 {
        self.model
            .get_cell_style(self.active_sheet(), (row + 1) as i32, (col + 1) as i32)
            .map(|s| s.font.sz)
            .unwrap_or(11)
    }

    /// Change font size on the active cell or selected range by a delta.
    /// Positive = bigger, negative = smaller.
    pub fn change_font_size(&mut self, row: usize, col: usize, delta: i32, cx: &mut Context<Self>) {
        let (r0, c0, r1, c1) = self.effective_range(row, col);
        let area = self.range_area(r0, c0, r1, c1);
        let _ = self
            .model
            .update_range_style(&area, "font.size_delta", &delta.to_string());
        cx.emit(SpreadsheetEvent::UndoStateChanged);
        cx.notify();
    }

    /// Set text color on the active cell or selected range.
    pub fn set_text_color(&mut self, row: usize, col: usize, color: Hsla, cx: &mut Context<Self>) {
        let (r0, c0, r1, c1) = self.effective_range(row, col);
        let area = self.range_area(r0, c0, r1, c1);
        let _ = self
            .model
            .update_range_style(&area, "font.color", &hsla_to_hex(color));
        cx.emit(SpreadsheetEvent::UndoStateChanged);
        cx.notify();
    }

    /// Set background color on the active cell or selected range.
    pub fn set_bg_color(&mut self, row: usize, col: usize, color: Hsla, cx: &mut Context<Self>) {
        let (r0, c0, r1, c1) = self.effective_range(row, col);
        let area = self.range_area(r0, c0, r1, c1);
        let _ = self
            .model
            .update_range_style(&area, "fill.bg_color", &hsla_to_hex(color));
        cx.emit(SpreadsheetEvent::UndoStateChanged);
        cx.notify();
    }

    /// Toggle wrap text on the active cell or selected range.
    pub fn toggle_wrap_text(&mut self, row: usize, col: usize, cx: &mut Context<Self>) {
        let wrap = self
            .model
            .get_cell_style(self.active_sheet(), (row + 1) as i32, (col + 1) as i32)
            .ok()
            .and_then(|s| s.alignment.map(|a| a.wrap_text))
            .unwrap_or(false);
        let (r0, c0, r1, c1) = self.effective_range(row, col);
        let area = self.range_area(r0, c0, r1, c1);
        let _ = self.model.update_range_style(
            &area,
            "alignment.wrap_text",
            if wrap { "false" } else { "true" },
        );
        cx.emit(SpreadsheetEvent::UndoStateChanged);
        cx.notify();
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
        let area = self.range_area(r0, c0, r1, c1);
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
        let area = self.range_area(r0, c0, r1, c1);
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
        let area = self.range_area(r0, c0, r1, c1);
        let _ = self.model.range_clear_all(&area);
        self.model.evaluate();
        cx.emit(SpreadsheetEvent::UndoStateChanged);
        cx.notify();
    }

    // ── Column / row sizing ──────────────────────────────────────────────────

    /// Set column width in IronCalc (1-indexed column, width in points).
    pub fn set_column_width(&mut self, col: usize, width: f64) {
        let sheet = self.active_sheet();
        let _ = self
            .model
            .set_columns_width(sheet, (col + 1) as i32, (col + 1) as i32, width);
    }

    /// Set row height in IronCalc (0-indexed row, height in points).
    pub fn set_row_height(&mut self, row: usize, height: f64) {
        let sheet = self.active_sheet();
        let _ = self
            .model
            .set_rows_height(sheet, (row + 1) as i32, (row + 1) as i32, height);
    }

    // ── Sheet visibility ────────────────────────────────────────────────────

    pub fn hide_sheet(&mut self, sheet_index: u32, cx: &mut Context<Self>) {
        if let Err(e) = self.model.hide_sheet(sheet_index) {
            warn!("spreadsheet: failed to hide sheet {sheet_index}: {e}");
            return;
        }
        cx.emit(SpreadsheetEvent::SheetChanged);
        cx.notify();
    }

    pub fn unhide_sheet(&mut self, sheet_index: u32, cx: &mut Context<Self>) {
        if let Err(e) = self.model.unhide_sheet(sheet_index) {
            warn!("spreadsheet: failed to unhide sheet {sheet_index}: {e}");
            return;
        }
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

    /// Start editing a cell with an initial value (e.g. from a typed character).
    /// Replaces existing content rather than appending.
    pub fn start_editing_with(
        &mut self,
        row: usize,
        col: usize,
        initial: impl Into<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if row >= self.rows || col >= self.cols {
            return;
        }
        let initial = initial.into();
        self.editing_cell = Some((row, col));
        self.input.update(cx, |state, cx| {
            state.set_value(initial, window, cx);
            state.focus(window, cx);
        });
        cx.emit(SpreadsheetEvent::EditingStarted { row, col });
        cx.notify();
    }

    pub fn commit_edit(&mut self, cx: &mut Context<Self>) {
        if let Some((row, col)) = self.editing_cell.take() {
            let raw = self.input.read(cx).value().to_string();
            self.ensure_bounds(row, col);
            let sheet = self.active_sheet();
            let _ = self
                .model
                .set_user_input(sheet, (row + 1) as i32, (col + 1) as i32, &raw);
            self.model.evaluate();
            self.clear_formula_highlights();
            cx.emit(SpreadsheetEvent::CellChanged { row, col });
            cx.emit(SpreadsheetEvent::EditingCommitted { row, col });
            cx.emit(SpreadsheetEvent::UndoStateChanged);
        }
        cx.notify();
    }

    pub fn cancel_edit(&mut self, cx: &mut Context<Self>) {
        self.editing_cell = None;
        self.clear_formula_highlights();
        cx.emit(SpreadsheetEvent::EditingCancelled);
        cx.notify();
    }

    // ── Row / column operations ──────────────────────────────────────────────

    /// Append a new row at the bottom of the grid.
    pub fn add_row(&mut self, cx: &mut Context<Self>) {
        let sheet = self.active_sheet();
        if let Err(e) = self.model.insert_rows(sheet, (self.rows + 1) as i32, 1) {
            warn!("spreadsheet: failed to add row: {e}");
            return;
        }
        self.rows += 1;
        cx.emit(SpreadsheetEvent::UndoStateChanged);
        cx.notify();
    }

    /// Insert a new row before `before_row` (0-based).
    pub fn insert_row(&mut self, before_row: usize, cx: &mut Context<Self>) {
        let sheet = self.active_sheet();
        if let Err(e) = self.model.insert_rows(sheet, (before_row + 1) as i32, 1) {
            warn!("spreadsheet: failed to insert row at {before_row}: {e}");
            return;
        }
        self.rows += 1;
        cx.emit(SpreadsheetEvent::UndoStateChanged);
        cx.notify();
    }

    /// Delete the row at `row` (0-based). Minimum 1 row enforced.
    pub fn delete_row(&mut self, row: usize, cx: &mut Context<Self>) {
        if self.rows <= 1 {
            return;
        }
        let sheet = self.active_sheet();
        if let Err(e) = self.model.delete_rows(sheet, (row + 1) as i32, 1) {
            warn!("spreadsheet: failed to delete row {row}: {e}");
            return;
        }
        self.rows = self.rows.saturating_sub(1);
        cx.emit(SpreadsheetEvent::UndoStateChanged);
        cx.notify();
    }

    /// Insert a new column before `before_col` (0-based).
    pub fn insert_col(&mut self, before_col: usize, cx: &mut Context<Self>) {
        let sheet = self.active_sheet();
        if let Err(e) = self.model.insert_columns(sheet, (before_col + 1) as i32, 1) {
            warn!("spreadsheet: failed to insert column at {before_col}: {e}");
            return;
        }
        self.cols += 1;
        cx.emit(SpreadsheetEvent::UndoStateChanged);
        cx.notify();
    }

    /// Delete the column at `col` (0-based). Minimum 1 column enforced.
    pub fn delete_col(&mut self, col: usize, cx: &mut Context<Self>) {
        if self.cols <= 1 {
            return;
        }
        let sheet = self.active_sheet();
        if let Err(e) = self.model.delete_columns(sheet, (col + 1) as i32, 1) {
            warn!("spreadsheet: failed to delete column {col}: {e}");
            return;
        }
        self.cols = self.cols.saturating_sub(1);
        cx.emit(SpreadsheetEvent::UndoStateChanged);
        cx.notify();
    }
}
