//! `TableDelegate` implementation that renders a [`SpreadsheetData`] entity.

use crate::StyledExt as _;
use mozui::{
    App, ClickEvent, Context, ElementId, Entity, InteractiveElement as _, IntoElement,
    ParentElement as _, Stateful, Styled as _, TextAlign, Window, div, prelude::FluentBuilder as _,
    px,
};

use crate::{
    ActiveTheme as _,
    InteractiveElementExt as _,
    input::Input,
    menu::PopupMenu,
    table::{Column, TableDelegate, TableState},
};

use super::{
    ClearContents, ClearFormatting, DeleteCol, DeleteRow, InsertColLeft, InsertColRight,
    InsertRowAbove, InsertRowBelow, col_to_letter,
    data::SpreadsheetData,
};

/// Table delegate for a spreadsheet.
///
/// Column layout:
/// - Column 0: fixed row-number gutter (not selectable, not part of the data grid)
/// - Columns 1..=data.cols: data columns A, B, C, …
pub struct SpreadsheetTableDelegate {
    pub data: Entity<SpreadsheetData>,
}

impl SpreadsheetTableDelegate {
    /// Build a [`TableState`] backed by the given data entity.
    pub fn into_table_state(
        data: Entity<SpreadsheetData>,
        window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> TableState<Self> {
        TableState::new(Self { data }, window, cx)
            .cell_selectable(true)
            .row_selectable(false)
            .col_movable(false)
            .sortable(false)
    }
}

impl TableDelegate for SpreadsheetTableDelegate {
    fn columns_count(&self, cx: &App) -> usize {
        self.data.read(cx).cols + 1 // +1 for row-number gutter
    }

    fn rows_count(&self, cx: &App) -> usize {
        self.data.read(cx).rows
    }

    fn column(&self, col_ix: usize, _cx: &App) -> Column {
        if col_ix == 0 {
            Column::new("_row", "#")
                .width(px(36.0))
                .min_width(px(36.0))
                .max_width(px(36.0))
                .resizable(false)
                .movable(false)
                .selectable(false)
                .text_center()
        } else {
            let letter = col_to_letter(col_ix - 1);
            Column::new(letter.clone(), letter)
                .width(px(120.0))
                .min_width(px(48.0))
        }
    }

    fn render_tr(
        &mut self,
        row_ix: usize,
        _window: &mut Window,
        _cx: &mut Context<TableState<Self>>,
    ) -> Stateful<mozui::Div> {
        div().id(("spreadsheet-row", row_ix))
    }

    fn render_th(
        &mut self,
        col_ix: usize,
        _window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let label = if col_ix == 0 {
            "#".to_string()
        } else {
            col_to_letter(col_ix - 1)
        };
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .text_xs()
            .font_semibold()
            .text_color(cx.theme().muted_foreground)
            .child(label)
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let data = self.data.read(cx);

        // Row-number gutter
        if col_ix == 0 {
            return div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(format!("{}", row_ix + 1))
                .into_any_element();
        }

        let data_col = col_ix - 1;
        let is_editing = data.editing_cell == Some((row_ix, data_col));
        let display = data.display_value(row_ix, data_col);
        let fmt = data.format(row_ix, data_col);
        let input = data.input.clone();
        let _ = data;

        let fg = cx.theme().foreground;
        let is_error = display.starts_with('#')
            && matches!(
                display.as_str(),
                "#DIV/0!" | "#REF!" | "#NAME?" | "#VALUE!" | "#N/A" | "#NULL!" | "#ERROR!"
                    | "#NUM!" | "#GETTING_DATA" | "#SPILL!" | "#CALC!"
            );

        if is_editing {
            // Wrap the borderless Input so it inherits the correct text colour.
            div()
                .size_full()
                .text_color(fg)
                .child(
                    Input::new(&input)
                        .appearance(false)
                        .bordered(false)
                        .focus_bordered(false),
                )
                .into_any_element()
        } else {
            let data_entity = self.data.clone();
            let error_color = mozui::hsla(0.0, 0.7, 0.55, 1.0);

            div()
                .id(ElementId::Name(
                    format!("cell-{}-{}", row_ix, data_col).into(),
                ))
                .size_full()
                .flex()
                .items_center()
                .px_2()
                .text_sm()
                .when(fmt.bold, |d| d.font_bold())
                .when(fmt.italic, |d| d.italic())
                .map(|d| {
                    if is_error {
                        d.text_color(error_color).italic()
                    } else {
                        d.text_color(fmt.text_color.unwrap_or(fg))
                    }
                })
                .when_some(fmt.bg_color, |d, c| d.bg(c))
                .map(|d| match fmt.align {
                    TextAlign::Right => d.justify_end(),
                    TextAlign::Center => d.justify_center(),
                    _ => d.justify_start(),
                })
                .child(display)
                .on_double_click(cx.listener(move |_state, _: &ClickEvent, window, cx| {
                    data_entity.update(cx, |d, cx| {
                        d.start_editing(row_ix, data_col, window, cx);
                    });
                }))
                .into_any_element()
        }
    }

    fn cell_context_menu(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        menu: PopupMenu,
        _window: &mut Window,
        _cx: &mut Context<TableState<Self>>,
    ) -> PopupMenu {
        // col_ix 0 is the gutter — context menu only for data columns.
        if col_ix == 0 {
            return menu;
        }
        let col_label = col_to_letter(col_ix - 1);
        menu.menu(
            format!("Insert row above {}", row_ix + 1),
            Box::new(InsertRowAbove),
        )
        .menu(
            format!("Insert row below {}", row_ix + 1),
            Box::new(InsertRowBelow),
        )
        .menu(format!("Delete row {}", row_ix + 1), Box::new(DeleteRow))
        .separator()
        .menu(
            format!("Insert column before {col_label}"),
            Box::new(InsertColLeft),
        )
        .menu(
            format!("Insert column after {col_label}"),
            Box::new(InsertColRight),
        )
        .menu(format!("Delete column {col_label}"), Box::new(DeleteCol))
        .separator()
        .menu("Clear contents".to_string(), Box::new(ClearContents))
        .menu("Clear formatting".to_string(), Box::new(ClearFormatting))
    }
}
