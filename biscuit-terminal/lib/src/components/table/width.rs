use thiserror::Error;

use crate::utils::wrap_policy::WordWrap;

/// Measured width facts for a single visible table column.
#[derive(Debug, Clone, PartialEq)]
pub struct MeasuredColumn {
    pub original_index: usize,
    pub header_width: usize,
    pub header_line_width: usize,
    pub cell_max_width: usize,
    pub cell_line_max_width: usize,
    pub columnar_width_requirement: usize,
    pub fixed_width: Option<usize>,
    pub min_width: Option<usize>,
    pub max_width: Option<usize>,
    pub effective_word_wrap: WordWrap,
    pub natural_break_width: usize,
    pub resolved_width: usize,
    pub is_non_wrapping: bool,
    pub is_shrinkable: bool,
    pub drop_note: Option<String>,
    pub header_lines: Vec<String>,
}

/// Table-level measurements captured before a final width fit is resolved.
#[derive(Debug, Clone, PartialEq)]
pub struct TableWidthMeasurements {
    pub available_render_width: usize,
    pub border_overhead: usize,
    pub content_budget: usize,
    pub fixed_width_consumption: usize,
    pub non_wrapping_consumption: usize,
    pub working_width: usize,
    pub word_wrap_needed: bool,
    pub columns: Vec<MeasuredColumn>,
}

/// Final table width plan shared by all rendering paths.
#[derive(Debug, Clone, PartialEq)]
pub struct TableWidthPlan {
    pub available_render_width: usize,
    pub visible_column_indices: Vec<usize>,
    pub dropped_column_indices: Vec<usize>,
    pub columns: Vec<MeasuredColumn>,
    pub table_width: usize,
    pub dropped_notes: Vec<String>,
}

/// Structured width planning errors for tables.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum TableWidthError {
    #[error("Table could not be rendered because no columns are visible.")]
    NoVisibleColumns,
    #[error(
        "Table could not be rendered in {available_render_width} columns. Fixed-width columns require {required_width} content columns before borders."
    )]
    InsufficientWidthForFixedColumns {
        available_render_width: usize,
        border_overhead: usize,
        content_budget: usize,
        fixed_width_consumption: usize,
        non_wrapping_consumption: usize,
        required_width: usize,
        blocking_column_indices: Vec<usize>,
        droppable_columns_available: bool,
    },
    #[error(
        "Table could not be rendered in {available_render_width} columns. Fixed and non-wrapping columns require {required_width} content columns before borders."
    )]
    InsufficientWidthForNonWrappingColumns {
        available_render_width: usize,
        border_overhead: usize,
        content_budget: usize,
        fixed_width_consumption: usize,
        non_wrapping_consumption: usize,
        required_width: usize,
        blocking_column_indices: Vec<usize>,
        droppable_columns_available: bool,
    },
    #[error(
        "Table could not be rendered in {available_render_width} columns. Wrapping columns need at least {required_width} content columns after fixed and non-wrapping columns are reserved."
    )]
    InsufficientWidthForWrappingColumns {
        available_render_width: usize,
        border_overhead: usize,
        content_budget: usize,
        fixed_width_consumption: usize,
        non_wrapping_consumption: usize,
        required_width: usize,
        blocking_column_indices: Vec<usize>,
        droppable_columns_available: bool,
    },
    #[error(
        "Table could not be rendered in {available_render_width} columns even after dropping eligible columns."
    )]
    InsufficientWidthAfterDropping {
        available_render_width: usize,
        border_overhead: usize,
        content_budget: usize,
        fixed_width_consumption: usize,
        non_wrapping_consumption: usize,
        required_width: usize,
        blocking_column_indices: Vec<usize>,
        dropped_column_indices: Vec<usize>,
    },
}

impl TableWidthPlan {
    pub(super) fn content_widths(&self) -> Vec<usize> {
        self.columns
            .iter()
            .map(|column| column.resolved_width)
            .collect()
    }
}
