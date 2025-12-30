//! Integration tests for skill validation and file splitting.
//!
//! This test suite provides comprehensive coverage for:
//! - Frontmatter parsing and validation
//! - File splitting logic
//! - ResearchMetadata serialization with when_to_use field
//! - Regression tests for the original file separator bug
//! - Whitespace variation handling
//! - Edge cases and error scenarios

use research_lib::validation::frontmatter::{
    FrontmatterError, SkillFrontmatter, parse_and_validate_frontmatter,
};
use research_lib::{LibraryInfo, ResearchMetadata};

/// Helper function to access the split_into_files function from lib.rs
/// Since it's private, we'll test it through the public API or duplicate the logic here.
/// For now, we'll duplicate the logic for testing purposes.
fn split_into_files(content: &str) -> Vec<(String, String)> {
    let mut files = Vec::new();
    let mut current_filename = "SKILL.md".to_string();
    let mut current_content = String::new();

    for line in content.lines() {
        if line.starts_with("--- FILE:") && line.ends_with("---") {
            // Save previous file
            if !current_content.trim().is_empty() {
                files.push((current_filename.clone(), current_content.trim().to_string()));
            }

            // Extract new filename from separator
            current_filename = line
                .trim_start_matches("--- FILE:")
                .trim_end_matches("---")
                .trim()
                .to_string();
            current_content = String::new();
        } else {
            current_content.push_str(line);
            current_content.push('\n');
        }
    }

    // Don't forget the last file
    if !current_content.trim().is_empty() {
        files.push((current_filename, current_content.trim().to_string()));
    }

    files
}

// ===========================================
// Frontmatter Validation Tests
// ===========================================

#[test]
fn test_valid_frontmatter_minimal() {
    let content = r#"---
name: test-library
description: Expert knowledge for testing
---

# Test Library
"#;

    let result = parse_and_validate_frontmatter(content);
    assert!(result.is_ok());

    let (frontmatter, body) = result.unwrap();
    assert_eq!(frontmatter.name, "test-library");
    assert_eq!(frontmatter.description, "Expert knowledge for testing");
    assert_eq!(frontmatter.tools, None);
    assert_eq!(frontmatter.last_updated, None);
    assert_eq!(frontmatter.hash, None);
    assert!(body.contains("# Test Library"));
}

#[test]
fn test_valid_frontmatter_with_all_fields() {
    let content = r#"---
name: complete-skill
description: A complete skill with all fields
tools:
  - Read
  - Write
  - Bash
last_updated: "2025-12-29"
hash: "abc123def456"
---

# Complete Skill

Full content here.
"#;

    let result = parse_and_validate_frontmatter(content);
    assert!(result.is_ok());

    let (frontmatter, body) = result.unwrap();
    assert_eq!(frontmatter.name, "complete-skill");
    assert_eq!(frontmatter.description, "A complete skill with all fields");
    assert_eq!(
        frontmatter.tools,
        Some(vec![
            "Read".to_string(),
            "Write".to_string(),
            "Bash".to_string()
        ])
    );
    assert_eq!(frontmatter.last_updated, Some("2025-12-29".to_string()));
    assert_eq!(frontmatter.hash, Some("abc123def456".to_string()));
    assert!(body.contains("# Complete Skill"));
}

#[test]
fn test_valid_frontmatter_with_allowed_tools() {
    let content = r#"---
name: allowed-tools-skill
description: Skill using allowed-tools instead of tools
allowed-tools:
  - Grep
  - Glob
---

# Allowed Tools Skill
"#;

    let result = parse_and_validate_frontmatter(content);
    assert!(result.is_ok());

    let (frontmatter, _body) = result.unwrap();
    assert_eq!(frontmatter.name, "allowed-tools-skill");
    // The "allowed-tools" field should be aliased to "tools"
    assert_eq!(
        frontmatter.tools,
        Some(vec!["Grep".to_string(), "Glob".to_string()])
    );
}

#[test]
fn test_missing_frontmatter_error() {
    let content = "# No frontmatter here\n\nJust content.";
    let result = parse_and_validate_frontmatter(content);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        FrontmatterError::MissingFrontmatter
    ));
}

