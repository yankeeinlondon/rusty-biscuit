use crate::{
    components::renderable::Renderable,
    terminal::Terminal,
    utils::{
        block_constraint::visible_width,
        layout::{Alignment, Layout},
    },
};

use super::types::{ColumnType, Currency};

/// Content for a table cell.
#[derive(Debug, Clone)]
pub enum TableCellContent {
    /// Text (which can include escape characters)
    Text(String),
    /// Signed integer, formatted with thousands separators
    Integer(i64),
    /// Floating-point number, formatted with two decimal places
    Float(f64),
    /// Currency value with symbol prefix
    Currency(Currency, f64),
}

impl From<String> for TableCellContent {
    fn from(value: String) -> Self {
        TableCellContent::Text(value)
    }
}

impl From<&str> for TableCellContent {
    fn from(value: &str) -> Self {
        TableCellContent::Text(value.to_string())
    }
}

impl From<i64> for TableCellContent {
    fn from(value: i64) -> Self {
        TableCellContent::Integer(value)
    }
}

impl From<f64> for TableCellContent {
    fn from(value: f64) -> Self {
        TableCellContent::Float(value)
    }
}

impl std::fmt::Display for TableCellContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TableCellContent::Text(s) => write!(f, "{}", s),
            TableCellContent::Integer(n) => write!(f, "{}", format_integer(*n)),
            TableCellContent::Float(n) => write!(f, "{}", format_float(*n)),
            TableCellContent::Currency(c, amt) => write!(f, "{}", format_currency(c, *amt)),
        }
    }
}

/// Inserts commas as thousands separators into a numeric string of digits.
fn insert_thousands_separators(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    let mut result = String::with_capacity(len + len / 3);
    for (i, ch) in chars.iter().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            result.push(',');
        }
        result.push(*ch);
    }
    result
}

/// Formats an integer with thousands separators.
fn format_integer(value: i64) -> String {
    let negative = value < 0;
    let s = value.unsigned_abs().to_string();
    let with_commas = insert_thousands_separators(&s);
    if negative {
        format!("-{}", with_commas)
    } else {
        with_commas
    }
}

/// Formats a float with two decimal places and thousands separators on the
/// integer part.
fn format_float(value: f64) -> String {
    if value.is_nan() {
        return "NaN".to_string();
    }
    if value.is_infinite() {
        return if value.is_sign_negative() {
            "-\u{221e}".to_string()
        } else {
            "\u{221e}".to_string()
        };
    }

    let negative = value.is_sign_negative() && value != 0.0;
    let formatted = format!("{:.2}", value.abs());
    // Split at the decimal point to apply thousands separators to integer part
    let (int_part, dec_part) = formatted.split_once('.').unwrap_or((&formatted, "00"));
    let with_commas = insert_thousands_separators(int_part);

    if negative {
        format!("-{}.{}", with_commas, dec_part)
    } else {
        format!("{}.{}", with_commas, dec_part)
    }
}

/// Formats a currency value with symbol and thousands separators.
fn format_currency(currency: &Currency, value: f64) -> String {
    let formatted = format_float(value);
    if value.is_sign_negative() && value != 0.0 && !value.is_nan() {
        // Move negative sign before symbol: -$1,234.56
        format!("-{}{}", currency.symbol(), &formatted[1..])
    } else {
        format!("{}{}", currency.symbol(), formatted)
    }
}

/// Pads cell content to the given width, respecting escape codes.
///
/// Standard `format!("{:width$}", ...)` counts bytes, not visible characters.
/// This function computes the visible width (skipping ANSI escape sequences
/// and using Unicode character widths) and adds the correct number of spaces.
fn pad_cell(content: &str, width: usize, alignment: Alignment) -> String {
    let visible = visible_width(content) as usize;
    let padding = width.saturating_sub(visible);
    match alignment {
        Alignment::Left => format!("{}{}", content, " ".repeat(padding)),
        Alignment::Right => format!("{}{}", " ".repeat(padding), content),
        Alignment::Center => {
            let left = padding / 2;
            let right = padding - left;
            format!("{}{}{}", " ".repeat(left), content, " ".repeat(right))
        }
    }
}

/// Column definition for a table.
#[derive(Debug, Clone)]
pub struct TableColumn {
    /// Header text for the column
    pub header: String,
    /// Minimum width for the column (optional)
    pub min_width: Option<usize>,
    /// Maximum width for the column (optional)
    pub max_width: Option<usize>,
    /// The data type of this column, which drives default alignment
    pub column_type: ColumnType,
    /// Explicit alignment override (None = use column_type default)
    pub alignment: Option<Alignment>,
}

