//! Research topic health validation
//!
//! This module provides centralized health checking for research topics, consolidating
//! scattered validation checks into a single, type-safe API. It validates that all required
//! files are present and that the SKILL.md frontmatter is correctly formatted.

use super::frontmatter::parse_and_validate_frontmatter;
use crate::list::types::ResearchOutput;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use thiserror::Error;

/// Research topic type categories
///
/// Represents the different types of research topics supported by the research library.
/// This enum provides type safety and ensures only valid research types are used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResearchType {
    /// Software library (npm package, Rust crate, Python package, etc.)
    Library,
    /// Command-line tool or utility
    Tool,
    /// Software application or system
    Software,
    /// Software framework or platform
    Framework,
}

impl ResearchType {
    /// Returns the string representation of this research type
    ///
    /// This is used for directory names and display purposes.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Library => "library",
            Self::Tool => "tool",
            Self::Software => "software",
            Self::Framework => "framework",
        }
    }

    /// Returns all valid research type variants
    pub fn all() -> &'static [ResearchType] {
        &[
            ResearchType::Library,
            ResearchType::Tool,
            ResearchType::Software,
            ResearchType::Framework,
        ]
    }
}

impl FromStr for ResearchType {
    type Err = ValidationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "library" => Ok(Self::Library),
            "tool" => Ok(Self::Tool),
            "software" => Ok(Self::Software),
            "framework" => Ok(Self::Framework),
            _ => Err(ValidationError::InvalidResearchType {
                provided: s.to_string(),
                valid_types: vec![
                    "library".to_string(),
                    "tool".to_string(),
                    "software".to_string(),
                    "framework".to_string(),
                ],
            }),
        }
    }
}

impl std::fmt::Display for ResearchType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Health status for a research topic
///
/// This structure consolidates all validation checks for a research topic,
/// including missing files and SKILL.md frontmatter validation. It provides
/// a complete snapshot of the topic's health status.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResearchHealth {
    /// The type of research (library, tool, software, framework)
    pub research_type: ResearchType,

    /// The topic name
    pub topic: String,

    /// Overall health status - true if all checks pass
    ///
    /// This is true only when:
    /// - All underlying prompts are present
    /// - All deliverables are present
    /// - SKILL.md frontmatter is valid
    pub ok: bool,

    /// Missing Phase 1 prompt files (e.g., "Overview", "Use Cases")
    ///
    /// This field is omitted from serialization when empty.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub missing_underlying: Vec<String>,

    /// Missing Phase 2 output deliverables
    ///
    /// This field is omitted from serialization when empty.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub missing_deliverables: Vec<ResearchOutput>,

    /// Whether the SKILL.md frontmatter is valid
    ///
    /// This is false if:
    /// - The skill file doesn't exist
    /// - The frontmatter is malformed
    /// - Required fields are missing or empty
    pub skill_structure_valid: bool,

    /// Schema version for future evolution
    ///
    /// This allows for backwards-compatible changes to the health check logic.
    #[serde(default = "default_version")]
    pub version: u8,
}

impl ResearchHealth {
    /// Creates a new ResearchHealth instance
    ///
    /// The `ok` field is automatically set based on whether there are any issues.
    pub fn new(
        research_type: ResearchType,
        topic: String,
        missing_underlying: Vec<String>,
        missing_deliverables: Vec<ResearchOutput>,
        skill_structure_valid: bool,
    ) -> Self {
        let ok = missing_underlying.is_empty()
            && missing_deliverables.is_empty()
            && skill_structure_valid;

        Self {
            research_type,
            topic,
            ok,
            missing_underlying,
            missing_deliverables,
            skill_structure_valid,
            version: 1,
        }
    }
}

fn default_version() -> u8 {
    1
}

/// Errors that can occur during research validation
#[derive(Debug, Error)]
pub enum ValidationError {
    /// The research topic directory was not found
    #[error("Research topic not found at path: {}", path.display())]
    TopicNotFound { path: PathBuf },

    /// An invalid research type string was provided
    #[error(
        "Invalid research type: '{provided}'. Valid types are: {}",
        valid_types.join(", ")
    )]
    InvalidResearchType {
        provided: String,
        valid_types: Vec<String>,
    },

    /// File I/O error occurred
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Standard prompt files that should be present in Phase 1
///
/// These are the input files that researchers create before running the
/// research generation workflow.
const STANDARD_PROMPTS: &[(&str, &str)] = &[
    ("Overview", "overview.md"),
    ("Similar Libraries", "similar_libraries.md"),
    ("Integration Partners", "integration_partners.md"),
    ("Use Cases", "use_cases.md"),
    ("Changelog", "changelog.md"),
    ("Additional Context", "context.md"),
];

