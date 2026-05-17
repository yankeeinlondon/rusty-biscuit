use renderable::tree::{
    ColumnAlign, ColumnConditional, LayoutHints, RenderNode, TableCellHints, TableColumnHints,
    TableTerminalHints,
};

use crate::{
    components::renderable::TerminalRenderable,
    render_tree::render::resolve_cells,
    terminal::Terminal,
    utils::{
        block_constraint::{sanitize_wrapped_lines, split_lines, visible_width, wrap_lines},
        layout::{Alignment, Layout, LayoutTerminalExt},
        wrap_policy::WordWrap,
    },
};

pub use super::cell::TableCellContent;
use super::cell::pad_cell;
pub use super::column::TableColumn;
use super::types::{Currency, VerticalAlign};
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
#[derive(Debug, Default)]
pub struct Table {
    title: Option<String>,
    columns: Vec<TableColumn>,
    data: Vec<Vec<TableCellContent>>,
    layout: Layout,
    /// When true, use ANSI cursor positioning for cell alignment instead of
    /// space-based padding. This can improve alignment when glyphs render
    /// narrower than their computed Unicode width.
    prefer_cursor_alignment: bool,
    /// When true, apply a subtle background color to even data rows (0-indexed,
    /// so the second, fourth, etc. rows get the stripe). Requires true color
    /// support; silently ignored otherwise.
    alternate_background_color: bool,
    /// When true, apply a subtle text color shift to even data rows (0-indexed,
    /// so the second, fourth, etc. rows get the tint). Requires true color
    /// support; silently ignored otherwise.
    alternate_text_color: bool,
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

    /// Enable alternating row background colors.
    ///
    /// When enabled, even data rows (0-indexed: rows 1, 3, 5, ...) receive a
    /// very subtle background tint. The tint adapts to the terminal's light or
    /// dark color mode.
    ///
    /// Requires true-color (24-bit) support. On terminals without true color
    /// the setting is silently ignored and rows render without background.
    pub fn alternate_background_color(mut self) -> Self {
        self.alternate_background_color = true;
        self
    }

