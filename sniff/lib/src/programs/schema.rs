//! Schema definitions for program metadata and version parsing.
//!
//! This module provides the core traits and types for program detection,
//! including metadata lookup, version parsing strategies, and error handling.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during program version detection.
#[derive(Debug, Error)]
pub enum ProgramError {
    /// The program was not found in PATH.
    #[error("program '{0}' not found in PATH")]
    NotFound(String),

    /// Failed to execute the version command.
    #[error("failed to execute version command for '{program}': {source}")]
    ExecutionFailed {
        program: String,
        #[source]
        source: std::io::Error,
    },

    /// Failed to parse version output.
    #[error("failed to parse version output for '{program}': {details}")]
    ParseFailed { program: String, details: String },

    /// The version command returned non-zero exit code.
    #[error("version command for '{program}' failed with exit code {code}")]
    NonZeroExit { program: String, code: i32 },
}

/// Strategy for parsing version output from a program.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VersionParseStrategy {
    /// Extract first line of output (most common).
    FirstLine,

    /// Parse as semantic version (X.Y.Z pattern).
    SemVer,

    /// Use regex pattern to extract version.
    /// The pattern should have a capture group named "version".
    Regex,

    /// Version appears after a specific prefix on the first line.
    /// E.g., "git version 2.34.0" -> extract after "version "
    AfterPrefix,

    /// Custom parsing required (handled by implementation).
    Custom,
}

/// Command-line flag used to get version information.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionFlag {
    /// Standard --version flag
    Long,
    /// Short -v flag
    Short,
    /// Short -V flag (uppercase, used by some tools)
    ShortUpper,
    /// Version subcommand (e.g., "git version")
    Subcommand(&'static str),
    /// Custom flag or argument
    Custom(&'static str),
    /// No version command available
    None,
}

impl VersionFlag {
    /// Returns the command-line arguments for this version flag.
    pub fn as_args(&self) -> &[&str] {
        match self {
            VersionFlag::Long => &["--version"],
            VersionFlag::Short => &["-v"],
            VersionFlag::ShortUpper => &["-V"],
            VersionFlag::Subcommand(cmd) => std::slice::from_ref(cmd),
            VersionFlag::Custom(arg) => std::slice::from_ref(arg),
            VersionFlag::None => &[],
        }
    }
}

/// Metadata about a program including its name, website, and version detection.
#[derive(Debug, Clone)]
pub struct ProgramInfo {
    /// The binary name used to invoke this program.
    pub binary_name: &'static str,

    /// Human-readable display name.
    pub display_name: &'static str,

    /// One-line description of the program.
    pub description: &'static str,

    /// Official website URL.
    pub website: &'static str,

    /// Command-line flag to get version.
    pub version_flag: VersionFlag,

    /// Strategy for parsing version output.
    pub parse_strategy: VersionParseStrategy,

    /// Optional regex pattern for version extraction.
    /// Used when parse_strategy is Regex.
    pub version_regex: Option<&'static str>,

    /// Optional prefix to skip when parsing version.
    /// Used when parse_strategy is AfterPrefix.
    pub version_prefix: Option<&'static str>,
}

impl ProgramInfo {
    /// Creates a new ProgramInfo with standard --version flag.
    pub const fn standard(
        binary_name: &'static str,
        display_name: &'static str,
        description: &'static str,
        website: &'static str,
    ) -> Self {
        Self {
            binary_name,
            display_name,
            description,
            website,
            version_flag: VersionFlag::Long,
            parse_strategy: VersionParseStrategy::FirstLine,
            version_regex: None,
            version_prefix: None,
        }
    }

    /// Creates a new ProgramInfo with a prefix-based version parse.
    pub const fn with_prefix(
        binary_name: &'static str,
        display_name: &'static str,
        description: &'static str,
        website: &'static str,
        prefix: &'static str,
    ) -> Self {
        Self {
            binary_name,
            display_name,
            description,
            website,
            version_flag: VersionFlag::Long,
            parse_strategy: VersionParseStrategy::AfterPrefix,
            version_regex: None,
            version_prefix: Some(prefix),
        }
    }
}

/// Trait for program enums that provide metadata lookup.
///
/// This trait should be implemented by enums representing program categories
/// (e.g., `Editor`, `Utility`, `TtsClient`). It provides a uniform interface
/// for accessing program metadata.
///
/// ## Examples
///
/// ```ignore
/// use sniff_lib::programs::schema::ProgramMetadata;
///
/// // Get metadata for a specific editor
/// let info = Editor::Vim.info();
/// println!("Website: {}", info.website);
/// ```
pub trait ProgramMetadata: Sized {
    /// Returns the metadata for this program variant.
    fn info(&self) -> &'static ProgramInfo;

    /// Returns the binary name used to invoke this program.
    fn binary_name(&self) -> &'static str {
        self.info().binary_name
    }

    /// Returns the display name of this program.
    fn display_name(&self) -> &'static str {
        self.info().display_name
    }

    /// Returns a one-line description of this program.
    fn description(&self) -> &'static str {
        self.info().description
    }

    /// Returns the official website URL for this program.
    fn website(&self) -> &'static str {
        self.info().website
    }

    /// Returns the path to this program's binary if installed.
    fn path(&self) -> Option<PathBuf> {
        crate::programs::find_program::find_program(self.binary_name())
    }

    /// Returns the version of this program if available.
    ///
    /// ## Errors
    ///
    /// Returns an error if:
    /// - The program is not found in PATH
    /// - The version command fails to execute
    /// - The version output cannot be parsed
    fn version(&self) -> Result<String, ProgramError> {
        let info = self.info();

        // Check if program exists first
        let path = self
            .path()
            .ok_or_else(|| ProgramError::NotFound(info.binary_name.to_string()))?;

        // Get version flag arguments
        let args = info.version_flag.as_args();
        if args.is_empty() {
            return Err(ProgramError::ParseFailed {
                program: info.binary_name.to_string(),
                details: "no version command available".to_string(),
            });
        }

        // Execute version command
        let output = std::process::Command::new(&path)
            .args(args)
            .output()
            .map_err(|e| ProgramError::ExecutionFailed {
                program: info.binary_name.to_string(),
                source: e,
            })?;

        // Check exit code (some programs return non-zero for --version)
        // We'll be lenient and accept any output

        // Get stdout, falling back to stderr if empty
        let text = if output.stdout.is_empty() {
            String::from_utf8_lossy(&output.stderr).to_string()
        } else {
            String::from_utf8_lossy(&output.stdout).to_string()
        };

        // Parse version based on strategy
        parse_version(&text, info)
    }
}

