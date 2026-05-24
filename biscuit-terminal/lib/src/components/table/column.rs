use renderable::style::Style;

use crate::{
    components::prose::Prose,
    utils::{layout::Alignment, wrap_policy::WordWrap},
};

use super::types::{ColumnType, VerticalAlign};

/// Controls whether a table column is visible based on terminal width.
///
/// Columns default to `Always` (unconditionally visible). Use the width
/// variants to hide columns that don't fit in narrow terminals while
/// keeping them visible in wider ones.
///
/// The width checked is the **renderable width** — the terminal width
/// minus any layout margins.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::components::table::{Conditional, TableColumn};
///
/// // Always visible
/// let name = TableColumn::new("Name");
///
/// // Only visible when terminal is wider than 80 columns
/// let notes = TableColumn::new("Notes")
///     .with_when(Conditional::WidthGreaterThan(80));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Conditional {
    /// Column is always visible.
    #[default]
    Always,
    /// Column is visible when the renderable width is greater than the
    /// specified value.
    WidthGreaterThan(u32),
    /// Column is visible when the renderable width is less than or equal
    /// to the specified value.
    LessThanOrEqual(u32),
}

impl Conditional {
    /// Returns `true` when the condition is satisfied for the given width.
    pub fn is_satisfied(&self, available_width: u32) -> bool {
        match self {
            Conditional::Always => true,
            Conditional::WidthGreaterThan(threshold) => available_width > *threshold,
            Conditional::LessThanOrEqual(threshold) => available_width <= *threshold,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
enum DropBehavior {
    #[default]
    Keep,
    DropSilently,
    DropWithMessage(String),
}

/// Column definition for a table.
///
/// A `TableColumn` defines the header, data type, width constraints, alignment,
/// word wrapping, and vertical alignment for a column in a [`Table`].
///
/// ## Width Constraints
///
/// - `fixed_width`: Exact column width (overrides auto-calculation)
/// - `min_width`: Minimum width constraint
/// - `max_width`: Maximum width constraint (triggers word wrap for text)
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::components::table::{Conditional, TableColumn};
/// use biscuit_terminal::components::table::types::{ColumnType, Currency, VerticalAlign};
/// use biscuit_terminal::utils::layout::{Alignment, WordWrap};
///
/// // Simple text column with default settings
/// let name_col = TableColumn::new("Name");
///
/// // Bold header column
/// let header_col = TableColumn::new_with_bold("Status");
///
/// // Numeric column with currency formatting and minimum width
/// let price_col = TableColumn::new("Price")
///     .with_type(ColumnType::Currency(Currency::USD))
///     .with_min_width(12)
///     .with_alignment(Alignment::Right);
///
/// // Text column with word wrap and max width
/// let desc_col = TableColumn::new("Description")
///     .with_max_width(40)
///     .with_word_wrap(WordWrap::WrapProse(Some(4), None))
///     .with_vertical_align(VerticalAlign::Top);
///
/// // Column that hides on narrow terminals
/// let details_col = TableColumn::new("Details")
///     .with_when(Conditional::WidthGreaterThan(80));
/// ```
///
/// ## Conditional Visibility
///
/// Use [`TableColumn::with_when`] to control visibility based on terminal width:
///
/// ```
/// use biscuit_terminal::components::table::{Conditional, TableColumn};
///
/// // Only visible when terminal > 60 columns
/// let narrow_col = TableColumn::new("Notes")
///     .with_when(Conditional::WidthGreaterThan(60));
///
/// // Only visible when terminal <= 40 columns
/// let compact_col = TableColumn::new("Summary")
///     .with_when(Conditional::LessThanOrEqual(40));
/// ```
#[derive(Debug, Clone)]
pub struct TableColumn {
    /// Header text for the column
    pub header: String,
    /// Optional styled header using Prose.
    ///
    /// Compatibility shim: [`new_with_bold`](Self::new_with_bold) still
    /// populates this so existing callers keep working. The canonical typed
    /// home of header appearance is [`header_style`](Self::header_style).
    pub header_prose: Option<Prose>,
    /// Typed appearance slot for this column's header cell (Spec B D5).
    ///
    /// This per-column override is merged on top of the table-wide
    /// [`TableStyle::header`](crate::components::table::types::TableStyle::header)
    /// slot. An empty `Style` leaves the header to the table-wide default.
    pub header_style: Style,
    /// Fixed width for the column (overrides header/data widths when set)
    pub fixed_width: Option<usize>,
    /// Minimum width for the column (optional)
    pub min_width: Option<usize>,
    /// Maximum width for the column (optional)
    pub max_width: Option<usize>,
    /// The data type of this column, which drives default alignment and word wrap
    pub column_type: ColumnType,
    /// Explicit horizontal alignment override (None = use column_type default)
    pub alignment: Option<Alignment>,
    /// Word wrap strategy override (None = use column_type default).
    ///
    /// This is ignored for numeric columns (Integer, Float, Currency) which
    /// force `WordWrap::None` to preserve number formatting.
    pub word_wrap: Option<WordWrap>,
    /// Vertical alignment for multi-line cells in this column
    pub vertical_align: VerticalAlign,
    /// When true, all cells in this column align at the same position regardless
    /// of individual content width. Useful for columns with mixed-width characters
    /// (e.g., emoji ✅ width 2 and symbols ⤫ width 1) that should visually align.
    /// Default is false (traditional alignment where each cell is positioned
    /// based on its own content width).
    pub uniform_alignment: bool,
    /// Controls whether this column is visible based on terminal width.
    ///
    /// Defaults to `Conditional::Always`. Use [`TableColumn::with_when`] to
    /// make a column appear only when the terminal is wide enough.
    pub when: Conditional,
    /// Controls whether this column may be dropped when the table cannot fit.
    drop_behavior: DropBehavior,
}

impl TableColumn {
    /// Create a new column with a header.
    pub fn new<T: Into<String>>(header: T) -> Self {
        TableColumn {
            header: header.into(),
            header_prose: None,
            header_style: Style::default(),
            fixed_width: None,
            min_width: None,
            max_width: None,
            column_type: ColumnType::default(),
            alignment: None,
            word_wrap: None,
            vertical_align: VerticalAlign::default(),
            uniform_alignment: false,
            when: Conditional::default(),
            drop_behavior: DropBehavior::Keep,
        }
    }

    /// Create a new column with a bold header.
    ///
    /// The bold appearance is recorded on the typed
    /// [`header_style`](Self::header_style) slot (Spec B D5/D9). The legacy
    /// [`header_prose`](Self::header_prose) field is also populated as a
    /// compatibility shim.
    pub fn new_with_bold<T: Into<String>>(header: T) -> Self {
        let text = header.into();
        let prose = Prose::new(format!("<bold>{text}</bold>"));
        let header_style = Style {
            emphasis: renderable::style::TextEmphasis {
                bold: true,
                ..Default::default()
            },
            ..Style::default()
        };
        TableColumn {
            header: text,
            header_prose: Some(prose),
            header_style,
            fixed_width: None,
            min_width: None,
            max_width: None,
            column_type: ColumnType::default(),
            alignment: None,
            word_wrap: None,
            vertical_align: VerticalAlign::default(),
            uniform_alignment: false,
            when: Conditional::default(),
            drop_behavior: DropBehavior::Keep,
        }
    }

    /// Set a fixed width for the column.
    pub fn with_fixed_width(mut self, width: usize) -> Self {
        self.fixed_width = Some(width);
        self
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

    /// Set the typed appearance [`Style`] for this column's header cell.
    ///
    /// This per-column override is merged on top of the table-wide
    /// [`TableStyle::header`](crate::components::table::types::TableStyle::header)
    /// slot when the table renders.
    ///
    /// ## Examples
    ///
    /// ```
    /// use biscuit_terminal::components::table::TableColumn;
    /// use renderable::style::{Style, TextEmphasis};
    ///
    /// let col = TableColumn::new("Score").with_header_style(Style {
    ///     emphasis: TextEmphasis { bold: true, ..Default::default() },
    ///     ..Style::default()
    /// });
    /// assert!(col.header_style.emphasis.bold);
    /// ```
    pub fn with_header_style(mut self, style: Style) -> Self {
        self.header_style = style;
        self
    }

    /// Set the word wrap strategy for this column.
    ///
    /// Controls how long text is wrapped when it exceeds the column width.
    /// Text columns default to `WrapProse` for natural word boundaries.
    ///
    /// ## Notes
    ///
    /// This setting is **ignored** for numeric columns (Integer, Float, Currency)
    /// which always use `WordWrap::None` to preserve number formatting and
    /// decimal alignment.
    ///
    /// ## Examples
    ///
    /// ```
    /// use biscuit_terminal::components::table::TableColumn;
    /// use biscuit_terminal::utils::layout::WordWrap;
    ///
    /// // Truncate long text with ellipsis
    /// let col = TableColumn::new("Notes")
    ///     .with_max_width(20)
    ///     .with_word_wrap(WordWrap::Truncate(Some("...".into())));
    /// ```
    pub fn with_word_wrap(mut self, word_wrap: WordWrap) -> Self {
        self.word_wrap = Some(word_wrap);
        self
    }

    /// Set the vertical alignment for multi-line cells in this column.
    ///
    /// When a row contains cells with different numbers of lines, shorter
    /// cells need to be vertically positioned within the row height.
    ///
    /// ## Examples
    ///
    /// ```
    /// use biscuit_terminal::components::table::TableColumn;
    /// use biscuit_terminal::components::table::types::VerticalAlign;
    ///
    /// // Bottom-align numeric values with multi-line descriptions
    /// let amount_col = TableColumn::new("Amount")
    ///     .with_vertical_align(VerticalAlign::Bottom);
    /// ```
    pub fn with_vertical_align(mut self, vertical_align: VerticalAlign) -> Self {
        self.vertical_align = vertical_align;
        self
    }

    /// Enable uniform alignment for this column.
    ///
    /// When enabled, all cells in this column align at the same horizontal
    /// position regardless of individual content width. This is useful for
    /// columns containing mixed-width characters (e.g., emoji ✅ width 2 and
    /// symbols ⤫ width 1) that should visually align in a vertical line.
    ///
    /// Default is `false` (traditional alignment where each cell is positioned
    /// based on its own content width, so numbers right-align properly).
    ///
    /// ## Examples
    ///
    /// ```
    /// use biscuit_terminal::components::table::TableColumn;
    /// use biscuit_terminal::utils::layout::Alignment;
    ///
    /// // Emoji column where ✅ and ⤫ should align vertically
    /// let status_col = TableColumn::new("Status")
    ///     .with_alignment(Alignment::Center)
    ///     .with_uniform_alignment(true);
    /// ```
    pub fn with_uniform_alignment(mut self, uniform: bool) -> Self {
        self.uniform_alignment = uniform;
        self
    }

    /// Set a visibility condition for this column.
    ///
    /// When the condition is not satisfied at render time, the column (and
    /// its corresponding data cells) are excluded from the output.
    ///
    /// ## Examples
    ///
    /// ```
    /// use biscuit_terminal::components::table::{Conditional, TableColumn};
    ///
    /// // Only show this column when terminal is wider than 60
    /// let col = TableColumn::new("Details")
    ///     .with_when(Conditional::WidthGreaterThan(60));
    /// ```
    pub fn with_when(mut self, when: Conditional) -> Self {
        self.when = when;
        self
    }

    /// Allow this column to be dropped when the table cannot fit.
    ///
    /// Passing `Some(message)` records a note that is appended after the
    /// rendered table if this column is dropped. Passing `None` drops the
    /// column silently.
    pub fn drop_when_space_is_limited<T: Into<String>>(mut self, msg: Option<T>) -> Self {
        self.drop_behavior = match msg {
            Some(message) => DropBehavior::DropWithMessage(message.into()),
            None => DropBehavior::DropSilently,
        };
        self
    }

    /// Returns the effective alignment: explicit override or column type default.
    pub fn effective_alignment(&self) -> Alignment {
        self.alignment
            .unwrap_or_else(|| self.column_type.default_alignment())
    }

    /// Returns the effective header [`Style`] for this column.
    ///
    /// The per-column [`header_style`](Self::header_style) override is merged
    /// on top of the table-wide header slot via [`Style::inherited_from`], so
    /// the column's own appearance wins where it is set and the table-wide
    /// slot fills the rest.
    pub fn effective_header_style(&self, table_header: &Style) -> Style {
        self.header_style.inherited_from(table_header)
    }

    /// Returns the effective word wrap strategy.
    ///
    /// For numeric columns, always returns `WordWrap::None` regardless of override.
    /// For text columns, returns the explicit override or the column type default.
    pub fn effective_word_wrap(&self) -> WordWrap {
        if !self.column_type.allows_word_wrap_override() {
            return WordWrap::None;
        }
        self.word_wrap
            .clone()
            .unwrap_or_else(|| self.column_type.default_word_wrap())
    }

    pub(super) fn drop_note(&self) -> Option<String> {
        match &self.drop_behavior {
            DropBehavior::DropWithMessage(message) => Some(message.clone()),
            DropBehavior::Keep | DropBehavior::DropSilently => None,
        }
    }

    pub(super) fn is_droppable(&self) -> bool {
        !matches!(self.drop_behavior, DropBehavior::Keep)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_has_no_header_style() {
        let col = TableColumn::new("Name");
        assert!(col.header_style.is_empty());
    }

    #[test]
    fn new_with_bold_sets_bold_header_style_slot() {
        let col = TableColumn::new_with_bold("Status");
        assert!(
            col.header_style.emphasis.bold,
            "new_with_bold should set the typed bold header slot"
        );
        // The compatibility-shim Prose representation is still populated.
        assert!(col.header_prose.is_some());
    }

    #[test]
    fn with_header_style_overrides_the_slot() {
        let style = renderable::style::Style {
            emphasis: renderable::style::TextEmphasis {
                italic: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let col = TableColumn::new("Name").with_header_style(style.clone());
        assert_eq!(col.header_style, style);
    }
}