impl TableColumn {
    /// Create a new column with a header.
    pub fn new<T: Into<String>>(header: T) -> Self {
        TableColumn {
            header: header.into(),
            min_width: None,
            max_width: None,
            column_type: ColumnType::default(),
            alignment: None,
        }
    }

    /// Set minimum width for the column.
    pub fn with_min_width(mut self, width: usize) -> Self {
        self.min_width = Some(width);
        self
    }

    /// Set maximum width for the column.
    pub fn with_max_width(mut self, width: usize) -> Self {
        self.max_width = Some(width);
        self
    }

    /// Set the column data type.
    pub fn with_type(mut self, column_type: ColumnType) -> Self {
        self.column_type = column_type;
        self
    }

    /// Set an explicit alignment override.
    pub fn with_alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = Some(alignment);
        self
    }

    /// Returns the effective alignment: explicit override or column type default.
    pub fn effective_alignment(&self) -> Alignment {
        self.alignment
            .unwrap_or_else(|| self.column_type.default_alignment())
    }
}

/// A table component for rendering tabular data.
#[derive(Debug)]
pub struct Table {
    title: Option<String>,
    columns: Vec<TableColumn>,
    data: Vec<Vec<TableCellContent>>,
    layout: Layout,
}

impl Default for Table {
    fn default() -> Self {
        Table {
            title: None,
            columns: Vec::new(),
            data: Vec::new(),
            layout: Layout::default(),
        }
    }
}

impl Table {
    /// Create a new empty table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the table title.
    pub fn with_title<T: Into<String>>(mut self, title: T) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the columns.
    pub fn with_columns(mut self, columns: Vec<TableColumn>) -> Self {
        self.columns = columns;
        self
    }

    /// Add a row of data.
    pub fn add_row(&mut self, row: Vec<TableCellContent>) {
        self.data.push(row);
    }

    /// Set all data rows.
    pub fn with_data(mut self, data: Vec<Vec<TableCellContent>>) -> Self {
        self.data = data;
        self
    }

    /// Calculate column widths based on content.
    fn calculate_column_widths(&self) -> Vec<usize> {
        let num_cols = self
            .columns
            .len()
            .max(self.data.iter().map(|row| row.len()).max().unwrap_or(0));

        let mut widths = vec![0; num_cols];

        // Consider header widths
        for (i, col) in self.columns.iter().enumerate() {
            widths[i] = visible_width(&col.header) as usize;
            if let Some(min) = col.min_width {
                widths[i] = widths[i].max(min);
            }
        }

        // Consider data widths
        for row in &self.data {
            for (i, cell) in row.iter().enumerate() {
                if i < widths.len() {
                    let cell_width = visible_width(&cell.to_string()) as usize;
                    widths[i] = widths[i].max(cell_width);
                }
            }
        }

        // Apply max width constraints
        for (i, col) in self.columns.iter().enumerate() {
            if let Some(max) = col.max_width {
                if i < widths.len() {
                    widths[i] = widths[i].min(max);
                }
            }
        }

        widths
    }

    /// Render the table content.
    fn render_content(&self, _term: Option<&Terminal>) -> String {
        let mut result = String::new();
        let widths = self.calculate_column_widths();

        if let Some(ref title) = self.title {
            result.push_str(title);
            result.push('\n');
        }

        if !widths.is_empty() {
            let top_border = build_border(&widths, '┌', '┬', '┐');
            result.push_str(&top_border);
            result.push('\n');
        }

        // Render header row
        if !self.columns.is_empty() {
            let mut header_row = String::from("│ ");
            for (i, col) in self.columns.iter().enumerate() {
                let width = widths.get(i).copied().unwrap_or(col.header.len());
                header_row.push_str(&pad_cell(&col.header, width, Alignment::Left));
                if i < self.columns.len() - 1 {
                    header_row.push_str(" │ ");
                }
            }
            header_row.push_str(" │");
            result.push_str(&header_row);
            result.push('\n');

            // Render separator
            let separator = build_border(&widths, '├', '┼', '┤');
            result.push_str(&separator);
            result.push('\n');
        }

        // Render data rows
        for row in &self.data {
            let mut row_str = String::from("│ ");
            for (i, cell) in row.iter().enumerate() {
                let width = widths.get(i).copied().unwrap_or(0);
                let content = cell.to_string();
                let alignment = self
                    .columns
                    .get(i)
                    .map(|col| col.effective_alignment())
                    .unwrap_or(Alignment::Left);
                row_str.push_str(&pad_cell(&content, width, alignment));
                if i < row.len() - 1 {
                    row_str.push_str(" │ ");
                }
            }
            row_str.push_str(" │");
            result.push_str(&row_str);
            result.push('\n');
        }

        if !widths.is_empty() {
            let bottom_border = build_border(&widths, '└', '┴', '┘');
            result.push_str(&bottom_border);
            result.push('\n');
        }

        // Remove trailing newline
        if result.ends_with('\n') {
            result.pop();
        }

        result
    }
}

