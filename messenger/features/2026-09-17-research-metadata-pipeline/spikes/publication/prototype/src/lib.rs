//! Publication spike: two strategies for publishing a multi-file accepted
//! research snapshot to fixed, git-committed paths.
//!
//! - [`journal`] (strategy A): committed manifest + local journal with
//!   staged and backup copies. The fixed manifest replacement is the single
//!   selection point.
//! - [`generations`] (strategy B): versioned generation directories in local
//!   state plus an atomically replaced pointer file; fixed paths are a mirror.
//!
//! Both share the fixture model in [`model`], the file primitives in
//! [`fsutil`], and the fault injection in [`fault`]. See `../findings.md`.

pub mod fault;
pub mod fsutil;
pub mod generations;
pub mod journal;
pub mod model;

use std::fmt;
use std::io;
use std::path::PathBuf;

/// Errors from publication, recovery, and verified reads.
#[derive(Debug)]
pub enum SpikeError {
    Io { context: String, source: io::Error },
    /// A fault hook fired (in-process "crash"); disk is left as-is.
    Crash(fault::Point),
    /// Another publisher or recovery holds the lock.
    Locked,
    /// No committed manifest exists at the fixed manifest path.
    NoSnapshot,
    /// A fixed artifact does not match the committed manifest.
    Inconsistent { path: String, reason: String },
    /// A pending journal exists; a writer must run recovery first.
    RecoveryRequired,
    /// Local state is structurally invalid (unparseable journal, missing staged bytes).
    Corrupt(String),
}

impl fmt::Display for SpikeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { context, source } => write!(f, "{context}: {source} (raw_os_error={:?})", source.raw_os_error()),
            Self::Crash(point) => write!(f, "injected crash at {point:?}"),
            Self::Locked => write!(f, "publication lock is held"),
            Self::NoSnapshot => write!(f, "no committed publication manifest"),
            Self::Inconsistent { path, reason } => write!(f, "inconsistent snapshot at {path}: {reason}"),
            Self::RecoveryRequired => write!(f, "pending publication journal; recovery required"),
            Self::Corrupt(msg) => write!(f, "corrupt publication state: {msg}"),
        }
    }
}

impl std::error::Error for SpikeError {}

pub(crate) fn io_err(context: impl Into<String>) -> impl FnOnce(io::Error) -> SpikeError {
    let context = context.into();
    move |source| SpikeError::Io { context, source }
}

/// Resolve a portable `/`-separated repo-relative path under `root`.
pub fn resolve(root: &std::path::Path, rel: &str) -> PathBuf {
    let mut out = root.to_path_buf();
    for segment in rel.split('/') {
        out.push(segment);
    }
    out
}
