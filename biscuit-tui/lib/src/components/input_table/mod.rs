//! Grid container with heterogeneous editable cells.
//!
//! [`InputTable`] arranges a fixed vector of [`InputTableColumn`]s and
//! a fixed number of rows into a 2D grid. Each cell owns a
//! [`CellState`] that delegates event handling and rendering to the
//! corresponding primitive component ([`TextInput`], [`BooleanSwitch`],
//! [`TextAreaInput`], [`ChooseOne`], [`ChooseMany`]) or renders a
//! display-only string for [`InputTableColumn::StaticText`] columns.
//!
//! [`TextInput`]: crate::components::TextInput
//! [`BooleanSwitch`]: crate::components::BooleanSwitch
//! [`TextAreaInput`]: crate::components::TextAreaInput
//! [`ChooseOne`]: crate::components::ChooseOne
//! [`ChooseMany`]: crate::components::ChooseMany
//!
//! ## Focus & Submit
//!
//! - Left/Right arrows move focus across columns (except inside
//!   editable text cells where they control the cursor).
//! - Up/Down arrows move focus across rows (except inside choice cells
//!   where they navigate the option list; use Tab/BackTab or Alt+Up/
//!   Alt+Down to move between rows from choice cells).
//! - Tab and Shift-Tab wrap across rows.
//! - Ctrl-S validates all cells and either submits (returning
//!   [`EventOutcome::Submitted`]) or routes focus to the first cell
//!   with an active validation error.
//! - Esc cancels the table.
//!
//! [`EventOutcome::Submitted`]: crate::core::EventOutcome::Submitted
//!
//! ## Value Shape
//!
//! [`InputTableState::value`] returns `&[Row]` — each [`Row`] contains
//! a vector of [`RowCell`]s pairing column ids with typed [`CellValue`]s.
//! This preserves semantic types (booleans, multi-line text,
//! multi-select arrays) rather than flattening to strings.
//!
//! [`InputTableState::value`]: table::InputTableState::value

pub mod cell;
pub mod column;
pub mod error;
pub mod table;

pub use cell::{CellState, CellValue, Row, RowCell};
pub use column::{BooleanSwitchConfig, InputTableColumn, TextAreaInputConfig, TextInputConfig};
pub use error::InputTableError;
pub use table::{InputTable, InputTableState};
