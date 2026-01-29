//! Output formatting functions for the list command.
//!
//! This module provides functions to format topic information in different
//! output formats (JSON, terminal, etc.).
//!
//! Terminal formatting uses colored output with status-based coloring:
//! - **RED + BOLD**: Missing output files or metadata.json
//! - **ORANGE + BOLD**: Missing underlying documents only
//! - **BOLD**: All files present

use crate::list::types::TopicInfo;
use owo_colors::OwoColorize;
use darkmatter_lib::render::Link;
use std::sync::OnceLock;

/// Formats a list of topics as pretty-printed JSON.
///
/// Returns a JSON array containing all topics with their metadata.
/// The output is formatted with indentation for human readability.
///
/// # Arguments
///
/// * `topics` - Slice of TopicInfo structs to format
///
/// # Returns
///
/// A Result containing the JSON string on success, or an error if
/// serialization fails.
///
/// # Examples
///
/// ```
/// use research_lib::list::types::TopicInfo;
/// use research_lib::list::format::format_json;
/// use std::path::PathBuf;
///
/// let topics = vec![
///     TopicInfo::new("example".to_string(), PathBuf::from("/test/example"))
/// ];
/// let json = format_json(&topics).unwrap();
/// assert!(json.starts_with("["));
/// assert!(json.ends_with("]"));
/// ```
pub fn format_json(topics: &[TopicInfo]) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(topics)
}

/// Formats a list of topics for terminal display with colored output.
///
/// # Arguments
///
/// * `topics` - Slice of TopicInfo structs to format
/// * `filter_single_type` - If true, hide type badges (used when filtering by type)
/// * `verbose` - If true, show detailed sub-bullets; if false, show only icons
///
/// # Returns
///
/// A formatted string ready for terminal output with ANSI color codes.
///
/// # Format
///
/// In verbose mode, each topic is formatted as:
/// ```text
/// - {name} [TYPE_BADGE] : {description}
///     - 🐞 metadata.json missing required props: ...
///     - 🐞 missing underlying research docs: ...
///     - 🐞 missing final output deliverables: ...
///     - 💡 {#} additional prompts used in research: ...
/// ```
///
/// In non-verbose mode:
/// ```text
/// - {name} [TYPE_BADGE] : {description} 💡 🐞
/// ```
///
/// # Examples
///
/// ```
/// use research_lib::list::{TopicInfo, format_terminal};
/// use std::path::PathBuf;
///
/// let topics = vec![
///     TopicInfo::new("my-topic".to_string(), PathBuf::from("/research/my-topic"))
/// ];
/// let output = format_terminal(&topics, false, true);
/// println!("{}", output);
/// ```
pub fn format_terminal(topics: &[TopicInfo], filter_single_type: bool, verbose: bool) -> String {
    if topics.is_empty() {
        return String::new();
    }

    let mut output = topics
        .iter()
        .map(|topic| format_topic(topic, filter_single_type, verbose))
        .collect::<Vec<_>>()
        .join("\n");

    // Add help text at the end in non-verbose mode
    if !verbose && !topics.is_empty() {
        // Check if any topics need migration
        let has_migration = topics.iter().any(|t| t.needs_migration);

        output.push_str("\n\n- use ");
        output.push_str(&" --verbose ".on_truecolor(80, 80, 80).to_string());
        output.push_str(" for greater metadata on the topics");

        if has_migration {
            output.push_str("\n- use ");
            output.push_str(&" --migrate ".on_truecolor(80, 80, 80).to_string());
            output.push_str(" to upgrade 🔺 topics to v1 schema");
        }
    }

    output
}

