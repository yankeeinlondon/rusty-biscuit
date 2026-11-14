//! The approvals that decide which reads a schema slot may make, beyond
//! what the captured configuration pre-approves.
//!
//! A local read is pre-approved when the request's `permissions.file-read`
//! admits the file ([`PermissionPolicy::file_read`]), and a URL read when the
//! effective network permission or the command's `--allow host=HOST` allows
//! it. Anything else needs an approval for this run: a [`Grant`] pairing a
//! [`GrantDecision`] (allow, or deny for this run) with a [`GrantTarget`] (one
//! file or one exact URL). An "always allow" answer for a file
//! ([`GrantScope::Persisted`]) is saved to the user's `permissions.file-read`
//! as an exact entry and also applies for the rest of the run; the captured
//! configuration is never reloaded mid-run.
//!
//! Nothing is denied persistently, and no approval covers a folder or a
//! site: a listing needs no approval of its own (it returns only the files
//! the slot may read without asking), and blanket URL consent is the
//! configured `network: allow`.
//!
//! [`PermissionPolicy::file_read`]: crate::config::permissions::PermissionPolicy::file_read

pub(crate) mod gate;
mod legacy;
mod matching;
mod request;
pub(crate) mod targets;

use std::fmt;
use std::path::PathBuf;

pub use legacy::GRANT_STORE_RELATIVE_PATH;
pub use matching::GrantBook;
pub use request::{SlotGrantDecision, SlotGrantRequest};

/// What an approval covers.
///
/// Only [`Self::File`] and [`Self::Url`] are ever offered or applied.
/// [`Self::Dir`], [`Self::UrlAnyQuery`], and [`Self::Site`] are the retired
/// grant store's line forms, parsed only so the one-time migration into
/// `permissions.file-read` can convert or report them; an approval naming
/// one is ignored.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GrantTarget {
    /// One file, by its absolute (canonical when it exists) path.
    File(PathBuf),
    /// A retired store form: a folder and everything below it.
    Dir(PathBuf),
    /// One URL, exactly, query included.
    Url(url::Url),
    /// A retired store form: a URL whatever its query.
    UrlAnyQuery(url::Url),
    /// A retired store form: every URL with this scheme, host, and port.
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
    /// The target the kind word and argument name (`file /abs/path`,
    /// `url https://…`, and the retired `dir`, `url-any-query`, and `site`
    /// forms), judged as the retired store judged a line: an absolute path,
    /// an `http`/`https` URL, a `url-any-query` URL without a query, a site
    /// without a path.
    ///
    /// ## Errors
    ///
    /// The reason the words name no target.
    pub fn parse(kind: &str, argument: &str) -> Result<Self, String> {
        legacy::target_from_words(kind, argument.trim())
    }

    /// The kind word for this target.
    pub fn kind_word(&self) -> &'static str {
        match self {
            Self::File(_) => "file",
            Self::Dir(_) => "dir",
            Self::Url(_) => "url",
            Self::UrlAnyQuery(_) => "url-any-query",
            Self::Site { .. } => "site",
        }
    }

    /// Whether an approval may name this target: one file or one exact URL.
    pub fn is_approvable(&self) -> bool {
        matches!(self, Self::File(_) | Self::Url(_))
    }
}

/// The argument form: a path as the host spells it, a URL as `url::Url`
/// serializes it, a site as `scheme://host[:port]`.
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

/// How long an approval lasts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GrantScope {
    /// This run only.
    Once,
    /// "Always allow" a file: saved to the user's `permissions.file-read` as
    /// an exact entry, and applied for the rest of this run whether or not
    /// the save succeeded. Never a deny, and never a URL.
    Persisted,
}

/// Whether an approval allows or denies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GrantDecision {
    /// The read may happen.
    Allow,
    /// The read may not happen for the rest of this run.
    Deny,
}

impl GrantDecision {
    /// The decision's word (`allow`, `deny`).
    pub fn word(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Deny => "deny",
        }
    }
}

/// One approval.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Grant {
    /// Allow or deny.
    pub decision: GrantDecision,
    /// What it covers.
    pub target: GrantTarget,
    /// How long it lasts.
    pub scope: GrantScope,
}

impl Grant {
    /// Whether this approval can be applied: an allow or deny of one file or
    /// one exact URL, with only a file allow ever persisted.
    pub fn is_applicable(&self) -> bool {
        self.target.is_approvable()
            && match self.scope {
                GrantScope::Once => true,
                GrantScope::Persisted => {
                    self.decision == GrantDecision::Allow && matches!(self.target, GrantTarget::File(_))
                }
            }
    }
}

#[cfg(test)]
mod tests;
