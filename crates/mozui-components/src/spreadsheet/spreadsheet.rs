//! Composable spreadsheet component with opt-in panels.

use mozui::{
    AppContext as _, ClipboardItem, Context, Entity, HighlightStyle, Hsla, InteractiveElement as _,
    IntoElement, KeyDownEvent, ParentElement as _, Render, SharedString,
    StatefulInteractiveElement as _, Styled as _, Subscription, TextAlign, Window, div, hsla,
    prelude::FluentBuilder as _, px,
};

use crate::{
    ActiveTheme as _, Disableable as _, InteractiveElementExt as _, Sizable, StyledExt as _,
    actions::{Redo, Undo},
    button::{Button, ButtonVariants as _},
    input::{Input, InputEvent, InputState},
    menu::ContextMenuExt as _,
    table::{DataTable, TableEvent, TableState},
};

use super::{
    ClearContents, ClearFormatting, Copy, Cut, DeleteCol, DeleteRow, DeleteSheet, HideSheet,
    InsertColLeft, InsertColRight, InsertRowAbove, InsertRowBelow, NumberFormat, Paste,
    RenameSheet, ToggleGridLines, UnhideSheet, col_to_letter,
    data::{CellFormat, SpreadsheetData, formula_bar_highlights},
    delegate::SpreadsheetTableDelegate,
};

// ── Builder config ───────────────────────────────────────────────────────────

/// Composable spreadsheet widget.
///
/// Default: grid only. Add panels via builder methods.
///
/// ```rust,ignore
/// Spreadsheet::new(data, window, cx)
///     .with_toolbar()
///     .with_formula_bar()
///     .with_sheet_tab_bar()
/// ```
pub struct Spreadsheet {
    data: Entity<SpreadsheetData>,
    table: Entity<TableState<SpreadsheetTableDelegate>>,
    selected_table_cell: Option<(usize, usize)>,
    renaming_sheet: Option<(u32, String)>,
    context_menu_sheet: Option<u32>,
    formula_input: Entity<InputState>,
    cell_ref_input: Entity<InputState>,
    formula_synced_cell: Option<(usize, usize)>,
    _subs: Vec<Subscription>,

    // Panel flags
    show_toolbar: bool,
    show_formula_bar: bool,
    show_sheet_tabs: bool,
}

impl Spreadsheet {
    pub fn new(data: Entity<SpreadsheetData>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let table =
            cx.new(|cx| SpreadsheetTableDelegate::into_table_state(data.clone(), window, cx));

        let input = data.read(cx).input.clone();
        let formula_input = cx.new(|cx| InputState::new(window, cx));
        let cell_ref_input = cx.new(|cx| InputState::new(window, cx));

        let subs = vec![
            cx.observe(&data, {
                let table = table.clone();
                move |_, _, cx| {
                    table.update(cx, |_, cx| cx.notify());
                }
            }),
            cx.observe(&input, {
                let table = table.clone();
                let data = data.clone();
                let input = input.clone();
                move |_, _, cx| {
                    // Update formula highlights when cell input text changes during editing.
                    let is_editing = data.read(cx).editing_cell.is_some();
                    if is_editing {
                        let text = input.read(cx).value().to_string();
                        data.update(cx, |d, _| d.update_formula_highlights(&text));
                    }
                    table.update(cx, |_, cx| cx.notify());
                }
            }),
            cx.observe(&formula_input, {
                let data = data.clone();
                let table = table.clone();
                let formula_input = formula_input.clone();
                move |_, _, cx| {
                    // Update grid highlights when formula bar text changes.
                    let text = formula_input.read(cx).value().to_string();
                    data.update(cx, |d, _| d.update_formula_highlights(&text));
                    table.update(cx, |_, cx| cx.notify());
                }
            }),
            cx.subscribe(&formula_input, {
                let data = data.clone();
                let table = table.clone();
                move |this, _input, event: &InputEvent, cx| match event {
                    InputEvent::PressEnter { .. } | InputEvent::Blur => {
                        if let Some((row, col)) = this.selected_data_cell() {
                            let val = this.formula_input.read(cx).value().to_string();
                            data.update(cx, |d, _cx| {
                                d.set_cell(row, col, &val);
                                d.clear_formula_highlights();
                            });
                            table.update(cx, |_, cx| cx.notify());
                        }
                    }
                    _ => {}
                }
            }),
            cx.subscribe(&cell_ref_input, {
                let data = data.clone();
                let table = table.clone();
                move |this, _input, event: &InputEvent, cx| match event {
                    InputEvent::PressEnter { .. } => {
                        let text = this.cell_ref_input.read(cx).value().to_string();
                        if let Some((row, col)) = parse_cell_ref(&text) {
                            data.update(cx, |d, _cx| d.set_selected_cell(row, col));
                            this.selected_table_cell = Some((row, col + 1));
                            table.update(cx, |t, cx| t.set_selected_cell(row, col + 1, cx));
                        }
                    }
                    _ => {}
                }
            }),
            cx.subscribe(&table, {
                let data = data.clone();
                move |this, _table, event: &TableEvent, cx| match event {
                    TableEvent::SelectCell(row, col) => {
                        this.selected_table_cell = Some((*row, *col));
                        if *col > 0 {
                            data.update(cx, |d, _cx| d.set_selected_cell(*row, *col - 1));
                        }
                        cx.notify();
                    }
                    TableEvent::SelectRange(r0, c0, r1, c1) => {
                        let dc0 = c0.saturating_sub(1);
                        let dc1 = c1.saturating_sub(1);
                        data.update(cx, |d, _cx| {
                            // Set anchor first (drag selection doesn't fire SelectCell).
                            d.set_selected_cell(*r0, dc0);
                            d.extend_selection_to(*r1, dc1);
                        });
                        cx.notify();
                    }
                    TableEvent::RightClickedCell(row, col) => {
                        this.selected_table_cell = Some((*row, *col));
                        cx.notify();
                    }
                    TableEvent::ColumnWidthsChanged(widths) => {
                        // Sync column widths to IronCalc (skip col 0 = gutter).
                        data.update(cx, |d, _cx| {
                            for (i, w) in widths.iter().enumerate() {
                                if i > 0 {
                                    d.set_column_width(i - 1, f32::from(*w) as f64);
                                }
                            }
                        });
                    }
                    _ => {}
                }
            }),
        ];

        Self {
            data,
            table,
            selected_table_cell: None,
            renaming_sheet: None,
            context_menu_sheet: None,
            formula_input,
            cell_ref_input,
            formula_synced_cell: None,
            _subs: subs,
            show_toolbar: false,
            show_formula_bar: false,
            show_sheet_tabs: false,
        }
    }