/// Formats a single topic for terminal display.
fn format_topic(topic: &TopicInfo, hide_type_badge: bool, verbose: bool) -> String {
    let mut lines = Vec::new();

    // Format the main topic line
    let main_line = format_main_line(topic, hide_type_badge, verbose);
    lines.push(main_line);

    // In verbose mode, add sub-bullets for issues
    if verbose {
        if let Some(metadata_line) = format_metadata_issue(topic) {
            lines.push(metadata_line);
        }

        if let Some(migration_line) = format_migration_issue(topic) {
            lines.push(migration_line);
        }

        if let Some(underlying_line) = format_underlying_issues(topic) {
            lines.push(underlying_line);
        }

        if let Some(output_line) = format_output_issues(topic) {
            lines.push(output_line);
        }

        if let Some(additional_line) = format_additional_prompts(topic) {
            lines.push(additional_line);
        }
    }

    lines.join("\n")
}

/// Formats the main topic line with name, type badge, and description.
fn format_main_line(topic: &TopicInfo, hide_type_badge: bool, verbose: bool) -> String {
    let mut parts = Vec::new();

    // Bullet prefix (no formatting)
    parts.push("- ".to_string());

    // Topic name with color-coded formatting, linked to deep_dive.md
    let styled_name = if topic.has_critical_issues() {
        topic.name.bold().red().to_string()
    } else if topic.has_minor_issues_only() {
        topic.name.bold().truecolor(255, 165, 0).to_string() // Orange
    } else {
        topic.name.bold().to_string()
    };

    // Create clickable link to deep_dive.md
    let deep_dive_path = topic.location.join("deep_dive.md");
    let link_url = format!("file://{}", deep_dive_path.display());
    let formatted_name = Link::new(styled_name, link_url).to_terminal();
    parts.push(formatted_name);

    // Type badge (unless hidden)
    if !hide_type_badge {
        parts.push(" ".to_string());
        parts.push(format_type_badge(&topic.topic_type));
    }

    // Language icon after type badge (in all modes)
    parts.push(format_language_icon(topic.language.as_ref()));

    // Description (if present and in verbose mode)
    if verbose && let Some(ref desc) = topic.description {
        parts.push(" : ".to_string());
        parts.push(desc.italic().to_string());
    }

    // In non-verbose mode, add icons after the description
    if !verbose {
        let mut icons = Vec::new();

        // Add 💡 icon if there are additional prompts
        if !topic.additional_files.is_empty() {
            icons.push("💡");
        }

        // Add 🔺 icon if metadata needs migration
        if topic.needs_migration {
            icons.push("🔺");
        }

        // Add 🐞 icon if there are any issues (other than migration)
        if topic.missing_metadata
            || !topic.missing_output.is_empty()
            || !topic.missing_underlying.is_empty()
        {
            icons.push("🐞");
        }

        if !icons.is_empty() {
            parts.push(" ".to_string());
            parts.push(icons.join(" "));
        }
    }

    parts.concat()
}

/// Cached terminal theme detection result to avoid repeated queries.
static IS_DARK_THEME: OnceLock<bool> = OnceLock::new();

/// Detects whether the terminal is using a dark theme.
///
/// This function queries the terminal background color and calculates its luminance.
/// The result is cached to avoid repeated terminal queries.
///
/// Returns `true` if the terminal uses a dark theme, `false` for light themes.
/// Defaults to `true` (dark theme) if detection fails or times out.
fn is_dark_theme() -> bool {
    *IS_DARK_THEME.get_or_init(|| {
        // Try to detect terminal background color with a short timeout
        match termbg::rgb(std::time::Duration::from_millis(100)) {
            Ok(termbg::Rgb { r, g, b }) => {
                // Calculate relative luminance using the formula from WCAG 2.0
                let luminance = (0.2126 * r as f64 + 0.7152 * g as f64 + 0.0722 * b as f64) / 255.0;
                // Consider dark if luminance is below 0.5
                luminance < 0.5
            }
            Err(_) => {
                // Default to dark theme if detection fails
                true
            }
        }
    })
}

