//! Code block DSL parser for title, line numbering, and highlighting.
//!
//! Provides parsing and representation for code block metadata specified
//! in the info string (the text after the opening backticks).
//!
//! ## Examples
//!
//! Basic language-only:
//! ````markdown
//! ```rust
//! fn main() {}
//! ```
//! ````
//!
//! With title:
//! ````markdown
//! ```rust title="Main function"
//! fn main() {}
//! ```
//! ````
//!
//! With line numbering and highlighting:
//! ````markdown
//! ```ts line-numbering=true highlight=1,4-6
//! const x = 1;
//! const y = 2;
//! const z = 3;
//! const result = x + y;
//! const final = result + z;
//! console.log(final);
//! ```
//! ````

mod parser;

pub use parser::parse_code_info;

use std::collections::HashMap;
use std::fmt;

/// Metadata extracted from a code block's info string.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CodeBlockMeta {
    /// Programming language identifier (first token).
    pub language: String,
    /// Optional title for the code block.
    pub title: Option<String>,
    /// Whether to show line numbers.
    pub line_numbering: bool,
    /// Line ranges to highlight.
    pub highlight: HighlightSpec,
    /// Custom key-value pairs for future extensions.
    pub custom: HashMap<String, String>,
}

/// Validated highlight specification containing line ranges.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HighlightSpec(Vec<ValidLineRange>);

impl HighlightSpec {
    /// Creates an empty highlight specification.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a single line to highlight.
    pub fn add_line(&mut self, line: usize) {
        self.0.push(ValidLineRange::single(line));
    }

    /// Adds a range of lines to highlight.
    pub fn add_range(
        &mut self,
        start: usize,
        end: usize,
    ) -> Result<(), crate::markdown::MarkdownError> {
        self.0.push(ValidLineRange::range(start, end)?);
        Ok(())
    }

    /// Checks if a line number should be highlighted.
    pub fn contains(&self, line: usize) -> bool {
        self.0.iter().any(|range| range.contains(line))
    }

    /// Returns the number of highlight ranges.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Checks if the specification is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Iterates the underlying ranges in declaration order.
    pub fn iter(&self) -> std::slice::Iter<'_, ValidLineRange> {
        self.0.iter()
    }
}

impl std::fmt::Display for HighlightSpec {
    /// Re-renders the highlight spec in the same `1,3-5` format the
    /// [`parse_code_info`](super::parser::parse_code_info) DSL accepts.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut first = true;
        for range in &self.0 {
            if !first {
                f.write_str(",")?;
            }
            first = false;
            if range.start == range.end {
                write!(f, "{}", range.start)?;
            } else {
                write!(f, "{}-{}", range.start, range.end)?;
            }
        }
        Ok(())
    }
}

/// Errors that can occur while parsing a [`HighlightSpec`].
#[derive(Debug, Clone, PartialEq)]
pub enum HighlightSpecParseError {
    /// A range segment did not contain exactly one hyphen.
    InvalidRangeFormat {
        /// The raw segment that could not be parsed as a range.
        part: String,
    },
    /// A range segment had a non-numeric start value.
    InvalidStartNumber {
        /// The full raw segment.
        part: String,
        /// The start portion that failed to parse.
        source: String,
    },
    /// A range segment had a non-numeric end value.
    InvalidEndNumber {
        /// The full raw segment.
        part: String,
        /// The end portion that failed to parse.
        source: String,
    },
    /// A non-range segment was not a valid line number.
    InvalidLineNumber {
        /// The raw segment that failed to parse.
        part: String,
    },
    /// A range had a start greater than its end.
    InvalidRange {
        /// Start of the invalid range.
        start: usize,
        /// End of the invalid range.
        end: usize,
    },
}

impl fmt::Display for HighlightSpecParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HighlightSpecParseError::InvalidRangeFormat { part } => {
                write!(f, "Invalid range format: {part}")
            }
            HighlightSpecParseError::InvalidStartNumber { source, .. } => {
                write!(f, "Invalid start number: {source}")
            }
            HighlightSpecParseError::InvalidEndNumber { source, .. } => {
                write!(f, "Invalid end number: {source}")
            }
            HighlightSpecParseError::InvalidLineNumber { part } => {
                write!(f, "Invalid line number: {part}")
            }
            HighlightSpecParseError::InvalidRange { start, end } => {
                write!(f, "{start}-{end} (start must be <= end)")
            }
        }
    }
}

/// Parses a highlight specification from a raw string.
///
/// The grammar is a comma-separated list of line numbers and inclusive
/// ranges such as `1,4-6`. Empty comma segments are ignored, matching the
/// behavior of the code-block DSL parser.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::dsl::{parse_highlight_spec, HighlightSpec};
///
/// let spec = parse_highlight_spec("1,4-6").unwrap();
/// assert!(spec.contains(1));
/// assert!(spec.contains(5));
/// assert!(!spec.contains(2));
/// ```
///
/// ## Errors
///
/// Returns an error if a segment is not a valid line number, a range is
/// malformed, or a range has start > end.
pub fn parse_highlight_spec(raw: &str) -> Result<HighlightSpec, HighlightSpecParseError> {
    let mut spec = HighlightSpec::new();

    for part in raw.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        if part.contains('-') {
            let range_parts: Vec<&str> = part.split('-').collect();
            if range_parts.len() != 2 {
                return Err(HighlightSpecParseError::InvalidRangeFormat {
                    part: part.to_string(),
                });
            }

            let start_src = range_parts[0].trim();
            let end_src = range_parts[1].trim();
            let start = start_src.parse::<usize>().map_err(|_| {
                HighlightSpecParseError::InvalidStartNumber {
                    part: part.to_string(),
                    source: start_src.to_string(),
                }
            })?;
            let end = end_src.parse::<usize>().map_err(|_| {
                HighlightSpecParseError::InvalidEndNumber {
                    part: part.to_string(),
                    source: end_src.to_string(),
                }
            })?;

            let range = ValidLineRange::range(start, end).map_err(|_| {
                HighlightSpecParseError::InvalidRange { start, end }
            })?;
            spec.0.push(range);
        } else {
            let line = part.parse::<usize>().map_err(|_| {
                HighlightSpecParseError::InvalidLineNumber {
                    part: part.to_string(),
                }
            })?;
            spec.add_line(line);
        }
    }

    Ok(spec)
}

/// Validated line range with enforced invariants (start <= end).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ValidLineRange {
    start: usize,
    end: usize,
}

impl ValidLineRange {
    /// Creates a single-line range.
    pub fn single(line: usize) -> Self {
        Self {
            start: line,
            end: line,
        }
    }

    /// Creates a range from start to end (inclusive).
    ///
    /// ## Errors
    ///
    /// Returns an error if start > end.
    pub fn range(start: usize, end: usize) -> Result<Self, crate::markdown::MarkdownError> {
        if start > end {
            Err(crate::markdown::MarkdownError::InvalidLineRange(format!(
                "{}-{} (start must be <= end)",
                start, end
            )))
        } else {
            Ok(Self { start, end })
        }
    }

    /// Checks if a line number falls within this range.
    pub fn contains(&self, line: usize) -> bool {
        line >= self.start && line <= self.end
    }

    /// Returns the start of the range.
    pub fn start(&self) -> usize {
        self.start
    }

    /// Returns the end of the range.
    pub fn end(&self) -> usize {
        self.end
    }
}
