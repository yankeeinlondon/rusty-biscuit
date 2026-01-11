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
    pub fn add_range(&mut self, start: usize, end: usize) -> Result<(), crate::markdown::MarkdownError> {
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
        Self { start: line, end: line }
    }

    /// Creates a range from start to end (inclusive).
    ///
    /// ## Errors
    ///
    /// Returns an error if start > end.
    pub fn range(start: usize, end: usize) -> Result<Self, crate::markdown::MarkdownError> {
        if start > end {
            Err(crate::markdown::MarkdownError::InvalidLineRange(
                format!("{}-{} (start must be <= end)", start, end)
            ))
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
