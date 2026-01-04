//! Integration tests for metadata migration (v0 -> v1).
//!
//! This test suite verifies end-to-end functionality of the migration system,
//! including loading v0 files, creating backups, and auto-saving migrated data.

use research_lib::metadata::{migration, MetadataV0};
use research_lib::{ResearchKind, ResearchMetadata};
use std::path::PathBuf;
use tempfile::TempDir;
use tokio::fs;

/// Returns the path to the test fixtures directory.
fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// Helper to read fixture content as a string.
fn read_fixture(name: &str) -> String {
    let path = fixtures_dir().join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("Failed to read fixture {name}: {e}"))
}

// =============================================================================
// Migration Function Tests
// =============================================================================

#[test]
fn test_migrate_complete_v0_fixture() {
    let content = read_fixture("metadata_v0_complete.json");
    let v0: MetadataV0 = serde_json::from_str(&content).expect("Failed to parse v0 fixture");

    assert_eq!(v0.schema_version, 0);
    assert!(v0.library_info.is_some());

    let v1 = migration::migrate_v0_to_v1(v0);

    // Schema version upgraded
    assert_eq!(v1.schema_version, 1);

    // Kind preserved
    assert!(matches!(v1.kind, ResearchKind::Library));

    // Library details migrated
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
    assert_eq!(v1.additional_files.len(), 2);
    assert!(v1.additional_files.contains_key("question_1.md"));
    assert!(v1.additional_files.contains_key("question_2.md"));

    // Optional fields preserved
    assert!(v1.brief.is_some());
    assert!(v1.summary.is_some());
    assert!(v1.when_to_use.is_some());
}

#[test]
fn test_migrate_minimal_v0_fixture() {
    let content = read_fixture("metadata_v0_minimal.json");
    let v0: MetadataV0 = serde_json::from_str(&content).expect("Failed to parse v0 fixture");

    assert_eq!(v0.schema_version, 0);
    assert!(v0.library_info.is_none());

    let v1 = migration::migrate_v0_to_v1(v0);

    // Schema version upgraded
    assert_eq!(v1.schema_version, 1);

    // Should have default library details
    let details = v1.library_details().expect("should have library details");
    assert!(details.package_manager.is_none());
    assert!(details.language.is_none());

    // Empty collections
    assert!(v1.additional_files.is_empty());
}

#[test]
fn test_migrate_missing_schema_version_fixture() {
    let content = read_fixture("metadata_v0_missing_schema.json");

    // Verify the JSON doesn't have schema_version
    let value: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(value.get("schema_version").is_none());

    // Should still parse as v0 (default to 0)
    let v0: MetadataV0 = serde_json::from_str(&content).expect("Failed to parse v0 fixture");
    assert_eq!(v0.schema_version, 0); // Default value

    let v1 = migration::migrate_v0_to_v1(v0);
    assert_eq!(v1.schema_version, 1);

    // Library info should be migrated
    let details = v1.library_details().expect("should have library details");
    assert_eq!(details.package_manager, Some("npm".to_string()));
    assert_eq!(details.language, Some("JavaScript".to_string()));
}

#[test]
fn test_migrate_null_library_info_fixture() {
    let content = read_fixture("metadata_v0_null_library_info.json");
    let v0: MetadataV0 = serde_json::from_str(&content).expect("Failed to parse v0 fixture");

    // Explicitly null library_info
    assert!(v0.library_info.is_none());

    let v1 = migration::migrate_v0_to_v1(v0);

    // Should have default library details
    let details = v1.library_details().expect("should have library details");
    assert!(details.package_manager.is_none());
    assert!(details.language.is_none());
    assert!(details.url.is_none());
    assert!(details.repository.is_none());

    // Additional files preserved
    assert_eq!(v1.additional_files.len(), 1);
}

#[test]
fn test_malformed_json_handling() {
    let malformed = r#"{ "kind": "library", "broken }"#;
    let result: Result<MetadataV0, _> = serde_json::from_str(malformed);
    assert!(result.is_err());
}

// =============================================================================
// Schema Version Detection Tests
// =============================================================================

