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

/// Domain prefix for checked-out-repos registers (Kind-2, one per
/// host): canonical repo id → HEAD commit hash. Kept separate from the
/// capability register because its cadence (every commit) is far
/// hotter than the cold hardware fields.
pub const REPOS_DOMAIN: &str = "repos";

/// Domain prefix for active-session registers (Kind-2, one per host):
/// session id → JSON entry describing a live session. Written on
/// session *transitions* (start / status change / end — never
/// heartbeats), read mesh-wide by the dashboard's NOW view.
pub const SESSIONS_ACTIVE_DOMAIN: &str = "sessions-active";

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
    /// A checked-out-repos register: `repos/{node_id}`.
    Repos {
        /// Node whose checkouts the register describes; also the
        /// document's single writer.
        owner_node_id: String,
    },
    /// An active-sessions register: `sessions-active/{node_id}`.
    SessionsActive {
        /// Node whose live sessions the register describes; also the
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

    /// Repos register id for `owner_node_id`.
    #[must_use]
    pub fn repos(owner_node_id: impl Into<String>) -> Self {
        Self::Repos {
            owner_node_id: owner_node_id.into(),
        }
    }

    /// Active-sessions register id for `owner_node_id`.
    #[must_use]
    pub fn sessions_active(owner_node_id: impl Into<String>) -> Self {
        Self::SessionsActive {
            owner_node_id: owner_node_id.into(),
        }
    }

    /// The node allowed to write this document.
    #[must_use]
    pub fn owner_node_id(&self) -> &str {
        match self {
            Self::SessionChunk(chunk) => &chunk.owner_node_id,
            Self::Capability { owner_node_id }
            | Self::Repos { owner_node_id }
            | Self::SessionsActive { owner_node_id } => owner_node_id,
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
            Self::Repos { owner_node_id } => format!("{REPOS_DOMAIN}/{owner_node_id}"),
            Self::SessionsActive { owner_node_id } => {
                format!("{SESSIONS_ACTIVE_DOMAIN}/{owner_node_id}")
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

    /// A register path did not have exactly `{domain}/{node_id}`.
    #[error("malformed register id `{input}`: expected `{domain}/<node_id>`")]
    RegisterShape { domain: &'static str, input: String },
}

impl FromStr for DocumentId {
    type Err = DocumentIdParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        fn register_owner(
            s: &str,
            domain: &'static str,
        ) -> Result<String, DocumentIdParseError> {
            let segments: Vec<&str> = s.split('/').collect();
            if segments.len() != 2 || segments[1].is_empty() {
                return Err(DocumentIdParseError::RegisterShape {
                    domain,
                    input: s.to_string(),
                });
            }
            Ok(segments[1].to_string())
        }

        let domain = s.split('/').next().unwrap_or_default();
        match domain {
            SESSION_DOMAIN => Ok(Self::SessionChunk(s.parse::<ChunkId>()?)),
            CAPABILITY_DOMAIN => Ok(Self::Capability {
                owner_node_id: register_owner(s, CAPABILITY_DOMAIN)?,
            }),
            REPOS_DOMAIN => Ok(Self::Repos {
                owner_node_id: register_owner(s, REPOS_DOMAIN)?,
            }),
            SESSIONS_ACTIVE_DOMAIN => Ok(Self::SessionsActive {
                owner_node_id: register_owner(s, SESSIONS_ACTIVE_DOMAIN)?,
            }),
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
    fn repos_round_trips() {
        let id: DocumentId = "repos/node-d".parse().expect("parse");
        assert_eq!(id, DocumentId::repos("node-d"));
        assert_eq!(id.as_path(), "repos/node-d");
        assert_eq!(id.owner_node_id(), "node-d");
    }

    #[test]
    fn sessions_active_round_trips() {
        let id: DocumentId = "sessions-active/node-e".parse().expect("parse");
        assert_eq!(id, DocumentId::sessions_active("node-e"));
        assert_eq!(id.as_path(), "sessions-active/node-e");
        assert_eq!(id.owner_node_id(), "node-e");
    }

    #[test]
    fn malformed_known_domains_are_errors() {
        assert!(matches!(
            "capability/".parse::<DocumentId>(),
            Err(DocumentIdParseError::RegisterShape { .. })
        ));
        assert!(matches!(
            "capability/node/extra".parse::<DocumentId>(),
            Err(DocumentIdParseError::RegisterShape { .. })
        ));
        assert!(matches!(
            "repos/node/extra".parse::<DocumentId>(),
            Err(DocumentIdParseError::RegisterShape { .. })
        ));
        assert!(matches!(
            "session/only-two/segments".parse::<DocumentId>(),
            Err(DocumentIdParseError::Chunk(_))
        ));
    }
}
