mod support;

use mozui::prelude::*;
use mozui::{
    ClickEvent, ClipboardItem, Context, Entity, SharedString, Subscription, TextAlign, Window, div,
    hsla, px, size,
};
use mozui_components::{
    ActiveTheme, Disableable, InteractiveElementExt as _, Sizable, StyledExt as _,
    actions::{Redo, Undo},
    button::{Button, ButtonVariants as _},
    input::{Input, InputEvent, InputState},
    menu::ContextMenuExt as _,
    spreadsheet::{
        CellFormat, ClearContents, ClearFormatting, Copy, Cut, DeleteCol, DeleteRow, DeleteSheet,
        HideSheet, InsertColLeft, InsertColRight, InsertRowAbove, InsertRowBelow, NumberFormat,
        Paste, RenameSheet, SpreadsheetData, SpreadsheetTableDelegate, ToggleGridLines,
        UnhideSheet,
    },
    table::{DataTable, TableEvent, TableState},
    theme::ThemeMode,
};
use support::run_rooted_example;

fn main() {
    run_rooted_example(
        "Spreadsheet",
        ThemeMode::Dark,
        size(px(1200.0), px(820.0)),
        |window, cx| cx.new(|cx| SpreadsheetExample::new(window, cx)),
    );
}

// ── View ─────────────────────────────────────────────────────────────────────

struct SpreadsheetExample {
    data: Entity<SpreadsheetData>,
    table: Entity<TableState<SpreadsheetTableDelegate>>,
    /// Selected cell in table coordinates (col 0 = row gutter).
    selected_table_cell: Option<(usize, usize)>,
    /// Sheet tab currently being renamed: `(sheet_index, current_text)`.
    renaming_sheet: Option<(u32, String)>,
    /// Sheet index targeted by a right-click context menu action.
    context_menu_sheet: Option<u32>,
    /// Formula bar input state — editable, commits on Enter.
    formula_input: Entity<InputState>,
    /// Last cell synced to the formula bar (to avoid re-setting on every render).
    formula_synced_cell: Option<(usize, usize)>,
    _subs: Vec<Subscription>,
}

impl SpreadsheetExample {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let data = cx.new(|cx| {
            let mut d = SpreadsheetData::new(22, 7, window, cx);
            populate_budget(&mut d);
            // Clear undo history so initial data population is not undoable.
            d.snapshot_clear_history();
            d
        });

        let table =
            cx.new(|cx| SpreadsheetTableDelegate::into_table_state(data.clone(), window, cx));

        let input = data.read(cx).input.clone();
        let formula_input = cx.new(|cx| InputState::new(window, cx));

        let subs = vec![
            cx.observe(&data, {
                let table = table.clone();
                move |_, _, cx| {
                    table.update(cx, |_, cx| cx.notify());
                }
            }),
            cx.observe(&input, {
                let table = table.clone();
                move |_, _, cx| {
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
                            });
                            table.update(cx, |_, cx| cx.notify());
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
                        // Sync single-cell selection to IronCalc (skip gutter col 0).
                        if *col > 0 {
                            data.update(cx, |d, _cx| d.set_selected_cell(*row, *col - 1));
                        }
                        cx.notify();
                    }
                    TableEvent::SelectRange(r0, c0, r1, c1) => {
                        // Convert table cols to data cols (subtract 1 for gutter).
                        let dc0 = c0.saturating_sub(1);
                        let dc1 = c1.saturating_sub(1);
                        data.update(cx, |d, _cx| d.extend_selection_to(*r1, dc1));
                        let _ = (dc0, r0);
                        cx.notify();
                    }
                    TableEvent::RightClickedCell(row, col) => {
                        this.selected_table_cell = Some((*row, *col));
                        cx.notify();
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
            formula_synced_cell: None,
            _subs: subs,
        }
    }

    // ── Selected-cell helpers ────────────────────────────────────────────────

