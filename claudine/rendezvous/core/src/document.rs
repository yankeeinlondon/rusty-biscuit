//! Multi-domain document identity.
//!
//! The mesh synchronizes more than session-log chunks. Every synced
//! Loro document follows the ratified addressing grammar
//! (`rendezvous/docs/crdt.md`):
//!
//! ```text
//! {domain}/{owner_node_id}[/{entity_id}][/part/{chunk_index}]
//! ```
//!
//! [`DocumentId`] is the typed form of that grammar. The embedded
//! `owner_node_id` is the single-writer declaration: only that node may
//! produce ops for the document, and receivers enforce it. Unknown
//! domains parse to [`DocumentIdParseError::UnknownDomain`] so a newer
//! peer's documents can be skipped (not fatal) by an older daemon.

use std::fmt;
use std::str::FromStr;

use crate::session_log::{ChunkId, ChunkIdParseError};

/// Domain prefix for session-log chunk documents (Kind-1 fact logs).
pub const SESSION_DOMAIN: &str = "session";

/// Domain prefix for host-capability registers (Kind-2 state
/// registers, one per host).
pub const CAPABILITY_DOMAIN: &str = "capability";

/// Typed identity of one synced document.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum DocumentId {
    /// A session-log chunk: `session/{node}/{session}/part/{idx}`.
    SessionChunk(ChunkId),
    /// A host-capability register: `capability/{node_id}`.
    Capability {
        /// Node whose capabilities the register describes; also the
        /// document's single writer.
        owner_node_id: String,
    },
}

impl DocumentId {
    /// Capability register id for `owner_node_id`.
    #[must_use]
    pub fn capability(owner_node_id: impl Into<String>) -> Self {
        Self::Capability {
            owner_node_id: owner_node_id.into(),
        }
    }

    /// The node allowed to write this document.
    #[must_use]
    pub fn owner_node_id(&self) -> &str {
        match self {
            Self::SessionChunk(chunk) => &chunk.owner_node_id,
            Self::Capability { owner_node_id } => owner_node_id,
        }
    }

    /// Stable string form used as the redb key, wire identifier, and
    /// DuckDB column value.
    #[must_use]
    pub fn as_path(&self) -> String {
        match self {
            Self::SessionChunk(chunk) => chunk.as_path(),
            Self::Capability { owner_node_id } => {
                format!("{CAPABILITY_DOMAIN}/{owner_node_id}")
            }
        }
    }
}

impl fmt::Display for DocumentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.as_path())
    }
}

impl From<ChunkId> for DocumentId {
    fn from(chunk: ChunkId) -> Self {
        Self::SessionChunk(chunk)
    }
}

/// Errors returned when parsing a document path fails.
#[derive(Debug, thiserror::Error)]
pub enum DocumentIdParseError {
    /// The leading path segment names a domain this daemon does not
    /// know. Receivers should skip such documents (a newer peer may
    /// legitimately sync domains we have not learned yet) rather than
    /// failing the session.
    #[error("unknown document domain `{domain}` in `{input}`")]
    UnknownDomain { domain: String, input: String },

    /// The path claimed a known domain but did not match its shape.
    #[error(transparent)]
    Chunk(#[from] ChunkIdParseError),

    /// A capability path did not have exactly `capability/{node_id}`.
    #[error("malformed capability id `{input}`: expected `capability/<node_id>`")]
    CapabilityShape { input: String },
}

impl FromStr for DocumentId {
    type Err = DocumentIdParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let domain = s.split('/').next().unwrap_or_default();
        match domain {
            SESSION_DOMAIN => Ok(Self::SessionChunk(s.parse::<ChunkId>()?)),
            CAPABILITY_DOMAIN => {
                let segments: Vec<&str> = s.split('/').collect();
                if segments.len() != 2 || segments[1].is_empty() {
                    return Err(DocumentIdParseError::CapabilityShape {
                        input: s.to_string(),
                    });
                }
                Ok(Self::Capability {
                    owner_node_id: segments[1].to_string(),
                })
            }
            other => Err(DocumentIdParseError::UnknownDomain {
                domain: other.to_string(),
                input: s.to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_chunk_round_trips() {
        let id: DocumentId = "session/node-a/s1/part/2".parse().expect("parse");
        assert_eq!(
            id,
            DocumentId::SessionChunk(ChunkId::new("node-a", "s1", 2)),
        );
        assert_eq!(id.as_path(), "session/node-a/s1/part/2");
        assert_eq!(id.owner_node_id(), "node-a");
    }

    #[test]
    fn capability_round_trips() {
        let id: DocumentId = "capability/node-b".parse().expect("parse");
        assert_eq!(id, DocumentId::capability("node-b"));
        assert_eq!(id.as_path(), "capability/node-b");
        assert_eq!(id.owner_node_id(), "node-b");
    }

    #[test]
    fn unknown_domain_is_distinguishable() {
        let err = "presence-log/node-c/part/0".parse::<DocumentId>().unwrap_err();
        assert!(matches!(
            err,
            DocumentIdParseError::UnknownDomain { ref domain, .. } if domain == "presence-log"
        ));
    }

    #[test]
    fn malformed_known_domains_are_errors() {
        assert!(matches!(
            "capability/".parse::<DocumentId>(),
            Err(DocumentIdParseError::CapabilityShape { .. })
        ));
        assert!(matches!(
            "capability/node/extra".parse::<DocumentId>(),
            Err(DocumentIdParseError::CapabilityShape { .. })
        ));
        assert!(matches!(
            "session/only-two/segments".parse::<DocumentId>(),
            Err(DocumentIdParseError::Chunk(_))
        ));
    }
}
