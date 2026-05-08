use crate::utils::{
    block_constraint::visible_width,
    layout::Alignment,
};

use super::types::Currency;

/// Content for a table cell.
///
/// This enum supports four cell types that each have distinct rendering behavior:
///
/// - **Text**: Renders as-is, supports word wrapping and alignment
/// - **Integer**: Formats with thousands separators (e.g., `1,234,567`)
/// - **Float**: Formats with two decimal places (e.g., `12,345.67`)
/// - **Currency**: Formats with currency symbol prefix and two decimal places (e.g., `$1,234.56`)
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::components::table::TableCellContent;
/// use biscuit_terminal::components::table::types::Currency;
///
/// // Different cell content types
/// let text = TableCellContent::Text("Hello World".into());
/// let integer = TableCellContent::Integer(1234567);
/// let float = TableCellContent::Float(12345.678);
/// let currency = TableCellContent::Currency(Currency::USD, 1234.56);
///
/// // Display formatting
/// assert_eq!(format!("{}", text), "Hello World");
/// assert_eq!(format!("{}", integer), "1,234,567");
/// assert_eq!(format!("{}", float), "12,345.68");
/// assert_eq!(format!("{}", currency), "$1,234.56");
/// ```
///
/// ## Type Conversions
///
/// Convenience `From` implementations allow using primitive types directly:
///
/// ```
/// use biscuit_terminal::components::table::TableCellContent;
///
/// let text: TableCellContent = "hello".into();
/// let num: TableCellContent = 42i64.into();
/// let decimals: TableCellContent = 3.14f64.into();
/// ```
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
pub(super) fn insert_thousands_separators(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    let mut result = String::with_capacity(len + len / 3);
    for (i, ch) in chars.iter().enumerate() {
        if i > 0 && (len - i).is_multiple_of(3) {
            result.push(',');
        }
        result.push(*ch);
    }
    result
}

/// Formats an integer with thousands separators.
pub(super) fn format_integer(value: i64) -> String {
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
pub(super) fn format_float(value: f64) -> String {
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
pub(super) fn format_currency(currency: &Currency, value: f64) -> String {
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
///
/// If `width_for_alignment` is provided, alignment offsets use this width instead
/// of the actual content width. This ensures consistent alignment across rows with
/// mixed-width content (e.g., emoji vs symbols).
pub(super) fn pad_cell(
    content: &str,
    width: usize,
    alignment: Alignment,
    width_for_alignment: Option<usize>,
) -> String {
    let visible = visible_width(content) as usize;
    let align_width = width_for_alignment.unwrap_or(visible);
    match alignment {
        Alignment::Left => {
            // Left align: content at start, padding at end
            let padding = width.saturating_sub(visible);
            format!("{}{}", content, " ".repeat(padding))
        }
        Alignment::Right => {
            // Right align: use align_width for offset calculation
            let offset = width.saturating_sub(align_width);
            let content_padding = offset.saturating_sub(0); // Spaces before content
            let end_padding = width.saturating_sub(offset + visible);
            format!(
                "{}{}{}",
                " ".repeat(content_padding),
                content,
                " ".repeat(end_padding)
            )
        }
        Alignment::Center => {
            // Center align: use align_width for offset calculation
            let total_space = width.saturating_sub(align_width);
            let left_offset = total_space / 2;
            let end_padding = width.saturating_sub(left_offset + visible);
            format!(
                "{}{}{}",
                " ".repeat(left_offset),
                content,
                " ".repeat(end_padding)
            )
        }
    }
}
