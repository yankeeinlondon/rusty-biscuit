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
//! - Left/Right and Up/Down arrows move focus within the grid,
//!   skipping [`CellState::StaticText`] cells.
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
//! `state.value()` returns `Vec<Vec<String>>` — one inner vector per
//! row, and one string per column. The string encoding is:
//!
//! - `StaticText`: the column's text.
//! - `BooleanSwitch`: `"true"` / `"false"`.
//! - `TextInput`: the edit buffer contents.
//! - `TextAreaInput`: the edit buffer, lines joined with `\n`.
//! - `ChooseOne`: the selected option value (empty string when none).
//! - `ChooseMany`: comma-separated selected values.

pub mod cell;
pub mod column;
pub mod table;

pub use cell::CellState;
pub use column::{BooleanSwitchConfig, InputTableColumn, TextAreaInputConfig, TextInputConfig};
pub use table::{InputTable, InputTableState};