#[test]
fn test_is_v0_schema_with_fixtures() {
    // Complete v0 fixture
    let content = read_fixture("metadata_v0_complete.json");
    let value: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(migration::is_v0_schema(&value));

    // Missing schema version fixture
    let content = read_fixture("metadata_v0_missing_schema.json");
    let value: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(migration::is_v0_schema(&value));
}

#[test]
fn test_get_schema_version_with_fixtures() {
    // Explicit version 0
    let content = read_fixture("metadata_v0_complete.json");
    let value: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(migration::get_schema_version(&value), 0);

    // Missing version (defaults to 0)
    let content = read_fixture("metadata_v0_missing_schema.json");
    let value: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(migration::get_schema_version(&value), 0);
}

// =============================================================================
// Timestamp Handling Tests
// =============================================================================

#[test]
fn test_timestamp_preservation() {
    let content = read_fixture("metadata_v0_complete.json");
    let v0: MetadataV0 = serde_json::from_str(&content).unwrap();

    let original_created_at = v0.created_at;
    let original_updated_at = v0.updated_at;

    let v1 = migration::migrate_v0_to_v1(v0);

    // created_at must be preserved exactly
    assert_eq!(v1.created_at, original_created_at);

    // updated_at should be updated (migration time > original)
    assert!(v1.updated_at > original_updated_at);
}

// =============================================================================
// Idempotency Tests
// =============================================================================

#[test]
fn test_migration_idempotency_with_fixtures() {
    // Migrate the same fixture twice
    let content = read_fixture("metadata_v0_complete.json");
    let v0_a: MetadataV0 = serde_json::from_str(&content).unwrap();
    let v0_b: MetadataV0 = serde_json::from_str(&content).unwrap();

    let v1_a = migration::migrate_v0_to_v1(v0_a);
    let v1_b = migration::migrate_v0_to_v1(v0_b);

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

// =============================================================================
// ResearchMetadata::load() Integration Tests
// =============================================================================

#[tokio::test]
async fn test_load_v0_creates_backup() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let output_dir = temp_dir.path();

    // Copy v0 fixture to temp directory
    let v0_content = read_fixture("metadata_v0_complete.json");
    let metadata_path = output_dir.join("metadata.json");
    fs::write(&metadata_path, &v0_content)
        .await
        .expect("Failed to write v0 fixture");

    // Load should trigger migration
    let metadata = ResearchMetadata::load(output_dir)
        .await
        .expect("Failed to load metadata");

    // Verify migration occurred
    assert_eq!(metadata.schema_version, 1);

    // Verify backup was created
    let backup_path = output_dir.join("metadata.v0.json.backup");
    assert!(backup_path.exists(), "Backup file should exist");

    // Verify backup contains original v0 content
    let backup_content = fs::read_to_string(&backup_path)
        .await
        .expect("Failed to read backup");
    assert_eq!(backup_content, v0_content);
}

#[tokio::test]
async fn test_load_v0_auto_saves_v1() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let output_dir = temp_dir.path();

    // Copy v0 fixture to temp directory
    let v0_content = read_fixture("metadata_v0_complete.json");
    let metadata_path = output_dir.join("metadata.json");
    fs::write(&metadata_path, &v0_content)
        .await
        .expect("Failed to write v0 fixture");

    // Load should trigger migration and auto-save
    let _metadata = ResearchMetadata::load(output_dir)
        .await
        .expect("Failed to load metadata");

    // Re-read the metadata.json file
    let saved_content = fs::read_to_string(&metadata_path)
        .await
        .expect("Failed to read saved metadata");
    let saved_value: serde_json::Value =
        serde_json::from_str(&saved_content).expect("Failed to parse saved metadata");

    // Should now be v1 format
    assert_eq!(
        saved_value.get("schema_version").and_then(|v| v.as_u64()),
        Some(1)
    );

    // Should have details field instead of library_info
    assert!(saved_value.get("details").is_some());
    assert!(saved_value.get("library_info").is_none());
}

