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
    ActiveChoiceColor, BooleanSwitch, BooleanSwitchState, CellState, CellValue, ChoiceInput,
    ChoiceOption, ChooseMany, ChooseManyState, ChooseOne, ChooseOneState, HotkeyDisplayMode,
    HotkeySpec, InputTable, InputTableColumn, InputTableState, Orientation, Row, RowCell,
    SelectionMode, TextAreaInput, TextAreaInputState, TextInput, TextInputState,
};
pub use crate::core::{
    ABORTED_KIND, BorderStyle, CANCELLED_KIND, ComponentTheme, EventOutcome, FrameChrome,
    FrameChromeConfig, FuzzyFilter, HandleEvent, HeightSpec, KeyBindings, Label, LabelPosition,
    LoopExit, Margin, NerdFontStatus, OptionSort, Padding, SortOrder, StandaloneState,
    TerminalBackground, TerminalStyle, ValidationState, drive_event_loop,
    drive_event_loop_with_chrome, render_with_label, run_standalone, run_standalone_with_chrome,
};
