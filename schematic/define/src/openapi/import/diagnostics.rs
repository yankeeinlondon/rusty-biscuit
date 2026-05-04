//! Diagnostic types for OpenAPI import.
//!
//! This module provides types for tracking issues encountered during
//! OpenAPI import, categorized by severity.

/// Severity level for import diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DiagnosticSeverity {
    /// Informational message (e.g., alternate content types available).
    Info,
    /// Warning (e.g., unsupported feature that was skipped).
    Warn,
    /// Error (e.g., unresolvable reference).
    Error,
}

impl std::fmt::Display for DiagnosticSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiagnosticSeverity::Info => write!(f, "INFO"),
            DiagnosticSeverity::Warn => write!(f, "WARN"),
            DiagnosticSeverity::Error => write!(f, "ERROR"),
        }
    }
}

/// A diagnostic message from the OpenAPI import process.
///
/// Diagnostics capture issues, warnings, and informational messages
/// encountered during import. They include the severity, location
/// in the source document (as a JSON pointer), and a descriptive message.
///
/// ## Examples
///
/// ```
/// use schematic_define::openapi::import::{OpenApiDiagnostic, DiagnosticSeverity};
///
/// let diag = OpenApiDiagnostic::warn(
///     "#/paths/~1pets/get".to_string(),
///     "Missing operationId, using fallback name".to_string(),
/// );
///
/// assert_eq!(diag.severity, DiagnosticSeverity::Warn);
/// assert!(diag.message.contains("operationId"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenApiDiagnostic {
    /// Severity of the diagnostic.
    pub severity: DiagnosticSeverity,
    /// Location in the OpenAPI document (JSON pointer format).
    pub location: String,
    /// Human-readable description of the issue.
    pub message: String,
}

impl OpenApiDiagnostic {
    /// Creates an info-level diagnostic.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::openapi::import::OpenApiDiagnostic;
    ///
    /// let diag = OpenApiDiagnostic::info(
    ///     "#/paths/~1pets/get".to_string(),
    ///     "Also supports XML content type".to_string(),
    /// );
    /// ```
    pub fn info(location: String, message: String) -> Self {
        Self {
            severity: DiagnosticSeverity::Info,
            location,
            message,
        }
    }

    /// Creates a warning-level diagnostic.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::openapi::import::OpenApiDiagnostic;
    ///
    /// let diag = OpenApiDiagnostic::warn(
    ///     "#/components/schemas/User".to_string(),
    ///     "Circular reference detected, using Unknown type".to_string(),
    /// );
    /// ```
    pub fn warn(location: String, message: String) -> Self {
        Self {
            severity: DiagnosticSeverity::Warn,
            location,
            message,
        }
    }

    /// Creates an error-level diagnostic.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::openapi::import::OpenApiDiagnostic;
    ///
    /// let diag = OpenApiDiagnostic::error(
    ///     "#/components/schemas/Missing".to_string(),
    ///     "Referenced schema not found".to_string(),
    /// );
    /// ```
    pub fn error(location: String, message: String) -> Self {
        Self {
            severity: DiagnosticSeverity::Error,
            location,
            message,
        }
    }

    /// Returns true if this is an error-level diagnostic.
    pub fn is_error(&self) -> bool {
        self.severity == DiagnosticSeverity::Error
    }

    /// Returns true if this is a warning-level diagnostic.
    pub fn is_warn(&self) -> bool {
        self.severity == DiagnosticSeverity::Warn
    }

    /// Returns true if this is an info-level diagnostic.
    pub fn is_info(&self) -> bool {
        self.severity == DiagnosticSeverity::Info
    }
}

impl std::fmt::Display for OpenApiDiagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}: {}", self.severity, self.location, self.message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== DiagnosticSeverity Tests ==========

    #[test]
    fn severity_display() {
        assert_eq!(DiagnosticSeverity::Info.to_string(), "INFO");
        assert_eq!(DiagnosticSeverity::Warn.to_string(), "WARN");
        assert_eq!(DiagnosticSeverity::Error.to_string(), "ERROR");
    }

    #[test]
    fn severity_ordering() {
        assert!(DiagnosticSeverity::Info < DiagnosticSeverity::Warn);
        assert!(DiagnosticSeverity::Warn < DiagnosticSeverity::Error);
    }

    #[test]
    #[allow(clippy::clone_on_copy)] // Explicitly exercising Clone on a Copy type.
    fn severity_clone_eq() {
        let severity = DiagnosticSeverity::Warn;
        let cloned = severity.clone();
        assert_eq!(severity, cloned);
    }

    #[test]
    fn severity_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(DiagnosticSeverity::Info);
        set.insert(DiagnosticSeverity::Warn);
        set.insert(DiagnosticSeverity::Error);
        assert_eq!(set.len(), 3);
    }

    // ========== OpenApiDiagnostic Tests ==========

    #[test]
    fn diagnostic_info() {
        let diag = OpenApiDiagnostic::info("#/test".to_string(), "Info message".to_string());

        assert_eq!(diag.severity, DiagnosticSeverity::Info);
        assert_eq!(diag.location, "#/test");
        assert_eq!(diag.message, "Info message");
        assert!(diag.is_info());
        assert!(!diag.is_warn());
        assert!(!diag.is_error());
    }

    #[test]
    fn diagnostic_warn() {
        let diag = OpenApiDiagnostic::warn("#/test".to_string(), "Warning message".to_string());

        assert_eq!(diag.severity, DiagnosticSeverity::Warn);
        assert!(diag.is_warn());
    }

    #[test]
    fn diagnostic_error() {
        let diag = OpenApiDiagnostic::error("#/test".to_string(), "Error message".to_string());

        assert_eq!(diag.severity, DiagnosticSeverity::Error);
        assert!(diag.is_error());
    }

    #[test]
    fn diagnostic_display() {
        let diag = OpenApiDiagnostic::warn(
            "#/paths/~1pets".to_string(),
            "Missing operationId".to_string(),
        );

        let display = diag.to_string();
        assert!(display.contains("WARN"));
        assert!(display.contains("#/paths/~1pets"));
        assert!(display.contains("Missing operationId"));
    }

    #[test]
    fn diagnostic_clone_eq() {
        let diag = OpenApiDiagnostic::error("#/test".to_string(), "Test".to_string());
        let cloned = diag.clone();
        assert_eq!(diag, cloned);
    }

    #[test]
    fn diagnostic_debug() {
        let diag = OpenApiDiagnostic::info("#/test".to_string(), "Test".to_string());
        let debug = format!("{:?}", diag);
        assert!(debug.contains("OpenApiDiagnostic"));
        assert!(debug.contains("Info"));
    }
}