#[tokio::test]
async fn test_load_v1_no_migration() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let output_dir = temp_dir.path();

    // Create a v1 metadata file directly
    let v1_json = r#"{
        "schema_version": 1,
        "kind": "library",
        "details": {
            "type": "Library",
            "package_manager": "crates.io",
            "language": "Rust"
        },
        "additional_files": {},
        "created_at": "2024-01-01T00:00:00Z",
        "updated_at": "2024-01-01T00:00:00Z"
    }"#;

    let metadata_path = output_dir.join("metadata.json");
    fs::write(&metadata_path, v1_json)
        .await
        .expect("Failed to write v1 fixture");

    // Load should NOT create backup (already v1)
    let metadata = ResearchMetadata::load(output_dir)
        .await
        .expect("Failed to load metadata");

    assert_eq!(metadata.schema_version, 1);

    // No backup should be created
    let backup_path = output_dir.join("metadata.v0.json.backup");
    assert!(!backup_path.exists(), "Backup should not exist for v1 files");
}

#[tokio::test]
async fn test_load_missing_schema_version_triggers_migration() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let output_dir = temp_dir.path();

    // Copy fixture with missing schema_version
    let v0_content = read_fixture("metadata_v0_missing_schema.json");
    let metadata_path = output_dir.join("metadata.json");
    fs::write(&metadata_path, &v0_content)
        .await
        .expect("Failed to write v0 fixture");

    // Load should trigger migration
    let metadata = ResearchMetadata::load(output_dir)
        .await
        .expect("Failed to load metadata");

    assert_eq!(metadata.schema_version, 1);

    // Backup should be created
    let backup_path = output_dir.join("metadata.v0.json.backup");
    assert!(backup_path.exists(), "Backup file should exist");
}

#[tokio::test]
async fn test_load_nonexistent_returns_none() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let output_dir = temp_dir.path();

    // No metadata.json file exists
    let result = ResearchMetadata::load(output_dir).await;
    assert!(result.is_none());
}

#[tokio::test]
async fn test_load_malformed_json_returns_none() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let output_dir = temp_dir.path();

    // Write malformed JSON
    let metadata_path = output_dir.join("metadata.json");
    fs::write(&metadata_path, r#"{ "broken json"#)
        .await
        .expect("Failed to write");

    let result = ResearchMetadata::load(output_dir).await;
    assert!(result.is_none());
}

// =============================================================================
// Migration Produces Valid V1 Format
// =============================================================================

#[test]
fn test_migrated_v1_serializes_correctly() {
    let content = read_fixture("metadata_v0_complete.json");
    let v0: MetadataV0 = serde_json::from_str(&content).unwrap();
    let v1 = migration::migrate_v0_to_v1(v0);

    // Serialize to JSON
    let v1_json = serde_json::to_string_pretty(&v1).expect("Failed to serialize v1");

    // Parse back as Value to verify structure
    let value: serde_json::Value = serde_json::from_str(&v1_json).unwrap();

    // Should have v1 structure
    assert_eq!(value.get("schema_version").and_then(|v| v.as_u64()), Some(1));
    assert!(value.get("details").is_some());
    assert!(value.get("library_info").is_none()); // v0 field should not be present

    // details should be properly tagged
    let details = value.get("details").unwrap();
    assert_eq!(
        details.get("type").and_then(|v| v.as_str()),
        Some("Library")
    );
}

#[test]
fn test_migrated_v1_roundtrips() {
    let content = read_fixture("metadata_v0_complete.json");
    let v0: MetadataV0 = serde_json::from_str(&content).unwrap();
    let v1 = migration::migrate_v0_to_v1(v0);

    // Serialize and deserialize
    let json = serde_json::to_string(&v1).expect("Failed to serialize");
    let roundtrip: ResearchMetadata = serde_json::from_str(&json).expect("Failed to deserialize");

    // All fields should match
    assert_eq!(v1.schema_version, roundtrip.schema_version);
    assert_eq!(v1.kind, roundtrip.kind);
    assert_eq!(v1.details, roundtrip.details);
    assert_eq!(v1.additional_files, roundtrip.additional_files);
    assert_eq!(v1.created_at, roundtrip.created_at);
    assert_eq!(v1.brief, roundtrip.brief);
    assert_eq!(v1.summary, roundtrip.summary);
    assert_eq!(v1.when_to_use, roundtrip.when_to_use);
}

