//! Error type for the side-effect engine.

use std::path::PathBuf;
use thiserror::Error;

/// Errors raised by [`crate::effects::EffectEngine`] operations.
#[derive(Debug, Error)]
pub enum EffectError {
    #[error("invalid file path: {0}")]
    InvalidFilePath(String),

    #[error("invalid state key {0:?}: keys must be a single top-level name (no empty or dotted-path keys)")]
    InvalidKey(String),

    #[error("refusing to write outside the mutation root: {} (root: {})", path.display(), root.display())]
    OutsideMutationRoot { path: PathBuf, root: PathBuf },

    #[error("network host not allowed: {0}")]
    HostNotAllowed(String),

    #[error("invalid URL: {0}")]
    InvalidUrl(String),

    #[error("network request failed: {0}")]
    Network(String),

    #[error("frontmatter property {prop:?} has the wrong type for {op}")]
    PropertyType { op: &'static str, prop: String },

    #[error("io error for {}: {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("markdown error: {0}")]
    Markdown(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_variants_render_messages() {
        let e = EffectError::OutsideMutationRoot {
            path: "/etc/passwd".into(),
            root: "/repo".into(),
        };
        let msg = e.to_string();
        assert!(msg.contains("/etc/passwd"));
        assert!(msg.contains("/repo"));
    }
}
