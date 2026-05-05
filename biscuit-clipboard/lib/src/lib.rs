//! Clipboard interaction library for the biscuit-clipboard ecosystem.
//!
//! This crate provides clipboard observation, history management, and
//! content models for the `clipper` background service and `clip` CLI.
//!
//! ## Module Layout
//!
//! - [`content`] — `ContentType`, `ClipboardFormat`, `ImageSnapshot`
//! - [`entry`] — `ClipboardEntry`, the [`EntryId`] newtype, and the
//!   single source of `FORMAT_PRIORITY`
//! - [`backend`] — `ClipboardBackend` trait and `SystemClipboard`
//!   implementation
//! - [`history`] — `VecDeque`-backed ring buffer with TTL + min/max
//!   bounds
//! - [`storage`] — Disk-spill logic for large entries
//! - [`watcher`] — Change-listener watcher (bounded mpsc; drop-newest
//!   on overflow) and the [`Supervisor`] type
//! - [`api_types`] — Shared REST request/response types
//!   ([`EntrySummary`], [`SetRequest`], [`ErrorResponse`], …)
//! - [`client`] — [`ClipperClient`] (REST client used by `clip`)
//! - [`config`] — Port resolution, runtime-dir, cross-platform PID
//!   liveness checks
//! - [`error`] — [`ClipboardError`] enum
//!
//! ## Notes
//!
//! The Axum router that exposes the REST surface lives in the
//! `biscuit-clipboard-service` crate (`service/src/api.rs`), not in
//! this lib crate. Consumers wanting to embed the service should
//! depend on `biscuit-clipboard-service` directly. This split means:
//!
//! - the lib crate is async-runtime-light (no Axum dependency); it can
//!   be compiled into thin wrappers that only consume the client and
//!   types.
//! - the wire types (in [`api_types`]) live here so both crates speak
//!   the same shape.

pub mod api_types;
pub mod backend;
pub mod client;
pub mod config;
pub mod content;
pub mod entry;
pub mod error;
pub mod history;
pub mod storage;
pub mod watcher;

pub use api_types::{
    ContentQuery, Encoding, EntrySummary, ErrorBody, ErrorCode, ErrorResponse, HealthResponse,
    HistoryQuery, HistoryResponse, SetRequest, SetResponse,
};
pub use backend::ClipboardBackend;
pub use client::{ClientError, ClipperClient, ServiceStatus};
pub use content::{ClipboardFormat, ContentType, ImageSnapshot};
pub use entry::{ClipboardEntry, EntryId, EntryIdParseError, FORMAT_PRIORITY};
pub use error::ClipboardError;
pub use history::History;
pub use storage::Storage;
pub use watcher::{
    DEFAULT_CHANNEL_CAPACITY, Supervisor, SupervisorAction, SupervisorStatus, WatcherEvent,
    WatcherTrigger, spawn_system_watcher, spawn_watcher, spawn_watcher_with_trigger,
};