// =============================================================================
// Regression Tests: when_to_use Extraction from SKILL.md
// =============================================================================
// Bug fix: Migration should populate when_to_use from SKILL.md frontmatter
// if the metadata doesn't have it.

/// Helper to create a valid SKILL.md with frontmatter
fn create_skill_md(description: &str) -> String {
    format!(
        r#"---
name: test-skill
description: {description}
---
# Test Skill

This is the body content.
"#
    )
}

#[tokio::test]
async fn test_v0_migration_extracts_when_to_use_from_skill_md() {
    // Regression test: v0 metadata without when_to_use should extract it from SKILL.md
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let output_dir = temp_dir.path();

    // Create v0 metadata WITHOUT when_to_use
    let v0_json = r#"{
        "schema_version": 0,
        "kind": "library",
        "library_info": {
            "package_manager": "npm",
            "language": "JavaScript",
            "url": "https://www.npmjs.com/package/chalk"
        },
        "additional_files": {},
        "created_at": "2024-01-01T00:00:00Z",
        "updated_at": "2024-01-01T00:00:00Z",
        "brief": "Terminal string styling library"
    }"#;

    let metadata_path = output_dir.join("metadata.json");
    fs::write(&metadata_path, v0_json)
        .await
        .expect("Failed to write v0 metadata");

    // Create skill/SKILL.md with frontmatter
    let skill_dir = output_dir.join("skill");
    fs::create_dir_all(&skill_dir)
        .await
        .expect("Failed to create skill dir");

    let skill_content = create_skill_md(
        "Expert knowledge for styling terminal output with Chalk. Use when building CLIs.",
    );
    fs::write(skill_dir.join("SKILL.md"), &skill_content)
        .await
        .expect("Failed to write SKILL.md");

    // Load should trigger migration AND extract when_to_use from SKILL.md
    let metadata = ResearchMetadata::load(output_dir)
        .await
        .expect("Failed to load metadata");

    // Verify migration occurred
    assert_eq!(metadata.schema_version, 1);

    // Verify when_to_use was extracted from SKILL.md frontmatter
    assert_eq!(
        metadata.when_to_use,
        Some("Expert knowledge for styling terminal output with Chalk. Use when building CLIs.".to_string())
    );

    // Verify it was saved to metadata.json
    let saved_content = fs::read_to_string(&metadata_path)
        .await
        .expect("Failed to read saved metadata");
    assert!(saved_content.contains("when_to_use"));
    assert!(saved_content.contains("Expert knowledge for styling terminal output"));
}

#[tokio::test]
async fn test_v1_without_when_to_use_extracts_from_skill_md() {
    // Regression test: v1 metadata without when_to_use should extract it from SKILL.md
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let output_dir = temp_dir.path();

    // Create v1 metadata WITHOUT when_to_use
    let v1_json = r#"{
        "schema_version": 1,
        "kind": "library",
        "details": {
            "type": "Library",
            "package_manager": "crates.io",
            "language": "Rust"
        },
        "additional_files": {},
        "created_at": "2024-01-01T00:00:00Z",
        "updated_at": "2024-01-01T00:00:00Z",
        "brief": "A Rust library"
    }"#;

    let metadata_path = output_dir.join("metadata.json");
    fs::write(&metadata_path, v1_json)
        .await
        .expect("Failed to write v1 metadata");

    // Create skill/SKILL.md with frontmatter
    let skill_dir = output_dir.join("skill");
    fs::create_dir_all(&skill_dir)
        .await
        .expect("Failed to create skill dir");

    let skill_content =
        create_skill_md("Expert knowledge for error handling in Rust. Use when building libraries.");
    fs::write(skill_dir.join("SKILL.md"), &skill_content)
        .await
        .expect("Failed to write SKILL.md");

    // Load should extract when_to_use from SKILL.md
    let metadata = ResearchMetadata::load(output_dir)
        .await
        .expect("Failed to load metadata");

    // Verify when_to_use was extracted from SKILL.md frontmatter
    assert_eq!(
        metadata.when_to_use,
        Some("Expert knowledge for error handling in Rust. Use when building libraries.".to_string())
    );

    // Verify it was saved to metadata.json
    let saved_content = fs::read_to_string(&metadata_path)
        .await
        .expect("Failed to read saved metadata");
    assert!(saved_content.contains("when_to_use"));
    assert!(saved_content.contains("Expert knowledge for error handling"));
}

