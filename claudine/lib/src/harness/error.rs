//! Error types for the harness module.

use std::path::PathBuf;

use biscuit_terminal::components::status::StatusState;
use biscuit_terminal::components::status_block::StatusBlock;
use biscuit_terminal::errors::{BlockError, ErrorHeader, StatusBlockExt};
use biscuit_terminal::terminal::Terminal;
use darkmatter::markdown::compose::shell_expansion::ShellExpansionError;
use serde_json::{Value, json};

use crate::diagnostics::{Category, Diagnostic, Disposition, Origin, code_spec, null_detail_for};

/// Why a harness path reference could not be resolved to a usable file.
///
/// The three arms are the resolver's *own* distinctions, each taken at a
/// different stage: the reference is rejected before resolution
/// ([`EmptyReference`]), the anchor it needs is unavailable
/// ([`NoSourceParent`]), or resolution succeeded and the probe found nothing
/// ([`TargetMissing`]). Because the resolver draws these lines itself, they
/// project to `err.detail.failure` — unlike Darkmatter's `FileRefFailure`,
/// which folds permission and missing-context failures into `NotFound` and so
/// cannot honestly claim `no_match` (see the feature's `decisions.md` §D-5).
///
/// [`EmptyReference`]: PathResolutionFailure::EmptyReference
/// [`NoSourceParent`]: PathResolutionFailure::NoSourceParent
/// [`TargetMissing`]: PathResolutionFailure::TargetMissing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PathResolutionFailure {
    /// The reference was empty or whitespace-only.
    EmptyReference,
    /// The source document has no parent directory to anchor a relative
    /// reference against.
    NoSourceParent,
    /// The reference resolved, but nothing exists at the resolved path.
    TargetMissing,
}

impl PathResolutionFailure {
    /// Stable snake_case slug, drawn from the `failure` vocabulary the
    /// `composition.invalid_file_reference` catalog entry declares.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::EmptyReference => "invalid_syntax",
            Self::NoSourceParent => "missing_context",
            Self::TargetMissing => "no_match",
        }
    }
}

/// The `Display` tail for [`HarnessError::PathResolutionFailed`].
///
/// Kept out of the `#[error]` attribute because each arm reads a different
/// optional field.
fn path_resolution_detail(
    failure: PathResolutionFailure,
    source_path: Option<&PathBuf>,
    resolved: Option<&PathBuf>,
) -> String {
    match failure {
        PathResolutionFailure::EmptyReference => "path is empty".to_string(),
        PathResolutionFailure::NoSourceParent => match source_path {
            Some(path) => format!("source path \"{}\" has no parent directory", path.display()),
            None => "source path has no parent directory".to_string(),
        },
        PathResolutionFailure::TargetMissing => match resolved {
            Some(path) => format!("target does not exist: {}", path.display()),
            None => "target does not exist".to_string(),
        },
    }
}

/// Why a shell command's execution failed.
///
/// The four arms are the four failure modes of
/// [`crate::harness::shell::execute_approved_command`], each carrying the typed
/// error that stage raised. No arm takes `#[from]`: the arm *is* the stage, and
/// `Spawn` and `Wait` both hold an `io::Error` that cannot say which of the two
/// produced it, so every construction names its stage explicitly.
#[derive(Debug, thiserror::Error)]
pub enum ShellExecCause {
    /// The executable was not found on `PATH`.
    #[error(transparent)]
    Which(which::Error),

    /// The child process could not be spawned.
    #[error(transparent)]
    Spawn(std::io::Error),

    /// Waiting on the spawned child failed.
    #[error(transparent)]
    Wait(std::io::Error),

    /// The command exceeded its timeout and was killed.
    #[error(transparent)]
    Timeout(tokio::time::error::Elapsed),
}

/// All errors that can occur within the harness subsystem.
#[derive(Debug, thiserror::Error)]
pub enum HarnessError {
    // --- Parse / configuration ---
    /// Frontmatter property is structurally invalid or has an unexpected type.
    #[error("{source_path}: invalid `{property}` frontmatter property: {detail}")]
    InvalidFrontmatter {
        source_path: PathBuf,
        property: String,
        detail: String,
    },

