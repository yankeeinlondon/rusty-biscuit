//! Session-log document model shared by the daemon and clients.
//!
//! The session-log POC stores one Loro document per **chunk** of a
//! session. A chunk identifier is deterministic and follows the path
//! `session/{owner_node_id}/{session_id}/part/{chunk_index}` so callers
//! can reconstruct it without consulting the daemon.
//!
//! This module is intentionally provider-agnostic: it defines the
//! identity, entry schema, and chunk-rotation rules, but it does not
//! depend on Loro, redb, DuckDB, or any persistence layer.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Default cap on the number of entries stored in a single chunk before
/// the daemon rotates to a fresh chunk. Chosen small enough for tests to
/// trigger rotation deliberately without ballooning the suite runtime.
pub const DEFAULT_MAX_ENTRIES_PER_CHUNK: u64 = 64;

/// Default cap on the in-memory byte estimate (sum of `message` lengths
/// plus rough overhead) for a single chunk before rotation. The
/// estimator is deliberately coarse so tests can target it with simple
/// fixed-size messages.
pub const DEFAULT_MAX_BYTES_PER_CHUNK: u64 = 16 * 1024;

/// Deterministic identifier for one session-log chunk.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChunkId {
    /// Identifier of the node that owns the session.
    pub owner_node_id: String,
    /// Identifier of the session.
    pub session_id: String,
    /// Zero-based index of this chunk inside the session.
    pub chunk_index: u64,
}

impl ChunkId {
    /// Build a new chunk identifier.
    #[must_use]
    pub fn new(
        owner_node_id: impl Into<String>,
        session_id: impl Into<String>,
        chunk_index: u64,
    ) -> Self {
        Self {
            owner_node_id: owner_node_id.into(),
            session_id: session_id.into(),
            chunk_index,
        }
    }

    /// Identifier of the next chunk in the same session.
    #[must_use]
    pub fn next(&self) -> Self {
        Self {
            owner_node_id: self.owner_node_id.clone(),
            session_id: self.session_id.clone(),
            chunk_index: self.chunk_index + 1,
        }
    }

    /// Stable string key used as the redb key, gRPC string, and DuckDB
    /// column value: `session/{owner_node_id}/{session_id}/part/{idx}`.
    #[must_use]
    pub fn as_path(&self) -> String {
        format!(
            "session/{}/{}/part/{}",
            self.owner_node_id, self.session_id, self.chunk_index,
        )
    }

    /// Stable key for the `(owner_node_id, session_id)` pair, useful for
    /// listing chunks belonging to a single session.
    #[must_use]
    pub fn session_key(&self) -> String {
        format!("session/{}/{}", self.owner_node_id, self.session_id)
    }
}

impl fmt::Display for ChunkId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.as_path())
    }
}

/// Errors returned when parsing a chunk-id string fails.
#[derive(Debug, thiserror::Error)]
pub enum ChunkIdParseError {
    /// The string did not contain the expected number of `/`-separated
    /// segments or it lacked the leading `session` / interior `part`
    /// markers.
    #[error("malformed chunk id `{input}`: expected `session/<node>/<session>/part/<index>`")]
    Shape { input: String },

    /// The trailing chunk-index segment was not a valid unsigned integer.
    #[error("malformed chunk id `{input}`: chunk index `{segment}` is not a non-negative integer")]
    Index { input: String, segment: String },
}

impl FromStr for ChunkId {
    type Err = ChunkIdParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let segments: Vec<&str> = s.split('/').collect();
        if segments.len() != 5 || segments[0] != "session" || segments[3] != "part" {
            return Err(ChunkIdParseError::Shape {
                input: s.to_string(),
            });
        }
        let chunk_index = segments[4]
            .parse::<u64>()
            .map_err(|_| ChunkIdParseError::Index {
                input: s.to_string(),
                segment: segments[4].to_string(),
            })?;
        Ok(Self {
            owner_node_id: segments[1].to_string(),
            session_id: segments[2].to_string(),
            chunk_index,
        })
    }
}

/// Configuration controlling when the daemon rotates from one chunk to
/// the next. The thresholds are intentionally deterministic so tests can
/// trigger rotation precisely.
#[derive(Clone, Copy, Debug)]
pub struct ChunkConfig {
    /// Maximum number of entries a single chunk may hold.
    pub max_entries_per_chunk: u64,
    /// Maximum estimated byte size (sum of `message` lengths) a single
    /// chunk may hold.
    pub max_bytes_per_chunk: u64,
}

impl ChunkConfig {
    /// Build a config with explicit caps. Both caps must be non-zero;
    /// `0` falls back to the corresponding default.
    #[must_use]
    pub fn new(max_entries_per_chunk: u64, max_bytes_per_chunk: u64) -> Self {
        Self {
            max_entries_per_chunk: if max_entries_per_chunk == 0 {
                DEFAULT_MAX_ENTRIES_PER_CHUNK
            } else {
                max_entries_per_chunk
            },
            max_bytes_per_chunk: if max_bytes_per_chunk == 0 {
                DEFAULT_MAX_BYTES_PER_CHUNK
            } else {
                max_bytes_per_chunk
            },
        }
    }

    /// Returns `true` when the *next* append would push the chunk past
    /// either configured cap and the daemon should rotate first.
    #[must_use]
    pub fn should_rotate(&self, current_entries: u64, current_bytes: u64) -> bool {
        current_entries >= self.max_entries_per_chunk
            || current_bytes >= self.max_bytes_per_chunk
    }
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_ENTRIES_PER_CHUNK, DEFAULT_MAX_BYTES_PER_CHUNK)
    }
}

