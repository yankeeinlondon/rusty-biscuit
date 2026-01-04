//! Schema migration for research metadata.
//!
//! This module provides automatic migration from legacy metadata formats (v0)
//! to the current format (v1). Migration happens transparently during load.
//!
//! ## Migration Process
//!
//! 1. Read `metadata.json` and check `schema_version`
//! 2. If v0 (or missing), parse as [`MetadataV0`] and migrate
//! 3. Create backup at `metadata.v0.json.backup`
//! 4. Save migrated v1 format
//! 5. Return the migrated [`ResearchMetadata`]
//!
//! ## Schema Changes (v0 -> v1)
//!
//! | Field | v0 | v1 |
//! |-------|----|----|
//! | `schema_version` | 0 or missing | 1 |
//! | `library_info` | `Option<LibraryInfoMetadata>` | Removed |
//! | `details` | Not present | `ResearchDetails` enum |

use chrono::Utc;
use thiserror::Error;

use super::types::{ApiDetails, LibraryDetails, ResearchDetails};
use super::v0::MetadataV0;
use crate::{ResearchKind, ResearchMetadata};

/// Errors that can occur during metadata migration.
#[derive(Debug, Error)]
pub enum MigrationError {
    /// Failed to read metadata file.
    #[error("Failed to read metadata: {0}")]
    Read(#[from] std::io::Error),

    /// Failed to parse v0 metadata format.
    #[error("Failed to parse v0 metadata: {0}")]
    ParseV0(#[source] serde_json::Error),

    /// Failed to save migrated metadata.
    #[error("Failed to save migrated metadata: {0}")]
    Save(#[source] std::io::Error),

    /// Unknown schema version encountered.
    #[error("Unknown schema version: {version}")]
    UnknownVersion {
        /// The unrecognized schema version.
        version: u32,
    },
}

/// Migrate v0 metadata to v1 format.
///
/// This function converts the legacy `library_info` field into the new
/// `ResearchDetails` enum structure. The migration preserves all data
/// while updating the schema version.
///
/// ## Examples
///
/// ```
/// use research_lib::metadata::{MetadataV0, migration::migrate_v0_to_v1};
/// use research_lib::ResearchKind;
/// use chrono::Utc;
/// use std::collections::HashMap;
///
/// let v0 = MetadataV0 {
///     schema_version: 0,
///     kind: ResearchKind::Library,
///     library_info: None,
///     additional_files: HashMap::new(),
///     created_at: Utc::now(),
///     updated_at: Utc::now(),
///     brief: Some("A test library".to_string()),
///     summary: None,
///     when_to_use: None,
/// };
///
/// let v1 = migrate_v0_to_v1(v0);
/// assert_eq!(v1.schema_version, 1);
/// ```
///
/// ## Timestamp Handling
///
/// - `created_at`: Preserved from v0 (original creation time)
/// - `updated_at`: Set to current time (migration is an update)
#[must_use]
pub fn migrate_v0_to_v1(v0: MetadataV0) -> ResearchMetadata {
    let details = match v0.kind {
        ResearchKind::Library => {
            let lib_details = v0
                .library_info
                .map(|info| LibraryDetails {
                    package_manager: Some(info.package_manager),
                    language: Some(info.language),
                    url: Some(info.url),
                    repository: info.repository,
                })
                .unwrap_or_default();
            ResearchDetails::Library(lib_details)
        }
        ResearchKind::Api => {
            // Api is a new kind in v1, so any v0 Api data would be minimal
            ResearchDetails::Api(ApiDetails::default())
        } // Future kinds will have their own migration logic
    };

    ResearchMetadata {
        schema_version: 1,
        kind: v0.kind,
        details,
        additional_files: v0.additional_files,
        created_at: v0.created_at, // Preserve original creation time
        updated_at: Utc::now(),    // Migration counts as an update
        brief: v0.brief,
        summary: v0.summary,
        when_to_use: v0.when_to_use,
    }
}

/// Check if a JSON value represents a v0 schema.
///
/// Returns `true` if `schema_version` is 0 or missing.
#[must_use]
pub fn is_v0_schema(value: &serde_json::Value) -> bool {
    value
        .get("schema_version")
        .and_then(|v| v.as_u64())
        .unwrap_or(0)
        == 0
}

/// Get the schema version from a JSON value.
///
/// Returns 0 if `schema_version` is missing.
#[must_use]
pub fn get_schema_version(value: &serde_json::Value) -> u32 {
    value
        .get("schema_version")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LibraryInfoMetadata;
    use std::collections::HashMap;

    fn make_v0_with_library_info() -> MetadataV0 {
        MetadataV0 {
            schema_version: 0,
            kind: ResearchKind::Library,
            library_info: Some(LibraryInfoMetadata {
                package_manager: "crates.io".to_string(),
                language: "Rust".to_string(),
                url: "https://crates.io/crates/serde".to_string(),
                repository: Some("https://github.com/serde-rs/serde".to_string()),
            }),
            additional_files: {
                let mut map = HashMap::new();
                map.insert("question_1.md".to_string(), "How does it work?".to_string());
                map
            },
            created_at: Utc::now() - chrono::Duration::days(30),
            updated_at: Utc::now() - chrono::Duration::days(1),
            brief: Some("A serialization library".to_string()),
            summary: Some("Serde is a framework for serializing Rust data.".to_string()),
            when_to_use: Some("Use when working with JSON".to_string()),
        }
    }

    fn make_v0_minimal() -> MetadataV0 {
        MetadataV0 {
            schema_version: 0,
            kind: ResearchKind::Library,
            library_info: None,
            additional_files: HashMap::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            brief: None,
            summary: None,
            when_to_use: None,
        }
    }

    #[test]
    fn test_migrate_v0_to_v1_with_library_info() {
        let v0 = make_v0_with_library_info();
        let original_created_at = v0.created_at;

        let v1 = migrate_v0_to_v1(v0);

        // Schema version updated
        assert_eq!(v1.schema_version, 1);

        // Kind preserved
        assert!(matches!(v1.kind, ResearchKind::Library));

        // Library details migrated correctly
        let details = v1.library_details().expect("should have library details");
        assert_eq!(details.package_manager, Some("crates.io".to_string()));
        assert_eq!(details.language, Some("Rust".to_string()));
        assert_eq!(
            details.url,
            Some("https://crates.io/crates/serde".to_string())
        );
        assert_eq!(
            details.repository,
            Some("https://github.com/serde-rs/serde".to_string())
        );

        // Additional files preserved
        assert_eq!(v1.additional_files.len(), 1);
        assert_eq!(
            v1.additional_files.get("question_1.md"),
            Some(&"How does it work?".to_string())
        );

        // Timestamps handled correctly
        assert_eq!(v1.created_at, original_created_at); // Preserved
        assert!(v1.updated_at > original_created_at); // Updated due to migration

        // Optional fields preserved
        assert_eq!(v1.brief, Some("A serialization library".to_string()));
        assert!(v1.summary.is_some());
        assert!(v1.when_to_use.is_some());
    }

    #[test]
    fn test_migrate_v0_to_v1_minimal() {
        let v0 = make_v0_minimal();
        let v1 = migrate_v0_to_v1(v0);

        assert_eq!(v1.schema_version, 1);
        assert!(matches!(v1.kind, ResearchKind::Library));

        // Should have default library details
        let details = v1.library_details().expect("should have library details");
        assert_eq!(details.package_manager, None);
        assert_eq!(details.language, None);
        assert_eq!(details.url, None);
        assert_eq!(details.repository, None);

        // Empty collections
        assert!(v1.additional_files.is_empty());

        // No optional fields
        assert!(v1.brief.is_none());
        assert!(v1.summary.is_none());
        assert!(v1.when_to_use.is_none());
    }

    #[test]
    fn test_migrate_v0_null_library_info() {
        // Explicitly null library_info should result in default LibraryDetails
        let json = r#"{
            "kind": "library",
            "library_info": null,
            "additional_files": {},
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T00:00:00Z"
        }"#;

        let v0: MetadataV0 = serde_json::from_str(json).unwrap();
        assert!(v0.library_info.is_none());

        let v1 = migrate_v0_to_v1(v0);
        let details = v1.library_details().expect("should have library details");
        assert_eq!(details.package_manager, None);
        assert_eq!(details.language, None);
    }

    #[test]
    fn test_migrate_v0_missing_schema_version() {
        // Missing schema_version should default to 0
        let json = r#"{
            "kind": "library",
            "additional_files": {},
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T00:00:00Z"
        }"#;

        let v0: MetadataV0 = serde_json::from_str(json).unwrap();
        assert_eq!(v0.schema_version, 0); // Default value

        let v1 = migrate_v0_to_v1(v0);
        assert_eq!(v1.schema_version, 1);
    }

    #[test]
    fn test_is_v0_schema() {
        // Missing schema_version
        let json: serde_json::Value = serde_json::json!({
            "kind": "library"
        });
        assert!(is_v0_schema(&json));

        // Explicit 0
        let json: serde_json::Value = serde_json::json!({
            "schema_version": 0,
            "kind": "library"
        });
        assert!(is_v0_schema(&json));

        // Version 1
        let json: serde_json::Value = serde_json::json!({
            "schema_version": 1,
            "kind": "library"
        });
        assert!(!is_v0_schema(&json));
    }

    #[test]
    fn test_get_schema_version() {
        let json: serde_json::Value = serde_json::json!({});
        assert_eq!(get_schema_version(&json), 0);

        let json: serde_json::Value = serde_json::json!({"schema_version": 0});
        assert_eq!(get_schema_version(&json), 0);

        let json: serde_json::Value = serde_json::json!({"schema_version": 1});
        assert_eq!(get_schema_version(&json), 1);

        let json: serde_json::Value = serde_json::json!({"schema_version": 99});
        assert_eq!(get_schema_version(&json), 99);
    }

    #[test]
    fn test_migration_idempotency() {
        // Migrating the same v0 data should produce equivalent v1 results
        // (except for updated_at timestamp)
        let v0_a = make_v0_with_library_info();
        let v0_b = MetadataV0 {
            schema_version: v0_a.schema_version,
            kind: v0_a.kind.clone(),
            library_info: v0_a.library_info.clone(),
            additional_files: v0_a.additional_files.clone(),
            created_at: v0_a.created_at,
            updated_at: v0_a.updated_at,
            brief: v0_a.brief.clone(),
            summary: v0_a.summary.clone(),
            when_to_use: v0_a.when_to_use.clone(),
        };

        let v1_a = migrate_v0_to_v1(v0_a);
        let v1_b = migrate_v0_to_v1(v0_b);

        // All fields except updated_at should match
        assert_eq!(v1_a.schema_version, v1_b.schema_version);
        assert_eq!(v1_a.kind, v1_b.kind);
        assert_eq!(v1_a.details, v1_b.details);
        assert_eq!(v1_a.additional_files, v1_b.additional_files);
        assert_eq!(v1_a.created_at, v1_b.created_at);
        assert_eq!(v1_a.brief, v1_b.brief);
        assert_eq!(v1_a.summary, v1_b.summary);
        assert_eq!(v1_a.when_to_use, v1_b.when_to_use);
    }

    #[test]
    fn test_migration_preserves_created_at() {
        let original_time = Utc::now() - chrono::Duration::days(365);
        let v0 = MetadataV0 {
            schema_version: 0,
            kind: ResearchKind::Library,
            library_info: None,
            additional_files: HashMap::new(),
            created_at: original_time,
            updated_at: original_time,
            brief: None,
            summary: None,
            when_to_use: None,
        };

        let v1 = migrate_v0_to_v1(v0);

        // created_at must be preserved exactly
        assert_eq!(v1.created_at, original_time);
        // updated_at should be recent (migration time)
        assert!(v1.updated_at > original_time);
    }

    #[test]
    fn test_migration_error_display() {
        let err = MigrationError::UnknownVersion { version: 99 };
        assert_eq!(err.to_string(), "Unknown schema version: 99");

        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err = MigrationError::Read(io_err);
        assert!(err.to_string().contains("Failed to read metadata"));
    }
}
