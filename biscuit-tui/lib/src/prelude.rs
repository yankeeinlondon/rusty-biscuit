//! Convenience re-exports for the most commonly used tui-chrome
//! types.
//!
//! Glob-importing this module is the easiest way to start building
//! with the library:
//!
//! ```
//! use tui_chrome::prelude::*;
//! ```
//!
//! Component widgets (TextInput, BooleanSwitch, ...) land in later
//! phases and are re-exported here as they arrive.

pub use crate::core::{
    CANCELLED_KIND, ComponentTheme, EventOutcome, HandleEvent, KeyBindings, Label, LabelPosition,
    StandaloneState, ValidationState, drive_event_loop, render_with_label, run_standalone,
};
