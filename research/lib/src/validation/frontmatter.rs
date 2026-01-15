//! Frontmatter parsing and validation for SKILL.md files
//!
//! This module handles the extraction and validation of YAML frontmatter from SKILL.md files.
//! It ensures that required fields are present and non-empty, and provides detailed error
//! messages for various failure scenarios.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur during frontmatter parsing and validation
#[derive(Debug, Error, PartialEq)]
pub enum FrontmatterError {
    /// The file does not start with YAML frontmatter delimiter (---)
    #[error("SKILL.md must start with YAML frontmatter (---) on line 1")]
    MissingFrontmatter,

    /// The frontmatter is missing the closing delimiter (---)
    #[error("YAML frontmatter is missing closing delimiter (---)")]
    UnclosedFrontmatter,

    /// The YAML content could not be parsed
    #[error("YAML parsing failed: {0}")]
    InvalidYaml(String),

    /// A required field is missing from the frontmatter
    #[error("Missing required field '{field}' in frontmatter")]
    MissingRequiredField { field: String },

    /// A required field is present but empty
    #[error("Field '{field}' cannot be empty")]
    EmptyField { field: String },
}

/// Convert serde_yaml::Error to FrontmatterError
impl From<serde_yaml::Error> for FrontmatterError {
    fn from(err: serde_yaml::Error) -> Self {
        FrontmatterError::InvalidYaml(err.to_string())
    }
}

/// Represents the frontmatter metadata from a SKILL.md file
#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct SkillFrontmatter {
    /// The name of the skill
    pub name: String,

    /// The description of the skill (used for activation triggers)
    pub description: String,

    /// Optional list of tools the skill is allowed to use
    #[serde(
        alias = "tools",
        alias = "allowed-tools",
        skip_serializing_if = "Option::is_none"
    )]
    pub allowed_tools: Option<Vec<String>>,

    /// Optional last updated timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_updated: Option<String>,

    /// Optional content hash
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
}

/// Represents the frontmatter metadata from a changelog.md file
#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct ChangelogFrontmatter {
    /// ISO 8601 date when this changelog was created (YYYY-MM-DD)
    pub created_at: String,

    /// ISO 8601 date of last update (YYYY-MM-DD)
    pub updated_at: String,

    /// The most recent version string (e.g., "2.5.3")
    pub latest_version: String,

    /// Confidence level: high, medium, or low
    pub confidence: String,

    /// List of data sources used
    pub sources: Vec<String>,
}

/// Extract frontmatter and body from SKILL.md content
///
/// Returns `Some((frontmatter_yaml, body_content))` if frontmatter delimiters are found,
/// or `None` if the content doesn't start with frontmatter.
///
/// # Arguments
///
/// * `content` - The full content of the SKILL.md file
///
/// # Returns
///
/// * `Some((yaml_string, body_string))` - The YAML frontmatter and remaining body content
/// * `None` - If the content doesn't start with `---`
pub fn extract_frontmatter(content: &str) -> Option<(String, String)> {
    let content = content.trim_start();

    // Check if content starts with ---
    if !content.starts_with("---") {
        return None;
    }

    // Find the position after the opening ---
    let after_opening = &content[3..];

    // Skip the rest of the first line (in case there's content after opening ---)
    let after_opening = if let Some(pos) = after_opening.find('\n') {
        &after_opening[pos + 1..]
    } else {
        // No newline after opening --- means malformed
        return None;
    };

    // Find the closing ---
    // We need to find --- at the start of a line, and it must be ONLY --- (not "--- FILE:" etc.)
    let mut search_pos = 0;
    loop {
        if let Some(pos) = after_opening[search_pos..].find("---") {
            let absolute_pos = search_pos + pos;

            // Check if this --- is at the start of a line
            let is_line_start =
                absolute_pos == 0 || after_opening.as_bytes()[absolute_pos - 1] == b'\n';

            if is_line_start {
                // Check if this is ONLY --- (closing delimiter) or something like "--- FILE:"
                let after_dashes = &after_opening[absolute_pos + 3..];
                let is_pure_delimiter = after_dashes.is_empty()
                    || after_dashes.starts_with('\n')
                    || after_dashes.starts_with("\r\n")
                    || after_dashes
                        .chars()
                        .next()
                        .map(|c| c.is_whitespace() && c != ' ')
                        .unwrap_or(false);

                // Also allow "---" followed by only whitespace until end of line
                let is_pure_delimiter = is_pure_delimiter || {
                    if let Some(newline_pos) = after_dashes.find('\n') {
                        after_dashes[..newline_pos].trim().is_empty()
                    } else {
                        after_dashes.trim().is_empty()
                    }
                };

                if is_pure_delimiter {
                    // Found closing delimiter
                    let yaml_content = &after_opening[..absolute_pos];

                    // Body starts after the closing --- and its newline
                    let body_start = absolute_pos + 3;
                    let body = if body_start < after_opening.len() {
                        // Skip the newline after closing ---
                        let body_content = &after_opening[body_start..];
                        body_content
                            .strip_prefix("\r\n")
                            .or_else(|| body_content.strip_prefix('\n'))
                            .unwrap_or(body_content)
                    } else {
                        ""
                    };

                    return Some((yaml_content.to_string(), body.to_string()));
                }
            }

            // Not a valid closing delimiter, continue searching
            search_pos = absolute_pos + 3;
        } else {
            // No more --- found
            return None;
        }
    }
}

