//! TUI module for the queue application.
//!
//! This module provides the terminal user interface for managing scheduled tasks.

use ratatui::style::Color;

mod app;
mod color_context;
mod event;
mod history_modal;
mod input_modal;
mod modal;
mod render;

pub use app::App;
pub use event::run_app;

pub const PANEL_BG: Color = Color::Rgb(64, 70, 86);
