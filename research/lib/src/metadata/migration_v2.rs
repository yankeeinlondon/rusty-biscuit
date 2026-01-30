//! Migration from v1 per-topic metadata to v2 centralized inventory.
//!
//! This module provides lazy migration from the legacy per-topic `metadata.json`
//! files to the new centralized [`ResearchInventory`] system.
//!
//! ## Migration Strategy
//!
//! Migration is **lazy**: when the inventory is loaded and doesn't exist,
//! the system scans the filesystem for existing research topics and builds
//! an inventory from their `metadata.json` files.
//!
//! The migration:
//! - Reads existing `metadata.json` files in `$RESEARCH_DIR/.research/library/*/`
//! - Creates [`Topic`] entries with populated [`Document`] lists
//! - Infers `ContentType` from filenames (e.g., `overview.md` → `Overview`)
//! - Computes content hashes using xxHash
//! - **Preserves** the original `metadata.json` files (non-destructive)
//!
//! ## Examples
//!
//! ```no_run
//! use research_lib::metadata::migration_v2::scan_and_build_inventory;
//! use std::path::PathBuf;
//!
//! let research_dir = PathBuf::from("/home/user/.research");
//! let inventory = scan_and_build_inventory(&research_dir).unwrap();
//! println!("Found {} topics", inventory.len());
//! ```

use std::fs;
use std::path::Path;

use chrono::{DateTime, TimeZone, Utc};
use thiserror::Error;
use xxhash_rust::xxh3::xxh3_64;

use super::inventory::ResearchInventory;
use sniff_lib::package::LanguagePackageManager;

use super::topic::{ContentType, Document, KindCategory, Library, Topic};
use crate::ResearchMetadata;

/// Errors that can occur during v2 migration.
#[derive(Debug, Error)]
pub enum MigrationV2Error {
    /// An I/O error occurred.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Failed to parse metadata JSON.
    #[error("Failed to parse metadata: {0}")]
    Parse(#[from] serde_json::Error),

    /// Invalid research directory structure.
    #[error("Invalid research directory: {0}")]
    InvalidDirectory(String),
}

/// Result type for migration operations.
pub type Result<T> = std::result::Result<T, MigrationV2Error>;

/// Build a [`Topic`] from an existing `metadata.json` file.
///
/// This function reads the v1 `metadata.json` and converts it to a v2 [`Topic`],
/// also scanning the topic directory for documents and computing their hashes.
///
/// ## Arguments
///
/// * `metadata_path` - Path to the `metadata.json` file
///
/// ## Returns
///
/// A [`Topic`] populated with metadata and document entries.
pub fn build_topic_from_metadata_json(metadata_path: &Path) -> Result<Topic> {
    let topic_dir = metadata_path
        .parent()
        .ok_or_else(|| MigrationV2Error::InvalidDirectory("metadata.json has no parent".into()))?;

    let topic_name = topic_dir
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| MigrationV2Error::InvalidDirectory("Cannot get topic name".into()))?
        .to_string();

    // Read and parse the v1 metadata
    let content = fs::read_to_string(metadata_path)?;
    let v1_metadata: ResearchMetadata = serde_json::from_str(&content)?;

    // Convert v1 kind to v2 KindCategory
    let kind = convert_kind(&v1_metadata, &topic_name);

    // Scan for documents in the topic directory
    let documents = scan_documents(topic_dir)?;

    // Build the Topic
    let topic = Topic::with_metadata(
        topic_name,
        kind,
        v1_metadata.created_at,
        v1_metadata.updated_at,
        v1_metadata.brief.unwrap_or_default(),
        v1_metadata.summary.unwrap_or_default(),
        v1_metadata.when_to_use.unwrap_or_default(),
        documents,
        Vec::new(), // children - not populated during migration
    );

    Ok(topic)
}

/// Scan the research directory and build a complete inventory.
///
/// This function walks `$research_dir/.research/library/*/metadata.json`
/// and builds a [`ResearchInventory`] from all discovered topics.
///
/// ## Arguments
///
/// * `research_dir` - The base research directory (e.g., `~/.research` or `$RESEARCH_DIR`)
///
/// ## Returns
///
/// A [`ResearchInventory`] populated with all discovered topics.
pub fn scan_and_build_inventory(research_dir: &Path) -> Result<ResearchInventory> {
    let library_dir = research_dir.join("library");

    if !library_dir.exists() {
        return Ok(ResearchInventory::new());
    }

    let mut inventory = ResearchInventory::new();

    // Iterate over topic directories
    for entry in fs::read_dir(&library_dir)? {
        let entry = entry?;
        let topic_path = entry.path();

        if !topic_path.is_dir() {
            continue;
        }

        let metadata_path = topic_path.join("metadata.json");
        if !metadata_path.exists() {
            continue;
        }

        match build_topic_from_metadata_json(&metadata_path) {
            Ok(topic) => {
                let name = topic.name().to_string();
                inventory.insert(name, topic);
            }
            Err(e) => {
                // Log warning but continue with other topics
                eprintln!(
                    "Warning: Failed to migrate topic at {:?}: {}",
                    topic_path, e
                );
            }
        }
    }

    Ok(inventory)
}