impl Renderable for Table {
    fn render(&self, term_width: Option<u32>) -> String {
        let width = term_width.unwrap_or(80);
        let content = self.render_content(None);
        self.layout.apply_layout(&content, width)
    }

    fn fallback_render(&self, term: &Terminal) -> String {
        let width = term.width();
        let content = self.render_content(Some(term));
        self.layout.apply_layout(&content, width)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn layout(&self) -> &Layout {
        &self.layout
    }

    fn layout_mut(&mut self) -> &mut Layout {
        &mut self.layout
    }

    fn is_block_level(&self) -> bool {
        true
    }
}

fn build_border(widths: &[usize], left: char, junction: char, right: char) -> String {
    if widths.is_empty() {
        return String::new();
    }

    let mut border = String::from(left);
    border.push('─');
    for (i, width) in widths.iter().enumerate() {
        border.push_str(&"─".repeat(*width));
        if i < widths.len() - 1 {
            border.push('─');
            border.push(junction);
            border.push('─');
        }
    }
    border.push('─');
    border.push(right);
    border
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Existing tests ────────────────────────────────────────────

    #[test]
    fn test_simple_table() {
        let table = Table::new()
            .with_columns(vec![TableColumn::new("Name"), TableColumn::new("Age")])
            .with_data(vec![
                vec!["Alice".into(), "30".into()],
                vec!["Bob".into(), "25".into()],
            ]);

        let result = table.render(None);
        assert!(result.contains("Name"));
        assert!(result.contains("Alice"));
        assert!(result.contains("Bob"));
    }

    #[test]
    fn test_table_with_title() {
        let table = Table::new()
            .with_title("Users")
            .with_columns(vec![TableColumn::new("Name")]);

        let result = table.render(None);
        assert!(result.starts_with("Users\n"));
    }

    #[test]
    fn test_empty_table() {
        let table = Table::new();
        let result = table.render(None);
        assert_eq!(result, "");
    }

    // ── Formatting tests ──────────────────────────────────────────

    #[test]
    fn test_insert_thousands_separators() {
        assert_eq!(insert_thousands_separators("1234567"), "1,234,567");
        assert_eq!(insert_thousands_separators("123"), "123");
        assert_eq!(insert_thousands_separators("1234"), "1,234");
        assert_eq!(insert_thousands_separators("0"), "0");
        assert_eq!(insert_thousands_separators("12"), "12");
    }

    #[test]
    fn test_format_integer_positive() {
        assert_eq!(format_integer(1_234_567), "1,234,567");
        assert_eq!(format_integer(42), "42");
        assert_eq!(format_integer(999), "999");
        assert_eq!(format_integer(1000), "1,000");
    }

    #[test]
    fn test_format_integer_negative() {
        assert_eq!(format_integer(-42), "-42");
        assert_eq!(format_integer(-1234), "-1,234");
    }

    #[test]
    fn test_format_integer_zero() {
        assert_eq!(format_integer(0), "0");
    }

    #[test]
    fn test_format_integer_large() {
        assert_eq!(format_integer(1_000_000_000), "1,000,000,000");
    }

    #[test]
    fn test_format_float_normal() {
        assert_eq!(format_float(1234.5), "1,234.50");
        assert_eq!(format_float(3.14), "3.14");
        assert_eq!(format_float(0.5), "0.50");
    }

    #[test]
    fn test_format_float_zero() {
        assert_eq!(format_float(0.0), "0.00");
    }

    #[test]
    fn test_format_float_nan() {
        assert_eq!(format_float(f64::NAN), "NaN");
    }

    #[test]
    fn test_format_float_infinity() {
        assert_eq!(format_float(f64::INFINITY), "\u{221e}");
        assert_eq!(format_float(f64::NEG_INFINITY), "-\u{221e}");
    }

    #[test]
    fn test_format_currency_usd() {
        assert_eq!(format_currency(&Currency::USD, 1234.56), "$1,234.56");
    }

    #[test]
    fn test_format_currency_gbp() {
        assert_eq!(format_currency(&Currency::GBP, 99.0), "\u{00a3}99.00");
    }

    #[test]
    fn test_format_currency_eur() {
        assert_eq!(format_currency(&Currency::EUR, 42.0), "\u{20ac}42.00");
    }

    #[test]
    fn test_format_currency_negative() {
        assert_eq!(format_currency(&Currency::USD, -1234.56), "-$1,234.56");
    }

    #[test]
    fn test_format_currency_zero() {
        assert_eq!(format_currency(&Currency::USD, 0.0), "$0.00");
    }

    #[test]
    fn test_display_impl() {
        assert_eq!(TableCellContent::Integer(42).to_string(), "42");
        assert_eq!(TableCellContent::Float(3.14).to_string(), "3.14");
        assert_eq!(
            TableCellContent::Currency(Currency::USD, 9.99).to_string(),
            "$9.99"
        );
        assert_eq!(
            TableCellContent::Text("hello".to_string()).to_string(),
            "hello"
        );
    }

    #[test]
    fn test_from_i64() {
        let cell = TableCellContent::from(42i64);
        assert!(matches!(cell, TableCellContent::Integer(42)));
    }

    #[test]
    fn test_from_f64() {
        let cell = TableCellContent::from(3.14f64);
        assert!(matches!(cell, TableCellContent::Float(v) if (v - 3.14).abs() < f64::EPSILON));
    }

    // ── pad_cell tests ────────────────────────────────────────────

    #[test]
    fn test_pad_cell_left_alignment() {
        assert_eq!(pad_cell("hello", 10, Alignment::Left), "hello     ");
    }

    #[test]
    fn test_pad_cell_right_alignment() {
        assert_eq!(pad_cell("hello", 10, Alignment::Right), "     hello");
    }

    #[test]
    fn test_pad_cell_center_alignment() {
        assert_eq!(pad_cell("hi", 10, Alignment::Center), "    hi    ");
    }

    #[test]
    fn test_pad_cell_content_wider_than_width() {
        // Content wider than target: no crash, no truncation
        assert_eq!(pad_cell("hello world", 5, Alignment::Left), "hello world");
    }

    #[test]
    fn test_pad_cell_with_ansi_colors() {
        let colored = "\x1b[31mred\x1b[0m"; // "red" in red (3 visible chars)
        let padded = pad_cell(colored, 10, Alignment::Left);
        assert_eq!(visible_width(&padded), 10);
        // Should have 7 trailing spaces
        assert!(padded.ends_with("       "));
        assert!(padded.starts_with("\x1b[31mred\x1b[0m"));
    }

    #[test]
    fn test_pad_cell_with_osc8_link() {
        let link = "\x1b]8;;https://example.com\x07click\x1b]8;;\x07"; // "click" = 5 visible
        let padded = pad_cell(link, 10, Alignment::Right);
        assert_eq!(visible_width(&padded), 10);
        // 5 spaces of left padding for right-align
        assert!(padded.starts_with("     "));
    }

    #[test]
    fn test_pad_cell_with_mixed_escape_and_osc8() {
        // Bold + OSC8 link: "\x1b[1m" + OSC8 link + "\x1b[0m"
        let mixed = "\x1b[1m\x1b]8;;https://rust-lang.org\x07Rust\x1b]8;;\x07\x1b[0m";
        // "Rust" = 4 visible chars
        let padded = pad_cell(mixed, 10, Alignment::Left);
        assert_eq!(visible_width(&padded), 10);
    }

    // ── Table rendering with escape codes ─────────────────────────

    #[test]
    fn test_table_with_colored_cells_alignment() {
        let table = Table::new()
            .with_columns(vec![TableColumn::new("Name"), TableColumn::new("Status")])
            .with_data(vec![
                vec![
                    "Alice".into(),
                    TableCellContent::Text("\x1b[32mActive\x1b[0m".to_string()),
                ],
                vec!["Bob".into(), TableCellContent::Text("Inactive".to_string())],
            ]);

        let result = table.render_content(None);
        let lines: Vec<&str> = result.lines().collect();
        // Top border, header, separator, row1, row2, bottom border
        assert_eq!(lines.len(), 6);
        // All content lines should have the same visible width
        let header_width = visible_width(lines[1]);
        let row1_width = visible_width(lines[3]);
        let row2_width = visible_width(lines[4]);
        assert_eq!(header_width, row1_width);
        assert_eq!(header_width, row2_width);
    }

    #[test]
    fn test_table_with_osc8_links() {
        let link = "\x1b]8;;https://rust-lang.org\x07Rust\x1b]8;;\x07";
        let table = Table::new()
            .with_columns(vec![TableColumn::new("Language"), TableColumn::new("Year")])
            .with_data(vec![
                vec![TableCellContent::Text(link.to_string()), "2010".into()],
                vec!["Python".into(), "1991".into()],
            ]);

        let result = table.render_content(None);
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines.len(), 6);
        let header_width = visible_width(lines[1]);
        let row1_width = visible_width(lines[3]);
        let row2_width = visible_width(lines[4]);
        assert_eq!(header_width, row1_width);
        assert_eq!(header_width, row2_width);
    }

