# Spreadsheet Component — Feature Roadmap

IronCalc (`ironcalc_base 0.7.1`) is the engine.
This document tracks what needs building to expose that power through a composable UI.

---

## Architecture: Opt-in Panel System

Spreadsheet composed of discrete panels. Each panel independent, opt-in via builder.

```rust
Spreadsheet::new(data)
    .with_toolbar()
    .with_formula_bar()
    .with_column_headers()
    .with_row_headers()
    .with_sheet_tab_bar()
    .with_undo_redo()
```

Default: grid only. Add panels as needed.

---

## Panels

### 1. Grid (`SpreadsheetGrid`) — Core, always present
The scrollable cell canvas. Horizontal + vertical scroll. All interaction roots here.

**Done:**
- [x] Keyboard navigation — arrow keys, Tab/Shift+Tab, Enter moves down
- [x] Range selection — Shift+click, Shift+arrow, click-drag
- [x] Visual selection highlight (single cell + multi-cell range)
- [x] Context menu (right-click): insert row/col, delete row/col, clear contents, clear formatting
- [x] Error cell display — distinct style for `#DIV/0!`, `#REF!`, `#NAME?`, etc.
- [x] Clipboard — Cmd+C/X/V → `copy_to_clipboard` / `paste_csv_string`
- [x] Freeze panes — `set_frozen_rows_count` / `set_frozen_columns_count`
- [x] Show/hide grid lines → `set_show_grid_lines`
- [x] Vertical cell borders between columns
- [x] Delete/Backspace key clears selected cells → `range_clear_contents`

**Remaining:**
- [ ] Row/column resize via drag handles on header borders → `set_columns_width` / `set_rows_height`
- [ ] Auto-fill handle (drag bottom-right corner of selection) → `auto_fill_rows` / `auto_fill_columns`

### 2. Column Header (`SpreadsheetColumnHeader`) — Opt-in
Horizontal bar: A, B, C… letters above grid columns.

**Done:**
- [x] Render column labels synced to horizontal scroll position
- [x] Click to select entire column
- [x] Highlight selected column(s)

**Remaining:**
- [ ] Drag border to resize column → `set_columns_width`
- [ ] Right-click: insert col, delete col, hide col

### 3. Row Header (`SpreadsheetRowHeader`) — Opt-in
Vertical bar: 1, 2, 3… numbers left of grid rows.

**Done:**
- [x] Render row numbers synced to vertical scroll position
- [x] Click to select entire row
- [x] Highlight selected row(s)

**Remaining:**
- [ ] Drag border to resize row → `set_rows_height`
- [ ] Right-click: insert row, delete row, hide row

### 4. Formula Bar (`SpreadsheetFormulaBar`) — Opt-in
Cell reference box + `fx` label + editable formula input.

**Done:**
- [x] Cell reference box — shows current cell address (e.g. `B7`)
- [x] Editable formula input — editing in formula bar commits to active cell on Enter
- [x] Formula input syncs to active cell on selection change

**Remaining:**
- [ ] Editable cell reference box for navigation (type `C5` to jump)
- [ ] Formula input receives focus on `=` keypress in grid
- [ ] Range highlight in grid when formula bar shows range refs

### 5. Toolbar (`SpreadsheetToolbar`) — Opt-in
Formatting + action buttons above formula bar.

**Done:**
- [x] Bold/italic/underline toggle → `update_range_style` (font.b, font.i, font.u)
- [x] Alignment — left/center/right → `update_range_style` (alignment.horizontal)
- [x] Number format selector — General, Currency, Percent → `update_range_style` (num_fmt)
- [x] Grid lines toggle → `set_show_grid_lines`
- [x] Freeze/unfreeze panes → `set_frozen_rows_count` / `set_frozen_columns_count`
- [x] Add/delete row buttons
- [x] Clear formatting button → `range_clear_formatting`

**Remaining:**
- [ ] Font size control → `update_range_style` (font.size_delta)
- [ ] Text color picker → `update_range_style` (font.color)
- [ ] Background color picker → `update_range_style` (fill.bg_color)
- [ ] Wrap text toggle → `update_range_style` (alignment.wrap_text)
- [ ] Merge cells button
- [ ] Borders picker → `set_area_with_border`

### 6. Undo/Redo Controls (`SpreadsheetUndoRedo`) — Opt-in

**Done:**
- [x] Cmd+Z → `model.undo()` (guarded with `can_undo()`)
- [x] Cmd+Shift+Z → `model.redo()` (guarded with `can_redo()`)
- [x] Toolbar ↩/↪ buttons with disabled state
- [x] `UndoStateChanged` event emitted on all mutations

### 7. Sheet Tab Bar (`SpreadsheetSheetTabBar`) — Opt-in
Bottom bar with sheet tabs.

