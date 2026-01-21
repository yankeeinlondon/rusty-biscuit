//! TUI module for the queue application.
//!
//! This module provides the terminal user interface for managing scheduled tasks.

mod app;
mod event;
mod history_modal;
mod input_modal;
mod modal;
mod render;

pub use app::App;
pub use event::run_app;
