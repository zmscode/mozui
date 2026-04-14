pub mod data;
pub mod delegate;

pub use data::{CellFormat, NumberFormat, SpreadsheetData, SpreadsheetEvent};
pub use delegate::SpreadsheetTableDelegate;

use mozui::App;

use crate::actions::{Redo, Undo};

mozui::actions!(
    spreadsheet,
    [
        InsertRowAbove,
        InsertRowBelow,
        DeleteRow,
        InsertColLeft,
        InsertColRight,
        DeleteCol,
        Copy,
        Cut,
        Paste,
        ToggleGridLines,
        RenameSheet,
        DeleteSheet,
        HideSheet,
        UnhideSheet,
        ClearFormatting,
        ClearContents,
    ]
);

pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        mozui::KeyBinding::new("cmd-z", Undo, Some("Spreadsheet")),
        mozui::KeyBinding::new("cmd-shift-z", Redo, Some("Spreadsheet")),
        mozui::KeyBinding::new("cmd-c", Copy, Some("Spreadsheet")),
        mozui::KeyBinding::new("cmd-x", Cut, Some("Spreadsheet")),
        mozui::KeyBinding::new("cmd-v", Paste, Some("Spreadsheet")),
        mozui::KeyBinding::new("backspace", ClearContents, Some("Spreadsheet")),
        mozui::KeyBinding::new("delete", ClearContents, Some("Spreadsheet")),
    ]);
}

/// Convert a 0-indexed column number to spreadsheet letters (A, B, …, Z, AA, …).
pub fn col_to_letter(col: usize) -> String {
    let mut n = col + 1;
    let mut s = String::new();
    while n > 0 {
        n -= 1;
        s.push(char::from_u32('A' as u32 + (n % 26) as u32).unwrap_or('?'));
        n /= 26;
    }
    s.chars().rev().collect()
}
