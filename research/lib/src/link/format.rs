//! Output formatting for link command results.
//!
//! This module provides terminal-friendly and JSON output formatting for the
//! link command, displaying skill linking results with appropriate colors and
//! structure for both human and machine consumption.
//!
//! Supports three services: Claude Code, OpenCode, and Roo Code.

use crate::link::types::{LinkResult, SkillAction, SkillLink};
use owo_colors::OwoColorize;

/// Format link results for terminal output with colors and formatting.
///
/// Uses owo-colors to provide visual feedback about the status of each skill link:
/// - Topic names are **bold**
/// - Successful link creation is shown in **green**
/// - Already linked skills are shown **dim + italic**
/// - Local definitions are shown in **yellow** (warning)
/// - Invalid skill directories are shown in **yellow**
/// - Failed operations are shown in **red** with error messages
///
/// # Arguments
///
/// * `result` - The link result containing all skill link operations
///
/// # Returns
///
/// A formatted string ready for terminal output
///
/// # Example
///
/// ```rust
/// use research_lib::link::{LinkResult, SkillLink, SkillAction};
/// use research_lib::link::format::format_terminal;
///
/// let mut result = LinkResult::new();
/// result.links.push(SkillLink::new(
///     "clap".to_string(),
///     SkillAction::CreatedLink,
///     SkillAction::CreatedLink,
///     SkillAction::CreatedLink,
/// ));
///
/// let output = format_terminal(&result);
/// println!("{}", output);
/// ```
pub fn format_terminal(result: &LinkResult) -> String {
    let mut output = String::new();

    // Report stale symlinks that were cleaned up
    if !result.stale_removed.is_empty() {
        output.push_str(&format!(
            "{}\n",
            format!("Cleaned up {} stale symlink(s)", result.stale_removed.len())
                .yellow()
                .bold()
        ));
        output.push('\n');
    }

    if result.links.is_empty() {
        output.push_str(&"No topics processed\n".dimmed().to_string());
        return output;
    }

    for link in &result.links {
        output.push_str(&format_skill_link(link));
        output.push('\n');
    }

    // Add summary if there are errors
    if result.has_errors() {
        output.push('\n');
        output.push_str(&format!(
            "{}\n",
            format!("Summary: {} failed", result.total_failed())
                .red()
                .bold()
        ));
    }

    output
}

/// Format a single skill link for terminal output.
fn format_skill_link(link: &SkillLink) -> String {
    let topic_name = link.name.bold();

    // Format skill status for all three services
    let skill_status =
        format_skill_actions(&link.claude_action, &link.opencode_action, &link.roo_action);

    // Format doc status if present (all three must be present)
    let doc_status = match (
        &link.claude_doc_action,
        &link.opencode_doc_action,
        &link.roo_doc_action,
    ) {
        (Some(claude), Some(opencode), Some(roo)) => {
            let doc_info = format_doc_actions(claude, opencode, roo);
            if !doc_info.is_empty() {
                format!(" | {}", doc_info)
            } else {
                String::new()
            }
        }
        _ => String::new(),
    };

    format!("- {}: {}{}", topic_name, skill_status, doc_status)
}

