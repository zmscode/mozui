# Spreadsheet Phase 2 — Implementation Plan

Picks up where SHEET.md left off. Fixes structural issues, fills gaps, adds missing features.

---

## Phase 1: Data Layer Fixes (foundation — do first)

### 1.1 Range-aware formatting operations
**Problem:** `toggle_bold`, `toggle_italic`, `set_align`, `set_number_format`, `toggle_underline`
all operate on a single `(row, col)`. Toolbar applies to `selected_data_cell()` only — range
selection ignored.

**Fix:** Add range variants that iterate `selected_range()` or accept `(r0, c0, r1, c1)`:
```rust
pub fn toggle_bold_range(&mut self, r0: usize, c0: usize, r1: usize, c1: usize, cx: &mut Context<Self>)
pub fn toggle_italic_range(...)
pub fn set_align_range(...)
pub fn set_number_format_range(...)
pub fn toggle_underline_range(...)
```
Wrap each batch in `pause_evaluation()` / `resume_evaluation()`.

**Files:** `data.rs`, `components_spreadsheet.rs` (toolbar handlers)

### 1.2 Bulk evaluation batching
**Problem:** `set_cell()` calls `evaluate()` per call. Paste of 100 cells = 100 evaluations.

**Fix:** Add `set_cell_batch` or expose `pause_evaluation()`/`resume_evaluation()` wrappers.
Update `paste_tsv` and `cut_selection_as_tsv` to use batching (cut already does one eval — good).

**Files:** `data.rs`

### 1.3 Auto-grow grid boundaries
**Problem:** `rows`/`cols` are fixed at construction. User can't type past the boundary.
`delete_row` can hit 0.

**Fix:**
- `set_cell` / `paste_tsv` / `commit_edit`: if target row/col >= current bounds, grow `rows`/`cols`
- Minimum 1 row, 1 col enforced on delete
- Add `ensure_bounds(row, col)` helper

**Files:** `data.rs`

### 1.4 Clipboard: preserve formulas on copy
**Problem:** `copy_selection_as_tsv` uses `get_formatted_cell_value` — copies display, not formula.

**Fix:** Use `get_cell_content` (raw value / formula) for TSV export. Paste already uses
`paste_csv_string` which handles `=` prefixed strings as formulas.

**Files:** `data.rs` — `copy_selection_as_tsv`, `cut_selection_as_tsv`

### 1.5 Error propagation
**Problem:** Every IronCalc call discards `Result` with `let _ =`.

**Fix:** Log errors via `log::warn!` for operations that can legitimately fail (delete last sheet,
invalid range). Return `Result` from operations where callers should handle failure
(`rename_sheet`, `set_user_input`).

**Files:** `data.rs`

---

## Phase 2: Keyboard & Editing UX

### 2.1 Type-to-edit
**Problem:** Must double-click to start editing. Typing a character in grid does nothing.

**Fix:** In `TableDelegate` key handling (or action on the spreadsheet root), intercept printable
key events. On keypress:
- If not editing: `start_editing(row, col)`, pre-fill input with typed character
- `=` keypress: start editing + focus formula bar instead of inline input

**Files:** `delegate.rs`, `components_spreadsheet.rs`

### 2.2 Arrow-key commit + move
**Problem:** After editing, Enter commits and that's it. Standard spreadsheets: Enter moves down,
Tab moves right.

**Fix:** On `InputEvent::PressEnter` in editing state:
- Commit edit
- Move selection down one row (or right for Tab)
- Already partially done (Enter moves down) per SHEET.md — verify + add Tab

**Files:** `data.rs`, `components_spreadsheet.rs`

### 2.3 Escape cancels edit
Verify Escape during inline edit calls `cancel_edit` and restores original value. Wire if missing.

**Files:** `data.rs`, `delegate.rs`

---

## Phase 3: Composable Panel System

### 3.1 `Spreadsheet` builder struct
Create `crates/mozui-components/src/spreadsheet/spreadsheet.rs`:

```rust
pub struct Spreadsheet {
    data: Entity<SpreadsheetData>,
    table: Entity<TableState<SpreadsheetTableDelegate>>,
    show_toolbar: bool,
    show_formula_bar: bool,
    show_column_headers: bool,  // already from DataTable
    show_row_headers: bool,     // gutter column
    show_sheet_tabs: bool,
    show_undo_redo: bool,       // part of toolbar
}

impl Spreadsheet {
    pub fn new(data: Entity<SpreadsheetData>, window: &mut Window, cx: &mut App) -> Self { ... }

    // Opt-in panels
    pub fn with_toolbar(mut self) -> Self { self.show_toolbar = true; self }
    pub fn with_formula_bar(mut self) -> Self { self.show_formula_bar = true; self }
    pub fn with_column_headers(mut self) -> Self { self.show_column_headers = true; self }
    pub fn with_row_headers(mut self) -> Self { self.show_row_headers = true; self }
    pub fn with_sheet_tab_bar(mut self) -> Self { self.show_sheet_tabs = true; self }
    pub fn with_undo_redo(mut self) -> Self { self.show_undo_redo = true; self }

    // Bundles
    pub fn minimal(data, window, cx) -> Self { Self::new(data, window, cx) }
    pub fn standard(data, window, cx) -> Self { Self::new(...).with_column_headers().with_row_headers().with_formula_bar() }
    pub fn full(data, window, cx) -> Self { /* all panels */ }
}

impl Render for Spreadsheet { ... }
```

