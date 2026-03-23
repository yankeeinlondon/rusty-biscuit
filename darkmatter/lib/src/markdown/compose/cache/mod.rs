//! Caching infrastructure for the compose pipeline.
//!
//! Provides a two-layer caching strategy:
//! - **Run-local** (Phase 1): In-memory single-flight deduplication for
//!   concurrent transclusion within a single compose invocation.
//! - **Persistent** (Phase 2): File-backed artifact cache with
//!   Merkle-style dependency invalidation across runs.

pub(crate) mod hashing;
pub(crate) mod manifest;
pub(crate) mod operation;
pub(crate) mod runtime;
pub(crate) mod store;
pub(crate) mod types;

pub use types::{CacheAccessMode, CacheFreshnessMode, CacheStats};
pub(crate) use hashing::compose_cache_key as compose_cache_key_for_path;
pub(crate) use operation::{TocLinkingOperation, code_cache_key};
pub(crate) use runtime::{ComposeResult, OperationResult, PersistentContext, RunLocalCache};
pub(crate) use store::FileStore;
