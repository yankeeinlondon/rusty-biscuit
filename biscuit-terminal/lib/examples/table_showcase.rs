//! Table component showcase
//!
//! Demonstrates all Table capabilities including:
//! - Column types (String, Integer, Float, Currency)
//! - Column alignment (default type-driven and explicit override)
//! - ANSI color codes in cells
//! - OSC8 hyperlinks in cells
//! - Min/max column width constraints
//! - Title rows
//!
//! Run with: cargo run -p biscuit-terminal --example table_showcase

use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::components::table::table::{Table, TableCellContent, TableColumn};
use biscuit_terminal::components::table::types::{ColumnType, Currency};
use biscuit_terminal::utils::layout::Alignment;

fn main() {
    println!("=== Table Component Showcase ===\n");

    // ── 1. Column Types ───────────────────────────────────────────
    println!("--- 1. All Column Types (String, Integer, Float, Currency) ---\n");

    let table = Table::new()
        .with_title("Product Catalog")
        .with_columns(vec![
            TableColumn::new("Product"),
            TableColumn::new("SKU").with_type(ColumnType::Integer),
            TableColumn::new("Weight (kg)").with_type(ColumnType::Float),
            TableColumn::new("Price").with_type(ColumnType::Currency(Currency::USD)),
        ])
        .with_data(vec![
            vec![
                "Widget Pro".into(),
                TableCellContent::Integer(10042),
                TableCellContent::Float(1.5),
                TableCellContent::Currency(Currency::USD, 29.99),
            ],
            vec![
                "Mega Gadget".into(),
                TableCellContent::Integer(20187),
                TableCellContent::Float(12.75),
                TableCellContent::Currency(Currency::USD, 1249.00),
            ],
            vec![
                "Tiny Thing".into(),
                TableCellContent::Integer(30001),
                TableCellContent::Float(0.05),
                TableCellContent::Currency(Currency::USD, 3.50),
            ],
        ]);
    println!("{}\n", table.render(Some(80)));

    // ── 2. Colored Cell Content ───────────────────────────────────
    println!("--- 2. ANSI Colored Cell Content ---\n");

    let table = Table::new()
        .with_columns(vec![
            TableColumn::new("Service"),
            TableColumn::new("Status"),
            TableColumn::new("Uptime"),
        ])
        .with_data(vec![
            vec![
                "API Gateway".into(),
                TableCellContent::Text("\x1b[32m\u{25cf} Online\x1b[0m".into()),
                "99.97%".into(),
            ],
            vec![
                "Database".into(),
                TableCellContent::Text("\x1b[32m\u{25cf} Online\x1b[0m".into()),
                "99.99%".into(),
            ],
            vec![
                "Cache".into(),
                TableCellContent::Text("\x1b[31m\u{25cf} Offline\x1b[0m".into()),
                "87.50%".into(),
            ],
            vec![
                "Worker Pool".into(),
                TableCellContent::Text("\x1b[33m\u{25cf} Degraded\x1b[0m".into()),
                "94.20%".into(),
            ],
        ]);
    println!("{}\n", table.render(Some(80)));

    // ── 3. OSC8 Hyperlinks ────────────────────────────────────────
    println!("--- 3. OSC8 Hyperlinks (clickable in supported terminals) ---\n");

    let rust_link = "\x1b]8;;https://www.rust-lang.org\x07Rust\x1b]8;;\x07";
    let python_link = "\x1b]8;;https://www.python.org\x07Python\x1b]8;;\x07";
    let go_link = "\x1b]8;;https://go.dev\x07Go\x1b]8;;\x07";
    let ts_link = "\x1b]8;;https://www.typescriptlang.org\x07TypeScript\x1b]8;;\x07";

    let table = Table::new()
        .with_title("Programming Languages")
        .with_columns(vec![
            TableColumn::new("Language"),
            TableColumn::new("Year"),
            TableColumn::new("GitHub\nStars").with_type(ColumnType::Integer),
        ])
        .with_data(vec![
            vec![
                TableCellContent::Text(rust_link.into()),
                "2010".into(),
                TableCellContent::Integer(99800),
            ],
            vec![
                TableCellContent::Text(python_link.into()),
                "1991".into(),
                TableCellContent::Integer(64500),
            ],
            vec![
                TableCellContent::Text(go_link.into()),
                "2009".into(),
                TableCellContent::Integer(125000),
            ],
            vec![
                TableCellContent::Text(ts_link.into()),
                "2012".into(),
                TableCellContent::Integer(102000),
            ],
        ])
        .alternate_background_color()
        .alternate_text_color();
    println!("{}\n", table.render(Some(80)));

    // ── 4. Multi-Currency ─────────────────────────────────────────
    println!("--- 4. Multi-Currency ---\n");

    // Note: Column types determine alignment, not cell content types.
    // Use any Currency variant to get right-alignment; the cell content
    // determines the actual currency symbol displayed.
    let table = Table::new()
        .with_title("Exchange Rates (from $1,000 USD)")
        .with_columns(vec![
            TableColumn::new("To"),
            TableColumn::new("Amount").with_type(ColumnType::Currency(Currency::USD)),
            TableColumn::new("Rate").with_type(ColumnType::Float),
        ])
        .with_data(vec![
            vec![
                "GBP".into(),
                TableCellContent::Currency(Currency::GBP, 792.50),
                TableCellContent::Float(0.7925),
            ],
            vec![
                "EUR".into(),
                TableCellContent::Currency(Currency::EUR, 921.30),
                TableCellContent::Float(0.9213),
            ],
            vec![
                "USD".into(),
                TableCellContent::Currency(Currency::USD, 1000.00),
                TableCellContent::Float(1.0),
            ],
        ]);
    println!("{}\n", table.render(Some(80)));

    // ── 5. Center-Aligned Override ─────────────────────────────────
    println!("--- 5. Center-Aligned Override ---\n");

    let table = Table::new()
        .with_columns(vec![
            TableColumn::new("Grade")
                .with_type(ColumnType::String)
                .with_alignment(Alignment::Center)
                .with_min_width(8),
            TableColumn::new("Students")
                .with_type(ColumnType::Integer)
                .with_alignment(Alignment::Center)
                .with_min_width(10),
            TableColumn::new("Percentage")
                .with_alignment(Alignment::Center)
                .with_min_width(12),
        ])
        .with_data(vec![
            vec!["A".into(), TableCellContent::Integer(15), "30%".into()],
            vec!["B".into(), TableCellContent::Integer(23), "46%".into()],
            vec!["C".into(), TableCellContent::Integer(8), "16%".into()],
            vec!["D".into(), TableCellContent::Integer(4), "8%".into()],
        ]);
    println!("{}\n", table.render(Some(80)));
}