/// Check the health of a research topic
///
/// This function performs a comprehensive health check on a research topic,
/// validating that all required files are present and properly formatted.
///
/// # Arguments
///
/// * `research_type` - The type of research (library, tool, software, framework)
/// * `topic` - The name of the research topic
///
/// # Returns
///
/// * `Ok(ResearchHealth)` - Health status with details about any issues
/// * `Err(ValidationError)` - Error if the topic doesn't exist or I/O fails
///
/// # Examples
///
/// ```no_run
/// use research_lib::validation::health::{research_health, ResearchType};
///
/// let health = research_health(ResearchType::Library, "pulldown-cmark")?;
/// if health.ok {
///     println!("Topic is healthy!");
/// } else {
///     println!("Found {} issues", health.missing_underlying.len());
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn research_health(
    research_type: ResearchType,
    topic: &str,
) -> Result<ResearchHealth, ValidationError> {
    // Determine base path from environment or default location
    let base_path = get_research_base_path()?;
    let topic_path = base_path.join(research_type.as_str()).join(topic);

    if !topic_path.exists() {
        return Err(ValidationError::TopicNotFound { path: topic_path });
    }

    // Check Phase 1 prompts
    let missing_underlying = check_missing_prompts(&topic_path);

    // Check Phase 2 outputs
    let missing_deliverables = check_missing_outputs(&topic_path);

    // Validate SKILL.md frontmatter
    let skill_structure_valid = validate_skill_frontmatter(&topic_path);

    Ok(ResearchHealth::new(
        research_type,
        topic.to_string(),
        missing_underlying,
        missing_deliverables,
        skill_structure_valid,
    ))
}

/// Get the base path for research topics
///
/// This checks the RESEARCH_DIR environment variable, falling back to
/// the current directory if not set.
fn get_research_base_path() -> Result<PathBuf, ValidationError> {
    if let Ok(research_dir) = std::env::var("RESEARCH_DIR") {
        Ok(PathBuf::from(research_dir))
    } else {
        // Default to current directory
        std::env::current_dir().map_err(ValidationError::IoError)
    }
}

/// Check for missing Phase 1 prompt files
///
/// Returns a list of prompt names (not filenames) that are missing.
fn check_missing_prompts(topic_path: &Path) -> Vec<String> {
    STANDARD_PROMPTS
        .iter()
        .filter(|(_, filename)| !topic_path.join(filename).exists())
        .map(|(name, _)| name.to_string())
        .collect()
}

/// Check for missing Phase 2 output files
///
/// Returns a list of output types that are missing.
fn check_missing_outputs(topic_path: &Path) -> Vec<ResearchOutput> {
    let mut missing = Vec::new();

    if !topic_path.join("deep_dive.md").exists() {
        missing.push(ResearchOutput::DeepDive);
    }
    if !topic_path.join("brief.md").exists() {
        missing.push(ResearchOutput::Brief);
    }
    if !topic_path.join("skill/SKILL.md").exists() {
        missing.push(ResearchOutput::Skill);
    }

    missing
}