    fn selected_data_cell(&self) -> Option<(usize, usize)> {
        match self.selected_table_cell {
            Some((row, col)) if col > 0 => Some((row, col - 1)),
            _ => None,
        }
    }

    fn cell_ref(&self, cx: &Context<Self>) -> SharedString {
        let range = self.data.read(cx).selected_range();
        if let Some((r0, c0, r1, c1)) = range {
            let start = format!(
                "{}{}",
                mozui_components::spreadsheet::col_to_letter(c0),
                r0 + 1
            );
            let end = format!(
                "{}{}",
                mozui_components::spreadsheet::col_to_letter(c1),
                r1 + 1
            );
            format!("{}:{}", start, end).into()
        } else {
            match self.selected_data_cell() {
                Some((row, col)) => {
                    let letter = mozui_components::spreadsheet::col_to_letter(col);
                    format!("{}{}", letter, row + 1).into()
                }
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

    // ── Toolbar actions ──────────────────────────────────────────────────────

    fn on_bold(&mut self, _: &ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        if let Some((row, col)) = self.selected_data_cell() {
            self.data.update(cx, |d, cx| d.toggle_bold(row, col, cx));
        }
    }

    fn on_italic(&mut self, _: &ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        if let Some((row, col)) = self.selected_data_cell() {
            self.data.update(cx, |d, cx| d.toggle_italic(row, col, cx));
        }
    }

    fn on_align_left(&mut self, _: &ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        if let Some((row, col)) = self.selected_data_cell() {
            self.data
                .update(cx, |d, cx| d.set_align(row, col, TextAlign::Left, cx));
        }
    }

    fn on_align_center(&mut self, _: &ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        if let Some((row, col)) = self.selected_data_cell() {
            self.data
                .update(cx, |d, cx| d.set_align(row, col, TextAlign::Center, cx));
        }
    }

    fn on_align_right(&mut self, _: &ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        if let Some((row, col)) = self.selected_data_cell() {
            self.data
                .update(cx, |d, cx| d.set_align(row, col, TextAlign::Right, cx));
        }
    }

    fn on_fmt_currency(&mut self, _: &ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        if let Some((row, col)) = self.selected_data_cell() {
            self.data.update(cx, |d, cx| {
                d.set_number_format(row, col, NumberFormat::Currency, cx)
            });
        }
    }

    fn on_fmt_percent(&mut self, _: &ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        if let Some((row, col)) = self.selected_data_cell() {
            self.data.update(cx, |d, cx| {
                d.set_number_format(row, col, NumberFormat::Percent, cx)
            });
        }
    }

    fn on_fmt_plain(&mut self, _: &ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        if let Some((row, col)) = self.selected_data_cell() {
            self.data.update(cx, |d, cx| {
                d.set_number_format(row, col, NumberFormat::Auto, cx)
            });
        }
    }

    fn on_add_row(&mut self, _: &ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        self.data.update(cx, |d, cx| d.add_row(cx));
        self.table.update(cx, |_, cx| cx.notify());
    }

    fn on_delete_selected_row(&mut self, _: &ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        if let Some((row, _)) = self.selected_data_cell() {
            let data = self.data.clone();
            let table = self.table.clone();
            data.update(cx, |d, cx| d.delete_row(row, cx));
            self.selected_table_cell = None;
            table.update(cx, |_, cx| cx.notify());
        }
    }

    fn on_undo(&mut self, _: &Undo, _w: &mut Window, cx: &mut Context<Self>) {
        self.data.update(cx, |d, cx| d.undo(cx));
        self.table.update(cx, |_, cx| cx.notify());
    }

    fn on_redo(&mut self, _: &Redo, _w: &mut Window, cx: &mut Context<Self>) {
        self.data.update(cx, |d, cx| d.redo(cx));
        self.table.update(cx, |_, cx| cx.notify());
    }

    // ── Context menu actions ──────────────────────────────────────────────

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

    fn on_delete_row_action(&mut self, _: &DeleteRow, _w: &mut Window, cx: &mut Context<Self>) {
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

    fn on_delete_col_action(&mut self, _: &DeleteCol, _w: &mut Window, cx: &mut Context<Self>) {
        if let Some((_, col)) = self.selected_data_cell() {
            self.data.update(cx, |d, cx| d.delete_col(col, cx));
            self.selected_table_cell = None;
            self.table.update(cx, |t, cx| t.refresh(cx));
        }
    }

    // ── Clipboard ────────────────────────────────────────────────────────

    fn on_copy(&mut self, _: &Copy, _w: &mut Window, cx: &mut Context<Self>) {
        let tsv = self.data.read(cx).copy_selection_as_tsv();
        cx.write_to_clipboard(ClipboardItem::new_string(tsv));
    }

    fn on_cut(&mut self, _: &Cut, _w: &mut Window, cx: &mut Context<Self>) {
        let tsv = self.data.update(cx, |d, cx| d.cut_selection_as_tsv(cx));
        cx.write_to_clipboard(ClipboardItem::new_string(tsv));
    }

    fn on_paste(&mut self, _: &Paste, _w: &mut Window, cx: &mut Context<Self>) {
        if let Some(item) = cx.read_from_clipboard() {
            if let Some(text) = item.text() {
                self.data.update(cx, |d, cx| d.paste_tsv(&text, cx));
                self.table.update(cx, |_, cx| cx.notify());
            }
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

    fn on_freeze_row(&mut self, _: &ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        if let Some((row, _)) = self.selected_data_cell() {
            self.data.update(cx, |d, cx| d.set_frozen_rows(row + 1, cx));
            self.table.update(cx, |_, cx| cx.notify());
        }
    }

    fn on_unfreeze(&mut self, _: &ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        self.data.update(cx, |d, cx| {
            d.set_frozen_rows(0, cx);
            d.set_frozen_cols(0, cx);
        });
        self.table.update(cx, |_, cx| cx.notify());
    }

    fn on_add_sheet(&mut self, _: &ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        self.data.update(cx, |d, cx| d.add_sheet(cx));
    }

    fn on_switch_sheet(&mut self, sheet_id: u32, _w: &mut Window, cx: &mut Context<Self>) {
        self.data
            .update(cx, |d, cx| d.set_active_sheet(sheet_id, cx));
        self.selected_table_cell = None;
        self.table.update(cx, |_, cx| cx.notify());
    }

    // ── Sheet tab rename / delete ────────────────────────────────────────

    fn start_rename_sheet(&mut self, sheet_index: u32, cx: &mut Context<Self>) {
        let names = self.data.read(cx).sheet_names();
        let props = self.data.read(cx).visible_sheets();
        let name = props
            .iter()
            .find(|(i, _)| *i == sheet_index)
            .map(|(_, n)| n.clone())
            .unwrap_or_else(|| names.first().cloned().unwrap_or_default());
        self.renaming_sheet = Some((sheet_index, name));
        cx.notify();
    }

    fn commit_rename_sheet(&mut self, cx: &mut Context<Self>) {
        if let Some((sheet_index, new_name)) = self.renaming_sheet.take() {
            let trimmed = new_name.trim().to_string();
            if !trimmed.is_empty() {
                self.data
                    .update(cx, |d, cx| d.rename_sheet(sheet_index, &trimmed, cx));
            }
            cx.notify();
        }
    }

    fn cancel_rename_sheet(&mut self, cx: &mut Context<Self>) {
        self.renaming_sheet = None;
        cx.notify();
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
        // Unhide the first hidden sheet (simplified — full UI would show picker).
        let hidden = self.data.read(cx).hidden_sheets();
        if let Some((idx, _)) = hidden.first() {
            let idx = *idx;
            self.data.update(cx, |d, cx| d.unhide_sheet(idx, cx));
            self.table.update(cx, |_, cx| cx.notify());
        }
    }

    // ── Formatting actions ───────────────────────────────────────────────

    fn on_underline(&mut self, _: &ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        if let Some((row, col)) = self.selected_data_cell() {
            self.data
                .update(cx, |d, cx| d.toggle_underline(row, col, cx));
        }
    }

    fn on_clear_formatting(
        &mut self,
        _: &ClearFormatting,
        _w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some((row, col)) = self.selected_data_cell() {
            // Check for multi-cell range selection from IronCalc.
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
}

impl Render for SpreadsheetExample {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Sync formula bar input when selected cell changes.
        let current_data_cell = self.selected_data_cell();
        if current_data_cell != self.formula_synced_cell {
            self.formula_synced_cell = current_data_cell;
            if let Some((row, col)) = current_data_cell {
                let raw = self.data.read(cx).raw_value(row, col);
                self.formula_input
                    .update(cx, |s, cx| s.set_value(raw, window, cx));
            } else {
                self.formula_input
                    .update(cx, |s, cx| s.set_value("", window, cx));
            }
        }

        let cell_ref = self.cell_ref(cx);
        let fmt = self.selected_format(cx);
        let has_cell = self.selected_data_cell().is_some();
        let data_read = self.data.read(cx);
        let can_undo = data_read.can_undo();
        let can_redo = data_read.can_redo();
        let grid_lines = data_read.show_grid_lines();
        let has_freeze = data_read.frozen_rows() > 0 || data_read.frozen_cols() > 0;
        drop(data_read);

        div()
            .id("spreadsheet-root")
            .key_context("Spreadsheet")
            .on_action(cx.listener(Self::on_undo))
            .on_action(cx.listener(Self::on_redo))
            .on_action(cx.listener(Self::on_insert_row_above))
            .on_action(cx.listener(Self::on_insert_row_below))
            .on_action(cx.listener(Self::on_delete_row_action))
            .on_action(cx.listener(Self::on_insert_col_left))
            .on_action(cx.listener(Self::on_insert_col_right))
            .on_action(cx.listener(Self::on_delete_col_action))
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
            .size_full()
            .flex()
            .flex_col()
            .bg(cx.theme().background)
            // ── Menu bar (title) ──────────────────────────────────────────────
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.0))
                    .px(px(16.0))
                    .h(px(40.0))
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .text_sm()
                            .font_semibold()
                            .text_color(cx.theme().foreground)
                            .child("Q1 Budget.xlsx"),
                    )
                    .child(div().flex_1())
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(
                                "Double-click to edit · Shift+click for range · ⌘Z undo · ⌘⇧Z redo",
                            ),
                    ),
            )
            // ── Toolbar ───────────────────────────────────────────────────────
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(2.0))
                    .px(px(8.0))
                    .h(px(40.0))
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().table_head)
                    // Undo/Redo group
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(2.0))
                            .child(
                                Button::new("undo")
                                    .label("↩")
                                    .with_size(mozui_components::Size::Small)
                                    .disabled(!can_undo)
                                    .on_click(cx.listener(|this, e, w, cx| {
                                        this.on_undo(&Undo, w, cx);
                                        let _ = e;
                                    })),
                            )
                            .child(
                                Button::new("redo")
                                    .label("↪")
                                    .with_size(mozui_components::Size::Small)
                                    .disabled(!can_redo)
                                    .on_click(cx.listener(|this, e, w, cx| {
                                        this.on_redo(&Redo, w, cx);
                                        let _ = e;
                                    })),
                            ),
                    )
                    .child(toolbar_sep(cx))
                    // Formatting group
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(2.0))
                            .child(
                                Button::new("bold")
                                    .label("B")
                                    .with_size(mozui_components::Size::Small)
                                    .when(fmt.bold && has_cell, |b| b.ghost())
                                    .on_click(cx.listener(Self::on_bold)),
                            )
                            .child(
                                Button::new("italic")
                                    .label("I")
                                    .with_size(mozui_components::Size::Small)
                                    .when(fmt.italic && has_cell, |b| b.ghost())
                                    .on_click(cx.listener(Self::on_italic)),
                            )
                            .child(
                                Button::new("underline")
                                    .label("U")
                                    .with_size(mozui_components::Size::Small)
                                    .when(fmt.underline && has_cell, |b| b.ghost())
                                    .on_click(cx.listener(Self::on_underline)),
                            )
                            .child(
                                Button::new("clear-fmt")
                                    .label("Tx")
                                    .with_size(mozui_components::Size::Small)
                                    .disabled(!has_cell)
                                    .on_click(cx.listener(|this, _, w, cx| {
                                        this.on_clear_formatting(&ClearFormatting, w, cx);
                                    })),
                            ),
                    )
                    .child(toolbar_sep(cx))
                    // Alignment group
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(2.0))
                            .child(
                                Button::new("align-left")
                                    .label("≡←")
                                    .with_size(mozui_components::Size::Small)
                                    .when(has_cell && fmt.align == TextAlign::Left, |b| b.ghost())
                                    .on_click(cx.listener(Self::on_align_left)),
                            )
                            .child(
                                Button::new("align-center")
                                    .label("≡")
                                    .with_size(mozui_components::Size::Small)
                                    .when(has_cell && fmt.align == TextAlign::Center, |b| b.ghost())
                                    .on_click(cx.listener(Self::on_align_center)),
                            )
                            .child(
                                Button::new("align-right")
                                    .label("≡→")
                                    .with_size(mozui_components::Size::Small)
                                    .when(has_cell && fmt.align == TextAlign::Right, |b| b.ghost())
                                    .on_click(cx.listener(Self::on_align_right)),
                            ),
                    )
                    .child(toolbar_sep(cx))
                    // Number format group
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(2.0))
                            .child(
                                Button::new("fmt-currency")
                                    .label("$")
                                    .with_size(mozui_components::Size::Small)
                                    .when(
                                        has_cell && fmt.number_format == NumberFormat::Currency,
                                        |b| b.ghost(),
                                    )
                                    .on_click(cx.listener(Self::on_fmt_currency)),
                            )
                            .child(
                                Button::new("fmt-percent")
                                    .label("%")
                                    .with_size(mozui_components::Size::Small)
                                    .when(
                                        has_cell && fmt.number_format == NumberFormat::Percent,
                                        |b| b.ghost(),
                                    )
                                    .on_click(cx.listener(Self::on_fmt_percent)),
                            )
                            .child(
                                Button::new("fmt-plain")
                                    .label("123")
                                    .with_size(mozui_components::Size::Small)
                                    .when(
                                        has_cell && fmt.number_format == NumberFormat::Auto,
                                        |b| b.ghost(),
                                    )
                                    .on_click(cx.listener(Self::on_fmt_plain)),
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
                                    .with_size(mozui_components::Size::Small)
                                    .when(grid_lines, |b| b.ghost())
                                    .on_click(cx.listener(|this, _, w, cx| {
                                        this.on_toggle_grid_lines(&ToggleGridLines, w, cx);
                                    })),
                            )
                            .child(
                                Button::new("freeze")
                                    .label("Freeze")
                                    .with_size(mozui_components::Size::Small)
                                    .disabled(!has_cell)
                                    .on_click(cx.listener(Self::on_freeze_row)),
                            )
                            .when(has_freeze, |d| {
                                d.child(
                                    Button::new("unfreeze")
                                        .label("Unfreeze")
                                        .with_size(mozui_components::Size::Small)
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
                                    .with_size(mozui_components::Size::Small)
                                    .on_click(cx.listener(Self::on_add_row)),
                            )
                            .child(
                                Button::new("delete-row")
                                    .label("- Row")
                                    .with_size(mozui_components::Size::Small)
                                    .disabled(!has_cell)
                                    .on_click(cx.listener(Self::on_delete_selected_row)),
                            ),
                    ),
            )
            // ── Formula bar ───────────────────────────────────────────────────
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .px(px(10.0))
                    .h(px(30.0))
                    .border_b_1()
                    .border_color(cx.theme().border)
                    // Cell reference box
                    .child(
                        div()
                            .min_w(px(52.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(3.0))
                            .border_1()
                            .border_color(cx.theme().border)
                            .text_xs()
                            .font_semibold()
                            .text_color(cx.theme().foreground)
                            .child(cell_ref),
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
                                    .with_size(mozui_components::Size::Small),
                            ),
                    ),
            )
            // ── Spreadsheet grid ──────────────────────────────────────────────
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .child(DataTable::new(&self.table).bordered(false)),
            )
            // ── Sheet tab bar ─────────────────────────────────────────────────
            .child({
                let data = self.data.read(cx);
                let active = data.active_sheet();
                let sheet_tabs: Vec<(u32, String, bool)> = data
                    .visible_sheets()
                    .into_iter()
                    .map(|(index, name)| (index, name, index == active))
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

                for (sheet_index, name, is_active) in sheet_tabs {
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
                        .map(|d| {
                            if is_renaming {
                                d.child(rename_text.unwrap_or_default())
                            } else {
                                d.child(name.clone())
                            }
                        })
                        .on_click(cx.listener(move |this, _, w, cx| {
                            if !is_active {
                                this.on_switch_sheet(idx, w, cx);
                            }
                        }))
                        .on_double_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            this.start_rename_sheet(idx, cx);
                        }))
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
                                        menu.separator().menu(
                                            format!("Delete \"{}\"", name),
                                            Box::new(DeleteSheet),
                                        )
                                    })
                            }
                        });

                    // Track which sheet the context menu targets.
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
                        .flex()
                        .items_center()
                        .justify_center()
                        .w(px(24.0))
                        .h_full()
                        .cursor_pointer()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .hover(|d| d.text_color(cx.theme().foreground))
                        .child("+")
                        .on_click(cx.listener(Self::on_add_sheet)),
                )
            })
    }
}

