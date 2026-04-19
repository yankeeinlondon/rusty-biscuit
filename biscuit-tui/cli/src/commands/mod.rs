//! Subcommand implementations for the `question` CLI.
//!
//! Each subcommand is its own submodule: it parses its CLI args, maps
//! them to a component state, drives the component via
//! [`tui_chrome::run_standalone`], then formats the resulting value
//! per [`crate::output::OutputMode`].

pub mod text_input;
