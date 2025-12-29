//! Type definitions for the link command.
//!
//! This module defines the core data structures used to represent skill linking
//! actions and results when creating symbolic links from research topic skill
//! directories to Claude Code and OpenCode user-scoped skill locations.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::list::discovery::DiscoveryError;
use crate::list::filter::FilterError;

/// Represents the outcome of attempting to create a skill symlink.
///
/// Each skill can have a different action for Claude Code vs OpenCode,
/// allowing for asymmetric failure handling.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillAction {
    /// Successfully created a new symlink
    CreatedLink,

    /// No action taken - symlink already exists
    NoneAlreadyLinked,

    /// No action taken - a local skill definition exists at the target location
    /// (i.e., a real directory or file, not a symlink)
    NoneLocalDefinition,

    /// No action taken - source skill directory is invalid
    /// (missing skill/SKILL.md or skill/ directory doesn't exist)
    NoneSkillDirectoryInvalid,

    /// Failed to create symlink due to permission denied
    /// Contains the error message for user feedback
    FailedPermissionDenied(String),

    /// Failed to create symlink due to other I/O error
    /// Contains the error message for user feedback
    FailedOther(String),
}

impl SkillAction {
    /// Returns true if this action represents a successful outcome
    /// (either created a link or a link already exists)
    pub fn is_success(&self) -> bool {
        matches!(
            self,
            SkillAction::CreatedLink | SkillAction::NoneAlreadyLinked
        )
    }

    /// Returns true if this action represents a failure
    pub fn is_failure(&self) -> bool {
        matches!(
            self,
            SkillAction::FailedPermissionDenied(_) | SkillAction::FailedOther(_)
        )
    }

    /// Returns true if no action was taken (already linked, local definition, or invalid)
    pub fn is_skipped(&self) -> bool {
        matches!(
            self,
            SkillAction::NoneAlreadyLinked
                | SkillAction::NoneLocalDefinition
                | SkillAction::NoneSkillDirectoryInvalid
        )
    }
}

/// Information about a skill link operation for both Claude Code and OpenCode.
///
/// This structure tracks the action taken for each service independently,
/// allowing for scenarios where one service succeeds while the other fails.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillLink {
    /// The name of the skill/topic
    pub name: String,

    /// Action taken for Claude Code (~/.claude/skills/)
    pub claude_action: SkillAction,

    /// Action taken for OpenCode (~/.config/opencode/skill/)
    pub opencode_action: SkillAction,
}

impl SkillLink {
    /// Creates a new SkillLink with the given name and actions
    pub fn new(name: String, claude_action: SkillAction, opencode_action: SkillAction) -> Self {
        Self {
            name,
            claude_action,
            opencode_action,
        }
    }

    /// Returns true if both actions were successful
    pub fn both_succeeded(&self) -> bool {
        self.claude_action.is_success() && self.opencode_action.is_success()
    }

    /// Returns true if at least one action failed
    pub fn has_failure(&self) -> bool {
        self.claude_action.is_failure() || self.opencode_action.is_failure()
    }

    /// Returns true if both actions were skipped (no changes made)
    pub fn both_skipped(&self) -> bool {
        self.claude_action.is_skipped() && self.opencode_action.is_skipped()
    }
}

/// Result of the link command execution.
///
/// Contains all skill link operations and any errors encountered during processing.
#[derive(Debug, Serialize, Deserialize)]
pub struct LinkResult {
    /// All skill link operations attempted
    pub links: Vec<SkillLink>,

    /// Errors encountered during processing that prevented specific operations
    /// Format: (topic_name, error_message)
    pub errors: Vec<(String, String)>,
}