#[test]
fn test_unclosed_frontmatter_error() {
    let content = r#"---
name: unclosed
description: Missing closing delimiter
# Content without closing ---
"#;

    let result = parse_and_validate_frontmatter(content);
    assert!(result.is_err());

    // Should be either UnclosedFrontmatter or MissingFrontmatter
    match result.unwrap_err() {
        FrontmatterError::UnclosedFrontmatter | FrontmatterError::MissingFrontmatter => {}
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

    // serde will fail to deserialize without name field
    match result.unwrap_err() {
        FrontmatterError::InvalidYaml(_) => {}
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

    // serde will fail to deserialize without description field
    match result.unwrap_err() {
        FrontmatterError::InvalidYaml(_) => {}
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
fn test_optional_fields_are_optional() {
    let content = r#"---
name: minimal
description: Just the required fields
---
Body
"#;

    let result = parse_and_validate_frontmatter(content);
    assert!(result.is_ok());

    let (frontmatter, _) = result.unwrap();
    assert_eq!(frontmatter.tools, None);
    assert_eq!(frontmatter.last_updated, None);
    assert_eq!(frontmatter.hash, None);
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

// ===========================================
// ResearchMetadata Tests
// ===========================================

#[test]
fn test_metadata_with_when_to_use_serialization() {
    let mut metadata = ResearchMetadata::new_library(None);
    metadata.when_to_use = Some("Use when building CLIs".to_string());

    let json = serde_json::to_string(&metadata).unwrap();
    assert!(json.contains("when_to_use"));

    let deserialized: ResearchMetadata = serde_json::from_str(&json).unwrap();
    assert_eq!(
        deserialized.when_to_use,
        Some("Use when building CLIs".to_string())
    );
}

#[test]
fn test_metadata_schema_version_defaults_to_zero() {
    let metadata = ResearchMetadata::new_library(None);
    assert_eq!(metadata.schema_version, 0);

    // Verify it serializes correctly
    let json = serde_json::to_string(&metadata).unwrap();
    let deserialized: ResearchMetadata = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.schema_version, 0);
}

#[test]
fn test_backward_compatibility_missing_when_to_use() {
    // Old metadata.json without when_to_use field
    let old_json = r#"{
        "schema_version": 0,
        "kind": "library",
        "created_at": "2025-01-01T00:00:00Z",
        "updated_at": "2025-01-01T00:00:00Z",
        "brief": "",
        "summary": "",
        "questions": [],
        "additional_files": {}
    }"#;

    let result: Result<ResearchMetadata, _> = serde_json::from_str(old_json);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().when_to_use, None);
}

#[test]
fn test_backward_compatibility_missing_schema_version() {
    // Very old metadata.json without schema_version field
    let old_json = r#"{
        "kind": "library",
        "created_at": "2025-01-01T00:00:00Z",
        "updated_at": "2025-01-01T00:00:00Z",
        "brief": "",
        "summary": "",
        "questions": [],
        "additional_files": {}
    }"#;

    let result: Result<ResearchMetadata, _> = serde_json::from_str(old_json);
    assert!(result.is_ok());
    // Should default to 0
    assert_eq!(result.unwrap().schema_version, 0);
}

#[test]
fn test_metadata_when_to_use_roundtrip() {
    let mut metadata = ResearchMetadata::new_library(Some(&LibraryInfo {
        package_manager: "npm".to_string(),
        language: "typescript".to_string(),
        url: "https://example.com".to_string(),
        repository: None,
        description: None,
    }));

    metadata.when_to_use = Some("Use when you need advanced testing capabilities with snapshot support and parallel execution".to_string());

    // Serialize and deserialize
    let json = serde_json::to_string_pretty(&metadata).unwrap();
    let deserialized: ResearchMetadata = serde_json::from_str(&json).unwrap();

    assert_eq!(metadata.when_to_use, deserialized.when_to_use);
    assert_eq!(metadata.schema_version, deserialized.schema_version);
}

// ===========================================
// File Splitting Tests
// ===========================================

#[test]
fn test_split_into_files_single_file_no_separators() {
    let content = r#"---
name: Test Skill
description: Test
---

# Test Content

Some content here."#;

    let files = split_into_files(content);

    assert_eq!(files.len(), 1);
    assert_eq!(files[0].0, "SKILL.md");
    assert!(files[0].1.contains("Test Content"));
}

#[test]
fn test_split_into_files_multiple_files() {
    let content = r#"---
name: Test Skill
description: Test
---

# Main Content

This is the main skill content.

--- FILE: advanced-usage.md ---

# Advanced Usage

Advanced content here.

--- FILE: examples.md ---

# Examples

Example content here."#;

    let files = split_into_files(content);

    assert_eq!(files.len(), 3);
    assert_eq!(files[0].0, "SKILL.md");
    assert!(files[0].1.contains("Main Content"));

    assert_eq!(files[1].0, "advanced-usage.md");
    assert!(files[1].1.contains("Advanced Usage"));

    assert_eq!(files[2].0, "examples.md");
    assert!(files[2].1.contains("Examples"));
}

// ===========================================
// Regression Test for Original Bug
// ===========================================

#[test]
fn test_regression_llm_output_with_file_separator_at_start() {
    // Simulate broken LLM output that caused the original bug
    // LLM incorrectly includes "--- FILE: SKILL.md ---" at the start
    let broken_llm_output = r#"--- FILE: SKILL.md ---
---
name: test-lib
description: Test library for regression testing
---

# Test Library

Some content here.

--- FILE: examples.md ---
# Examples

Example content."#;

    // After splitting, first file should be SKILL.md (implicit)
    let files = split_into_files(broken_llm_output);
    assert_eq!(files.len(), 2);
    assert_eq!(files[0].0, "SKILL.md");
    assert_eq!(files[1].0, "examples.md");

    // SKILL.md content should NOT contain the file separator
    assert!(
        !files[0].1.contains("--- FILE:"),
        "SKILL.md should not contain file separator"
    );

    // SKILL.md should have valid frontmatter
    let result = parse_and_validate_frontmatter(&files[0].1);
    assert!(result.is_ok(), "SKILL.md should have valid frontmatter");

    let (frontmatter, _body) = result.unwrap();
    assert_eq!(frontmatter.name, "test-lib");
    assert_eq!(
        frontmatter.description,
        "Test library for regression testing"
    );
}

#[test]
fn test_regression_multiple_file_separators_at_start() {
    // Edge case: LLM includes separator at start AND creates multiple files
    let content = r#"--- FILE: SKILL.md ---
---
name: multi-file
description: Multi-file skill
---

# Main

Main content

--- FILE: usage.md ---
# Usage

Usage content

--- FILE: api.md ---
# API

API content"#;

    let files = split_into_files(content);
    assert_eq!(files.len(), 3);

    // All files should be correctly named
    assert_eq!(files[0].0, "SKILL.md");
    assert_eq!(files[1].0, "usage.md");
    assert_eq!(files[2].0, "api.md");

    // SKILL.md should be valid
    let result = parse_and_validate_frontmatter(&files[0].1);
    assert!(result.is_ok());
}

// ===========================================
// Whitespace Variation Tests
// ===========================================

#[test]
fn test_whitespace_variations_crlf_line_endings() {
    // CRLF line endings (Windows style)
    let content_crlf = "---\r\nname: test\r\ndescription: Test lib\r\n---\r\n\r\n# Content";
    let result = parse_and_validate_frontmatter(content_crlf);
    assert!(result.is_ok());

    let (frontmatter, _) = result.unwrap();
    assert_eq!(frontmatter.name, "test");
}

#[test]
fn test_whitespace_variations_trailing_spaces() {
    // Trailing whitespace after ---
    let content_trailing = "---   \nname: test\ndescription: Test lib\n---   \n\n# Content";
    let result = parse_and_validate_frontmatter(content_trailing);
    assert!(result.is_ok());

    let (frontmatter, _) = result.unwrap();
    assert_eq!(frontmatter.name, "test");
}

#[test]
fn test_whitespace_variations_extra_blank_lines() {
    // Extra blank lines in frontmatter
    let content_blanks = r#"---

name: test

description: Test lib

---

# Content"#;
    let result = parse_and_validate_frontmatter(content_blanks);
    assert!(result.is_ok());

    let (frontmatter, _) = result.unwrap();
    assert_eq!(frontmatter.name, "test");
}

#[test]
fn test_whitespace_in_field_values_trimmed() {
    let content = r#"---
name: "  test-skill  "
description: "  Description with spaces  "
---
Body"#;

    let result = parse_and_validate_frontmatter(content);
    assert!(result.is_ok());

    let (frontmatter, _) = result.unwrap();
    // Values should be trimmed
    assert_eq!(frontmatter.name, "test-skill");
    assert_eq!(frontmatter.description, "Description with spaces");
}

#[test]
fn test_file_separator_with_extra_whitespace() {
    let content = r#"---
name: test
description: test
---

Main content

--- FILE:   spaces.md   ---

Content with spaces in separator."#;

    let files = split_into_files(content);
    assert_eq!(files.len(), 2);
    assert_eq!(files[0].0, "SKILL.md");
    // Filename should be trimmed
    assert_eq!(files[1].0, "spaces.md");
}

// ===========================================
// Edge Cases
// ===========================================

#[test]
fn test_empty_content_between_separators_skipped() {
    let content = r#"---
name: Test Skill
description: test
---

# Main Content

--- FILE: empty.md ---

--- FILE: real.md ---

# Real Content"#;

    let files = split_into_files(content);

    // Empty file should be skipped
    assert_eq!(files.len(), 2);
    assert_eq!(files[0].0, "SKILL.md");
    assert_eq!(files[1].0, "real.md");
    assert!(files[1].1.contains("Real Content"));
}

#[test]
fn test_file_separator_at_end() {
    let content = r#"---
name: Test Skill
description: test
---

# Main Content

--- FILE: additional.md ---

# Additional Content

Last line."#;

    let files = split_into_files(content);

    assert_eq!(files.len(), 2);
    assert_eq!(files[0].0, "SKILL.md");
    assert_eq!(files[1].0, "additional.md");
    assert!(files[1].1.contains("Last line"));
}

#[test]
fn test_only_file_separator_no_content() {
    let content = "--- FILE: SKILL.md ---";

    let files = split_into_files(content);
    // Should return empty vec since there's no actual content
    assert_eq!(files.len(), 0);
}

#[test]
fn test_frontmatter_with_complex_yaml_structures() {
    let content = r#"---
name: complex-skill
description: Skill with complex YAML structures
tools:
  - Read
  - Write
  - Bash
  - Grep
  - Glob
last_updated: "2025-12-29T10:30:00Z"
hash: "sha256:abc123def456"
---

# Complex Skill"#;

    let result = parse_and_validate_frontmatter(content);
    assert!(result.is_ok());

    let (frontmatter, _) = result.unwrap();
    assert_eq!(frontmatter.tools.as_ref().unwrap().len(), 5);
    assert!(frontmatter.last_updated.is_some());
    assert!(frontmatter.hash.is_some());
}

#[test]
fn test_body_content_preserved_exactly() {
    let content = r#"---
name: test
description: test
---
# Heading 1

Content line 1
Content line 2

## Heading 2

More content"#;

    let result = parse_and_validate_frontmatter(content);
    assert!(result.is_ok());

    let (_frontmatter, body) = result.unwrap();
    assert!(body.contains("# Heading 1"));
    assert!(body.contains("Content line 1"));
    assert!(body.contains("Content line 2"));
    assert!(body.contains("## Heading 2"));
    assert!(body.contains("More content"));
}

#[test]
fn test_frontmatter_serialization_roundtrip() {
    let original = SkillFrontmatter {
        name: "test-skill".to_string(),
        description: "A test skill".to_string(),
        tools: Some(vec!["Bash".to_string(), "Read".to_string()]),
        last_updated: Some("2025-12-29".to_string()),
        hash: Some("abc123".to_string()),
    };

    // Serialize to YAML
    let yaml = serde_yaml::to_string(&original).unwrap();

    // Deserialize back
    let deserialized: SkillFrontmatter = serde_yaml::from_str(&yaml).unwrap();

    assert_eq!(original.name, deserialized.name);
    assert_eq!(original.description, deserialized.description);
    assert_eq!(original.tools, deserialized.tools);
    assert_eq!(original.last_updated, deserialized.last_updated);
    assert_eq!(original.hash, deserialized.hash);
}

#[test]
fn test_multiple_files_all_have_valid_content() {
    let content = r#"---
name: multi-file
description: Multi-file skill
---

# Main

Main content here

--- FILE: usage.md ---

# Usage Guide

Usage content here

--- FILE: examples.md ---

# Examples

Example 1
Example 2"#;

    let files = split_into_files(content);
    assert_eq!(files.len(), 3);

    // Verify each file has substantial content
    for (filename, file_content) in &files {
        assert!(
            !file_content.trim().is_empty(),
            "{} should not be empty",
            filename
        );
        assert!(
            file_content.contains("#"),
            "{} should contain headers",
            filename
        );
    }
}

#[test]
fn test_skill_with_code_blocks_preserved() {
    let content = r#"---
name: code-skill
description: Skill with code blocks
---

# Code Examples

```rust
fn main() {
    println!("Hello");
}
```

--- FILE: more-examples.md ---

```typescript
console.log("test");
```"#;

    let files = split_into_files(content);
    assert_eq!(files.len(), 2);

    assert!(files[0].1.contains("```rust"));
    assert!(files[0].1.contains("fn main()"));
    assert!(files[1].1.contains("```typescript"));
}
