use crate::utils::layout::Alignment;
use crate::utils::wrap_policy::WordWrap;

/// Vertical alignment for table cells with multi-line content.
///
/// When cells contain wrapped text or explicit newlines, this controls
/// how content is positioned vertically within the cell. This is a per-column
/// setting that applies to all cells in that column.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::components::table::types::VerticalAlign;
///
/// // Default is Top alignment
/// assert_eq!(VerticalAlign::default(), VerticalAlign::Top);
///
/// // Use with TableColumn::with_vertical_align()
/// // let column = TableColumn::new("Description")
/// //     .with_vertical_align(VerticalAlign::Middle);
/// ```
///
/// ## Behavior
///
/// - **Top** (default): Content aligns to the top, padding added below
/// - **Middle**: Content centered, empty space distributed evenly
/// - **Bottom**: Content aligns to the bottom, padding added above
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VerticalAlign {
    /// Content aligns to the top of the cell, padding added below.
    ///
    /// This is the default alignment and matches typical table behavior.
    #[default]
    Top,
    /// Content is centered vertically within the cell.
    ///
    /// Empty lines are distributed evenly above and below the content.
    Middle,
    /// Content aligns to the bottom of the cell, padding added above.
    ///
    /// Useful when you want content to align with the last line of
    /// multi-line content in adjacent cells.
    Bottom,
}

/// The data type of a table column, which drives default alignment
/// and formatting behavior.
///
/// Column types provide semantic meaning to table data, enabling:
/// - **Default alignment**: Text aligns left, numbers align right
/// - **Word wrapping**: Text columns wrap naturally, numeric columns don't
/// - **Formatting**: Currency adds symbols, floats add decimals
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::components::table::types::{ColumnType, Currency};
/// use biscuit_terminal::utils::layout::{Alignment, WordWrap};
///
/// // String columns are left-aligned by default with word wrap
/// let string_type = ColumnType::String;
/// assert_eq!(string_type.default_alignment(), Alignment::Left);
/// assert!(matches!(string_type.default_word_wrap(), WordWrap::WrapProse(_, _)));
///
/// // Numeric columns are right-aligned, no wrapping
/// let int_type = ColumnType::Integer;
/// assert_eq!(int_type.default_alignment(), Alignment::Right);
/// assert_eq!(int_type.default_word_wrap(), WordWrap::None);
///
/// // Currency columns include symbol and are right-aligned
/// let usd_type = ColumnType::Currency(Currency::USD);
/// assert_eq!(usd_type.default_alignment(), Alignment::Right);
/// assert_eq!(usd_type.default_word_wrap(), WordWrap::None);
///
/// // String columns can have word wrap overridden
/// assert!(string_type.allows_word_wrap_override());
///
/// // Numeric columns forbid word wrap to preserve formatting
/// assert!(!int_type.allows_word_wrap_override());
/// assert!(!usd_type.allows_word_wrap_override());
/// ```
///
/// ## Type-Specific Behavior
///
/// | Type       | Default Alignment | Word Wrap | Use Case                |
/// |------------|------------------|-----------|------------------------|
/// | `String`   | Left             | WrapProse | Text descriptions      |
/// | `Integer`  | Right            | None      | Counts, IDs            |
/// | `Float`    | Right            | None      | Measurements, scores  |
/// | `Currency` | Right            | None      | Money values           |
#[derive(Debug, Clone, PartialEq, Default)]
pub enum ColumnType {
    /// Free-form text. Default alignment: Left.
    #[default]
    String,
    /// Signed integer with thousands separators. Default alignment: Right.
    Integer,
    /// Floating-point number with two decimal places. Default alignment: Right.
    Float,
    /// Currency value with symbol prefix. Default alignment: Right.
    Currency(Currency),
}

impl ColumnType {
    /// Returns the default alignment for this column type.
    pub fn default_alignment(&self) -> Alignment {
        match self {
            ColumnType::String => Alignment::Left,
            ColumnType::Integer | ColumnType::Float | ColumnType::Currency(_) => Alignment::Right,
        }
    }

    /// Returns the default word wrap strategy for this column type.
    ///
    /// Text columns default to `WrapProse` for natural text wrapping.
    /// Numeric columns (Integer, Float, Currency) default to `None` to preserve
    /// number formatting and alignment.
    pub fn default_word_wrap(&self) -> WordWrap {
        match self {
            ColumnType::String => WordWrap::WrapProse(None, None),
            ColumnType::Integer | ColumnType::Float | ColumnType::Currency(_) => WordWrap::None,
        }
    }