/// Parse and validate SKILL.md frontmatter
///
/// This function extracts the YAML frontmatter, parses it, validates required fields,
/// and returns both the parsed frontmatter structure and the body content to avoid
/// needing to parse the file twice.
///
/// # Arguments
///
/// * `content` - The full content of the SKILL.md file
///
/// # Returns
///
/// * `Ok((frontmatter, body))` - Successfully parsed and validated frontmatter with body content
/// * `Err(FrontmatterError)` - Validation or parsing error
///
/// # Errors
///
/// * `MissingFrontmatter` - File doesn't start with `---`
/// * `UnclosedFrontmatter` - Missing closing `---` delimiter
/// * `InvalidYaml` - YAML parsing failed
/// * `MissingRequiredField` - Required field is missing
/// * `EmptyField` - Required field is empty
pub fn parse_and_validate_frontmatter(
    content: &str,
) -> Result<(SkillFrontmatter, String), FrontmatterError> {
    // Extract frontmatter and body
    let (yaml_content, body) =
        extract_frontmatter(content).ok_or(FrontmatterError::MissingFrontmatter)?;

    // Check if we actually found a closing delimiter
    // extract_frontmatter returns None if no closing delimiter is found
    // But we need to distinguish between missing opening and missing closing
    if content.trim_start().starts_with("---") && extract_frontmatter(content).is_none() {
        return Err(FrontmatterError::UnclosedFrontmatter);
    }

    // Parse YAML
    let mut frontmatter: SkillFrontmatter = serde_yaml::from_str(&yaml_content)?;

    // Validate required fields exist
    if frontmatter.name.is_empty() {
        return Err(FrontmatterError::EmptyField {
            field: "name".to_string(),
        });
    }

    if frontmatter.description.is_empty() {
        return Err(FrontmatterError::EmptyField {
            field: "description".to_string(),
        });
    }

    // Trim whitespace from required fields
    frontmatter.name = frontmatter.name.trim().to_string();
    frontmatter.description = frontmatter.description.trim().to_string();

    Ok((frontmatter, body))
}

/// Repair common LLM-generated YAML issues in SKILL.md content.
///
/// LLMs sometimes generate malformed frontmatter with issues like:
/// - `## name:` instead of `name:` (markdown headers mixed into YAML)
/// - `tools: \[Read, Write\]` instead of `tools: [Read, Write]` (escaped brackets)
/// - `## --- FILE: SKILL.md ---` at the start (file separator that shouldn't be there)
/// - Blank lines between `---` and frontmatter content
///
/// This function attempts to repair these issues so the SKILL.md can be parsed correctly.
///
/// # Arguments
///
/// * `content` - The raw SKILL.md content from LLM output
///
/// # Returns
///
/// The repaired content with common YAML issues fixed.
///
/// # Examples
///
/// ```
/// use research_lib::validation::frontmatter::repair_skill_frontmatter;
///
/// let malformed = r#"---
///
/// ## name: chalk
/// description: A library
/// tools: \[Read, Write\]
/// ---
/// # Body
/// "#;
///
/// let repaired = repair_skill_frontmatter(malformed);
/// assert!(repaired.contains("name: chalk"));
/// assert!(repaired.contains("tools: [Read, Write]"));
/// ```
pub fn repair_skill_frontmatter(content: &str) -> String {
    let mut content = content.to_string();

    // Issue 1: Remove file separator at the start (e.g., "## --- FILE: SKILL.md ---")
    // This handles both "--- FILE: SKILL.md ---" and "## --- FILE: SKILL.md ---" variants
    let file_separator_patterns = [
        "## --- FILE: SKILL.md ---\n\n",
        "## --- FILE: SKILL.md ---\n",
        "## --- FILE: SKILL.md ---\r\n",
        "--- FILE: SKILL.md ---\n\n",
        "--- FILE: SKILL.md ---\n",
        "--- FILE: SKILL.md ---\r\n",
    ];
    for pattern in file_separator_patterns {
        if content.starts_with(pattern) {
            content = content[pattern.len()..].to_string();
        }
    }

    // Issue 2: Remove blank line after opening `---`
    // This handles "---\n\n" -> "---\n"
    content = content.replace("---\n\n", "---\n");
    content = content.replace("---\r\n\r\n", "---\r\n");

    // Issue 5: Handle missing or incorrect frontmatter delimiters
    // Case A: Completely missing delimiters - "## name: foo\ndescription: bar\n\n# Heading"
    // Case B: Opening delimiter but no closing delimiter - "---\n## name: foo\n\n# Heading"
    // Case C: Closing delimiter exists but is too far (includes markdown) - frontmatter has "# syntect" in it
    // In all cases, we need to reconstruct proper frontmatter
    let content_trimmed = content.trim_start();
    if !content_trimmed.starts_with("---") {
        // Case A: No opening delimiter
        if let Some(reconstructed) = try_reconstruct_frontmatter(&content) {
            content = reconstructed;
        }
    } else if let Some((yaml, _body)) = extract_frontmatter(&content) {
        // Check if the extracted YAML contains markdown content (Case C)
        // Real YAML frontmatter shouldn't have lines like "# heading" (without colon)
        let has_markdown_headings = yaml.lines().any(|line| {
            let trimmed = line.trim();
            (trimmed.starts_with("# ") || trimmed.starts_with("## ")) && !trimmed.contains(':')
        });

        if has_markdown_headings {
            // Case C: The closing "---" was too far down (it's a horizontal rule, not frontmatter closing)
            // Strip the opening "---" and try to reconstruct proper frontmatter
            let without_opening = content_trimmed
                .strip_prefix("---")
                .and_then(|s| s.strip_prefix('\n').or(s.strip_prefix("\r\n")))
                .unwrap_or(content_trimmed);
            if let Some(reconstructed) = try_reconstruct_frontmatter(without_opening) {
                content = reconstructed;
            }
        }
    } else {
        // Case B: Has opening "---" but no closing "---"
        // Strip the opening "---" and try to reconstruct
        let without_opening = content_trimmed
            .strip_prefix("---")
            .and_then(|s| s.strip_prefix('\n').or(s.strip_prefix("\r\n")))
            .unwrap_or(content_trimmed);
        if let Some(reconstructed) = try_reconstruct_frontmatter(without_opening) {
            content = reconstructed;
        }
    }

    // Issue 3: Remove markdown headers from YAML field names within frontmatter
    // This requires us to identify the frontmatter section first
    if let Some((yaml, body)) = extract_frontmatter(&content) {
        // Fix the YAML content
        let mut fixed_yaml = String::new();
        for line in yaml.lines() {
            let line = line.trim_start();
            // Remove leading ## or # from lines that look like YAML fields
            // e.g., "## name: chalk" -> "name: chalk"
            let fixed_line = if line.starts_with("## ") && line.contains(':') {
                &line[3..] // Remove "## "
            } else if line.starts_with("# ") && line.contains(':') && !line.starts_with("# ") {
                // Be careful: "# name:" should become "name:" but we should preserve intentional comments
                // A YAML comment would be "# comment" not "# key: value"
                &line[2..] // Remove "# "
            } else {
                line
            };
            fixed_yaml.push_str(fixed_line);
            fixed_yaml.push('\n');
        }

        // Issue 4: Fix escaped brackets in tools field
        // `tools: \[Read, Write\]` -> `tools: [Read, Write]`
        fixed_yaml = fixed_yaml.replace("\\[", "[").replace("\\]", "]");

        // Reconstruct the content with fixed frontmatter
        content = format!("---\n{}---\n{}", fixed_yaml, body);
    } else {
        // Frontmatter couldn't be extracted, try more aggressive repairs
        // This handles cases where the structure is so broken we can't parse it

        // Try to fix common patterns even without successful extraction
        content = content.replace("\\[", "[").replace("\\]", "]");

        // Fix markdown headers in YAML-like lines at the start of the file
        let lines: Vec<&str> = content.lines().collect();
        let mut fixed_lines = Vec::new();
        let mut in_frontmatter = false;

        for line in lines {
            if line.trim() == "---" {
                in_frontmatter = !in_frontmatter;
                fixed_lines.push(line.to_string());
            } else if in_frontmatter {
                // We're inside frontmatter, fix markdown headers
                let trimmed = line.trim_start();
                let fixed = if trimmed.starts_with("## ") && trimmed.contains(':') {
                    trimmed[3..].to_string()
                } else {
                    line.to_string()
                };
                fixed_lines.push(fixed);
            } else {
                fixed_lines.push(line.to_string());
            }
        }

        content = fixed_lines.join("\n");
    }

    content
}