/// Parses version string based on the specified strategy.
fn parse_version(output: &str, info: &ProgramInfo) -> Result<String, ProgramError> {
    let program = info.binary_name.to_string();

    match info.parse_strategy {
        VersionParseStrategy::FirstLine => output
            .lines()
            .next()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| ProgramError::ParseFailed {
                program,
                details: "empty output".to_string(),
            }),

        VersionParseStrategy::AfterPrefix => {
            let prefix = info.version_prefix.unwrap_or("");
            output
                .lines()
                .next()
                .map(|line| {
                    if let Some(idx) = line.find(prefix) {
                        line[idx + prefix.len()..].trim().to_string()
                    } else {
                        line.trim().to_string()
                    }
                })
                .filter(|s| !s.is_empty())
                .ok_or_else(|| ProgramError::ParseFailed {
                    program,
                    details: format!("prefix '{}' not found", prefix),
                })
        }

        VersionParseStrategy::SemVer => {
            // Look for X.Y.Z pattern
            let re = regex::Regex::new(r"(\d+\.\d+(?:\.\d+)?(?:-[\w.]+)?(?:\+[\w.]+)?)")
                .expect("valid regex");
            re.captures(output)
                .and_then(|caps| caps.get(1))
                .map(|m| m.as_str().to_string())
                .ok_or_else(|| ProgramError::ParseFailed {
                    program,
                    details: "no semantic version found".to_string(),
                })
        }

        VersionParseStrategy::Regex => {
            let pattern = info.version_regex.ok_or_else(|| ProgramError::ParseFailed {
                program: program.clone(),
                details: "regex pattern not specified".to_string(),
            })?;
            let re = regex::Regex::new(pattern).map_err(|e| ProgramError::ParseFailed {
                program: program.clone(),
                details: format!("invalid regex: {}", e),
            })?;
            re.captures(output)
                .and_then(|caps| caps.name("version").or_else(|| caps.get(1)))
                .map(|m| m.as_str().to_string())
                .ok_or_else(|| ProgramError::ParseFailed {
                    program,
                    details: "regex did not match".to_string(),
                })
        }

        VersionParseStrategy::Custom => {
            // Custom parsing should be handled by the implementation
            Err(ProgramError::ParseFailed {
                program,
                details: "custom parsing not implemented".to_string(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_flag_as_args() {
        assert_eq!(VersionFlag::Long.as_args(), &["--version"]);
        assert_eq!(VersionFlag::Short.as_args(), &["-v"]);
        assert_eq!(VersionFlag::ShortUpper.as_args(), &["-V"]);
        assert_eq!(VersionFlag::Subcommand("version").as_args(), &["version"]);
        assert_eq!(VersionFlag::None.as_args(), &[] as &[&str]);
    }

    #[test]
    fn test_parse_version_first_line() {
        let info = ProgramInfo::standard("test", "Test", "A test", "https://test.com");
        let output = "1.2.3\nsome other stuff";
        let result = parse_version(output, &info);
        assert_eq!(result.unwrap(), "1.2.3");
    }

    #[test]
    fn test_parse_version_after_prefix() {
        let info = ProgramInfo::with_prefix("git", "Git", "VCS", "https://git-scm.com", "version ");
        let output = "git version 2.34.0";
        let result = parse_version(output, &info);
        assert_eq!(result.unwrap(), "2.34.0");
    }

    #[test]
    fn test_parse_version_semver() {
        let info = ProgramInfo {
            binary_name: "test",
            display_name: "Test",
            description: "A test",
            website: "https://test.com",
            version_flag: VersionFlag::Long,
            parse_strategy: VersionParseStrategy::SemVer,
            version_regex: None,
            version_prefix: None,
        };
        let output = "Some program version 1.23.456-beta+build.123";
        let result = parse_version(output, &info);
        assert_eq!(result.unwrap(), "1.23.456-beta+build.123");
    }
}
