//! Filesystem discovery logic for research topics.
//!
//! This module provides functionality to walk the research library directory
//! and discover all research topics, parsing their metadata and checking
//! for completeness.

use super::types::{ResearchOutput, TopicInfo};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tracing::{debug, warn};
use walkdir::WalkDir;

/// Errors that can occur during topic discovery.
#[derive(Debug, Error)]
pub enum DiscoveryError {
    /// The research library directory was not found
    #[error("Research library directory not found: {0}")]
    DirectoryNotFound(PathBuf),

    /// Failed to read a directory during traversal
    #[error("Failed to read directory: {0}")]
    DirectoryRead(#[from] std::io::Error),

    /// Failed to parse metadata.json
    #[error("Failed to parse metadata.json: {0}")]
    MetadataParse(#[from] serde_json::Error),
}

/// Library information from metadata.json
#[derive(Debug, Deserialize)]
struct LibraryInfo {
    language: Option<String>,
}

/// Metadata structure as defined in metadata.json files.
///
/// This represents the schema of the metadata.json file that should exist
/// in each research topic directory.
#[derive(Debug, Deserialize)]
struct Metadata {
    /// Schema version (0 = legacy, 1 = current)
    #[serde(default)]
    schema_version: u32,

    /// The kind/type of research (e.g., "library", "software", "framework")
    #[serde(alias = "type")]
    kind: Option<String>,

    /// Brief one-sentence description of the topic
    brief: Option<String>,

    /// Library-specific information (language, etc.)
    #[serde(default)]
    library_info: Option<LibraryInfo>,

    /// Guidance on when to use this research (required for v1 schema)
    when_to_use: Option<String>,
}

impl Metadata {
    /// Check if this metadata needs migration/repair.
    ///
    /// Returns true if:
    /// - schema_version is 0 (needs v0 -> v1 migration)
    /// - schema_version is 1 but when_to_use is missing (incomplete v1)
    fn needs_migration(&self) -> bool {
        self.schema_version == 0 || (self.schema_version >= 1 && self.when_to_use.is_none())
    }
}

/// Expected underlying research document filenames.
///
/// These are the standard research documents that should be created
/// during the research process, before generating final deliverables.
const UNDERLYING_DOCS: &[&str] = &[
    "overview.md",
    "similar_libraries.md",
    "integration_partners.md",
    "use_cases.md",
    "changelog.md",
];

/// Discovers all research topics in the given base directory.
///
/// This function walks the filesystem at `base_dir` looking for topic directories,
/// parses their metadata, and checks for the presence of expected files.
///
/// # Arguments
///
/// * `base_dir` - The base directory to search (e.g., `$HOME/.research/library/`)
///
/// # Returns
///
/// A vector of `TopicInfo` structures, one for each discovered topic.
///
/// # Errors
///
/// Returns an error if the base directory doesn't exist or cannot be read.
///
/// # Example
///
/// ```no_run
/// use std::path::PathBuf;
/// use research_lib::list::discovery::discover_topics;
///
/// let topics = discover_topics(PathBuf::from("/home/user/.research/library")).unwrap();
/// for topic in topics {
///     println!("Found topic: {}", topic.name);
/// }
/// ```
pub fn discover_topics(base_dir: PathBuf) -> Result<Vec<TopicInfo>, DiscoveryError> {
    // Verify base directory exists
    if !base_dir.exists() {
        return Err(DiscoveryError::DirectoryNotFound(base_dir));
    }

    let mut topics = Vec::new();

    // Walk the directory tree, but only go 1 level deep (immediate subdirectories are topics)
    // Skip symlinks for security (prevent symlink attacks)
    for entry in WalkDir::new(&base_dir)
        .min_depth(1)
        .max_depth(1)
        .follow_links(false)
    {
        let entry = match entry {
            Ok(e) => e,
            Err(err) => {
                warn!("Failed to read directory entry: {}", err);
                continue;
            }
        };

        // Only process directories
        if !entry.file_type().is_dir() {
            continue;
        }

        let topic_dir = entry.path();
        let topic_name = match topic_dir.file_name() {
            Some(name) => name.to_string_lossy().to_string(),
            None => {
                warn!("Failed to extract topic name from path: {:?}", topic_dir);
                continue;
            }
        };

        debug!("Discovered topic directory: {}", topic_name);

        // Parse metadata and analyze topic
        let topic_info = analyze_topic(topic_name, topic_dir.to_path_buf());
        topics.push(topic_info);
    }

    Ok(topics)
}

/// Analyzes a single topic directory to determine its status and metadata.
///
/// This function:
/// 1. Reads and parses metadata.json (if present)
/// 2. Checks for the presence of expected output files
/// 3. Checks for the presence of underlying research documents
/// 4. Identifies any additional custom prompt files
fn analyze_topic(name: String, location: PathBuf) -> TopicInfo {
    let mut topic = TopicInfo::new(name.clone(), location.clone());

    // Try to read and parse metadata.json
    let metadata_path = location.join("metadata.json");
    match read_metadata(&metadata_path) {
        Ok(metadata) => {
            // Check if migration/repair is needed FIRST (before moving any fields)
            topic.needs_migration = metadata.needs_migration();
            topic.missing_metadata = false;
            topic.topic_type = metadata.kind.unwrap_or_else(|| "library".to_string());
            topic.description = metadata.brief;
            topic.language = metadata.library_info.and_then(|li| li.language);
        }
        Err(err) => {
            debug!("Failed to read metadata for topic '{}': {}", name, err);
            topic.missing_metadata = true;
            // Keep default values for topic_type, description, and language
        }
    }

    // Check for missing output deliverables
    check_output_files(&location, &mut topic);

    // Check for missing underlying research documents
    check_underlying_docs(&location, &mut topic);

    // Find additional custom prompt files
    find_additional_prompts(&location, &mut topic);

    topic
}

/// Reads and parses a metadata.json file.
fn read_metadata(path: &Path) -> Result<Metadata, DiscoveryError> {
    let content = std::fs::read_to_string(path)?;
    let metadata = serde_json::from_str(&content)?;
    Ok(metadata)
}

/// Checks for the presence of expected output files and updates topic.missing_output.
fn check_output_files(location: &Path, topic: &mut TopicInfo) {
    let outputs = [
        ResearchOutput::DeepDive,
        ResearchOutput::Brief,
        ResearchOutput::Skill,
    ];

    for output in outputs {
        let file_path = location.join(output.filename());
        if !file_path.exists() {
            debug!(
                "Topic '{}' is missing output file: {}",
                topic.name,
                output.filename()
            );
            topic.missing_output.push(output);
        }
    }
}

/// Checks for the presence of underlying research documents and updates topic.missing_underlying.
fn check_underlying_docs(location: &Path, topic: &mut TopicInfo) {
    for doc in UNDERLYING_DOCS {
        let doc_path = location.join(doc);
        if !doc_path.exists() {
            debug!("Topic '{}' is missing underlying doc: {}", topic.name, doc);
            topic.missing_underlying.push(doc.to_string());
        }
    }
}

/// Finds additional custom prompt files (question_*.md) and updates topic.additional_files.
fn find_additional_prompts(location: &Path, topic: &mut TopicInfo) {
    let read_dir = match std::fs::read_dir(location) {
        Ok(dir) => dir,
        Err(err) => {
            warn!("Failed to read directory for additional prompts: {}", err);
            return;
        }
    };

    for entry in read_dir {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();

        // Skip non-files
        if !path.is_file() {
            continue;
        }

        // Check if filename matches question_*.md pattern
        if let Some(filename) = path.file_name() {
            let filename = filename.to_string_lossy();
            if filename.starts_with("question_") && filename.ends_with(".md") {
                // Extract name without .md extension
                let name = filename.trim_end_matches(".md").to_string();
                debug!("Topic '{}' has additional prompt: {}", topic.name, name);
                topic.additional_files.push(name);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Helper to create a test topic directory with given files
    fn create_test_topic(
        base_dir: &Path,
        name: &str,
        metadata: Option<&str>,
        output_files: &[ResearchOutput],
        underlying_docs: &[&str],
        additional_prompts: &[&str],
    ) -> PathBuf {
        let topic_dir = base_dir.join(name);
        fs::create_dir(&topic_dir).unwrap();

        // Create metadata.json if provided
        if let Some(content) = metadata {
            fs::write(topic_dir.join("metadata.json"), content).unwrap();
        }

        // Create output files
        for output in output_files {
            let file_path = topic_dir.join(output.filename());
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(file_path, "test content").unwrap();
        }

        // Create underlying docs
        for doc in underlying_docs {
            fs::write(topic_dir.join(doc), "test content").unwrap();
        }

        // Create additional prompts
        for prompt in additional_prompts {
            fs::write(topic_dir.join(format!("{}.md", prompt)), "test content").unwrap();
        }

        topic_dir
    }

    #[test]
    fn test_discover_topics_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let topics = discover_topics(temp_dir.path().to_path_buf()).unwrap();
        assert_eq!(topics.len(), 0);
    }

    #[test]
    fn test_discover_topics_nonexistent_directory() {
        let result = discover_topics(PathBuf::from("/nonexistent/path"));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            DiscoveryError::DirectoryNotFound(_)
        ));
    }

    #[test]
    fn test_discover_complete_topic() {
        let temp_dir = TempDir::new().unwrap();
        let metadata = r#"{"schema_version": 1, "kind": "library", "brief": "A test library", "when_to_use": "Use for testing"}"#;

        create_test_topic(
            temp_dir.path(),
            "test-lib",
            Some(metadata),
            &[
                ResearchOutput::DeepDive,
                ResearchOutput::Brief,
                ResearchOutput::Skill,
            ],
            UNDERLYING_DOCS,
            &[],
        );

        let topics = discover_topics(temp_dir.path().to_path_buf()).unwrap();
        assert_eq!(topics.len(), 1);

        let topic = &topics[0];
        assert_eq!(topic.name, "test-lib");
        assert_eq!(topic.topic_type, "library");
        assert_eq!(topic.description, Some("A test library".to_string()));
        assert!(!topic.missing_metadata);
        assert!(!topic.needs_migration);
        assert!(topic.missing_output.is_empty());
        assert!(topic.missing_underlying.is_empty());
        assert!(topic.additional_files.is_empty());
        assert!(!topic.has_issues());
    }

    #[test]
    fn test_discover_topic_missing_metadata() {
        let temp_dir = TempDir::new().unwrap();

        create_test_topic(
            temp_dir.path(),
            "test-lib",
            None, // No metadata.json
            &[
                ResearchOutput::DeepDive,
                ResearchOutput::Brief,
                ResearchOutput::Skill,
            ],
            UNDERLYING_DOCS,
            &[],
        );

        let topics = discover_topics(temp_dir.path().to_path_buf()).unwrap();
        assert_eq!(topics.len(), 1);

        let topic = &topics[0];
        assert!(topic.missing_metadata);
        assert_eq!(topic.topic_type, "library"); // Default value
        assert_eq!(topic.description, None);
        assert!(topic.has_critical_issues());
    }

    #[test]
    fn test_discover_topic_corrupt_metadata() {
        let temp_dir = TempDir::new().unwrap();

        create_test_topic(
            temp_dir.path(),
            "test-lib",
            Some("invalid json {{{"),
            &[
                ResearchOutput::DeepDive,
                ResearchOutput::Brief,
                ResearchOutput::Skill,
            ],
            UNDERLYING_DOCS,
            &[],
        );

        let topics = discover_topics(temp_dir.path().to_path_buf()).unwrap();
        assert_eq!(topics.len(), 1);

        let topic = &topics[0];
        assert!(topic.missing_metadata);
        assert!(topic.has_critical_issues());
    }

    #[test]
    fn test_discover_topic_missing_outputs() {
        let temp_dir = TempDir::new().unwrap();
        let metadata = r#"{"kind": "framework", "brief": "A test framework"}"#;

        create_test_topic(
            temp_dir.path(),
            "test-framework",
            Some(metadata),
            &[ResearchOutput::DeepDive], // Missing Brief and Skill
            UNDERLYING_DOCS,
            &[],
        );

        let topics = discover_topics(temp_dir.path().to_path_buf()).unwrap();
        assert_eq!(topics.len(), 1);

        let topic = &topics[0];
        assert_eq!(topic.name, "test-framework");
        assert_eq!(topic.topic_type, "framework");
        assert_eq!(topic.missing_output.len(), 2);
        assert!(topic.missing_output.contains(&ResearchOutput::Brief));
        assert!(topic.missing_output.contains(&ResearchOutput::Skill));
        assert!(topic.has_critical_issues());
    }

    #[test]
    fn test_discover_topic_missing_underlying() {
        let temp_dir = TempDir::new().unwrap();
        let metadata = r#"{"kind": "library"}"#;

        create_test_topic(
            temp_dir.path(),
            "test-lib",
            Some(metadata),
            &[
                ResearchOutput::DeepDive,
                ResearchOutput::Brief,
                ResearchOutput::Skill,
            ],
            &["overview.md", "use_cases.md"], // Missing some underlying docs
            &[],
        );

        let topics = discover_topics(temp_dir.path().to_path_buf()).unwrap();
        assert_eq!(topics.len(), 1);

        let topic = &topics[0];
        assert_eq!(topic.missing_underlying.len(), 3);
        assert!(
            topic
                .missing_underlying
                .contains(&"similar_libraries.md".to_string())
        );
        assert!(
            topic
                .missing_underlying
                .contains(&"integration_partners.md".to_string())
        );
        assert!(
            topic
                .missing_underlying
                .contains(&"changelog.md".to_string())
        );
        assert!(topic.has_minor_issues_only());
    }

    #[test]
    fn test_discover_topic_with_additional_prompts() {
        let temp_dir = TempDir::new().unwrap();
        let metadata = r#"{"kind": "library"}"#;

        create_test_topic(
            temp_dir.path(),
            "test-lib",
            Some(metadata),
            &[
                ResearchOutput::DeepDive,
                ResearchOutput::Brief,
                ResearchOutput::Skill,
            ],
            UNDERLYING_DOCS,
            &["question_architecture", "question_performance"],
        );

        let topics = discover_topics(temp_dir.path().to_path_buf()).unwrap();
        assert_eq!(topics.len(), 1);

        let topic = &topics[0];
        assert_eq!(topic.additional_files.len(), 2);
        assert!(
            topic
                .additional_files
                .contains(&"question_architecture".to_string())
        );
        assert!(
            topic
                .additional_files
                .contains(&"question_performance".to_string())
        );
    }

    #[test]
    fn test_discover_multiple_topics() {
        let temp_dir = TempDir::new().unwrap();

        // Create three different topics
        create_test_topic(
            temp_dir.path(),
            "topic1",
            Some(r#"{"kind": "library"}"#),
            &[ResearchOutput::DeepDive],
            &[],
            &[],
        );

        create_test_topic(
            temp_dir.path(),
            "topic2",
            Some(r#"{"schema_version": 1, "kind": "framework", "when_to_use": "Use for testing"}"#),
            &[
                ResearchOutput::DeepDive,
                ResearchOutput::Brief,
                ResearchOutput::Skill,
            ],
            UNDERLYING_DOCS,
            &[],
        );

        create_test_topic(temp_dir.path(), "topic3", None, &[], &[], &[]);

        let mut topics = discover_topics(temp_dir.path().to_path_buf()).unwrap();
        assert_eq!(topics.len(), 3);

        // Sort by name for predictable testing
        topics.sort_by(|a, b| a.name.cmp(&b.name));

        assert_eq!(topics[0].name, "topic1");
        assert_eq!(topics[1].name, "topic2");
        assert_eq!(topics[2].name, "topic3");

        assert!(topics[0].has_critical_issues()); // Missing outputs
        assert!(!topics[1].has_issues()); // Complete
        assert!(topics[2].has_critical_issues()); // Missing everything
    }

    // =========================================================================
    // Regression Tests: needs_migration detection for when_to_use
    // =========================================================================

    #[test]
    fn test_v0_schema_needs_migration() {
        let temp_dir = TempDir::new().unwrap();
        // v0 schema (no schema_version field defaults to 0)
        let metadata = r#"{"kind": "library", "brief": "A v0 library"}"#;

        create_test_topic(
            temp_dir.path(),
            "v0-lib",
            Some(metadata),
            &[
                ResearchOutput::DeepDive,
                ResearchOutput::Brief,
                ResearchOutput::Skill,
            ],
            UNDERLYING_DOCS,
            &[],
        );

        let topics = discover_topics(temp_dir.path().to_path_buf()).unwrap();
        assert_eq!(topics.len(), 1);

        let topic = &topics[0];
        assert!(topic.needs_migration, "v0 schema should need migration");
        assert!(topic.has_issues(), "v0 schema should have issues");
    }

    #[test]
    fn test_v1_without_when_to_use_needs_migration() {
        // Regression test: v1 metadata without when_to_use should flag for migration
        let temp_dir = TempDir::new().unwrap();
        let metadata = r#"{"schema_version": 1, "kind": "library", "brief": "A v1 library without when_to_use"}"#;

        create_test_topic(
            temp_dir.path(),
            "incomplete-v1",
            Some(metadata),
            &[
                ResearchOutput::DeepDive,
                ResearchOutput::Brief,
                ResearchOutput::Skill,
            ],
            UNDERLYING_DOCS,
            &[],
        );

        let topics = discover_topics(temp_dir.path().to_path_buf()).unwrap();
        assert_eq!(topics.len(), 1);

        let topic = &topics[0];
        assert!(
            topic.needs_migration,
            "v1 without when_to_use should need migration"
        );
        assert!(
            topic.has_issues(),
            "v1 without when_to_use should have issues"
        );
    }

    #[test]
    fn test_v1_with_when_to_use_complete() {
        let temp_dir = TempDir::new().unwrap();
        let metadata = r#"{"schema_version": 1, "kind": "library", "brief": "A complete v1 library", "when_to_use": "Expert knowledge for testing"}"#;

        create_test_topic(
            temp_dir.path(),
            "complete-v1",
            Some(metadata),
            &[
                ResearchOutput::DeepDive,
                ResearchOutput::Brief,
                ResearchOutput::Skill,
            ],
            UNDERLYING_DOCS,
            &[],
        );

        let topics = discover_topics(temp_dir.path().to_path_buf()).unwrap();
        assert_eq!(topics.len(), 1);

        let topic = &topics[0];
        assert!(
            !topic.needs_migration,
            "v1 with when_to_use should NOT need migration"
        );
        assert!(!topic.has_issues(), "complete v1 should have no issues");
    }

    #[test]
    fn test_metadata_needs_migration_method() {
        // Test the Metadata::needs_migration() method directly
        let v0: Metadata = serde_json::from_str(r#"{"kind": "library"}"#).unwrap();
        assert!(v0.needs_migration(), "v0 (default schema_version=0) needs migration");

        let v1_incomplete: Metadata =
            serde_json::from_str(r#"{"schema_version": 1, "kind": "library"}"#).unwrap();
        assert!(
            v1_incomplete.needs_migration(),
            "v1 without when_to_use needs migration"
        );

        let v1_complete: Metadata = serde_json::from_str(
            r#"{"schema_version": 1, "kind": "library", "when_to_use": "Use for X"}"#,
        )
        .unwrap();
        assert!(
            !v1_complete.needs_migration(),
            "v1 with when_to_use does NOT need migration"
        );
    }
}
