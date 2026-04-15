mod support;

use mozui::prelude::*;
use mozui::{Context, Entity, TextAlign, Window, div, hsla, px, size};
use mozui_components::{
    spreadsheet::{CellFormat, NumberFormat, Spreadsheet, SpreadsheetData},
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
    spreadsheet: Entity<Spreadsheet>,
}

impl SpreadsheetExample {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let data = cx.new(|cx| {
            let mut d = SpreadsheetData::new(22, 7, window, cx);
            populate_budget(&mut d);
            d.snapshot_clear_history();
            d
        });

        let spreadsheet = cx.new(|cx| Spreadsheet::full(data, window, cx));

        Self { spreadsheet }
    }
}

impl Render for SpreadsheetExample {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(self.spreadsheet.clone())
    }
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
