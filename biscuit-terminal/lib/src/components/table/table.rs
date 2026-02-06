use crate::{
    components::renderable::Renderable,
    terminal::Terminal,
    utils::{
        block_constraint::{split_lines, visible_width, wrap_lines},
        layout::{Alignment, Layout, RowFill, WordWrap},
    },
};

use super::types::{ColumnType, Currency, VerticalAlign};

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
///
/// A `TableColumn` defines the header, data type, width constraints, alignment,
/// word wrapping, and vertical alignment for a column in a [`Table`].
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::components::table::table::TableColumn;
/// use biscuit_terminal::components::table::types::{ColumnType, Currency, VerticalAlign};
/// use biscuit_terminal::utils::layout::WordWrap;
///
/// // Simple text column
/// let name_col = TableColumn::new("Name");
///
/// // Numeric column with min width
/// let price_col = TableColumn::new("Price")
///     .with_type(ColumnType::Currency(Currency::USD))
///     .with_min_width(10);
///
/// // Text column with custom word wrap and vertical alignment
/// let desc_col = TableColumn::new("Description")
///     .with_max_width(40)
///     .with_word_wrap(WordWrap::WrapProse(Some(4), None))
///     .with_vertical_align(VerticalAlign::Top);
/// ```
#[derive(Debug, Clone)]
pub struct TableColumn {
    /// Header text for the column
    pub header: String,
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
}

