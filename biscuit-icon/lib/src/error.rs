//! Library error type.

use thiserror::Error;

/// Errors produced by `biscuit-icon`.
#[derive(Debug, Error)]
pub enum IconError {
    /// A string did not match any variant of the named domain set.
    #[error("unknown {set} icon: {name}")]
    UnknownDomainIcon {
        /// The domain set name, e.g. "os".
        set: &'static str,
        /// The unmatched icon name.
        name: String,
    },

    /// An Iconify identifier was not in `prefix:name` form.
    #[error("invalid iconify identifier: {0}")]
    InvalidIdentifier(String),

    /// The requested Iconify icon was not found upstream.
    #[error("iconify icon not found: {0}")]
    NotFound(String),

    /// A network request to the Iconify API failed.
    #[error("iconify fetch failed: {0}")]
    Fetch(String),

    /// A cache (SQLite) operation failed.
    #[error("cache error: {0}")]
    Cache(String),
}

/// Convenience result alias.
pub type Result<T> = std::result::Result<T, IconError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_domain_icon_renders_set_and_name() {
        let err = IconError::UnknownDomainIcon { set: "os", name: "frobnicate".into() };
        assert_eq!(err.to_string(), "unknown os icon: frobnicate");
    }

    #[test]
    fn invalid_identifier_renders_input() {
        let err = IconError::InvalidIdentifier("mdihome".into());
        assert_eq!(err.to_string(), "invalid iconify identifier: mdihome");
    }
}
