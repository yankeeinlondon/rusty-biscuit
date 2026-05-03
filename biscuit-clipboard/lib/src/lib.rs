//! Clipboard interaction library for the biscuit-clipboard ecosystem.
//!
//! This crate provides clipboard observation, history management, and content
//! models for the `clipper` background service and `clip` CLI.
//!
//! ## Module Layout
//!
//! - [`content`] — `ContentType`, `ClipboardFormat`, `ImageSnapshot`
//! - [`entry`] — `ClipboardEntry` with xxHash-based `EntryId`
//! - [`backend`] — `ClipboardBackend` trait and `SystemClipboard` implementation
//! - [`history`] — Ring buffer with 1-hour TTL and 2-entry floor
//! - [`storage`] — Disk-spill logic for large entries
//! - [`error`] — Error types

pub mod backend;
pub mod client;
pub mod config;
pub mod content;
pub mod entry;
pub mod error;
pub mod history;
pub mod storage;
pub mod watcher;

pub use backend::ClipboardBackend;
pub use client::{ClipperClient, EntrySummary, ServiceStatus};
pub use content::{ClipboardFormat, ContentType, ImageSnapshot};
pub use entry::{ClipboardEntry, EntryId};
pub use error::ClipboardError;
pub use history::History;
pub use storage::Storage;
pub use watcher::{spawn_watcher, Supervisor, SupervisorAction, SupervisorStatus, WatcherEvent};