/// Convert v1 ResearchMetadata to v2 KindCategory.
fn convert_kind(v1: &ResearchMetadata, topic_name: &str) -> KindCategory {
    use crate::ResearchKind;

    match v1.kind {
        ResearchKind::Library => {
            // Extract library details if available
            if let Some(lib_details) = v1.library_details() {
                let pm_str = lib_details
                    .package_manager
                    .as_deref()
                    .unwrap_or("unknown");
                let package_manager = parse_package_manager(pm_str);
                let language = lib_details.language.clone().unwrap_or_default();
                let url = lib_details.url.clone().unwrap_or_default();

                KindCategory::Library(Library::new(
                    package_manager,
                    topic_name.to_string(),
                    language,
                    url,
                ))
            } else {
                // Default to Cargo for unknown libraries (most common case)
                KindCategory::Library(Library::new(
                    LanguagePackageManager::Cargo,
                    topic_name.to_string(),
                    "Rust".to_string(),
                    String::new(),
                ))
            }
        }
        ResearchKind::Api => {
            // For API research, still use Library with unknown package manager
            KindCategory::Library(Library::new(
                LanguagePackageManager::Cargo,
                topic_name.to_string(),
                String::new(),
                String::new(),
            ))
        }
    }
}

/// Parse a package manager string into a LanguagePackageManager enum.
fn parse_package_manager(pm: &str) -> LanguagePackageManager {
    match pm.to_lowercase().as_str() {
        "crates.io" | "cargo" => LanguagePackageManager::Cargo,
        "npm" | "npmjs" => LanguagePackageManager::Npm,
        "pnpm" => LanguagePackageManager::Pnpm,
        "yarn" => LanguagePackageManager::Yarn,
        "bun" => LanguagePackageManager::Bun,
        "pypi" | "pip" => LanguagePackageManager::Pip,
        "poetry" => LanguagePackageManager::Poetry,
        "uv" => LanguagePackageManager::Uv,
        "go" | "go.mod" | "pkg.go.dev" => LanguagePackageManager::GoModules,
        "composer" | "packagist" => LanguagePackageManager::Composer,
        "luarocks" => LanguagePackageManager::Luarocks,
        "hex" => LanguagePackageManager::Hex,
        "nuget" => LanguagePackageManager::Nuget,
        "cpan" => LanguagePackageManager::Cpan,
        "cpanm" => LanguagePackageManager::Cpanm,
        _ => LanguagePackageManager::Cargo, // Default to Cargo
    }
}

