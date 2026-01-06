//! EditorConfig detection and formatting configuration.
//!
//! This module provides detection of `.editorconfig` files and parsing of their
//! formatting rules. EditorConfig is a file format for defining consistent coding
//! styles across different editors and IDEs.
//!
//! ## Examples
//!
//! ```no_run
//! use sniff_lib::filesystem::formatting::detect_formatting;
//! use std::path::Path;
//!
//! let config = detect_formatting(Path::new(".")).unwrap();
//! if let Some(formatting) = config {
//!     for section in &formatting.sections {
//!         println!("Pattern: {}", section.pattern);
//!         if let Some(style) = &section.indent_style {
//!             println!("  indent_style: {}", style);
//!         }
//!     }
//! }
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::Result;

/// Complete formatting configuration from EditorConfig.
///
/// Contains all sections parsed from `.editorconfig` files found in the
/// directory hierarchy.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FormattingConfig {
    /// Path to the `.editorconfig` file.
    pub config_path: PathBuf,
    /// Whether this is a root configuration (stops searching parent directories).
    pub is_root: bool,
    /// All sections from the EditorConfig file.
    pub sections: Vec<EditorConfigSection>,
}

/// A single section from an EditorConfig file.
///
/// Each section corresponds to a glob pattern (e.g., `[*]`, `[*.py]`, `[Makefile]`)
/// and contains the formatting properties that apply to matching files.
///
/// ## Examples
///
/// For an EditorConfig section like:
/// ```text
/// [*.rs]
/// indent_style = space
/// indent_size = 4
/// ```
///
/// The resulting `EditorConfigSection` would have:
/// - `pattern`: `"*.rs"`
/// - `indent_style`: `Some("space")`
/// - `indent_size`: `Some(4)`
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EditorConfigSection {
    /// The glob pattern for this section (e.g., `*`, `*.py`, `Makefile`).
    pub pattern: String,
    /// Indentation style: `tab` or `space`.
    pub indent_style: Option<String>,
    /// Number of spaces for indentation (when using spaces).
    pub indent_size: Option<u32>,
    /// Width of a tab character.
    pub tab_width: Option<u32>,
    /// Line ending style: `lf`, `crlf`, or `cr`.
    pub end_of_line: Option<String>,
    /// Character encoding: `utf-8`, `utf-8-bom`, `utf-16be`, `utf-16le`, `latin1`.
    pub charset: Option<String>,
    /// Whether to trim trailing whitespace.
    pub trim_trailing_whitespace: Option<bool>,
    /// Whether to ensure file ends with a newline.
    pub insert_final_newline: Option<bool>,
    /// Maximum line length.
    pub max_line_length: Option<u32>,
    /// Additional non-standard properties.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub extra: HashMap<String, String>,
}

/// Detects EditorConfig formatting configuration for a directory.
///
/// Searches for `.editorconfig` files starting from `root` and traversing
/// parent directories until a root configuration is found or the filesystem
/// root is reached.
///
/// ## Returns
///
/// - `Ok(Some(config))` if an `.editorconfig` file is found
/// - `Ok(None)` if no `.editorconfig` file exists in the hierarchy
///
/// ## Errors
///
/// Returns an error if the `.editorconfig` file exists but cannot be read.
///
/// ## Examples
///
/// ```no_run
/// use sniff_lib::filesystem::formatting::detect_formatting;
/// use std::path::Path;
///
/// let config = detect_formatting(Path::new("/path/to/project")).unwrap();
/// match config {
///     Some(cfg) => println!("Found config at: {:?}", cfg.config_path),
///     None => println!("No .editorconfig found"),
/// }
/// ```
pub fn detect_formatting(root: &Path) -> Result<Option<FormattingConfig>> {
    // Search for .editorconfig in root and parent directories
    let config_path = find_editorconfig(root);

    let Some(path) = config_path else {
        return Ok(None);
    };

    let content = fs::read_to_string(&path)?;
    let config = parse_editorconfig(&content, path);

    Ok(Some(config))
}

