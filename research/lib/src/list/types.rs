//! Type definitions for the list command.
//!
//! This module defines the core data structures used to represent research topics
//! and their associated metadata.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Represents the different types of research output deliverables.
///
/// Each research topic should produce multiple output files documenting
/// the research findings in different formats.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResearchOutput {
    /// The comprehensive deep dive document (deep_dive.md)
    DeepDive,
    /// The Claude Code skill package (skill/SKILL.md)
    Skill,
    /// The brief summary document (brief.md)
    Brief,
}

impl ResearchOutput {
    /// Returns the expected filename for this output type.
    pub fn filename(&self) -> &'static str {
        match self {
            ResearchOutput::DeepDive => "deep_dive.md",
            ResearchOutput::Brief => "brief.md",
            ResearchOutput::Skill => "skill/SKILL.md",
        }
    }
}

impl std::fmt::Display for ResearchOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResearchOutput::DeepDive => write!(f, "Deep Dive Document"),
            ResearchOutput::Skill => write!(f, "Skill"),
            ResearchOutput::Brief => write!(f, "Brief"),
        }
    }
}

/// Information about a research topic discovered in the filesystem.
///
/// This structure contains metadata about a single research topic,
/// including its completeness status and any issues detected.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicInfo {
    /// The name of the topic (directory name)
    pub name: String,

    /// The type/kind of the topic (e.g., "library", "software", "framework")
    #[serde(rename = "type")]
    pub topic_type: String,

    /// One-sentence description from metadata.json `brief` property
    pub description: Option<String>,

    /// Programming language from metadata.json `library_info.language` property
    pub language: Option<String>,

    /// Additional custom prompt files beyond core research prompts
    /// (e.g., question_*.md files without the .md extension)
    pub additional_files: Vec<String>,

    /// Missing underlying research documents (overview.md, use_cases.md, etc.)
    pub missing_underlying: Vec<String>,

    /// Missing final output deliverables (deep_dive, skill, brief)
    pub missing_output: Vec<ResearchOutput>,

    /// Whether metadata.json is missing or has missing required properties
    pub missing_metadata: bool,

    /// Whether metadata schema needs migration (v0 → v1)
    pub needs_migration: bool,

    /// The filepath to this topic's directory
    pub location: PathBuf,
}

impl TopicInfo {
    /// Creates a new TopicInfo with the given name and location.
    ///
    /// This is a convenience constructor that initializes all other fields
    /// to their default values. Use the builder pattern methods to set
    /// additional fields.
    pub fn new(name: String, location: PathBuf) -> Self {
        Self {
            name,
            topic_type: "library".to_string(),
            description: None,
            language: None,
            additional_files: Vec::new(),
            missing_underlying: Vec::new(),
            missing_output: Vec::new(),
            missing_metadata: false,
            needs_migration: false,
            location,
        }
    }

    /// Returns true if this topic has any missing files or metadata issues.
    pub fn has_issues(&self) -> bool {
        self.missing_metadata
            || self.needs_migration
            || !self.missing_output.is_empty()
            || !self.missing_underlying.is_empty()
    }

    /// Returns true if this topic is missing critical output files or metadata.
    ///
    /// Critical issues include missing metadata.json or missing final output
    /// deliverables (deep_dive.md, brief.md, skill/SKILL.md).
    pub fn has_critical_issues(&self) -> bool {
        self.missing_metadata || !self.missing_output.is_empty()
    }

    /// Returns true if this topic only has minor issues (missing underlying docs).
    pub fn has_minor_issues_only(&self) -> bool {
        !self.missing_metadata
            && self.missing_output.is_empty()
            && !self.missing_underlying.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_research_output_filename() {
        assert_eq!(ResearchOutput::DeepDive.filename(), "deep_dive.md");
        assert_eq!(ResearchOutput::Brief.filename(), "brief.md");
        assert_eq!(ResearchOutput::Skill.filename(), "skill/SKILL.md");
    }

    #[test]
    fn test_research_output_display() {
        assert_eq!(
            format!("{}", ResearchOutput::DeepDive),
            "Deep Dive Document"
        );
        assert_eq!(format!("{}", ResearchOutput::Skill), "Skill");
        assert_eq!(format!("{}", ResearchOutput::Brief), "Brief");
    }

    #[test]
    fn test_research_output_serialization() {
        let output = ResearchOutput::DeepDive;
        let json = serde_json::to_string(&output).unwrap();
        assert_eq!(json, "\"deep_dive\"");

        let deserialized: ResearchOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, ResearchOutput::DeepDive);
    }