    // ── Builder methods ─────────────────────────────────────────────────

    pub fn with_toolbar(mut self) -> Self {
        self.show_toolbar = true;
        self
    }

    pub fn with_formula_bar(mut self) -> Self {
        self.show_formula_bar = true;
        self
    }

    pub fn with_sheet_tab_bar(mut self) -> Self {
        self.show_sheet_tabs = true;
        self
    }

    /// Grid + column headers + row headers + formula bar.
    pub fn standard(
        data: Entity<SpreadsheetData>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        Self::new(data, window, cx).with_formula_bar()
    }

    /// All panels enabled.
    pub fn full(
        data: Entity<SpreadsheetData>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        Self::new(data, window, cx)
            .with_toolbar()
            .with_formula_bar()
            .with_sheet_tab_bar()
    }

    // ── Helpers ─────────────────────────────────────────────────────────

    fn selected_data_cell(&self) -> Option<(usize, usize)> {
        match self.selected_table_cell {
            Some((row, col)) if col > 0 => Some((row, col - 1)),
            _ => None,
        }
    }

    fn cell_ref(&self, cx: &Context<Self>) -> SharedString {
        let range = self.data.read(cx).selected_range();
        if let Some((r0, c0, r1, c1)) = range {
            format!(
                "{}{}:{}{}",
                col_to_letter(c0),
                r0 + 1,
                col_to_letter(c1),
                r1 + 1,
            )
            .into()
        } else {
            match self.selected_data_cell() {
                Some((row, col)) => format!("{}{}", col_to_letter(col), row + 1).into(),
                None => "".into(),
            }
        }
    }

    fn selected_format(&self, cx: &Context<Self>) -> CellFormat {
        match self.selected_data_cell() {
            Some((row, col)) => self.data.read(cx).format(row, col),
            None => CellFormat::default(),
        }
    }

    // ── Selection sync ───────────────────────────────────────────────────

    /// Read IronCalc's selection state and push it to both `selected_table_cell`
    /// and `TableState`, making IronCalc the single source of truth.
    fn sync_selection_from_data(&mut self, cx: &mut Context<Self>) {
        let (row, col) = self.data.read(cx).selected_cell();
        let table_col = col + 1; // +1 for gutter column
        self.selected_table_cell = Some((row, table_col));

        let range = self.data.read(cx).selected_range();
        if let Some((r0, c0, r1, c1)) = range {
            self.table.update(cx, |t, cx| {
                t.set_selected_range(r0, c0 + 1, r1, c1 + 1, cx);
            });
        } else {
            self.table.update(cx, |t, cx| {
                t.set_selected_cell(row, table_col, cx);
            });
        }
    }

    // ── Actions ─────────────────────────────────────────────────────────

    fn on_undo(&mut self, _: &Undo, _w: &mut Window, cx: &mut Context<Self>) {
        self.data.update(cx, |d, cx| d.undo(cx));
        self.sync_selection_from_data(cx);
    }

    fn on_redo(&mut self, _: &Redo, _w: &mut Window, cx: &mut Context<Self>) {
        self.data.update(cx, |d, cx| d.redo(cx));
        self.sync_selection_from_data(cx);
    }