#[tokio::test]
async fn test_existing_when_to_use_not_overwritten() {
    // Ensure existing when_to_use is preserved and not overwritten by SKILL.md
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let output_dir = temp_dir.path();

    // Create v1 metadata WITH when_to_use already set
    let v1_json = r#"{
        "schema_version": 1,
        "kind": "library",
        "details": {
            "type": "Library",
            "package_manager": "crates.io",
            "language": "Rust"
        },
        "additional_files": {},
        "created_at": "2024-01-01T00:00:00Z",
        "updated_at": "2024-01-01T00:00:00Z",
        "when_to_use": "Original when_to_use value"
    }"#;

    let metadata_path = output_dir.join("metadata.json");
    fs::write(&metadata_path, v1_json)
        .await
        .expect("Failed to write v1 metadata");

    // Create skill/SKILL.md with DIFFERENT description
    let skill_dir = output_dir.join("skill");
    fs::create_dir_all(&skill_dir)
        .await
        .expect("Failed to create skill dir");

    let skill_content = create_skill_md("Different description from SKILL.md");
    fs::write(skill_dir.join("SKILL.md"), &skill_content)
        .await
        .expect("Failed to write SKILL.md");

    // Load should NOT overwrite existing when_to_use
    let metadata = ResearchMetadata::load(output_dir)
        .await
        .expect("Failed to load metadata");

    // Verify original when_to_use was preserved
    assert_eq!(
        metadata.when_to_use,
        Some("Original when_to_use value".to_string())
    );
}

#[tokio::test]
async fn test_missing_skill_md_no_when_to_use() {
    // If SKILL.md doesn't exist, when_to_use remains None
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let output_dir = temp_dir.path();

    // Create v1 metadata without when_to_use and no SKILL.md
    let v1_json = r#"{
        "schema_version": 1,
        "kind": "library",
        "details": {
            "type": "Library"
        },
        "additional_files": {},
        "created_at": "2024-01-01T00:00:00Z",
        "updated_at": "2024-01-01T00:00:00Z"
    }"#;

    let metadata_path = output_dir.join("metadata.json");
    fs::write(&metadata_path, v1_json)
        .await
        .expect("Failed to write v1 metadata");

    // No skill/SKILL.md created

    // Load should return metadata with when_to_use still None
    let metadata = ResearchMetadata::load(output_dir)
        .await
        .expect("Failed to load metadata");

    assert!(metadata.when_to_use.is_none());
}

#[tokio::test]
async fn test_invalid_skill_md_frontmatter_no_when_to_use() {
    // If SKILL.md has invalid frontmatter, when_to_use remains None
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let output_dir = temp_dir.path();

    // Create v1 metadata without when_to_use
    let v1_json = r#"{
        "schema_version": 1,
        "kind": "library",
        "details": {
            "type": "Library"
        },
        "additional_files": {},
        "created_at": "2024-01-01T00:00:00Z",
        "updated_at": "2024-01-01T00:00:00Z"
    }"#;

    let metadata_path = output_dir.join("metadata.json");
    fs::write(&metadata_path, v1_json)
        .await
        .expect("Failed to write v1 metadata");

    // Create skill/SKILL.md with INVALID frontmatter (missing required description)
    let skill_dir = output_dir.join("skill");
    fs::create_dir_all(&skill_dir)
        .await
        .expect("Failed to create skill dir");

    let invalid_skill = r#"---
name: test-skill
---
# Missing description field
"#;
    fs::write(skill_dir.join("SKILL.md"), invalid_skill)
        .await
        .expect("Failed to write SKILL.md");

    // Load should return metadata with when_to_use still None (invalid frontmatter)
    let metadata = ResearchMetadata::load(output_dir)
        .await
        .expect("Failed to load metadata");

    assert!(metadata.when_to_use.is_none());
}