/// Scan a topic directory for documents and build Document entries.
fn scan_documents(topic_dir: &Path) -> Result<Vec<Document>> {
    let mut documents = Vec::new();

    for entry in fs::read_dir(topic_dir)? {
        let entry = entry?;
        let path = entry.path();

        // Only process .md files (skip metadata.json, directories, etc.)
        if !path.is_file() {
            continue;
        }

        let extension = path.extension().and_then(|e| e.to_str());
        if extension != Some("md") {
            continue;
        }

        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        // Infer content type based on document provenance (how it was created)
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let content_type = infer_content_type_from_filename(stem);

        // Get file timestamps
        let metadata = fs::metadata(&path)?;
        let (created, last_updated) = get_file_timestamps(&metadata);

        // Compute content hash
        let content = fs::read(&path)?;
        let content_hash = xxh3_64(&content);

        let doc = Document::with_metadata(
            filename,
            content_type,
            None, // prompt - not available in migration
            None, // flow not yet implemented
            created,
            last_updated,
            None, // model - not available in migration
            None, // model_capability - not available in migration
            content_hash,
            0, // interpolated_hash - not available in migration
        );

        documents.push(doc);
    }

    // Also check for skill directory
    let skill_dir = topic_dir.join("skill");
    if skill_dir.exists() && skill_dir.is_dir() {
        for entry in fs::read_dir(&skill_dir)? {
            let entry = entry?;
            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            let extension = path.extension().and_then(|e| e.to_str());
            if extension != Some("md") {
                continue;
            }

            let filename = format!(
                "skill/{}",
                path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("SKILL.md")
            );

            let metadata = fs::metadata(&path)?;
            let (created, last_updated) = get_file_timestamps(&metadata);

            let content = fs::read(&path)?;
            let content_hash = xxh3_64(&content);

            let doc = Document::with_metadata(
                filename,
                ContentType::Skill,
                None,
                None, // flow not yet implemented
                created,
                last_updated,
                None,
                None,
                content_hash,
                0,
            );

            documents.push(doc);
        }
    }

    // Also check for deep-dive directory (new structure: deep-dive/{topic}.md)
    let deep_dive_dir = topic_dir.join("deep-dive");
    if deep_dive_dir.exists() && deep_dive_dir.is_dir() {
        for entry in fs::read_dir(&deep_dive_dir)? {
            let entry = entry?;
            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            let extension = path.extension().and_then(|e| e.to_str());
            if extension != Some("md") {
                continue;
            }

            let filename = format!(
                "deep-dive/{}",
                path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("deep-dive.md")
            );

            let metadata = fs::metadata(&path)?;
            let (created, last_updated) = get_file_timestamps(&metadata);

            let content = fs::read(&path)?;
            let content_hash = xxh3_64(&content);

            let doc = Document::with_metadata(
                filename,
                ContentType::DeepDive,
                None,
                None, // flow not yet implemented
                created,
                last_updated,
                None,
                None,
                content_hash,
                0,
            );

            documents.push(doc);
        }
    }

    // Legacy: Migrate old-style deep_dive.md to new deep-dive/{topic}.md structure
    let legacy_deep_dive = topic_dir.join("deep_dive.md");
    if legacy_deep_dive.exists() && legacy_deep_dive.is_file() {
        // Get topic name from directory
        let topic_name = topic_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        // Create deep-dive directory and move file
        let deep_dive_dir = topic_dir.join("deep-dive");
        fs::create_dir_all(&deep_dive_dir)?;
        let new_path = deep_dive_dir.join(format!("{}.md", topic_name));

        // Move (rename) the file to new location
        fs::rename(&legacy_deep_dive, &new_path)?;

        // Now record with the new path
        let metadata = fs::metadata(&new_path)?;
        let (created, last_updated) = get_file_timestamps(&metadata);

        let content = fs::read(&new_path)?;
        let content_hash = xxh3_64(&content);

        let doc = Document::with_metadata(
            format!("deep-dive/{}.md", topic_name),
            ContentType::DeepDive,
            None,
            None,
            created,
            last_updated,
            None,
            None,
            content_hash,
            0,
        );

        documents.push(doc);
    }

    Ok(documents)
}

/// Infer content type based on filename for migration.
///
/// This maps known research filenames to their document provenance:
/// - Kind-derived: overview, similar_libraries, integration_partners, use_cases, changelog, brief
/// - Prompt: question_* files (user-provided prompts)
/// - DeepDive: files in deep-dive/ directory
/// - Skill: files in skill/ directory
fn infer_content_type_from_filename(stem: &str) -> ContentType {
    match stem.to_lowercase().as_str() {
        // Kind-derived documents (generated based on topic kind)
        "overview" | "similar_libraries" | "integration_partners" | "use_cases" | "changelog"
        | "brief" => ContentType::KindDerived,

        // Question files are prompt-generated
        s if s.starts_with("question_") => ContentType::Prompt,

        // Unknown files default to static (user-provided)
        _ => ContentType::Static,
    }
}