/// Searches for `.editorconfig` in the given directory and its parents.
fn find_editorconfig(start: &Path) -> Option<PathBuf> {
    let mut current = if start.is_file() {
        start.parent()?.to_path_buf()
    } else {
        start.to_path_buf()
    };

    loop {
        let config_path = current.join(".editorconfig");
        if config_path.exists() {
            return Some(config_path);
        }

        match current.parent() {
            Some(parent) if parent != current => {
                current = parent.to_path_buf();
            }
            _ => break,
        }
    }

    None
}

/// Parses EditorConfig content into a `FormattingConfig`.
fn parse_editorconfig(content: &str, path: PathBuf) -> FormattingConfig {
    let mut config = FormattingConfig {
        config_path: path,
        is_root: false,
        sections: Vec::new(),
    };

    let mut current_section: Option<EditorConfigSection> = None;

    for line in content.lines() {
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }

        // Check for section header
        if line.starts_with('[') && line.ends_with(']') {
            // Save previous section
            if let Some(section) = current_section.take() {
                config.sections.push(section);
            }

            // Start new section
            let pattern = line[1..line.len() - 1].to_string();
            current_section = Some(EditorConfigSection {
                pattern,
                ..Default::default()
            });
            continue;
        }

        // Parse key = value
        if let Some((key, value)) = parse_property(line) {
            let key_lower = key.to_lowercase();
            let value_trimmed = value.trim();

            // Handle root property (not in a section)
            if key_lower == "root" && current_section.is_none() {
                config.is_root = value_trimmed.eq_ignore_ascii_case("true");
                continue;
            }

            // Handle section properties
            if let Some(ref mut section) = current_section {
                apply_property(section, &key_lower, value_trimmed);
            }
        }
    }

    // Don't forget the last section
    if let Some(section) = current_section {
        config.sections.push(section);
    }

    config
}

/// Parses a `key = value` or `key: value` line.
fn parse_property(line: &str) -> Option<(&str, &str)> {
    // Try '=' first, then ':'
    let (key, value) = if let Some(pos) = line.find('=') {
        (&line[..pos], &line[pos + 1..])
    } else if let Some(pos) = line.find(':') {
        (&line[..pos], &line[pos + 1..])
    } else {
        return None;
    };

    Some((key.trim(), value.trim()))
}

/// Applies a property to an EditorConfigSection.
fn apply_property(section: &mut EditorConfigSection, key: &str, value: &str) {
    // Handle 'unset' values
    if value.eq_ignore_ascii_case("unset") {
        match key {
            "indent_style" => section.indent_style = None,
            "indent_size" => section.indent_size = None,
            "tab_width" => section.tab_width = None,
            "end_of_line" => section.end_of_line = None,
            "charset" => section.charset = None,
            "trim_trailing_whitespace" => section.trim_trailing_whitespace = None,
            "insert_final_newline" => section.insert_final_newline = None,
            "max_line_length" => section.max_line_length = None,
            _ => {
                section.extra.remove(key);
            }
        }
        return;
    }

    match key {
        "indent_style" => {
            section.indent_style = Some(value.to_lowercase());
        }
        "indent_size" => {
            // Handle 'tab' as a special value
            if value.eq_ignore_ascii_case("tab") {
                // When indent_size is 'tab', it should use tab_width
                // We store it as None to indicate this special case
                section.indent_size = None;
            } else {
                section.indent_size = value.parse().ok();
            }
        }
        "tab_width" => {
            section.tab_width = value.parse().ok();
        }
        "end_of_line" => {
            section.end_of_line = Some(value.to_lowercase());
        }
        "charset" => {
            section.charset = Some(value.to_lowercase());
        }
        "trim_trailing_whitespace" => {
            section.trim_trailing_whitespace = parse_bool(value);
        }
        "insert_final_newline" => {
            section.insert_final_newline = parse_bool(value);
        }
        "max_line_length" => {
            // Handle 'off' as a special value
            if !value.eq_ignore_ascii_case("off") {
                section.max_line_length = value.parse().ok();
            }
        }
        _ => {
            // Store unknown properties in extra
            section.extra.insert(key.to_string(), value.to_string());
        }
    }
}