### 3.2 Extract panel render functions
Move toolbar, formula bar, sheet tab bar rendering from `components_spreadsheet.rs` example
into the `Spreadsheet` struct as private render methods. Example becomes thin wrapper.

### 3.3 Internalize action wiring
All action handlers (undo, redo, copy, paste, insert/delete row/col, formatting) move into
`Spreadsheet`. Consumer only needs to provide `SpreadsheetData` entity.

**Files:** New `spreadsheet.rs`, update `mod.rs`, simplify example

---

## Phase 4: Missing Toolbar Features

### 4.1 Font size control
Dropdown or +/- buttons. Wire to `update_range_style(&area, "font.size_delta", value)`.
Apply to range if selected.

### 4.2 Text color picker
Color palette popup. Wire to `update_range_style(&area, "font.color", hex)`.
`hsla_to_hex` already exists.

### 4.3 Background color picker
Same as text color, wire to `update_range_style(&area, "fill.bg_color", hex)`.

### 4.4 Wrap text toggle
Wire to `update_range_style(&area, "alignment.wrap_text", "true"/"false")`.

### 4.5 Merge cells
Need to investigate IronCalc merge API. May require `update_range_style` or dedicated method.

### 4.6 Borders picker
Wire to `set_area_with_border(sheet, area, border)`. Need border UI (dropdown with
top/bottom/left/right/all/none).

**Files:** `data.rs` (new wrappers), `spreadsheet.rs` (toolbar rendering)

---

## Phase 5: Header & Tab Improvements

### 5.1 Column header resize
Column headers already come from `DataTable` which has resize handles. Verify IronCalc
`set_columns_width` is called on resize. Add `on_resize` callback to sync widths.

### 5.2 Row header resize
Row headers (gutter) don't support resize natively. Need drag handle on row borders.
Wire to `set_rows_height`.

### 5.3 Header right-click menus
Column header: insert col, delete col, hide col.
Row header: insert row, delete row, hide row.

### 5.4 Sheet tab color
Wire to `set_sheet_color(sheet, color)`. Add color dot/indicator on tab.

### 5.5 Sheet tab drag reorder
Drag-and-drop reorder. IronCalc doesn't have a reorder API — may need to track order externally
or use move_row/column patterns.

**Files:** `delegate.rs`, `spreadsheet.rs`, `data.rs`

---

## Phase 6: Formula Bar Enhancements

### 6.1 Editable cell reference box
Make cell ref box (`A1`, `B7:D10`) an editable input. On Enter, parse and jump to that cell/range.

### 6.2 Range highlighting
When formula bar shows `=SUM(A1:B5)`, highlight cells A1:B5 in grid with colored border.
Requires parsing formula refs from input text.

**Files:** `spreadsheet.rs`

---

## Phase 7: Selection Model Cleanup

### 7.1 Single source of truth
**Problem:** Selection in both `TableState::selected_cell/selected_range` and
`SpreadsheetData` (via IronCalc `set_selected_cell`/`set_selected_range`). Two sources diverge
on undo/paste.

**Fix:** Make `SpreadsheetData` the authority. After undo/redo/paste, read selection back from
IronCalc and push to `TableState`. Add `sync_selection_to_table()` helper.

**Files:** `data.rs`, `spreadsheet.rs`

---

## Phase 8: Viewport & Performance

### 8.1 Viewport sync with IronCalc
Call `set_window_width()` / `set_window_height()` / `set_top_left_visible_cell()` on scroll
and resize. Enables IronCalc to optimize evaluation for visible area.

### 8.2 Lazy row/col count
Instead of fixed `rows`/`cols`, use IronCalc's edge-finding APIs
(`get_last_non_empty_in_row_before_column`, etc.) to determine actual data extent.
Render a buffer beyond (e.g., 50 extra rows, 10 extra cols).

**Files:** `data.rs`, `delegate.rs`

---

## Execution Order

| Step | Phase | Status | Notes |
|------|-------|--------|-------|
| 1 | 1.1–1.5 | ✅ Done | Range formatting, batching, auto-grow, clipboard, error logging |
| 2 | 2.1–2.3 | ✅ Done | Type-to-edit, Tab commit, backspace/delete |
| 3 | 3.1–3.3 | ✅ Done | Composable Spreadsheet builder, example simplified to ~270 lines |
| 4 | 7.1 | ✅ Done | IronCalc as single source of truth, sync_selection_from_data |
| 5 | 4.1–4.6 | ✅ Partial | Font size ±, text/bg color presets, wrap text. Merge/borders deferred |
| 6 | 5.1–5.5 | ✅ Partial | Column resize sync, sheet tab color. Row resize/header menus/drag reorder deferred (need table module changes) |
| 7 | 6.1–6.2 | ✅ Done | Editable cell ref box with jump-to. Formula range highlighting with IronCalc tokenizer + colored borders |
| 8 | 8.1–8.2 | ✅ Partial | Lazy row/col count with data extent + buffer. Viewport sync skipped (not needed with mozui virtual scroll) |

### Remaining work (deferred items)
- **4.5 Merge cells** — investigate IronCalc merge API
- **4.6 Borders picker** — `set_area_with_border` UI
- **5.2 Row header resize** — needs table module drag handle
- **5.3 Header right-click menus** — needs `TableDelegate` trait extension
- **5.5 Sheet tab drag reorder** — no IronCalc API
