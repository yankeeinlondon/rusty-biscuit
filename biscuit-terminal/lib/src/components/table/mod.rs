//! Table component split into focused submodules.
//!
//! - [`cell`] — `TableCellContent` plus number formatting helpers
//! - [`column`] — `TableColumn`, `Conditional`, and column metadata
//! - [`width`] — width planning types: `MeasuredColumn`, `TableWidthMeasurements`,
//!   `TableWidthPlan`, `TableWidthError`
//! - [`types`] — `ColumnType`, `Currency`, `VerticalAlign`
//! - [`table`] — `Table` struct, `impl Table`, `impl TerminalRenderable for Table`,
//!   and rendering helpers
//!
//! Public items are re-exported here so external consumers can import them
//! from `biscuit_terminal::components::table` directly.

pub mod cell;
pub mod column;
#[allow(clippy::module_inception)]
pub mod table;
pub mod types;
pub mod width;

pub use cell::TableCellContent;
pub use column::{Conditional, TableColumn};
pub use table::Table;
pub use types::{ColumnType, Currency, VerticalAlign};
pub use width::{MeasuredColumn, TableWidthError, TableWidthMeasurements, TableWidthPlan};
