//! Visual input components (text input, toggle, choice, text area, table).
//!
//! Each component follows the same pattern: a zero-sized
//! [`ratatui::widgets::StatefulWidget`] marker paired with a
//! component-specific `*State` struct owned by the caller. Widgets
//! implement [`crate::core::HandleEvent`] so they can be driven by
//! [`crate::core::run_standalone`].

pub mod boolean_switch;
pub mod choose;
pub mod choose_many;
pub mod choose_one;
pub mod input_table;
pub mod text_area_input;
pub mod text_input;

pub use boolean_switch::{BooleanSwitch, BooleanSwitchState};
pub use choose::{ChoiceInput, ChoiceOption, SelectionMode};
pub use choose_many::{ChooseMany, ChooseManyState};
pub use choose_one::{ChooseOne, ChooseOneState};
pub use input_table::{
    CellState, CellValue, InputTable, InputTableColumn, InputTableState, Row, RowCell,
};
pub use text_area_input::{TextAreaInput, TextAreaInputState};
pub use text_input::{TextInput, TextInputState};