/// Parses a boolean value (true/false).
fn parse_bool(value: &str) -> Option<bool> {
    match value.to_lowercase().as_str() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_no_editorconfig_returns_none() {
        let dir = TempDir::new().unwrap();
        let result = detect_formatting(dir.path()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_single_section() {
        let content = r#"
root = true

[*]
indent_style = space
indent_size = 4
"#;
        let config = parse_editorconfig(content, PathBuf::from(".editorconfig"));

        assert!(config.is_root);
        assert_eq!(config.sections.len(), 1);

        let section = &config.sections[0];
        assert_eq!(section.pattern, "*");
        assert_eq!(section.indent_style, Some("space".to_string()));
        assert_eq!(section.indent_size, Some(4));
    }

    #[test]
    fn test_parse_multiple_sections() {
        let content = r#"
root = true

[*]
indent_style = space
indent_size = 2

[*.py]
indent_size = 4

[Makefile]
indent_style = tab

[*.{js,ts}]
indent_size = 2
max_line_length = 100
"#;
        let config = parse_editorconfig(content, PathBuf::from(".editorconfig"));

        assert!(config.is_root);
        assert_eq!(config.sections.len(), 4);

        // Check [*] section
        assert_eq!(config.sections[0].pattern, "*");
        assert_eq!(config.sections[0].indent_style, Some("space".to_string()));
        assert_eq!(config.sections[0].indent_size, Some(2));

        // Check [*.py] section
        assert_eq!(config.sections[1].pattern, "*.py");
        assert_eq!(config.sections[1].indent_size, Some(4));

        // Check [Makefile] section
        assert_eq!(config.sections[2].pattern, "Makefile");
        assert_eq!(config.sections[2].indent_style, Some("tab".to_string()));

        // Check [*.{js,ts}] section
        assert_eq!(config.sections[3].pattern, "*.{js,ts}");
        assert_eq!(config.sections[3].indent_size, Some(2));
        assert_eq!(config.sections[3].max_line_length, Some(100));
    }

    #[test]
    fn test_parse_all_property_types() {
        let content = r#"
[*]
indent_style = tab
indent_size = 4
tab_width = 8
end_of_line = lf
charset = utf-8
trim_trailing_whitespace = true
insert_final_newline = true
max_line_length = 120
"#;
        let config = parse_editorconfig(content, PathBuf::from(".editorconfig"));
        let section = &config.sections[0];

        assert_eq!(section.pattern, "*");
        assert_eq!(section.indent_style, Some("tab".to_string()));
        assert_eq!(section.indent_size, Some(4));
        assert_eq!(section.tab_width, Some(8));
        assert_eq!(section.end_of_line, Some("lf".to_string()));
        assert_eq!(section.charset, Some("utf-8".to_string()));
        assert_eq!(section.trim_trailing_whitespace, Some(true));
        assert_eq!(section.insert_final_newline, Some(true));
        assert_eq!(section.max_line_length, Some(120));
    }

    #[test]
    fn test_parse_boolean_false() {
        let content = r#"
[*]
trim_trailing_whitespace = false
insert_final_newline = false
"#;
        let config = parse_editorconfig(content, PathBuf::from(".editorconfig"));
        let section = &config.sections[0];

        assert_eq!(section.trim_trailing_whitespace, Some(false));
        assert_eq!(section.insert_final_newline, Some(false));
    }

    #[test]
    fn test_parse_crlf_line_ending() {
        let content = r#"
[*.bat]
end_of_line = crlf
"#;
        let config = parse_editorconfig(content, PathBuf::from(".editorconfig"));
        let section = &config.sections[0];

        assert_eq!(section.end_of_line, Some("crlf".to_string()));
    }

    #[test]
    fn test_parse_comments_ignored() {
        let content = r#"
# This is a comment
; This is also a comment
root = true

[*]
# Another comment
indent_style = space
"#;
        let config = parse_editorconfig(content, PathBuf::from(".editorconfig"));

        assert!(config.is_root);
        assert_eq!(config.sections.len(), 1);
        assert_eq!(config.sections[0].indent_style, Some("space".to_string()));
    }

    #[test]
    fn test_parse_max_line_length_off() {
        let content = r#"
[*]
max_line_length = off
"#;
        let config = parse_editorconfig(content, PathBuf::from(".editorconfig"));
        let section = &config.sections[0];

        // 'off' should result in None
        assert_eq!(section.max_line_length, None);
    }

    #[test]
    fn test_parse_extra_properties() {
        let content = r#"
[*]
indent_style = space
some_custom_property = custom_value
"#;
        let config = parse_editorconfig(content, PathBuf::from(".editorconfig"));
        let section = &config.sections[0];

        assert_eq!(section.indent_style, Some("space".to_string()));
        assert_eq!(
            section.extra.get("some_custom_property"),
            Some(&"custom_value".to_string())
        );
    }

    #[test]
    fn test_parse_case_insensitive_values() {
        let content = r#"
root = TRUE

[*]
indent_style = SPACE
end_of_line = CRLF
trim_trailing_whitespace = TRUE
"#;
        let config = parse_editorconfig(content, PathBuf::from(".editorconfig"));

        assert!(config.is_root);
        let section = &config.sections[0];
        assert_eq!(section.indent_style, Some("space".to_string()));
        assert_eq!(section.end_of_line, Some("crlf".to_string()));
        assert_eq!(section.trim_trailing_whitespace, Some(true));
    }

    #[test]
    fn test_parse_unset_value() {
        let content = r#"
[*]
indent_style = space
indent_size = 4

[*.md]
indent_size = unset
"#;
        let config = parse_editorconfig(content, PathBuf::from(".editorconfig"));

        assert_eq!(config.sections.len(), 2);
        assert_eq!(config.sections[0].indent_size, Some(4));
        assert_eq!(config.sections[1].indent_size, None);
    }

    #[test]
    fn test_detect_formatting_with_file() {
        let dir = TempDir::new().unwrap();
        let editorconfig_path = dir.path().join(".editorconfig");

        fs::write(
            &editorconfig_path,
            r#"
root = true

[*]
indent_style = space
indent_size = 2
"#,
        )
        .unwrap();

        let result = detect_formatting(dir.path()).unwrap();
        assert!(result.is_some());

        let config = result.unwrap();
        assert!(config.is_root);
        assert_eq!(config.config_path, editorconfig_path);
        assert_eq!(config.sections.len(), 1);
        assert_eq!(config.sections[0].indent_size, Some(2));
    }

    #[test]
    fn test_find_editorconfig_in_parent() {
        let parent = TempDir::new().unwrap();
        let child = parent.path().join("subdir");
        fs::create_dir(&child).unwrap();

        let editorconfig_path = parent.path().join(".editorconfig");
        fs::write(
            &editorconfig_path,
            r#"
root = true

[*]
indent_style = tab
"#,
        )
        .unwrap();

        let result = detect_formatting(&child).unwrap();
        assert!(result.is_some());

        let config = result.unwrap();
        // Canonicalize both paths to handle /private/var vs /var on macOS
        assert_eq!(
            config.config_path.canonicalize().unwrap(),
            editorconfig_path.canonicalize().unwrap()
        );
        assert_eq!(config.sections[0].indent_style, Some("tab".to_string()));
    }

    #[test]
    fn test_parse_colon_separator() {
        let content = r#"
[*]
indent_style: space
indent_size: 4
"#;
        let config = parse_editorconfig(content, PathBuf::from(".editorconfig"));
        let section = &config.sections[0];

        assert_eq!(section.indent_style, Some("space".to_string()));
        assert_eq!(section.indent_size, Some(4));
    }

    #[test]
    fn test_parse_non_root_config() {
        let content = r#"
[*]
indent_style = space
"#;
        let config = parse_editorconfig(content, PathBuf::from(".editorconfig"));

        assert!(!config.is_root);
    }

    #[test]
    fn test_parse_root_false() {
        let content = r#"
root = false

[*]
indent_style = space
"#;
        let config = parse_editorconfig(content, PathBuf::from(".editorconfig"));

        assert!(!config.is_root);
    }
}