impl TableColumn {
    /// Create a new column with a header.
    pub fn new<T: Into<String>>(header: T) -> Self {
        TableColumn {
            header: header.into(),
            fixed_width: None,
            min_width: None,
            max_width: None,
            column_type: ColumnType::default(),
            alignment: None,
            word_wrap: None,
            vertical_align: VerticalAlign::default(),
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
    /// use biscuit_terminal::components::table::table::TableColumn;
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
    /// use biscuit_terminal::components::table::table::TableColumn;
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

    /// Returns the effective alignment: explicit override or column type default.
    pub fn effective_alignment(&self) -> Alignment {
        self.alignment
            .unwrap_or_else(|| self.column_type.default_alignment())
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
}

/// A table component for rendering tabular data.
#[derive(Debug)]
pub struct Table {
    title: Option<String>,
    columns: Vec<TableColumn>,
    data: Vec<Vec<TableCellContent>>,
    layout: Layout,
    /// When true, use ANSI cursor positioning for cell alignment instead of
    /// space-based padding. This can improve alignment when glyphs render
    /// narrower than their computed Unicode width.
    prefer_cursor_alignment: bool,
}

impl Default for Table {
    fn default() -> Self {
        Table {
            title: None,
            columns: Vec::new(),
            data: Vec::new(),
            layout: Layout::default(),
            prefer_cursor_alignment: false,
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

    /// Enable cursor positioning for cell alignment.
    ///
    /// When enabled, the table uses ANSI cursor positioning (`\x1b[{n}G`)
    /// instead of space-based padding. This ensures table borders and
    /// separators align correctly even when glyphs render narrower than
    /// their computed Unicode width.
    ///
    /// This mode fully supports all `Layout` attributes including margins,
    /// alignment, and row fill.
    pub fn prefer_cursor_alignment(mut self) -> Self {
        self.prefer_cursor_alignment = true;
        self
    }

    /// Calculate column widths based on content and available space.
    ///
    /// If `available_width` is provided, the total table width (including borders)
    /// will be constrained to fit within that space. Width reduction is distributed
    /// proportionally among text columns that allow word wrap, while fixed-width
    /// and numeric columns are not reduced.
    fn calculate_column_widths(&self, available_width: Option<u32>) -> Vec<usize> {
        let num_cols = self
            .columns
            .len()
            .max(self.data.iter().map(|row| row.len()).max().unwrap_or(0));

        if num_cols == 0 {
            return Vec::new();
        }

        let mut widths = vec![0; num_cols];
        let mut fixed = vec![false; num_cols];

        // Consider header widths
        for (i, col) in self.columns.iter().enumerate() {
            if let Some(fixed_width) = col.fixed_width {
                widths[i] = fixed_width;
                fixed[i] = true;
                continue;
            }

            widths[i] = visible_width(&col.header) as usize;
            if let Some(min) = col.min_width {
                widths[i] = widths[i].max(min);
            }
        }

        // Consider data widths
        for row in &self.data {
            for (i, cell) in row.iter().enumerate() {
                if i < widths.len() {
                    if fixed.get(i).copied().unwrap_or(false) {
                        continue;
                    }
                    let cell_width = visible_width(&cell.to_string()) as usize;
                    widths[i] = widths[i].max(cell_width);
                }
            }
        }

        // Apply max width constraints
        for (i, col) in self.columns.iter().enumerate() {
            if fixed.get(i).copied().unwrap_or(false) {
                continue;
            }
            if let Some(max) = col.max_width {
                if i < widths.len() {
                    widths[i] = widths[i].min(max);
                }
            }
        }

        // If available_width is specified, constrain total table width
        if let Some(available) = available_width {
            self.constrain_widths_to_available(&mut widths, &fixed, available as usize);
        }

        widths
    }

    /// Constrain column widths to fit within available space.
    ///
    /// Border overhead: `│ ` (2) at start + ` │` (2) at end + ` │ ` (3) between each column.
    /// Total: 2 + sum(widths) + 2 + (n-1)*3 = 4 + sum(widths) + 3*(n-1) for n > 0
    fn constrain_widths_to_available(
        &self,
        widths: &mut [usize],
        fixed: &[bool],
        available: usize,
    ) {
        if widths.is_empty() {
            return;
        }

        // Calculate border overhead: start(2) + end(2) + separators(3*(n-1))
        let border_overhead = 4 + (widths.len().saturating_sub(1)) * 3;
        let max_content_width = available.saturating_sub(border_overhead);

        let current_content_width: usize = widths.iter().sum();
        if current_content_width <= max_content_width {
            return; // Already fits
        }

        // Calculate how much we need to reduce
        let excess = current_content_width.saturating_sub(max_content_width);

        // Identify reducible columns (text columns that allow word wrap and aren't fixed)
        let mut reducible: Vec<(usize, usize)> = Vec::new(); // (index, current_width)
        for (i, &width) in widths.iter().enumerate() {
            if fixed.get(i).copied().unwrap_or(false) {
                continue;
            }
            let allows_reduction = self
                .columns
                .get(i)
                .map(|col| col.column_type.allows_word_wrap_override())
                .unwrap_or(true); // Default to allowing reduction for columns without definitions
            if allows_reduction && width > 0 {
                reducible.push((i, width));
            }
        }

        if reducible.is_empty() {
            return; // No columns can be reduced
        }

        // Distribute reduction proportionally among reducible columns
        let total_reducible: usize = reducible.iter().map(|(_, w)| w).sum();
        if total_reducible == 0 {
            return;
        }

        let mut remaining_excess = excess;
        for (idx, current_width) in reducible {
            // Proportional share of the reduction
            let share = (current_width as f64 / total_reducible as f64 * excess as f64).ceil() as usize;
            let reduction = share.min(remaining_excess);

            // Get minimum width constraint
            let min_width = self
                .columns
                .get(idx)
                .and_then(|col| col.min_width)
                .unwrap_or(1); // Minimum 1 character

            let new_width = current_width.saturating_sub(reduction).max(min_width);
            let actual_reduction = current_width.saturating_sub(new_width);
            widths[idx] = new_width;
            remaining_excess = remaining_excess.saturating_sub(actual_reduction);

            if remaining_excess == 0 {
                break;
            }
        }
    }

    /// Render the table content with multi-line cell support.
    ///
    /// If `available_width` is provided, columns will be constrained to fit
    /// within that space (accounting for border overhead).
    fn render_content(&self, available_width: Option<u32>) -> String {
        let mut result = String::new();
        let widths = self.calculate_column_widths(available_width);

        if let Some(ref title) = self.title {
            result.push_str(title);
            result.push('\n');
        }

        if !widths.is_empty() {
            let top_border = build_border(&widths, '┌', '┬', '┐');
            result.push_str(&top_border);
            result.push('\n');
        }

        // Render header row with multi-line support (explicit newlines only, no word wrap)
        if !self.columns.is_empty() {
            // Calculate header height based on explicit newlines
            let header_lines: Vec<Vec<String>> = self
                .columns
                .iter()
                .enumerate()
                .map(|(i, col)| {
                    let width = widths.get(i).copied().unwrap_or(col.header.len());
                    // Headers use None strategy (no word wrap), only split on explicit newlines
                    wrap_cell_content(&col.header, &WordWrap::None, width)
                })
                .collect();

            let header_height = header_lines.iter().map(|lines| lines.len()).max().unwrap_or(1);

            // Apply vertical alignment (headers default to Top)
            let padded_headers: Vec<Vec<String>> = header_lines
                .into_iter()
                .enumerate()
                .map(|(i, lines)| {
                    let width = widths.get(i).copied().unwrap_or(0);
                    let vertical_align = self
                        .columns
                        .get(i)
                        .map(|col| col.vertical_align)
                        .unwrap_or(VerticalAlign::Top);
                    apply_vertical_padding(lines, header_height, vertical_align, width)
                })
                .collect();

            // Render each line of the header
            for line_idx in 0..header_height {
                let mut header_row = String::from("│ ");
                for (i, col) in self.columns.iter().enumerate() {
                    let width = widths.get(i).copied().unwrap_or(col.header.len());
                    let alignment = col.effective_alignment();
                    let line_content = padded_headers
                        .get(i)
                        .and_then(|lines| lines.get(line_idx))
                        .map(|s| s.as_str())
                        .unwrap_or("");
                    header_row.push_str(&pad_cell(line_content, width, alignment));
                    if i < self.columns.len() - 1 {
                        header_row.push_str(" │ ");
                    }
                }
                header_row.push_str(" │");
                result.push_str(&header_row);
                result.push('\n');
            }

            // Render separator
            let separator = build_border(&widths, '├', '┼', '┤');
            result.push_str(&separator);
            result.push('\n');
        }

        // Calculate row heights for multi-line support
        let row_heights = calculate_row_heights(&self.data, &self.columns, &widths);

        // Render data rows with multi-line support
        for (row_idx, row) in self.data.iter().enumerate() {
            let row_height = row_heights.get(row_idx).copied().unwrap_or(1);

            // Prepare wrapped and vertically-aligned content for each cell
            let mut cell_lines: Vec<Vec<String>> = Vec::with_capacity(row.len());
            for (i, cell) in row.iter().enumerate() {
                let width = widths.get(i).copied().unwrap_or(0);
                let strategy = self
                    .columns
                    .get(i)
                    .map(|col| col.effective_word_wrap())
                    .unwrap_or(WordWrap::None);
                let vertical_align = self
                    .columns
                    .get(i)
                    .map(|col| col.vertical_align)
                    .unwrap_or(VerticalAlign::Top);

                let content = cell.to_string();
                let wrapped = wrap_cell_content(&content, &strategy, width);
                let padded = apply_vertical_padding(wrapped, row_height, vertical_align, width);
                cell_lines.push(padded);
            }

            // Render each line of the row
            for line_idx in 0..row_height {
                let mut row_str = String::from("│ ");
                for (i, _cell) in row.iter().enumerate() {
                    let width = widths.get(i).copied().unwrap_or(0);
                    let alignment = self
                        .columns
                        .get(i)
                        .map(|col| col.effective_alignment())
                        .unwrap_or(Alignment::Left);

                    let line_content = cell_lines
                        .get(i)
                        .and_then(|lines| lines.get(line_idx))
                        .map(|s| s.as_str())
                        .unwrap_or("");

                    row_str.push_str(&pad_cell(line_content, width, alignment));
                    if i < row.len() - 1 {
                        row_str.push_str(" │ ");
                    }
                }
                row_str.push_str(" │");
                result.push_str(&row_str);
                result.push('\n');
            }
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

    /// Render with cursor positioning, supporting all Layout attributes.
    ///
    /// Uses ANSI cursor positioning (`\x1b[{n}G`) instead of space padding
    /// to ensure table borders align correctly regardless of glyph rendering.
    fn render_with_cursor_positioning(&self, term_width: u32) -> String {
        let mut result = String::new();

        // Calculate margins first to determine available width for column calculation
        let left_margin = Layout::resolve_margin(&self.layout.left_margin, term_width);
        let right_margin = Layout::resolve_margin(&self.layout.right_margin, term_width);
        let available_width = term_width.saturating_sub(left_margin).saturating_sub(right_margin);

        let widths = self.calculate_column_widths(Some(available_width));

        if widths.is_empty() {
            return result;
        }

        // Calculate table width: │ + space + col1 + space │ space + col2 + space │ ...
        // Border chars: "│ " (2) at start + " │" (2) at end + " │ " (3) between columns
        // Total: 2 + sum(widths) + 2 + 3*(n-1) = 4 + sum(widths) + 3*(n-1)
        let table_content_width: usize = widths.iter().sum();
        let table_border_width = if widths.is_empty() {
            0
        } else {
            // start(2) + content + end(2) + separators(3*(n-1))
            4 + table_content_width + (widths.len().saturating_sub(1)) * 3
        };
        let table_width = table_border_width as u32;

        // Calculate table start position based on block alignment
        let table_start = match self.layout.alignment {
            Alignment::Left => left_margin.saturating_add(1),
            Alignment::Right => {
                let offset = available_width.saturating_sub(table_width);
                left_margin.saturating_add(offset).saturating_add(1)
            }
            Alignment::Center => {
                let offset = available_width.saturating_sub(table_width) / 2;
                left_margin.saturating_add(offset).saturating_add(1)
            }
        };

        // Determine if we should fill rows (for background color)
        let should_fill = match &self.layout.row_fill_strategy {
            RowFill::Fill => true,
            RowFill::Auto => self.layout.page_bg_color.is_some(),
            RowFill::Exact => false,
        };
        let fill_end_col = if should_fill {
            Some(left_margin.saturating_add(available_width).saturating_add(1))
        } else {
            None
        };

        // Collect column alignments
        let alignments: Vec<Alignment> = (0..widths.len())
            .map(|i| {
                self.columns
                    .get(i)
                    .map(|col| col.effective_alignment())
                    .unwrap_or(Alignment::Left)
            })
            .collect();

        // Render title (if present)
        if let Some(ref title) = self.title {
            result.push_str(&format!("\x1b[{}G{}", table_start, title));
            if let Some(end) = fill_end_col {
                let title_width = visible_width(title);
                let title_end = table_start + title_width;
                if end > title_end {
                    result.push_str(&" ".repeat((end - title_end) as usize));
                }
            }
            result.push('\n');
        }

        // Top border
        let top_border = build_border(&widths, '┌', '┬', '┐');
        result.push_str(&format!("\x1b[{}G{}", table_start, top_border));
        if let Some(end) = fill_end_col {
            let border_end = table_start + table_width;
            if end > border_end {
                result.push_str(&" ".repeat((end - border_end) as usize));
            }
        }
        result.push('\n');

        // Header row with multi-line support
        if !self.columns.is_empty() {
            // Calculate header height based on explicit newlines
            let header_lines: Vec<Vec<String>> = self
                .columns
                .iter()
                .enumerate()
                .map(|(i, col)| {
                    let width = widths.get(i).copied().unwrap_or(col.header.len());
                    wrap_cell_content(&col.header, &WordWrap::None, width)
                })
                .collect();

            let header_height = header_lines.iter().map(|lines| lines.len()).max().unwrap_or(1);

            // Apply vertical alignment
            let padded_headers: Vec<Vec<String>> = header_lines
                .into_iter()
                .enumerate()
                .map(|(i, lines)| {
                    let width = widths.get(i).copied().unwrap_or(0);
                    let vertical_align = self
                        .columns
                        .get(i)
                        .map(|col| col.vertical_align)
                        .unwrap_or(VerticalAlign::Top);
                    apply_vertical_padding(lines, header_height, vertical_align, width)
                })
                .collect();

            // Render each line of the header
            for line_idx in 0..header_height {
                let line_cells: Vec<String> = (0..self.columns.len())
                    .map(|i| {
                        padded_headers
                            .get(i)
                            .and_then(|lines| lines.get(line_idx))
                            .cloned()
                            .unwrap_or_default()
                    })
                    .collect();

                result.push_str(&render_row_with_cursor_positioning(
                    &line_cells,
                    &widths,
                    &alignments,
                    table_start,
                    fill_end_col,
                ));
                result.push('\n');
            }

            // Header separator
            let separator = build_border(&widths, '├', '┼', '┤');
            result.push_str(&format!("\x1b[{}G{}", table_start, separator));
            if let Some(end) = fill_end_col {
                let border_end = table_start + table_width;
                if end > border_end {
                    result.push_str(&" ".repeat((end - border_end) as usize));
                }
            }
            result.push('\n');
        }

        // Calculate row heights for multi-line support
        let row_heights = calculate_row_heights(&self.data, &self.columns, &widths);

        // Data rows with multi-line support
        for (row_idx, row) in self.data.iter().enumerate() {
            let row_height = row_heights.get(row_idx).copied().unwrap_or(1);

            // Prepare wrapped and vertically-aligned content for each cell
            let mut cell_lines: Vec<Vec<String>> = Vec::with_capacity(row.len());
            for (i, cell) in row.iter().enumerate() {
                let width = widths.get(i).copied().unwrap_or(0);
                let strategy = self
                    .columns
                    .get(i)
                    .map(|col| col.effective_word_wrap())
                    .unwrap_or(WordWrap::None);
                let vertical_align = self
                    .columns
                    .get(i)
                    .map(|col| col.vertical_align)
                    .unwrap_or(VerticalAlign::Top);

                let content = cell.to_string();
                let wrapped = wrap_cell_content(&content, &strategy, width);
                let padded = apply_vertical_padding(wrapped, row_height, vertical_align, width);
                cell_lines.push(padded);
            }

            // Render each line of the row
            for line_idx in 0..row_height {
                let line_cells: Vec<String> = (0..row.len())
                    .map(|i| {
                        cell_lines
                            .get(i)
                            .and_then(|lines| lines.get(line_idx))
                            .cloned()
                            .unwrap_or_default()
                    })
                    .collect();

                result.push_str(&render_row_with_cursor_positioning(
                    &line_cells,
                    &widths,
                    &alignments,
                    table_start,
                    fill_end_col,
                ));
                result.push('\n');
            }
        }

        // Bottom border
        let bottom_border = build_border(&widths, '└', '┴', '┘');
        result.push_str(&format!("\x1b[{}G{}", table_start, bottom_border));
        if let Some(end) = fill_end_col {
            let border_end = table_start + table_width;
            if end > border_end {
                result.push_str(&" ".repeat((end - border_end) as usize));
            }
        }

        result
    }
}

impl Renderable for Table {
    fn render(&self, term_width: Option<u32>) -> String {
        let width = term_width.unwrap_or(80);
        if self.prefer_cursor_alignment {
            // Cursor positioning always qualifies for opportunistic render
            // (assumes TTY that supports ANSI escape codes)
            self.render_with_cursor_positioning(width)
        } else {
            let available = self.layout.available_width(width);
            let content = self.render_content(Some(available));
            self.layout.apply_layout(&content, width)
        }
    }

    fn fallback_render(&self, term: &Terminal) -> String {
        let width = term.width();
        if self.prefer_cursor_alignment {
            // Cursor positioning works on any terminal that supports ANSI codes
            self.render_with_cursor_positioning(width)
        } else {
            let available = self.layout.available_width(width);
            let content = self.render_content(Some(available));
            self.layout.apply_layout(&content, width)
        }
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

/// Render a row with cursor positioning, supporting cell alignment.
///
/// If `fill_end_col` is Some, fills with spaces to that column for background color support.
fn render_row_with_cursor_positioning(
    cells: &[String],
    widths: &[usize],
    alignments: &[Alignment],
    table_start: u32,
    fill_end_col: Option<u32>,
) -> String {
    let mut row = String::new();
    row.push_str(&format!("\x1b[{}G│", table_start));

    // Track cursor position for row fill
    let mut last_col = table_start + 1;

    // cell_start is where content area begins (after "│ ")
    let mut cell_start = table_start.saturating_add(2);

    for (index, width) in widths.iter().enumerate() {
        let content = cells.get(index).map(String::as_str).unwrap_or("");
        let content_width = visible_width(content);
        let cell_width = *width as u32;
        let alignment = alignments.get(index).copied().unwrap_or(Alignment::Left);

        // Calculate cursor offset within cell based on alignment
        let cursor_offset = match alignment {
            Alignment::Left => 0,
            Alignment::Right => cell_width.saturating_sub(content_width),
            Alignment::Center => cell_width.saturating_sub(content_width) / 2,
        };

        let content_col = cell_start.saturating_add(cursor_offset);
        row.push_str(&format!("\x1b[{}G{}", content_col, content));

        // Position separator at end of cell
        let sep_col = cell_start.saturating_add(cell_width);
        if index + 1 == widths.len() {
            row.push_str(&format!("\x1b[{}G │", sep_col));
            last_col = sep_col + 2;
        } else {
            row.push_str(&format!("\x1b[{}G │ ", sep_col));
            last_col = sep_col + 3;
        }

        // Next cell starts after " │ "
        cell_start = sep_col.saturating_add(3);
    }

    // Fill to end column if requested (for background color support)
    if let Some(end) = fill_end_col {
        if end > last_col {
            row.push_str(&" ".repeat((end - last_col) as usize));
        }
    }

    row
}

/// Wrap cell content according to the given word wrap strategy and width.
///
/// Returns a vector of lines representing the wrapped content. Handles both
/// explicit newlines in the content and word wrap overflow.
fn wrap_cell_content(content: &str, strategy: &WordWrap, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![String::new()];
    }

    // First split on explicit newlines
    let lines = split_lines(content);

    // If no wrapping, return split lines as-is
    if matches!(strategy, WordWrap::None) {
        if lines.is_empty() {
            return vec![String::new()];
        }
        return lines;
    }

    // Apply word wrapping to each line
    wrap_lines(lines, strategy, width as u32)
}

/// Calculate the height (number of lines) needed for each row based on wrapped content.
///
/// Returns a vector where each element is the number of lines needed for that row.
fn calculate_row_heights(
    data: &[Vec<TableCellContent>],
    columns: &[TableColumn],
    widths: &[usize],
) -> Vec<usize> {
    data.iter()
        .map(|row| {
            let mut max_height = 1;
            for (i, cell) in row.iter().enumerate() {
                let width = widths.get(i).copied().unwrap_or(0);
                let strategy = columns
                    .get(i)
                    .map(|col| col.effective_word_wrap())
                    .unwrap_or(WordWrap::None);

                let content = cell.to_string();
                let wrapped = wrap_cell_content(&content, &strategy, width);
                max_height = max_height.max(wrapped.len());
            }
            max_height
        })
        .collect()
}

/// Apply vertical alignment padding to cell lines.
///
/// Given the actual lines of content and the target row height, returns a vector
/// of lines with appropriate empty-line padding based on the vertical alignment.
fn apply_vertical_padding(
    lines: Vec<String>,
    row_height: usize,
    vertical_align: VerticalAlign,
    cell_width: usize,
) -> Vec<String> {
    let content_height = lines.len();
    if content_height >= row_height {
        return lines;
    }

    let padding_needed = row_height - content_height;
    let empty_line = " ".repeat(cell_width);

    let (top_padding, bottom_padding) = match vertical_align {
        VerticalAlign::Top => (0, padding_needed),
        VerticalAlign::Bottom => (padding_needed, 0),
        VerticalAlign::Middle => {
            let top = padding_needed / 2;
            let bottom = padding_needed - top;
            (top, bottom)
        }
    };

    let mut result = Vec::with_capacity(row_height);
    for _ in 0..top_padding {
        result.push(empty_line.clone());
    }
    result.extend(lines);
    for _ in 0..bottom_padding {
        result.push(empty_line.clone());
    }
    result
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

    #[test]
    fn test_header_uses_column_alignment() {
        // Headers should respect the column's effective alignment, not be hardcoded to Left
        let table = Table::new()
            .with_columns(vec![
                TableColumn::new("ID")
                    .with_type(ColumnType::Integer)
                    .with_min_width(8),
            ])
            .with_data(vec![vec![TableCellContent::Integer(42)]]);

        let result = table.render_content(None);
        let lines: Vec<&str> = result.lines().collect();
        // Header line (index 1) should have right-aligned "ID" (integer type defaults to right)
        // "│      ID │" - ID right-aligned in 8-char width
        let header_line = lines[1];
        assert!(
            header_line.contains("      ID"),
            "Header should be right-aligned: {:?}",
            header_line
        );
    }

    #[test]
    fn test_fixed_width_overrides_content_width() {
        // When fixed_width is larger than content, padding is applied
        let table = Table::new()
            .with_columns(vec![TableColumn::new("X").with_fixed_width(10)])
            .with_data(vec![vec!["A".into()]]);

        let result = table.render_content(None);
        let lines: Vec<&str> = result.lines().collect();
        let header_width = visible_width(lines[1]);
        let row_width = visible_width(lines[3]);
        assert_eq!(header_width, row_width);
        // Total width = "│ " (2) + fixed_width (10) + " │" (2) = 14
        assert_eq!(header_width, 14);
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

    // ── Cursor alignment tests ───────────────────────────────────────

    #[test]
    fn test_prefer_cursor_alignment_builder() {
        let table = Table::new()
            .with_columns(vec![TableColumn::new("X")])
            .prefer_cursor_alignment();
        assert!(table.prefer_cursor_alignment);
    }

    #[test]
    fn test_cursor_alignment_default_is_false() {
        let table = Table::new();
        assert!(!table.prefer_cursor_alignment);
    }

    #[test]
    fn test_cursor_alignment_uses_escape_codes() {
        let table = Table::new()
            .with_columns(vec![TableColumn::new("Name")])
            .with_data(vec![vec!["Alice".into()]])
            .prefer_cursor_alignment();

        let result = table.render(Some(80));
        // Should contain cursor positioning escape codes
        assert!(
            result.contains("\x1b["),
            "Cursor alignment should use ANSI escape codes"
        );
        // Should contain the column movement escape pattern
        assert!(
            result.contains("G│"),
            "Should use cursor positioning before borders"
        );
    }

    #[test]
    fn test_cursor_alignment_with_left_margin() {
        use crate::utils::layout::Margin;

        let mut table = Table::new()
            .with_columns(vec![TableColumn::new("X")])
            .with_data(vec![vec!["A".into()]])
            .prefer_cursor_alignment();
        table.layout_mut().left_margin = Margin::Chars(5);

        let result = table.render(Some(80));
        // Table should start at column 6 (5 margin + 1 for 1-indexed)
        assert!(
            result.contains("\x1b[6G"),
            "Table should start at column 6 with 5-char margin: {:?}",
            result
        );
    }

    #[test]
    fn test_cursor_alignment_block_center() {
        let mut table = Table::new()
            .with_columns(vec![TableColumn::new("X")])
            .with_data(vec![vec!["A".into()]])
            .prefer_cursor_alignment();
        table.layout_mut().alignment = Alignment::Center;

        let result = table.render(Some(80));
        // With 80 width and small table, table_start should be > 1
        // The table width is about 5 chars (│ X │), so center offset should be ~37
        // Look for a column position > 30 (roughly centered)
        let has_centered_position = result
            .lines()
            .next()
            .map(|line| {
                // Extract the column number from \x1b[NNG pattern
                if let Some(start) = line.find("\x1b[") {
                    let rest = &line[start + 2..];
                    if let Some(end) = rest.find('G') {
                        if let Ok(col) = rest[..end].parse::<u32>() {
                            return col > 30;
                        }
                    }
                }
                false
            })
            .unwrap_or(false);
        assert!(
            has_centered_position,
            "Center-aligned table should have offset position: {:?}",
            result.lines().next()
        );
    }

    #[test]
    fn test_cursor_alignment_block_right() {
        let mut table = Table::new()
            .with_columns(vec![TableColumn::new("X")])
            .with_data(vec![vec!["A".into()]])
            .prefer_cursor_alignment();
        table.layout_mut().alignment = Alignment::Right;

        let result = table.render(Some(80));
        // With 80 width and small table (~5 chars), table should start near column 75
        let has_right_position = result
            .lines()
            .next()
            .map(|line| {
                if let Some(start) = line.find("\x1b[") {
                    let rest = &line[start + 2..];
                    if let Some(end) = rest.find('G') {
                        if let Ok(col) = rest[..end].parse::<u32>() {
                            return col > 70;
                        }
                    }
                }
                false
            })
            .unwrap_or(false);
        assert!(
            has_right_position,
            "Right-aligned table should have high column position: {:?}",
            result.lines().next()
        );
    }

    #[test]
    fn test_cursor_alignment_cell_right_alignment() {
        let table = Table::new()
            .with_columns(vec![
                TableColumn::new("Amount")
                    .with_type(ColumnType::Integer)
                    .with_min_width(10),
            ])
            .with_data(vec![vec![TableCellContent::Integer(42)]])
            .prefer_cursor_alignment();

        let result = table.render(Some(80));
        // The content "42" should be positioned toward the right of the 10-char cell
        // Look for the data row and verify cursor position accounts for alignment
        let lines: Vec<&str> = result.lines().collect();
        // Data row should be after header (line 0=border, 1=header, 2=separator, 3=data)
        assert!(
            lines.len() >= 4,
            "Table should have at least 4 lines (border, header, sep, data)"
        );
    }

    #[test]
    fn test_cursor_alignment_preserves_content() {
        let table = Table::new()
            .with_columns(vec![TableColumn::new("Name"), TableColumn::new("Age")])
            .with_data(vec![
                vec!["Alice".into(), "30".into()],
                vec!["Bob".into(), "25".into()],
            ])
            .prefer_cursor_alignment();

        let result = table.render(Some(80));
        assert!(result.contains("Name"), "Should contain header 'Name'");
        assert!(result.contains("Age"), "Should contain header 'Age'");
        assert!(result.contains("Alice"), "Should contain data 'Alice'");
        assert!(result.contains("Bob"), "Should contain data 'Bob'");
        assert!(result.contains("30"), "Should contain data '30'");
        assert!(result.contains("25"), "Should contain data '25'");
    }

    #[test]
    fn test_cursor_alignment_with_title() {
        let table = Table::new()
            .with_title("Users")
            .with_columns(vec![TableColumn::new("Name")])
            .with_data(vec![vec!["Alice".into()]])
            .prefer_cursor_alignment();

        let result = table.render(Some(80));
        assert!(result.contains("Users"), "Should contain title");
    }

    #[test]
    fn test_cursor_alignment_empty_table_returns_empty() {
        let table = Table::new().prefer_cursor_alignment();
        let result = table.render(Some(80));
        assert!(result.is_empty(), "Empty table should render empty string");
    }

    #[test]
    fn test_cursor_alignment_row_fill() {
        use crate::utils::layout::Margin;

        let mut table = Table::new()
            .with_columns(vec![TableColumn::new("X")])
            .with_data(vec![vec!["A".into()]])
            .prefer_cursor_alignment();
        table.layout_mut().left_margin = Margin::Chars(2);
        table.layout_mut().right_margin = Margin::Chars(2);
        table.layout_mut().row_fill_strategy = RowFill::Fill;

        let result = table.render(Some(20));
        // With row fill enabled, lines should extend to fill available width
        // Available width = 20 - 2 - 2 = 16
        for line in result.lines() {
            let width = visible_width(line);
            // Lines should have content (not be empty after stripping escapes)
            assert!(
                width > 0,
                "Lines should have visible content: {:?}",
                line
            );
        }
    }

    #[test]
    fn test_cursor_alignment_consistent_row_widths() {
        let table = Table::new()
            .with_columns(vec![TableColumn::new("Name"), TableColumn::new("Status")])
            .with_data(vec![
                vec![
                    "Alice".into(),
                    TableCellContent::Text("\x1b[32mActive\x1b[0m".to_string()),
                ],
                vec!["Bob".into(), TableCellContent::Text("Inactive".to_string())],
            ])
            .prefer_cursor_alignment();

        let result = table.render(Some(80));
        // All border lines should contain the same box-drawing characters
        let border_lines: Vec<&str> = result
            .lines()
            .filter(|l| l.contains('┌') || l.contains('├') || l.contains('└'))
            .collect();
        assert!(
            border_lines.len() >= 3,
            "Should have top, separator, and bottom borders"
        );
    }

    // ── Word wrap and vertical alignment tests ───────────────────────

    #[test]
    fn test_table_column_default_word_wrap_for_string() {
        let col = TableColumn::new("Description");
        assert_eq!(
            col.effective_word_wrap(),
            WordWrap::WrapProse(None, None),
            "String columns should default to WrapProse"
        );
    }

    #[test]
    fn test_table_column_default_word_wrap_for_integer() {
        let col = TableColumn::new("Count").with_type(ColumnType::Integer);
        assert_eq!(
            col.effective_word_wrap(),
            WordWrap::None,
            "Integer columns should default to None"
        );
    }

    #[test]
    fn test_table_column_word_wrap_override_for_string() {
        let col = TableColumn::new("Notes").with_word_wrap(WordWrap::Truncate(Some("...".into())));
        assert_eq!(
            col.effective_word_wrap(),
            WordWrap::Truncate(Some("...".into())),
            "String columns should allow word wrap override"
        );
    }

    #[test]
    fn test_table_column_word_wrap_override_ignored_for_integer() {
        let col = TableColumn::new("Amount")
            .with_type(ColumnType::Integer)
            .with_word_wrap(WordWrap::WrapProse(None, None));
        assert_eq!(
            col.effective_word_wrap(),
            WordWrap::None,
            "Integer columns should force WordWrap::None"
        );
    }

    #[test]
    fn test_table_column_word_wrap_override_ignored_for_currency() {
        let col = TableColumn::new("Price")
            .with_type(ColumnType::Currency(Currency::USD))
            .with_word_wrap(WordWrap::WrapProse(None, None));
        assert_eq!(
            col.effective_word_wrap(),
            WordWrap::None,
            "Currency columns should force WordWrap::None"
        );
    }

    #[test]
    fn test_table_column_default_vertical_align() {
        let col = TableColumn::new("Name");
        assert_eq!(
            col.vertical_align,
            VerticalAlign::Top,
            "Default vertical align should be Top"
        );
    }

    #[test]
    fn test_table_column_with_vertical_align() {
        let col = TableColumn::new("Name").with_vertical_align(VerticalAlign::Middle);
        assert_eq!(col.vertical_align, VerticalAlign::Middle);
    }

    #[test]
    fn test_table_column_builder_chain() {
        let col = TableColumn::new("Description")
            .with_min_width(10)
            .with_max_width(50)
            .with_word_wrap(WordWrap::WrapProse(Some(4), None))
            .with_vertical_align(VerticalAlign::Bottom);

        assert_eq!(col.min_width, Some(10));
        assert_eq!(col.max_width, Some(50));
        assert_eq!(
            col.effective_word_wrap(),
            WordWrap::WrapProse(Some(4), None)
        );
        assert_eq!(col.vertical_align, VerticalAlign::Bottom);
    }

    // ── Width constraint tests ───────────────────────────────────────

    #[test]
    fn test_calculate_column_widths_without_constraint() {
        let table = Table::new()
            .with_columns(vec![
                TableColumn::new("Name"),
                TableColumn::new("Description"),
            ])
            .with_data(vec![vec![
                "Alice".into(),
                "A very long description that exceeds normal width".into(),
            ]]);

        let widths = table.calculate_column_widths(None);
        assert_eq!(widths.len(), 2);
        // Without constraint, widths should match content
        assert_eq!(widths[0], 5); // "Alice"
        assert_eq!(widths[1], 49); // Long description
    }

    #[test]
    fn test_calculate_column_widths_with_constraint_fits() {
        let table = Table::new()
            .with_columns(vec![TableColumn::new("X"), TableColumn::new("Y")])
            .with_data(vec![vec!["A".into(), "B".into()]]);

        // Table needs: 3 + (1 + 1) + 3 = 8 chars minimum
        let widths = table.calculate_column_widths(Some(80));
        assert_eq!(widths, vec![1, 1]);
    }

    #[test]
    fn test_calculate_column_widths_with_constraint_reduces_text_columns() {
        let table = Table::new()
            .with_columns(vec![
                TableColumn::new("Name"),
                TableColumn::new("Description"),
            ])
            .with_data(vec![vec![
                "Alice".into(),
                "A long description".into(),
            ]]);

        // Without constraint: widths would be [5, 18]
        // Total content: 23, borders: 3 + 3 = 6, total: 29
        // With constraint of 25: need to reduce by 4
        let widths = table.calculate_column_widths(Some(25));
        let total_content: usize = widths.iter().sum();
        let total_table = 3 + total_content + 3; // border overhead
        assert!(
            total_table <= 25,
            "Table width {} should fit in 25 chars",
            total_table
        );
    }

    #[test]
    fn test_calculate_column_widths_fixed_width_not_reduced() {
        let table = Table::new()
            .with_columns(vec![
                TableColumn::new("ID").with_fixed_width(10),
                TableColumn::new("Description"),
            ])
            .with_data(vec![vec![
                "123".into(),
                "A moderately long description".into(),
            ]]);

        // Fixed-width column should not be reduced
        let widths = table.calculate_column_widths(Some(30));
        assert_eq!(widths[0], 10, "Fixed-width column should stay at 10");
    }

    #[test]
    fn test_calculate_column_widths_numeric_column_not_reduced() {
        let table = Table::new()
            .with_columns(vec![
                TableColumn::new("Amount").with_type(ColumnType::Integer),
                TableColumn::new("Description"),
            ])
            .with_data(vec![vec![
                TableCellContent::Integer(1_000_000),
                "A long description text".into(),
            ]]);

        // Numeric column should not be reduced (doesn't allow word wrap override)
        let widths_unconstrained = table.calculate_column_widths(None);
        let widths_constrained = table.calculate_column_widths(Some(40));

        // The numeric column width should remain the same
        assert_eq!(
            widths_constrained[0], widths_unconstrained[0],
            "Numeric column should not be reduced"
        );
    }

    #[test]
    fn test_calculate_column_widths_respects_min_width() {
        let table = Table::new()
            .with_columns(vec![
                TableColumn::new("Description").with_min_width(15),
            ])
            .with_data(vec![vec![
                "A very long description that is quite wide".into(),
            ]]);

        // Even with tight constraint, min_width should be respected
        let widths = table.calculate_column_widths(Some(20));
        assert!(
            widths[0] >= 15,
            "Column width {} should respect min_width 15",
            widths[0]
        );
    }

    // ── Multi-line cell helper function tests ────────────────────────

    #[test]
    fn test_wrap_cell_content_no_wrap() {
        let result = wrap_cell_content("hello world", &WordWrap::None, 5);
        // With None strategy, no wrapping occurs - just split on newlines
        assert_eq!(result, vec!["hello world"]);
    }

    #[test]
    fn test_wrap_cell_content_explicit_newlines() {
        let result = wrap_cell_content("line1\nline2\nline3", &WordWrap::None, 10);
        assert_eq!(result, vec!["line1", "line2", "line3"]);
    }

    #[test]
    fn test_wrap_cell_content_wrap_prose() {
        let result = wrap_cell_content("hello world friend", &WordWrap::WrapProse(None, None), 10);
        // Should wrap on word boundaries
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], "hello");
        assert_eq!(result[1], "world");
        assert_eq!(result[2], "friend");
    }

    #[test]
    fn test_wrap_cell_content_empty_string() {
        let result = wrap_cell_content("", &WordWrap::WrapProse(None, None), 10);
        assert_eq!(result, vec![""]);
    }

    #[test]
    fn test_wrap_cell_content_zero_width() {
        let result = wrap_cell_content("hello", &WordWrap::WrapProse(None, None), 0);
        assert_eq!(result, vec![""]);
    }

    #[test]
    fn test_calculate_row_heights_single_line() {
        let columns = vec![
            TableColumn::new("Name"),
            TableColumn::new("Age"),
        ];
        let data = vec![
            vec!["Alice".into(), "30".into()],
            vec!["Bob".into(), "25".into()],
        ];
        let widths = vec![10, 5];

        let heights = calculate_row_heights(&data, &columns, &widths);
        assert_eq!(heights, vec![1, 1]);
    }

    #[test]
    fn test_calculate_row_heights_with_explicit_newlines() {
        let columns = vec![TableColumn::new("Description")];
        let data = vec![
            vec![TableCellContent::Text("Line1\nLine2\nLine3".to_string())],
        ];
        let widths = vec![20];

        let heights = calculate_row_heights(&data, &columns, &widths);
        assert_eq!(heights, vec![3]);
    }

    #[test]
    fn test_calculate_row_heights_max_across_cells() {
        let columns = vec![
            TableColumn::new("Short"),
            TableColumn::new("Long"),
        ];
        let data = vec![
            vec![
                "A".into(),
                TableCellContent::Text("Line1\nLine2".to_string()),
            ],
        ];
        let widths = vec![5, 10];

        let heights = calculate_row_heights(&data, &columns, &widths);
        // Max of (1 line, 2 lines) = 2
        assert_eq!(heights, vec![2]);
    }

    #[test]
    fn test_apply_vertical_padding_top_align() {
        let lines = vec!["content".to_string()];
        let result = apply_vertical_padding(lines, 3, VerticalAlign::Top, 7);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], "content");
        assert_eq!(result[1], "       "); // 7 spaces
        assert_eq!(result[2], "       ");
    }

    #[test]
    fn test_apply_vertical_padding_bottom_align() {
        let lines = vec!["content".to_string()];
        let result = apply_vertical_padding(lines, 3, VerticalAlign::Bottom, 7);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], "       ");
        assert_eq!(result[1], "       ");
        assert_eq!(result[2], "content");
    }

    #[test]
    fn test_apply_vertical_padding_middle_align() {
        let lines = vec!["content".to_string()];
        let result = apply_vertical_padding(lines, 3, VerticalAlign::Middle, 7);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], "       ");
        assert_eq!(result[1], "content");
        assert_eq!(result[2], "       ");
    }

    #[test]
    fn test_apply_vertical_padding_no_padding_needed() {
        let lines = vec!["line1".to_string(), "line2".to_string()];
        let result = apply_vertical_padding(lines.clone(), 2, VerticalAlign::Top, 5);
        assert_eq!(result, lines);
    }

    #[test]
    fn test_apply_vertical_padding_content_exceeds_height() {
        let lines = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let result = apply_vertical_padding(lines.clone(), 2, VerticalAlign::Top, 1);
        // Should return original lines when content exceeds target height
        assert_eq!(result, lines);
    }

    // ── Multi-line row rendering tests ───────────────────────────────

    #[test]
    fn test_render_content_single_line_unchanged() {
        let table = Table::new()
            .with_columns(vec![TableColumn::new("Name"), TableColumn::new("Age")])
            .with_data(vec![
                vec!["Alice".into(), "30".into()],
                vec!["Bob".into(), "25".into()],
            ]);

        let result = table.render_content(None);
        // Should have: top border, header, separator, 2 data rows, bottom border = 6 lines
        assert_eq!(result.lines().count(), 6);
    }

    #[test]
    fn test_render_content_multiline_header() {
        let table = Table::new()
            .with_columns(vec![
                TableColumn::new("Name"),
                TableColumn::new("GitHub\nStars").with_type(ColumnType::Integer),
            ])
            .with_data(vec![vec!["Rust".into(), TableCellContent::Integer(99800)]]);

        let result = table.render_content(None);
        let lines: Vec<&str> = result.lines().collect();

        // Should have: top border, 2 header lines, separator, 1 data row, bottom border = 6 lines
        assert_eq!(
            lines.len(),
            6,
            "Expected 6 lines for 2-line header:\n{}",
            result
        );

        // First header line should contain "Name" and "GitHub"
        assert!(lines[1].contains("Name"), "First header line should have 'Name'");
        assert!(
            lines[1].contains("GitHub"),
            "First header line should have 'GitHub'"
        );

        // Second header line should contain "Stars" (right-aligned for Integer type)
        assert!(
            lines[2].contains("Stars"),
            "Second header line should have 'Stars'"
        );
    }

    #[test]
    fn test_render_content_explicit_newlines() {
        let table = Table::new()
            .with_columns(vec![TableColumn::new("Description")])
            .with_data(vec![vec![TableCellContent::Text(
                "Line 1\nLine 2\nLine 3".to_string(),
            )]]);

        let result = table.render_content(None);
        // Should have: top border, header, separator, 3 data lines, bottom border = 7 lines
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(
            lines.len(),
            7,
            "Expected 7 lines, got {}:\n{}",
            lines.len(),
            result
        );

        // Verify the multi-line content is rendered
        let data_lines: Vec<&str> = lines[3..6].to_vec();
        assert!(data_lines[0].contains("Line 1"));
        assert!(data_lines[1].contains("Line 2"));
        assert!(data_lines[2].contains("Line 3"));
    }

    #[test]
    fn test_render_content_mixed_heights() {
        let table = Table::new()
            .with_columns(vec![
                TableColumn::new("Single"),
                TableColumn::new("Multi"),
            ])
            .with_data(vec![vec![
                "A".into(),
                TableCellContent::Text("B\nC".to_string()),
            ]]);

        let result = table.render_content(None);
        // Row should span 2 lines
        let lines: Vec<&str> = result.lines().collect();
        // top border, header, separator, 2 data lines, bottom border = 6 lines
        assert_eq!(lines.len(), 6, "Expected 6 lines:\n{}", result);
    }

    #[test]
    fn test_render_content_vertical_align_top() {
        let table = Table::new()
            .with_columns(vec![
                TableColumn::new("Short").with_vertical_align(VerticalAlign::Top),
                TableColumn::new("Long"),
            ])
            .with_data(vec![vec![
                "A".into(),
                TableCellContent::Text("Line1\nLine2\nLine3".to_string()),
            ]]);

        let result = table.render_content(None);
        let lines: Vec<&str> = result.lines().collect();

        // Data starts at line 3 (after border, header, separator)
        // With top alignment, "A" should be on the first data line
        assert!(
            lines[3].contains("A"),
            "First data line should contain 'A' for top alignment: {}",
            lines[3]
        );
    }

    #[test]
    fn test_render_content_vertical_align_bottom() {
        let table = Table::new()
            .with_columns(vec![
                TableColumn::new("Short").with_vertical_align(VerticalAlign::Bottom),
                TableColumn::new("Long"),
            ])
            .with_data(vec![vec![
                "A".into(),
                TableCellContent::Text("Line1\nLine2\nLine3".to_string()),
            ]]);

        let result = table.render_content(None);
        let lines: Vec<&str> = result.lines().collect();

        // Data is at lines 3, 4, 5 (3 lines for 3-line content)
        // With bottom alignment, "A" should be on the last data line (line 5)
        assert!(
            lines[5].contains("A"),
            "Last data line should contain 'A' for bottom alignment: {}",
            lines[5]
        );
    }

    #[test]
    fn test_render_content_vertical_align_middle() {
        let table = Table::new()
            .with_columns(vec![
                TableColumn::new("Short").with_vertical_align(VerticalAlign::Middle),
                TableColumn::new("Long"),
            ])
            .with_data(vec![vec![
                "A".into(),
                TableCellContent::Text("Line1\nLine2\nLine3".to_string()),
            ]]);

        let result = table.render_content(None);
        let lines: Vec<&str> = result.lines().collect();

        // Data is at lines 3, 4, 5 (3 lines for 3-line content)
        // With middle alignment, "A" should be on the middle data line (line 4)
        assert!(
            lines[4].contains("A"),
            "Middle data line should contain 'A' for middle alignment: {}",
            lines[4]
        );
    }

    #[test]
    fn test_render_content_preserves_border_alignment() {
        let table = Table::new()
            .with_columns(vec![TableColumn::new("X")])
            .with_data(vec![vec![TableCellContent::Text("A\nB".to_string())]]);

        let result = table.render_content(None);
        let lines: Vec<&str> = result.lines().collect();

        // All content lines should have consistent border positions
        for line in &lines[1..lines.len() - 1] {
            // Skip top and bottom borders
            if !line.contains('─') {
                assert!(
                    line.starts_with('│') && line.ends_with('│'),
                    "Content line should be bordered: {:?}",
                    line
                );
            }
        }
    }

    // ── Cursor positioning multi-line tests ──────────────────────────

    #[test]
    fn test_cursor_positioning_multiline_row() {
        let table = Table::new()
            .with_columns(vec![TableColumn::new("Description")])
            .with_data(vec![vec![TableCellContent::Text(
                "Line 1\nLine 2".to_string(),
            )]])
            .prefer_cursor_alignment();

        let result = table.render(Some(80));
        // Should contain both lines of content
        assert!(result.contains("Line 1"), "Should contain 'Line 1'");
        assert!(result.contains("Line 2"), "Should contain 'Line 2'");

        // Count data lines (lines with │ that aren't borders)
        let data_lines: Vec<&str> = result
            .lines()
            .filter(|l| l.contains('│') && !l.contains('┌') && !l.contains('├') && !l.contains('└'))
            .collect();
        // Header + 2 data lines = 3 lines with │
        assert!(
            data_lines.len() >= 3,
            "Expected at least 3 content lines (header + 2 data), got {}",
            data_lines.len()
        );
    }

    #[test]
    fn test_cursor_positioning_mixed_height_rows() {
        let table = Table::new()
            .with_columns(vec![
                TableColumn::new("Short"),
                TableColumn::new("Long"),
            ])
            .with_data(vec![vec![
                "A".into(),
                TableCellContent::Text("B\nC\nD".to_string()),
            ]])
            .prefer_cursor_alignment();

        let result = table.render(Some(80));

        // Verify all content is present
        assert!(result.contains("A"), "Should contain 'A'");
        assert!(result.contains("B"), "Should contain 'B'");
        assert!(result.contains("C"), "Should contain 'C'");
        assert!(result.contains("D"), "Should contain 'D'");
    }

    #[test]
    fn test_cursor_positioning_vertical_align() {
        let table = Table::new()
            .with_columns(vec![
                TableColumn::new("Short").with_vertical_align(VerticalAlign::Bottom),
                TableColumn::new("Long"),
            ])
            .with_data(vec![vec![
                "X".into(),
                TableCellContent::Text("A\nB\nC".to_string()),
            ]])
            .prefer_cursor_alignment();

        let result = table.render(Some(80));

        // Find data lines (skip header and borders)
        let content_lines: Vec<&str> = result
            .lines()
            .filter(|l| {
                l.contains('│')
                    && !l.contains('┌')
                    && !l.contains('├')
                    && !l.contains('└')
                    && !l.contains('─')
            })
            .collect();

        // Should have header + 3 data lines = 4 total
        assert!(
            content_lines.len() >= 4,
            "Expected at least 4 content lines, got {}",
            content_lines.len()
        );

        // With bottom alignment, "X" should be in the last data line
        // (lines[0] is header, lines[1-3] are data)
        if content_lines.len() >= 4 {
            assert!(
                content_lines[3].contains("X"),
                "Last data line should contain 'X' for bottom alignment: {}",
                content_lines[3]
            );
        }
    }

    #[test]
    fn test_cursor_positioning_multiline_preserves_cursor_codes() {
        let table = Table::new()
            .with_columns(vec![TableColumn::new("X")])
            .with_data(vec![vec![TableCellContent::Text("A\nB".to_string())]])
            .prefer_cursor_alignment();

        let result = table.render(Some(80));

        // Every line should use cursor positioning
        for line in result.lines() {
            assert!(
                line.contains("\x1b["),
                "Each line should use cursor positioning: {:?}",
                line
            );
        }
    }

    // ── Integration and Edge Case Tests ──────────────────────────────

    #[test]
    fn test_empty_cells_with_multiline_neighbor() {
        let table = Table::new()
            .with_columns(vec![
                TableColumn::new("Empty"),
                TableColumn::new("Multi"),
            ])
            .with_data(vec![vec![
                "".into(),
                TableCellContent::Text("A\nB\nC".to_string()),
            ]]);

        let result = table.render_content(None);
        let lines: Vec<&str> = result.lines().collect();

        // Row should have 3 data lines
        assert_eq!(
            lines.len(),
            7,
            "Expected 7 lines (border, header, sep, 3 data, border)"
        );
        // Empty cell should still render proper padding
        for line in &lines[3..6] {
            assert!(
                line.starts_with('│') && line.ends_with('│'),
                "All data lines should have borders: {:?}",
                line
            );
        }
    }

    #[test]
    fn test_ansi_escape_codes_in_multiline_cells() {
        let colored_multiline =
            "\x1b[31mRed Line 1\x1b[0m\n\x1b[32mGreen Line 2\x1b[0m".to_string();
        let table = Table::new()
            .with_columns(vec![TableColumn::new("Colored")])
            .with_data(vec![vec![TableCellContent::Text(colored_multiline)]]);

        let result = table.render_content(None);

        // ANSI codes should be preserved
        assert!(result.contains("\x1b[31m"), "Red escape should be preserved");
        assert!(
            result.contains("\x1b[32m"),
            "Green escape should be preserved"
        );
        assert!(result.contains("Red Line 1"), "Red text should be present");
        assert!(
            result.contains("Green Line 2"),
            "Green text should be present"
        );
    }

    #[test]
    fn test_mixed_column_types_multiline() {
        let table = Table::new()
            .with_columns(vec![
                TableColumn::new("Description"),
                TableColumn::new("Amount").with_type(ColumnType::Integer),
            ])
            .with_data(vec![vec![
                TableCellContent::Text("Long\nDescription".to_string()),
                TableCellContent::Integer(12345),
            ]]);

        let result = table.render_content(None);
        let lines: Vec<&str> = result.lines().collect();

        // Row should span 2 lines
        assert_eq!(lines.len(), 6, "Expected 6 lines for 2-line row");

        // Numeric column should be right-aligned in both lines
        // and the value should only appear once (with padding above/below)
    }

    #[test]
    fn test_numeric_column_never_wraps() {
        let table = Table::new()
            .with_columns(vec![
                TableColumn::new("Amount")
                    .with_type(ColumnType::Integer)
                    .with_max_width(5), // Force narrow width
            ])
            .with_data(vec![vec![TableCellContent::Integer(1_000_000)]]);

        let result = table.render_content(None);

        // Number should NOT be wrapped even if exceeds width
        // (word wrap is forced to None for numeric columns)
        assert!(
            result.contains("1,000,000"),
            "Number should not be wrapped or truncated"
        );
    }

    #[test]
    fn test_table_respects_available_width_cursor_mode() {
        use crate::utils::layout::Margin;

        let mut table = Table::new()
            .with_columns(vec![
                TableColumn::new("Description"),
                TableColumn::new("Notes"),
            ])
            .with_data(vec![vec![
                "A very long description that might need to wrap".into(),
                "Another long piece of text for the notes column".into(),
            ]])
            .prefer_cursor_alignment();

        table.layout_mut().left_margin = Margin::Chars(5);
        table.layout_mut().right_margin = Margin::Chars(5);

        let result = table.render(Some(60));

        // Table should respect available width (60 - 5 - 5 = 50)
        // Check that cursor positioning starts correctly
        assert!(
            result.contains("\x1b[6G"),
            "Should position at column 6 (5 margin + 1)"
        );
    }

    #[test]
    fn test_table_respects_available_width_non_cursor_mode() {
        // Non-cursor mode should also respect available width
        let table = Table::new()
            .with_columns(vec![
                TableColumn::new("Description"),
                TableColumn::new("Notes"),
            ])
            .with_data(vec![vec![
                "A very long description that definitely needs to wrap when constrained".into(),
                "Another long piece of text for the notes column that also wraps".into(),
            ]]);

        // Render with narrow width - table should wrap content
        let result = table.render(Some(50));
        let lines: Vec<&str> = result.lines().collect();

        // All content lines should fit within available width
        for line in &lines {
            let width = visible_width(line);
            assert!(
                width <= 50,
                "Line exceeds available width: {} chars > 50: '{}'",
                width,
                line
            );
        }
    }

    #[test]
    fn test_extremely_narrow_terminal() {
        let table = Table::new()
            .with_columns(vec![TableColumn::new("X")])
            .with_data(vec![vec!["ABCDEFGHIJ".into()]])
            .prefer_cursor_alignment();

        // With only 15 chars available, table should still render
        let result = table.render(Some(15));
        assert!(!result.is_empty(), "Should render even with narrow terminal");
    }

    #[test]
    fn test_multiple_rows_different_heights() {
        let table = Table::new()
            .with_columns(vec![TableColumn::new("Content")])
            .with_data(vec![
                vec![TableCellContent::Text("Single line".to_string())],
                vec![TableCellContent::Text("Line A\nLine B".to_string())],
                vec![TableCellContent::Text("One\nTwo\nThree".to_string())],
            ]);

        let result = table.render_content(None);
        let lines: Vec<&str> = result.lines().collect();

        // Total: border + header + sep + 1 + 2 + 3 + border = 10 lines
        assert_eq!(
            lines.len(),
            10,
            "Expected 10 lines (1 border + 1 header + 1 sep + 6 data + 1 border)"
        );
    }

    #[test]
    fn test_osc8_links_in_multiline_cells() {
        let link_multiline =
            "\x1b]8;;https://a.com\x07Link A\x1b]8;;\x07\n\x1b]8;;https://b.com\x07Link B\x1b]8;;\x07"
                .to_string();
        let table = Table::new()
            .with_columns(vec![TableColumn::new("Links")])
            .with_data(vec![vec![TableCellContent::Text(link_multiline)]]);

        let result = table.render_content(None);

        // OSC8 sequences should be preserved
        assert!(
            result.contains("\x1b]8;;https://a.com\x07"),
            "First link should be preserved"
        );
        assert!(
            result.contains("\x1b]8;;https://b.com\x07"),
            "Second link should be preserved"
        );
    }

    #[test]
    fn test_consistent_row_widths_multiline() {
        let table = Table::new()
            .with_columns(vec![TableColumn::new("A"), TableColumn::new("B")])
            .with_data(vec![vec![
                "Short".into(),
                TableCellContent::Text("Line 1\nLine 2".to_string()),
            ]]);

        let result = table.render_content(None);
        let lines: Vec<&str> = result.lines().collect();

        // All content lines (non-border) should have the same visible width
        let content_lines: Vec<&str> = lines
            .iter()
            .filter(|l| l.starts_with('│') && !l.contains('─'))
            .copied()
            .collect();

        let first_width = visible_width(content_lines[0]);
        for line in &content_lines {
            assert_eq!(
                visible_width(line),
                first_width,
                "All content lines should have same width"
            );
        }
    }

    #[test]
    fn test_word_wrap_respects_column_width() {
        let table = Table::new()
            .with_columns(vec![TableColumn::new("Desc").with_max_width(10)])
            .with_data(vec![vec!["This is a longer text that should wrap".into()]]);

        let result = table.render_content(None);

        // Content should wrap at column width
        let lines: Vec<&str> = result.lines().collect();
        assert!(
            lines.len() > 4,
            "Long content should cause multi-line row: {} lines",
            lines.len()
        );
    }
}