    #[test]
    fn test_topic_info_new() {
        let topic = TopicInfo::new("test-topic".to_string(), PathBuf::from("/test/path"));

        assert_eq!(topic.name, "test-topic");
        assert_eq!(topic.topic_type, "library");
        assert_eq!(topic.description, None);
        assert!(topic.additional_files.is_empty());
        assert!(topic.missing_underlying.is_empty());
        assert!(topic.missing_output.is_empty());
        assert!(!topic.missing_metadata);
        assert_eq!(topic.location, PathBuf::from("/test/path"));
    }

    #[test]
    fn test_topic_info_has_issues() {
        let mut topic = TopicInfo::new("test".to_string(), PathBuf::from("/test"));
        assert!(!topic.has_issues());

        topic.missing_metadata = true;
        assert!(topic.has_issues());

        let mut topic2 = TopicInfo::new("test".to_string(), PathBuf::from("/test"));
        topic2.missing_output.push(ResearchOutput::DeepDive);
        assert!(topic2.has_issues());

        let mut topic3 = TopicInfo::new("test".to_string(), PathBuf::from("/test"));
        topic3.missing_underlying.push("overview.md".to_string());
        assert!(topic3.has_issues());
    }

    #[test]
    fn test_topic_info_has_critical_issues() {
        let mut topic = TopicInfo::new("test".to_string(), PathBuf::from("/test"));
        assert!(!topic.has_critical_issues());

        topic.missing_metadata = true;
        assert!(topic.has_critical_issues());

        let mut topic2 = TopicInfo::new("test".to_string(), PathBuf::from("/test"));
        topic2.missing_output.push(ResearchOutput::Brief);
        assert!(topic2.has_critical_issues());

        let mut topic3 = TopicInfo::new("test".to_string(), PathBuf::from("/test"));
        topic3.missing_underlying.push("overview.md".to_string());
        assert!(!topic3.has_critical_issues());
    }

    #[test]
    fn test_topic_info_has_minor_issues_only() {
        let mut topic = TopicInfo::new("test".to_string(), PathBuf::from("/test"));
        assert!(!topic.has_minor_issues_only());

        topic.missing_underlying.push("overview.md".to_string());
        assert!(topic.has_minor_issues_only());

        topic.missing_metadata = true;
        assert!(!topic.has_minor_issues_only());

        let mut topic2 = TopicInfo::new("test".to_string(), PathBuf::from("/test"));
        topic2.missing_underlying.push("overview.md".to_string());
        topic2.missing_output.push(ResearchOutput::DeepDive);
        assert!(!topic2.has_minor_issues_only());
    }

    #[test]
    fn test_topic_info_serialization() {
        let topic = TopicInfo {
            name: "test-lib".to_string(),
            topic_type: "library".to_string(),
            description: Some("A test library".to_string()),
            language: None,
            additional_files: vec!["custom_prompt".to_string()],
            missing_underlying: vec!["overview.md".to_string()],
            missing_output: vec![ResearchOutput::Brief],
            missing_metadata: false,
            needs_migration: false,
            location: PathBuf::from("/test/test-lib"),
        };

        let json = serde_json::to_string(&topic).unwrap();
        let deserialized: TopicInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.name, topic.name);
        assert_eq!(deserialized.topic_type, topic.topic_type);
        assert_eq!(deserialized.description, topic.description);
        assert_eq!(deserialized.additional_files, topic.additional_files);
        assert_eq!(deserialized.missing_underlying, topic.missing_underlying);
        assert_eq!(deserialized.missing_output, topic.missing_output);
        assert_eq!(deserialized.missing_metadata, topic.missing_metadata);
    }
}