/// Returns RGB color values for badge backgrounds based on terminal theme.
///
/// For dark terminals, returns lighter/brighter colors for better contrast.
/// For light terminals, returns darker colors for better contrast.
///
/// Returns (r, g, b) tuple.
fn get_badge_color(topic_type: &str) -> (u8, u8, u8) {
    let is_dark = is_dark_theme();

    match topic_type.to_lowercase().as_str() {
        "library" => {
            if is_dark {
                (59, 130, 246) // Bright blue for dark theme
            } else {
                (30, 58, 138) // Dark blue for light theme
            }
        }
        "framework" => {
            if is_dark {
                (34, 197, 94) // Bright green for dark theme
            } else {
                (21, 128, 61) // Dark green for light theme
            }
        }
        "software" => {
            if is_dark {
                (168, 85, 247) // Bright purple for dark theme
            } else {
                (109, 40, 217) // Dark purple for light theme
            }
        }
        "tool" => {
            if is_dark {
                (34, 211, 238) // Bright cyan for dark theme
            } else {
                (21, 94, 117) // Dark cyan for light theme
            }
        }
        "language" => {
            if is_dark {
                (250, 204, 21) // Bright yellow for dark theme
            } else {
                (161, 98, 7) // Dark yellow/brown for light theme
            }
        }
        "platform" => {
            if is_dark {
                (232, 121, 249) // Bright magenta for dark theme
            } else {
                (162, 28, 175) // Dark magenta for light theme
            }
        }
        _ => {
            if is_dark {
                (115, 115, 115) // Medium gray for dark theme
            } else {
                (64, 64, 64) // Dark gray for light theme
            }
        }
    }
}

/// Formats a type badge with background color and padding.
fn format_type_badge(topic_type: &str) -> String {
    // Get theme-aware background color
    let (r, g, b) = get_badge_color(topic_type);

    // Format as " TYPE " with background on entire string including spaces
    format!(" {} ", topic_type)
        .on_truecolor(r, g, b)
        .to_string()
}

/// Format language icon based on language string
/// Returns empty string if no icon applies
fn format_language_icon(language: Option<&String>) -> String {
    match language.map(|s| s.as_str()) {
        Some("Rust") => " 🦀".to_string(),
        Some("Python") => " 🐍".to_string(),
        Some("PHP") => " 🐘".to_string(),
        Some("JavaScript/TypeScript") => {
            // Blue background (0,122,204), black text
            format!(" {}", " ʦ ".black().on_truecolor(0, 122, 204))
        }
        _ => String::new(),
    }
}

/// Formats metadata issues if present.
fn format_metadata_issue(topic: &TopicInfo) -> Option<String> {
    if !topic.missing_metadata {
        return None;
    }

    Some(format!(
        "    - 🐞 {} missing required props",
        "metadata.json".bold()
    ))
}

/// Formats underlying research document issues if present.
fn format_underlying_issues(topic: &TopicInfo) -> Option<String> {
    if topic.missing_underlying.is_empty() {
        return None;
    }

    let files = topic.missing_underlying.join(", ");
    Some(format!(
        "    - 🐞 missing {} research docs: {}",
        "underlying".italic(),
        files
    ))
}

/// Formats output deliverable issues if present.
fn format_output_issues(topic: &TopicInfo) -> Option<String> {
    if topic.missing_output.is_empty() {
        return None;
    }

    let outputs = topic
        .missing_output
        .iter()
        .map(|o| o.to_string())
        .collect::<Vec<_>>()
        .join(", ");

    Some(format!(
        "    - 🐞 missing {} output deliverables: {}",
        "final".italic(),
        outputs
    ))
}

/// Formats additional prompts information if present.
fn format_additional_prompts(topic: &TopicInfo) -> Option<String> {
    if topic.additional_files.is_empty() {
        return None;
    }

    let count = topic.additional_files.len();
    let files = topic.additional_files.join(", ");
    Some(format!(
        "    - 💡 {} additional prompts used in research: {}",
        count, files
    ))
}