    #[test]
    fn test_table_with_empty_cells() {
        let table = Table::new()
            .with_columns(vec![TableColumn::new("Name"), TableColumn::new("Value")])
            .with_data(vec![
                vec!["Item".into(), "".into()],
                vec!["".into(), "42".into()],
            ]);

        let result = table.render_content(None);
        let lines: Vec<&str> = result.lines().collect();
        let header_width = visible_width(lines[1]);
        let row1_width = visible_width(lines[3]);
        let row2_width = visible_width(lines[4]);
        assert_eq!(header_width, row1_width);
        assert_eq!(header_width, row2_width);
    }

    // ── Alignment tests ───────────────────────────────────────────

    #[test]
    fn test_numeric_column_right_alignment() {
        let table = Table::new()
            .with_columns(vec![
                TableColumn::new("Item"),
                TableColumn::new("Price").with_type(ColumnType::Currency(Currency::USD)),
            ])
            .with_data(vec![
                vec![
                    "Widget".into(),
                    TableCellContent::Currency(Currency::USD, 9.99),
                ],
                vec![
                    "Gadget".into(),
                    TableCellContent::Currency(Currency::USD, 1234.56),
                ],
            ]);

        let result = table.render_content(None);
        let lines: Vec<&str> = result.lines().collect();
        // First data row: "$9.99" should be right-padded to match "$1,234.56"
        let data_line_1 = lines[3];
        // The shorter value should have leading spaces (right-aligned)
        assert!(data_line_1.contains("    $9.99"));
    }