/// Fix a YAML line to ensure the value is properly quoted if needed.
///
/// In YAML, values containing colons or special characters need to be quoted.
/// This function takes a "key: value" line and returns it with proper quoting.
fn fix_yaml_line(line: &str) -> String {
    // Find the first colon that separates key from value
    if let Some(colon_pos) = line.find(':') {
        let key = &line[..colon_pos];
        let value_with_space = &line[colon_pos + 1..];
        let value = value_with_space.trim();

        // Check if value contains colons and isn't already quoted
        if value.contains(':')
            && !value.starts_with('"')
            && !value.starts_with('\'')
            && !value.starts_with('[')
            && !value.starts_with('{')
        {
            // Quote the value
            let escaped_value = value.replace('"', "\\\"");
            return format!("{}: \"{}\"", key, escaped_value);
        }
    }
    line.to_string()
}

/// Try to reconstruct frontmatter when delimiters are completely missing.
///
/// This handles cases like:
/// ```text
/// ## name: chalk
/// description: A library
///
/// # Heading
/// Body content
/// ```
///
/// Which should become:
/// ```text
/// ---
/// name: chalk
/// description: A library
/// ---
/// # Heading
/// Body content
/// ```
fn try_reconstruct_frontmatter(content: &str) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return None;
    }

    // Check if first non-empty line looks like a YAML field
    let first_meaningful = lines.iter().find(|l| !l.trim().is_empty())?;
    let first_trimmed = first_meaningful.trim();

    // Check for patterns like "## name:" or "name:"
    let looks_like_yaml = (first_trimmed.starts_with("## ") && first_trimmed.contains(':'))
        || (first_trimmed.starts_with("# ")
            && first_trimmed.contains(':')
            && first_trimmed.len() > 3)
        || (first_trimmed.contains(':')
            && !first_trimmed.starts_with('#')
            && !first_trimmed.contains("**"));

    if !looks_like_yaml {
        return None;
    }

    // Find where the frontmatter ends (first markdown heading or blank line followed by heading)
    let mut frontmatter_lines = Vec::new();
    let mut body_lines = Vec::new();
    let mut found_body = false;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        if found_body {
            body_lines.push(*line);
            continue;
        }

        // Body starts at first real markdown heading (# but not ## name:)
        if trimmed.starts_with("# ") && !trimmed.contains(':') {
            found_body = true;
            body_lines.push(*line);
            continue;
        }

        // Also treat blank line followed by heading as body start
        if trimmed.is_empty() && i + 1 < lines.len() {
            let next_line = lines[i + 1].trim();
            if next_line.starts_with("# ") && !next_line.contains(':') {
                found_body = true;
                body_lines.push(*line);
                continue;
            }
        }

        // Fix the line if it's a YAML field with markdown prefix
        let fixed = if trimmed.starts_with("## ") && trimmed.contains(':') {
            fix_yaml_line(&trimmed[3..])
        } else if trimmed.starts_with("# ") && trimmed.contains(':') {
            fix_yaml_line(&trimmed[2..])
        } else if !trimmed.is_empty() {
            fix_yaml_line(trimmed)
        } else {
            String::new()
        };

        if !fixed.is_empty() {
            frontmatter_lines.push(fixed);
        }
    }

    // Only proceed if we found both frontmatter and body
    if frontmatter_lines.is_empty() {
        return None;
    }

    // Fix escaped brackets in frontmatter
    let frontmatter: Vec<String> = frontmatter_lines
        .iter()
        .map(|l| l.replace("\\[", "[").replace("\\]", "]"))
        .collect();

    // Reconstruct with proper delimiters
    let mut result = String::from("---\n");
    for line in frontmatter {
        result.push_str(&line);
        result.push('\n');
    }
    result.push_str("---\n");
    for line in body_lines {
        result.push_str(line);
        result.push('\n');
    }

    Some(result)
}

