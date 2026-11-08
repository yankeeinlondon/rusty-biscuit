//! The read gate a schema slot's evaluation consults for every read.
//!
//! A slot evaluates on the ordinary expression engine, but only the reads
//! its approvals allow may reach it. The slot evaluator sets a
//! [`ReadGate`] on the [`ResolutionContext`](super::ResolutionContext) it
//! evaluates in; no other surface sets one, so document evaluation is
//! unchanged.
//!
//! The gate is asked twice over:
//!
//! - **before** a content read or a fetch, at the read site, and before the
//!   shared `load_data` memo is consulted, so a forbidden file is never read
//!   for the slot and a file the document already loaded is not handed to it
//!   from the memo;
//! - **after** any input-reading function returns, for every input it
//!   observed (a probed candidate, a listing, a contract import, a schema
//!   file `validate_schema` read), before its result can reach the slot.
//!
//! A refusal is [`ExpressionError::SlotReadRefused`](super::ExpressionError::SlotReadRefused),
//! which no recovery (`try_frontmatter`, `||`) turns into a value.

use std::fmt;
use std::path::PathBuf;

use super::data::{Observation, ObservationKind};
use super::functions::ReadKind;

/// Decides whether a slot may read a target.
pub(crate) trait ReadGate: Send + Sync + fmt::Debug {
    /// `Ok` lets the read happen (or its result reach the slot).
    ///
    /// ## Errors
    ///
    /// Why the read is refused.
    fn check(&self, target: &ReadTarget) -> Result<(), GateRefusal>;
}

/// What a read names.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ReadLocation {
    /// A local file or folder.
    Local(PathBuf),
    /// An HTTP(S) URL.
    Remote(url::Url),
    /// A glob pattern, as written (`find_files`).
    Glob(String),
}

impl fmt::Display for ReadLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Local(path) => f.write_str(&biscuit_file::to_portable_string(path)),
            Self::Remote(url) => f.write_str(url.as_str()),
            Self::Glob(pattern) => f.write_str(pattern),
        }
    }
}

/// One read a slot's evaluation is about to make, or made.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ReadTarget {
    /// The function reading.
    pub function: String,
    /// Content, an existence probe, or a listing.
    pub kind: ReadKind,
    /// What is read.
    pub location: ReadLocation,
}

impl ReadTarget {
    /// The target an observation names. A file or URL observation is a
    /// content read, an existence answer or missing probe is a probe, and a
    /// listing is a listing.
    pub(crate) fn from_observation(function: &str, observation: &Observation) -> Option<Self> {
        let (kind, location) = match observation.kind {
            ObservationKind::File => (ReadKind::Content, ReadLocation::Local(PathBuf::from(&observation.entry))),
            ObservationKind::Url => (ReadKind::Content, ReadLocation::Remote(url::Url::parse(&observation.entry).ok()?)),
            ObservationKind::Exists | ObservationKind::MissingProbe => {
                (ReadKind::Probe, ReadLocation::Local(PathBuf::from(&observation.entry)))
            }
            ObservationKind::Listing => (ReadKind::Listing, ReadLocation::Local(PathBuf::from(&observation.entry))),
            ObservationKind::Glob => (ReadKind::Listing, ReadLocation::Glob(observation.entry.clone())),
        };
        Some(Self { function: function.to_string(), kind, location })
    }
}

impl fmt::Display for ReadTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let what = match self.kind {
            ReadKind::Content => "read",
            ReadKind::Probe => "probe",
            ReadKind::Listing => "list",
        };
        write!(f, "{}() may not {what} {}", self.function, self.location)
    }
}

/// Why a gate refused a read.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum GateRefusal {
    /// A deny decision covers the target.
    Denied,
    /// No grant covers the target, and none was given.
    ApprovalRequired,
}

impl fmt::Display for GateRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Denied => "the read is denied",
            Self::ApprovalRequired => "the read needs approval",
        })
    }
}
