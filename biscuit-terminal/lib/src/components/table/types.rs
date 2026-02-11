use crate::utils::layout::{Alignment, WordWrap};

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
/// ```
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

/// Currency with a display symbol.
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
        let cloned = align.clone();
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