/// Format skill actions for all three services (Claude Code, OpenCode, and Roo Code).
fn format_skill_actions(
    claude: &SkillAction,
    opencode: &SkillAction,
    roo: &SkillAction,
) -> String {
    // All three same state shortcuts
    match (claude, opencode, roo) {
        // All created successfully
        (SkillAction::CreatedLink, SkillAction::CreatedLink, SkillAction::CreatedLink) => {
            return "skills linked to all".green().to_string();
        }

        // All already linked
        (
            SkillAction::NoneAlreadyLinked,
            SkillAction::NoneAlreadyLinked,
            SkillAction::NoneAlreadyLinked,
        ) => {
            return "skills already linked".dimmed().italic().to_string();
        }

        // All have local definitions
        (
            SkillAction::NoneLocalDefinition,
            SkillAction::NoneLocalDefinition,
            SkillAction::NoneLocalDefinition,
        ) => {
            return "local skill definitions exist".yellow().to_string();
        }

        // All invalid
        (
            SkillAction::NoneSkillDirectoryInvalid,
            SkillAction::NoneSkillDirectoryInvalid,
            SkillAction::NoneSkillDirectoryInvalid,
        ) => {
            return "skill directory invalid (no SKILL.md)".yellow().to_string();
        }

        // Any failure takes priority - show error message
        (SkillAction::FailedPermissionDenied(msg), _, _)
        | (_, SkillAction::FailedPermissionDenied(msg), _)
        | (_, _, SkillAction::FailedPermissionDenied(msg)) => {
            return format!("failed (permission denied: {})", msg)
                .red()
                .to_string();
        }

        (SkillAction::FailedOther(msg), _, _)
        | (_, SkillAction::FailedOther(msg), _)
        | (_, _, SkillAction::FailedOther(msg)) => {
            return format!("failed ({})", msg).red().to_string();
        }

        // Mixed states - fall through to per-service breakdown
        _ => {}
    }

    // Mixed states - show per-service breakdown
    let claude_status = format_action_status(claude, "Claude Code");
    let opencode_status = format_action_status(opencode, "OpenCode");
    let roo_status = format_action_status(roo, "Roo Code");
    format!("{}, {}, {}", claude_status, opencode_status, roo_status)
}

/// Format doc actions for all three services (Claude Code, OpenCode, and Roo Code).
fn format_doc_actions(claude: &SkillAction, opencode: &SkillAction, roo: &SkillAction) -> String {
    match (claude, opencode, roo) {
        // All created successfully
        (SkillAction::CreatedLink, SkillAction::CreatedLink, SkillAction::CreatedLink) => {
            "docs linked".green().to_string()
        }

        // All already linked
        (
            SkillAction::NoneAlreadyLinked,
            SkillAction::NoneAlreadyLinked,
            SkillAction::NoneAlreadyLinked,
        ) => "docs already linked".dimmed().to_string(),

        // All have local definitions
        (
            SkillAction::NoneLocalDefinition,
            SkillAction::NoneLocalDefinition,
            SkillAction::NoneLocalDefinition,
        ) => "local docs exist".yellow().to_string(),

        // Any failure in docs
        (SkillAction::FailedPermissionDenied(_), _, _)
        | (_, SkillAction::FailedPermissionDenied(_), _)
        | (_, _, SkillAction::FailedPermissionDenied(_))
        | (SkillAction::FailedOther(_), _, _)
        | (_, SkillAction::FailedOther(_), _)
        | (_, _, SkillAction::FailedOther(_)) => "doc linking failed".red().to_string(),

        // Mixed success states (some created, some already linked)
        (c, o, r)
            if matches!(c, SkillAction::CreatedLink | SkillAction::NoneAlreadyLinked)
                && matches!(o, SkillAction::CreatedLink | SkillAction::NoneAlreadyLinked)
                && matches!(r, SkillAction::CreatedLink | SkillAction::NoneAlreadyLinked) =>
        {
            "docs linked (partial)".green().to_string()
        }

        // Other combinations
        _ => String::new(),
    }
}

/// Format the status of a single action for a service.
fn format_action_status(action: &SkillAction, service: &str) -> String {
    match action {
        SkillAction::CreatedLink => format!("{}: {}", service.italic(), "created".green()),
        SkillAction::NoneAlreadyLinked => {
            format!("{}: {}", service.italic(), "already linked".dimmed())
        }
        SkillAction::NoneLocalDefinition => {
            format!("{}: {}", service.italic(), "local definition".yellow())
        }
        SkillAction::NoneSkillDirectoryInvalid => {
            format!("{}: {}", service.italic(), "invalid".yellow())
        }
        SkillAction::FailedPermissionDenied(msg) => {
            format!(
                "{}: {}",
                service.italic(),
                format!("permission denied ({})", msg).red()
            )
        }
        SkillAction::FailedOther(msg) => {
            format!(
                "{}: {}",
                service.italic(),
                format!("failed ({})", msg).red()
            )
        }
    }
}