/// A single append-only entry in a session log chunk.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// Per-session monotonic counter assigned by the daemon when the
    /// entry is appended. Stable across restarts.
    pub sequence: u64,
    /// Wall-clock at append time, in milliseconds since the Unix epoch.
    pub created_at_unix_ms: i64,
    /// Free-form source identifier (e.g. `"claude"`, `"test-client"`).
    pub source: String,
    /// Severity / log level (e.g. `"info"`, `"warn"`).
    pub level: String,
    /// Human-readable message body.
    pub message: String,
    /// Optional structured metadata. Stored as a JSON value so the
    /// schema is intentionally generic.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl Entry {
    /// Coarse byte-size estimate used for chunk rotation. Counts the
    /// message length plus fixed-size overhead for the other fields and
    /// the metadata payload (when present).
    #[must_use]
    pub fn size_estimate(&self) -> u64 {
        let base = self.source.len() + self.level.len() + self.message.len() + 32;
        let meta = self
            .metadata
            .as_ref()
            .map(|m| serde_json::to_string(m).map(|s| s.len()).unwrap_or(0))
            .unwrap_or(0);
        (base + meta) as u64
    }
}

/// Deterministic per-chunk metadata stored alongside the entry list in
/// each Loro document. Phase 2 keeps this in-memory only; later phases
/// will persist a copy for restart/replay.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkMetadata {
    pub owner_node_id: String,
    pub session_id: String,
    pub chunk_index: u64,
    pub created_at_unix_ms: i64,
    /// Path of the previous chunk in this session, if any.
    pub previous_chunk_id: Option<String>,
}

impl ChunkMetadata {
    /// Build metadata for the very first chunk in a session.
    #[must_use]
    pub fn initial(
        owner_node_id: impl Into<String>,
        session_id: impl Into<String>,
        created_at_unix_ms: i64,
    ) -> Self {
        Self {
            owner_node_id: owner_node_id.into(),
            session_id: session_id.into(),
            chunk_index: 0,
            created_at_unix_ms,
            previous_chunk_id: None,
        }
    }

    /// Build metadata for the chunk that follows `previous`.
    #[must_use]
    pub fn rotated_from(previous: &ChunkMetadata, created_at_unix_ms: i64) -> Self {
        let previous_id = ChunkId::new(
            previous.owner_node_id.clone(),
            previous.session_id.clone(),
            previous.chunk_index,
        )
        .as_path();
        Self {
            owner_node_id: previous.owner_node_id.clone(),
            session_id: previous.session_id.clone(),
            chunk_index: previous.chunk_index + 1,
            created_at_unix_ms,
            previous_chunk_id: Some(previous_id),
        }
    }

    /// Convenience accessor for the deterministic chunk ID.
    #[must_use]
    pub fn chunk_id(&self) -> ChunkId {
        ChunkId::new(
            self.owner_node_id.clone(),
            self.session_id.clone(),
            self.chunk_index,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_id_round_trips_through_string() {
        let id = ChunkId::new("node-a", "session-7", 3);
        let path = id.as_path();
        assert_eq!(path, "session/node-a/session-7/part/3");
        let parsed: ChunkId = path.parse().expect("parse");
        assert_eq!(parsed, id);
    }

    #[test]
    fn chunk_id_session_key_is_stable() {
        let id = ChunkId::new("node-a", "session-7", 2);
        assert_eq!(id.session_key(), "session/node-a/session-7");
    }

    #[test]
    fn chunk_id_rejects_malformed_input() {
        assert!(matches!(
            "not-a-chunk".parse::<ChunkId>(),
            Err(ChunkIdParseError::Shape { .. })
        ));
        assert!(matches!(
            "session/a/b/wrong/3".parse::<ChunkId>(),
            Err(ChunkIdParseError::Shape { .. })
        ));
        assert!(matches!(
            "session/a/b/part/not-int".parse::<ChunkId>(),
            Err(ChunkIdParseError::Index { .. })
        ));
    }

    #[test]
    fn config_rotates_on_entry_or_byte_cap() {
        let cfg = ChunkConfig::new(4, 1024);
        assert!(!cfg.should_rotate(3, 256));
        assert!(cfg.should_rotate(4, 0));
        assert!(cfg.should_rotate(0, 1024));
    }

    #[test]
    fn config_substitutes_defaults_for_zero_caps() {
        let cfg = ChunkConfig::new(0, 0);
        assert_eq!(cfg.max_entries_per_chunk, DEFAULT_MAX_ENTRIES_PER_CHUNK);
        assert_eq!(cfg.max_bytes_per_chunk, DEFAULT_MAX_BYTES_PER_CHUNK);
    }

    #[test]
    fn entry_size_estimate_includes_metadata_payload() {
        let entry = Entry {
            sequence: 1,
            created_at_unix_ms: 0,
            source: "src".into(),
            level: "info".into(),
            message: "hello".into(),
            metadata: Some(serde_json::json!({"k": "v"})),
        };
        // 3 + 4 + 5 + 32 base + metadata len(~9) = 53
        assert!(entry.size_estimate() >= 53);
    }

    #[test]
    fn rotated_metadata_links_to_previous() {
        let first = ChunkMetadata::initial("node-a", "session-7", 100);
        let second = ChunkMetadata::rotated_from(&first, 200);
        assert_eq!(second.chunk_index, 1);
        assert_eq!(
            second.previous_chunk_id.as_deref(),
            Some("session/node-a/session-7/part/0"),
        );
    }
}