/// Get created and modified timestamps from file metadata.
fn get_file_timestamps(metadata: &fs::Metadata) -> (DateTime<Utc>, DateTime<Utc>) {
    let modified = metadata
        .modified()
        .ok()
        .and_then(|t| {
            t.duration_since(std::time::UNIX_EPOCH)
                .ok()
                .map(|d| Utc.timestamp_opt(d.as_secs() as i64, d.subsec_nanos()).single())
        })
        .flatten()
        .unwrap_or_else(Utc::now);

    let created = metadata
        .created()
        .ok()
        .and_then(|t| {
            t.duration_since(std::time::UNIX_EPOCH)
                .ok()
                .map(|d| Utc.timestamp_opt(d.as_secs() as i64, d.subsec_nanos()).single())
        })
        .flatten()
        .unwrap_or(modified);

    (created, modified)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::types::LibraryDetails;
    use crate::ResearchKind;
    use std::collections::HashMap;
    use tempfile::TempDir;

    fn create_test_metadata_json(topic_dir: &Path, name: &str) {
        let metadata = ResearchMetadata {
            schema_version: 1,
            kind: ResearchKind::Library,
            details: crate::metadata::ResearchDetails::Library(LibraryDetails {
                package_manager: Some("crates.io".to_string()),
                language: Some("Rust".to_string()),
                url: Some(format!("https://crates.io/crates/{}", name)),
                repository: None,
            }),
            additional_files: HashMap::new(),
            created_at: Utc::now() - chrono::Duration::days(30),
            updated_at: Utc::now(),
            brief: Some(format!("A test library: {}", name)),
            summary: Some(format!("Summary for {}", name)),
            when_to_use: Some(format!("Use {} when testing", name)),
        };

        let content = serde_json::to_string_pretty(&metadata).unwrap();
        fs::write(topic_dir.join("metadata.json"), content).unwrap();
    }

    fn create_test_document(topic_dir: &Path, filename: &str, content: &str) {
        fs::write(topic_dir.join(filename), content).unwrap();
    }

    #[test]
    fn test_build_topic_from_metadata_json() {
        let temp = TempDir::new().unwrap();
        let topic_dir = temp.path().join("test-lib");
        fs::create_dir_all(&topic_dir).unwrap();

        create_test_metadata_json(&topic_dir, "test-lib");
        create_test_document(&topic_dir, "overview.md", "# Overview\nTest content");
        create_test_document(&topic_dir, "similar_libraries.md", "# Similar\nOther libs");

        let metadata_path = topic_dir.join("metadata.json");
        let topic = build_topic_from_metadata_json(&metadata_path).unwrap();

        assert_eq!(topic.name(), "test-lib");
        assert_eq!(topic.brief(), "A test library: test-lib");
        assert_eq!(topic.documents().len(), 2);

        // Check document types are inferred correctly (both are KindDerived)
        for doc in topic.documents() {
            assert_eq!(
                doc.content_type(),
                &ContentType::KindDerived,
                "overview.md and similar_libraries.md should be KindDerived"
            );
        }
    }

    #[test]
    fn test_scan_and_build_inventory() {
        let temp = TempDir::new().unwrap();
        let research_dir = temp.path();
        let library_dir = research_dir.join("library");

        // Create multiple topics
        for name in ["topic-a", "topic-b", "topic-c"] {
            let topic_dir = library_dir.join(name);
            fs::create_dir_all(&topic_dir).unwrap();
            create_test_metadata_json(&topic_dir, name);
            create_test_document(&topic_dir, "overview.md", &format!("# {}", name));
        }

        let inventory = scan_and_build_inventory(research_dir).unwrap();

        assert_eq!(inventory.len(), 3);
        assert!(inventory.contains("topic-a"));
        assert!(inventory.contains("topic-b"));
        assert!(inventory.contains("topic-c"));
    }

    #[test]
    fn test_scan_empty_directory() {
        let temp = TempDir::new().unwrap();
        let inventory = scan_and_build_inventory(temp.path()).unwrap();
        assert!(inventory.is_empty());
    }

    #[test]
    fn test_scan_nonexistent_library_dir() {
        let temp = TempDir::new().unwrap();
        // Don't create the library directory
        let inventory = scan_and_build_inventory(temp.path()).unwrap();
        assert!(inventory.is_empty());
    }

    #[test]
    fn test_content_hash_computed() {
        let temp = TempDir::new().unwrap();
        let topic_dir = temp.path().join("hash-test");
        fs::create_dir_all(&topic_dir).unwrap();

        create_test_metadata_json(&topic_dir, "hash-test");
        create_test_document(&topic_dir, "overview.md", "Content for hashing");

        let metadata_path = topic_dir.join("metadata.json");
        let topic = build_topic_from_metadata_json(&metadata_path).unwrap();

        let doc = topic.documents().first().unwrap();
        assert!(doc.content_hash() != 0, "Content hash should be computed");
    }

    #[test]
    fn test_skill_directory_scanned() {
        let temp = TempDir::new().unwrap();
        let topic_dir = temp.path().join("skill-test");
        let skill_dir = topic_dir.join("skill");
        fs::create_dir_all(&skill_dir).unwrap();

        create_test_metadata_json(&topic_dir, "skill-test");
        create_test_document(&topic_dir, "overview.md", "Overview");
        fs::write(skill_dir.join("SKILL.md"), "# Skill\nContent").unwrap();

        let metadata_path = topic_dir.join("metadata.json");
        let topic = build_topic_from_metadata_json(&metadata_path).unwrap();

        assert_eq!(topic.documents().len(), 2);

        let skill_doc = topic
            .documents()
            .iter()
            .find(|d| d.filename() == "skill/SKILL.md");
        assert!(skill_doc.is_some());
        assert_eq!(skill_doc.unwrap().content_type(), &ContentType::Skill);
    }

    #[test]
    fn test_metadata_preserved_after_migration() {
        let temp = TempDir::new().unwrap();
        let topic_dir = temp.path().join("preserve-test");
        fs::create_dir_all(&topic_dir).unwrap();

        create_test_metadata_json(&topic_dir, "preserve-test");
        let metadata_path = topic_dir.join("metadata.json");

        // Migration should NOT modify the original file
        let original_content = fs::read_to_string(&metadata_path).unwrap();
        let _topic = build_topic_from_metadata_json(&metadata_path).unwrap();
        let after_content = fs::read_to_string(&metadata_path).unwrap();

        assert_eq!(original_content, after_content);
    }
}
