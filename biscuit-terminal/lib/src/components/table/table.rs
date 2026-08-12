use renderable::browser::PageOptions;
use renderable::browser::fragment::{BrowserFragment, Ready};
use renderable::color::{BasicColor, Color, RgbColor};
use renderable::html::HtmlPage;
use renderable::markdown::MarkdownRenderable;
use renderable::tree::render::{
    BrowserRenderOptions, MarkdownDialect, MarkdownRenderOptions, render_browser_node,
    render_markdown_node,
};
use renderable::tree::{
    ColumnAlign, ColumnConditional, RenderNode, RenderStrictness, TableCellHints, TableColumnHints,
    TableTerminalHints, TreeRenderable,
};

use renderable::style::Style;

use crate::{
    components::renderable::{BrowserRenderable, TerminalRenderable},
    render_tree::render::resolve_cells,
    render_tree::style::{SGR_RESET, color_sgr, text_appearance_sgr},
    render_tree::{TerminalRenderOptions, render_terminal_node},
    terminal::Terminal,
    utils::{
        block_constraint::{sanitize_wrapped_lines, split_lines, visible_width, wrap_lines},
        layout::{Alignment, Layout, LayoutTerminalExt},
        wrap_policy::WordWrap,
    },
};
use renderable::layout::Width;

pub use super::cell::TableCellContent;
use super::cell::{expand_tabs_with_width, pad_cell};
pub use super::column::TableColumn;
use super::types::{Currency, TableStyle, VerticalAlign};
use super::width::{MeasuredColumn, TableWidthError, TableWidthMeasurements, TableWidthPlan};
use crate::discovery::detection::{ColorDepth, ColorMode};

/// A table component for rendering tabular data.
///
/// Tables render with Unicode box-drawing characters and support:
/// - Multi-line cell content with word wrapping
/// - Conditional column visibility based on terminal width
/// - Column-specific alignment and formatting
/// - Alternating row colors (background and text tint)
///
/// ## Basic Usage
///
/// ```
/// use biscuit_terminal::components::table::{Table, TableColumn, TableCellContent};
/// use biscuit_terminal::components::table::types::{ColumnType, Currency};
/// use biscuit_terminal::components::renderable::TerminalRenderable;
///
/// // Build a table with columns and data
/// let mut table = Table::new()
///     .with_title("Product Inventory")
///     .with_columns(vec![
///         TableColumn::new("Product"),
///         TableColumn::new("Price").with_type(ColumnType::Currency(Currency::USD)),
///         TableColumn::new("Stock"),
///     ]);
///
/// table.add_row(vec![
///     TableCellContent::Text("Widget".into()),
///     TableCellContent::Currency(Currency::USD, 29.99),
///     TableCellContent::Integer(150),
/// ]);
///
/// table.add_row(vec![
///     TableCellContent::Text("Gadget".into()),
///     TableCellContent::Currency(Currency::USD, 49.99),
///     TableCellContent::Integer(75),
/// ]);
///
/// let output = table.render_optimistic(Some(80));
/// assert!(output.contains("Product Inventory"));
/// ```
///
/// ## Row Striping
///
/// Enable alternating row colors for improved readability:
///
/// ```
/// use biscuit_terminal::components::table::{Table, TableColumn, TableCellContent};
///
/// let table = Table::new()
///     .with_columns(vec![
///         TableColumn::new("ID"),
///         TableColumn::new("Name"),
///     ])
///     .with_data(vec![
///         vec![TableCellContent::Integer(1), TableCellContent::Text("First".into())],
///         vec![TableCellContent::Integer(2), TableCellContent::Text("Second".into())],
///     ])
///     .alternate_background_color()  // Subtle background on even rows
///     .alternate_text_color();       // Subtle text tint on even rows
/// ```
///
/// ## Multi-line Content
///
/// Cells can contain newlines for multi-line content:
///
/// ```
/// use biscuit_terminal::components::table::{Table, TableColumn, TableCellContent};
/// use biscuit_terminal::utils::layout::WordWrap;
///
/// let table = Table::new()
///     .with_columns(vec![
///         TableColumn::new("Task")
///             .with_max_width(20)
///             .with_word_wrap(WordWrap::WrapProse(Some(3), None)),
///         TableColumn::new("Status"),
///     ])
///     .with_data(vec![
///         vec![
///             TableCellContent::Text("Complete the\nproject documentation".into()),
///             TableCellContent::Text("Done".into()),
///         ],
///     ]);
/// ```
///
/// ## Layout & Style Contract
///
/// `Table` is an internal-layout component (spec C2/C3/C4). The shared
/// render-tree fold resolves the outer box; `Table`'s default width hugs its
/// content, while an explicit fixed width such as `width: 100%` fills the
/// resolved width it receives. In the fill case, the slack sink is the last
/// visible flexible column (spec D2). All applicable `Layout` and `Style`
/// properties route through the fold on every target (C1).
///
/// The `prefer_cursor_alignment` knob (spec C5/C6) keeps a bespoke
/// terminal-only escape hatch: ANSI cursor moves (`CSI N G`) cannot be
/// represented in the render tree, so the cursor core is irreducible. The
/// honored subset for that bespoke path is `margin` / `alignment` /
/// `max_width` (outer-box placement, target-agnostic). Cursor moves
/// replace inter-cell space padding only — they do not change the visible
/// cell text or the outer box position. `render_bespoke` and
/// `render_via_tree` agree on the honored subset after cursor escapes are
/// stripped; parity is pinned in `table_parity.rs`.
#[derive(Debug, Default, Clone)]
pub struct Table {
    title: Option<String>,
    columns: Vec<TableColumn>,
    data: Vec<Vec<TableCellContent>>,
    layout: Layout,
    /// When true, use ANSI cursor positioning for cell alignment instead of
    /// space-based padding. This can improve alignment when glyphs render
    /// narrower than their computed Unicode width.
    prefer_cursor_alignment: bool,
    /// Typed style slot for row striping — the migrated home of the former
    /// `alternate_background_color` / `alternate_text_color` boolean fields.
    style: TableStyle,
    /// Caller-supplied block appearance (color/background/emphasis/border)
    /// overlaid onto the projected table node so both render paths carry it.
    block_style: Style,
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

    /// Expand header and text-cell tabs using a terminal column interval.
    pub(crate) fn expand_tabs_in_place(&mut self, tab_width: usize) {
        for column in &mut self.columns {
            if let std::borrow::Cow::Owned(header) =
                expand_tabs_with_width(&column.header, tab_width)
            {
                column.header = header;
            }
        }
        for row in &mut self.data {
            for cell in row {
                if let TableCellContent::Text(content) = cell
                    && let std::borrow::Cow::Owned(expanded) =
                        expand_tabs_with_width(content, tab_width)
                {
                    *content = expanded;
                }
            }
        }
    }

    pub(crate) fn columns(&self) -> &[TableColumn] {
        &self.columns
    }