    /// A timeout string could not be parsed.
    #[error("{source_path}: invalid timeout \"{raw}\": {detail}")]
    InvalidTimeout {
        source_path: PathBuf,
        raw: String,
        detail: String,
    },

    /// A shell command failed during execution.
    #[error("shell command execution failed: {detail}")]
    ShellCommandExecutionFailed {
        detail: String,
        /// Which stage failed, and the typed error it raised.
        #[source]
        source: ShellExecCause,
    },

    // --- Shell approval failures ---
    /// A shell command was denied by the approval system.
    #[error("shell command denied: {command}")]
    ShellCommandDenied { command: String },

    /// A shell command matched the blacklist.
    #[error("shell command blacklisted: \"{command}\" — {reason}")]
    ShellCommandBlacklisted { command: String, reason: String },

    // --- Path resolution ---
    /// A `@`-prefixed path requires a repo root, but none was available.
    #[error("repo root required to resolve path \"{path}\"")]
    RepoRootRequired { path: String },

    /// Path resolution failed for another reason.
    #[error(
        "path resolution failed for \"{raw}\": {}",
        path_resolution_detail(*failure, source_path.as_ref(), resolved.as_ref())
    )]
    PathResolutionFailed {
        /// The reference exactly as authored.
        raw: String,
        /// Which of the resolver's own distinctions was drawn.
        failure: PathResolutionFailure,
        /// The document the reference was anchored against, when known.
        source_path: Option<PathBuf>,
        /// The path the reference resolved to, when resolution got that far.
        resolved: Option<PathBuf>,
    },

    // --- Shell audit ---
    /// Failed to parse shell directives from source page during audit.
    #[error("shell audit parse error: {detail}")]
    ShellAuditParseError {
        detail: String,
        /// Darkmatter's structured directive-parse failure.
        ///
        /// Boxed, mirroring the otherwise-identical
        /// `CompositionError::ShellExpansionFailed`, and for the same reason:
        /// `ShellExpansionError` is ~160 bytes, and `CompositionError` holds a
        /// `HarnessError` unboxed, so an unboxed field here trips
        /// `clippy::result_large_err` on 376 call sites across both enums.
        ///
        /// The cost is the D-7 trap: a `Box<T>` publishes `Box<T>` to the cause
        /// chain, so `Error::source()` cannot be downcast to
        /// `ShellExpansionError` and a chain walk skips it. Recovery is by
        /// matching this variant on the concrete `HarnessError` instead. No
        /// discovery seam is harmed — `ShellExpansionError` is a Darkmatter
        /// type, not a registered Claudine `Diagnostic`, so `as_diagnostic`
        /// never needed to downcast it.
        #[source]
        source: Box<ShellExpansionError>,
    },
}

impl BlockError for HarnessError {
    fn status_block(&self, _term: &Terminal) -> StatusBlock {
        StatusBlock::new(StatusState::Error)
            .error_header(ErrorHeader::new("HarnessError", self.code()))
            .body(self.to_string())
    }
}