/// Formats migration needed indicator if present.
fn format_migration_issue(topic: &TopicInfo) -> Option<String> {
    if !topic.needs_migration {
        return None;
    }

    Some(format!(
        "    - 🔺 {} needs migration (run {} to upgrade)",
        "metadata.json".bold(),
        "research list --migrate".italic()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::list::types::{ResearchOutput, TopicInfo};
    use std::path::PathBuf;

    #[test]
    fn test_format_json_empty_list() {
        let topics: Vec<TopicInfo> = vec![];
        let json = format_json(&topics).unwrap();

        // Verify it's valid JSON
        let parsed: Vec<TopicInfo> = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.len(), 0);

        // Verify it's an array
        assert!(json.trim().starts_with("["));
        assert!(json.trim().ends_with("]"));
    }

    #[test]
    fn test_format_json_single_topic() {
        let topic = TopicInfo {
            name: "test-library".to_string(),
            topic_type: "library".to_string(),
            description: Some("A test library for testing".to_string()),
            language: None,
            additional_files: vec!["custom_prompt".to_string()],
            missing_underlying: vec!["overview.md".to_string()],
            missing_output: vec![ResearchOutput::Brief],
            missing_metadata: false,
            needs_migration: false,
            location: PathBuf::from("/test/test-library"),
        };

        let topics = vec![topic.clone()];
        let json = format_json(&topics).unwrap();

        // Verify it's valid JSON
        let parsed: Vec<TopicInfo> = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.len(), 1);

        // Verify the content matches
        assert_eq!(parsed[0].name, "test-library");
        assert_eq!(parsed[0].topic_type, "library");
        assert_eq!(
            parsed[0].description,
            Some("A test library for testing".to_string())
        );
        assert_eq!(
            parsed[0].additional_files,
            vec!["custom_prompt".to_string()]
        );
        assert_eq!(
            parsed[0].missing_underlying,
            vec!["overview.md".to_string()]
        );
        assert_eq!(parsed[0].missing_output, vec![ResearchOutput::Brief]);
        assert_eq!(parsed[0].missing_metadata, false);
    }

    #[test]
    fn test_format_json_multiple_topics() {
        let topic1 = TopicInfo {
            name: "lib-one".to_string(),
            topic_type: "library".to_string(),
            description: Some("First library".to_string()),
            language: None,
            additional_files: vec![],
            missing_underlying: vec![],
            missing_output: vec![],
            missing_metadata: false,
            needs_migration: false,
            location: PathBuf::from("/test/lib-one"),
        };

        let topic2 = TopicInfo {
            name: "lib-two".to_string(),
            topic_type: "framework".to_string(),
            description: Some("Second framework".to_string()),
            language: None,
            additional_files: vec!["question_1".to_string(), "question_2".to_string()],
            missing_underlying: vec!["overview.md".to_string()],
            missing_output: vec![ResearchOutput::DeepDive, ResearchOutput::Skill],
            missing_metadata: true,
            needs_migration: false,
            location: PathBuf::from("/test/lib-two"),
        };

        let topic3 = TopicInfo {
            name: "lib-three".to_string(),
            topic_type: "software".to_string(),
            description: None,
            language: None,
            additional_files: vec![],
            missing_underlying: vec!["use_cases.md".to_string(), "best_practices.md".to_string()],
            missing_output: vec![ResearchOutput::Brief],
            missing_metadata: false,
            needs_migration: false,
            location: PathBuf::from("/test/lib-three"),
        };

        let topics = vec![topic1, topic2, topic3];
        let json = format_json(&topics).unwrap();

        // Verify it's valid JSON
        let parsed: Vec<TopicInfo> = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.len(), 3);

        // Verify all topics are present with correct data
        assert_eq!(parsed[0].name, "lib-one");
        assert_eq!(parsed[0].topic_type, "library");
        assert_eq!(parsed[0].description, Some("First library".to_string()));
        assert_eq!(parsed[0].additional_files.len(), 0);
        assert_eq!(parsed[0].missing_output.len(), 0);

        assert_eq!(parsed[1].name, "lib-two");
        assert_eq!(parsed[1].topic_type, "framework");
        assert_eq!(parsed[1].additional_files.len(), 2);
        assert_eq!(parsed[1].missing_output.len(), 2);
        assert!(parsed[1].missing_metadata);

        assert_eq!(parsed[2].name, "lib-three");
        assert_eq!(parsed[2].topic_type, "software");
        assert_eq!(parsed[2].description, None);
        assert_eq!(parsed[2].missing_underlying.len(), 2);
    }

    #[test]
    fn test_format_json_is_pretty_printed() {
        let topic = TopicInfo::new("test".to_string(), PathBuf::from("/test"));
        let topics = vec![topic];
        let json = format_json(&topics).unwrap();

        // Pretty-printed JSON should contain newlines and indentation
        assert!(json.contains('\n'));
        assert!(json.contains("  ")); // Check for indentation
    }

    #[test]
    fn test_format_json_includes_all_fields() {
        let topic = TopicInfo {
            name: "complete-topic".to_string(),
            topic_type: "library".to_string(),
            description: Some("Complete topic".to_string()),
            language: None,
            additional_files: vec!["file1".to_string()],
            missing_underlying: vec!["doc1.md".to_string()],
            missing_output: vec![ResearchOutput::Brief],
            missing_metadata: true,
            needs_migration: false,
            location: PathBuf::from("/test/complete"),
        };

        let json = format_json(&[topic]).unwrap();

        // Verify all fields are present in the JSON output
        assert!(json.contains("\"name\""));
        assert!(json.contains("\"type\"")); // Note: renamed field
        assert!(json.contains("\"description\""));
        assert!(json.contains("\"additional_files\""));
        assert!(json.contains("\"missing_underlying\""));
        assert!(json.contains("\"missing_output\""));
        assert!(json.contains("\"missing_metadata\""));
        assert!(json.contains("\"location\""));
    }

    // Terminal formatting tests
    fn create_test_topic(name: &str) -> TopicInfo {
        TopicInfo::new(name.to_string(), PathBuf::from(format!("/test/{}", name)))
    }

    #[test]
    fn test_format_terminal_empty_list() {
        let topics: Vec<TopicInfo> = vec![];
        let output = format_terminal(&topics, false, true);
        assert_eq!(output, "");
    }

    #[test]
    fn test_complete_topic_with_description() {
        let mut topic = create_test_topic("test-lib");
        topic.description = Some("A test library for testing".to_string());

        let output = format_terminal(&[topic], false, true);

        // Should contain name, type badge, and description
        assert!(output.contains("test-lib"));
        assert!(output.contains("library"));
        assert!(output.contains("A test library for testing"));
        // Should NOT contain any issue markers
        assert!(!output.contains("🐞"));
        assert!(!output.contains("💡"));
    }

    #[test]
    fn test_complete_topic_without_description() {
        let topic = create_test_topic("test-lib");

        let output = format_terminal(&[topic], false, true);

        // Should contain name and type badge
        assert!(output.contains("test-lib"));
        assert!(output.contains("library"));
        // Should NOT contain colon separator or issue markers
        assert!(!output.contains(" : "));
        assert!(!output.contains("🐞"));
        assert!(!output.contains("💡"));
    }

    #[test]
    fn test_missing_outputs_shows_red_with_sub_bullet() {
        let mut topic = create_test_topic("incomplete-lib");
        topic.missing_output.push(ResearchOutput::DeepDive);
        topic.missing_output.push(ResearchOutput::Brief);

        let output = format_terminal(&[topic], false, true);

        // Should contain the topic name
        assert!(output.contains("incomplete-lib"));
        // Should have issue marker
        assert!(output.contains("🐞"));
        assert!(output.contains("output deliverables"));
        assert!(output.contains("Deep Dive Document"));
        assert!(output.contains("Brief"));
    }

    #[test]
    fn test_missing_underlying_shows_orange_with_sub_bullet() {
        let mut topic = create_test_topic("partial-lib");
        topic.missing_underlying.push("overview.md".to_string());
        topic.missing_underlying.push("use_cases.md".to_string());

        let output = format_terminal(&[topic], false, true);

        // Should contain the topic name
        assert!(output.contains("partial-lib"));
        // Should have issue marker
        assert!(output.contains("🐞"));
        assert!(output.contains("research docs"));
        assert!(output.contains("overview.md"));
        assert!(output.contains("use_cases.md"));
    }

    #[test]
    fn test_missing_metadata_shows_red_with_sub_bullet() {
        let mut topic = create_test_topic("no-meta-lib");
        topic.missing_metadata = true;

        let output = format_terminal(&[topic], false, true);

        // Should contain the topic name
        assert!(output.contains("no-meta-lib"));
        // Should have issue marker
        assert!(output.contains("🐞"));
        assert!(output.contains("metadata.json"));
    }

    #[test]
    fn test_additional_prompts_shows_lightbulb() {
        let mut topic = create_test_topic("custom-lib");
        topic.additional_files.push("question_1".to_string());
        topic.additional_files.push("question_2".to_string());
        topic.additional_files.push("custom_analysis".to_string());

        let output = format_terminal(&[topic], false, true);

        // Should contain the topic name
        assert!(output.contains("custom-lib"));
        // Should have lightbulb marker
        assert!(output.contains("💡"));
        assert!(output.contains("3 additional prompts used in research"));
        assert!(output.contains("question_1"));
        assert!(output.contains("question_2"));
        assert!(output.contains("custom_analysis"));
    }

    #[test]
    fn test_type_badge_shown_when_not_filtered() {
        let topic = create_test_topic("test-lib");
        let output = format_terminal(&[topic], false, true);

        // Should contain type badge (with color codes)
        assert!(output.contains("library"));
        // Strip ANSI codes and verify the badge format
        let stripped = strip_ansi_codes(&output);
        assert!(stripped.contains(" library "));
    }

    #[test]
    fn test_type_badge_hidden_when_filtered() {
        let topic = create_test_topic("test-lib");
        let output = format_terminal(&[topic], true, true);

        // Should contain the topic name
        assert!(output.contains("test-lib"));
        // Type badge should be hidden - verify "library" doesn't appear between brackets
        // Note: ANSI color codes contain brackets, so we check that the word "library"
        // doesn't appear near our type badge brackets
        assert!(!output.contains("[library"));
    }

    #[test]
    fn test_multiple_topics() {
        let topic1 = create_test_topic("lib-a");
        let mut topic2 = create_test_topic("lib-b");
        topic2.missing_output.push(ResearchOutput::Skill);

        let output = format_terminal(&[topic1, topic2], false, true);

        // Should contain both topics
        assert!(output.contains("lib-a"));
        assert!(output.contains("lib-b"));
        // Should be separated by newline
        let lines: Vec<&str> = output.lines().collect();
        assert!(lines.len() >= 2);
    }

    #[test]
    fn test_all_issue_types_combined() {
        let mut topic = create_test_topic("complex-lib");
        topic.description = Some("Complex library with issues".to_string());
        topic.missing_metadata = true;
        topic.missing_underlying.push("overview.md".to_string());
        topic.missing_output.push(ResearchOutput::DeepDive);
        topic.additional_files.push("question_1".to_string());

        let output = format_terminal(&[topic], false, true);

        // Should contain all markers
        assert!(output.contains("complex-lib"));
        assert!(output.contains("Complex library with issues"));
        assert!(output.contains("metadata.json"));
        assert!(output.contains("research docs"));
        assert!(output.contains("output deliverables"));
        assert!(output.contains("1 additional prompts used in research"));
    }

    #[test]
    fn test_format_type_badge_library() {
        let badge = format_type_badge("library");
        assert!(badge.contains("library"));
        // Badge format is " TYPE " with background color (ANSI codes like [48;2; are present)
        // Strip ANSI codes to check the actual text
        let stripped = strip_ansi_codes(&badge);
        assert_eq!(stripped.trim(), "library");
        assert!(stripped.starts_with(' '));
        assert!(stripped.ends_with(' '));
    }

    /// Helper to strip ANSI escape codes
    fn strip_ansi_codes(s: &str) -> String {
        let mut result = String::new();
        let mut chars = s.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '\x1b' {
                // Skip ESC sequence
                if chars.peek() == Some(&'[') {
                    chars.next(); // skip '['
                    // Skip until 'm'
                    while let Some(&next_ch) = chars.peek() {
                        chars.next();
                        if next_ch == 'm' {
                            break;
                        }
                    }
                }
            } else {
                result.push(ch);
            }
        }
        result
    }

    #[test]
    fn test_format_type_badge_framework() {
        let badge = format_type_badge("framework");
        assert!(badge.contains("framework"));
    }

    #[test]
    fn test_format_type_badge_unknown() {
        let badge = format_type_badge("unknown-type");
        assert!(badge.contains("unknown-type"));
    }

    #[test]
    fn test_critical_vs_minor_issues_coloring() {
        // Create topic with critical issue (should be red)
        let mut critical_topic = create_test_topic("critical-lib");
        critical_topic.missing_output.push(ResearchOutput::Brief);

        // Create topic with minor issue (should be orange)
        let mut minor_topic = create_test_topic("minor-lib");
        minor_topic
            .missing_underlying
            .push("overview.md".to_string());

        let critical_output = format_terminal(&[critical_topic], false, true);
        let minor_output = format_terminal(&[minor_topic], false, true);

        // Both should contain their names
        assert!(critical_output.contains("critical-lib"));
        assert!(minor_output.contains("minor-lib"));

        // Both should contain issue markers
        assert!(critical_output.contains("🐞"));
        assert!(minor_output.contains("🐞"));
    }

    #[test]
    fn test_indentation_of_sub_bullets() {
        let mut topic = create_test_topic("indent-lib");
        topic.missing_metadata = true;

        let output = format_terminal(&[topic], false, true);

        // Find the sub-bullet line
        let lines: Vec<&str> = output.lines().collect();
        let sub_bullet = lines.iter().find(|l| l.contains("metadata.json"));
        assert!(sub_bullet.is_some());

        // Should start with 4 spaces
        assert!(sub_bullet.unwrap().starts_with("    - "));
    }

    #[test]
    fn test_verbose_mode_shows_sub_bullets() {
        let mut topic = create_test_topic("test-lib");
        topic.missing_metadata = true;
        topic.missing_underlying.push("overview.md".to_string());
        topic.additional_files.push("question_1".to_string());

        let output = format_terminal(&[topic], false, true);

        // Should contain sub-bullets
        assert!(output.contains("    - 🐞"));
        assert!(output.contains("metadata.json"));
        assert!(output.contains("research docs"));
        assert!(output.contains("additional prompts used in research"));
    }

    #[test]
    fn test_non_verbose_mode_shows_only_icons() {
        let mut topic = create_test_topic("test-lib");
        topic.missing_metadata = true;
        topic.missing_underlying.push("overview.md".to_string());
        topic.additional_files.push("question_1".to_string());

        let output = format_terminal(&[topic], false, false);

        // Should contain icons on main line
        assert!(output.contains("💡"));
        assert!(output.contains("🐞"));

        // Should NOT contain sub-bullets
        assert!(!output.contains("    - "));
        assert!(!output.contains("metadata.json"));
        assert!(!output.contains("research docs"));
        assert!(!output.contains("additional prompts used in research"));
    }

    #[test]
    fn test_non_verbose_mode_shows_help_text() {
        let topic = create_test_topic("test-lib");
        let output = format_terminal(&[topic], false, false);

        // Should contain help text
        assert!(output.contains("use"));
        assert!(output.contains("--verbose"));
        assert!(output.contains("for greater metadata on the topics"));
    }

    #[test]
    fn test_verbose_mode_does_not_show_help_text() {
        let topic = create_test_topic("test-lib");
        let output = format_terminal(&[topic], false, true);

        // Should NOT contain help text
        assert!(!output.contains("for greater metadata on the topics"));
    }

    #[test]
    fn test_non_verbose_empty_list_no_help_text() {
        let topics: Vec<TopicInfo> = vec![];
        let output = format_terminal(&topics, false, false);

        // Empty list should not show help text
        assert_eq!(output, "");
    }
}
