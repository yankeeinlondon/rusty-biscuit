//! Legacy metadata format (v0) for migration support.
//!
//! This module provides the [`MetadataV0`] struct which represents the original
//! metadata format used before the introduction of [`ResearchDetails`].
//!
//! ## Schema Differences
//!
//! | Field | v0 | v1 |
//! |-------|----|----|
//! | `library_info` | `Option<LibraryInfoMetadata>` | Removed (moved to `details`) |
//! | `details` | Not present | `ResearchDetails` enum |
//! | `schema_version` | 0 (or missing) | 1 |

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{LibraryInfoMetadata, ResearchKind};

/// Legacy metadata format (schema_version: 0 or missing).
///
/// This struct preserves the original metadata structure to support
/// reading and migrating old `metadata.json` files.
///
/// ## Migration
///
/// When loading metadata, the system checks `schema_version`:
/// - `0` or missing: Parse as `MetadataV0`, then migrate to v1
/// - `1`: Parse directly as `ResearchMetadata`
///
/// ## Examples
///
/// ```
/// use research_lib::metadata::MetadataV0;
///
/// let json = r#"{
///     "kind": "library",
///     "library_info": {
///         "package_manager": "crates.io",
///         "language": "Rust",
///         "url": "https://crates.io/crates/serde"
///     },
///     "additional_files": {},
///     "created_at": "2024-01-01T00:00:00Z",
///     "updated_at": "2024-01-01T00:00:00Z"
/// }"#;
///
/// let v0: MetadataV0 = serde_json::from_str(json).unwrap();
/// assert_eq!(v0.schema_version, 0);
/// assert!(v0.library_info.is_some());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataV0 {
    /// Schema version (0 or missing for legacy format)
    #[serde(default)]
    pub schema_version: u32,

    /// The kind of research
    pub kind: ResearchKind,

    /// Information about the library (if kind is Library)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub library_info: Option<LibraryInfoMetadata>,

    /// Additional files created from user prompts (filename -> prompt)
    #[serde(default)]
    pub additional_files: HashMap<String, String>,

    /// When the research was first created
    pub created_at: DateTime<Utc>,

    /// When the research was last updated
    pub updated_at: DateTime<Utc>,

    /// Single-sentence summary
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brief: Option<String>,

    /// Paragraph summary
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,

    /// Guidance on when to use this research
    #[serde(skip_serializing_if = "Option::is_none")]
    pub when_to_use: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_v0_with_library_info() {
        // Note: ResearchKind uses #[serde(rename_all = "lowercase")]
        let json = r#"{
            "kind": "library",
            "library_info": {
                "package_manager": "crates.io",
                "language": "Rust",
                "url": "https://crates.io/crates/serde"
            },
            "additional_files": {},
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T00:00:00Z"
        }"#;

        let v0: MetadataV0 = serde_json::from_str(json).unwrap();
        assert_eq!(v0.schema_version, 0);
        assert!(matches!(v0.kind, ResearchKind::Library));

        let info = v0.library_info.unwrap();
        assert_eq!(info.package_manager, "crates.io");
        assert_eq!(info.language, "Rust");
    }

    #[test]
    fn test_v0_without_library_info() {
        let json = r#"{
            "kind": "library",
            "additional_files": {},
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T00:00:00Z"
        }"#;

        let v0: MetadataV0 = serde_json::from_str(json).unwrap();
        assert_eq!(v0.schema_version, 0);
        assert!(v0.library_info.is_none());
    }

    #[test]
    fn test_v0_with_explicit_schema_version() {
        let json = r#"{
            "schema_version": 0,
            "kind": "library",
            "additional_files": {},
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T00:00:00Z"
        }"#;

        let v0: MetadataV0 = serde_json::from_str(json).unwrap();
        assert_eq!(v0.schema_version, 0);
    }

    #[test]
    fn test_v0_with_optional_fields() {
        let json = r#"{
            "kind": "library",
            "additional_files": {"question_1.md": "How does it work?"},
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-02T00:00:00Z",
            "brief": "A serialization library",
            "summary": "Serde is a framework for serializing and deserializing Rust data structures.",
            "when_to_use": "Use when working with JSON, YAML, or other data formats"
        }"#;

        let v0: MetadataV0 = serde_json::from_str(json).unwrap();
        assert_eq!(v0.brief, Some("A serialization library".to_string()));
        assert!(v0.summary.is_some());
        assert!(v0.when_to_use.is_some());
        assert_eq!(v0.additional_files.len(), 1);
    }

    #[test]
    fn test_v0_with_repository() {
        let json = r#"{
            "kind": "library",
            "library_info": {
                "package_manager": "npm",
                "language": "JavaScript",
                "url": "https://www.npmjs.com/package/lodash",
                "repository": "https://github.com/lodash/lodash"
            },
            "additional_files": {},
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T00:00:00Z"
        }"#;

        let v0: MetadataV0 = serde_json::from_str(json).unwrap();
        let info = v0.library_info.unwrap();
        assert_eq!(
            info.repository,
            Some("https://github.com/lodash/lodash".to_string())
        );
    }
}