impl Diagnostic for HarnessError {
    fn code(&self) -> &'static str {
        match self {
            // Author-authored harness frontmatter (timeouts, typed properties).
            HarnessError::InvalidFrontmatter { .. } | HarnessError::InvalidTimeout { .. } => {
                "composition.lifecycle_invalid"
            }
            // Shell directives the author wrote in the prompt document.
            HarnessError::ShellCommandExecutionFailed { .. }
            | HarnessError::ShellCommandDenied { .. }
            | HarnessError::ShellCommandBlacklisted { .. }
            | HarnessError::ShellAuditParseError { .. } => "composition.shell_expansion",
            // A reference the author wrote could not be resolved. This is an
            // authoring mistake in the prompt document, not an environment
            // failure: the file the run wanted to read is named by a value the
            // author typed. Classifying it `io.read_failed` (`Category::Io` /
            // `Origin::Operator`) sent the reader to check the filesystem when
            // the fix is in their frontmatter.
            HarnessError::PathResolutionFailed { .. } => "composition.invalid_file_reference",
            // Still environmental: the run needed a repo root and the process
            // was launched outside one.
            HarnessError::RepoRootRequired { .. } => "io.read_failed",
        }
    }

    fn category(&self) -> Category {
        code_spec(self.code())
            .map(|spec| spec.category)
            .unwrap_or(Category::Composition)
    }

    fn disposition(&self) -> Disposition {
        code_spec(self.code())
            .map(|spec| spec.disposition)
            .unwrap_or(Disposition::Correctable)
    }

    fn origin(&self) -> Origin {
        code_spec(self.code())
            .map(|spec| spec.origin)
            .unwrap_or(Origin::Author)
    }

    fn detail(&self) -> Value {
        match self {
            HarnessError::InvalidFrontmatter {
                property, detail, ..
            } => json!({ "property": property, "message": detail }),
            HarnessError::InvalidTimeout { detail, .. } => {
                json!({ "property": "timeout", "message": detail })
            }
            HarnessError::ShellCommandExecutionFailed { detail, .. } => {
                json!({ "command": detail })
            }
            HarnessError::ShellCommandDenied { command }
            | HarnessError::ShellCommandBlacklisted { command, .. } => {
                json!({ "command": command })
            }
            HarnessError::ShellAuditParseError { detail, .. } => json!({ "command": detail }),
            HarnessError::RepoRootRequired { path } => json!({ "path": path }),
            // Seeded from the catalog so every declared key is present. Only
            // `reference`, `source_path`, and `failure` are things this
            // resolver actually knows; the rest stay `null` rather than being
            // invented (spec §D3). `failure` is populated from the typed
            // `PathResolutionFailure`, never back-derived from `kind` — which
            // is exactly why `kind` itself stays `null` here.
            HarnessError::PathResolutionFailed {
                raw,
                failure,
                source_path,
                ..
            } => {
                let mut base = null_detail_for("composition.invalid_file_reference");
                base["reference"] = json!(raw);
                base["failure"] = json!(failure.as_str());
                base["source_path"] =
                    json!(source_path.as_ref().map(|p| p.to_string_lossy().into_owned()));
                base
            }
        }
    }
}

#[cfg(test)]
mod source_chain_tests {
    use std::error::Error;

    use super::*;

    /// `HarnessError` had no `#[source]` on any variant before this: the enum
    /// carried no cause chain at all, so every lower-layer failure reaching it
    /// died at the `detail` string. These assert the chain now exists.
    ///
    /// `ShellExecCause` is `#[error(transparent)]`, so it *replaces* the stage's
    /// error in the chain rather than adding a link above it. Downcasting to the
    /// cause enum and matching its arm is the recovery path — and the arm is the
    /// part that matters, since `Spawn` and `Wait` are indistinguishable from
    /// their `io::Error` alone.
    #[test]
    fn shell_exec_failure_publishes_the_stage_that_failed() {
        let err = HarnessError::ShellCommandExecutionFailed {
            detail: "boom".to_owned(),
            source: ShellExecCause::Spawn(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "denied",
            )),
        };