/// Format link results as JSON.
///
/// Serializes the LinkResult structure to pretty-printed JSON for machine consumption
/// or structured logging.
///
/// # Arguments
///
/// * `result` - The link result containing all skill link operations
///
/// # Returns
///
/// A JSON string representation of the result, or an error if serialization fails
///
/// # Example
///
/// ```rust
/// use research_lib::link::{LinkResult, SkillLink, SkillAction};
/// use research_lib::link::format::format_json;
///
/// let mut result = LinkResult::new();
/// result.links.push(SkillLink::new(
///     "clap".to_string(),
///     SkillAction::CreatedLink,
///     SkillAction::NoneAlreadyLinked,
///     SkillAction::CreatedLink,
/// ));
///
/// let json = format_json(&result).unwrap();
/// println!("{}", json);
/// ```
pub fn format_json(result: &LinkResult) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_terminal_all_created() {
        let mut result = LinkResult::new();
        result.links.push(SkillLink::new(
            "clap".to_string(),
            SkillAction::CreatedLink,
            SkillAction::CreatedLink,
            SkillAction::CreatedLink,
        ));

        let output = format_terminal(&result);
        assert!(output.contains("clap"));
        assert!(output.contains("skills linked to all"));
    }

    #[test]
    fn test_format_terminal_mixed_created_and_linked() {
        let mut result = LinkResult::new();
        result.links.push(SkillLink::new(
            "serde".to_string(),
            SkillAction::CreatedLink,
            SkillAction::NoneAlreadyLinked,
            SkillAction::CreatedLink,
        ));

        let output = format_terminal(&result);
        assert!(output.contains("serde"));
        // Mixed state shows per-service breakdown
        assert!(output.contains("Claude Code"));
        assert!(output.contains("OpenCode"));
        assert!(output.contains("Roo Code"));
    }

    #[test]
    fn test_format_terminal_roo_only_created() {
        let mut result = LinkResult::new();
        result.links.push(SkillLink::new(
            "tokio".to_string(),
            SkillAction::NoneAlreadyLinked,
            SkillAction::NoneAlreadyLinked,
            SkillAction::CreatedLink,
        ));

        let output = format_terminal(&result);
        assert!(output.contains("tokio"));
        assert!(output.contains("Roo Code"));
        assert!(output.contains("created"));
    }

    #[test]
    fn test_format_terminal_all_already_linked() {
        let mut result = LinkResult::new();
        result.links.push(SkillLink::new(
            "tokio".to_string(),
            SkillAction::NoneAlreadyLinked,
            SkillAction::NoneAlreadyLinked,
            SkillAction::NoneAlreadyLinked,
        ));

        let output = format_terminal(&result);
        assert!(output.contains("tokio"));
        assert!(output.contains("skills already linked"));
    }

    #[test]
    fn test_format_terminal_all_local_definition() {
        let mut result = LinkResult::new();
        result.links.push(SkillLink::new(
            "actix".to_string(),
            SkillAction::NoneLocalDefinition,
            SkillAction::NoneLocalDefinition,
            SkillAction::NoneLocalDefinition,
        ));

        let output = format_terminal(&result);
        assert!(output.contains("actix"));
        assert!(output.contains("local skill definitions exist"));
    }

    #[test]
    fn test_format_terminal_all_invalid() {
        let mut result = LinkResult::new();
        result.links.push(SkillLink::new(
            "broken".to_string(),
            SkillAction::NoneSkillDirectoryInvalid,
            SkillAction::NoneSkillDirectoryInvalid,
            SkillAction::NoneSkillDirectoryInvalid,
        ));

        let output = format_terminal(&result);
        assert!(output.contains("broken"));
        assert!(output.contains("skill directory invalid"));
        assert!(output.contains("SKILL.md"));
    }

    #[test]
    fn test_format_terminal_one_failed_permission() {
        let mut result = LinkResult::new();
        result.links.push(SkillLink::new(
            "denied".to_string(),
            SkillAction::CreatedLink,
            SkillAction::FailedPermissionDenied("permission denied".to_string()),
            SkillAction::CreatedLink,
        ));

        let output = format_terminal(&result);
        assert!(output.contains("denied"));
        assert!(output.contains("permission denied"));
    }

    #[test]
    fn test_format_terminal_roo_failed() {
        let mut result = LinkResult::new();
        result.links.push(SkillLink::new(
            "roo-error".to_string(),
            SkillAction::CreatedLink,
            SkillAction::CreatedLink,
            SkillAction::FailedOther("Roo I/O error".to_string()),
        ));

        let output = format_terminal(&result);
        assert!(output.contains("roo-error"));
        assert!(output.contains("Roo I/O error"));
    }

    #[test]
    fn test_format_terminal_failed_other() {
        let mut result = LinkResult::new();
        result.links.push(SkillLink::new(
            "error".to_string(),
            SkillAction::FailedOther("I/O error".to_string()),
            SkillAction::CreatedLink,
            SkillAction::CreatedLink,
        ));

        let output = format_terminal(&result);
        assert!(output.contains("error"));
        assert!(output.contains("I/O error"));
    }

    #[test]
    fn test_format_terminal_mixed_actions() {
        let mut result = LinkResult::new();
        result.links.push(SkillLink::new(
            "clap".to_string(),
            SkillAction::CreatedLink,
            SkillAction::CreatedLink,
            SkillAction::CreatedLink,
        ));
        result.links.push(SkillLink::new(
            "serde".to_string(),
            SkillAction::NoneAlreadyLinked,
            SkillAction::CreatedLink,
            SkillAction::NoneAlreadyLinked,
        ));
        result.links.push(SkillLink::new(
            "tokio".to_string(),
            SkillAction::NoneAlreadyLinked,
            SkillAction::NoneAlreadyLinked,
            SkillAction::NoneAlreadyLinked,
        ));
        result.links.push(SkillLink::new(
            "actix".to_string(),
            SkillAction::NoneLocalDefinition,
            SkillAction::NoneLocalDefinition,
            SkillAction::NoneLocalDefinition,
        ));

        let output = format_terminal(&result);
        assert!(output.contains("clap"));
        assert!(output.contains("serde"));
        assert!(output.contains("tokio"));
        assert!(output.contains("actix"));
    }

    #[test]
    fn test_format_terminal_empty_result() {
        let result = LinkResult::new();
        let output = format_terminal(&result);
        assert!(output.contains("No topics processed"));
    }

    #[test]
    fn test_format_terminal_with_errors() {
        let mut result = LinkResult::new();
        result.links.push(SkillLink::new(
            "failed".to_string(),
            SkillAction::FailedPermissionDenied("permission denied".to_string()),
            SkillAction::FailedOther("I/O error".to_string()),
            SkillAction::CreatedLink,
        ));

        let output = format_terminal(&result);
        assert!(output.contains("Summary"));
        assert!(output.contains("1 failed"));
    }

    #[test]
    fn test_format_terminal_color_codes_verified() {
        // This test verifies that color codes are present in the output
        // by checking for ANSI escape sequences
        let mut result = LinkResult::new();
        result.links.push(SkillLink::new(
            "test".to_string(),
            SkillAction::CreatedLink,
            SkillAction::CreatedLink,
            SkillAction::CreatedLink,
        ));

        let output = format_terminal(&result);
        // ANSI escape codes should be present (owo-colors adds them)
        // Format: \x1b[...m for color codes
        assert!(output.contains("\x1b["));
    }

    #[test]
    fn test_format_json_valid_structure() {
        let mut result = LinkResult::new();
        result.links.push(SkillLink::new(
            "clap".to_string(),
            SkillAction::CreatedLink,
            SkillAction::NoneAlreadyLinked,
            SkillAction::CreatedLink,
        ));

        let json = format_json(&result).unwrap();
        assert!(json.contains("\"name\""));
        assert!(json.contains("\"clap\""));
        assert!(json.contains("\"claude_action\""));
        assert!(json.contains("\"opencode_action\""));
        assert!(json.contains("\"roo_action\""));
    }

    #[test]
    fn test_format_json_roundtrip() {
        let mut result = LinkResult::new();
        result.links.push(SkillLink::new(
            "clap".to_string(),
            SkillAction::CreatedLink,
            SkillAction::NoneAlreadyLinked,
            SkillAction::CreatedLink,
        ));
        result
            .errors
            .push(("error-topic".to_string(), "test error".to_string()));

        let json = format_json(&result).unwrap();
        let deserialized: LinkResult = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.links.len(), 1);
        assert_eq!(deserialized.links[0].name, "clap");
        assert_eq!(deserialized.links[0].roo_action, SkillAction::CreatedLink);
        assert_eq!(deserialized.errors.len(), 1);
        assert_eq!(deserialized.errors[0].0, "error-topic");
    }

    #[test]
    fn test_format_json_all_action_variants() {
        let mut result = LinkResult::new();
        result.links.push(SkillLink::new(
            "created".to_string(),
            SkillAction::CreatedLink,
            SkillAction::CreatedLink,
            SkillAction::CreatedLink,
        ));
        result.links.push(SkillLink::new(
            "linked".to_string(),
            SkillAction::NoneAlreadyLinked,
            SkillAction::NoneAlreadyLinked,
            SkillAction::NoneAlreadyLinked,
        ));
        result.links.push(SkillLink::new(
            "local".to_string(),
            SkillAction::NoneLocalDefinition,
            SkillAction::NoneLocalDefinition,
            SkillAction::NoneLocalDefinition,
        ));
        result.links.push(SkillLink::new(
            "invalid".to_string(),
            SkillAction::NoneSkillDirectoryInvalid,
            SkillAction::NoneSkillDirectoryInvalid,
            SkillAction::NoneSkillDirectoryInvalid,
        ));
        result.links.push(SkillLink::new(
            "permission".to_string(),
            SkillAction::FailedPermissionDenied("denied".to_string()),
            SkillAction::CreatedLink,
            SkillAction::CreatedLink,
        ));
        result.links.push(SkillLink::new(
            "other".to_string(),
            SkillAction::CreatedLink,
            SkillAction::FailedOther("error".to_string()),
            SkillAction::CreatedLink,
        ));

        let json = format_json(&result).unwrap();

        // Verify all variants are present
        assert!(json.contains("created_link"));
        assert!(json.contains("none_already_linked"));
        assert!(json.contains("none_local_definition"));
        assert!(json.contains("none_skill_directory_invalid"));
        assert!(json.contains("failed_permission_denied"));
        assert!(json.contains("failed_other"));
    }

    #[test]
    fn test_format_json_errors_included() {
        let mut result = LinkResult::new();
        result
            .errors
            .push(("topic1".to_string(), "error1".to_string()));
        result
            .errors
            .push(("topic2".to_string(), "error2".to_string()));

        let json = format_json(&result).unwrap();
        assert!(json.contains("\"errors\""));
        assert!(json.contains("topic1"));
        assert!(json.contains("error1"));
        assert!(json.contains("topic2"));
        assert!(json.contains("error2"));
    }

    #[test]
    fn test_format_json_empty_result() {
        let result = LinkResult::new();
        let json = format_json(&result).unwrap();

        // Should be valid JSON with empty arrays
        let deserialized: LinkResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.links.len(), 0);
        assert_eq!(deserialized.errors.len(), 0);
    }

    // Tests for stale symlink reporting
    #[test]
    fn test_format_terminal_with_stale_symlinks_removed() {
        let mut result = LinkResult::new();
        result
            .stale_removed
            .push("/home/user/.claude/skills/old-skill".to_string());
        result
            .stale_removed
            .push("/home/user/.config/opencode/skill/another".to_string());

        let output = format_terminal(&result);
        assert!(output.contains("Cleaned up 2 stale symlink"));
    }

    #[test]
    fn test_format_terminal_no_stale_symlinks() {
        let result = LinkResult::new();
        let output = format_terminal(&result);
        // Should not mention stale symlinks when there are none
        assert!(!output.contains("stale"));
    }

    // Tests for doc action formatting
    #[test]
    fn test_format_terminal_with_doc_actions_all_created() {
        let mut result = LinkResult::new();
        result.links.push(SkillLink::new_with_docs(
            "clap".to_string(),
            SkillAction::CreatedLink,
            SkillAction::CreatedLink,
            SkillAction::CreatedLink,
            Some(SkillAction::CreatedLink),
            Some(SkillAction::CreatedLink),
            Some(SkillAction::CreatedLink),
        ));

        let output = format_terminal(&result);
        assert!(output.contains("clap"));
        assert!(output.contains("docs linked"));
    }

    #[test]
    fn test_format_terminal_with_doc_actions_all_already_linked() {
        let mut result = LinkResult::new();
        result.links.push(SkillLink::new_with_docs(
            "serde".to_string(),
            SkillAction::NoneAlreadyLinked,
            SkillAction::NoneAlreadyLinked,
            SkillAction::NoneAlreadyLinked,
            Some(SkillAction::NoneAlreadyLinked),
            Some(SkillAction::NoneAlreadyLinked),
            Some(SkillAction::NoneAlreadyLinked),
        ));

        let output = format_terminal(&result);
        assert!(output.contains("serde"));
        assert!(output.contains("docs already linked"));
    }

    #[test]
    fn test_format_terminal_without_doc_actions() {
        let mut result = LinkResult::new();
        result.links.push(SkillLink::new(
            "tokio".to_string(),
            SkillAction::CreatedLink,
            SkillAction::CreatedLink,
            SkillAction::CreatedLink,
        ));

        let output = format_terminal(&result);
        assert!(output.contains("tokio"));
        // Should not mention docs when there are no doc actions
        assert!(!output.contains("docs linked"));
    }

    #[test]
    fn test_format_json_with_doc_actions() {
        let mut result = LinkResult::new();
        result.links.push(SkillLink::new_with_docs(
            "clap".to_string(),
            SkillAction::CreatedLink,
            SkillAction::CreatedLink,
            SkillAction::CreatedLink,
            Some(SkillAction::CreatedLink),
            Some(SkillAction::NoneAlreadyLinked),
            Some(SkillAction::CreatedLink),
        ));

        let json = format_json(&result).unwrap();
        assert!(json.contains("\"claude_doc_action\""));
        assert!(json.contains("\"opencode_doc_action\""));
        assert!(json.contains("\"roo_doc_action\""));
    }

    #[test]
    fn test_format_json_with_stale_symlinks() {
        let mut result = LinkResult::new();
        result.stale_removed.push("/path/to/removed".to_string());
        result.stale_failed.push((
            "/path/to/failed".to_string(),
            "permission denied".to_string(),
        ));

        let json = format_json(&result).unwrap();
        assert!(json.contains("\"stale_removed\""));
        assert!(json.contains("/path/to/removed"));
        assert!(json.contains("\"stale_failed\""));
        assert!(json.contains("/path/to/failed"));
    }
}