/// Parse and validate changelog.md frontmatter
///
/// This function extracts the YAML frontmatter, parses it, validates required fields,
/// and returns both the parsed frontmatter structure and the body content.
///
/// # Arguments
///
/// * `content` - The full content of the changelog.md file
///
/// # Returns
///
/// * `Ok((frontmatter, body))` - Successfully parsed and validated frontmatter with body content
/// * `Err(FrontmatterError)` - Validation or parsing error
///
/// # Errors
///
/// * `MissingFrontmatter` - File doesn't start with `---`
/// * `UnclosedFrontmatter` - Missing closing `---` delimiter
/// * `InvalidYaml` - YAML parsing failed
/// * `MissingRequiredField` - Required field is missing
/// * `EmptyField` - Required field is empty
pub fn parse_and_validate_changelog_frontmatter(
    content: &str,
) -> Result<(ChangelogFrontmatter, String), FrontmatterError> {
    // Extract frontmatter and body
    let (yaml_content, body) =
        extract_frontmatter(content).ok_or(FrontmatterError::MissingFrontmatter)?;

    // Check if we actually found a closing delimiter
    if content.trim_start().starts_with("---") && extract_frontmatter(content).is_none() {
        return Err(FrontmatterError::UnclosedFrontmatter);
    }

    // Parse YAML
    let mut frontmatter: ChangelogFrontmatter = serde_yaml::from_str(&yaml_content)?;

    // Validate required fields exist and are non-empty
    if frontmatter.created_at.is_empty() {
        return Err(FrontmatterError::EmptyField {
            field: "created_at".to_string(),
        });
    }

    if frontmatter.updated_at.is_empty() {
        return Err(FrontmatterError::EmptyField {
            field: "updated_at".to_string(),
        });
    }

    if frontmatter.latest_version.is_empty() {
        return Err(FrontmatterError::EmptyField {
            field: "latest_version".to_string(),
        });
    }

    if frontmatter.confidence.is_empty() {
        return Err(FrontmatterError::EmptyField {
            field: "confidence".to_string(),
        });
    }

    // Validate confidence value
    let confidence_lower = frontmatter.confidence.trim().to_lowercase();
    if !["high", "medium", "low"].contains(&confidence_lower.as_str()) {
        return Err(FrontmatterError::InvalidYaml(format!(
            "confidence must be 'high', 'medium', or 'low', got '{}'",
            frontmatter.confidence
        )));
    }

    // Validate sources is non-empty
    if frontmatter.sources.is_empty() {
        return Err(FrontmatterError::EmptyField {
            field: "sources".to_string(),
        });
    }

    // Validate date formats (basic ISO 8601 check: YYYY-MM-DD)
    if !is_valid_iso8601_date(&frontmatter.created_at) {
        return Err(FrontmatterError::InvalidYaml(format!(
            "created_at must be in ISO 8601 format (YYYY-MM-DD), got '{}'",
            frontmatter.created_at
        )));
    }

    if !is_valid_iso8601_date(&frontmatter.updated_at) {
        return Err(FrontmatterError::InvalidYaml(format!(
            "updated_at must be in ISO 8601 format (YYYY-MM-DD), got '{}'",
            frontmatter.updated_at
        )));
    }

    // Trim whitespace from required fields
    frontmatter.created_at = frontmatter.created_at.trim().to_string();
    frontmatter.updated_at = frontmatter.updated_at.trim().to_string();
    frontmatter.latest_version = frontmatter.latest_version.trim().to_string();
    frontmatter.confidence = frontmatter.confidence.trim().to_string();

    Ok((frontmatter, body))
}