    fn on_copy(&mut self, _: &Copy, _w: &mut Window, cx: &mut Context<Self>) {
        let tsv = self.data.read(cx).copy_selection_as_tsv();
        cx.write_to_clipboard(ClipboardItem::new_string(tsv));
    }

    fn on_cut(&mut self, _: &Cut, _w: &mut Window, cx: &mut Context<Self>) {
        let tsv = self.data.update(cx, |d, cx| d.cut_selection_as_tsv(cx));
        cx.write_to_clipboard(ClipboardItem::new_string(tsv));
        self.sync_selection_from_data(cx);
    }

    fn on_paste(&mut self, _: &Paste, _w: &mut Window, cx: &mut Context<Self>) {
        if let Some(item) = cx.read_from_clipboard() {
            if let Some(text) = item.text() {
                self.data.update(cx, |d, cx| d.paste_tsv(&text, cx));
                self.sync_selection_from_data(cx);
            }
        }
    }

    fn on_insert_row_above(&mut self, _: &InsertRowAbove, _w: &mut Window, cx: &mut Context<Self>) {
        if let Some((row, _)) = self.selected_data_cell() {
            self.data.update(cx, |d, cx| d.insert_row(row, cx));
            self.table.update(cx, |t, cx| t.refresh(cx));
        }
    }

    fn on_insert_row_below(&mut self, _: &InsertRowBelow, _w: &mut Window, cx: &mut Context<Self>) {
        if let Some((row, _)) = self.selected_data_cell() {
            self.data.update(cx, |d, cx| d.insert_row(row + 1, cx));
            self.table.update(cx, |t, cx| t.refresh(cx));
        }
    }

    fn on_delete_row(&mut self, _: &DeleteRow, _w: &mut Window, cx: &mut Context<Self>) {
        if let Some((row, _)) = self.selected_data_cell() {
            self.data.update(cx, |d, cx| d.delete_row(row, cx));
            self.selected_table_cell = None;
            self.table.update(cx, |t, cx| t.refresh(cx));
        }
    }

    fn on_insert_col_left(&mut self, _: &InsertColLeft, _w: &mut Window, cx: &mut Context<Self>) {
        if let Some((_, col)) = self.selected_data_cell() {
            self.data.update(cx, |d, cx| d.insert_col(col, cx));
            self.table.update(cx, |t, cx| t.refresh(cx));
        }
    }

    fn on_insert_col_right(&mut self, _: &InsertColRight, _w: &mut Window, cx: &mut Context<Self>) {
        if let Some((_, col)) = self.selected_data_cell() {
            self.data.update(cx, |d, cx| d.insert_col(col + 1, cx));
            self.table.update(cx, |t, cx| t.refresh(cx));
        }
    }

    fn on_delete_col(&mut self, _: &DeleteCol, _w: &mut Window, cx: &mut Context<Self>) {
        if let Some((_, col)) = self.selected_data_cell() {
            self.data.update(cx, |d, cx| d.delete_col(col, cx));
            self.selected_table_cell = None;
            self.table.update(cx, |t, cx| t.refresh(cx));
        }
    }

    fn on_toggle_grid_lines(
        &mut self,
        _: &ToggleGridLines,
        _w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let show = self.data.read(cx).show_grid_lines();
        self.data
            .update(cx, |d, cx| d.set_show_grid_lines(!show, cx));
        self.table.update(cx, |_, cx| cx.notify());
    }

    fn on_clear_formatting(
        &mut self,
        _: &ClearFormatting,
        _w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some((row, col)) = self.selected_data_cell() {
            let range = self.data.read(cx).selected_range();
            let (r0, c0, r1, c1) = range.unwrap_or((row, col, row, col));
            self.data
                .update(cx, |d, cx| d.clear_formatting_range(r0, c0, r1, c1, cx));
            self.table.update(cx, |_, cx| cx.notify());
        }
    }

    fn on_clear_contents(&mut self, _: &ClearContents, _w: &mut Window, cx: &mut Context<Self>) {
        if let Some((row, col)) = self.selected_data_cell() {
            let range = self.data.read(cx).selected_range();
            let (r0, c0, r1, c1) = range.unwrap_or((row, col, row, col));
            self.data
                .update(cx, |d, cx| d.clear_contents_range(r0, c0, r1, c1, cx));
            self.table.update(cx, |_, cx| cx.notify());
        }
    }

    // ── Toolbar formatting ──────────────────────────────────────────────

    fn on_bold(&mut self, _: &mozui::ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        if let Some((row, col)) = self.selected_data_cell() {
            self.data.update(cx, |d, cx| d.toggle_bold(row, col, cx));
        }
    }

    fn on_italic(&mut self, _: &mozui::ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        if let Some((row, col)) = self.selected_data_cell() {
            self.data.update(cx, |d, cx| d.toggle_italic(row, col, cx));
        }
    }

    fn on_underline(&mut self, _: &mozui::ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        if let Some((row, col)) = self.selected_data_cell() {
            self.data
                .update(cx, |d, cx| d.toggle_underline(row, col, cx));
        }
    }