**Done:**
- [x] Render all visible sheets from `get_worksheets_properties()`
- [x] Click tab → `set_active_sheet` + grid refresh
- [x] `+` button → `new_sheet()`
- [x] Double-click tab → inline rename → `rename_sheet`
- [x] Right-click tab menu: rename, hide, unhide, delete
- [x] Hide/unhide sheets → `hide_sheet` / `unhide_sheet`

**Remaining:**
- [ ] Set tab color → `set_sheet_color`
- [ ] Drag to reorder sheets

---

## Data Layer

**Done:**
- [x] All formatting via `update_range_style` / `get_cell_style` (participates in undo/redo)
- [x] `add_row` wired to `insert_rows` so IronCalc adjusts formula refs
- [x] `insert_row`, `delete_row`, `insert_col`, `delete_col` implemented
- [x] Selection synced to IronCalc via `set_selected_cell` / `set_selected_range`
- [x] Clipboard via manual TSV build from `get_formatted_cell_value`
- [x] `clear_formatting_range`, `clear_contents_range`, `clear_all_range`
- [x] `set_locale` / `set_timezone` / `locale()` / `timezone()` exposed

**Remaining:**
- [ ] Locale/timezone config in constructor (currently hardcoded `"en"/"UTC"/"en"`)

---

## Serialization / Persistence

**Done:**
- [x] `SpreadsheetData::save() -> Vec<u8>` → `model.to_bytes()`
- [x] `SpreadsheetData::load(bytes, rows, cols, window, cx)` → `UserModel::from_bytes(bytes, lang)`

**Remaining:**
- [ ] File open/save plumbing in host app (drag-drop, file picker)
- [ ] Auto-save / dirty state tracking

---

## Unexposed IronCalc API

The following `UserModel` capabilities exist but have no wrapper in `SpreadsheetData` yet:

### Cell Operations
- `get_cell_type(sheet, row, col)` — returns `CellType` enum (number/text/bool/error)
- `range_clear_all(area)` — clear contents + formatting *(wrapper exists)*
- `move_column_action(sheet, col, delta)` — reorder columns
- `move_row_action(sheet, row, delta)` — reorder rows

### Layout
- `set_columns_width(sheet, col, count, width)` — set column widths
- `set_rows_height(sheet, row, count, height)` — set row heights
- `get_column_width(sheet, col)` / `get_row_height(sheet, row)` — read dimensions

### Auto-fill
- `auto_fill_rows(source_area, to_row)` — extend pattern down/up
- `auto_fill_columns(source_area, to_column)` — extend pattern left/right

### Clipboard (internal)
- `copy_to_clipboard()` — returns `Clipboard` struct (fields are `pub(crate)`, not accessible)
- `paste_from_clipboard(clipboard)` — paste with full style info
- `on_paste_styles(styles)` — paste formatting only

### Borders
- `set_area_with_border(sheet, area, border)` — cell border styling

### Named Ranges
- `get_defined_name_list()` — list all named ranges
- `new_defined_name(name, scope, formula)` — create named range
- `update_defined_name(name, scope, formula)` — modify named range
- `delete_defined_name(name, scope)` — remove named range
- `is_valid_defined_name(name, scope)` — validate name

### Navigation (UI helpers)
- `on_expand_selected_range(key)` — Shift+Arrow selection via engine
- `on_arrow_right/left/up/down()` — single-cell movement via engine
- `on_page_down()` / `on_page_up()` — page navigation
- `on_area_selecting(row, col)` — mouse-drag selection
- `on_navigate_to_edge_in_direction(key)` — Ctrl+Arrow jump to edge
- `set_top_left_visible_cell(sheet, row, col)` — scroll position sync
- `get_scroll_x()` / `get_scroll_y()` — read scroll offsets
- `set_window_width/height()` — inform engine of viewport
- `get_last_non_empty_in_row_before_column()` — find data edges
- `get_first_non_empty_in_row_after_column()` — find data edges

### Sheet Operations
- `set_sheet_color(sheet, color)` — tab color

### Locale / Settings
- `get_fmt_settings()` — format settings for current locale

### Collaboration
- `flush_send_queue()` — get pending diffs for sync
- `apply_external_diffs(bytes)` — apply remote changes
- `pause_evaluation()` / `resume_evaluation()` — batch operations

### Workbook Metadata
- `get_name()` / `set_name(name)` — workbook name

---

## Potential Default Component Bundles

| Bundle | Panels Included |
|--------|----------------|
| `Spreadsheet::minimal()` | Grid only |
| `Spreadsheet::standard()` | Grid + Column Header + Row Header + Formula Bar |
| `Spreadsheet::full()` | All panels |
| `Spreadsheet::editor()` | Grid + Column Header + Row Header + Formula Bar + Toolbar + Undo/Redo + Sheet Tab Bar |