    pub(crate) fn data(&self) -> &[Vec<TableCellContent>] {
        &self.data
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

    /// Enable alternating row background colors.
    ///
    /// When enabled, even data rows (0-indexed: rows 1, 3, 5, ...) receive a
    /// background stripe in the adaptive default color for the terminal's
    /// light or dark mode. The color is degraded across color depths, so the
    /// stripe still renders on 256-color and 16-color terminals.
    ///
    /// Compatibility shim: the toggle is stored on the typed [`TableStyle`]
    /// slot. Pair with [`with_stripe_bg`](Self::with_stripe_bg) to override
    /// the default stripe color.
    pub fn alternate_background_color(mut self) -> Self {
        self.style.striped_rows = true;
        self
    }

    /// Enable alternating row text colors.
    ///
    /// When enabled, even data rows (0-indexed: rows 1, 3, 5, ...) receive a
    /// text-color stripe in the adaptive default color for the terminal's
    /// light or dark mode, degraded across color depths.
    ///
    /// Compatibility shim: the toggle is stored on the typed [`TableStyle`]
    /// slot. Pair with [`with_stripe_text`](Self::with_stripe_text) to
    /// override the default stripe color.
    pub fn alternate_text_color(mut self) -> Self {
        self.style.striped_text = true;
        self
    }

    /// Set an explicit background stripe color and enable row striping.
    ///
    /// The [`Color`] is stored on the typed [`TableStyle`] slot and lowered
    /// through the shared, capability-aware color path when the table renders.
    pub fn with_stripe_bg(mut self, color: Color) -> Self {
        self.style.striped_rows = true;
        self.style.stripe_bg = Some(color);
        self
    }

    /// Set an explicit text stripe color and enable text striping.
    pub fn with_stripe_text(mut self, color: Color) -> Self {
        self.style.striped_text = true;
        self.style.stripe_text = Some(color);
        self
    }

    /// The typed [`TableStyle`] slots for this table — row striping plus the
    /// header and body appearance slots.
    pub fn style(&self) -> TableStyle {
        self.style.clone()
    }

    /// Set the typed appearance [`Style`] applied to every header cell.
    ///
    /// A per-column override is merged on top via
    /// [`TableColumn::with_header_style`]. The slot is lowered to ANSI by both
    /// the bespoke renderer and the render-tree renderer.
    ///
    /// ## Examples
    ///
    /// ```
    /// use biscuit_terminal::components::table::Table;
    /// use renderable::style::{Style, TextEmphasis};
    ///
    /// let table = Table::new().with_header_style(Style {
    ///     emphasis: TextEmphasis { bold: true, ..Default::default() },
    ///     ..Style::default()
    /// });
    /// assert!(table.style().header.emphasis.bold);
    /// ```
    pub fn with_header_style(mut self, style: Style) -> Self {
        self.style.header = style;
        self
    }

    /// Set the typed appearance [`Style`] applied to every data (body) cell.
    pub fn with_body_style(mut self, style: Style) -> Self {
        self.style.body = style;
        self
    }

    /// Returns a new `Table` containing only the columns (and their
    /// corresponding data cells) whose `when` condition is satisfied at
    /// the given `available_width`.
    ///
    /// Returns `None` when all columns are already visible (no filtering
    /// needed), avoiding an unnecessary clone.
    #[cfg(test)]
    #[allow(dead_code)]
    fn with_visible_columns(&self, available_width: u32) -> Option<Table> {
        let visible: Vec<usize> = self
            .columns
            .iter()
            .enumerate()
            .filter(|(_, col)| col.when.is_satisfied(available_width))
            .map(|(i, _)| i)
            .collect();

        if visible.len() == self.columns.len() {
            return None; // All columns visible — no filtering needed
        }

        Some(Table {
            title: self.title.clone(),
            columns: visible
                .iter()
                .filter_map(|&i| self.columns.get(i).cloned())
                .collect(),
            data: self
                .data
                .iter()
                .map(|row| {
                    visible
                        .iter()
                        .filter_map(|&i| row.get(i).cloned())
                        .collect()
                })
                .collect(),
            layout: self.layout.clone(),
            prefer_cursor_alignment: self.prefer_cursor_alignment,
            style: self.style.clone(),
            block_style: self.block_style.clone(),
        })
    }

    /// Return the width budget available to the table after margins resolve.
    pub fn available_render_width(&self, terminal_width: u32) -> u32 {
        self.layout.available_width(terminal_width)
    }

    /// Measure the currently visible columns without dropping any columns.
    pub fn measure_widths(
        &self,
        terminal_width: u32,
    ) -> Result<TableWidthMeasurements, TableWidthError> {
        let available_render_width = self.available_render_width(terminal_width) as usize;
        let visible_indices =
            self.visible_column_indices(self.available_render_width(terminal_width));
        self.measure_widths_for_indices(available_render_width, &visible_indices)
    }

    /// Produce the full width plan used by table rendering.
    pub fn plan_widths(&self, terminal_width: u32) -> Result<TableWidthPlan, TableWidthError> {
        let available_render_width = self.available_render_width(terminal_width) as usize;
        self.plan_widths_for_render_width(available_render_width)
    }

    /// Produce the width plan using the supplied terminal's tab interval.
    pub fn plan_widths_for_terminal(
        &self,
        term: &Terminal,
    ) -> Result<TableWidthPlan, TableWidthError> {
        let (table, _) = self.prepare_for_terminal(term);
        table.plan_widths(term.width())
    }

    /// Return whether this table would need wrapping or truncation at the given width.
    pub fn would_wrap(&self, terminal_width: u32) -> Result<bool, TableWidthError> {
        Ok(self.measure_widths(terminal_width)?.word_wrap_needed)
    }

    /// Compatibility helper retained for existing tests.
    #[cfg(test)]
    #[allow(dead_code)]
    fn calculate_column_widths(&self, available_width: Option<u32>) -> Vec<usize> {
        let available_render_width = available_width.unwrap_or(u32::MAX) as usize;
        if self.total_column_count() == 0 {
            return Vec::new();
        }

        match self.plan_widths_for_render_width(available_render_width) {
            Ok(plan) => plan.content_widths(),
            Err(_) => {
                let visible_indices = self
                    .visible_column_indices(available_render_width.min(u32::MAX as usize) as u32);
                self.measure_widths_for_indices(available_render_width, &visible_indices)
                    .map(|measurements| {
                        let mut widths: Vec<usize> = measurements
                            .columns
                            .iter()
                            .map(|column| column.columnar_width_requirement)
                            .collect();
                        if let Some(available) = available_width {
                            compatibility_constrain_widths(
                                &mut widths,
                                &measurements.columns,
                                available as usize,
                            );
                        }
                        widths
                    })
                    .unwrap_or_default()
            }
        }
    }

    fn plan_widths_for_render_width(
        &self,
        available_render_width: usize,
    ) -> Result<TableWidthPlan, TableWidthError> {
        let mut visible_indices = self.visible_column_indices(available_render_width as u32);
        if visible_indices.is_empty() {
            return Err(TableWidthError::NoVisibleColumns);
        }

        let mut dropped_column_indices = Vec::new();
        let mut dropped_notes = Vec::new();

        loop {
            let measurements =
                self.measure_widths_for_indices(available_render_width, &visible_indices)?;

            match self.resolve_width_plan(
                available_render_width,
                visible_indices.clone(),
                measurements,
                dropped_column_indices.clone(),
                dropped_notes.clone(),
            ) {
                Ok(plan) => return Ok(plan),
                Err(error) => {
                    let droppable = visible_indices.iter().enumerate().rev().find_map(
                        |(position, original_index)| {
                            self.column_definition(*original_index)
                                .filter(|column| column.is_droppable())
                                .map(|_| (position, *original_index))
                        },
                    );

                    let Some((drop_position, dropped_index)) = droppable else {
                        return if dropped_column_indices.is_empty() {
                            Err(error)
                        } else {
                            Err(self.to_after_drop_error(error, dropped_column_indices.clone()))
                        };
                    };

                    visible_indices.remove(drop_position);
                    dropped_column_indices.push(dropped_index);
                    if let Some(note) = self
                        .column_definition(dropped_index)
                        .and_then(TableColumn::drop_note)
                    {
                        dropped_notes.push(note);
                    }

                    if visible_indices.is_empty() {
                        return Err(TableWidthError::InsufficientWidthAfterDropping {
                            available_render_width,
                            border_overhead: 0,
                            content_budget: 0,
                            fixed_width_consumption: 0,
                            non_wrapping_consumption: 0,
                            required_width: 0,
                            blocking_column_indices: Vec::new(),
                            dropped_column_indices,
                        });
                    }
                }
            }
        }
    }

    fn measure_widths_for_indices(
        &self,
        available_render_width: usize,
        visible_indices: &[usize],
    ) -> Result<TableWidthMeasurements, TableWidthError> {
        if visible_indices.is_empty() {
            return Err(TableWidthError::NoVisibleColumns);
        }

        let border_overhead = table_border_overhead(visible_indices.len());
        let content_budget = available_render_width.saturating_sub(border_overhead);

        let columns: Vec<MeasuredColumn> = visible_indices
            .iter()
            .map(|&original_index| self.measure_column(original_index))
            .collect();

        let fixed_width_consumption = columns
            .iter()
            .filter(|column| column.fixed_width.is_some())
            .map(|column| column.resolved_width)
            .sum();

        let non_wrapping_consumption = columns
            .iter()
            .filter(|column| column.fixed_width.is_none() && column.is_non_wrapping)
            .map(|column| column.columnar_width_requirement)
            .sum();

        let full_unwrapped_content_width: usize = columns
            .iter()
            .map(|column| column.columnar_width_requirement)
            .sum();

        let reserved_width = fixed_width_consumption + non_wrapping_consumption;
        let working_width = content_budget.saturating_sub(reserved_width);
        let word_wrap_needed = full_unwrapped_content_width > content_budget;

        Ok(TableWidthMeasurements {
            available_render_width,
            border_overhead,
            content_budget,
            fixed_width_consumption,
            non_wrapping_consumption,
            working_width,
            word_wrap_needed,
            columns,
        })
    }

    fn resolve_width_plan(
        &self,
        available_render_width: usize,
        visible_column_indices: Vec<usize>,
        mut measurements: TableWidthMeasurements,
        dropped_column_indices: Vec<usize>,
        dropped_notes: Vec<String>,
    ) -> Result<TableWidthPlan, TableWidthError> {
        let content_budget = measurements.content_budget;
        let reserved_width =
            measurements.fixed_width_consumption + measurements.non_wrapping_consumption;

        if measurements.fixed_width_consumption > content_budget {
            return Err(self.build_width_error(
                &measurements,
                WidthFailureKind::Fixed,
                measurements.fixed_width_consumption,
            ));
        }

        if reserved_width > content_budget {
            return Err(self.build_width_error(
                &measurements,
                WidthFailureKind::NonWrapping,
                reserved_width,
            ));
        }

        let border_overhead = measurements.border_overhead;

        if !measurements.word_wrap_needed {
            for column in &mut measurements.columns {
                if column.fixed_width.is_none() {
                    column.resolved_width = column.columnar_width_requirement;
                }
            }

            self.apply_width_fill(
                &mut measurements.columns,
                available_render_width,
                border_overhead,
            );

            let table_width = table_total_width(
                measurements
                    .columns
                    .iter()
                    .map(|column| column.resolved_width)
                    .collect(),
            );

            return Ok(TableWidthPlan {
                available_render_width,
                visible_column_indices,
                dropped_column_indices,
                columns: measurements.columns,
                table_width,
                dropped_notes,
            });
        }

        let natural_break_total: usize = measurements
            .columns
            .iter()
            .filter(|column| column.is_shrinkable)
            .map(|column| column.natural_break_width)
            .sum();

        if natural_break_total > measurements.working_width {
            let required_width = reserved_width + natural_break_total;
            return Err(self.build_width_error(
                &measurements,
                WidthFailureKind::Wrapping,
                required_width,
            ));
        }

        for column in &mut measurements.columns {
            if column.is_shrinkable {
                column.resolved_width = column.natural_break_width;
            }
        }

        let mut surplus = measurements
            .working_width
            .saturating_sub(natural_break_total);
        if surplus > 0 {
            for column in &mut measurements.columns {
                if !column.is_shrinkable {
                    continue;
                }

                let desired = column
                    .columnar_width_requirement
                    .saturating_sub(column.resolved_width);
                let grant = desired.min(surplus);
                column.resolved_width += grant;
                surplus = surplus.saturating_sub(grant);

                if surplus == 0 {
                    break;
                }
            }
        }

        self.apply_width_fill(
            &mut measurements.columns,
            available_render_width,
            border_overhead,
        );

        let table_width = table_total_width(
            measurements
                .columns
                .iter()
                .map(|column| column.resolved_width)
                .collect(),
        );

        Ok(TableWidthPlan {
            available_render_width,
            visible_column_indices,
            dropped_column_indices,
            columns: measurements.columns,
            table_width,
            dropped_notes,
        })
    }

    /// Grow the last visible column when the table has an explicit width.
    ///
    /// Honors [`Layout::width`](renderable::layout::Layout):
    /// - [`Width::Auto`] (the default) and [`Width::FitContent`] hug the
    ///   content's widest line.
    /// - [`Width::Fixed`] (e.g. `width: 100%` ⇒
    ///   `Width::Fixed(Length::Percent(100.0))`) fills the available width.
    ///
    /// The fill targets the *available width already handed to the planner*, not
    /// a freshly resolved length: whoever sizes the table's box (the render-tree
    /// block-layout step) has already resolved the percentage / `ch` / `max_width`
    /// into `available_render_width`, so re-resolving here would apply a
    /// percentage twice. The last visible column absorbs the slack, capped at its
    /// `max_width` when one is set; slack is only ever added, so a table whose
    /// content already fills the width is unchanged.
    fn apply_width_fill(
        &self,
        columns: &mut [MeasuredColumn],
        available_render_width: usize,
        border_overhead: usize,
    ) {
        if !matches!(self.layout.width, Width::Fixed(_)) {
            return;
        }

        // Filling requires a finite width to fill to. An unbounded width — the
        // `u32::MAX` sentinel a natural-width measurement passes — has nothing to
        // fill, so the table hugs its content regardless of `width`.
        if available_render_width >= u32::MAX as usize {
            return;
        }

        let content_budget = available_render_width.saturating_sub(border_overhead);
        let content_used: usize = columns.iter().map(|column| column.resolved_width).sum();
        if content_budget <= content_used {
            return;
        }
        let mut slack = content_budget - content_used;

        if let Some(last) = columns.last_mut() {
            if let Some(max) = last.max_width {
                slack = slack.min(max.saturating_sub(last.resolved_width));
            }
            last.resolved_width += slack;
        }
    }

    fn visible_column_indices(&self, available_width: u32) -> Vec<usize> {
        (0..self.total_column_count())
            .filter(|index| {
                self.column_definition(*index)
                    .map(|column| column.when.is_satisfied(available_width))
                    .unwrap_or(true)
            })
            .collect()
    }

    fn total_column_count(&self) -> usize {
        self.columns
            .len()
            .max(self.data.iter().map(|row| row.len()).max().unwrap_or(0))
    }

    fn column_definition(&self, column_index: usize) -> Option<&TableColumn> {
        self.columns.get(column_index)
    }

    fn measure_column(&self, column_index: usize) -> MeasuredColumn {
        let default_column = TableColumn::new("");
        let column = self
            .column_definition(column_index)
            .unwrap_or(&default_column);
        // Width is measured from the plain header text: the typed header slot
        // style is lowered to ANSI at render time and contributes no visible
        // width.
        let header_content = column.header.as_str();
        let header_lines = split_lines(header_content);
        let header_width = visible_width(header_content) as usize;
        let header_line_width = measure_max_explicit_line_width(header_content);

        let formatted_cells = self.formatted_column_cells(column_index);
        let cell_max_width = formatted_cells
            .iter()
            .map(|cell| visible_width(cell) as usize)
            .max()
            .unwrap_or(0);
        let cell_line_max_width = formatted_cells
            .iter()
            .map(|cell| measure_max_explicit_line_width(cell))
            .max()
            .unwrap_or(0);

        let normalized_min_width = column.min_width;
        let normalized_max_width = normalize_max_width(column.min_width, column.max_width);
        let effective_word_wrap = column.effective_word_wrap();
        let unclamped_requirement = header_line_width.max(cell_line_max_width);
        let is_non_wrapping = matches!(effective_word_wrap, WordWrap::None);
        let is_shrinkable = !is_non_wrapping && column.fixed_width.is_none();

        let resolved_requirement = if let Some(fixed_width) = column.fixed_width {
            fixed_width
        } else {
            apply_width_constraints(
                unclamped_requirement,
                normalized_min_width,
                normalized_max_width,
            )
        };

        let natural_break_width = if let Some(fixed_width) = column.fixed_width {
            fixed_width
        } else {
            natural_break_width(
                header_content,
                &formatted_cells,
                &effective_word_wrap,
                normalized_min_width,
                normalized_max_width,
                resolved_requirement,
            )
        };

        MeasuredColumn {
            original_index: column_index,
            header_width,
            header_line_width,
            cell_max_width,
            cell_line_max_width,
            columnar_width_requirement: resolved_requirement,
            fixed_width: column.fixed_width,
            min_width: normalized_min_width,
            max_width: normalized_max_width,
            effective_word_wrap,
            natural_break_width,
            resolved_width: resolved_requirement,
            is_non_wrapping,
            is_shrinkable,
            drop_note: column.drop_note(),
            header_lines,
        }
    }

    fn formatted_column_cells(&self, column_index: usize) -> Vec<String> {
        self.data
            .iter()
            .filter_map(|row| row.get(column_index))
            .map(ToString::to_string)
            .collect()
    }

    fn build_width_error(
        &self,
        measurements: &TableWidthMeasurements,
        kind: WidthFailureKind,
        required_width: usize,
    ) -> TableWidthError {
        let blocking_column_indices = match kind {
            WidthFailureKind::Fixed => measurements
                .columns
                .iter()
                .filter(|column| column.fixed_width.is_some())
                .map(|column| column.original_index)
                .collect(),
            WidthFailureKind::NonWrapping => measurements
                .columns
                .iter()
                .filter(|column| column.fixed_width.is_some() || column.is_non_wrapping)
                .map(|column| column.original_index)
                .collect(),
            WidthFailureKind::Wrapping => measurements
                .columns
                .iter()
                .filter(|column| column.is_shrinkable)
                .map(|column| column.original_index)
                .collect(),
        };
        let droppable_columns_available = measurements.columns.iter().any(|column| {
            self.column_definition(column.original_index)
                .map(|definition| definition.is_droppable())
                .unwrap_or(false)
        });

        match kind {
            WidthFailureKind::Fixed => TableWidthError::InsufficientWidthForFixedColumns {
                available_render_width: measurements.available_render_width,
                border_overhead: measurements.border_overhead,
                content_budget: measurements.content_budget,
                fixed_width_consumption: measurements.fixed_width_consumption,
                non_wrapping_consumption: measurements.non_wrapping_consumption,
                required_width,
                blocking_column_indices,
                droppable_columns_available,
            },
            WidthFailureKind::NonWrapping => {
                TableWidthError::InsufficientWidthForNonWrappingColumns {
                    available_render_width: measurements.available_render_width,
                    border_overhead: measurements.border_overhead,
                    content_budget: measurements.content_budget,
                    fixed_width_consumption: measurements.fixed_width_consumption,
                    non_wrapping_consumption: measurements.non_wrapping_consumption,
                    required_width,
                    blocking_column_indices,
                    droppable_columns_available,
                }
            }
            WidthFailureKind::Wrapping => TableWidthError::InsufficientWidthForWrappingColumns {
                available_render_width: measurements.available_render_width,
                border_overhead: measurements.border_overhead,
                content_budget: measurements.content_budget,
                fixed_width_consumption: measurements.fixed_width_consumption,
                non_wrapping_consumption: measurements.non_wrapping_consumption,
                required_width,
                blocking_column_indices,
                droppable_columns_available,
            },
        }
    }

    fn to_after_drop_error(
        &self,
        error: TableWidthError,
        dropped_column_indices: Vec<usize>,
    ) -> TableWidthError {
        match error {
            TableWidthError::InsufficientWidthForFixedColumns {
                available_render_width,
                border_overhead,
                content_budget,
                fixed_width_consumption,
                non_wrapping_consumption,
                required_width,
                blocking_column_indices,
                ..
            }
            | TableWidthError::InsufficientWidthForNonWrappingColumns {
                available_render_width,
                border_overhead,
                content_budget,
                fixed_width_consumption,
                non_wrapping_consumption,
                required_width,
                blocking_column_indices,
                ..
            }
            | TableWidthError::InsufficientWidthForWrappingColumns {
                available_render_width,
                border_overhead,
                content_budget,
                fixed_width_consumption,
                non_wrapping_consumption,
                required_width,
                blocking_column_indices,
                ..
            } => TableWidthError::InsufficientWidthAfterDropping {
                available_render_width,
                border_overhead,
                content_budget,
                fixed_width_consumption,
                non_wrapping_consumption,
                required_width,
                blocking_column_indices,
                dropped_column_indices,
            },
            other => other,
        }
    }

    fn max_content_widths_for_plan(&self, plan: &TableWidthPlan) -> Vec<usize> {
        plan.columns
            .iter()
            .map(|column| {
                self.data
                    .iter()
                    .filter_map(|row| row.get(column.original_index))
                    .map(|cell| visible_width(&cell.to_string()) as usize)
                    .max()
                    .unwrap_or(0)
            })
            .collect()
    }

    fn max_content_widths_for_cursor_plan(&self, plan: &TableWidthPlan) -> Vec<Option<u32>> {
        plan.columns
            .iter()
            .map(|column| {
                let is_uniform = self
                    .column_definition(column.original_index)
                    .map(|definition| definition.uniform_alignment)
                    .unwrap_or(false);

                if is_uniform {
                    Some(
                        self.data
                            .iter()
                            .filter_map(|row| row.get(column.original_index))
                            .map(|cell| visible_width(&cell.to_string()))
                            .max()
                            .unwrap_or(0),
                    )
                } else {
                    None
                }
            })
            .collect()
    }

    fn calculate_row_heights_for_plan(&self, plan: &TableWidthPlan) -> Vec<usize> {
        self.data
            .iter()
            .map(|row| {
                let mut max_height = 1;
                for column in &plan.columns {
                    let strategy = self
                        .column_definition(column.original_index)
                        .map(|definition| definition.effective_word_wrap())
                        .unwrap_or(WordWrap::None);
                    let content = row
                        .get(column.original_index)
                        .map(ToString::to_string)
                        .unwrap_or_default();
                    let wrapped = wrap_cell_content(&content, &strategy, column.resolved_width);
                    max_height = max_height.max(wrapped.len());
                }
                max_height
            })
            .collect()
    }

    /// Resolves the background and text stripe SGR escapes for the given
    /// terminal color mode and depth.
    ///
    /// A stripe is produced only when its [`TableStyle`] toggle is enabled.
    /// The declared slot color — or the adaptive default when the slot is
    /// `None` — is degraded against `depth` through the shared color path, so
    /// striping renders on truecolor, 256-color, and 16-color terminals and
    /// yields `None` only when the terminal has no color support at all.
    fn resolve_stripe_escapes(
        &self,
        color_mode: &ColorMode,
        depth: ColorDepth,
    ) -> (Option<String>, Option<String>) {
        let bg = self
            .style
            .striped_rows
            .then(|| stripe_bg_escape(self.style.stripe_bg, color_mode, depth))
            .flatten();
        let fg = self
            .style
            .striped_text
            .then(|| stripe_fg_escape(self.style.stripe_text, color_mode, depth))
            .flatten();
        (bg, fg)
    }

    /// Render the table content with multi-line cell support.
    ///
    /// If `available_width` is provided, columns will be constrained to fit
    /// within that space (accounting for border overhead).
    ///
    /// `stripe_bg` / `stripe_fg`, when `Some`, are the pre-resolved SGR
    /// escape sequences applied to even data rows (0-indexed: 1, 3, 5, ...).
    /// They are resolved by the caller through the shared, capability-aware
    /// color path so striping degrades with the terminal's color depth.
    ///
    /// `term` supplies the capability context for lowering the typed
    /// [`TableStyle`] header and body appearance slots to ANSI.
    fn render_content(
        &self,
        available_width: Option<u32>,
        stripe_bg: Option<&str>,
        stripe_fg: Option<&str>,
        term: &Terminal,
    ) -> String {
        if self.total_column_count() == 0 {
            return self.title.clone().unwrap_or_default();
        }

        let mut result = String::new();
        let plan =
            match self.plan_widths_for_render_width(available_width.unwrap_or(u32::MAX) as usize) {
                Ok(plan) => plan,
                Err(error) => return error.to_string(),
            };
        let widths = plan.content_widths();

        let max_content_widths = self.max_content_widths_for_plan(&plan);

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
        if !plan.columns.is_empty() {
            let header_lines: Vec<Vec<String>> = plan
                .columns
                .iter()
                .map(|column_plan| {
                    let column = self.column_definition(column_plan.original_index);
                    let header_content = column.map(|col| col.header.as_str()).unwrap_or("");
                    let lines = wrap_cell_content(
                        header_content,
                        &WordWrap::None,
                        column_plan.resolved_width,
                    );
                    // Apply the typed header slot style as ANSI per line, so
                    // each wrapped line is self-contained and survives padding.
                    let header_style = column
                        .map(|col| col.effective_header_style(&self.style.header))
                        .unwrap_or_default();
                    apply_slot_style_lines(lines, &header_style, term)
                })
                .collect();

            let header_height = header_lines
                .iter()
                .map(|lines| lines.len())
                .max()
                .unwrap_or(1);

            // Apply vertical alignment (headers default to Top)
            let padded_headers: Vec<Vec<String>> = header_lines
                .into_iter()
                .enumerate()
                .map(|(i, lines)| {
                    let width = widths.get(i).copied().unwrap_or(0);
                    let vertical_align = self
                        .column_definition(plan.columns[i].original_index)
                        .map(|col| col.vertical_align)
                        .unwrap_or(VerticalAlign::Top);
                    apply_vertical_padding(lines, header_height, vertical_align, width)
                })
                .collect();

            // Render each line of the header
            // Headers use traditional alignment (no max_content_width) since they
            // don't need to align with data rows.
            for line_idx in 0..header_height {
                let mut header_row = String::from("│ ");
                for (i, column_plan) in plan.columns.iter().enumerate() {
                    let width = widths.get(i).copied().unwrap_or(column_plan.resolved_width);
                    let alignment = self
                        .column_definition(column_plan.original_index)
                        .map(TableColumn::effective_alignment)
                        .unwrap_or(Alignment::Left);
                    let line_content = padded_headers
                        .get(i)
                        .and_then(|lines| lines.get(line_idx))
                        .map(|s| s.as_str())
                        .unwrap_or("");
                    header_row.push_str(&pad_cell(line_content, width, alignment, None));
                    if i < plan.columns.len() - 1 {
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
        let row_heights = self.calculate_row_heights_for_plan(&plan);

        // Render data rows with multi-line support
        for (row_idx, row) in self.data.iter().enumerate() {
            let row_height = row_heights.get(row_idx).copied().unwrap_or(1);
            let is_striped = (stripe_bg.is_some() || stripe_fg.is_some()) && row_idx % 2 == 1;
            let active_bg = if stripe_bg.is_some() && row_idx % 2 == 1 {
                stripe_bg
            } else {
                None
            };
            let active_fg = if stripe_fg.is_some() && row_idx % 2 == 1 {
                stripe_fg
            } else {
                None
            };

            // Prepare wrapped and vertically-aligned content for each cell
            let mut cell_lines: Vec<Vec<String>> = Vec::with_capacity(plan.columns.len());
            for (i, column_plan) in plan.columns.iter().enumerate() {
                let width = widths.get(i).copied().unwrap_or(0);
                let strategy = self
                    .column_definition(column_plan.original_index)
                    .map(|col| col.effective_word_wrap())
                    .unwrap_or(WordWrap::None);
                let vertical_align = self
                    .column_definition(column_plan.original_index)
                    .map(|col| col.vertical_align)
                    .unwrap_or(VerticalAlign::Top);

                let content = row
                    .get(column_plan.original_index)
                    .map(ToString::to_string)
                    .unwrap_or_default();
                let wrapped = wrap_cell_content(&content, &strategy, width);
                let wrapped = apply_slot_style_lines(wrapped, &self.style.body, term);
                let padded = apply_vertical_padding(wrapped, row_height, vertical_align, width);
                cell_lines.push(padded);
            }

            // Render each line of the row
            for line_idx in 0..row_height {
                let mut row_str = String::new();
                // Left border is always outside the stripe
                row_str.push('│');
                if let Some(bg) = active_bg {
                    row_str.push_str(bg);
                }
                if let Some(fg) = active_fg {
                    row_str.push_str(fg);
                }
                row_str.push(' ');
                for (i, column_plan) in plan.columns.iter().enumerate() {
                    let width = widths.get(i).copied().unwrap_or(0);
                    let col = self.column_definition(column_plan.original_index);
                    let alignment = col
                        .map(TableColumn::effective_alignment)
                        .unwrap_or(Alignment::Left);
                    // Only use max_content_width for columns with uniform_alignment enabled
                    let max_width = if col.map(|c| c.uniform_alignment).unwrap_or(false) {
                        max_content_widths.get(i).copied()
                    } else {
                        None
                    };

                    let line_content = cell_lines
                        .get(i)
                        .and_then(|lines| lines.get(line_idx))
                        .map(|s| s.as_str())
                        .unwrap_or("");

                    // Cell content may contain \x1b[0m (full SGR reset) or
                    // \x1b[49m (background reset) which kills the stripe bg/fg.
                    // Replace every such reset *within* the content so the stripe
                    // survives between styled spans, not just after the last one.
                    if is_striped {
                        let mut restore = String::new();
                        if let Some(bg) = active_bg {
                            restore.push_str(bg);
                        }
                        if let Some(fg) = active_fg {
                            restore.push_str(fg);
                        }
                        let mut patched = line_content.to_string();
                        if !restore.is_empty() && patched.contains("\x1b[") {
                            // Full SGR reset – restore both bg and fg
                            patched = patched.replace("\x1b[0m", &format!("\x1b[0m{restore}"));
                            // Background-only reset – restore just bg
                            if let Some(bg) = active_bg {
                                patched = patched.replace("\x1b[49m", &format!("\x1b[49m{bg}"));
                            }
                        }
                        // Ensure stripe is active for trailing padding too
                        patched.push_str(&restore);
                        row_str.push_str(&pad_cell(&patched, width, alignment, max_width));
                    } else {
                        row_str.push_str(&pad_cell(line_content, width, alignment, max_width));
                    }
                    if i < plan.columns.len() - 1 {
                        row_str.push_str(" │ ");
                    }
                }
                row_str.push(' ');
                if active_bg.is_some() || active_fg.is_some() {
                    if active_bg.is_some() {
                        row_str.push_str(BG_RESET);
                    }
                    if active_fg.is_some() {
                        row_str.push_str(FG_RESET);
                    }
                }
                // Right border is always outside the stripe
                row_str.push('│');
                result.push_str(&row_str);
                result.push('\n');
            }
        }

        if !widths.is_empty() {
            let bottom_border = build_border(&widths, '└', '┴', '┘');
            result.push_str(&bottom_border);
            result.push('\n');
        }

        append_dropped_notes_block(&mut result, &plan.dropped_notes, None, None);

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
    fn render_with_cursor_positioning(
        &self,
        term_width: u32,
        stripe_bg: Option<&str>,
        stripe_fg: Option<&str>,
        term: &Terminal,
    ) -> String {
        if self.total_column_count() == 0 {
            return self.title.clone().unwrap_or_default();
        }

        let mut result = String::new();

        // Calculate margins first to determine available width for column calculation
        let left_margin = resolve_cells(&self.layout.margin.left, term_width);
        let right_margin = resolve_cells(&self.layout.margin.right, term_width);
        let available_width = term_width
            .saturating_sub(left_margin)
            .saturating_sub(right_margin);

        let plan = match self.plan_widths_for_render_width(available_width as usize) {
            Ok(plan) => plan,
            Err(error) => return error.to_string(),
        };
        let widths = plan.content_widths();

        if widths.is_empty() {
            return result;
        }

        let table_width = plan.table_width as u32;

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

        // Row fill / page background are no longer part of `Layout`.
        let fill_end_col: Option<u32> = None;

        // Collect column alignments
        let alignments: Vec<Alignment> = plan
            .columns
            .iter()
            .map(|column| {
                self.column_definition(column.original_index)
                    .map(TableColumn::effective_alignment)
                    .unwrap_or(Alignment::Left)
            })
            .collect();

        // Calculate max content width per column for uniform alignment.
        // Only populated for columns with `uniform_alignment` enabled;
        // other columns use per-cell visible width so that right/center
        // alignment positions each cell individually.
        let max_content_widths = self.max_content_widths_for_cursor_plan(&plan);

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
        if !plan.columns.is_empty() {
            let header_lines: Vec<Vec<String>> = plan
                .columns
                .iter()
                .map(|column_plan| {
                    let column = self.column_definition(column_plan.original_index);
                    let header_content = column.map(|col| col.header.as_str()).unwrap_or("");
                    let lines = wrap_cell_content(
                        header_content,
                        &WordWrap::None,
                        column_plan.resolved_width,
                    );
                    let header_style = column
                        .map(|col| col.effective_header_style(&self.style.header))
                        .unwrap_or_default();
                    apply_slot_style_lines(lines, &header_style, term)
                })
                .collect();

            let header_height = header_lines
                .iter()
                .map(|lines| lines.len())
                .max()
                .unwrap_or(1);

            // Apply vertical alignment
            let padded_headers: Vec<Vec<String>> = header_lines
                .into_iter()
                .enumerate()
                .map(|(i, lines)| {
                    let width = widths.get(i).copied().unwrap_or(0);
                    let vertical_align = self
                        .column_definition(plan.columns[i].original_index)
                        .map(|col| col.vertical_align)
                        .unwrap_or(VerticalAlign::Top);
                    apply_vertical_padding(lines, header_height, vertical_align, width)
                })
                .collect();

            // Render each line of the header
            // Headers use traditional alignment (no max_content_width) since they
            // don't need to align with data rows.
            for line_idx in 0..header_height {
                let line_cells: Vec<String> = (0..plan.columns.len())
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
                    None, // Headers don't use max_content_widths
                    None, // Headers are never striped
                    None, // Headers are never text-tinted
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
        let row_heights = self.calculate_row_heights_for_plan(&plan);

        // Data rows with multi-line support
        for (row_idx, row) in self.data.iter().enumerate() {
            let row_height = row_heights.get(row_idx).copied().unwrap_or(1);

            // Prepare wrapped and vertically-aligned content for each cell
            let mut cell_lines: Vec<Vec<String>> = Vec::with_capacity(plan.columns.len());
            for (i, column_plan) in plan.columns.iter().enumerate() {
                let width = widths.get(i).copied().unwrap_or(0);
                let strategy = self
                    .column_definition(column_plan.original_index)
                    .map(|col| col.effective_word_wrap())
                    .unwrap_or(WordWrap::None);
                let vertical_align = self
                    .column_definition(column_plan.original_index)
                    .map(|col| col.vertical_align)
                    .unwrap_or(VerticalAlign::Top);

                let content = row
                    .get(column_plan.original_index)
                    .map(ToString::to_string)
                    .unwrap_or_default();
                let wrapped = wrap_cell_content(&content, &strategy, width);
                let wrapped = apply_slot_style_lines(wrapped, &self.style.body, term);
                let padded = apply_vertical_padding(wrapped, row_height, vertical_align, width);
                cell_lines.push(padded);
            }

            // Render each line of the row
            for line_idx in 0..row_height {
                let line_cells: Vec<String> = (0..plan.columns.len())
                    .map(|i| {
                        cell_lines
                            .get(i)
                            .and_then(|lines| lines.get(line_idx))
                            .cloned()
                            .unwrap_or_default()
                    })
                    .collect();

                let row_stripe = if row_idx % 2 == 1 { stripe_bg } else { None };
                let row_text = if row_idx % 2 == 1 { stripe_fg } else { None };
                result.push_str(&render_row_with_cursor_positioning(
                    &line_cells,
                    &widths,
                    &alignments,
                    table_start,
                    fill_end_col,
                    Some(&max_content_widths),
                    row_stripe,
                    row_text,
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
        result.push('\n');
        append_dropped_notes_block(
            &mut result,
            &plan.dropped_notes,
            Some(table_start),
            fill_end_col,
        );

        if result.ends_with('\n') {
            result.pop();
        }

        result
    }
}

impl Table {
    /// Builds the canonical [`NodeKind::Table`] tree node for this table.
    ///
    /// This is the **single private projection helper**. Both
    /// [`TreeRenderable::render_tree`] and the legacy
    /// [`TerminalRenderable::render_tree_node`] hook delegate to it so the
    /// terminal compatibility surface cannot drift from the canonical
    /// tree-renderable producer.
    ///
    /// The first child row is the header row; each remaining child row is a
    /// data row. Most cells carry the readable pre-formatted text as a single
    /// [`NodeKind::Text`] child; a [`TableCellContent::StyledProse`] cell
    /// instead projects its parsed inline children
    /// ([`Prose::to_render_nodes`](crate::components::prose::Prose::to_render_nodes))
    /// directly, degrading any top-level
    /// fenced-code child to escaped literal text. Every cell also carries
    /// [`TableCellHints`] recording the cell kind, the original typed value as
    /// JSON (`null` for `StyledProse`), and alignment. The table node
    /// carries per-column [`TableColumnHints`] and [`TableTerminalHints`], and
    /// the consolidated [`Layout`] when margins are non-default. When the
    /// component carries a non-empty title, it is seeded onto the projected
    /// node as the [`set_table_title`](renderable::tree::NodeAttrs::set_table_title)
    /// hint so each renderer can lower it appropriately
    /// (Terminal: caption above the top border; Browser: `<caption>` inside
    /// `<table>`; Markdown: escaped plain text preceding the table).
    ///
    /// [`Layout`]: renderable::layout::Layout
    ///
    /// [`NodeKind::Table`]: renderable::tree::NodeKind::Table
    /// [`NodeKind::Text`]: renderable::tree::NodeKind::Text
    fn to_render_tree_node(&self) -> RenderNode {
        let align: Vec<ColumnAlign> = self
            .columns
            .iter()
            .map(|col| alignment_to_column_align(col.effective_alignment()))
            .collect();

        let mut rows: Vec<RenderNode> = Vec::with_capacity(self.data.len() + 1);

        // Header row: each cell is the column header text. The effective
        // header slot style (table-wide `TableStyle::header` merged with the
        // per-column override) is projected onto the cell node so the tree
        // renderer lowers the same appearance the bespoke renderer does.
        let header_cells: Vec<RenderNode> = self
            .columns
            .iter()
            .map(|col| {
                let mut cell = RenderNode::table_cell(vec![RenderNode::text(col.header.clone())]);
                let header_style = col.effective_header_style(&self.style.header);
                if !header_style.is_empty() {
                    cell.attrs.set_style(&header_style);
                }
                cell
            })
            .collect();
        rows.push(RenderNode::table_row(header_cells));

        // Data rows: each cell is the readable pre-formatted string plus hints.
        // The table-wide body slot style is projected onto every data cell.
        for row in &self.data {
            let cells: Vec<RenderNode> = row
                .iter()
                .enumerate()
                .map(|(col_idx, content)| {
                    let children = match content {
                        TableCellContent::StyledProse(prose) => {
                            degrade_code_nodes(prose.to_render_nodes())
                        }
                        _ => vec![RenderNode::text(content.to_string())],
                    };
                    let mut cell = RenderNode::table_cell(children);
                    let column = self.columns.get(col_idx);
                    let alignment = column
                        .map(TableColumn::effective_alignment)
                        .unwrap_or(Alignment::Left);
                    let vertical = column
                        .map(|c| c.vertical_align)
                        .unwrap_or(VerticalAlign::Top);
                    cell.attrs.set_table_cell_hints(&TableCellHints {
                        kind: cell_content_kind(content).to_string(),
                        raw_value: cell_content_raw_value(content),
                        alignment: alignment_token(alignment).to_string(),
                        vertical_alignment: vertical_align_token(vertical).to_string(),
                    });
                    if !self.style.body.is_empty() {
                        cell.attrs.set_style(&self.style.body);
                    }
                    cell
                })
                .collect();
            rows.push(RenderNode::table_row(cells));
        }

        let mut node = RenderNode::table(align, rows);

        // Per-column hints on the table node.
        for (idx, col) in self.columns.iter().enumerate() {
            let hints = TableColumnHints {
                min_width: col.min_width.and_then(|w| u32::try_from(w).ok()),
                max_width: col.max_width.and_then(|w| u32::try_from(w).ok()),
                fixed_width: col.fixed_width.and_then(|w| u32::try_from(w).ok()),
                conditional: conditional_to_hint(&col.when),
                // `droppable` is the authoritative signal — `drop_note` is
                // `Some` only for `DropWithMessage`, so silent-drop columns
                // would otherwise round-trip as non-droppable and produce a
                // "Table could not be rendered" error inline.
                droppable: col.is_droppable(),
                drop_note: col.drop_note(),
                uniform_alignment: col.uniform_alignment,
                // Carry the explicit per-column wrap override so the render-tree
                // planner and cell wrapper honor the same break behavior the
                // bespoke planner does. Without this a custom policy is dropped
                // in the tree round-trip and narrow tables fail to lay out.
                word_wrap: col.word_wrap.clone(),
            };
            node.attrs.set_table_column_hints(idx, &hints);
        }

        // Terminal hints on the table node — including any explicit stripe
        // slot colors so the tree renderer lowers the same appearance.
        node.attrs.set_table_terminal_hints(&TableTerminalHints {
            prefer_cursor_alignment: self.prefer_cursor_alignment,
            alternate_background: self.style.striped_rows,
            alternate_text_color: self.style.striped_text,
            stripe_bg: self.style.stripe_bg,
            stripe_text: self.style.stripe_text,
        });

        // Carry the consolidated layout when it differs from the default.
        if self.layout != renderable::layout::Layout::default() {
            node.attrs.set_layout(&self.layout);
        }

        // Honor the caller-supplied title via the typed render-tree caption
        // hint (`RT-TABLE-001`). Renderers ignore empty or whitespace-only
        // titles at render time; this avoids encoding the predicate twice.
        if let Some(title) = self.title.as_ref() {
            node.attrs.set_table_title(title);
        }

        crate::components::renderable::overlay_style_onto_node(&mut node, &self.block_style);
        node
    }

    /// Renders the table through the canonical render tree.
    ///
    /// Used by the [`TerminalRenderable`] impl to route Terminal output
    /// through the same tree the Browser and Markdown paths consume.
    ///
    /// ## Notes
    ///
    /// Failures are logged via `tracing::error!` and fall back to an empty
    /// string rather than the spec's `[render-tree error: …]` sentinel.
    /// The [`TerminalRenderable::render`] trait is infallible by contract,
    /// and emitting an in-band sentinel would pollute user-facing terminal
    /// output for the 30+ CLI consumers. The structured `tracing::error!`
    /// event preserves diagnosability without that user-visible cost; this
    /// is an intentional, documented divergence from the spec's textual
    /// fallback.
    fn render_via_tree(&self, term: &Terminal) -> String {
        let node = self.to_render_tree_node();
        let opts = TerminalRenderOptions::new(term, RenderStrictness::Warn);
        match render_terminal_node(&node, &opts) {
            Ok(rendered) => rendered.output,
            Err(error) => {
                tracing::error!(
                    component = "Table",
                    error = %error,
                    "render_terminal_node failed; emitting empty output"
                );
                String::new()
            }
        }
    }

    /// Resolves every [`StyledProse`](TableCellContent::StyledProse) cell of a
    /// cloned [`Table`] into [`Text`](TableCellContent::Text) for the active
    /// `term`, in place.
    ///
    /// The bespoke paths need a uniform `Vec<Vec<TableCellContent>>` of resolved
    /// content before any width planning, and each Prose cell must be resolved
    /// exactly once. Mutating `table.data` in place avoids a second full clone of
    /// the row data (`Table { data, ..self.clone() }` would clone `self.data`
    /// only to discard it).
    ///
    /// ## Returns
    ///
    /// The number of `StyledProse` cells resolved — used by
    /// [`Self::render_bespoke_instrumented`] to prove the single up-front
    /// resolution pass touches each cell exactly once.
    fn resolve_prose_cells_in_place(table: &mut Table, term: &Terminal) -> usize {
        let mut resolved = 0;
        for row in &mut table.data {
            for cell in row {
                if let TableCellContent::StyledProse(prose) = cell {
                    let rendered = prose.render(term);
                    *cell = TableCellContent::Text(rendered);
                    resolved += 1;
                }
            }
        }
        resolved
    }

    /// Resolve terminal-specific cell content before measuring or rendering.
    fn prepare_for_terminal(&self, term: &Terminal) -> (Table, usize) {
        let mut table = self.clone();
        let resolved = Self::resolve_prose_cells_in_place(&mut table, term);
        table.expand_tabs_in_place(term.tab_width);
        (table, resolved)
    }

    /// Renders via the sanctioned bespoke escape hatch.
    ///
    /// Retained as a `#[doc(hidden)]` surface because [`Table`] supports the
    /// `prefer_cursor_alignment` knob and a TTY-specific cursor-positioning
    /// render path that the render tree cannot yet express. The active
    /// [`TerminalRenderable::render`] path delegates to
    /// [`Self::render_via_tree`] for the standard layout and falls back to
    /// this method when cursor alignment on a TTY is required.
    ///
    /// ## Notes
    ///
    /// `#[doc(hidden)]` because this is an internal escape hatch, not part
    /// of the public surface; `pub` so integration parity tests can reach
    /// it. Removing this without first adding equivalent cursor-alignment
    /// capability to the render tree is a regression.
    #[doc(hidden)]
    pub fn render_bespoke(&self, term: &Terminal) -> String {
        self.render_bespoke_instrumented(term).0
    }

    /// [`Self::render_bespoke`] plus the number of `StyledProse` cells resolved
    /// during the single up-front resolution pass.
    ///
    /// ## Notes
    ///
    /// `#[doc(hidden)]`, `pub` for tests only. The returned count comes from the
    /// same resolution pass the real render uses, so a test can assert each
    /// Prose cell is resolved exactly once *before* any width planning — width
    /// planning then operates on a uniform `Text` grid with no `StyledProse`
    /// left to re-resolve.
    #[doc(hidden)]
    pub fn render_bespoke_instrumented(&self, term: &Terminal) -> (String, usize) {
        let (table, resolved) = self.prepare_for_terminal(term);
        let width = term.width();
        let (stripe_bg, stripe_fg) =
            table.resolve_stripe_escapes(&term.color_mode(), term.color_depth);
        let output = if table.prefer_cursor_alignment && term.is_tty {
            table.render_with_cursor_positioning(
                width,
                stripe_bg.as_deref(),
                stripe_fg.as_deref(),
                term,
            )
        } else {
            let available = table.layout.available_width(width);
            let content = table.render_content(
                Some(available),
                stripe_bg.as_deref(),
                stripe_fg.as_deref(),
                term,
            );
            table.layout.apply_block_layout(&content, width)
        };
        (output, resolved)
    }
}

impl TerminalRenderable for Table {
    /// Renders to a terminal string at an explicit width.
    ///
    /// Routes through the canonical render tree via [`Self::render_via_tree`]
    /// so terminal output matches the Browser and Markdown paths for the same
    /// component. The legacy bespoke output is retained on
    /// [`Self::render_bespoke`] for parity testing.
    ///
    /// ## Notes
    ///
    /// When [`Self::prefer_cursor_alignment`] is set the call delegates to
    /// [`Self::render_bespoke`] so the documented cursor-positioning escape
    /// hatch (used by ~30 production CLI call sites — `claudine`, `sniff`,
    /// `model-citizen`, `messenger`, …) is preserved through the tree-routing
    /// migration. The optimistic terminal is a TTY by construction, so the
    /// inner `is_tty` guard in `render_bespoke` is satisfied.
    fn render_optimistic(&self, term_width: Option<u32>) -> String {
        let term = Terminal::new_optimistic(term_width.unwrap_or(80));
        if self.prefer_cursor_alignment && term.is_tty {
            self.render_bespoke(&term)
        } else {
            self.render_via_tree(&term)
        }
    }

    /// Renders to the supplied terminal.
    ///
    /// Routes through the canonical render tree via [`Self::render_via_tree`].
    /// The legacy bespoke output is retained on [`Self::render_bespoke`] for
    /// parity testing.
    ///
    /// ## Notes
    ///
    /// When [`Self::prefer_cursor_alignment`] is set **and** the terminal is
    /// a TTY, the call delegates to [`Self::render_bespoke`] so the documented
    /// cursor-positioning escape hatch (used by ~30 production CLI call sites)
    /// keeps emitting ANSI column-move sequences (`CSI N G`) rather than
    /// silently degrading to tree-rendered space padding. Non-TTY destinations
    /// (pipes, file redirects, capture buffers) always take the tree path so
    /// captured output stays free of cursor-control bytes.
    fn render(&self, term: &Terminal) -> String {
        if self.prefer_cursor_alignment && term.is_tty {
            self.render_bespoke(term)
        } else {
            self.render_via_tree(term)
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

    fn style(&self) -> Style {
        self.block_style.clone()
    }

    fn style_mut(&mut self) -> Option<&mut Style> {
        Some(&mut self.block_style)
    }

    fn is_block_level(&self) -> bool {
        true
    }

    /// Projects this table into a canonical [`NodeKind::Table`] render node.
    ///
    /// Delegates to the single private projection helper
    /// [`Self::to_render_tree_node`], shared with
    /// [`TreeRenderable::render_tree`] so the terminal compatibility hook and
    /// the canonical tree producer cannot drift.
    ///
    /// [`NodeKind::Table`]: renderable::tree::NodeKind::Table
    fn render_tree_node(&self) -> Option<RenderNode> {
        Some(self.to_render_tree_node())
    }
}

impl TreeRenderable for Table {
    /// Projects the table into the canonical render tree.
    ///
    /// Delegates to the single private projection helper
    /// [`Table::to_render_tree_node`] so this canonical entry point and the
    /// terminal-compatibility [`TerminalRenderable::render_tree_node`] hook
    /// share one source of truth.
    ///
    /// [`NodeKind::Table`]: renderable::tree::NodeKind::Table
    fn render_tree(&self) -> RenderNode {
        self.to_render_tree_node()
    }
}

impl Table {
    /// Shared Markdown lowering for both portable Markdown and MarkdownPlus.
    ///
    /// Centralises the tree-projection + `render_markdown_node` invocation
    /// so the two `MarkdownRenderable` entry points cannot drift on error
    /// handling or option construction. The `dialect` parameter is passed
    /// straight through to [`MarkdownRenderOptions`], keeping the function
    /// faithful to whatever distinction the dialects encode.
    fn render_markdown_for_dialect(&self, dialect: MarkdownDialect) -> String {
        let node = <Self as TreeRenderable>::render_tree(self);
        let opts = MarkdownRenderOptions {
            dialect,
            ..MarkdownRenderOptions::default()
        };
        match render_markdown_node(&node, &opts) {
            Ok(rendered) => rendered.output,
            Err(error) => {
                tracing::error!(
                    component = "Table",
                    dialect = ?dialect,
                    error = %error,
                    "render_markdown_node failed; emitting empty output"
                );
                String::new()
            }
        }
    }
}

impl MarkdownRenderable for Table {
    /// Renders the table as portable Markdown (GFM) via the canonical render
    /// tree.
    ///
    /// Output is a pipe-delimited GFM table. Cell content is escaped to keep
    /// the table structure valid: literal `|` becomes `\|`, soft breaks become
    /// spaces, and hard breaks plus literal newlines become `<br>`. A
    /// non-empty title is emitted as escaped plain text on its own line
    /// followed by a blank line before the table.
    fn render_markdown(&self) -> String {
        self.render_markdown_for_dialect(MarkdownDialect::Markdown)
    }

    /// Renders the table as MarkdownPlus via the canonical render tree.
    ///
    /// Table structure is pure GFM, so MarkdownPlus output is structurally
    /// identical to portable Markdown. Inline emphasis/code/link rendering
    /// may diverge once style-aware Markdown lowering lands, but the
    /// pipe-delimited table shape is unchanged.
    fn render_markdown_plus(&self) -> String {
        self.render_markdown_for_dialect(MarkdownDialect::MarkdownPlus)
    }
}

impl BrowserRenderable for Table {
    /// Renders the table as an HTML fragment via the canonical render tree.
    ///
    /// The browser tree renderer emits a `<table>` with `<thead>` (first
    /// row), `<tbody>` (remaining rows), and column alignment as
    /// `style="text-align:…"`. A non-empty title is emitted as the first
    /// child `<caption>` element inside the `<table>`.
    ///
    /// Failures are logged via `tracing::error!` and fall back to an empty
    /// [`BrowserFragment`]: the [`BrowserRenderable`] contract is infallible,
    /// and surfacing a `[render-tree error: …]` sentinel as in-band HTML
    /// would pollute the rendered page.
    fn render_html_fragment(&self) -> BrowserFragment<Ready> {
        let node = <Self as TreeRenderable>::render_tree(self);
        let opts = BrowserRenderOptions::default();
        match render_browser_node(&node, &opts) {
            Ok(rendered) => rendered.output,
            Err(error) => {
                tracing::error!(
                    component = "Table",
                    error = %error,
                    "render_browser_node failed; emitting empty fragment"
                );
                BrowserFragment::new()
                    .define_as_text_fragment(String::new())
                    .finalize()
            }
        }
    }

    fn render_html_page(&self, page: Option<PageOptions>) -> HtmlPage {
        let mut html_page = HtmlPage::from(self.render_html_fragment());
        if let Some(options) = page {
            html_page.apply_page_options(options);
        }
        html_page
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Maps a layout [`Alignment`] to a render-tree [`ColumnAlign`].
fn alignment_to_column_align(alignment: Alignment) -> ColumnAlign {
    match alignment {
        Alignment::Left => ColumnAlign::Left,
        Alignment::Center => ColumnAlign::Center,
        Alignment::Right => ColumnAlign::Right,
    }
}

/// Returns the cell-hint alignment token for a layout [`Alignment`].
fn alignment_token(alignment: Alignment) -> &'static str {
    match alignment {
        Alignment::Left => "left",
        Alignment::Center => "center",
        Alignment::Right => "right",
    }
}

/// Returns the cell-hint vertical-alignment token for a [`VerticalAlign`].
fn vertical_align_token(vertical: VerticalAlign) -> &'static str {
    match vertical {
        VerticalAlign::Top => "top",
        VerticalAlign::Middle => "middle",
        VerticalAlign::Bottom => "bottom",
    }
}

/// Returns the cell-hint kind token for a [`TableCellContent`].
fn cell_content_kind(content: &TableCellContent) -> &'static str {
    match content {
        TableCellContent::Text(_) => "text",
        TableCellContent::Integer(_) => "integer",
        TableCellContent::Float(_) => "float",
        TableCellContent::Currency(_, _) => "currency",
        TableCellContent::StyledProse(_) => "styled_prose",
    }
}

/// Returns the original typed value of a [`TableCellContent`] as JSON.
fn cell_content_raw_value(content: &TableCellContent) -> serde_json::Value {
    match content {
        TableCellContent::Text(s) => serde_json::Value::String(s.clone()),
        TableCellContent::Integer(n) => serde_json::Value::from(*n),
        TableCellContent::Float(n) => serde_json::Value::from(*n),
        TableCellContent::Currency(currency, amount) => serde_json::json!({
            "currency": currency_token(currency),
            "amount": amount,
        }),
        TableCellContent::StyledProse(_) => serde_json::Value::Null,
    }
}

/// Replaces top-level `NodeKind::Code` children with `NodeKind::Text` nodes
/// containing the code body as literal text. Inline structure (Strong,
/// Emphasis, Link, Span, Text, etc.) is preserved as-is.
fn degrade_code_nodes(nodes: Vec<RenderNode>) -> Vec<RenderNode> {
    nodes
        .into_iter()
        .map(|node| match &node.kind {
            renderable::tree::NodeKind::Code { value, .. } => {
                RenderNode::text(value.clone())
            }
            _ => node,
        })
        .collect()
}

/// Returns the ISO-style token for a [`Currency`].
fn currency_token(currency: &Currency) -> &'static str {
    match currency {
        Currency::USD => "USD",
        Currency::GBP => "GBP",
        Currency::EUR => "EUR",
    }
}

/// Maps a column [`super::column::Conditional`] to a [`ColumnConditional`] hint.
fn conditional_to_hint(conditional: &super::column::Conditional) -> ColumnConditional {
    match conditional {
        super::column::Conditional::Always => ColumnConditional::Always,
        super::column::Conditional::WidthGreaterThan(n) => ColumnConditional::WidthGreaterThan(*n),
        super::column::Conditional::LessThanOrEqual(n) => ColumnConditional::LessThanOrEqual(*n),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WidthFailureKind {
    Fixed,
    NonWrapping,
    Wrapping,
}

fn table_border_overhead(column_count: usize) -> usize {
    if column_count == 0 {
        0
    } else {
        4 + 3 * (column_count.saturating_sub(1))
    }
}

fn table_total_width(widths: Vec<usize>) -> usize {
    if widths.is_empty() {
        0
    } else {
        table_border_overhead(widths.len()) + widths.iter().sum::<usize>()
    }
}

fn normalize_max_width(min_width: Option<usize>, max_width: Option<usize>) -> Option<usize> {
    match (min_width, max_width) {
        (Some(min), Some(max)) => Some(min.max(max)),
        (_, max) => max,
    }
}

fn apply_width_constraints(
    width: usize,
    min_width: Option<usize>,
    max_width: Option<usize>,
) -> usize {
    let mut resolved = width;
    if let Some(max) = max_width {
        resolved = resolved.min(max);
    }
    if let Some(min) = min_width {
        resolved = resolved.max(min);
    }
    resolved
}

fn measure_explicit_line_widths(content: &str) -> Vec<usize> {
    let lines = split_lines(content);
    if lines.is_empty() {
        vec![0]
    } else {
        lines
            .into_iter()
            .map(|line| visible_width(&line) as usize)
            .collect()
    }
}

fn measure_max_explicit_line_width(content: &str) -> usize {
    measure_explicit_line_widths(content)
        .into_iter()
        .max()
        .unwrap_or(0)
}

fn measure_break_segments(content: &str, wrap: &WordWrap) -> Vec<usize> {
    split_lines(content)
        .into_iter()
        .flat_map(|line| measure_break_segments_for_line(&line, wrap))
        .collect()
}

fn measure_break_segments_for_line(line: &str, wrap: &WordWrap) -> Vec<usize> {
    match wrap {
        WordWrap::None => vec![visible_width(line) as usize],
        WordWrap::Truncate(_) => vec![1],
        WordWrap::WrapProse(_, _) => measure_segment_widths(line, &['-']),
        WordWrap::BespokeProse(_, chars, _) => measure_segment_widths(line, chars),
    }
}

fn measure_segment_widths(line: &str, extra_break_chars: &[char]) -> Vec<usize> {
    let mut widths = Vec::new();
    let mut current_width = 0usize;
    let bytes = line.as_bytes();
    let mut idx = 0usize;

    while idx < line.len() {
        if bytes[idx] == 0x1b {
            idx = measurement_escape_sequence_end(line, idx);
            continue;
        }

        let Some(ch) = line[idx..].chars().next() else {
            break;
        };
        let ch_len = ch.len_utf8();
        let ch_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1);

        if ch_width == 0 {
            idx += ch_len;
            continue;
        }

        if ch.is_whitespace() {
            widths.push(current_width);
            current_width = 0;
        } else if extra_break_chars.contains(&ch) {
            current_width += ch_width;
            widths.push(current_width);
            current_width = 0;
        } else {
            current_width += ch_width;
        }

        idx += ch_len;
    }

    widths.push(current_width);

    let widths: Vec<usize> = widths.into_iter().filter(|width| *width > 0).collect();
    if widths.is_empty() { vec![0] } else { widths }
}

fn measurement_escape_sequence_end(content: &str, start: usize) -> usize {
    let bytes = content.as_bytes();
    if start >= bytes.len() {
        return bytes.len();
    }
    if bytes[start] != 0x1b {
        return (start + 1).min(bytes.len());
    }
    if start + 1 >= bytes.len() {
        return bytes.len();
    }

    match bytes[start + 1] {
        b'[' => {
            let mut idx = start + 2;
            while idx < bytes.len() {
                let byte = bytes[idx];
                idx += 1;
                if (0x40..=0x7e).contains(&byte) {
                    break;
                }
            }
            idx
        }
        b']' => {
            let mut idx = start + 2;
            while idx < bytes.len() {
                let byte = bytes[idx];
                if byte == 0x07 {
                    idx += 1;
                    break;
                }
                if byte == 0x1b && idx + 1 < bytes.len() && bytes[idx + 1] == b'\\' {
                    idx += 2;
                    break;
                }
                idx += 1;
            }
            idx
        }
        b'_' => {
            let mut idx = start + 2;
            while idx < bytes.len() {
                if bytes[idx] == 0x1b && idx + 1 < bytes.len() && bytes[idx + 1] == b'\\' {
                    idx += 2;
                    break;
                }
                idx += 1;
            }
            idx
        }
        _ => {
            if let Some(ch) = content[start + 1..].chars().next() {
                start + 1 + ch.len_utf8()
            } else {
                bytes.len()
            }
        }
    }
}

fn natural_break_width(
    header_content: &str,
    formatted_cells: &[String],
    wrap: &WordWrap,
    min_width: Option<usize>,
    max_width: Option<usize>,
    columnar_width_requirement: usize,
) -> usize {
    if columnar_width_requirement <= 5 {
        return columnar_width_requirement;
    }

    let base = match wrap {
        WordWrap::None => columnar_width_requirement,
        WordWrap::Truncate(_) => min_width.unwrap_or(1).max(1),
        WordWrap::WrapProse(_, _) | WordWrap::BespokeProse(_, _, _) => {
            let mut widest = measure_break_segments(header_content, wrap)
                .into_iter()
                .max()
                .unwrap_or(0);
            for cell in formatted_cells {
                widest = widest.max(
                    measure_break_segments(cell, wrap)
                        .into_iter()
                        .max()
                        .unwrap_or(0),
                );
            }
            widest
        }
    };

    apply_width_constraints(base.max(min_width.unwrap_or(1)), min_width, max_width)
}

fn append_dropped_notes_block(
    result: &mut String,
    dropped_notes: &[String],
    start_col: Option<u32>,
    fill_end_col: Option<u32>,
) {
    for note in dropped_notes {
        if let Some(col) = start_col {
            result.push_str(&format!("\x1b[{}G- {}", col, note));
            if let Some(end) = fill_end_col {
                let line_width = visible_width(&format!("- {}", note));
                let line_end = col + line_width;
                if end > line_end {
                    result.push_str(&" ".repeat((end - line_end) as usize));
                }
            }
        } else {
            result.push_str("- ");
            result.push_str(note);
        }
        result.push('\n');
    }
}

#[cfg(test)]
#[allow(dead_code)]
fn compatibility_constrain_widths(
    widths: &mut [usize],
    columns: &[MeasuredColumn],
    available_width: usize,
) {
    if widths.is_empty() {
        return;
    }

    let max_content_width = available_width.saturating_sub(table_border_overhead(widths.len()));
    let mut current_content_width: usize = widths.iter().sum();
    if current_content_width <= max_content_width {
        return;
    }

    loop {
        let mut progress = false;
        for (index, width) in widths.iter_mut().enumerate() {
            let Some(column) = columns.get(index) else {
                continue;
            };

            if column.fixed_width.is_some() || column.is_non_wrapping {
                continue;
            }

            let min_width = column.min_width.unwrap_or(1);
            if *width <= min_width {
                continue;
            }

            *width -= 1;
            current_content_width = current_content_width.saturating_sub(1);
            progress = true;

            if current_content_width <= max_content_width {
                return;
            }
        }

        if !progress {
            return;
        }
    }
}

/// Render a row with cursor positioning, supporting cell alignment.
///
/// If `fill_end_col` is Some, fills with spaces to that column for background color support.
/// If `max_content_widths` is provided, alignment offsets use these widths instead of
/// individual content widths for columns with `Some(width)`, ensuring uniform alignment.
/// Columns with `None` use per-cell visible width for individual positioning.
/// If `stripe_bg` is Some, the background escape is applied after the left border and
/// reset before the right border so the outer `│` characters remain uncolored.
/// If `stripe_fg` is Some, the foreground escape is applied similarly and reset at borders.
#[allow(clippy::too_many_arguments)]
fn render_row_with_cursor_positioning(
    cells: &[String],
    widths: &[usize],
    alignments: &[Alignment],
    table_start: u32,
    fill_end_col: Option<u32>,
    max_content_widths: Option<&[Option<u32>]>,
    stripe_bg: Option<&str>,
    stripe_fg: Option<&str>,
) -> String {
    let has_stripe = stripe_bg.is_some() || stripe_fg.is_some();
    let mut row = String::new();
    // Left border is always outside the stripe
    row.push_str(&format!("\x1b[{}G│", table_start));
    if has_stripe {
        if let Some(bg) = stripe_bg {
            row.push_str(bg);
        }
        if let Some(fg) = stripe_fg {
            row.push_str(fg);
        }
        // Explicitly fill the border padding space after left │ so the
        // cursor jump to cell_start doesn't leave it with default bg.
        row.push(' ');
    }

    // Track cursor position for row fill
    let mut last_col = table_start + 1;

    // cell_start is where content area begins (after "│ ")
    let mut cell_start = table_start.saturating_add(2);

    for (index, width) in widths.iter().enumerate() {
        let content = cells.get(index).map(String::as_str).unwrap_or("");
        let cell_width = *width as u32;
        let alignment = alignments.get(index).copied().unwrap_or(Alignment::Left);

        // Use max_content_width for uniform-aligned columns (consistent position
        // across rows), otherwise fall back to individual content width (each
        // cell positioned independently within the column).
        let width_for_alignment = max_content_widths
            .and_then(|mcw| mcw.get(index).copied())
            .flatten()
            .unwrap_or_else(|| visible_width(content));

        // Calculate cursor offset within cell based on alignment
        let cursor_offset = match alignment {
            Alignment::Left => 0,
            Alignment::Right => cell_width.saturating_sub(width_for_alignment),
            Alignment::Center => cell_width.saturating_sub(width_for_alignment) / 2,
        };

        // For striped rows, pre-fill the cell area with bg-colored spaces.
        // Cursor positioning jumps over positions without writing, so
        // alignment gaps would show the default bg without this fill.
        // The actual content overwrites its portion; the remaining spaces
        // keep the stripe bg.
        if stripe_bg.is_some() {
            row.push_str(&format!(
                "\x1b[{}G{}",
                cell_start,
                " ".repeat(cell_width as usize)
            ));
        }

        let content_col = cell_start.saturating_add(cursor_offset);

        // Skip output for padding-only content (all spaces from vertical
        // padding).  With cursor positioning the separator jump handles
        // the blank space; physically writing spaces at content_col would
        // overshoot the cell boundary when cursor_offset > 0 (right/center
        // alignment), making the terminal line wider than the table and
        // causing a visual line wrap ("blank line").
        let is_padding_only = content.bytes().all(|b| b == b' ');
        if !is_padding_only {
            // Patch mid-content resets so the stripe survives between
            // styled Prose spans (e.g. <bg-red>A</bg-red> gap <bg-red>B</bg-red>).
            let patched: std::borrow::Cow<'_, str> = if has_stripe && content.contains("\x1b[") {
                let mut s = content.to_string();
                // Build full restore (bg + fg)
                let mut restore = String::new();
                if let Some(bg) = stripe_bg {
                    restore.push_str(bg);
                }
                if let Some(fg) = stripe_fg {
                    restore.push_str(fg);
                }
                if !restore.is_empty() {
                    s = s.replace("\x1b[0m", &format!("\x1b[0m{restore}"));
                }
                // Background-only reset
                if let Some(bg) = stripe_bg {
                    s = s.replace("\x1b[49m", &format!("\x1b[49m{bg}"));
                }
                std::borrow::Cow::Owned(s)
            } else {
                std::borrow::Cow::Borrowed(content)
            };
            row.push_str(&format!("\x1b[{}G{}\x1b[0m", content_col, patched));
        }

        // The \x1b[0m above resets any active SGR from cell content (colors,
        // bold, etc.) so they don't bleed into separators or adjacent cells.
        // For striped rows, re-apply the stripe bg/fg that the reset killed.
        if has_stripe && !is_padding_only {
            if let Some(bg) = stripe_bg {
                row.push_str(bg);
            }
            if let Some(fg) = stripe_fg {
                row.push_str(fg);
            }
        }

        // Position separator at end of cell
        let sep_col = cell_start.saturating_add(cell_width);
        if index + 1 == widths.len() {
            // Last cell: reset bg/fg before right border so │ is uncolored
            if has_stripe {
                let mut resets = String::new();
                if stripe_bg.is_some() {
                    resets.push_str(BG_RESET);
                }
                if stripe_fg.is_some() {
                    resets.push_str(FG_RESET);
                }
                row.push_str(&format!("\x1b[{}G {}", sep_col, resets));
                row.push('│');
            } else {
                row.push_str(&format!("\x1b[{}G │", sep_col));
            }
            last_col = sep_col + 2;
        } else {
            row.push_str(&format!("\x1b[{}G │ ", sep_col));
            last_col = sep_col + 3;
        }

        // Next cell starts after " │ "
        cell_start = sep_col.saturating_add(3);
    }

    // Fill to end column if requested (for background color support)
    if let Some(end) = fill_end_col
        && end > last_col
    {
        row.push_str(&" ".repeat((end - last_col) as usize));
    }

    row
}

/// Wrap cell content according to the given word wrap strategy and width.
///
/// Returns a vector of lines representing the wrapped content. Handles both
/// explicit newlines in the content and word wrap overflow.
/// Wraps each line of cell content in the SGR run of a typed slot [`Style`].
///
/// The header and body appearance slots ([`TableStyle`]) are lowered to ANSI
/// through the shared, capability-aware [`text_appearance_sgr`] path. Each
/// non-empty line is opened with the slot's SGR run and closed with
/// [`SGR_RESET`], so a line stays self-contained across padding and column
/// boundaries. An empty slot style returns the lines unchanged.
fn apply_slot_style_lines(lines: Vec<String>, style: &Style, term: &Terminal) -> Vec<String> {
    if style.is_empty() {
        return lines;
    }
    let open = text_appearance_sgr(style, term);
    if open.is_empty() {
        return lines;
    }
    lines
        .into_iter()
        .map(|line| {
            if line.is_empty() {
                line
            } else {
                format!("{open}{line}{SGR_RESET}")
            }
        })
        .collect()
}

pub(crate) fn wrap_cell_content(content: &str, strategy: &WordWrap, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![String::new()];
    }

    // First split on explicit newlines
    let lines = split_lines(content);

    if matches!(strategy, WordWrap::None) {
        if lines.is_empty() {
            return vec![String::new()];
        }
        // Even without word wrap, a cell may hold explicit newlines (e.g. a
        // multiline `StyledProse`). Balance the SGR per line so a color or
        // emphasis run cannot bleed across the split into padding, borders, or
        // the next row. No-op for a single line.
        return sanitize_wrapped_lines(lines);
    }

    // Apply word wrapping to each line, then ensure each resulting line is
    // ANSI-self-contained so colors/links don't bleed across table columns.
    sanitize_wrapped_lines(wrap_lines(lines, strategy, width as u32))
}

/// Calculate the height (number of lines) needed for each row based on wrapped content.
///
/// Returns a vector where each element is the number of lines needed for that row.
#[cfg(test)]
#[allow(dead_code)]
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
pub(crate) fn apply_vertical_padding(
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

/// The adaptive default background stripe [`Color`] for a color mode.
///
/// A subtle tint that stays visible on common dark themes such as Tokyo Night,
/// plus a soft cool gray in light mode. Each carries a [`BasicColor`] fallback
/// so it degrades on 16-color terminals.
pub(crate) fn default_stripe_bg(color_mode: &ColorMode) -> Color {
    match color_mode {
        ColorMode::Light => Color::Rgb(RgbColor::new(226, 229, 236, BasicColor::White)),
        ColorMode::Dark | ColorMode::Unknown => {
            Color::Rgb(RgbColor::new(36, 40, 59, BasicColor::Black))
        }
    }
}

/// The adaptive default text stripe [`Color`] for a color mode.
pub(crate) fn default_stripe_text(color_mode: &ColorMode) -> Color {
    match color_mode {
        ColorMode::Light => Color::Rgb(RgbColor::new(80, 80, 90, BasicColor::Black)),
        ColorMode::Dark | ColorMode::Unknown => {
            Color::Rgb(RgbColor::new(180, 180, 190, BasicColor::White))
        }
    }
}

/// Lowers the alternating-row background stripe to an SGR escape.
///
/// `explicit` is the component's declared [`TableStyle`](super::types::TableStyle)
/// slot color; `None` selects [`default_stripe_bg`] for `color_mode`. Either
/// way the [`Color`] is degraded against `depth` through the shared
/// [`color_sgr`] path, so striping renders on truecolor, 256-color, and
/// 16-color terminals and is dropped only when the terminal has no color
/// support.
pub(crate) fn stripe_bg_escape(
    explicit: Option<Color>,
    color_mode: &ColorMode,
    depth: ColorDepth,
) -> Option<String> {
    let color = explicit.unwrap_or_else(|| default_stripe_bg(color_mode));
    color_sgr(color, &depth, true)
}

/// Lowers the alternating-row text stripe to an SGR escape.
///
/// See [`stripe_bg_escape`]; this is the foreground counterpart.
pub(crate) fn stripe_fg_escape(
    explicit: Option<Color>,
    color_mode: &ColorMode,
    depth: ColorDepth,
) -> Option<String> {
    let color = explicit.unwrap_or_else(|| default_stripe_text(color_mode));
    color_sgr(color, &depth, false)
}

/// The escape sequence that resets only the background color.
pub(crate) const BG_RESET: &str = "\x1b[49m";

/// The escape sequence that resets only the foreground color.
pub(crate) const FG_RESET: &str = "\x1b[39m";

pub(crate) fn build_border(widths: &[usize], left: char, junction: char, right: char) -> String {
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
    use super::super::cell::{
        format_currency, format_float, format_integer, insert_thousands_separators,
    };
    use super::super::column::Conditional;
    use super::super::types::{ColumnType, Currency};
    use super::*;

    /// The default truecolor background stripe escape for a color mode.
    ///
    /// Test-only convenience mirroring [`stripe_bg_escape`]'s no-explicit /
    /// truecolor result as a static string, so striping tests can build
    /// expected substrings without unwrapping.
    fn bg_escape(mode: &ColorMode) -> &'static str {
        match mode {
            ColorMode::Light => "\x1b[48;2;235;235;238m",
            ColorMode::Dark | ColorMode::Unknown => "\x1b[48;2;30;30;34m",
        }
    }

    /// The default truecolor text stripe escape for a color mode.
    fn fg_escape(mode: &ColorMode) -> &'static str {
        match mode {
            ColorMode::Light => "\x1b[38;2;80;80;90m",
            ColorMode::Dark | ColorMode::Unknown => "\x1b[38;2;180;180;190m",
        }
    }

    // ── Existing tests ────────────────────────────────────────────

    #[test]
    fn test_simple_table() {
        let table = Table::new()
            .with_columns(vec![TableColumn::new("Name"), TableColumn::new("Age")])
            .with_data(vec![
                vec!["Alice".into(), "30".into()],
                vec!["Bob".into(), "25".into()],
            ]);

        let result = table.render_optimistic(None);
        assert!(result.contains("Name"));
        assert!(result.contains("Alice"));
        assert!(result.contains("Bob"));
    }

    // ── table width planning ──────────────────────────────────────

    fn two_column_table() -> Table {
        Table::new()
            .with_columns(vec![TableColumn::new("A"), TableColumn::new("B")])
            .with_data(vec![vec!["x".into(), "y".into()]])
    }

    #[test]
    fn width_auto_hugs_content_below_available() {
        let plan = two_column_table().plan_widths(60).expect("plan");
        assert_eq!(plan.table_width, 9, "Auto must hug content by default");
        let widths = plan.content_widths();
        assert_eq!(widths, vec![1, 1], "Auto must not assign slack: {widths:?}");
    }

    #[test]
    fn width_auto_hugs_when_width_is_unbounded() {
        let widths = two_column_table().calculate_column_widths(None);
        assert_eq!(
            widths,
            vec![1, 1],
            "unbounded width must also hug, not fill to u32::MAX; widths={widths:?}"
        );
    }

    #[test]
    fn width_fit_content_matches_auto_hugging_behavior() {
        let mut table = two_column_table();
        table.layout_mut().width = Width::FitContent;
        let plan = table.plan_widths(60).expect("plan");
        assert_eq!(plan.table_width, 9, "FitContent must hug content");
    }

    #[test]
    fn default_worktree_shaped_table_does_not_expand_commits_column() {
        let table = Table::new()
            .with_columns(vec![
                TableColumn::new("Worktree"),
                TableColumn::new("Worktree Name"),
                TableColumn::new("Branch"),
                TableColumn::new("Merge"),
                TableColumn::new("Commits"),
            ])
            .with_data(vec![
                vec![
                    "Clean".into(),
                    "main::(rusty-biscuit)".into(),
                    "main".into(),
                    "".into(),
                    "+275 -55".into(),
                ],
                vec![
                    "Dirty".into(),
                    "terminal".into(),
                    "terminal".into(),
                    "clean".into(),
                    "+1".into(),
                ],
            ]);

        let plan = table.plan_widths(180).expect("plan");
        assert!(
            plan.table_width < 120,
            "default table should hug content, not fill terminal width: {}",
            plan.table_width
        );
        assert_eq!(
            plan.content_widths()[4],
            "+275 -55".len(),
            "last column should not absorb terminal slack by default"
        );
    }

    #[test]
    fn width_fixed_full_fills_last_column_to_available() {
        let mut table = two_column_table();
        table.layout_mut().width = Width::Fixed(renderable::layout::TargetValue::universal(
            renderable::layout::Length::Percent(100.0),
        ));

        let plan = table.plan_widths(60).expect("plan");
        assert_eq!(
            plan.table_width, 60,
            "width: 100% must fill the available width"
        );

        let widths = plan.content_widths();
        assert!(
            widths[1] > widths[0],
            "the last column must absorb the slack; widths={widths:?}"
        );
    }

    #[test]
    fn width_fixed_full_respects_last_column_max_width() {
        // When the last column is capped, fill cannot exceed that cap, so the
        // table may stop short of the available width rather than overflow it.
        let mut table = Table::new()
            .with_columns(vec![
                TableColumn::new("A"),
                TableColumn::new("B").with_max_width(4),
            ])
            .with_data(vec![vec!["x".into(), "y".into()]]);
        table.layout_mut().width = Width::Fixed(renderable::layout::TargetValue::universal(
            renderable::layout::Length::Percent(100.0),
        ));

        let plan = table.plan_widths(60).expect("plan");
        assert!(
            plan.content_widths()[1] <= 4,
            "capped last column must not exceed its max_width; widths={:?}",
            plan.content_widths()
        );
        assert!(
            plan.table_width <= 60,
            "fill never overflows the available width; table_width={}",
            plan.table_width
        );
    }

    #[test]
    fn test_table_with_title() {
        let table = Table::new()
            .with_title("Users")
            .with_columns(vec![TableColumn::new("Name")]);

        let result = table.render_optimistic(None);
        assert!(result.starts_with("Users\n"));
    }

    #[test]
    fn test_empty_table() {
        let table = Table::new();
        let result = table.render_optimistic(None);
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
        assert_eq!(format_float(3.15), "3.15");
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
        assert_eq!(TableCellContent::Float(3.15).to_string(), "3.15");
        assert_eq!(
            TableCellContent::Currency(Currency::USD, 9.99).to_string(),
            "$9.99"
        );
        assert_eq!(
            TableCellContent::Text("hello".to_string()).to_string(),
            "hello"
        );
        assert_eq!(
            TableCellContent::Text("a\tb".to_string()).to_string(),
            "a\tb"
        );
    }

    #[test]
    fn test_expand_tabs_uses_table_local_stops() {
        assert_eq!(expand_tabs_with_width("\t", 4), "    ");
        assert_eq!(expand_tabs_with_width("a\tb", 4), "a   b");
        assert_eq!(expand_tabs_with_width("abcd\tb", 4), "abcd    b");
        assert_eq!(expand_tabs_with_width("a\tb\n12\t3", 4), "a   b\n12  3");
        assert_eq!(
            expand_tabs_with_width("\x1b[31ma\x1b[0m\tb", 4),
            "\x1b[31ma\x1b[0m   b"
        );
    }

    fn assert_table_uses_tab_width(term: &Terminal, expected_tab_width: usize) {
        assert_eq!(term.tab_width, expected_tab_width);
        let table = Table::new()
            .with_columns(vec![TableColumn::new("Key\tValue")])
            .with_data(vec![vec!["1\t2\t3".into()]]);

        let plan = table
            .plan_widths_for_terminal(term)
            .expect("tabbed table width plan");
        let expected_width = ["Key\tValue", "1\t2\t3"]
            .into_iter()
            .map(|content| {
                visible_width(&expand_tabs_with_width(content, expected_tab_width)) as usize
            })
            .max()
            .expect("tabbed test content");
        assert_eq!(plan.content_widths(), vec![expected_width]);

        let expanded_data = expand_tabs_with_width("1\t2\t3", expected_tab_width);
        let output = table.render(term);
        assert!(!output.contains('\t'), "table output retained a raw tab: {output:?}");
        assert!(output.contains(expanded_data.as_ref()));
        let widths: Vec<u32> = output.lines().map(visible_width).collect();
        assert!(
            widths.windows(2).all(|pair| pair[0] == pair[1]),
            "table borders diverged after tab expansion: {widths:?}\n{output}",
        );
    }

    #[test]
    fn test_table_uses_detected_terminal_tab_width() {
        let mut term = Terminal::new();
        term.fixed_width = Some(80);
        let detected_tab_width = term.tab_width;
        assert!(detected_tab_width > 0);
        assert_table_uses_tab_width(&term, detected_tab_width);
    }

    #[test]
    fn test_table_uses_four_column_tab_override() {
        let term = Terminal::builder().width(80).tab_width(4).build();
        assert_table_uses_tab_width(&term, 4);
    }

    #[test]
    fn test_table_uses_eight_column_tab_override() {
        let term = Terminal::builder().width(80).tab_width(8).build();
        assert_table_uses_tab_width(&term, 8);
    }

    #[test]
    fn test_cursor_positioned_table_expands_tabs() {
        let table = Table::new()
            .with_columns(vec![TableColumn::new("Value")])
            .with_data(vec![vec!["1\t2\t3".into()]])
            .prefer_cursor_alignment();
        let mut term = Terminal::new_optimistic(80);
        term.is_tty = true;
        term.tab_width = 4;

        let output = table.render_bespoke(&term);
        assert!(!output.contains('\t'), "cursor-positioned output retained a raw tab");
        assert!(output.contains("1   2   3"));
    }

    #[test]
    fn test_from_i64() {
        let cell = TableCellContent::from(42i64);
        assert!(matches!(cell, TableCellContent::Integer(42)));
    }

    #[test]
    fn test_from_f64() {
        let cell = TableCellContent::from(3.15f64);
        assert!(matches!(cell, TableCellContent::Float(v) if (v - 3.15).abs() < f64::EPSILON));
    }

    // ── pad_cell tests ────────────────────────────────────────────

    #[test]
    fn test_pad_cell_left_alignment() {
        assert_eq!(pad_cell("hello", 10, Alignment::Left, None), "hello     ");
    }

    #[test]
    fn test_pad_cell_right_alignment() {
        assert_eq!(pad_cell("hello", 10, Alignment::Right, None), "     hello");
    }

    #[test]
    fn test_pad_cell_center_alignment() {
        assert_eq!(pad_cell("hi", 10, Alignment::Center, None), "    hi    ");
    }

    #[test]
    fn test_pad_cell_content_wider_than_width() {
        // Content wider than target: no crash, no truncation
        assert_eq!(
            pad_cell("hello world", 5, Alignment::Left, None),
            "hello world"
        );
    }

    #[test]
    fn test_pad_cell_with_ansi_colors() {
        let colored = "\x1b[31mred\x1b[0m"; // "red" in red (3 visible chars)
        let padded = pad_cell(colored, 10, Alignment::Left, None);
        assert_eq!(visible_width(&padded), 10);
        // Should have 7 trailing spaces
        assert!(padded.ends_with("       "));
        assert!(padded.starts_with("\x1b[31mred\x1b[0m"));
    }

    #[test]
    fn test_pad_cell_with_osc8_link() {
        let link = "\x1b]8;;https://example.com\x07click\x1b]8;;\x07"; // "click" = 5 visible
        let padded = pad_cell(link, 10, Alignment::Right, None);
        assert_eq!(visible_width(&padded), 10);
        // 5 spaces of left padding for right-align
        assert!(padded.starts_with("     "));
    }

    #[test]
    fn test_pad_cell_with_mixed_escape_and_osc8() {
        // Bold + OSC8 link: "\x1b[1m" + OSC8 link + "\x1b[0m"
        let mixed = "\x1b[1m\x1b]8;;https://rust-lang.org\x07Rust\x1b]8;;\x07\x1b[0m";
        // "Rust" = 4 visible chars
        let padded = pad_cell(mixed, 10, Alignment::Left, None);
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

        let result = table.render_content(None, None, None, &Terminal::default());
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

        let result = table.render_content(None, None, None, &Terminal::default());
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

        let result = table.render_content(None, None, None, &Terminal::default());
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

        let result = table.render_content(None, None, None, &Terminal::default());
        let lines: Vec<&str> = result.lines().collect();
        // Both values should start at the same position (aligned by max content width).
        // With max_width alignment, content is positioned consistently across rows.
        let data_line_1 = lines[3];
        let data_line_2 = lines[4];
        // Both values should be present and aligned at the same column position
        assert!(data_line_1.contains("$9.99"), "Should contain $9.99");
        assert!(
            data_line_2.contains("$1,234.56"),
            "Should contain $1,234.56"
        );
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

        let result = table.render_content(None, None, None, &Terminal::default());
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

        let result = table.render_content(None, None, None, &Terminal::default());
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

        let result = table.render_content(None, None, None, &Terminal::default());
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
        use crate::utils::layout::{Length, TargetValue};

        let mut table = Table::new()
            .with_columns(vec![TableColumn::new("X")])
            .with_data(vec![vec!["A".into()]]);
        table.layout_mut().margin.left = TargetValue::universal(Length::ch(1));

        let rendered = table.render_optimistic(Some(60));
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

    // The cursor-alignment unit tests below cover the **bespoke**
    // cursor-positioning path. The active `TerminalRenderable::render`
    // routes through the canonical render tree, which does not (yet) lower
    // the `prefer_cursor_alignment` terminal hint. These tests are pinned to
    // [`Table::render_bespoke`] so they keep documenting the bespoke
    // behavior precisely; cursor positioning at the tree-renderer level is
    // tracked separately as a render-tree feature request.

    #[test]
    fn test_cursor_alignment_uses_escape_codes() {
        let table = Table::new()
            .with_columns(vec![TableColumn::new("Name")])
            .with_data(vec![vec!["Alice".into()]])
            .prefer_cursor_alignment();

        let result = table.render_bespoke(&Terminal::new_optimistic(80));
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
        use crate::utils::layout::{Length, TargetValue};

        let mut table = Table::new()
            .with_columns(vec![TableColumn::new("X")])
            .with_data(vec![vec!["A".into()]])
            .prefer_cursor_alignment();
        table.layout_mut().margin.left = TargetValue::universal(Length::ch(5));

        let result = table.render_bespoke(&Terminal::new_optimistic(80));
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
        // Block alignment is observable because the default `Auto` table width
        // hugs content instead of filling all available columns.

        let result = table.render_bespoke(&Terminal::new_optimistic(80));
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
                    if let Some(end) = rest.find('G')
                        && let Ok(col) = rest[..end].parse::<u32>()
                    {
                        return col > 30;
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
        // Right alignment is observable because the default `Auto` table width
        // hugs content instead of filling all available columns.

        let result = table.render_bespoke(&Terminal::new_optimistic(80));
        // With 80 width and small table (~5 chars), table should start near column 75
        let has_right_position = result
            .lines()
            .next()
            .map(|line| {
                if let Some(start) = line.find("\x1b[") {
                    let rest = &line[start + 2..];
                    if let Some(end) = rest.find('G')
                        && let Ok(col) = rest[..end].parse::<u32>()
                    {
                        return col > 70;
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

        let result = table.render_optimistic(Some(80));
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

        let result = table.render_optimistic(Some(80));
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

        let result = table.render_optimistic(Some(80));
        assert!(result.contains("Users"), "Should contain title");
    }

    #[test]
    fn test_cursor_alignment_empty_table_returns_empty() {
        let table = Table::new().prefer_cursor_alignment();
        let result = table.render_optimistic(Some(80));
        assert!(result.is_empty(), "Empty table should render empty string");
    }

    #[test]
    fn test_cursor_alignment_row_fill() {
        use crate::utils::layout::{Length, TargetValue};

        let mut table = Table::new()
            .with_columns(vec![TableColumn::new("X")])
            .with_data(vec![vec!["A".into()]])
            .prefer_cursor_alignment();
        table.layout_mut().margin.left = TargetValue::universal(Length::ch(2));
        table.layout_mut().margin.right = TargetValue::universal(Length::ch(2));

        let result = table.render_optimistic(Some(20));
        // With row fill enabled, lines should extend to fill available width
        // Available width = 20 - 2 - 2 = 16
        for line in result.lines() {
            let width = visible_width(line);
            // Lines should have content (not be empty after stripping escapes)
            assert!(width > 0, "Lines should have visible content: {:?}", line);
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

        let result = table.render_optimistic(Some(80));
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

        let widths = table.calculate_column_widths(Some(80));
        assert_eq!(widths, vec![1, 1], "default table hugs content: {widths:?}");
    }

    #[test]
    fn test_calculate_column_widths_with_constraint_reduces_text_columns() {
        let table = Table::new()
            .with_columns(vec![
                TableColumn::new("Name"),
                TableColumn::new("Description"),
            ])
            .with_data(vec![vec!["Alice".into(), "A long description".into()]]);

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
            .with_columns(vec![TableColumn::new("Description").with_min_width(15)])
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

    #[test]
    fn test_min_width_with_short_content() {
        let table = Table::new()
            .with_columns(vec![TableColumn::new("Grade").with_min_width(8)])
            .with_data(vec![vec!["A".into()]]);

        let widths = table.calculate_column_widths(Some(100));
        assert!(
            widths[0] >= 8,
            "Column width {} should respect min_width 8, got {}",
            8,
            widths[0]
        );
    }

    #[test]
    fn test_min_width_with_max_width_conflict() {
        let table = Table::new()
            .with_columns(vec![
                TableColumn::new("Text")
                    .with_min_width(20)
                    .with_max_width(5),
            ])
            .with_data(vec![vec!["Hi".into()]]);

        let widths = table.calculate_column_widths(Some(100));
        // max_width < min_width is a config error, but min_width should still apply as floor
        assert!(
            widths[0] >= 20,
            "Column width {} should respect min_width 20 (not max_width 5), got {}",
            20,
            widths[0]
        );
    }

    #[test]
    fn test_multiple_columns_min_width_with_constraint() {
        let table = Table::new()
            .with_columns(vec![
                TableColumn::new("Name").with_min_width(15),
                TableColumn::new("Value").with_min_width(10),
            ])
            .with_data(vec![vec!["Short".into(), "12345678901234567890".into()]]);

        let widths = table.calculate_column_widths(Some(40));
        assert!(
            widths[0] >= 15,
            "Col1 width {} should respect min_width 15",
            widths[0]
        );
        assert!(
            widths[1] >= 10,
            "Col2 width {} should respect min_width 10",
            widths[1]
        );
    }

    #[test]
    fn test_min_width_columns_not_reduced_when_constraining() {
        // Scenario: Two columns, first has min_width=15, second is long content
        // Available width is tight - the second column should shrink, not the first
        let table = Table::new()
            .with_columns(vec![
                TableColumn::new("Label").with_min_width(15),
                TableColumn::new("Description"), // No min_width
            ])
            .with_data(vec![vec![
                "A".into(),
                "This is a very long description that should be wrapped".into(),
            ]]);

        // With tight constraint, first column should stay at min_width=15
        // and second column should absorb all the reduction
        let widths = table.calculate_column_widths(Some(30));

        // First column should be at least min_width (15)
        assert!(
            widths[0] >= 15,
            "Col1 (with min_width=15) should not be reduced below 15, got {}",
            widths[0]
        );

        // Total should fit within available (30 - border overhead)
        let total_with_borders: usize = widths.iter().sum::<usize>() + 4 + (widths.len() - 1) * 3;
        assert!(
            total_with_borders <= 30,
            "Total width {} should fit in available 30",
            total_with_borders
        );
    }

    #[test]
    fn test_measure_widths_uses_widest_explicit_header_line() {
        let table = Table::new().with_columns(vec![TableColumn::new("Tool\nCalls")]);

        let measurements = table.measure_widths(80).unwrap();
        let column = &measurements.columns[0];

        assert_eq!(column.header_line_width, 5);
        assert_eq!(column.columnar_width_requirement, 5);
    }

    #[test]
    fn test_measure_widths_marks_word_wrap_none_columns_as_non_wrapping() {
        let table = Table::new()
            .with_columns(vec![
                TableColumn::new("Name"),
                TableColumn::new("Path").with_word_wrap(WordWrap::None),
            ])
            .with_data(vec![vec![
                "app".into(),
                "/very/long/path/that/should/not/shrink".into(),
            ]]);

        let measurements = table.measure_widths(30).unwrap();
        let path = measurements
            .columns
            .iter()
            .find(|column| column.original_index == 1)
            .unwrap();

        assert!(path.is_non_wrapping);
        assert!(!path.is_shrinkable);
    }

    #[test]
    fn test_plan_widths_drops_rightmost_eligible_column_and_renders_note() {
        let table = Table::new()
            .with_columns(vec![
                TableColumn::new("Name").with_fixed_width(8),
                TableColumn::new("ID").with_fixed_width(4),
                TableColumn::new("Notes")
                    .with_word_wrap(WordWrap::None)
                    .drop_when_space_is_limited(Some("Notes hidden on narrow terminals")),
            ])
            .with_data(vec![vec![
                "Widget".into(),
                "42".into(),
                "This column is intentionally too wide".into(),
            ]]);

        let plan = table.plan_widths(28).unwrap();
        assert_eq!(plan.visible_column_indices, vec![0, 1]);
        assert_eq!(plan.dropped_column_indices, vec![2]);
        assert_eq!(plan.dropped_notes, vec!["Notes hidden on narrow terminals"]);

        let rendered = table.render_content(Some(28), None, None, &Terminal::default());
        assert!(rendered.contains("- Notes hidden on narrow terminals"));
    }

    #[test]
    fn test_plan_widths_truncate_columns_have_small_natural_break_width() {
        let table = Table::new()
            .with_columns(vec![
                TableColumn::new("Notes")
                    .with_min_width(3)
                    .with_word_wrap(WordWrap::Truncate(Some("...".into()))),
            ])
            .with_data(vec![vec!["This is a much longer note".into()]]);

        let plan = table.plan_widths(20).unwrap();
        assert_eq!(plan.columns[0].natural_break_width, 3);
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
        let columns = vec![TableColumn::new("Name"), TableColumn::new("Age")];
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
        let data = vec![vec![TableCellContent::Text(
            "Line1\nLine2\nLine3".to_string(),
        )]];
        let widths = vec![20];

        let heights = calculate_row_heights(&data, &columns, &widths);
        assert_eq!(heights, vec![3]);
    }

    #[test]
    fn test_calculate_row_heights_max_across_cells() {
        let columns = vec![TableColumn::new("Short"), TableColumn::new("Long")];
        let data = vec![vec![
            "A".into(),
            TableCellContent::Text("Line1\nLine2".to_string()),
        ]];
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

        let result = table.render_content(None, None, None, &Terminal::default());
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

        let result = table.render_content(None, None, None, &Terminal::default());
        let lines: Vec<&str> = result.lines().collect();

        // Should have: top border, 2 header lines, separator, 1 data row, bottom border = 6 lines
        assert_eq!(
            lines.len(),
            6,
            "Expected 6 lines for 2-line header:\n{}",
            result
        );

        // First header line should contain "Name" and "GitHub"
        assert!(
            lines[1].contains("Name"),
            "First header line should have 'Name'"
        );
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

        let result = table.render_content(None, None, None, &Terminal::default());
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
            .with_columns(vec![TableColumn::new("Single"), TableColumn::new("Multi")])
            .with_data(vec![vec![
                "A".into(),
                TableCellContent::Text("B\nC".to_string()),
            ]]);

        let result = table.render_content(None, None, None, &Terminal::default());
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

        let result = table.render_content(None, None, None, &Terminal::default());
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

        let result = table.render_content(None, None, None, &Terminal::default());
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

        let result = table.render_content(None, None, None, &Terminal::default());
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

        let result = table.render_content(None, None, None, &Terminal::default());
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

        let result = table.render_optimistic(Some(80));
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
            .with_columns(vec![TableColumn::new("Short"), TableColumn::new("Long")])
            .with_data(vec![vec![
                "A".into(),
                TableCellContent::Text("B\nC\nD".to_string()),
            ]])
            .prefer_cursor_alignment();

        let result = table.render_optimistic(Some(80));

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

        // Bespoke path: cursor positioning is not yet lowered by the
        // canonical render tree, so the test pins the legacy bespoke output.
        let result = table.render_bespoke(&Terminal::new_optimistic(80));

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

        // Bespoke path: see note above for cursor-alignment unit tests.
        let result = table.render_bespoke(&Terminal::new_optimistic(80));

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
            .with_columns(vec![TableColumn::new("Empty"), TableColumn::new("Multi")])
            .with_data(vec![vec![
                "".into(),
                TableCellContent::Text("A\nB\nC".to_string()),
            ]]);

        let result = table.render_content(None, None, None, &Terminal::default());
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

        let result = table.render_content(None, None, None, &Terminal::default());

        // ANSI codes should be preserved
        assert!(
            result.contains("\x1b[31m"),
            "Red escape should be preserved"
        );
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

        let result = table.render_content(None, None, None, &Terminal::default());
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

        let result = table.render_content(None, None, None, &Terminal::default());

        // Number should NOT be wrapped even if exceeds width
        // (word wrap is forced to None for numeric columns)
        assert!(
            result.contains("1,000,000"),
            "Number should not be wrapped or truncated"
        );
    }

    #[test]
    fn test_table_respects_available_width_cursor_mode() {
        use crate::utils::layout::{Length, TargetValue};

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

        table.layout_mut().margin.left = TargetValue::universal(Length::ch(5));
        table.layout_mut().margin.right = TargetValue::universal(Length::ch(5));

        // Bespoke path: see note above for cursor-alignment unit tests.
        let result = table.render_bespoke(&Terminal::new_optimistic(60));

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
        let result = table.render_optimistic(Some(50));
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
        let result = table.render_optimistic(Some(15));
        assert!(
            !result.is_empty(),
            "Should render even with narrow terminal"
        );
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

        let result = table.render_content(None, None, None, &Terminal::default());
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

        let result = table.render_content(None, None, None, &Terminal::default());

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

        let result = table.render_content(None, None, None, &Terminal::default());
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

        let result = table.render_content(None, None, None, &Terminal::default());

        // Content should wrap at column width
        let lines: Vec<&str> = result.lines().collect();
        assert!(
            lines.len() > 4,
            "Long content should cause multi-line row: {} lines",
            lines.len()
        );
    }

    // ── Alternate background color tests ─────────────────────────────

    #[test]
    fn test_alternate_background_color_builder() {
        let table = Table::new()
            .with_columns(vec![TableColumn::new("X")])
            .alternate_background_color();
        assert!(table.style().striped_rows);
    }

    #[test]
    fn test_alternate_background_color_default_is_false() {
        let table = Table::new();
        assert!(!table.style().striped_rows);
    }

    #[test]
    fn test_stripe_bg_escape_truecolor_modes() {
        assert_eq!(
            stripe_bg_escape(None, &ColorMode::Dark, ColorDepth::TrueColor),
            Some("\x1b[48;2;36;40;59m".to_string())
        );
        assert_eq!(
            stripe_bg_escape(None, &ColorMode::Light, ColorDepth::TrueColor),
            Some("\x1b[48;2;226;229;236m".to_string())
        );
    }

    #[test]
    fn test_stripe_bg_escape_unknown_uses_dark() {
        assert_eq!(
            stripe_bg_escape(None, &ColorMode::Unknown, ColorDepth::TrueColor),
            stripe_bg_escape(None, &ColorMode::Dark, ColorDepth::TrueColor),
        );
    }

    #[test]
    fn test_stripe_bg_escape_degrades_across_color_depths() {
        // 256-color: the color cube; 16-color: the basic-palette fallback;
        // no color support: the stripe is dropped entirely.
        let enhanced = stripe_bg_escape(None, &ColorMode::Dark, ColorDepth::Enhanced).unwrap();
        assert!(enhanced.contains("\x1b[48;5;"), "got {enhanced:?}");

        let basic = stripe_bg_escape(None, &ColorMode::Dark, ColorDepth::Basic).unwrap();
        assert!(
            !basic.contains("\x1b[48;2;") && !basic.contains("\x1b[48;5;"),
            "16-color terminal should degrade to a basic escape: {basic:?}"
        );

        assert_eq!(
            stripe_bg_escape(None, &ColorMode::Dark, ColorDepth::None),
            None,
            "a terminal with no color support emits no stripe"
        );
    }

    #[test]
    fn test_stripe_bg_escape_explicit_color_overrides_default() {
        let esc = stripe_bg_escape(
            Some(Color::BasicColor(BasicColor::Blue)),
            &ColorMode::Dark,
            ColorDepth::TrueColor,
        );
        // Basic blue as a background lowers to SGR code 44.
        assert_eq!(esc, Some("\x1b[44m".to_string()));
    }

    #[test]
    fn striped_table_degrades_below_truecolor() {
        let table = Table::new()
            .with_columns(vec![TableColumn::new("X")])
            .with_data(vec![vec!["A".into()], vec!["B".into()]])
            .alternate_background_color();

        let mut term = Terminal::new_optimistic(40);

        // 16-color terminal: striping still renders, degraded to the basic
        // background fallback (code 40) rather than being silently disabled.
        term.color_depth = ColorDepth::Basic;
        let basic = table.render(&term);
        assert!(
            basic.contains("\x1b[40m"),
            "striping must still render (degraded) on a 16-color terminal: {basic:?}"
        );
        assert!(
            !basic.contains("\x1b[48;2;"),
            "no truecolor escape on a 16-color terminal: {basic:?}"
        );

        // 256-color terminal: degrades to the color cube.
        term.color_depth = ColorDepth::Enhanced;
        assert!(
            table.render(&term).contains("\x1b[48;5;"),
            "striping should degrade to the 256-color cube"
        );

        // No color support: the stripe is dropped.
        term.color_depth = ColorDepth::None;
        let none = table.render(&term);
        assert!(
            !none.contains("\x1b[40m") && !none.contains("\x1b[48"),
            "no stripe escape on a terminal without color support: {none:?}"
        );
    }

    #[test]
    fn tree_rendered_stripe_covers_wrapped_row_lines() {
        let table = Table::new()
            .with_columns(vec![
                TableColumn::new("Name").with_max_width(8),
                TableColumn::new("Notes")
                    .with_max_width(18)
                    .with_word_wrap(WordWrap::WrapProse(None, None)),
            ])
            .with_data(vec![
                vec!["row0".into(), "short".into()],
                vec![
                    "row1".into(),
                    "deliberately long text that wraps".into(),
                ],
            ])
            .alternate_background_color();

        let mut term = Terminal::new_optimistic(48);
        term.color_depth = ColorDepth::TrueColor;
        term.color_mode = ColorMode::Dark;

        let rendered = table.render(&term);
        let bg = stripe_bg_escape(None, &ColorMode::Dark, ColorDepth::TrueColor).unwrap();
        let striped_lines: Vec<&str> = rendered
            .lines()
            .filter(|line| {
                line.contains("row1")
                    || line.contains("deliberately")
                    || line.contains("long text")
                    || line.contains("wraps")
            })
            .collect();

        assert!(
            striped_lines.len() >= 2,
            "test fixture should produce a wrapped striped row: {rendered:?}"
        );
        for line in striped_lines {
            assert!(
                line.contains(&bg),
                "wrapped striped row line should carry the stripe background: {line:?}"
            );
        }
    }

    #[test]
    fn test_render_content_no_stripe_without_flag() {
        let table = Table::new()
            .with_columns(vec![TableColumn::new("X")])
            .with_data(vec![vec!["A".into()], vec!["B".into()]]);

        let result = table.render_content(None, None, None, &Terminal::default());
        // No background escape codes should be present
        assert!(
            !result.contains("\x1b[48;2;"),
            "Should not contain background color escapes without stripe flag"
        );
    }

    #[test]
    fn test_render_content_stripe_on_even_rows() {
        let table = Table::new()
            .with_columns(vec![TableColumn::new("X")])
            .with_data(vec![
                vec!["Row0".into()],
                vec!["Row1".into()],
                vec!["Row2".into()],
                vec!["Row3".into()],
            ]);

        let result = table.render_content(
            None,
            Some(bg_escape(&ColorMode::Dark)),
            None,
            &Terminal::default(),
        );
        let lines: Vec<&str> = result.lines().collect();

        // Data rows start at line 3 (border=0, header=1, separator=2)
        // Row 0 (line 3): no stripe
        // Row 1 (line 4): stripe (odd index = even row in 0-based)
        // Row 2 (line 5): no stripe
        // Row 3 (line 6): stripe

        let bg_dark = bg_escape(&ColorMode::Dark);
        assert!(!lines[3].contains(bg_dark), "Row 0 should not be striped");
        assert!(
            lines[4].contains(bg_dark),
            "Row 1 should be striped: {:?}",
            lines[4]
        );
        assert!(!lines[5].contains(bg_dark), "Row 2 should not be striped");
        assert!(
            lines[6].contains(bg_dark),
            "Row 3 should be striped: {:?}",
            lines[6]
        );
    }

    #[test]
    fn test_render_content_stripe_resets_background() {
        let table = Table::new()
            .with_columns(vec![TableColumn::new("X")])
            .with_data(vec![vec!["A".into()], vec!["B".into()]]);

        let result = table.render_content(
            None,
            Some(bg_escape(&ColorMode::Dark)),
            None,
            &Terminal::default(),
        );
        // Striped rows should have a background reset at the end
        let striped_lines: Vec<&str> = result
            .lines()
            .filter(|l| l.contains("\x1b[48;2;"))
            .collect();

        assert!(!striped_lines.is_empty(), "Should have striped lines");
        for line in &striped_lines {
            assert!(
                line.contains(BG_RESET),
                "Striped line should end with background reset: {:?}",
                line
            );
        }
    }

    #[test]
    fn test_render_content_stripe_light_mode_uses_light_color() {
        let table = Table::new()
            .with_columns(vec![TableColumn::new("X")])
            .with_data(vec![vec!["A".into()], vec!["B".into()]]);

        let result = table.render_content(
            None,
            Some(bg_escape(&ColorMode::Light)),
            None,
            &Terminal::default(),
        );
        let bg_light = bg_escape(&ColorMode::Light);
        assert!(
            result.contains(bg_light),
            "Light mode should use light stripe color"
        );
    }

    #[test]
    fn test_render_content_stripe_single_row_no_stripe() {
        let table = Table::new()
            .with_columns(vec![TableColumn::new("X")])
            .with_data(vec![vec!["Only".into()]]);

        let result = table.render_content(
            None,
            Some(bg_escape(&ColorMode::Dark)),
            None,
            &Terminal::default(),
        );
        // Single row at index 0 should not be striped
        assert!(
            !result.contains("\x1b[48;2;"),
            "Single row should not be striped (row 0 is unstriped)"
        );
    }

    #[test]
    fn test_cursor_alignment_with_stripe() {
        let table = Table::new()
            .with_columns(vec![TableColumn::new("X")])
            .with_data(vec![vec!["A".into()], vec!["B".into()]])
            .prefer_cursor_alignment();

        let result = table.render_with_cursor_positioning(
            80,
            Some(bg_escape(&ColorMode::Dark)),
            None,
            &Terminal::default(),
        );
        let bg_dark = bg_escape(&ColorMode::Dark);
        // Row 1 (index 1) should be striped
        assert!(
            result.contains(bg_dark),
            "Cursor-positioned table should support striping"
        );
        assert!(
            result.contains(BG_RESET),
            "Striped rows should reset background"
        );
    }

    #[test]
    fn test_stripe_outer_borders_uncolored_space_padded() {
        let table = Table::new()
            .with_columns(vec![TableColumn::new("X")])
            .with_data(vec![vec!["A".into()], vec!["B".into()]]);

        let result = table.render_content(
            None,
            Some(bg_escape(&ColorMode::Dark)),
            None,
            &Terminal::default(),
        );
        let bg = bg_escape(&ColorMode::Dark);

        // Find the striped line (row index 1 = second data row)
        let striped_line = result
            .lines()
            .find(|l| l.contains(bg))
            .expect("Should have a striped line");

        // Left border: line should start with │ THEN the bg escape
        assert!(
            striped_line.starts_with(&format!("│{}", bg)),
            "Left │ should precede stripe bg: {:?}",
            striped_line
        );

        // Right border: line should end with bg reset THEN │
        assert!(
            striped_line.ends_with(&format!("{}│", BG_RESET)),
            "Right │ should follow bg reset: {:?}",
            striped_line
        );
    }

    #[test]
    fn test_stripe_outer_borders_uncolored_cursor_positioned() {
        let table = Table::new()
            .with_columns(vec![TableColumn::new("X")])
            .with_data(vec![vec!["A".into()], vec!["B".into()]])
            .prefer_cursor_alignment();

        let result = table.render_with_cursor_positioning(
            80,
            Some(bg_escape(&ColorMode::Dark)),
            None,
            &Terminal::default(),
        );
        let bg = bg_escape(&ColorMode::Dark);

        // Find the striped line
        let striped_line = result
            .lines()
            .find(|l| l.contains(bg))
            .expect("Should have a striped line");

        // Left border: the line has cursor positioning then │ then bg
        // Pattern: \x1b[nG│<bg_escape>
        assert!(
            striped_line.contains(&format!("│{}", bg)),
            "Left │ should precede stripe bg: {:?}",
            striped_line
        );

        // Right border: bg reset then │ (no bg escape between reset and │)
        assert!(
            striped_line.contains(&format!("{}│", BG_RESET)),
            "Right │ should follow bg reset: {:?}",
            striped_line
        );
    }

    #[test]
    fn test_stripe_survives_sgr_reset_in_cell_space_padded() {
        // Cell content with \x1b[0m (full SGR reset) must not break the stripe
        // for padding spaces and subsequent separators.
        let table = Table::new()
            .with_columns(vec![
                TableColumn::new("A"),
                TableColumn::new("B").with_min_width(10),
            ])
            .with_data(vec![
                vec!["row0".into(), "plain".into()],
                vec![
                    "row1".into(),
                    TableCellContent::Text("\x1b[2mdim\x1b[0m".to_string()),
                ],
            ]);

        let bg = bg_escape(&ColorMode::Dark);
        let result = table.render_content(
            None,
            Some(bg_escape(&ColorMode::Dark)),
            None,
            &Terminal::default(),
        );

        // Find the striped line (row 1)
        let striped_line = result
            .lines()
            .find(|l| l.contains("row1"))
            .expect("Should have row1 line");

        // The bg must be re-applied after the cell content's \x1b[0m,
        // so the trailing padding spaces keep the stripe.  Verify the
        // bg escape appears AFTER the \x1b[0m.
        let after_reset = striped_line
            .rfind("\x1b[0m")
            .expect("Should contain SGR reset");
        let bg_restore = striped_line[after_reset..]
            .find(bg)
            .expect("Stripe bg should be re-applied after SGR reset in cell");
        assert!(bg_restore > 0, "Stripe bg must follow the SGR reset");
    }

    #[test]
    fn test_stripe_survives_sgr_reset_in_cell_cursor_positioned() {
        let table = Table::new()
            .with_columns(vec![
                TableColumn::new("A"),
                TableColumn::new("B").with_min_width(10),
            ])
            .with_data(vec![
                vec!["row0".into(), "plain".into()],
                vec![
                    "row1".into(),
                    TableCellContent::Text("\x1b[2mdim\x1b[0m".to_string()),
                ],
            ])
            .prefer_cursor_alignment();

        let bg = bg_escape(&ColorMode::Dark);
        let result = table.render_with_cursor_positioning(
            80,
            Some(bg_escape(&ColorMode::Dark)),
            None,
            &Terminal::default(),
        );

        // Find the striped data line containing "row1"
        let striped_line = result
            .lines()
            .find(|l| l.contains("row1"))
            .expect("Should have row1 line");

        // Stripe bg must be re-applied after the cell content's \x1b[0m
        let after_reset = striped_line
            .rfind("\x1b[0m")
            .expect("Should contain SGR reset");
        let bg_restore = striped_line[after_reset..]
            .find(bg)
            .expect("Stripe bg should be re-applied after SGR reset in cell");
        assert!(bg_restore > 0, "Stripe bg must follow the SGR reset");
    }

    #[test]
    fn test_stripe_survives_bg_reset_mid_content_space_padded() {
        // When Prose content like <bg-red>A</bg-red> emits \x1b[49m (bg-only
        // reset), the stripe bg must be re-applied so that text between styled
        // spans keeps the stripe.
        let content = "\x1b[41mERR\x1b[49m, \x1b[41mWARN\x1b[49m";
        let table = Table::new()
            .with_columns(vec![
                TableColumn::new("A"),
                TableColumn::new("B").with_min_width(20),
            ])
            .with_data(vec![
                vec!["row0".into(), "plain".into()],
                vec!["row1".into(), TableCellContent::Text(content.to_string())],
            ]);

        let bg = bg_escape(&ColorMode::Dark);
        let result = table.render_content(
            None,
            Some(bg_escape(&ColorMode::Dark)),
            None,
            &Terminal::default(),
        );

        let striped_line = result
            .lines()
            .find(|l| l.contains("row1"))
            .expect("Should have row1 line");

        // Count how many times the stripe bg appears after a \x1b[49m.
        // There are two \x1b[49m resets in the content; the stripe bg must
        // follow each one.
        let bg_after_49m: Vec<_> = striped_line
            .match_indices("\x1b[49m")
            .filter_map(|(pos, _)| {
                let after = pos + "\x1b[49m".len();
                striped_line
                    .get(after..)
                    .and_then(|s| if s.starts_with(bg) { Some(pos) } else { None })
            })
            .collect();
        assert!(
            bg_after_49m.len() >= 2,
            "Stripe bg must be restored after each \\x1b[49m; found {} restorations in: {:?}",
            bg_after_49m.len(),
            striped_line
        );
    }

    #[test]
    fn test_stripe_survives_bg_reset_mid_content_cursor_positioned() {
        let content = "\x1b[41mERR\x1b[49m, \x1b[41mWARN\x1b[49m";
        let table = Table::new()
            .with_columns(vec![
                TableColumn::new("A"),
                TableColumn::new("B").with_min_width(20),
            ])
            .with_data(vec![
                vec!["row0".into(), "plain".into()],
                vec!["row1".into(), TableCellContent::Text(content.to_string())],
            ])
            .prefer_cursor_alignment();

        let bg = bg_escape(&ColorMode::Dark);
        let result = table.render_with_cursor_positioning(
            80,
            Some(bg_escape(&ColorMode::Dark)),
            None,
            &Terminal::default(),
        );

        let striped_line = result
            .lines()
            .find(|l| l.contains("row1"))
            .expect("Should have row1 line");

        let bg_after_49m: Vec<_> = striped_line
            .match_indices("\x1b[49m")
            .filter_map(|(pos, _)| {
                let after = pos + "\x1b[49m".len();
                striped_line
                    .get(after..)
                    .and_then(|s| if s.starts_with(bg) { Some(pos) } else { None })
            })
            .collect();
        assert!(
            bg_after_49m.len() >= 2,
            "Stripe bg must be restored after each \\x1b[49m; found {} restorations in: {:?}",
            bg_after_49m.len(),
            striped_line
        );
    }

    // ── Alternate text color tests ──────────────────────────────────

    #[test]
    fn test_alternate_text_color_builder() {
        let table = Table::new()
            .with_columns(vec![TableColumn::new("X")])
            .alternate_text_color();
        assert!(table.style().striped_text);
    }

    #[test]
    fn test_alternate_text_color_default_is_false() {
        let table = Table::new();
        assert!(!table.style().striped_text);
    }

    #[test]
    fn test_stripe_fg_escape_truecolor_modes() {
        assert_eq!(
            stripe_fg_escape(None, &ColorMode::Dark, ColorDepth::TrueColor),
            Some("\x1b[38;2;180;180;190m".to_string())
        );
        assert_eq!(
            stripe_fg_escape(None, &ColorMode::Light, ColorDepth::TrueColor),
            Some("\x1b[38;2;80;80;90m".to_string())
        );
    }

    #[test]
    fn test_stripe_fg_escape_unknown_uses_dark() {
        assert_eq!(
            stripe_fg_escape(None, &ColorMode::Unknown, ColorDepth::TrueColor),
            stripe_fg_escape(None, &ColorMode::Dark, ColorDepth::TrueColor),
        );
    }

    #[test]
    fn test_stripe_fg_escape_degrades_across_color_depths() {
        let enhanced = stripe_fg_escape(None, &ColorMode::Dark, ColorDepth::Enhanced).unwrap();
        assert!(enhanced.contains("\x1b[38;5;"), "got {enhanced:?}");

        let basic = stripe_fg_escape(None, &ColorMode::Dark, ColorDepth::Basic).unwrap();
        assert!(
            !basic.contains("\x1b[38;2;") && !basic.contains("\x1b[38;5;"),
            "16-color terminal should degrade to a basic escape: {basic:?}"
        );

        assert_eq!(
            stripe_fg_escape(None, &ColorMode::Dark, ColorDepth::None),
            None
        );
    }

    #[test]
    fn test_stripe_fg_escape_explicit_color_overrides_default() {
        let esc = stripe_fg_escape(
            Some(Color::BasicColor(BasicColor::Blue)),
            &ColorMode::Dark,
            ColorDepth::TrueColor,
        );
        // Basic blue as a foreground lowers to SGR code 34.
        assert_eq!(esc, Some("\x1b[34m".to_string()));
    }

    #[test]
    fn test_render_content_no_text_tint_without_flag() {
        let table = Table::new()
            .with_columns(vec![TableColumn::new("X")])
            .with_data(vec![vec!["A".into()], vec!["B".into()]]);

        let result = table.render_content(None, None, None, &Terminal::default());
        assert!(
            !result.contains("\x1b[38;2;"),
            "Should not contain foreground color escapes without text color flag"
        );
    }

    #[test]
    fn test_render_content_text_tint_on_even_rows() {
        let table = Table::new()
            .with_columns(vec![TableColumn::new("X")])
            .with_data(vec![
                vec!["Row0".into()],
                vec!["Row1".into()],
                vec!["Row2".into()],
                vec!["Row3".into()],
            ]);

        let result = table.render_content(
            None,
            None,
            Some(fg_escape(&ColorMode::Dark)),
            &Terminal::default(),
        );
        let lines: Vec<&str> = result.lines().collect();

        let fg_dark = fg_escape(&ColorMode::Dark);
        // Data rows start at line 3
        assert!(
            !lines[3].contains(fg_dark),
            "Row 0 should not have text tint"
        );
        assert!(
            lines[4].contains(fg_dark),
            "Row 1 should have text tint: {:?}",
            lines[4]
        );
        assert!(
            !lines[5].contains(fg_dark),
            "Row 2 should not have text tint"
        );
        assert!(
            lines[6].contains(fg_dark),
            "Row 3 should have text tint: {:?}",
            lines[6]
        );
    }

    #[test]
    fn test_render_content_text_tint_resets_foreground() {
        let table = Table::new()
            .with_columns(vec![TableColumn::new("X")])
            .with_data(vec![vec!["A".into()], vec!["B".into()]]);

        let result = table.render_content(
            None,
            None,
            Some(fg_escape(&ColorMode::Dark)),
            &Terminal::default(),
        );
        let tinted_lines: Vec<&str> = result
            .lines()
            .filter(|l| l.contains("\x1b[38;2;"))
            .collect();

        assert!(!tinted_lines.is_empty(), "Should have tinted lines");
        for line in &tinted_lines {
            assert!(
                line.contains(FG_RESET),
                "Tinted line should contain foreground reset: {:?}",
                line
            );
        }
    }

    #[test]
    fn test_render_content_text_tint_light_mode_uses_light_color() {
        let table = Table::new()
            .with_columns(vec![TableColumn::new("X")])
            .with_data(vec![vec!["A".into()], vec!["B".into()]]);

        let result = table.render_content(
            None,
            None,
            Some(fg_escape(&ColorMode::Light)),
            &Terminal::default(),
        );
        let fg_light = fg_escape(&ColorMode::Light);
        assert!(
            result.contains(fg_light),
            "Light mode should use light text tint color"
        );
    }

    #[test]
    fn test_render_content_text_tint_single_row_no_tint() {
        let table = Table::new()
            .with_columns(vec![TableColumn::new("X")])
            .with_data(vec![vec!["Only".into()]]);

        let result = table.render_content(
            None,
            None,
            Some(fg_escape(&ColorMode::Dark)),
            &Terminal::default(),
        );
        assert!(
            !result.contains("\x1b[38;2;"),
            "Single row should not have text tint (row 0 is untinted)"
        );
    }

    #[test]
    fn test_text_tint_outer_borders_uncolored_space_padded() {
        let table = Table::new()
            .with_columns(vec![TableColumn::new("X")])
            .with_data(vec![vec!["A".into()], vec!["B".into()]]);

        let result = table.render_content(
            None,
            None,
            Some(fg_escape(&ColorMode::Dark)),
            &Terminal::default(),
        );
        let fg = fg_escape(&ColorMode::Dark);

        let tinted_line = result
            .lines()
            .find(|l| l.contains(fg))
            .expect("Should have a tinted line");

        // Left border: line should start with │ THEN the fg escape
        assert!(
            tinted_line.starts_with(&format!("│{}", fg)),
            "Left │ should precede text tint: {:?}",
            tinted_line
        );

        // Right border: line should end with fg reset THEN │
        assert!(
            tinted_line.ends_with(&format!("{}│", FG_RESET)),
            "Right │ should follow fg reset: {:?}",
            tinted_line
        );
    }

    #[test]
    fn test_cursor_alignment_with_text_tint() {
        let table = Table::new()
            .with_columns(vec![TableColumn::new("X")])
            .with_data(vec![vec!["A".into()], vec!["B".into()]])
            .prefer_cursor_alignment();

        let result = table.render_with_cursor_positioning(
            80,
            None,
            Some(fg_escape(&ColorMode::Dark)),
            &Terminal::default(),
        );
        let fg_dark = fg_escape(&ColorMode::Dark);
        assert!(
            result.contains(fg_dark),
            "Cursor-positioned table should support text tinting"
        );
        assert!(
            result.contains(FG_RESET),
            "Tinted rows should reset foreground"
        );
    }

    #[test]
    fn test_combined_bg_and_fg_stripe() {
        let table = Table::new()
            .with_columns(vec![TableColumn::new("X")])
            .with_data(vec![vec!["A".into()], vec!["B".into()]]);

        let result = table.render_content(
            None,
            Some(bg_escape(&ColorMode::Dark)),
            Some(fg_escape(&ColorMode::Dark)),
            &Terminal::default(),
        );
        let bg = bg_escape(&ColorMode::Dark);
        let fg = fg_escape(&ColorMode::Dark);

        // Striped row should contain both bg and fg escapes
        let tinted_line = result
            .lines()
            .find(|l| l.contains(bg) && l.contains(fg))
            .expect("Should have a line with both bg and fg stripe");

        // Both resets should be present
        assert!(tinted_line.contains(BG_RESET), "Should reset background");
        assert!(tinted_line.contains(FG_RESET), "Should reset foreground");
    }

    #[test]
    fn test_text_tint_survives_sgr_reset_in_cell() {
        let table = Table::new()
            .with_columns(vec![
                TableColumn::new("A"),
                TableColumn::new("B").with_min_width(10),
            ])
            .with_data(vec![
                vec!["row0".into(), "plain".into()],
                vec![
                    "row1".into(),
                    TableCellContent::Text("\x1b[2mdim\x1b[0m".to_string()),
                ],
            ]);

        let fg = fg_escape(&ColorMode::Dark);
        let result = table.render_content(
            None,
            None,
            Some(fg_escape(&ColorMode::Dark)),
            &Terminal::default(),
        );

        let tinted_line = result
            .lines()
            .find(|l| l.contains("row1"))
            .expect("Should have row1 line");

        // The fg must be re-applied after the cell content's \x1b[0m
        let after_reset = tinted_line
            .rfind("\x1b[0m")
            .expect("Should contain SGR reset");
        let fg_restore = tinted_line[after_reset..]
            .find(fg)
            .expect("Text tint should be re-applied after SGR reset in cell");
        assert!(fg_restore > 0, "Text tint must follow the SGR reset");
    }

    #[test]
    fn test_cursor_positioned_padding_no_overshoot() {
        // Regression test: padding lines (from apply_vertical_padding) must not
        // extend past the cell boundary when alignment offset is non-zero.
        // Previously, " ".repeat(cell_width) was output at content_col (which
        // includes alignment offset), overshooting sep_col and making the
        // terminal line wider than the table — causing visual line wraps.
        let table = Table::new()
            .with_columns(vec![
                TableColumn::new("Repo"),
                TableColumn::new("Author").with_alignment(Alignment::Center),
                TableColumn::new("DL").with_alignment(Alignment::Right),
                TableColumn::new("Likes").with_alignment(Alignment::Right),
            ])
            .prefer_cursor_alignment()
            .with_data(vec![
                // This row wraps in Author col, creating padding lines in
                // Repo, DL, and Likes columns.
                vec![
                    "short-repo".into(),
                    "lmstudio-community".into(),
                    "13.1K".into(),
                    "56".into(),
                ],
                vec!["other".into(), "short".into(), "1K".into(), "10".into()],
            ]);

        // Render at a width that forces Author wrapping
        let output = table.render_with_cursor_positioning(50, None, None, &Terminal::default());
        let border_width = output.lines().next().map(visible_width).unwrap_or(0);

        for (i, line) in output.lines().enumerate() {
            let vw = visible_width(line);
            assert!(
                vw <= border_width,
                "Line {} visible width ({}) exceeds border width ({}): {:?}",
                i,
                vw,
                border_width,
                line
            );
        }
    }

    // ── Conditional column visibility tests ──────────────────────────

    #[test]
    fn test_conditional_is_satisfied_always() {
        assert!(Conditional::Always.is_satisfied(0));
        assert!(Conditional::Always.is_satisfied(200));
    }

    #[test]
    fn test_conditional_is_satisfied_width_greater_than() {
        let cond = Conditional::WidthGreaterThan(80);
        assert!(!cond.is_satisfied(60));
        assert!(!cond.is_satisfied(80));
        assert!(cond.is_satisfied(81));
    }

    #[test]
    fn test_conditional_is_satisfied_less_than_or_equal() {
        let cond = Conditional::LessThanOrEqual(40);
        assert!(cond.is_satisfied(30));
        assert!(cond.is_satisfied(40));
        assert!(!cond.is_satisfied(41));
    }

    #[test]
    fn test_conditional_default_is_always() {
        assert_eq!(Conditional::default(), Conditional::Always);
    }

    #[test]
    fn test_table_column_when_default_is_always() {
        let col = TableColumn::new("Name");
        assert_eq!(col.when, Conditional::Always);
    }

    #[test]
    fn test_table_column_with_when_builder() {
        let col = TableColumn::new("Details").with_when(Conditional::WidthGreaterThan(60));
        assert_eq!(col.when, Conditional::WidthGreaterThan(60));
    }

    #[test]
    fn test_with_visible_columns_none_when_all_visible() {
        let table = Table::new()
            .with_columns(vec![TableColumn::new("A"), TableColumn::new("B")])
            .with_data(vec![vec!["x".into(), "y".into()]]);

        assert!(
            table.with_visible_columns(80).is_none(),
            "Should return None when all columns are Always-visible"
        );
    }

    #[test]
    fn test_with_visible_columns_filters_hidden_column() {
        let table = Table::new()
            .with_columns(vec![
                TableColumn::new("Name"),
                TableColumn::new("Details").with_when(Conditional::WidthGreaterThan(80)),
            ])
            .with_data(vec![vec!["Alice".into(), "Some details".into()]]);

        // At width 60, Details should be hidden
        let filtered = table.with_visible_columns(60);
        assert!(filtered.is_some(), "Should filter when column is hidden");
        let filtered = filtered.unwrap();
        assert_eq!(filtered.columns.len(), 1);
        assert_eq!(filtered.columns[0].header, "Name");
        assert_eq!(filtered.data[0].len(), 1);

        // At width 100, both should be visible
        assert!(table.with_visible_columns(100).is_none());
    }

    #[test]
    fn test_conditional_column_hidden_in_render_content() {
        let table = Table::new()
            .with_columns(vec![
                TableColumn::new("Name"),
                TableColumn::new("Extra").with_when(Conditional::WidthGreaterThan(80)),
            ])
            .with_data(vec![vec!["Alice".into(), "detail".into()]]);

        // Narrow: Extra column should be hidden
        let narrow = table.render_content(Some(50), None, None, &Terminal::default());
        assert!(narrow.contains("Name"), "Should contain Name header");
        assert!(
            !narrow.contains("Extra"),
            "Extra should be hidden at width 50"
        );
        assert!(!narrow.contains("detail"), "Extra data should be hidden");
        assert!(narrow.contains("Alice"), "Name data should be present");

        // Wide: both columns should appear
        let wide = table.render_content(Some(100), None, None, &Terminal::default());
        assert!(wide.contains("Name"));
        assert!(wide.contains("Extra"));
        assert!(wide.contains("Alice"));
        assert!(wide.contains("detail"));
    }

    #[test]
    fn test_conditional_column_hidden_in_cursor_mode() {
        let table = Table::new()
            .with_columns(vec![
                TableColumn::new("Name"),
                TableColumn::new("Extra").with_when(Conditional::WidthGreaterThan(80)),
            ])
            .with_data(vec![vec!["Alice".into(), "detail".into()]])
            .prefer_cursor_alignment();

        // Narrow: Extra column should be hidden
        let narrow = table.render_optimistic(Some(50));
        assert!(narrow.contains("Name"));
        assert!(!narrow.contains("Extra"));
        assert!(!narrow.contains("detail"));
        assert!(narrow.contains("Alice"));

        // Wide: both visible
        let wide = table.render_optimistic(Some(100));
        assert!(wide.contains("Extra"));
        assert!(wide.contains("detail"));
    }

    #[test]
    fn test_conditional_all_columns_hidden_no_header() {
        let table = Table::new()
            .with_columns(vec![
                TableColumn::new("A").with_when(Conditional::WidthGreaterThan(200)),
            ])
            .with_data(vec![vec!["x".into()]]);

        let result = table.render_content(Some(50), None, None, &Terminal::default());
        // All columns filtered out → no header, no column data
        assert!(!result.contains("A"), "Header should not appear");
        assert!(!result.contains("x"), "Data should not appear");
    }

    #[test]
    fn test_conditional_less_than_or_equal_hides_wide_terminal() {
        let table = Table::new()
            .with_columns(vec![
                TableColumn::new("Name"),
                TableColumn::new("Compact").with_when(Conditional::LessThanOrEqual(40)),
            ])
            .with_data(vec![vec!["Alice".into(), "short".into()]]);

        // Narrow: both visible
        let narrow = table.render_content(Some(40), None, None, &Terminal::default());
        assert!(narrow.contains("Compact"));

        // Wide: Compact hidden
        let wide = table.render_content(Some(80), None, None, &Terminal::default());
        assert!(!wide.contains("Compact"));
        assert!(wide.contains("Name"));
    }

    #[test]
    fn test_conditional_preserves_row_widths() {
        let table = Table::new()
            .with_columns(vec![
                TableColumn::new("Name"),
                TableColumn::new("Details").with_when(Conditional::WidthGreaterThan(80)),
            ])
            .with_data(vec![
                vec!["Alice".into(), "detail A".into()],
                vec!["Bob".into(), "detail B".into()],
            ]);

        // At narrow width, table should still have consistent row widths
        let result = table.render_content(Some(50), None, None, &Terminal::default());
        let lines: Vec<&str> = result.lines().collect();
        let content_lines: Vec<&str> = lines
            .iter()
            .filter(|l| l.starts_with('│') && !l.contains('─'))
            .copied()
            .collect();

        if content_lines.len() >= 2 {
            let first_width = visible_width(content_lines[0]);
            for line in &content_lines {
                assert_eq!(
                    visible_width(line),
                    first_width,
                    "All content lines should have same width"
                );
            }
        }
    }

    #[test]
    fn test_conditional_no_available_width_shows_all() {
        let table = Table::new()
            .with_columns(vec![
                TableColumn::new("Name"),
                TableColumn::new("Details").with_when(Conditional::WidthGreaterThan(80)),
            ])
            .with_data(vec![vec!["Alice".into(), "detail".into()]]);

        // Without available_width, render_content does not filter
        let result = table.render_content(None, None, None, &Terminal::default());
        assert!(result.contains("Details"), "No width = no filtering");
    }

    #[test]
    fn table_render_tree_node_carries_layout_when_margins_set() {
        use renderable::layout::{Length, Edges};
        let mut table = Table::new()
            .with_columns(vec![TableColumn::new("Name")])
            .with_data(vec![vec!["Alice".into()]]);
        table.layout.margin = Edges::x(Length::ch(2));
        let node = table.render_tree_node().unwrap();
        assert!(node.attrs.layout().is_some());
    }
}