    fn on_align(&mut self, align: TextAlign, _w: &mut Window, cx: &mut Context<Self>) {
        if let Some((row, col)) = self.selected_data_cell() {
            self.data
                .update(cx, |d, cx| d.set_align(row, col, align, cx));
        }
    }

    fn on_number_format(&mut self, fmt: NumberFormat, _w: &mut Window, cx: &mut Context<Self>) {
        if let Some((row, col)) = self.selected_data_cell() {
            self.data
                .update(cx, |d, cx| d.set_number_format(row, col, fmt, cx));
        }
    }

    fn on_font_size_delta(&mut self, delta: i32, _w: &mut Window, cx: &mut Context<Self>) {
        if let Some((row, col)) = self.selected_data_cell() {
            self.data
                .update(cx, |d, cx| d.change_font_size(row, col, delta, cx));
        }
    }

    fn on_text_color(&mut self, color: Hsla, _w: &mut Window, cx: &mut Context<Self>) {
        if let Some((row, col)) = self.selected_data_cell() {
            self.data
                .update(cx, |d, cx| d.set_text_color(row, col, color, cx));
        }
    }

    fn on_bg_color(&mut self, color: Hsla, _w: &mut Window, cx: &mut Context<Self>) {
        if let Some((row, col)) = self.selected_data_cell() {
            self.data
                .update(cx, |d, cx| d.set_bg_color(row, col, color, cx));
        }
    }

    fn on_wrap_text(&mut self, _: &mozui::ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        if let Some((row, col)) = self.selected_data_cell() {
            self.data
                .update(cx, |d, cx| d.toggle_wrap_text(row, col, cx));
        }
    }

    fn on_add_row(&mut self, _: &mozui::ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        self.data.update(cx, |d, cx| d.add_row(cx));
        self.table.update(cx, |_, cx| cx.notify());
    }

    fn on_delete_selected_row(
        &mut self,
        _: &mozui::ClickEvent,
        _w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some((row, _)) = self.selected_data_cell() {
            self.data.update(cx, |d, cx| d.delete_row(row, cx));
            self.selected_table_cell = None;
            self.table.update(cx, |_, cx| cx.notify());
        }
    }

    fn on_freeze_row(&mut self, _: &mozui::ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        if let Some((row, _)) = self.selected_data_cell() {
            self.data.update(cx, |d, cx| d.set_frozen_rows(row + 1, cx));
            self.table.update(cx, |_, cx| cx.notify());
        }
    }

    fn on_unfreeze(&mut self, _: &mozui::ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        self.data.update(cx, |d, cx| {
            d.set_frozen_rows(0, cx);
            d.set_frozen_cols(0, cx);
        });
        self.table.update(cx, |_, cx| cx.notify());
    }

    // ── Sheet tab actions ───────────────────────────────────────────────

    fn on_switch_sheet(&mut self, sheet_id: u32, _w: &mut Window, cx: &mut Context<Self>) {
        self.data
            .update(cx, |d, cx| d.set_active_sheet(sheet_id, cx));
        self.selected_table_cell = None;
        self.table.update(cx, |_, cx| cx.notify());
    }

    fn on_add_sheet(&mut self, _: &mozui::ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        self.data.update(cx, |d, cx| d.add_sheet(cx));
    }

    fn start_rename_sheet(&mut self, sheet_index: u32, cx: &mut Context<Self>) {
        let props = self.data.read(cx).visible_sheets();
        let name = props
            .iter()
            .find(|(i, _, _)| *i == sheet_index)
            .map(|(_, n, _)| n.clone())
            .unwrap_or_default();
        self.renaming_sheet = Some((sheet_index, name));
        cx.notify();
    }

    pub fn commit_rename_sheet(&mut self, cx: &mut Context<Self>) {
        if let Some((sheet_index, new_name)) = self.renaming_sheet.take() {
            let trimmed = new_name.trim().to_string();
            if !trimmed.is_empty() {
                self.data
                    .update(cx, |d, cx| d.rename_sheet(sheet_index, &trimmed, cx));
            }
            cx.notify();
        }
    }

    fn on_rename_sheet_action(&mut self, _: &RenameSheet, _w: &mut Window, cx: &mut Context<Self>) {
        if let Some(idx) = self.context_menu_sheet.take() {
            self.start_rename_sheet(idx, cx);
        }
    }

    fn on_delete_sheet_action(&mut self, _: &DeleteSheet, _w: &mut Window, cx: &mut Context<Self>) {
        if let Some(idx) = self.context_menu_sheet.take() {
            self.data.update(cx, |d, cx| d.delete_sheet(idx, cx));
            self.table.update(cx, |_, cx| cx.notify());
        }
    }

    fn on_hide_sheet_action(&mut self, _: &HideSheet, _w: &mut Window, cx: &mut Context<Self>) {
        if let Some(idx) = self.context_menu_sheet.take() {
            self.data.update(cx, |d, cx| d.hide_sheet(idx, cx));
            self.table.update(cx, |_, cx| cx.notify());
        }
    }