/// Validate SKILL.md frontmatter
///
/// Returns true if the skill file exists and has valid frontmatter.
fn validate_skill_frontmatter(topic_path: &Path) -> bool {
    let skill_path = topic_path.join("skill/SKILL.md");
    if !skill_path.exists() {
        return false;
    }

    match std::fs::read_to_string(&skill_path) {
        Ok(content) => parse_and_validate_frontmatter(&content).is_ok(),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Create a test research topic directory structure
    fn create_test_topic(temp: &TempDir, research_type: &str, topic: &str) -> PathBuf {
        let topic_path = temp.path().join(research_type).join(topic);
        fs::create_dir_all(&topic_path).unwrap();
        topic_path
    }

    /// Create a valid SKILL.md file
    fn create_valid_skill(topic_path: &Path) {
        let skill_dir = topic_path.join("skill");
        fs::create_dir_all(&skill_dir).unwrap();

        let skill_content = r#"---
name: test-skill
description: A test skill for validation
---
# Test Skill

This is the skill content.
"#;
        fs::write(skill_dir.join("SKILL.md"), skill_content).unwrap();
    }

    /// Create all standard prompt files
    fn create_all_prompts(topic_path: &Path) {
        for (_, filename) in STANDARD_PROMPTS {
            fs::write(topic_path.join(filename), "Test content").unwrap();
        }
    }

    /// Create all output deliverables
    fn create_all_outputs(topic_path: &Path) {
        create_valid_skill(topic_path);
        fs::write(topic_path.join("deep_dive.md"), "Deep dive content").unwrap();
        fs::write(topic_path.join("brief.md"), "Brief content").unwrap();
    }

    #[test]
    fn test_research_type_from_str_valid() {
        assert_eq!(
            ResearchType::from_str("library").unwrap(),
            ResearchType::Library
        );
        assert_eq!(ResearchType::from_str("tool").unwrap(), ResearchType::Tool);
        assert_eq!(
            ResearchType::from_str("software").unwrap(),
            ResearchType::Software
        );
        assert_eq!(
            ResearchType::from_str("framework").unwrap(),
            ResearchType::Framework
        );
    }

    #[test]
    fn test_research_type_from_str_case_insensitive() {
        assert_eq!(
            ResearchType::from_str("LIBRARY").unwrap(),
            ResearchType::Library
        );
        assert_eq!(
            ResearchType::from_str("Library").unwrap(),
            ResearchType::Library
        );
        assert_eq!(
            ResearchType::from_str("LiBrArY").unwrap(),
            ResearchType::Library
        );
    }

    #[test]
    fn test_research_type_from_str_invalid() {
        let result = ResearchType::from_str("invalid");
        assert!(result.is_err());
        match result {
            Err(ValidationError::InvalidResearchType {
                provided,
                valid_types,
            }) => {
                assert_eq!(provided, "invalid");
                assert_eq!(valid_types.len(), 4);
            }
            _ => panic!("Expected InvalidResearchType error"),
        }
    }

    #[test]
    fn test_research_type_display() {
        assert_eq!(ResearchType::Library.to_string(), "library");
        assert_eq!(ResearchType::Tool.to_string(), "tool");
        assert_eq!(ResearchType::Software.to_string(), "software");
        assert_eq!(ResearchType::Framework.to_string(), "framework");
    }

    #[test]
    fn test_research_type_as_str() {
        assert_eq!(ResearchType::Library.as_str(), "library");
        assert_eq!(ResearchType::Tool.as_str(), "tool");
        assert_eq!(ResearchType::Software.as_str(), "software");
        assert_eq!(ResearchType::Framework.as_str(), "framework");
    }

    #[test]
    fn test_research_type_serde_serialization() {
        let rt = ResearchType::Library;
        let json = serde_json::to_string(&rt).unwrap();
        assert_eq!(json, r#""library""#);
    }

    #[test]
    fn test_research_type_serde_deserialization() {
        let rt: ResearchType = serde_json::from_str(r#""library""#).unwrap();
        assert_eq!(rt, ResearchType::Library);
    }

    #[test]
    fn test_healthy_topic_all_files_present() {
        let temp = TempDir::new().unwrap();
        let topic_path = create_test_topic(&temp, "library", "test-lib");

        create_all_prompts(&topic_path);
        create_all_outputs(&topic_path);

        // Set RESEARCH_DIR to temp directory
        unsafe {
            std::env::set_var("RESEARCH_DIR", temp.path());
        }

        let health = research_health(ResearchType::Library, "test-lib").unwrap();

        assert!(health.ok, "Health should be OK");
        assert!(health.missing_underlying.is_empty());
        assert!(health.missing_deliverables.is_empty());
        assert!(health.skill_structure_valid);
        assert_eq!(health.research_type, ResearchType::Library);
        assert_eq!(health.topic, "test-lib");
        assert_eq!(health.version, 1);

        // Cleanup
        unsafe {
            std::env::remove_var("RESEARCH_DIR");
        }
    }

    #[test]
    fn test_missing_phase1_prompts() {
        let temp = TempDir::new().unwrap();
        let topic_path = create_test_topic(&temp, "library", "incomplete-lib");

        // Create only some prompts
        fs::write(topic_path.join("overview.md"), "content").unwrap();
        fs::write(topic_path.join("use_cases.md"), "content").unwrap();

        create_all_outputs(&topic_path);

        unsafe {
            std::env::set_var("RESEARCH_DIR", temp.path());
        }

        let health = research_health(ResearchType::Library, "incomplete-lib").unwrap();

        assert!(!health.ok, "Health should not be OK");
        assert_eq!(health.missing_underlying.len(), 4); // 6 total - 2 created = 4 missing
        assert!(
            health
                .missing_underlying
                .contains(&"Similar Libraries".to_string())
        );
        assert!(
            health
                .missing_underlying
                .contains(&"Integration Partners".to_string())
        );
        assert!(health.missing_underlying.contains(&"Changelog".to_string()));
        assert!(
            health
                .missing_underlying
                .contains(&"Additional Context".to_string())
        );
        assert!(health.missing_deliverables.is_empty());
        assert!(health.skill_structure_valid);

        unsafe {
            std::env::remove_var("RESEARCH_DIR");
        }
    }

    #[test]
    fn test_missing_phase2_outputs() {
        let temp = TempDir::new().unwrap();
        let topic_path = create_test_topic(&temp, "tool", "incomplete-tool");

        create_all_prompts(&topic_path);

        // Create only skill, missing deep_dive and brief
        create_valid_skill(&topic_path);

        unsafe {
            std::env::set_var("RESEARCH_DIR", temp.path());
        }

        let health = research_health(ResearchType::Tool, "incomplete-tool").unwrap();

        assert!(!health.ok, "Health should not be OK");
        assert!(health.missing_underlying.is_empty());
        assert_eq!(health.missing_deliverables.len(), 2);
        assert!(
            health
                .missing_deliverables
                .contains(&ResearchOutput::DeepDive)
        );
        assert!(health.missing_deliverables.contains(&ResearchOutput::Brief));
        assert!(health.skill_structure_valid);

        unsafe {
            std::env::remove_var("RESEARCH_DIR");
        }
    }

    #[test]
    fn test_invalid_skill_frontmatter() {
        let temp = TempDir::new().unwrap();
        let topic_path = create_test_topic(&temp, "software", "bad-skill");

        create_all_prompts(&topic_path);

        let skill_dir = topic_path.join("skill");
        fs::create_dir_all(&skill_dir).unwrap();

        // Create skill with invalid frontmatter
        let invalid_skill = r#"---
name: ""
description: test
---
Content
"#;
        fs::write(skill_dir.join("SKILL.md"), invalid_skill).unwrap();

        fs::write(topic_path.join("deep_dive.md"), "content").unwrap();
        fs::write(topic_path.join("brief.md"), "content").unwrap();

        unsafe {
            std::env::set_var("RESEARCH_DIR", temp.path());
        }

        let health = research_health(ResearchType::Software, "bad-skill").unwrap();

        assert!(!health.ok, "Health should not be OK");
        assert!(health.missing_underlying.is_empty());
        assert!(health.missing_deliverables.is_empty());
        assert!(
            !health.skill_structure_valid,
            "Skill structure should be invalid"
        );

        unsafe {
            std::env::remove_var("RESEARCH_DIR");
        }
    }

    #[test]
    fn test_missing_skill_file() {
        let temp = TempDir::new().unwrap();
        let topic_path = create_test_topic(&temp, "framework", "no-skill");

        create_all_prompts(&topic_path);
        fs::write(topic_path.join("deep_dive.md"), "content").unwrap();
        fs::write(topic_path.join("brief.md"), "content").unwrap();

        unsafe {
            std::env::set_var("RESEARCH_DIR", temp.path());
        }

        let health = research_health(ResearchType::Framework, "no-skill").unwrap();

        assert!(!health.ok, "Health should not be OK");
        assert!(health.missing_underlying.is_empty());
        assert_eq!(health.missing_deliverables.len(), 1);
        assert!(health.missing_deliverables.contains(&ResearchOutput::Skill));
        assert!(
            !health.skill_structure_valid,
            "Skill structure should be invalid when file missing"
        );

        unsafe {
            std::env::remove_var("RESEARCH_DIR");
        }
    }

    #[test]
    fn test_topic_not_found() {
        let temp = TempDir::new().unwrap();
        unsafe {
            std::env::set_var("RESEARCH_DIR", temp.path());
        }

        let result = research_health(ResearchType::Library, "nonexistent");

        assert!(result.is_err());
        match result {
            Err(ValidationError::TopicNotFound { path }) => {
                assert!(path.to_string_lossy().contains("nonexistent"));
            }
            _ => panic!("Expected TopicNotFound error"),
        }

        unsafe {
            std::env::remove_var("RESEARCH_DIR");
        }
    }

    #[test]
    fn test_research_health_serde_serialization() {
        let health = ResearchHealth::new(
            ResearchType::Library,
            "test".to_string(),
            vec!["Overview".to_string()],
            vec![ResearchOutput::Brief],
            false,
        );

        let json = serde_json::to_string(&health).unwrap();

        // Check that it serializes to camelCase
        assert!(json.contains(r#""researchType""#));
        assert!(json.contains(r#""topic""#));
        assert!(json.contains(r#""ok""#));
        assert!(json.contains(r#""missingUnderlying""#));
        assert!(json.contains(r#""missingDeliverables""#));
        assert!(json.contains(r#""skillStructureValid""#));
    }

    #[test]
    fn test_research_health_empty_vecs_omitted() {
        let health =
            ResearchHealth::new(ResearchType::Tool, "test".to_string(), vec![], vec![], true);

        let json = serde_json::to_string(&health).unwrap();

        // Empty vecs should be omitted
        assert!(!json.contains(r#""missingUnderlying""#));
        assert!(!json.contains(r#""missingDeliverables""#));
    }
}
