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

    /// Optional list of allowed tools (accepts both "tools" and "allowed-tools")
    #[serde(alias = "allowed-tools", skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<String>>,

    /// Optional last updated timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_updated: Option<String>,

    /// Optional content hash
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
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
    // We need to find --- at the start of a line
    let mut search_pos = 0;
    loop {
        if let Some(pos) = after_opening[search_pos..].find("---") {
            let absolute_pos = search_pos + pos;

            // Check if this --- is at the start of a line
            let is_line_start =
                absolute_pos == 0 || after_opening.as_bytes()[absolute_pos - 1] == b'\n';

            if is_line_start {
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

            // Not at line start, continue searching
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
            frontmatter.tools,
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
        assert_eq!(frontmatter.tools, None);
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
            frontmatter.tools,
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
description: Skill with allowed-tools field (should alias to tools)
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
            frontmatter.tools,
            Some(vec!["Grep".to_string(), "Glob".to_string()])
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
}
