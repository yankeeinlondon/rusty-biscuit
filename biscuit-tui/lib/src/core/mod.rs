//! Shared core types for biscuit-tui components.
//!
//! This module centralises the small set of cross-cutting primitives
//! every component relies on:
//!
//! - [`EventOutcome`] — the canonical outcome enum returned from
//!   `handle_event`.
//! - [`ValidationState`] — uniform read access to a component's
//!   active validation error.
//! - [`Label`] / [`LabelPosition`] / [`render_with_label`] — label
//!   placement shared across every component.
//! - [`ComponentTheme`] — centralised visual constants.
//! - [`KeyBindings`] — configurable key bindings with vim-compatible
//!   defaults.
//! - [`run_standalone`] / [`drive_event_loop`] / [`StandaloneState`]
//!   / [`HandleEvent`] — helpers for running a single component in a
//!   dedicated terminal.

pub mod event;
pub mod frame;
pub mod fuzzy;
pub mod keybindings;
pub mod label;
pub mod sort;
pub mod split_pane;
pub mod standalone;
pub mod terminal_style;
pub mod theme;
pub mod validation;

pub use event::EventOutcome;
pub use frame::{BorderStyle, FrameChrome, FrameChromeConfig, HeightSpec, Margin, Padding};
pub use fuzzy::FuzzyFilter;
pub use keybindings::KeyBindings;
pub use label::{Label, LabelPosition, render_with_label};
pub use sort::{OptionSort, SortOrder};
pub use split_pane::{SplitDirection, SplitPane, SplitRatio};
pub use standalone::{
    ABORTED_KIND, CANCELLED_KIND, HandleEvent, LoopExit, StandaloneState, drive_event_loop,
    drive_event_loop_with_chrome, run_standalone, run_standalone_with_chrome,
};
pub use terminal_style::{NerdFontStatus, TerminalBackground, TerminalStyle, resolve_active_style};
pub use theme::ComponentTheme;
pub use validation::ValidationState;
