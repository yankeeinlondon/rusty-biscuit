//! Caching infrastructure for the compose pipeline.
//!
//! - **Run-local**: in-memory single-flight deduplication for concurrent
//!   transclusion within a single compose invocation. Never persisted.
//! - **Remote transport cache**: fetched HTTP(S) response bodies under an
//!   explicitly configured cache root, governed by response `Cache-Control`
//!   (`remote_cache`). No semantic result is persisted until a
//!   `ContentPolicy` exists.

pub(crate) mod hashing;
pub(crate) mod manifest;
pub(crate) mod operation;
pub(crate) mod remote_cache;
pub(crate) mod runtime;
pub(crate) mod store;
pub(crate) mod types;

pub(crate) use hashing::compose_cache_key as compose_cache_key_for_path;
pub(crate) use operation::{CodeOperation, TocLinkingOperation};
pub(crate) use runtime::{ComposeResult, OperationResult, RunLocalCache};
pub(crate) use store::FileStore;
pub use types::{CacheAccessMode, CacheStats};
