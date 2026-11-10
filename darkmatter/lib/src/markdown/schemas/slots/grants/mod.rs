//! The grants that decide which reads a schema slot may make.
//!
//! A grant pairs a [`GrantDecision`] (allow or deny) with a [`GrantTarget`]
//! (a file, a folder, a URL, a URL whatever its query, or a whole site) and a
//! [`GrantScope`]: a `Once` grant lasts for one run, a `Persisted` grant is
//! read from the user's grant store.
//!
//! ## Grant store
//!
//! The store is one user-level file, `<home>/.config/darkmatter/schema-read-policy`
//! on every OS ([`GrantStore::path_for_home`]). Without a home there is no
//! store: nothing persisted applies and nothing can be persisted. It is UTF-8
//! text with one grant per line:
//!
//! ```text
//! # comments and blank lines are ignored
//! allow file /home/me/data/people.yaml
//! allow dir /home/me/schemas
//! deny dir /home/me/schemas/private
//! allow url https://example.com/types.yaml?v=2
//! allow url-any-query https://example.com/types.yaml
//! allow site https://schemas.example.com
//! ```
//!
//! A missing or empty file grants nothing. Any invalid line makes the whole
//! file grant nothing and is reported as a located
//! [`GrantStoreError`] (`dm.schema.slot_grant_store_invalid`): skipping the
//! line could silently drop a deny.
//!
//! ## Deny wins
//!
//! [`GrantBook::decide`] answers `Deny` when any deny grant, persisted or for
//! this run, covers the read, even when an allow grant covers it too. Only
//! then do allow grants (and the command's `--allow host=HOST` values) count.

pub(crate) mod auto;
pub(crate) mod gate;
mod matching;
mod request;
mod store;
pub(crate) mod targets;

use std::fmt;
use std::path::PathBuf;

pub use matching::GrantBook;
pub use request::{SlotGrantDecision, SlotGrantRequest};
pub use store::{
    GRANT_STORE_RELATIVE_PATH, GrantStore, GrantStoreError, GrantStoreErrorKind, GrantStoreWriteError,
};

/// What a grant covers.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GrantTarget {
    /// One file, by its absolute (canonical when it exists) path.
    File(PathBuf),
    /// A folder and everything below it, by its absolute (canonical when it
    /// exists) path.
    Dir(PathBuf),
    /// One URL, exactly, query included.
    Url(url::Url),
    /// A URL whatever its query; stored without one.
    UrlAnyQuery(url::Url),
    /// Every URL with this scheme, host, and port; no subdomains.
    Site {
        /// `http` or `https`.
        scheme: String,
        /// The host, as `url::Url` normalizes it (lowercase domain).
        host: String,
        /// `None` is the scheme's default port.
        port: Option<u16>,
    },
}

impl GrantTarget {
    /// The target a grant-store line spells as `kind` and `argument` (the
    /// [`Self::kind_word`] and [`Display`](fmt::Display) forms), judged as
    /// the store judges a line: an absolute path, an `http`/`https` URL, a
    /// `url-any-query` URL without a query, a site without a path.
    ///
    /// ## Errors
    ///
    /// The reason the words name no target, worded as a store-line error.
    pub fn parse(kind: &str, argument: &str) -> Result<Self, String> {
        store::target_from_words(kind, argument.trim())
    }

    /// The store's kind word for this target.
    pub fn kind_word(&self) -> &'static str {
        match self {
            Self::File(_) => "file",
            Self::Dir(_) => "dir",
            Self::Url(_) => "url",
            Self::UrlAnyQuery(_) => "url-any-query",
            Self::Site { .. } => "site",
        }
    }
}

/// The store-line argument form: a path as the host spells it, a URL as
/// `url::Url` serializes it, a site as `scheme://host[:port]`.
impl fmt::Display for GrantTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::File(path) | Self::Dir(path) => write!(f, "{}", path.display()),
            Self::Url(url) | Self::UrlAnyQuery(url) => f.write_str(url.as_str()),
            Self::Site { scheme, host, port: Some(port) } => write!(f, "{scheme}://{host}:{port}"),
            Self::Site { scheme, host, port: None } => write!(f, "{scheme}://{host}"),
        }
    }
}

/// How long a grant lasts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GrantScope {
    /// This run only.
    Once,
    /// Read from, or written to, the grant store.
    Persisted,
}

/// Whether a grant allows or denies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GrantDecision {
    /// The read may happen.
    Allow,
    /// The read may not happen, whatever else allows it.
    Deny,
}

impl GrantDecision {
    /// The store's verb for this decision.
    pub fn word(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Deny => "deny",
        }
    }
}

/// One grant.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Grant {
    /// Allow or deny.
    pub decision: GrantDecision,
    /// What it covers.
    pub target: GrantTarget,
    /// How long it lasts.
    pub scope: GrantScope,
}

#[cfg(test)]
mod tests;
