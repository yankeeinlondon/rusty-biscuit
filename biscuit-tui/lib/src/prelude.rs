//! Convenience re-exports for the most commonly used tui-chrome
//! types.
//!
//! Glob-importing this module is the easiest way to start building
//! with the library:
//!
//! ```
//! use tui_chrome::prelude::*;
//! ```

pub use crate::components::{
    BooleanSwitch, BooleanSwitchState, CellState, CellValue, ChoiceInput, ChoiceOption, ChooseMany,
    ChooseManyState, ChooseOne, ChooseOneState, InputTable, InputTableColumn, InputTableState, Row,
    RowCell, SelectionMode, TextAreaInput, TextAreaInputState, TextInput, TextInputState,
};
pub use crate::core::{
    BorderStyle, CANCELLED_KIND, ComponentTheme, EventOutcome, FrameChrome, FrameChromeConfig,
    HandleEvent, HeightSpec, KeyBindings, Label, LabelPosition, Margin, SortOrder, StandaloneState,
    ValidationState, drive_event_loop, render_with_label, run_standalone,
};
