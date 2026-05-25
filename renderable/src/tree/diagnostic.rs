//! Diagnostics raised while building or rendering a render tree.
//!
//! A [`Diagnostic`] is a non-fatal observation: a renderer may emit several
//! while still producing output. Fatal conditions are modelled separately by
//! [`RenderError`](crate::tree::RenderError).

use serde::{Deserialize, Serialize};

use crate::tree::source::SourceSpan;

/// The severity of a [`Diagnostic`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Severity {
    /// A problem that prevents a faithful render.
    Error,
    /// A non-fatal concern, such as a lossy conversion.
    Warning,
}

/// A classification for a [`Diagnostic`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DiagnosticKind {
    /// Content that has no canonical representation for the target.
    Unsupported,
    /// A conversion that discards or approximates information.
    Lossy,
    /// A structural irregularity in the tree.
    Structural,
    /// A finding surfaced by tree validation.
    Validation,
}

/// A non-fatal observation raised while building or rendering a tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    /// The classification of the diagnostic.
    pub kind: DiagnosticKind,
    /// How serious the diagnostic is.
    pub severity: Severity,
    /// A human-readable description.
    pub message: String,
    /// The source location the diagnostic refers to, if known.
    pub span: Option<SourceSpan>,
}

impl Diagnostic {
    /// Creates an [`DiagnosticKind::Unsupported`] warning diagnostic.
    #[must_use]
    pub fn unsupported(message: impl Into<String>, span: Option<SourceSpan>) -> Self {
        Self {
            kind: DiagnosticKind::Unsupported,
            severity: Severity::Warning,
            message: message.into(),
            span,
        }
    }

    /// Creates a [`DiagnosticKind::Lossy`] warning diagnostic.
    #[must_use]
    pub fn lossy(message: impl Into<String>, span: Option<SourceSpan>) -> Self {
        Self {
            kind: DiagnosticKind::Lossy,
            severity: Severity::Warning,
            message: message.into(),
            span,
        }
    }

    /// Creates a [`DiagnosticKind::Structural`] error diagnostic.
    #[must_use]
    pub fn structural(message: impl Into<String>, span: Option<SourceSpan>) -> Self {
        Self {
            kind: DiagnosticKind::Structural,
            severity: Severity::Error,
            message: message.into(),
            span,
        }
    }

    /// Creates a [`DiagnosticKind::Validation`] diagnostic with the given severity.
    #[must_use]
    pub fn validation(
        severity: Severity,
        message: impl Into<String>,
        span: Option<SourceSpan>,
    ) -> Self {
        Self {
            kind: DiagnosticKind::Validation,
            severity,
            message: message.into(),
            span,
        }
    }

    /// Returns `true` if this diagnostic has [`Severity::Error`].
    #[must_use]
    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }
}