impl LinkResult {
    /// Creates a new empty LinkResult
    pub fn new() -> Self {
        Self {
            links: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Returns the total number of skills processed
    pub fn total_processed(&self) -> usize {
        self.links.len()
    }

    /// Returns the number of skills where links were successfully created
    pub fn total_created(&self) -> usize {
        self.links
            .iter()
            .filter(|link| {
                matches!(link.claude_action, SkillAction::CreatedLink)
                    || matches!(link.opencode_action, SkillAction::CreatedLink)
            })
            .count()
    }

    /// Returns the number of skills with failures
    pub fn total_failed(&self) -> usize {
        self.links.iter().filter(|link| link.has_failure()).count()
    }

    /// Returns true if any errors occurred
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty() || self.total_failed() > 0
    }
}

impl Default for LinkResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors that can occur during the link command execution.
///
/// Uses thiserror for automatic error implementation and error chaining.
#[derive(Debug, Error)]
pub enum LinkError {
    /// Error occurred during topic discovery
    #[error("Discovery failed: {0}")]
    Discovery(#[from] DiscoveryError),

    /// Error occurred during topic filtering
    #[error("Filter failed: {0}")]
    Filter(#[from] FilterError),

    /// Failed to determine the user's home directory
    #[error("Failed to determine home directory")]
    HomeDirectory,

    /// Generic I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_action_is_success() {
        assert!(SkillAction::CreatedLink.is_success());
        assert!(SkillAction::NoneAlreadyLinked.is_success());
        assert!(!SkillAction::NoneLocalDefinition.is_success());
        assert!(!SkillAction::FailedPermissionDenied("error".to_string()).is_success());
    }

    #[test]
    fn test_skill_action_is_failure() {
        assert!(SkillAction::FailedPermissionDenied("error".to_string()).is_failure());
        assert!(SkillAction::FailedOther("error".to_string()).is_failure());
        assert!(!SkillAction::CreatedLink.is_failure());
        assert!(!SkillAction::NoneAlreadyLinked.is_failure());
    }

    #[test]
    fn test_skill_action_is_skipped() {
        assert!(SkillAction::NoneAlreadyLinked.is_skipped());
        assert!(SkillAction::NoneLocalDefinition.is_skipped());
        assert!(SkillAction::NoneSkillDirectoryInvalid.is_skipped());
        assert!(!SkillAction::CreatedLink.is_skipped());
        assert!(!SkillAction::FailedPermissionDenied("error".to_string()).is_skipped());
    }

    #[test]
    fn test_skill_link_new() {
        let link = SkillLink::new(
            "test-skill".to_string(),
            SkillAction::CreatedLink,
            SkillAction::NoneAlreadyLinked,
        );

        assert_eq!(link.name, "test-skill");
        assert_eq!(link.claude_action, SkillAction::CreatedLink);
        assert_eq!(link.opencode_action, SkillAction::NoneAlreadyLinked);
    }

    #[test]
    fn test_skill_link_both_succeeded() {
        let link1 = SkillLink::new(
            "test".to_string(),
            SkillAction::CreatedLink,
            SkillAction::CreatedLink,
        );
        assert!(link1.both_succeeded());

        let link2 = SkillLink::new(
            "test".to_string(),
            SkillAction::NoneAlreadyLinked,
            SkillAction::CreatedLink,
        );
        assert!(link2.both_succeeded());

        let link3 = SkillLink::new(
            "test".to_string(),
            SkillAction::CreatedLink,
            SkillAction::FailedOther("error".to_string()),
        );
        assert!(!link3.both_succeeded());
    }

    #[test]
    fn test_skill_link_has_failure() {
        let link1 = SkillLink::new(
            "test".to_string(),
            SkillAction::FailedPermissionDenied("error".to_string()),
            SkillAction::CreatedLink,
        );
        assert!(link1.has_failure());

        let link2 = SkillLink::new(
            "test".to_string(),
            SkillAction::CreatedLink,
            SkillAction::FailedOther("error".to_string()),
        );
        assert!(link2.has_failure());

        let link3 = SkillLink::new(
            "test".to_string(),
            SkillAction::CreatedLink,
            SkillAction::NoneAlreadyLinked,
        );
        assert!(!link3.has_failure());
    }

    #[test]
    fn test_link_result_new() {
        let result = LinkResult::new();
        assert_eq!(result.links.len(), 0);
        assert_eq!(result.errors.len(), 0);
        assert_eq!(result.total_processed(), 0);
    }

    #[test]
    fn test_link_result_total_created() {
        let mut result = LinkResult::new();
        result.links.push(SkillLink::new(
            "test1".to_string(),
            SkillAction::CreatedLink,
            SkillAction::NoneAlreadyLinked,
        ));
        result.links.push(SkillLink::new(
            "test2".to_string(),
            SkillAction::NoneAlreadyLinked,
            SkillAction::CreatedLink,
        ));
        result.links.push(SkillLink::new(
            "test3".to_string(),
            SkillAction::NoneAlreadyLinked,
            SkillAction::NoneAlreadyLinked,
        ));

        assert_eq!(result.total_processed(), 3);
        assert_eq!(result.total_created(), 2);
    }

    #[test]
    fn test_link_result_serialization() {
        let mut result = LinkResult::new();
        result.links.push(SkillLink::new(
            "test-skill".to_string(),
            SkillAction::CreatedLink,
            SkillAction::NoneAlreadyLinked,
        ));
        result
            .errors
            .push(("error-topic".to_string(), "test error".to_string()));

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: LinkResult = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.links.len(), 1);
        assert_eq!(deserialized.links[0].name, "test-skill");
        assert_eq!(deserialized.errors.len(), 1);
        assert_eq!(deserialized.errors[0].0, "error-topic");
    }
}
