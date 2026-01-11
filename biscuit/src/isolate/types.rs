//! Core types for the isolate module.
//!
//! This module provides the fundamental types used across both markdown and HTML
//! isolation operations.

use std::borrow::Cow;
use thiserror::Error;

/// Specifies how isolated content should be returned.
///
/// When isolating content from markdown or HTML documents, you can either
/// receive results as individual pieces or concatenated into a single string.
///
/// ## Examples
///
/// ```
/// use shared::isolate::IsolateAction;
///
/// // Keep results as separate items
/// let action = IsolateAction::LeaveAsVector;
///
/// // Join results with newlines
/// let action = IsolateAction::Concatenate(Some("\n".to_string()));
///
/// // Join results with no separator
/// let action = IsolateAction::Concatenate(None);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IsolateAction {
    /// Return results as a vector of isolated content pieces.
    LeaveAsVector,
    /// Concatenate all results into a single string.
    ///
    /// The optional separator is inserted between each piece.
    /// If `None`, pieces are joined directly with no separator.
    Concatenate(Option<String>),
}

/// The result of an isolation operation.
///
/// Depending on the [`IsolateAction`] used, results are either returned as
/// individual pieces (potentially borrowing from the source) or as a single
/// concatenated string (always owned).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IsolateResult<'a> {
    /// Individual content pieces, potentially borrowing from the source document.
    Vector(Vec<Cow<'a, str>>),
    /// All content concatenated into a single owned string.
    Concatenated(String),
}

impl<'a> IsolateResult<'a> {
    /// Returns `true` if this is a `Vector` variant.
    pub fn is_vector(&self) -> bool {
        matches!(self, IsolateResult::Vector(_))
    }

    /// Returns `true` if this is a `Concatenated` variant.
    pub fn is_concatenated(&self) -> bool {
        matches!(self, IsolateResult::Concatenated(_))
    }

    /// Returns the number of items if `Vector`, or 1 if `Concatenated`.
    pub fn len(&self) -> usize {
        match self {
            IsolateResult::Vector(v) => v.len(),
            IsolateResult::Concatenated(_) => 1,
        }
    }

    /// Returns `true` if the result contains no content.
    pub fn is_empty(&self) -> bool {
        match self {
            IsolateResult::Vector(v) => v.is_empty(),
            IsolateResult::Concatenated(s) => s.is_empty(),
        }
    }

    /// Converts the result into an owned vector of strings.
    pub fn into_vec(self) -> Vec<String> {
        match self {
            IsolateResult::Vector(v) => v.into_iter().map(|c| c.into_owned()).collect(),
            IsolateResult::Concatenated(s) => vec![s],
        }
    }
}

/// Errors that can occur during content isolation.
#[derive(Error, Debug)]
pub enum IsolateError {
    /// The provided CSS selector is invalid.
    #[error("Invalid CSS selector: {0}")]
    InvalidSelector(String),

    /// Failed to parse the markdown document.
    #[error("Markdown parse error: {0}")]
    MarkdownParse(String),

    /// Failed to parse the HTML document.
    #[error("HTML parse error: {0}")]
    HtmlParse(String),

    /// A byte range did not align with UTF-8 character boundaries.
    #[error("Invalid byte range: {start}..{end} not on UTF-8 boundary")]
    InvalidByteRange {
        /// The start byte offset.
        start: usize,
        /// The end byte offset.
        end: usize,
    },
}

/// Errors that can occur during content interpolation.
#[derive(Error, Debug)]
pub enum InterpolateError {
    /// The provided regex pattern is invalid.
    #[error("Invalid regex pattern: {0}")]
    InvalidPattern(#[from] regex::Error),

    /// The underlying isolate operation failed.
    #[error("Isolate failed: {0}")]
    IsolateError(#[from] IsolateError),
}

/// Result type for isolate operations.
pub type IsolateResultType<T> = Result<T, IsolateError>;

/// Result type for interpolate operations.
pub type InterpolateResultType<T> = Result<T, InterpolateError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_isolate_action_variants() {
        let leave = IsolateAction::LeaveAsVector;
        let concat_none = IsolateAction::Concatenate(None);
        let concat_sep = IsolateAction::Concatenate(Some("\n".to_string()));

        assert_eq!(leave, IsolateAction::LeaveAsVector);
        assert_eq!(concat_none, IsolateAction::Concatenate(None));
        assert_ne!(concat_sep, concat_none);
    }

    #[test]
    fn test_isolate_result_vector() {
        let result: IsolateResult = IsolateResult::Vector(vec![
            Cow::Borrowed("hello"),
            Cow::Owned("world".to_string()),
        ]);

        assert!(result.is_vector());
        assert!(!result.is_concatenated());
        assert_eq!(result.len(), 2);
        assert!(!result.is_empty());
    }

    #[test]
    fn test_isolate_result_concatenated() {
        let result: IsolateResult = IsolateResult::Concatenated("hello world".to_string());

        assert!(!result.is_vector());
        assert!(result.is_concatenated());
        assert_eq!(result.len(), 1);
        assert!(!result.is_empty());
    }

    #[test]
    fn test_isolate_result_empty() {
        let empty_vec: IsolateResult = IsolateResult::Vector(vec![]);
        let empty_concat: IsolateResult = IsolateResult::Concatenated(String::new());

        assert!(empty_vec.is_empty());
        assert!(empty_concat.is_empty());
    }

    #[test]
    fn test_isolate_result_into_vec() {
        let vec_result: IsolateResult = IsolateResult::Vector(vec![
            Cow::Borrowed("a"),
            Cow::Borrowed("b"),
        ]);
        assert_eq!(vec_result.into_vec(), vec!["a", "b"]);

        let concat_result: IsolateResult = IsolateResult::Concatenated("combined".to_string());
        assert_eq!(concat_result.into_vec(), vec!["combined"]);
    }

    #[test]
    fn test_isolate_error_display() {
        let selector_err = IsolateError::InvalidSelector("div[".to_string());
        assert_eq!(selector_err.to_string(), "Invalid CSS selector: div[");

        let md_err = IsolateError::MarkdownParse("unclosed fence".to_string());
        assert_eq!(md_err.to_string(), "Markdown parse error: unclosed fence");

        let html_err = IsolateError::HtmlParse("malformed tag".to_string());
        assert_eq!(html_err.to_string(), "HTML parse error: malformed tag");

        let range_err = IsolateError::InvalidByteRange { start: 5, end: 10 };
        assert_eq!(
            range_err.to_string(),
            "Invalid byte range: 5..10 not on UTF-8 boundary"
        );
    }

    #[test]
    fn test_interpolate_error_from_isolate() {
        let isolate_err = IsolateError::MarkdownParse("test".to_string());
        let interpolate_err: InterpolateError = isolate_err.into();

        assert!(matches!(interpolate_err, InterpolateError::IsolateError(_)));
        assert!(interpolate_err.to_string().contains("Isolate failed"));
    }
}