    #[test]
    fn test_explicit_alignment_override() {
        let col = TableColumn::new("ID")
            .with_type(ColumnType::Integer)
            .with_alignment(Alignment::Center);
        assert_eq!(col.effective_alignment(), Alignment::Center);
    }

    #[test]
    fn test_default_string_left_alignment() {
        let col = TableColumn::new("Name");
        assert_eq!(col.effective_alignment(), Alignment::Left);
    }

    #[test]
    fn test_default_integer_right_alignment() {
        let col = TableColumn::new("Count").with_type(ColumnType::Integer);
        assert_eq!(col.effective_alignment(), Alignment::Right);
    }

    // ── Regression: table without title should not emit a title line ──

    #[test]
    fn test_table_without_title_has_no_title_line() {
        let table = Table::new()
            .with_columns(vec![TableColumn::new("Name"), TableColumn::new("Age")])
            .with_data(vec![vec!["Alice".into(), "30".into()]]);

        let result = table.render_content(None);
        // First line must be the top border, not a title
        assert!(
            result.starts_with('┌'),
            "Table without title should start with top border, got: {}",
            result.lines().next().unwrap_or("")
        );
    }

    // ── Regression: left margin is applied to every line ──

    #[test]
    fn test_table_with_left_margin() {
        use crate::utils::layout::Margin;

        let mut table = Table::new()
            .with_columns(vec![TableColumn::new("X")])
            .with_data(vec![vec!["A".into()]]);
        table.layout_mut().left_margin = Margin::Chars(1);

        let rendered = table.render(Some(60));
        for line in rendered.lines() {
            assert!(
                line.starts_with(' '),
                "Every line should have left margin padding, got: {:?}",
                line
            );
        }
    }
}