    /// Enable alternating row text colors.
    ///
    /// When enabled, even data rows (0-indexed: rows 1, 3, 5, ...) receive a
    /// subtle text color shift. The tint adapts to the terminal's light or
    /// dark color mode.
    ///
    /// Requires true-color (24-bit) support. On terminals without true color
    /// the setting is silently ignored and rows render without text tint.
    pub fn alternate_text_color(mut self) -> Self {
        self.alternate_text_color = true;
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
            alternate_background_color: self.alternate_background_color,
            alternate_text_color: self.alternate_text_color,
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

        if !measurements.word_wrap_needed {
            for column in &mut measurements.columns {
                if column.fixed_width.is_none() {
                    column.resolved_width = column.columnar_width_requirement;
                }
            }

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
        let header_content = column
            .header_prose
            .as_ref()
            .map(|prose| prose.content())
            .unwrap_or(&column.header);
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

    /// Render the table content with multi-line cell support.
    ///
    /// If `available_width` is provided, columns will be constrained to fit
    /// within that space (accounting for border overhead).
    ///
    /// If `stripe_color_mode` is `Some`, even data rows (0-indexed: 1, 3, 5, ...)
    /// receive a subtle background tint adapted to the given color mode.
    fn render_content(
        &self,
        available_width: Option<u32>,
        stripe_color_mode: Option<&ColorMode>,
        text_color_mode: Option<&ColorMode>,
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
                    let header_content = column
                        .and_then(|col| col.header_prose.as_ref().map(|p| p.content()))
                        .unwrap_or_else(|| column.map(|col| col.header.as_str()).unwrap_or(""));
                    wrap_cell_content(header_content, &WordWrap::None, column_plan.resolved_width)
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

        // Resolve stripe escape once (if enabled)
        let stripe_bg = stripe_color_mode.map(stripe_bg_escape);
        let stripe_fg = text_color_mode.map(stripe_fg_escape);

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
        stripe_color_mode: Option<&ColorMode>,
        text_color_mode: Option<&ColorMode>,
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
                    let header_content = column
                        .and_then(|col| col.header_prose.as_ref().map(|p| p.content()))
                        .unwrap_or_else(|| column.map(|col| col.header.as_str()).unwrap_or(""));
                    wrap_cell_content(header_content, &WordWrap::None, column_plan.resolved_width)
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

        // Resolve stripe escapes once (if enabled)
        let stripe_bg = stripe_color_mode.map(stripe_bg_escape);
        let stripe_fg = text_color_mode.map(stripe_fg_escape);

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

impl TerminalRenderable for Table {
    fn render_optimistic(&self, term_width: Option<u32>) -> String {
        let width = term_width.unwrap_or(80);
        // Opportunistic render assumes full capability; stripe if requested
        let stripe = if self.alternate_background_color {
            Some(ColorMode::Dark)
        } else {
            None
        };
        let text_tint = if self.alternate_text_color {
            Some(ColorMode::Dark)
        } else {
            None
        };
        if self.prefer_cursor_alignment {
            self.render_with_cursor_positioning(width, stripe.as_ref(), text_tint.as_ref())
        } else {
            let available = self.layout.available_width(width);
            let content = self.render_content(Some(available), stripe.as_ref(), text_tint.as_ref());
            // Box-drawing borders and column edges must stay vertically
            // aligned across all rows — align as a block.
            self.layout.apply_block_layout(&content, width)
        }
    }

    fn render(&self, term: &Terminal) -> String {
        let width = term.width();
        let has_true_color = term.color_depth == ColorDepth::TrueColor;
        // Only stripe when the terminal actually supports true color
        let stripe = if self.alternate_background_color && has_true_color {
            Some(term.color_mode())
        } else {
            None
        };
        let text_tint = if self.alternate_text_color && has_true_color {
            Some(term.color_mode())
        } else {
            None
        };
        if self.prefer_cursor_alignment && term.is_tty {
            self.render_with_cursor_positioning(width, stripe.as_ref(), text_tint.as_ref())
        } else {
            let available = self.layout.available_width(width);
            let content = self.render_content(Some(available), stripe.as_ref(), text_tint.as_ref());
            self.layout.apply_block_layout(&content, width)
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

    /// Projects this table into a canonical [`NodeKind::Table`] render node.
    ///
    /// The first child row is the header row; each remaining child row is a
    /// data row. Every cell carries the readable pre-formatted text as a
    /// [`NodeKind::Text`] node, plus [`TableCellHints`] recording the cell
    /// kind, the original typed value as JSON, and alignment. The table node
    /// carries per-column [`TableColumnHints`] and [`TableTerminalHints`], and
    /// [`LayoutHints`] when margins are non-default.
    ///
    /// [`NodeKind::Table`]: renderable::tree::NodeKind::Table
    /// [`NodeKind::Text`]: renderable::tree::NodeKind::Text
    fn render_tree_node(&self) -> Option<RenderNode> {
        let align: Vec<ColumnAlign> = self
            .columns
            .iter()
            .map(|col| alignment_to_column_align(col.effective_alignment()))
            .collect();

        let mut rows: Vec<RenderNode> = Vec::with_capacity(self.data.len() + 1);

        // Header row: each cell is the column header text.
        let header_cells: Vec<RenderNode> = self
            .columns
            .iter()
            .map(|col| RenderNode::table_cell(vec![RenderNode::text(col.header.clone())]))
            .collect();
        rows.push(RenderNode::table_row(header_cells));

        // Data rows: each cell is the readable pre-formatted string plus hints.
        for row in &self.data {
            let cells: Vec<RenderNode> = row
                .iter()
                .enumerate()
                .map(|(col_idx, content)| {
                    let mut cell =
                        RenderNode::table_cell(vec![RenderNode::text(content.to_string())]);
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
                drop_note: col.drop_note(),
                uniform_alignment: col.uniform_alignment,
            };
            node.attrs.set_table_column_hints(idx, &hints);
        }

        // Terminal hints on the table node.
        node.attrs.set_table_terminal_hints(&TableTerminalHints {
            prefer_cursor_alignment: self.prefer_cursor_alignment,
            alternate_background: self.alternate_background_color,
            alternate_text_color: self.alternate_text_color,
        });

        // Layout hints when margins are non-default. Margins are resolved to
        // character counts against a representative 80-column width.
        let margins_non_default = self.layout.margin != renderable::layout::Margin::default();
        if margins_non_default {
            let hints = LayoutHints {
                left_margin: Some(resolve_cells(&self.layout.margin.left, 80)),
                right_margin: Some(resolve_cells(&self.layout.margin.right, 80)),
                top_margin: Some(resolve_cells(&self.layout.margin.top, 80)),
                bottom_margin: Some(resolve_cells(&self.layout.margin.bottom, 80)),
            };
            for (key, value) in [
                ("left_margin", hints.left_margin),
                ("right_margin", hints.right_margin),
                ("top_margin", hints.top_margin),
                ("bottom_margin", hints.bottom_margin),
            ] {
                if let Some(v) = value {
                    node.attrs.set_hint(
                        renderable::tree::HintNamespace::LAYOUT,
                        key,
                        serde_json::Value::from(v),
                    );
                }
            }
        }

        Some(node)
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
    }
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
        return lines;
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

/// Returns the ANSI escape sequence for the alternating row background color.
///
/// Produces a very subtle tint that adapts to the terminal's color mode:
/// - **Dark mode**: a faint warm gray (`rgb(30, 30, 34)`)
/// - **Light mode**: a faint cool gray (`rgb(235, 235, 238)`)
pub(crate) fn stripe_bg_escape(color_mode: &ColorMode) -> &'static str {
    match color_mode {
        ColorMode::Light => "\x1b[48;2;235;235;238m",
        ColorMode::Dark | ColorMode::Unknown => "\x1b[48;2;30;30;34m",
    }
}

/// Returns the ANSI escape sequence for the alternating row text color.
///
/// Produces a subtle shift from the default foreground:
/// - **Dark mode**: a slightly muted light gray (`rgb(180, 180, 190)`)
/// - **Light mode**: a slightly softened dark gray (`rgb(80, 80, 90)`)
pub(crate) fn stripe_fg_escape(color_mode: &ColorMode) -> &'static str {
    match color_mode {
        ColorMode::Light => "\x1b[38;2;80;80;90m",
        ColorMode::Dark | ColorMode::Unknown => "\x1b[38;2;180;180;190m",
    }
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

        let result = table.render_content(None, None, None);
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

        let result = table.render_content(None, None, None);
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

        let result = table.render_content(None, None, None);
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

        let result = table.render_content(None, None, None);
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

        let result = table.render_content(None, None, None);
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

        let result = table.render_content(None, None, None);
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

        let result = table.render_content(None, None, None);
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

    #[test]
    fn test_cursor_alignment_uses_escape_codes() {
        let table = Table::new()
            .with_columns(vec![TableColumn::new("Name")])
            .with_data(vec![vec!["Alice".into()]])
            .prefer_cursor_alignment();

        let result = table.render_optimistic(Some(80));
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

        let result = table.render_optimistic(Some(80));
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

        let result = table.render_optimistic(Some(80));
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

        let result = table.render_optimistic(Some(80));
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
        use crate::utils::layout::Margin;

        let mut table = Table::new()
            .with_columns(vec![TableColumn::new("X")])
            .with_data(vec![vec!["A".into()]])
            .prefer_cursor_alignment();
        table.layout_mut().left_margin = Margin::Chars(2);
        table.layout_mut().right_margin = Margin::Chars(2);
        table.layout_mut().row_fill_strategy = RowFill::Fill;

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

        let rendered = table.render_content(Some(28), None, None);
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

        let result = table.render_content(None, None, None);
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

        let result = table.render_content(None, None, None);
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

        let result = table.render_content(None, None, None);
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

        let result = table.render_content(None, None, None);
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

        let result = table.render_content(None, None, None);
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

        let result = table.render_content(None, None, None);
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

        let result = table.render_content(None, None, None);
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

        let result = table.render_content(None, None, None);
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

        let result = table.render_optimistic(Some(80));

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

        let result = table.render_optimistic(Some(80));

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

        let result = table.render_content(None, None, None);
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

        let result = table.render_content(None, None, None);

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

        let result = table.render_content(None, None, None);
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

        let result = table.render_content(None, None, None);

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

        let result = table.render_optimistic(Some(60));

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

        let result = table.render_content(None, None, None);
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

        let result = table.render_content(None, None, None);

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

        let result = table.render_content(None, None, None);
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

        let result = table.render_content(None, None, None);

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
        assert!(table.alternate_background_color);
    }

    #[test]
    fn test_alternate_background_color_default_is_false() {
        let table = Table::new();
        assert!(!table.alternate_background_color);
    }

    #[test]
    fn test_stripe_bg_escape_dark_mode() {
        let esc = stripe_bg_escape(&ColorMode::Dark);
        assert_eq!(esc, "\x1b[48;2;30;30;34m");
    }

    #[test]
    fn test_stripe_bg_escape_light_mode() {
        let esc = stripe_bg_escape(&ColorMode::Light);
        assert_eq!(esc, "\x1b[48;2;235;235;238m");
    }

    #[test]
    fn test_stripe_bg_escape_unknown_uses_dark() {
        let esc = stripe_bg_escape(&ColorMode::Unknown);
        assert_eq!(esc, stripe_bg_escape(&ColorMode::Dark));
    }

    #[test]
    fn test_render_content_no_stripe_without_flag() {
        let table = Table::new()
            .with_columns(vec![TableColumn::new("X")])
            .with_data(vec![vec!["A".into()], vec!["B".into()]]);

        let result = table.render_content(None, None, None);
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

        let result = table.render_content(None, Some(&ColorMode::Dark), None);
        let lines: Vec<&str> = result.lines().collect();

        // Data rows start at line 3 (border=0, header=1, separator=2)
        // Row 0 (line 3): no stripe
        // Row 1 (line 4): stripe (odd index = even row in 0-based)
        // Row 2 (line 5): no stripe
        // Row 3 (line 6): stripe

        let bg_dark = stripe_bg_escape(&ColorMode::Dark);
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

        let result = table.render_content(None, Some(&ColorMode::Dark), None);
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

        let result = table.render_content(None, Some(&ColorMode::Light), None);
        let bg_light = stripe_bg_escape(&ColorMode::Light);
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

        let result = table.render_content(None, Some(&ColorMode::Dark), None);
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

        let result = table.render_with_cursor_positioning(80, Some(&ColorMode::Dark), None);
        let bg_dark = stripe_bg_escape(&ColorMode::Dark);
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

        let result = table.render_content(None, Some(&ColorMode::Dark), None);
        let bg = stripe_bg_escape(&ColorMode::Dark);

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

        let result = table.render_with_cursor_positioning(80, Some(&ColorMode::Dark), None);
        let bg = stripe_bg_escape(&ColorMode::Dark);

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

        let bg = stripe_bg_escape(&ColorMode::Dark);
        let result = table.render_content(None, Some(&ColorMode::Dark), None);

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

        let bg = stripe_bg_escape(&ColorMode::Dark);
        let result = table.render_with_cursor_positioning(80, Some(&ColorMode::Dark), None);

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

        let bg = stripe_bg_escape(&ColorMode::Dark);
        let result = table.render_content(None, Some(&ColorMode::Dark), None);

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

        let bg = stripe_bg_escape(&ColorMode::Dark);
        let result = table.render_with_cursor_positioning(80, Some(&ColorMode::Dark), None);

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
        assert!(table.alternate_text_color);
    }

    #[test]
    fn test_alternate_text_color_default_is_false() {
        let table = Table::new();
        assert!(!table.alternate_text_color);
    }

    #[test]
    fn test_stripe_fg_escape_dark_mode() {
        let esc = stripe_fg_escape(&ColorMode::Dark);
        assert_eq!(esc, "\x1b[38;2;180;180;190m");
    }

    #[test]
    fn test_stripe_fg_escape_light_mode() {
        let esc = stripe_fg_escape(&ColorMode::Light);
        assert_eq!(esc, "\x1b[38;2;80;80;90m");
    }

    #[test]
    fn test_stripe_fg_escape_unknown_uses_dark() {
        let esc = stripe_fg_escape(&ColorMode::Unknown);
        assert_eq!(esc, stripe_fg_escape(&ColorMode::Dark));
    }

    #[test]
    fn test_render_content_no_text_tint_without_flag() {
        let table = Table::new()
            .with_columns(vec![TableColumn::new("X")])
            .with_data(vec![vec!["A".into()], vec!["B".into()]]);

        let result = table.render_content(None, None, None);
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

        let result = table.render_content(None, None, Some(&ColorMode::Dark));
        let lines: Vec<&str> = result.lines().collect();

        let fg_dark = stripe_fg_escape(&ColorMode::Dark);
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

        let result = table.render_content(None, None, Some(&ColorMode::Dark));
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

        let result = table.render_content(None, None, Some(&ColorMode::Light));
        let fg_light = stripe_fg_escape(&ColorMode::Light);
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

        let result = table.render_content(None, None, Some(&ColorMode::Dark));
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

        let result = table.render_content(None, None, Some(&ColorMode::Dark));
        let fg = stripe_fg_escape(&ColorMode::Dark);

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

        let result = table.render_with_cursor_positioning(80, None, Some(&ColorMode::Dark));
        let fg_dark = stripe_fg_escape(&ColorMode::Dark);
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

        let result = table.render_content(None, Some(&ColorMode::Dark), Some(&ColorMode::Dark));
        let bg = stripe_bg_escape(&ColorMode::Dark);
        let fg = stripe_fg_escape(&ColorMode::Dark);

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

        let fg = stripe_fg_escape(&ColorMode::Dark);
        let result = table.render_content(None, None, Some(&ColorMode::Dark));

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
        let output = table.render_with_cursor_positioning(50, None, None);
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
        let narrow = table.render_content(Some(50), None, None);
        assert!(narrow.contains("Name"), "Should contain Name header");
        assert!(
            !narrow.contains("Extra"),
            "Extra should be hidden at width 50"
        );
        assert!(!narrow.contains("detail"), "Extra data should be hidden");
        assert!(narrow.contains("Alice"), "Name data should be present");

        // Wide: both columns should appear
        let wide = table.render_content(Some(100), None, None);
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

        let result = table.render_content(Some(50), None, None);
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
        let narrow = table.render_content(Some(40), None, None);
        assert!(narrow.contains("Compact"));

        // Wide: Compact hidden
        let wide = table.render_content(Some(80), None, None);
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
        let result = table.render_content(Some(50), None, None);
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
        let result = table.render_content(None, None, None);
        assert!(result.contains("Details"), "No width = no filtering");
    }
}