    fn on_unhide_sheet_action(&mut self, _: &UnhideSheet, _w: &mut Window, cx: &mut Context<Self>) {
        let hidden = self.data.read(cx).hidden_sheets();
        if let Some((idx, _)) = hidden.first() {
            let idx = *idx;
            self.data.update(cx, |d, cx| d.unhide_sheet(idx, cx));
            self.table.update(cx, |_, cx| cx.notify());
        }
    }

    // ── Key handling ────────────────────────────────────────────────────

    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let is_editing = self.data.read(cx).editing_cell.is_some();
        let key = event.keystroke.key.as_str();

        // Tab / Shift+Tab while editing: commit and move selection.
        if is_editing && key == "tab" {
            self.data.update(cx, |d, cx| d.commit_edit(cx));
            let direction: i32 = if event.keystroke.modifiers.shift {
                -1
            } else {
                1
            };
            if let Some((row, table_col)) = self.selected_table_cell {
                let cols_count = self.data.read(cx).cols + 1;
                let new_col = (table_col as i32 + direction)
                    .max(1)
                    .min(cols_count as i32 - 1) as usize;
                self.selected_table_cell = Some((row, new_col));
                if new_col > 0 {
                    self.data
                        .update(cx, |d, _cx| d.set_selected_cell(row, new_col - 1));
                }
            }
            self.table.update(cx, |_, cx| cx.notify());
            cx.notify();
            return;
        }

        if is_editing {
            return;
        }
        let Some((row, col)) = self.selected_data_cell() else {
            return;
        };

        // Backspace / Delete: clear contents.
        if key == "backspace" || key == "delete" {
            let range = self.data.read(cx).selected_range();
            let (r0, c0, r1, c1) = range.unwrap_or((row, col, row, col));
            self.data
                .update(cx, |d, cx| d.clear_contents_range(r0, c0, r1, c1, cx));
            self.table.update(cx, |_, cx| cx.notify());
            return;
        }

        let mods = &event.keystroke.modifiers;
        if mods.control || mods.platform || mods.function {
            return;
        }

        // Type-to-edit: printable key starts inline edit.
        let ch = event.keystroke.key_char.as_deref().or(Some(key));
        let ch = match ch {
            Some(s) if s.len() == 1 => s,
            _ => return,
        };
        let c = ch.chars().next().unwrap_or('\0');
        if !c.is_ascii_graphic() && c != ' ' {
            return;
        }