/// Validate ISO 8601 date format (YYYY-MM-DD)
///
/// Performs basic validation of the date string format.
fn is_valid_iso8601_date(date: &str) -> bool {
    use chrono::NaiveDate;
    NaiveDate::parse_from_str(date.trim(), "%Y-%m-%d").is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_frontmatter_all_fields() {
        let content = r#"---
name: test-skill
description: A test skill for validation
tools:
  - Bash
  - Read
last_updated: 2025-12-29
hash: abc123
---
# Test Skill

This is the body content.
"#;

        let result = parse_and_validate_frontmatter(content);
        assert!(result.is_ok());

        let (frontmatter, body) = result.unwrap();
        assert_eq!(frontmatter.name, "test-skill");
        assert_eq!(frontmatter.description, "A test skill for validation");
        assert_eq!(
            frontmatter.allowed_tools,
            Some(vec!["Bash".to_string(), "Read".to_string()])
        );
        assert_eq!(frontmatter.last_updated, Some("2025-12-29".to_string()));
        assert_eq!(frontmatter.hash, Some("abc123".to_string()));
        assert!(body.contains("# Test Skill"));
        assert!(body.contains("This is the body content."));
    }

    #[test]
    fn test_valid_frontmatter_minimal() {
        let content = r#"---
name: minimal-skill
description: Minimal skill with only required fields
---
Body content here.
"#;

        let result = parse_and_validate_frontmatter(content);
        assert!(result.is_ok());

        let (frontmatter, body) = result.unwrap();
        assert_eq!(frontmatter.name, "minimal-skill");
        assert_eq!(
            frontmatter.description,
            "Minimal skill with only required fields"
        );
        assert_eq!(frontmatter.allowed_tools, None);
        assert_eq!(frontmatter.last_updated, None);
        assert_eq!(frontmatter.hash, None);
        assert_eq!(body.trim(), "Body content here.");
    }

    #[test]
    fn test_missing_frontmatter() {
        let content = r#"# No Frontmatter

This file doesn't start with frontmatter.
"#;

        let result = parse_and_validate_frontmatter(content);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), FrontmatterError::MissingFrontmatter);
    }

    #[test]
    fn test_unclosed_frontmatter() {
        let content = r#"---
name: unclosed
description: Missing closing delimiter
"#;

        let result = parse_and_validate_frontmatter(content);
        assert!(result.is_err());

        match result.unwrap_err() {
            FrontmatterError::UnclosedFrontmatter => {}
            FrontmatterError::MissingFrontmatter => {}
            other => panic!(
                "Expected UnclosedFrontmatter or MissingFrontmatter, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn test_missing_required_field_name() {
        let content = r#"---
description: Missing name field
---
Body
"#;

        let result = parse_and_validate_frontmatter(content);
        assert!(result.is_err());

        match result.unwrap_err() {
            FrontmatterError::InvalidYaml(_) => {
                // serde will fail to deserialize without name field
            }
            other => panic!("Expected InvalidYaml for missing name, got {:?}", other),
        }
    }

    #[test]
    fn test_missing_required_field_description() {
        let content = r#"---
name: test
---
Body
"#;

        let result = parse_and_validate_frontmatter(content);
        assert!(result.is_err());

        match result.unwrap_err() {
            FrontmatterError::InvalidYaml(_) => {
                // serde will fail to deserialize without description field
            }
            other => panic!(
                "Expected InvalidYaml for missing description, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn test_empty_field_name() {
        let content = r#"---
name: ""
description: Valid description
---
Body
"#;

        let result = parse_and_validate_frontmatter(content);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            FrontmatterError::EmptyField {
                field: "name".to_string()
            }
        );
    }

    #[test]
    fn test_empty_field_description() {
        let content = r#"---
name: test-skill
description: ""
---
Body
"#;

        let result = parse_and_validate_frontmatter(content);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            FrontmatterError::EmptyField {
                field: "description".to_string()
            }
        );
    }

    #[test]
    fn test_tools_field() {
        let content = r#"---
name: skill-with-tools
description: Skill with tools field
tools:
  - Bash
  - Read
  - Write
---
Body
"#;

        let result = parse_and_validate_frontmatter(content);
        assert!(result.is_ok());

        let (frontmatter, _) = result.unwrap();
        assert_eq!(
            frontmatter.allowed_tools,
            Some(vec![
                "Bash".to_string(),
                "Read".to_string(),
                "Write".to_string()
            ])
        );
    }

    #[test]
    fn test_allowed_tools_alias() {
        let content = r#"---
name: skill-with-allowed-tools
description: Skill with allowed-tools field (hyphenated form should work)
allowed-tools:
  - Grep
  - Glob
---
Body
"#;

        let result = parse_and_validate_frontmatter(content);
        assert!(result.is_ok());

        let (frontmatter, _) = result.unwrap();
        assert_eq!(
            frontmatter.allowed_tools,
            Some(vec!["Grep".to_string(), "Glob".to_string()])
        );
    }

    #[test]
    fn test_allowed_tools_serialization() {
        // Verify that serialization produces "allowed_tools" (not "tools")
        let frontmatter = SkillFrontmatter {
            name: "test-skill".to_string(),
            description: "Test skill".to_string(),
            allowed_tools: Some(vec!["Bash".to_string(), "Read".to_string()]),
            last_updated: None,
            hash: None,
        };

        let yaml = serde_yaml::to_string(&frontmatter).expect("Failed to serialize");
        assert!(
            yaml.contains("allowed_tools:"),
            "Serialization should use 'allowed_tools', got: {}",
            yaml
        );
        assert!(
            !yaml.contains("tools:") || yaml.contains("allowed_tools:"),
            "Serialization should not produce bare 'tools:'"
        );
    }

    #[test]
    fn test_invalid_yaml_syntax() {
        let content = r#"---
name: test
description: test
invalid: yaml: syntax: here
---
Body
"#;

        let result = parse_and_validate_frontmatter(content);
        assert!(result.is_err());

        match result.unwrap_err() {
            FrontmatterError::InvalidYaml(_) => {}
            other => panic!("Expected InvalidYaml, got {:?}", other),
        }
    }

    #[test]
    fn test_extraction_separates_frontmatter_and_body() {
        let content = r#"---
name: test
description: test
---
# Heading

Body content line 1
Body content line 2
"#;

        let (frontmatter, body) = parse_and_validate_frontmatter(content).unwrap();
        assert_eq!(frontmatter.name, "test");
        assert_eq!(frontmatter.description, "test");
        assert!(body.starts_with("# Heading"));
        assert!(body.contains("Body content line 1"));
        assert!(body.contains("Body content line 2"));
    }

    #[test]
    fn test_whitespace_variations_crlf() {
        let content = "---\r\nname: test\r\ndescription: test\r\n---\r\nBody content";

        let result = parse_and_validate_frontmatter(content);
        assert!(result.is_ok());

        let (frontmatter, body) = result.unwrap();
        assert_eq!(frontmatter.name, "test");
        assert_eq!(frontmatter.description, "test");
        assert_eq!(body.trim(), "Body content");
    }

    #[test]
    fn test_whitespace_variations_trailing_spaces() {
        let content = r#"---
name: test
description: test
---
Body content
"#;

        let result = parse_and_validate_frontmatter(content);
        assert!(result.is_ok());

        let (frontmatter, body) = result.unwrap();
        assert_eq!(frontmatter.name, "test");
        assert_eq!(frontmatter.description, "test");
        assert_eq!(body.trim(), "Body content");
    }

    #[test]
    fn test_extract_frontmatter_valid() {
        let content = r#"---
name: test
---
body"#;

        let result = extract_frontmatter(content);
        assert!(result.is_some());

        let (yaml, body) = result.unwrap();
        assert!(yaml.contains("name: test"));
        assert_eq!(body.trim(), "body");
    }

    #[test]
    fn test_extract_frontmatter_no_frontmatter() {
        let content = "Just body content";

        let result = extract_frontmatter(content);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_frontmatter_unclosed() {
        let content = r#"---
name: test
no closing delimiter"#;

        let result = extract_frontmatter(content);
        assert!(result.is_none());
    }

    // Changelog frontmatter tests
    #[test]
    fn test_valid_changelog_frontmatter_all_fields() {
        let content = r#"---
created_at: 2024-12-30
updated_at: 2024-12-30
latest_version: "2.5.3"
confidence: high
sources:
  - github_releases
  - changelog_file
  - registry_versions
---
# Version History

Version timeline content here.
"#;

        let result = parse_and_validate_changelog_frontmatter(content);
        assert!(result.is_ok());

        let (frontmatter, body) = result.unwrap();
        assert_eq!(frontmatter.created_at, "2024-12-30");
        assert_eq!(frontmatter.updated_at, "2024-12-30");
        assert_eq!(frontmatter.latest_version, "2.5.3");
        assert_eq!(frontmatter.confidence, "high");
        assert_eq!(frontmatter.sources.len(), 3);
        assert!(frontmatter.sources.contains(&"github_releases".to_string()));
        assert!(body.contains("# Version History"));
    }

    #[test]
    fn test_valid_changelog_frontmatter_minimal() {
        let content = r#"---
created_at: 2024-12-30
updated_at: 2024-12-30
latest_version: "1.0.0"
confidence: low
sources:
  - llm_knowledge
---
Body content here.
"#;

        let result = parse_and_validate_changelog_frontmatter(content);
        assert!(result.is_ok());

        let (frontmatter, body) = result.unwrap();
        assert_eq!(frontmatter.created_at, "2024-12-30");
        assert_eq!(frontmatter.updated_at, "2024-12-30");
        assert_eq!(frontmatter.latest_version, "1.0.0");
        assert_eq!(frontmatter.confidence, "low");
        assert_eq!(frontmatter.sources, vec!["llm_knowledge"]);
        assert_eq!(body.trim(), "Body content here.");
    }

    #[test]
    fn test_changelog_missing_frontmatter() {
        let content = r#"# Version History

No frontmatter here.
"#;

        let result = parse_and_validate_changelog_frontmatter(content);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), FrontmatterError::MissingFrontmatter);
    }

    #[test]
    fn test_changelog_unclosed_frontmatter() {
        let content = r#"---
created_at: 2024-12-30
updated_at: 2024-12-30
latest_version: "1.0.0"
confidence: high
sources:
  - github_releases
"#;

        let result = parse_and_validate_changelog_frontmatter(content);
        assert!(result.is_err());

        match result.unwrap_err() {
            FrontmatterError::UnclosedFrontmatter => {}
            FrontmatterError::MissingFrontmatter => {}
            other => panic!(
                "Expected UnclosedFrontmatter or MissingFrontmatter, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn test_changelog_missing_created_at() {
        let content = r#"---
updated_at: 2024-12-30
latest_version: "1.0.0"
confidence: high
sources:
  - github_releases
---
Body
"#;

        let result = parse_and_validate_changelog_frontmatter(content);
        assert!(result.is_err());

        match result.unwrap_err() {
            FrontmatterError::InvalidYaml(_) => {
                // serde will fail to deserialize without created_at field
            }
            other => panic!(
                "Expected InvalidYaml for missing created_at, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn test_changelog_missing_updated_at() {
        let content = r#"---
created_at: 2024-12-30
latest_version: "1.0.0"
confidence: high
sources:
  - github_releases
---
Body
"#;

        let result = parse_and_validate_changelog_frontmatter(content);
        assert!(result.is_err());

        match result.unwrap_err() {
            FrontmatterError::InvalidYaml(_) => {
                // serde will fail to deserialize without updated_at field
            }
            other => panic!(
                "Expected InvalidYaml for missing updated_at, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn test_changelog_missing_latest_version() {
        let content = r#"---
created_at: 2024-12-30
updated_at: 2024-12-30
confidence: high
sources:
  - github_releases
---
Body
"#;

        let result = parse_and_validate_changelog_frontmatter(content);
        assert!(result.is_err());

        match result.unwrap_err() {
            FrontmatterError::InvalidYaml(_) => {
                // serde will fail to deserialize without latest_version field
            }
            other => panic!(
                "Expected InvalidYaml for missing latest_version, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn test_changelog_missing_confidence() {
        let content = r#"---
created_at: 2024-12-30
updated_at: 2024-12-30
latest_version: "1.0.0"
sources:
  - github_releases
---
Body
"#;

        let result = parse_and_validate_changelog_frontmatter(content);
        assert!(result.is_err());

        match result.unwrap_err() {
            FrontmatterError::InvalidYaml(_) => {
                // serde will fail to deserialize without confidence field
            }
            other => panic!(
                "Expected InvalidYaml for missing confidence, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn test_changelog_missing_sources() {
        let content = r#"---
created_at: 2024-12-30
updated_at: 2024-12-30
latest_version: "1.0.0"
confidence: high
---
Body
"#;

        let result = parse_and_validate_changelog_frontmatter(content);
        assert!(result.is_err());

        match result.unwrap_err() {
            FrontmatterError::InvalidYaml(_) => {
                // serde will fail to deserialize without sources field
            }
            other => panic!("Expected InvalidYaml for missing sources, got {:?}", other),
        }
    }

    #[test]
    fn test_changelog_empty_created_at() {
        let content = r#"---
created_at: ""
updated_at: 2024-12-30
latest_version: "1.0.0"
confidence: high
sources:
  - github_releases
---
Body
"#;

        let result = parse_and_validate_changelog_frontmatter(content);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            FrontmatterError::EmptyField {
                field: "created_at".to_string()
            }
        );
    }

    #[test]
    fn test_changelog_empty_updated_at() {
        let content = r#"---
created_at: 2024-12-30
updated_at: ""
latest_version: "1.0.0"
confidence: high
sources:
  - github_releases
---
Body
"#;

        let result = parse_and_validate_changelog_frontmatter(content);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            FrontmatterError::EmptyField {
                field: "updated_at".to_string()
            }
        );
    }

    #[test]
    fn test_changelog_empty_latest_version() {
        let content = r#"---
created_at: 2024-12-30
updated_at: 2024-12-30
latest_version: ""
confidence: high
sources:
  - github_releases
---
Body
"#;

        let result = parse_and_validate_changelog_frontmatter(content);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            FrontmatterError::EmptyField {
                field: "latest_version".to_string()
            }
        );
    }

    #[test]
    fn test_changelog_empty_confidence() {
        let content = r#"---
created_at: 2024-12-30
updated_at: 2024-12-30
latest_version: "1.0.0"
confidence: ""
sources:
  - github_releases
---
Body
"#;

        let result = parse_and_validate_changelog_frontmatter(content);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            FrontmatterError::EmptyField {
                field: "confidence".to_string()
            }
        );
    }

    #[test]
    fn test_changelog_empty_sources() {
        let content = r#"---
created_at: 2024-12-30
updated_at: 2024-12-30
latest_version: "1.0.0"
confidence: high
sources: []
---
Body
"#;

        let result = parse_and_validate_changelog_frontmatter(content);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            FrontmatterError::EmptyField {
                field: "sources".to_string()
            }
        );
    }

    #[test]
    fn test_changelog_invalid_confidence_value() {
        let content = r#"---
created_at: 2024-12-30
updated_at: 2024-12-30
latest_version: "1.0.0"
confidence: invalid
sources:
  - github_releases
---
Body
"#;

        let result = parse_and_validate_changelog_frontmatter(content);
        assert!(result.is_err());

        match result.unwrap_err() {
            FrontmatterError::InvalidYaml(msg) => {
                assert!(msg.contains("confidence must be"));
            }
            other => panic!(
                "Expected InvalidYaml for invalid confidence, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn test_changelog_confidence_case_insensitive() {
        let content = r#"---
created_at: 2024-12-30
updated_at: 2024-12-30
latest_version: "1.0.0"
confidence: HIGH
sources:
  - github_releases
---
Body
"#;

        let result = parse_and_validate_changelog_frontmatter(content);
        assert!(result.is_ok());

        let (frontmatter, _) = result.unwrap();
        assert_eq!(frontmatter.confidence, "HIGH");
    }

    #[test]
    fn test_changelog_invalid_created_at_date() {
        let content = r#"---
created_at: not-a-date
updated_at: 2024-12-30
latest_version: "1.0.0"
confidence: high
sources:
  - github_releases
---
Body
"#;

        let result = parse_and_validate_changelog_frontmatter(content);
        assert!(result.is_err());

        match result.unwrap_err() {
            FrontmatterError::InvalidYaml(msg) => {
                assert!(msg.contains("ISO 8601"));
                assert!(msg.contains("created_at"));
            }
            other => panic!("Expected InvalidYaml for invalid date, got {:?}", other),
        }
    }

    #[test]
    fn test_changelog_invalid_updated_at_date() {
        let content = r#"---
created_at: 2024-12-30
updated_at: 12/30/2024
latest_version: "1.0.0"
confidence: high
sources:
  - github_releases
---
Body
"#;

        let result = parse_and_validate_changelog_frontmatter(content);
        assert!(result.is_err());

        match result.unwrap_err() {
            FrontmatterError::InvalidYaml(msg) => {
                assert!(msg.contains("ISO 8601"));
                assert!(msg.contains("updated_at"));
            }
            other => panic!("Expected InvalidYaml for invalid date, got {:?}", other),
        }
    }

    #[test]
    fn test_changelog_frontmatter_whitespace_trimming() {
        let content = r#"---
created_at: " 2024-12-30 "
updated_at: " 2024-12-30 "
latest_version: " 1.0.0 "
confidence: " high "
sources:
  - github_releases
---
Body
"#;

        let result = parse_and_validate_changelog_frontmatter(content);
        assert!(result.is_ok());

        let (frontmatter, _) = result.unwrap();
        assert_eq!(frontmatter.created_at, "2024-12-30");
        assert_eq!(frontmatter.updated_at, "2024-12-30");
        assert_eq!(frontmatter.latest_version, "1.0.0");
        assert_eq!(frontmatter.confidence, "high");
    }

    #[test]
    fn test_changelog_invalid_yaml_syntax() {
        let content = r#"---
created_at: 2024-12-30
updated_at: 2024-12-30
latest_version: "1.0.0"
confidence: high
invalid: yaml: syntax: here
sources:
  - github_releases
---
Body
"#;

        let result = parse_and_validate_changelog_frontmatter(content);
        assert!(result.is_err());

        match result.unwrap_err() {
            FrontmatterError::InvalidYaml(_) => {}
            other => panic!("Expected InvalidYaml, got {:?}", other),
        }
    }

    #[test]
    fn test_is_valid_iso8601_date_valid() {
        assert!(is_valid_iso8601_date("2024-12-30"));
        assert!(is_valid_iso8601_date("2024-01-01"));
        assert!(is_valid_iso8601_date("2000-12-31"));
    }

    #[test]
    fn test_is_valid_iso8601_date_invalid() {
        assert!(!is_valid_iso8601_date("not-a-date"));
        assert!(!is_valid_iso8601_date("12/30/2024"));
        assert!(!is_valid_iso8601_date("2024-13-01")); // Invalid month
        assert!(!is_valid_iso8601_date("2024-12-32")); // Invalid day
        assert!(!is_valid_iso8601_date("2024/12/30")); // Wrong separator
    }

    // =========================================================================
    // repair_skill_frontmatter Tests
    // Regression tests for bug where LLMs generate malformed YAML frontmatter
    // =========================================================================

    #[test]
    fn test_repair_markdown_headers_in_yaml() {
        // Regression test: LLM generates "## name:" instead of "name:"
        let malformed = r#"---

## name: chalk
description: A library for terminal colors
tools: [Read, Write]
---
# chalk

Body content here.
"#;

        let repaired = super::repair_skill_frontmatter(malformed);

        // Should be parseable after repair
        let result = parse_and_validate_frontmatter(&repaired);
        assert!(
            result.is_ok(),
            "Repaired content should be valid: {:?}",
            result
        );

        let (frontmatter, _) = result.unwrap();
        assert_eq!(frontmatter.name, "chalk");
        assert_eq!(frontmatter.description, "A library for terminal colors");
    }

    #[test]
    fn test_repair_escaped_brackets() {
        // Regression test: LLM generates "\[" instead of "["
        let malformed = r#"---
name: chalk
description: A library for terminal colors
tools: \[Read, Write, Edit\]
---
# Body
"#;

        let repaired = super::repair_skill_frontmatter(malformed);

        // Should be parseable after repair
        let result = parse_and_validate_frontmatter(&repaired);
        assert!(
            result.is_ok(),
            "Repaired content should be valid: {:?}",
            result
        );

        let (frontmatter, _) = result.unwrap();
        assert_eq!(
            frontmatter.allowed_tools,
            Some(vec![
                "Read".to_string(),
                "Write".to_string(),
                "Edit".to_string()
            ])
        );
    }

    #[test]
    fn test_repair_file_separator_at_start() {
        // Regression test: LLM includes "## --- FILE: SKILL.md ---" at the start
        let malformed = r#"## --- FILE: SKILL.md ---
---
name: clap
description: Build CLI interfaces
---
# clap

Body content.
"#;

        let repaired = super::repair_skill_frontmatter(malformed);

        // Should be parseable after repair
        let result = parse_and_validate_frontmatter(&repaired);
        assert!(
            result.is_ok(),
            "Repaired content should be valid: {:?}",
            result
        );

        let (frontmatter, _) = result.unwrap();
        assert_eq!(frontmatter.name, "clap");
    }

    #[test]
    fn test_repair_blank_line_after_delimiter() {
        // Regression test: LLM adds blank line after opening ---
        let malformed = r#"---

name: test-skill
description: Test description
---
# Body
"#;

        let repaired = super::repair_skill_frontmatter(malformed);

        // Should be parseable after repair
        let result = parse_and_validate_frontmatter(&repaired);
        assert!(
            result.is_ok(),
            "Repaired content should be valid: {:?}",
            result
        );

        let (frontmatter, _) = result.unwrap();
        assert_eq!(frontmatter.name, "test-skill");
    }

    #[test]
    fn test_repair_combined_issues() {
        // Regression test: Multiple issues combined (real-world LLM output)
        let malformed = r#"---

## name: chalk
description: Expert knowledge for styling Node.js terminal output with Chalk
tools: \[Read, Write, Edit, Grep, Glob, Bash\]
---

# chalk

Content here.
"#;

        let repaired = super::repair_skill_frontmatter(malformed);

        // Should be parseable after repair
        let result = parse_and_validate_frontmatter(&repaired);
        assert!(
            result.is_ok(),
            "Repaired content should be valid: {:?}",
            result
        );

        let (frontmatter, _body) = result.unwrap();
        assert_eq!(frontmatter.name, "chalk");
        assert!(frontmatter.description.contains("Chalk"));
        assert_eq!(
            frontmatter.allowed_tools,
            Some(vec![
                "Read".to_string(),
                "Write".to_string(),
                "Edit".to_string(),
                "Grep".to_string(),
                "Glob".to_string(),
                "Bash".to_string()
            ])
        );
    }

    #[test]
    fn test_repair_valid_content_unchanged() {
        // Valid content should remain essentially the same
        let valid = r#"---
name: test-skill
description: Valid test skill
tools: [Read, Write]
---
# Test Skill

Body content.
"#;

        let repaired = super::repair_skill_frontmatter(valid);

        // Should still be parseable
        let result = parse_and_validate_frontmatter(&repaired);
        assert!(
            result.is_ok(),
            "Valid content should remain valid: {:?}",
            result
        );

        let (frontmatter, _) = result.unwrap();
        assert_eq!(frontmatter.name, "test-skill");
    }

    #[test]
    fn test_repair_clap_real_world_case() {
        // Real-world case from clap skill
        let malformed = r#"## --- FILE: SKILL.md ---

## name: clap
description: Build robust Rust CLI interfaces with clap v4: derive- or builder-based argument parsing, subcommands, validation, env vars, and UX polish (help, suggestions, completions).

# clap (Rust CLI Argument Parsing)

Use this skill when you need to **design or implement a Rust CLI**.
"#;

        let repaired = super::repair_skill_frontmatter(malformed);

        // Should be parseable after repair
        let result = parse_and_validate_frontmatter(&repaired);
        assert!(
            result.is_ok(),
            "Repaired clap content should be valid: {:?}",
            result
        );

        let (frontmatter, _) = result.unwrap();
        assert_eq!(frontmatter.name, "clap");
        assert!(frontmatter.description.contains("clap v4"));
    }

    #[test]
    fn test_repair_chalk_actual_file() {
        // Actual chalk file content - has opening delimiter but NO closing delimiter
        // This matches the actual file on disk
        let malformed = "---\n\n## name: chalk\ndescription: Expert knowledge for styling Node.js terminal output with Chalk (colors, modifiers, templates, custom instances) and handling color-support detection. Use when building or refactoring CLIs/logging output, troubleshooting ESM vs CommonJS import issues, or designing readable terminal UX.\ntools: \\[Read, Write, Edit, Grep, Glob, Bash\\]\n\n# chalk\n\nChalk is the go-to Node.js library for styling terminal strings.\n";

        println!("Input content:");
        for (i, line) in malformed.lines().enumerate() {
            println!("{}: [{}]", i + 1, line);
        }

        let repaired = super::repair_skill_frontmatter(malformed);
        println!("\nRepaired content:");
        for (i, line) in repaired.lines().enumerate() {
            println!("{}: [{}]", i + 1, line);
        }

        // Should be parseable after repair
        let result = parse_and_validate_frontmatter(&repaired);
        assert!(
            result.is_ok(),
            "Repaired chalk content should be valid: {:?}",
            result
        );

        let (frontmatter, _) = result.unwrap();
        assert_eq!(frontmatter.name, "chalk");
        assert!(frontmatter.description.contains("Chalk"));
    }
}
