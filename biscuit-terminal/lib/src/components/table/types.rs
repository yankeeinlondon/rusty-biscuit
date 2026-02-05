use crate::utils::layout::Alignment;

/// The data type of a table column, which drives default alignment
/// and formatting behavior.
#[derive(Debug, Clone, PartialEq)]
pub enum ColumnType {
    /// Free-form text. Default alignment: Left.
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
}

impl Default for ColumnType {
    fn default() -> Self {
        ColumnType::String
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
}