        self.data
            .update(cx, |d, cx| d.start_editing_with(row, col, ch, window, cx));
        self.table.update(cx, |_, cx| cx.notify());
    }

    // ── Panel renderers ─────────────────────────────────────────────────

    fn render_toolbar(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let fmt = self.selected_format(cx);
        let has_cell = self.selected_data_cell().is_some();
        let font_size = self
            .selected_data_cell()
            .map(|(r, c)| self.data.read(cx).font_size(r, c))
            .unwrap_or(11);
        let data_read = self.data.read(cx);
        let can_undo = data_read.can_undo();
        let can_redo = data_read.can_redo();
        let grid_lines = data_read.show_grid_lines();
        let has_freeze = data_read.frozen_rows() > 0 || data_read.frozen_cols() > 0;
        let _ = data_read;

        div()
            .flex()
            .items_center()
            .gap(px(2.0))
            .px(px(8.0))
            .h(px(40.0))
            .border_b_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().table_head)
            // Undo/Redo
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(2.0))
                    .child(
                        Button::new("undo")
                            .label("↩")
                            .with_size(crate::Size::Small)
                            .disabled(!can_undo)
                            .on_click(cx.listener(|this, _, w, cx| this.on_undo(&Undo, w, cx))),
                    )
                    .child(
                        Button::new("redo")
                            .label("↪")
                            .with_size(crate::Size::Small)
                            .disabled(!can_redo)
                            .on_click(cx.listener(|this, _, w, cx| this.on_redo(&Redo, w, cx))),
                    ),
            )
            .child(toolbar_sep(cx))
            // Formatting
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(2.0))
                    .child(
                        Button::new("bold")
                            .label("B")
                            .with_size(crate::Size::Small)
                            .when(fmt.bold && has_cell, |b| b.ghost())
                            .on_click(cx.listener(Self::on_bold)),
                    )
                    .child(
                        Button::new("italic")
                            .label("I")
                            .with_size(crate::Size::Small)
                            .when(fmt.italic && has_cell, |b| b.ghost())
                            .on_click(cx.listener(Self::on_italic)),
                    )
                    .child(
                        Button::new("underline")
                            .label("U")
                            .with_size(crate::Size::Small)
                            .when(fmt.underline && has_cell, |b| b.ghost())
                            .on_click(cx.listener(Self::on_underline)),
                    )
                    .child(
                        Button::new("clear-fmt")
                            .label("Tx")
                            .with_size(crate::Size::Small)
                            .disabled(!has_cell)
                            .on_click(cx.listener(|this, _, w, cx| {
                                this.on_clear_formatting(&ClearFormatting, w, cx);
                            })),
                    ),
            )
            .child(toolbar_sep(cx))
            // Alignment
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(2.0))
                    .child(
                        Button::new("align-left")
                            .label("≡←")
                            .with_size(crate::Size::Small)
                            .when(has_cell && fmt.align == TextAlign::Left, |b| b.ghost())
                            .on_click(
                                cx.listener(|this, _, w, cx| this.on_align(TextAlign::Left, w, cx)),
                            ),
                    )
                    .child(
                        Button::new("align-center")
                            .label("≡")
                            .with_size(crate::Size::Small)
                            .when(has_cell && fmt.align == TextAlign::Center, |b| b.ghost())
                            .on_click(cx.listener(|this, _, w, cx| {
                                this.on_align(TextAlign::Center, w, cx)
                            })),
                    )
                    .child(
                        Button::new("align-right")
                            .label("≡→")
                            .with_size(crate::Size::Small)
                            .when(has_cell && fmt.align == TextAlign::Right, |b| b.ghost())
                            .on_click(
                                cx.listener(|this, _, w, cx| {
                                    this.on_align(TextAlign::Right, w, cx)
                                }),
                            ),
                    ),
            )
            .child(toolbar_sep(cx))
            // Number format
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(2.0))
                    .child(
                        Button::new("fmt-currency")
                            .label("$")
                            .with_size(crate::Size::Small)
                            .when(
                                has_cell && fmt.number_format == NumberFormat::Currency,
                                |b| b.ghost(),
                            )
                            .on_click(cx.listener(|this, _, w, cx| {
                                this.on_number_format(NumberFormat::Currency, w, cx)
                            })),
                    )
                    .child(
                        Button::new("fmt-percent")
                            .label("%")
                            .with_size(crate::Size::Small)
                            .when(
                                has_cell && fmt.number_format == NumberFormat::Percent,
                                |b| b.ghost(),
                            )
                            .on_click(cx.listener(|this, _, w, cx| {
                                this.on_number_format(NumberFormat::Percent, w, cx)
                            })),
                    )
                    .child(
                        Button::new("fmt-plain")
                            .label("123")
                            .with_size(crate::Size::Small)
                            .when(has_cell && fmt.number_format == NumberFormat::Auto, |b| {
                                b.ghost()
                            })
                            .on_click(cx.listener(|this, _, w, cx| {
                                this.on_number_format(NumberFormat::Auto, w, cx)
                            })),
                    ),
            )
            .child(toolbar_sep(cx))
            // Font size
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(1.0))
                    .child(
                        Button::new("font-smaller")
                            .label("A-")
                            .with_size(crate::Size::Small)
                            .disabled(!has_cell || font_size <= 6)
                            .on_click(
                                cx.listener(|this, _, w, cx| this.on_font_size_delta(-2, w, cx)),
                            ),
                    )
                    .child(
                        div()
                            .min_w(px(24.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_xs()
                            .text_color(cx.theme().foreground)
                            .child(format!("{}", font_size)),
                    )
                    .child(
                        Button::new("font-larger")
                            .label("A+")
                            .with_size(crate::Size::Small)
                            .disabled(!has_cell || font_size >= 72)
                            .on_click(
                                cx.listener(|this, _, w, cx| this.on_font_size_delta(2, w, cx)),
                            ),
                    ),
            )
            .child(toolbar_sep(cx))
            // Colors & wrap
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(2.0))
                    .child(
                        Button::new("text-color-red")
                            .label("A")
                            .with_size(crate::Size::Small)
                            .disabled(!has_cell)
                            .on_click(cx.listener(|this, _, w, cx| {
                                this.on_text_color(hsla(0.0, 0.8, 0.5, 1.0), w, cx);
                            })),
                    )
                    .child(
                        Button::new("text-color-blue")
                            .label("A")
                            .with_size(crate::Size::Small)
                            .disabled(!has_cell)
                            .on_click(cx.listener(|this, _, w, cx| {
                                this.on_text_color(hsla(0.6, 0.8, 0.5, 1.0), w, cx);
                            })),
                    )
                    .child(
                        Button::new("bg-yellow")
                            .label("Bg")
                            .with_size(crate::Size::Small)
                            .disabled(!has_cell)
                            .on_click(cx.listener(|this, _, w, cx| {
                                this.on_bg_color(hsla(0.15, 0.9, 0.75, 1.0), w, cx);
                            })),
                    )
                    .child(
                        Button::new("wrap-text")
                            .label("Wrap")
                            .with_size(crate::Size::Small)
                            .disabled(!has_cell)
                            .on_click(cx.listener(Self::on_wrap_text)),
                    ),
            )
            .child(toolbar_sep(cx))
            // View controls
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(2.0))
                    .child(
                        Button::new("grid-lines")
                            .label(if grid_lines { "Grid ✓" } else { "Grid" })
                            .with_size(crate::Size::Small)
                            .when(grid_lines, |b| b.ghost())
                            .on_click(cx.listener(|this, _, w, cx| {
                                this.on_toggle_grid_lines(&ToggleGridLines, w, cx);
                            })),
                    )
                    .child(
                        Button::new("freeze")
                            .label("Freeze")
                            .with_size(crate::Size::Small)
                            .disabled(!has_cell)
                            .on_click(cx.listener(Self::on_freeze_row)),
                    )
                    .when(has_freeze, |d| {
                        d.child(
                            Button::new("unfreeze")
                                .label("Unfreeze")
                                .with_size(crate::Size::Small)
                                .on_click(cx.listener(Self::on_unfreeze)),
                        )
                    }),
            )
            .child(div().flex_1())
            // Row operations
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .child(
                        Button::new("add-row")
                            .label("+ Row")
                            .with_size(crate::Size::Small)
                            .on_click(cx.listener(Self::on_add_row)),
                    )
                    .child(
                        Button::new("delete-row")
                            .label("- Row")
                            .with_size(crate::Size::Small)
                            .disabled(!has_cell)
                            .on_click(cx.listener(Self::on_delete_selected_row)),
                    ),
            )
    }

    fn render_formula_bar(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .px(px(10.0))
            .h(px(30.0))
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                div()
                    .min_w(px(52.0))
                    .max_w(px(80.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_xs()
                    .font_semibold()
                    .text_color(cx.theme().foreground)
                    .child(
                        Input::new(&self.cell_ref_input)
                            .appearance(false)
                            .bordered(false)
                            .focus_bordered(false)
                            .with_size(crate::Size::XSmall),
                    ),
            )
            .child(div().w(px(1.0)).h(px(16.0)).bg(cx.theme().border))
            .child(
                div()
                    .text_xs()
                    .italic()
                    .text_color(cx.theme().muted_foreground)
                    .child("fx"),
            )
            .child(
                div()
                    .flex_1()
                    .text_sm()
                    .text_color(cx.theme().foreground)
                    .child(
                        Input::new(&self.formula_input)
                            .appearance(false)
                            .bordered(false)
                            .focus_bordered(false)
                            .with_size(crate::Size::Small),
                    ),
            )
    }

    fn render_sheet_tab_bar(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let data = self.data.read(cx);
        let active = data.active_sheet();
        let sheet_tabs: Vec<(u32, String, bool, Option<Hsla>)> = data
            .visible_sheets()
            .into_iter()
            .map(|(index, name, color)| (index, name, index == active, color))
            .collect();
        let _ = data;

        let mut tab_bar = div()
            .flex()
            .items_center()
            .h(px(30.0))
            .border_t_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().table_head)
            .px(px(4.0));

        let renaming = self.renaming_sheet.clone();
        let sheet_count = sheet_tabs.len();
        let has_hidden = !self.data.read(cx).hidden_sheets().is_empty();

        for (sheet_index, name, is_active, tab_color) in sheet_tabs {
            let idx = sheet_index;
            let is_renaming = renaming
                .as_ref()
                .map(|(i, _)| *i == sheet_index)
                .unwrap_or(false);
            let rename_text = renaming
                .as_ref()
                .filter(|(i, _)| *i == sheet_index)
                .map(|(_, t)| t.clone());

            let tab = div()
                .id(("sheet-tab", sheet_index))
                .on_click(cx.listener(move |this, _, w, cx| {
                    if !is_active {
                        this.on_switch_sheet(idx, w, cx);
                    }
                }))
                .on_double_click(cx.listener(move |this, _: &mozui::ClickEvent, _, cx| {
                    this.start_rename_sheet(idx, cx);
                }))
                .flex()
                .items_center()
                .px(px(14.0))
                .h_full()
                .border_r_1()
                .border_color(cx.theme().border)
                .when(is_active, |d| d.bg(cx.theme().background))
                .when(!is_active, |d| {
                    d.cursor_pointer().hover(|d| d.bg(cx.theme().secondary))
                })
                .text_xs()
                .font_semibold()
                .when(is_active, |d| d.text_color(cx.theme().foreground))
                .when(!is_active, |d| d.text_color(cx.theme().muted_foreground))
                .when_some(tab_color, |d, c| d.border_b_2().border_color(c))
                .map(|d| {
                    if is_renaming {
                        d.child(rename_text.unwrap_or_default())
                    } else {
                        d.child(name.clone())
                    }
                })
                .context_menu({
                    let name = name.clone();
                    move |menu, _, _| {
                        menu.menu(format!("Rename \"{}\"", name), Box::new(RenameSheet))
                            .when(sheet_count > 1, |menu| {
                                menu.menu(format!("Hide \"{}\"", name), Box::new(HideSheet))
                            })
                            .when(has_hidden, |menu| {
                                menu.menu("Unhide sheet".to_string(), Box::new(UnhideSheet))
                            })
                            .when(sheet_count > 1, |menu| {
                                menu.separator()
                                    .menu(format!("Delete \"{}\"", name), Box::new(DeleteSheet))
                            })
                    }
                });

            tab_bar = tab_bar.child(
                div()
                    .on_mouse_down(
                        mozui::MouseButton::Right,
                        cx.listener(move |this, _, _, cx| {
                            this.context_menu_sheet = Some(idx);
                            cx.notify();
                        }),
                    )
                    .child(tab),
            );
        }

        tab_bar.child(div().flex_1()).child(
            div()
                .id("add-sheet-btn")
                .on_click(cx.listener(Self::on_add_sheet))
                .flex()
                .items_center()
                .justify_center()
                .w(px(24.0))
                .h_full()
                .cursor_pointer()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .hover(|d| d.text_color(cx.theme().foreground))
                .child("+"),
        )
    }
}

