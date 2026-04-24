//! Shared core types for tui-chrome components.
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
pub mod keybindings;
pub mod label;
pub mod sort;
pub mod standalone;
pub mod theme;
pub mod validation;

pub use event::EventOutcome;
pub use frame::{BorderStyle, FrameChrome, FrameChromeConfig, HeightSpec, Margin};
pub use keybindings::KeyBindings;
pub use label::{Label, LabelPosition, render_with_label};
pub use sort::SortOrder;
pub use standalone::{
    CANCELLED_KIND, HandleEvent, StandaloneState, drive_event_loop, run_standalone,
};
pub use theme::ComponentTheme;
pub use validation::ValidationState;