    /// Returns whether this column type allows word wrap override.
    ///
    /// Numeric columns (Integer, Float, Currency) force `WordWrap::None` and
    /// do not allow overrides to preserve number formatting.
    pub fn allows_word_wrap_override(&self) -> bool {
        match self {
            ColumnType::String => true,
            ColumnType::Integer | ColumnType::Float | ColumnType::Currency(_) => false,
        }
    }
}

/// Supported currencies with their display symbols.
///
/// Each variant represents a currency that can be used in table columns
/// for formatted monetary values. The symbol is used when rendering
/// the currency in terminal output.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::components::table::types::Currency;
///
/// // Get the symbol for each currency
/// assert_eq!(Currency::USD.symbol(), "$");
/// assert_eq!(Currency::GBP.symbol(), "\u{00a3}"); // Pound sign
/// assert_eq!(Currency::EUR.symbol(), "\u{20ac}"); // Euro sign
/// ```
///
/// ```
/// use biscuit_terminal::components::table::TableColumn;
/// use biscuit_terminal::components::table::types::{Currency, ColumnType};
///
/// // Create a currency column for prices
/// let price_column = TableColumn::new("Price")
///     .with_type(ColumnType::Currency(Currency::USD));
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Currency {
    USD,
    GBP,
    EUR,
}

impl Currency {
    /// Returns the currency symbol.
    pub fn symbol(&self) -> &'static str {
        match self {
            Currency::USD => "$",
            Currency::GBP => "\u{00a3}",
            Currency::EUR => "\u{20ac}",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn column_type_default_alignment() {
        assert_eq!(ColumnType::String.default_alignment(), Alignment::Left);
        assert_eq!(ColumnType::Integer.default_alignment(), Alignment::Right);
        assert_eq!(ColumnType::Float.default_alignment(), Alignment::Right);
        assert_eq!(
            ColumnType::Currency(Currency::USD).default_alignment(),
            Alignment::Right
        );
    }

    #[test]
    fn column_type_default_is_string() {
        assert_eq!(ColumnType::default(), ColumnType::String);
    }

    #[test]
    fn currency_symbols() {
        assert_eq!(Currency::USD.symbol(), "$");
        assert_eq!(Currency::GBP.symbol(), "\u{00a3}");
        assert_eq!(Currency::EUR.symbol(), "\u{20ac}");
    }

    // ── VerticalAlign tests ────────────────────────────────────────

    #[test]
    fn vertical_align_default_is_top() {
        assert_eq!(VerticalAlign::default(), VerticalAlign::Top);
    }

    #[test]
    fn vertical_align_derives_debug_clone_copy() {
        let align = VerticalAlign::Middle;
        let cloned = align;
        let copied = align;
        assert_eq!(format!("{:?}", align), "Middle");
        assert_eq!(cloned, copied);
    }

    #[test]
    fn vertical_align_equality() {
        assert_eq!(VerticalAlign::Top, VerticalAlign::Top);
        assert_ne!(VerticalAlign::Top, VerticalAlign::Middle);
        assert_ne!(VerticalAlign::Middle, VerticalAlign::Bottom);
    }

    // ── default_word_wrap tests ────────────────────────────────────

    #[test]
    fn column_type_string_default_word_wrap() {
        assert_eq!(
            ColumnType::String.default_word_wrap(),
            WordWrap::WrapProse(None, None)
        );
    }

    #[test]
    fn column_type_integer_default_word_wrap() {
        assert_eq!(ColumnType::Integer.default_word_wrap(), WordWrap::None);
    }

    #[test]
    fn column_type_float_default_word_wrap() {
        assert_eq!(ColumnType::Float.default_word_wrap(), WordWrap::None);
    }

    #[test]
    fn column_type_currency_default_word_wrap() {
        assert_eq!(
            ColumnType::Currency(Currency::USD).default_word_wrap(),
            WordWrap::None
        );
    }

    // ── allows_word_wrap_override tests ────────────────────────────

    #[test]
    fn column_type_string_allows_word_wrap_override() {
        assert!(ColumnType::String.allows_word_wrap_override());
    }

    #[test]
    fn column_type_integer_disallows_word_wrap_override() {
        assert!(!ColumnType::Integer.allows_word_wrap_override());
    }

    #[test]
    fn column_type_float_disallows_word_wrap_override() {
        assert!(!ColumnType::Float.allows_word_wrap_override());
    }

    #[test]
    fn column_type_currency_disallows_word_wrap_override() {
        assert!(!ColumnType::Currency(Currency::EUR).allows_word_wrap_override());
    }
}