        let published = (&err as &(dyn Error + 'static))
            .source()
            .expect("no source published");
        let recovered = published
            .downcast_ref::<ShellExecCause>()
            .expect("source is not a `ShellExecCause`");

        let ShellExecCause::Spawn(io) = recovered else {
            panic!("wrong stage: {recovered:?}");
        };
        assert_eq!(io.kind(), std::io::ErrorKind::PermissionDenied);
    }

    #[test]
    fn a_wait_failure_is_distinguishable_from_a_spawn_failure() {
        let err = HarnessError::ShellCommandExecutionFailed {
            detail: "boom".to_owned(),
            source: ShellExecCause::Wait(std::io::Error::other("wait")),
        };

        let recovered = (&err as &(dyn Error + 'static))
            .source()
            .and_then(|c| c.downcast_ref::<ShellExecCause>())
            .expect("source is a `ShellExecCause`");
        assert!(matches!(recovered, ShellExecCause::Wait(_)));
    }

    /// `Which` and `Timeout` were previously discarded outright (`|_|` and
    /// `Err(_)`), so retaining them is the sharper win of the two.
    #[test]
    fn a_path_lookup_failure_is_retained_rather_than_discarded() {
        let err = HarnessError::ShellCommandExecutionFailed {
            detail: "executable 'nope' not found in PATH".to_owned(),
            source: ShellExecCause::Which(which::Error::CannotFindBinaryPath),
        };

        let cause = (&err as &(dyn Error + 'static))
            .source()
            .and_then(|c| c.downcast_ref::<ShellExecCause>())
            .expect("source is a `ShellExecCause`");
        assert!(matches!(
            cause,
            ShellExecCause::Which(which::Error::CannotFindBinaryPath)
        ));
    }

    /// Darkmatter's typed parse error must survive in the value. It is
    /// deliberately *not* reachable by downcasting `Error::source()`: the field
    /// is a `Box`, which publishes `Box<ShellExpansionError>` to the chain (the
    /// D-7 trap), and the variant's rustdoc records why the box is forced. This
    /// pins both halves of that trade so neither drifts silently.
    #[test]
    fn a_shell_audit_parse_failure_retains_darkmatters_typed_error() {
        // The only construction site is `collect_auditable_commands`; drive the
        // real parser rather than fabricating a `ShellExpansionError`.
        let ctx = biscuit_terminal::errors::SourceContext::new(
            std::path::PathBuf::from("<t>"),
            std::path::PathBuf::from("<t>"),
            String::new(),
        );
        let Err(parse_error) = darkmatter::markdown::compose::shell_expansion::parse_directives(
            "::shell \"unterminated",
            ctx,
            0,
        ) else {
            panic!("fixture no longer produces a parse error");
        };

        let err = HarnessError::ShellAuditParseError {
            detail: parse_error.to_string(),
            source: Box::new(parse_error),
        };

        let HarnessError::ShellAuditParseError { detail, source } = &err else {
            unreachable!()
        };
        // The typed value is retained beside the prose, not replaced by it.
        assert_eq!(&source.to_string(), detail);

        let published = (&err as &(dyn Error + 'static))
            .source()
            .expect("a source is published");
        assert!(
            published.downcast_ref::<ShellExpansionError>().is_none(),
            "the box no longer hides the cause — if `ShellAuditParseError` was \
             unboxed, drop this assertion and assert reachability instead"
        );
        assert!(
            published.downcast_ref::<Box<ShellExpansionError>>().is_some(),
            "the chain publishes neither `ShellExpansionError` nor its box"
        );
    }

    /// Adding the `#[source]` fields must leave `Display` and the machine
    /// surface exactly where they were (spec §D10).
    #[test]
    fn adding_a_source_leaves_display_and_detail_unmoved() {
        let err = HarnessError::ShellCommandExecutionFailed {
            detail: "failed to spawn 'ls': denied".to_owned(),
            source: ShellExecCause::Spawn(std::io::Error::other("denied")),
        };

        assert_eq!(
            err.to_string(),
            "shell command execution failed: failed to spawn 'ls': denied"
        );
        assert_eq!(err.code(), "composition.shell_expansion");
        assert_eq!(err.detail()["command"], json!("failed to spawn 'ls': denied"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_denied_classifies_as_shell_expansion() {
        let err = HarnessError::ShellCommandDenied {
            command: "rm -rf /".to_string(),
        };
        assert_eq!(err.code(), "composition.shell_expansion");
        assert_eq!(err.category(), Category::Composition);
        assert_eq!(err.origin(), Origin::Author);
        assert_eq!(err.detail()["command"], json!("rm -rf /"));
    }

    #[test]
    fn invalid_frontmatter_classifies_as_lifecycle_invalid() {
        let err = HarnessError::InvalidFrontmatter {
            source_path: PathBuf::from("p.md"),
            property: "timeout".to_string(),
            detail: "must be a string".to_string(),
        };
        assert_eq!(err.code(), "composition.lifecycle_invalid");
        assert_eq!(err.detail()["property"], json!("timeout"));
        assert_eq!(err.detail()["message"], json!("must be a string"));
    }

    #[test]
    fn repo_root_required_classifies_as_io_read() {
        let err = HarnessError::RepoRootRequired {
            path: "@/x".to_string(),
        };
        assert_eq!(err.code(), "io.read_failed");
        assert_eq!(err.category(), Category::Io);
        assert_eq!(err.detail()["path"], json!("@/x"));
    }

    #[test]
    fn path_resolution_classifies_as_an_authoring_failure() {
        // Regression: this reported an authoring mistake as `io.read_failed`
        // (`Category::Io` / `Origin::Operator`), sending the reader to check
        // the filesystem when the fix was in their frontmatter.
        let err = HarnessError::PathResolutionFailed {
            raw: "nope.md".to_string(),
            failure: PathResolutionFailure::TargetMissing,
            source_path: Some(PathBuf::from("/repo/run.md")),
            resolved: Some(PathBuf::from("/repo/nope.md")),
        };
        assert_eq!(err.code(), "composition.invalid_file_reference");
        assert_eq!(err.category(), Category::Composition);
        assert_eq!(err.origin(), Origin::Author);
        assert_eq!(err.disposition(), Disposition::Correctable);
    }

    #[test]
    fn path_resolution_detail_projects_only_what_the_resolver_knows() {
        let err = HarnessError::PathResolutionFailed {
            raw: "nope.md".to_string(),
            failure: PathResolutionFailure::TargetMissing,
            source_path: Some(PathBuf::from("/repo/run.md")),
            resolved: Some(PathBuf::from("/repo/nope.md")),
        };
        let detail = err.detail();

        assert_eq!(detail["reference"], json!("nope.md"));
        assert_eq!(detail["failure"], json!("no_match"));
        assert_eq!(detail["source_path"], json!("/repo/run.md"));

        // Every declared key is present; the ones this resolver cannot supply
        // are `null` rather than invented (spec §D3). `kind` in particular is
        // not back-derived from `failure`.
        for field in [
            "kind",
            "base_dir",
            "suggestions",
            "fallback_dir",
            "property",
            "event",
            "repository_root",
            "candidates",
        ] {
            assert!(
                detail.get(field).is_some(),
                "declared field `{field}` is absent from the projection"
            );
            assert_eq!(detail[field], Value::Null, "`{field}` should be null");
        }
    }

    #[test]
    fn every_path_resolution_failure_projects_a_declared_failure_slug() {
        // `failure` is a closed vocabulary the catalog documents; a new arm
        // must pick from it rather than coining a slug.
        for failure in [
            PathResolutionFailure::EmptyReference,
            PathResolutionFailure::NoSourceParent,
            PathResolutionFailure::TargetMissing,
        ] {
            assert!(
                [
                    "invalid_syntax",
                    "missing_context",
                    "no_match",
                    "permission_io",
                    "unsupported_remote",
                ]
                .contains(&failure.as_str()),
                "`{failure:?}` projects undeclared slug `{}`",
                failure.as_str()
            );
        }
    }

    #[test]
    fn path_resolution_display_names_the_reference_and_the_reason() {
        let err = HarnessError::PathResolutionFailed {
            raw: "nope.md".to_string(),
            failure: PathResolutionFailure::TargetMissing,
            source_path: None,
            resolved: Some(PathBuf::from("/repo/nope.md")),
        };
        let text = err.to_string();
        assert!(text.contains("nope.md"), "{text}");
        assert!(text.contains("target does not exist"), "{text}");

        let empty = HarnessError::PathResolutionFailed {
            raw: "  ".to_string(),
            failure: PathResolutionFailure::EmptyReference,
            source_path: None,
            resolved: None,
        };
        assert!(empty.to_string().contains("path is empty"));
    }
}