impl Render for Spreadsheet {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Sync formula bar inputs when selected cell changes.
        if self.show_formula_bar {
            let current_data_cell = self.selected_data_cell();
            if current_data_cell != self.formula_synced_cell {
                self.formula_synced_cell = current_data_cell;
                let cell_ref = self.cell_ref(cx);
                self.cell_ref_input
                    .update(cx, |s, cx| s.set_value(cell_ref, window, cx));
                if let Some((row, col)) = current_data_cell {
                    let raw = self.data.read(cx).raw_value(row, col);
                    self.formula_input
                        .update(cx, |s, cx| s.set_value(raw, window, cx));
                } else {
                    self.formula_input
                        .update(cx, |s, cx| s.set_value("", window, cx));
                }
            }

            // Color-code reference tokens in the formula bar text.
            let formula_text = self.formula_input.read(cx).value().to_string();
            let spans = formula_bar_highlights(&formula_text);
            let bar_highlights: Vec<_> = spans
                .into_iter()
                .map(|(range, color)| {
                    (
                        range,
                        HighlightStyle {
                            color,
                            ..Default::default()
                        },
                    )
                })
                .collect();
            let current = &self.formula_input.read(cx).custom_highlights;
            if *current != bar_highlights {
                self.formula_input
                    .update(cx, |s, cx| s.set_highlights(bar_highlights, cx));
            }
        }

