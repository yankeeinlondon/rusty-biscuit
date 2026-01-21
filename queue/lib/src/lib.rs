//! Queue library for scheduling commands.
//!
//! This library provides core data types, persistence, and parsing utilities
//! for the queue CLI, along with terminal detection capabilities for adaptive
//! TUI behavior.
//!
//! ## Core Types
//!
//! - [`ScheduledTask`] - A task scheduled for future execution
//! - [`ExecutionTarget`] - Where to run the task (pane, window, background)
//! - [`TaskStatus`] - Current status of a task (pending, running, completed, cancelled, failed)
//!
//! ## Task Execution
//!
//! - [`TaskExecutor`] - Executes scheduled tasks at their designated times
//! - [`TaskEvent`] - Events emitted during task execution
//!
//! ## History Storage
//!
//! - [`HistoryStore`] - Trait for history storage backends
//! - [`JsonFileStore`] - JSONL file-based storage with file locking
//!
//! ## Parsing Utilities
//!
//! - [`parse_at_time`] - Parse time strings like "7:00am" or "19:30"
//! - [`parse_delay`] - Parse delay strings like "15m" or "2h"
//!
//! ## Terminal Detection
//!
//! - [`TerminalDetector`] - Detects terminal emulator from environment
//! - [`TerminalCapabilities`] - Available features for the detected terminal
//! - [`TerminalKind`] - Known terminal emulator types

mod error;
mod executor;
mod history;
mod parse;
pub mod terminal;
mod types;

pub use error::HistoryError;
pub use executor::{TaskEvent, TaskExecutor};
pub use history::{HistoryStore, JsonFileStore};
pub use parse::{parse_at_time, parse_delay};
pub use terminal::{TerminalCapabilities, TerminalDetector, TerminalKind, TuiLayoutResult};
pub use types::{ExecutionTarget, ScheduledTask, TaskStatus};