// ── Toolbar helpers ───────────────────────────────────────────────────────────

fn toolbar_sep<V: 'static>(cx: &Context<V>) -> mozui::Div {
    div()
        .w(px(1.0))
        .h(px(20.0))
        .mx(px(4.0))
        .bg(cx.theme().border)
}

// ── Budget data ───────────────────────────────────────────────────────────────
//
//  Sheet layout (spreadsheet 1-indexed notation):
//  Row 1  = header:  Category | Q1 Budget | January | February | March | Q1 Actual | vs Budget
//  Row 2  = "── INCOME ──" section header
//  Rows 3-6  = income line items (Salary / Freelance / Investments / Other)
//  Row 7  = Total Income
//  Row 8  = spacer
//  Row 9  = "── EXPENSES ──" section header
//  Rows 10-15 = expense line items
//  Row 16 = Total Expenses
//  Row 17 = spacer
//  Row 18 = Net Income
//  Row 19 = Savings Rate
//  Row 20 = Budget Achievement
//  Row 21 = Avg Monthly Spend
//  Row 22 = Status
//
//  Rust data indices are 0-based; IronCalc API adds +1 internally.

fn populate_budget(d: &mut SpreadsheetData) {
    // Row 0 — header
    d.set_cell(0, 0, "Category");
    d.set_cell(0, 1, "Q1 Budget");
    d.set_cell(0, 2, "January");
    d.set_cell(0, 3, "February");
    d.set_cell(0, 4, "March");
    d.set_cell(0, 5, "Q1 Actual");
    d.set_cell(0, 6, "vs Budget");
    for col in 0..7 {
        d.set_format(
            0,
            col,
            CellFormat {
                bold: true,
                align: if col == 0 {
                    TextAlign::Left
                } else {
                    TextAlign::Center
                },
                ..Default::default()
            },
        );
    }

    // Row 1 — INCOME section header
    d.set_cell(1, 0, "── INCOME ──");
    d.set_format(
        1,
        0,
        CellFormat {
            bold: true,
            text_color: Some(hsla(0.38, 0.65, 0.52, 1.0)),
            ..Default::default()
        },
    );

    // Rows 2-5 — income line items
    let income = [
        ("Salary", 15_000.0, 5_000.0, 5_200.0, 4_800.0),
        ("Freelance", 3_000.0, 900.0, 800.0, 1_100.0),
        ("Investments", 1_500.0, 430.0, 370.0, 520.0),
        ("Other", 600.0, 210.0, 150.0, 290.0),
    ];
    for (i, (name, budget, jan, feb, mar)) in income.iter().enumerate() {
        let row = 2 + i;
        d.set_cell(row, 0, *name);
        d.set_cell(row, 1, &budget.to_string());
        d.set_cell(row, 2, &jan.to_string());
        d.set_cell(row, 3, &feb.to_string());
        d.set_cell(row, 4, &mar.to_string());
        d.set_cell(row, 5, &format!("=SUM(C{}:E{})", row + 1, row + 1));
        d.set_cell(row, 6, &format!("=F{}-B{}", row + 1, row + 1));
        for col in 1..7 {
            d.set_format(row, col, CellFormat::currency());
        }
    }

    // Row 6 — TOTAL INCOME
    let ti = 6;
    d.set_cell(ti, 0, "Total Income");
    for col in 1..=4 {
        let letter = ["B", "C", "D", "E"][col - 1];
        d.set_cell(ti, col, &format!("=SUM({}3:{}6)", letter, letter));
    }
    d.set_cell(ti, 5, "=SUM(C7:E7)");
    d.set_cell(ti, 6, "=F7-B7");
    for col in 0..7 {
        d.set_format(
            ti,
            col,
            CellFormat {
                bold: true,
                align: if col == 0 {
                    TextAlign::Left
                } else {
                    TextAlign::Right
                },
                number_format: if col > 0 {
                    NumberFormat::Currency
                } else {
                    NumberFormat::Auto
                },
                text_color: Some(hsla(0.38, 0.65, 0.52, 1.0)),
                ..Default::default()
            },
        );
    }

    // Row 8 — EXPENSES section header
    d.set_cell(8, 0, "── EXPENSES ──");
    d.set_format(
        8,
        0,
        CellFormat {
            bold: true,
            text_color: Some(hsla(0.02, 0.75, 0.60, 1.0)),
            ..Default::default()
        },
    );

    // Rows 9-14 — expense line items
    let expenses = [
        ("Rent", 4_500.0, 1_500.0, 1_500.0, 1_500.0),
        ("Food & Dining", 1_200.0, 380.0, 450.0, 410.0),
        ("Transport", 450.0, 120.0, 180.0, 95.0),
        ("Utilities", 300.0, 95.0, 110.0, 88.0),
        ("Entertainment", 600.0, 200.0, 150.0, 250.0),
        ("Healthcare", 300.0, 50.0, 0.0, 175.0),
    ];
    for (i, (name, budget, jan, feb, mar)) in expenses.iter().enumerate() {
        let row = 9 + i;
        d.set_cell(row, 0, *name);
        d.set_cell(row, 1, &budget.to_string());
        d.set_cell(row, 2, &jan.to_string());
        d.set_cell(row, 3, &feb.to_string());
        d.set_cell(row, 4, &mar.to_string());
        d.set_cell(row, 5, &format!("=SUM(C{}:E{})", row + 1, row + 1));
        d.set_cell(row, 6, &format!("=F{}-B{}", row + 1, row + 1));
        for col in 1..7 {
            d.set_format(row, col, CellFormat::currency());
        }
    }

    // Row 15 — TOTAL EXPENSES
    let te = 15;
    d.set_cell(te, 0, "Total Expenses");
    for col in 1..=4 {
        let letter = ["B", "C", "D", "E"][col - 1];
        d.set_cell(te, col, &format!("=SUM({}10:{}15)", letter, letter));
    }
    d.set_cell(te, 5, "=SUM(C16:E16)");
    d.set_cell(te, 6, "=F16-B16");
    for col in 0..7 {
        d.set_format(
            te,
            col,
            CellFormat {
                bold: true,
                align: if col == 0 {
                    TextAlign::Left
                } else {
                    TextAlign::Right
                },
                number_format: if col > 0 {
                    NumberFormat::Currency
                } else {
                    NumberFormat::Auto
                },
                text_color: Some(hsla(0.02, 0.75, 0.60, 1.0)),
                ..Default::default()
            },
        );
    }

    // Row 17 — NET INCOME
    let ni = 17;
    d.set_cell(ni, 0, "Net Income");
    d.set_cell(ni, 1, "=B7-B16");
    d.set_cell(ni, 2, "=C7-C16");
    d.set_cell(ni, 3, "=D7-D16");
    d.set_cell(ni, 4, "=E7-E16");
    d.set_cell(ni, 5, "=F7-F16");
    d.set_cell(ni, 6, "=G7-G16");
    for col in 0..7 {
        d.set_format(
            ni,
            col,
            CellFormat {
                bold: true,
                align: if col == 0 {
                    TextAlign::Left
                } else {
                    TextAlign::Right
                },
                number_format: if col > 0 {
                    NumberFormat::Currency
                } else {
                    NumberFormat::Auto
                },
                ..Default::default()
            },
        );
    }

    // Row 18 — Savings Rate
    let sr = 18;
    d.set_cell(sr, 0, "Savings Rate");
    d.set_cell(sr, 1, "=IF(B7>0,(B7-B16)/B7,0)");
    d.set_cell(sr, 2, "=IF(C7>0,(C7-C16)/C7,0)");
    d.set_cell(sr, 3, "=IF(D7>0,(D7-D16)/D7,0)");
    d.set_cell(sr, 4, "=IF(E7>0,(E7-E16)/E7,0)");
    d.set_cell(sr, 5, "=IF(F7>0,(F7-F16)/F7,0)");
    for col in 1..6 {
        d.set_format(sr, col, CellFormat::percent());
    }
    d.set_format(sr, 0, CellFormat::bold());

    // Row 19 — Budget Achievement
    let ba = 19;
    d.set_cell(ba, 0, "Budget Achievement");
    d.set_cell(ba, 1, "=IF(B7>0,F7/B7,0)");
    d.set_cell(ba, 5, "=IF(B16>0,F16/B16,0)");
    d.set_format(ba, 1, CellFormat::percent());
    d.set_format(ba, 5, CellFormat::percent());
    d.set_format(ba, 0, CellFormat::bold());

    // Row 20 — Average monthly spend
    let am = 20;
    d.set_cell(am, 0, "Avg Monthly Spend");
    d.set_cell(am, 2, "=AVERAGE(C10:C15)");
    d.set_cell(am, 3, "=AVERAGE(D10:D15)");
    d.set_cell(am, 4, "=AVERAGE(E10:E15)");
    d.set_cell(am, 5, "=AVERAGE(F10:F15)");
    for col in 2..6 {
        d.set_format(am, col, CellFormat::currency());
    }
    d.set_format(am, 0, CellFormat::bold());

    // Row 21 — Status
    let st = 21;
    d.set_cell(st, 0, "Status");
    d.set_cell(st, 5, r#"=IF(F18>0,"SURPLUS","DEFICIT")"#);
    d.set_format(st, 0, CellFormat::bold());
    d.set_format(
        st,
        5,
        CellFormat {
            bold: true,
            align: TextAlign::Center,
            ..Default::default()
        },
    );
}
