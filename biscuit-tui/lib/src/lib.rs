//! TUI chrome: reusable input components built on Ratatui.
//!
//! This crate provides composable input widgets (text input, toggle,
//! choice selectors, text area, grid) designed to work both embedded
//! inside a larger application and standalone via a helper runner.
//!
//! Public surface is organised into three modules:
//!
//! - [`core`] — cross-cutting primitives ([`EventOutcome`], [`Label`],
//!   [`ComponentTheme`], [`KeyBindings`], [`run_standalone`], ...)
//! - [`components`] — per-component widgets + state structs.
//! - [`helpers`] — free functions for constructing component configs
//!   from CSV, Markdown lists, and dictionaries.
//!
//! Most callers will prefer the [`prelude`] re-export which exposes
//! the commonly used core types in a single glob import.

pub mod components;
pub mod core;
pub mod helpers;
pub mod prelude;

pub use components::{
    BooleanSwitch, BooleanSwitchState, CellState, CellValue, ChoiceInput, ChoiceOption, ChooseMany,
    ChooseManyState, ChooseOne, ChooseOneState, InputTable, InputTableColumn, InputTableState, Row,
    RowCell, SelectionMode, TextAreaInput, TextAreaInputState, TextInput, TextInputState,
};
pub use core::{
    ABORTED_KIND, BorderStyle, CANCELLED_KIND, ComponentTheme, EventOutcome, FrameChrome,
    FrameChromeConfig, HandleEvent, HeightSpec, KeyBindings, Label, LabelPosition, LoopExit,
    Margin, SortOrder, StandaloneState, ValidationState, drive_event_loop, render_with_label,
    run_standalone,
};