        div()
            .id("spreadsheet-root")
            .key_context("Spreadsheet")
            .on_action(cx.listener(Self::on_undo))
            .on_action(cx.listener(Self::on_redo))
            .on_action(cx.listener(Self::on_insert_row_above))
            .on_action(cx.listener(Self::on_insert_row_below))
            .on_action(cx.listener(Self::on_delete_row))
            .on_action(cx.listener(Self::on_insert_col_left))
            .on_action(cx.listener(Self::on_insert_col_right))
            .on_action(cx.listener(Self::on_delete_col))
            .on_action(cx.listener(Self::on_copy))
            .on_action(cx.listener(Self::on_cut))
            .on_action(cx.listener(Self::on_paste))
            .on_action(cx.listener(Self::on_toggle_grid_lines))
            .on_action(cx.listener(Self::on_rename_sheet_action))
            .on_action(cx.listener(Self::on_delete_sheet_action))
            .on_action(cx.listener(Self::on_hide_sheet_action))
            .on_action(cx.listener(Self::on_unhide_sheet_action))
            .on_action(cx.listener(Self::on_clear_formatting))
            .on_action(cx.listener(Self::on_clear_contents))
            .on_key_down(cx.listener(Self::on_key_down))
            .size_full()
            .flex()
            .flex_col()
            .bg(cx.theme().background)
            // Toolbar (opt-in)
            .when(self.show_toolbar, |this| {
                this.child(self.render_toolbar(window, cx))
            })
            // Formula bar (opt-in)
            .when(self.show_formula_bar, |this| {
                this.child(self.render_formula_bar(window, cx))
            })
            // Grid (always)
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .child(DataTable::new(&self.table).bordered(false)),
            )
            // Sheet tab bar (opt-in)
            .when(self.show_sheet_tabs, |this| {
                this.child(self.render_sheet_tab_bar(window, cx))
            })
    }
}

fn toolbar_sep<V: 'static>(cx: &Context<V>) -> mozui::Div {
    div()
        .w(px(1.0))
        .h(px(20.0))
        .mx(px(4.0))
        .bg(cx.theme().border)
}

/// Parse a cell reference like "A1", "B12", "AA5" into 0-based `(row, col)`.
fn parse_cell_ref(s: &str) -> Option<(usize, usize)> {
    let s = s.trim().to_uppercase();
    let col_end = s.find(|c: char| c.is_ascii_digit())?;
    if col_end == 0 {
        return None;
    }
    let col_str = &s[..col_end];
    let row_str = &s[col_end..];

    // Convert column letters to 0-based index (A=0, B=1, ..., Z=25, AA=26, ...).
    let mut col: usize = 0;
    for c in col_str.chars() {
        if !c.is_ascii_uppercase() {
            return None;
        }
        col = col * 26 + (c as usize - 'A' as usize + 1);
    }
    col = col.checked_sub(1)?;

    let row: usize = row_str.parse().ok()?;
    let row = row.checked_sub(1)?; // 1-based → 0-based

    Some((row, col))
}
