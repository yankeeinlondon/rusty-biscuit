//! Darkmatter's lifecycle preflight errors, mapped onto their
//! [`CompositionError`] twins.
//!
//! The lifecycle shell walk moved to `darkmatter::lifecycle::preflight`;
//! Claudine's catalog, renderer, and tests stay keyed on the
//! `CompositionError` variants, so each [`PreflightError`] variant converts to
//! the same-named variant field by field, with the same `Display` and the same
//! `source` chain.

use darkmatter::lifecycle::preflight::PreflightError;

use super::CompositionError;

impl From<PreflightError> for CompositionError {
    fn from(error: PreflightError) -> Self {
        match error {
            PreflightError::LifecycleShellResolution { source_path, property, raw, message, source } => {
                Self::LifecycleShellResolution { source_path, property, raw, message, source }
            }
            PreflightError::PreFlightStateBuildFailed { source } => Self::PreFlightStateBuildFailed { source },
            PreflightError::ResolutionContext(source) => Self::ResolutionContext(source),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error as _;
    use std::path::PathBuf;

    use darkmatter::markdown::MarkdownError;
    use darkmatter::markdown::compose::ContextBuildError;
    use darkmatter::markdown::compose::context::merge::CtxMergeError;

    use super::*;

    /// One instance of every [`PreflightError`] variant, with the predicate
    /// its [`CompositionError`] twin must satisfy.
    #[allow(clippy::type_complexity)]
    fn every_variant() -> Vec<(PreflightError, fn(&CompositionError) -> bool)> {
        vec![
            (
                PreflightError::LifecycleShellResolution {
                    source_path: PathBuf::from("docs/prompt.md"),
                    property: "start[0].shell.command".to_string(),
                    raw: "echo {{ err.message }}".to_string(),
                    message: "late-binding reference `err` is not available in shell commands".to_string(),
                    source: Some(Box::new(MarkdownError::Transform("typed cause".to_string()))),
                },
                |e| matches!(e, CompositionError::LifecycleShellResolution { .. }),
            ),
            (
                PreflightError::PreFlightStateBuildFailed {
                    source: CtxMergeError::InvalidUserCtx { kind: "string".to_string() },
                },
                |e| matches!(e, CompositionError::PreFlightStateBuildFailed { .. }),
            ),
            (
                PreflightError::ResolutionContext(ContextBuildError::Discovery {
                    dir: PathBuf::from("repo"),
                    detail: "not readable".to_string(),
                }),
                |e| matches!(e, CompositionError::ResolutionContext(_)),
            ),
        ]
    }

    #[test]
    fn every_preflight_error_converts_to_its_same_named_twin_with_the_same_display_and_source() {
        let variants = every_variant();
        assert_eq!(variants.len(), 3, "one case per `PreflightError` variant");
        for (error, is_twin) in variants {
            let expected = error.to_string();
            let expected_source = error.source().map(ToString::to_string);
            let converted = CompositionError::from(error);
            assert!(is_twin(&converted), "{expected}: converted to {converted:?}");
            assert_eq!(converted.to_string(), expected);
            assert_eq!(converted.source().map(ToString::to_string), expected_source, "{expected}");
        }
    }
}
