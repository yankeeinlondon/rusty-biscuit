//! Render result and error types shared across render targets.
//!
//! Successful renders return a [`Rendered`] carrying the output and any
//! non-fatal [`Diagnostic`]s. Fatal conditions are reported via [`RenderError`].
//! [`RenderStrictness`] selects how a renderer treats lossy or unsupported
//! content.

use thiserror::Error;

use crate::browser::feature::{FeatureResolveError, PageFeature};
use crate::tree::diagnostic::Diagnostic;
use crate::tree::validate::{ValidationError, ValidationFinding};

/// How strictly a renderer treats lossy or unsupported content.
///
/// The [`Default`] is [`RenderStrictness::Warn`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RenderStrictness {
    /// Reject lossy conversions and unsupported nodes with a [`RenderError`].
    Strict,
    /// Permit them but record a [`Diagnostic`] for each.
    #[default]
    Warn,
    /// Permit them silently, recording no diagnostics.
    Lossy,
}

/// A successful render plus any non-fatal diagnostics raised along the way.
///
/// [`features`](Rendered::features) is a second side channel, mirroring
/// [`diagnostics`](Rendered::diagnostics): it carries the browser
/// [`PageFeature`]s requested while rendering, in first-seen order, so the
/// outermost document assembler can resolve and inject their assets exactly
/// once. [`new`](Rendered::new) starts it empty and [`map`](Rendered::map)
/// preserves it.
#[derive(Debug, Clone, PartialEq)]
pub struct Rendered<T> {
    /// The rendered output.
    pub output: T,
    /// Non-fatal diagnostics recorded during the render.
    pub diagnostics: Vec<Diagnostic>,
    /// Browser features requested while rendering, in first-seen order.
    /// Non-browser render paths leave this empty.
    pub features: Vec<PageFeature>,
}

impl<T> Rendered<T> {
    /// Creates a [`Rendered`] with no diagnostics and no features.
    #[must_use]
    pub fn new(output: T) -> Self {
        Self {
            output,
            diagnostics: Vec::new(),
            features: Vec::new(),
        }
    }

    /// Applies `f` to the output, preserving the diagnostics and features.
    #[must_use]
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Rendered<U> {
        Rendered {
            output: f(self.output),
            diagnostics: self.diagnostics,
            features: self.features,
        }
    }
}

/// A fatal error raised while rendering a render tree.
///
/// This is the single shared error type for the `renderable` crate. Concrete
/// renderers in other crates wrap this in their own error types rather than
/// adding target-specific variants here.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum RenderError {
    /// The tree failed structural validation before rendering.
    #[error("render tree failed validation with {} error(s)", .findings.len())]
    InvalidTree {
        /// The error-severity validation findings.
        findings: Vec<ValidationFinding>,
    },
    /// Strict mode encountered an [`Unsupported`](crate::tree::NodeKind::Unsupported) node.
    #[error("unsupported content in strict mode: {label}")]
    Unsupported {
        /// The label describing the unsupported content.
        label: String,
    },
    /// Strict mode encountered a conversion that would discard information.
    #[error("lossy conversion rejected in strict mode: {message}")]
    LossyRejected {
        /// A description of the rejected conversion.
        message: String,
    },
    /// A requested page feature could not be resolved or placed. The wrapped
    /// [`FeatureResolveError`] names the feature and target.
    #[error(transparent)]
    FeatureResolution(#[from] FeatureResolveError),
}

impl From<ValidationError> for RenderError {
    fn from(error: ValidationError) -> Self {
        Self::InvalidTree {
            findings: error.findings,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::Severity;
    use crate::tree::validate::ValidationError;

    #[test]
    fn rendered_new_has_no_diagnostics() {
        let rendered = Rendered::new("output".to_string());
        assert_eq!(rendered.output, "output");
        assert!(rendered.diagnostics.is_empty());
    }

    #[test]
    fn rendered_map_preserves_diagnostics() {
        let mut rendered = Rendered::new(2);
        rendered
            .diagnostics
            .push(Diagnostic::lossy("approximated", None));
        let mapped = rendered.map(|n| n * 3);
        assert_eq!(mapped.output, 6);
        assert_eq!(mapped.diagnostics.len(), 1);
    }

    #[test]
    fn rendered_map_preserves_features() {
        let mut rendered = Rendered::new(2);
        rendered.features.push(PageFeature::Popover);
        let mapped = rendered.map(|n| n * 3);
        assert_eq!(mapped.output, 6);
        assert_eq!(mapped.features, vec![PageFeature::Popover]);
    }

    #[test]
    fn feature_resolve_error_converts_and_names_feature_and_target() {
        let source = FeatureResolveError::UnresolvedFeature {
            feature: PageFeature::MermaidDiagram,
            target: crate::target::RenderTarget::Browser,
        };
        let render_error: RenderError = source.into();
        let message = render_error.to_string();
        assert!(
            message.contains("mermaid-diagram"),
            "error must name the feature: {message}"
        );
        assert!(
            message.contains("Browser"),
            "error must name the target: {message}"
        );
    }

    #[test]
    fn validation_error_converts_to_render_error() {
        let error = ValidationError {
            findings: vec![ValidationFinding {
                severity: Severity::Error,
                message: "bad".into(),
                span: None,
            }],
        };
        let render_error: RenderError = error.into();
        assert!(matches!(
            render_error,
            RenderError::InvalidTree { findings } if findings.len() == 1
        ));
    }
}
