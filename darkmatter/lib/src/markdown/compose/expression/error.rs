//! Typed expression-evaluation errors (Layer A of the "real errors" design).
//!
//! This is the typed substrate that replaces the stringly-typed
//! `Result<Value, String>` boundary in the expression engine. It is introduced
//! per the integrated design
//! (`claudine/features/2026-06-28-real-errors/integrated-design.md` §3–§5): a
//! flat [`ExpressionError`] (the *what*) plus a reusable
//! [`FileReferenceDiagnostic`] shared by every filesystem builtin
//! (`frontmatter`, `absolute`, `relative`, `load_markdown`, …).
//!
//! ## Why typed
//!
//! The same file-resolution failure is `format!`-ed identically at six call
//! sites today, and the resulting string is *also* string-prefix-matched to
//! drive a control-flow decision (`is_fatal_eval_error`). Typing the cause lets
//! that decision become a checked `match` ([`ExpressionError::is_authoring_fatal`])
//! and lets the renderer recover the real cause instead of the mechanism that
//! surfaced it.
//!
//! ## Scope of this layer
//!
//! - Filesystem builtins carry [`FileReferenceDiagnostic`] via
//!   [`ExpressionError::FileReference`].
//! - An unrecognized function name is [`ExpressionError::UnknownFunction`] — the
//!   sole authoring-fatal variant in lenient mode.
//! - Wrong argument count / type are [`ExpressionError::Arity`] /
//!   [`ExpressionError::ArgType`].
//! - The recursive-descent parser stays string-typed behind
//!   [`ExpressionError::Parse`].
//! - The long tail of pure builtins not yet individually classified is carried
//!   by [`ExpressionError::Other`], which always keeps the function name so it is
//!   never *less* informative than today's string.

use std::path::PathBuf;
use std::sync::Arc;

/// Which kind of file-reference resolution failure occurred.
///
/// Captures the distinction the engine already makes but throws away today:
/// `resolve_arg` distinguishes a malformed reference (`Err` from
/// `FileReference::new`) from a well-formed-but-absent one (`Ok(None)`), and the
/// remote-URL builtins distinguish "no remote runtime configured" from a plain
/// miss. `FoundElsewhere` is not produced from a [`biscuit_file::FileReferenceError`]
/// alone — it is determined at render time by the sibling-candidate search
/// (design §11 / Phase 4) and constructed explicitly there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileRefFailure {
    /// The reference string itself is invalid (e.g. empty, bad syntax).
    Malformed,
    /// The reference is well-formed but resolves to nothing on disk.
    NotFound,
    /// The file was not found where referenced but exists at a nearby location.
    FoundElsewhere,
    /// A remote URL reference was used where no remote runtime is enabled.
    RemoteNotEnabled,
}

impl FileRefFailure {
    /// Classifies a [`biscuit_file::FileReferenceError`] into a failure kind.
    ///
    /// A syntactically invalid reference is [`Malformed`]; a remote URL that
    /// cannot resolve to a local path is [`RemoteNotEnabled`]; everything else
    /// (filesystem I/O, relative-path computation, missing env/git/workspace
    /// state, and — when present — an invalid-URL parse) is treated as
    /// [`NotFound`] via the catch-all: the reference was understood but did not
    /// yield a usable path.
    ///
    /// [`Malformed`]: FileRefFailure::Malformed
    /// [`RemoteNotEnabled`]: FileRefFailure::RemoteNotEnabled
    /// [`NotFound`]: FileRefFailure::NotFound
    pub fn classify(error: &biscuit_file::FileReferenceError) -> Self {
        use biscuit_file::FileReferenceError as E;
        match error {
            E::InvalidSyntax(_) => FileRefFailure::Malformed,
            E::RemoteNotLocal(_) => FileRefFailure::RemoteNotEnabled,
            _ => FileRefFailure::NotFound,
        }
    }

    /// The catalog snake_case slug for this failure kind.
    ///
    /// This is the wire form the `composition.invalid_file_reference` `detail`
    /// payload serializes `kind` as (`err.detail.kind == "not_found"`), distinct
    /// from the `Debug` rendering (`"NotFound"`). Locked by error-catalog §2.7.
    pub fn as_str(&self) -> &'static str {
        match self {
            FileRefFailure::Malformed => "malformed",
            FileRefFailure::NotFound => "not_found",
            FileRefFailure::FoundElsewhere => "found_elsewhere",
            FileRefFailure::RemoteNotEnabled => "remote_not_enabled",
        }
    }
}

/// A reusable diagnostic for a file-reference resolution failure.
///
/// Shared by every filesystem builtin so they all inherit the same headline,
/// OSC8-linked paths, and did-you-mean suggestions (design §3). The fields are
/// exactly what `resolve_arg` already has in scope — `base_dir` and
/// `fallback_dir` come straight from the [`super::ResolutionContext`] — so the
/// typed variant simply *keeps what it already has* instead of `format!`-ing it
/// away.
///
/// ## Notes
///
/// `source` is optional because the [`FileRefFailure::NotFound`] case (a
/// well-formed reference that resolves to nothing — `resolve_arg` returns
/// `Ok(None)`) has no underlying [`biscuit_file::FileReferenceError`] to carry.
/// It is wrapped in [`Arc`] because [`biscuit_file::FileReferenceError`] is not
/// [`Clone`] (it holds a [`std::io::Error`]), while this struct — and the
/// `EvalResult` that will carry it — must be.
#[derive(Debug, Clone)]
pub struct FileReferenceDiagnostic {
    /// The builtin that produced the failure (`"frontmatter"`, `"absolute"`, …).
    pub function: &'static str,
    /// The raw reference argument, e.g. `"features/…/spec.md"`.
    pub reference: String,
    /// The kind of failure (absent vs malformed vs remote-not-enabled).
    pub kind: FileRefFailure,
    /// The document-relative base directory resolution started from.
    pub base_dir: PathBuf,
    /// Launch-area metadata retained by the resolution diagnostic.
    pub fallback_dir: Option<PathBuf>,
    /// The underlying typed cause, when one exists (absent for a clean miss).
    pub source: Option<Arc<biscuit_file::FileReferenceError>>,
}

/// The focused classification of a remote-provider query failure.
///
/// These kinds mirror the distinctions the provider layer already makes in
/// [`sniff::SniffError`] (spec AC27): not-found, denied host, authentication,
/// rate limit, unsupported capability, incomplete domain, and transport
/// failure. Carrying the classification as data — rather than only in the
/// message text — is what lets the memoization layer preserve it and lets
/// every expression surface apply one fatality rule to provider failures
/// without widening the generic [`ExpressionError::Other`] catch-all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderFailureKind {
    /// The addressed item does not exist on the provider (a genuine 404, or
    /// a shorthand no provider could resolve).
    NotFound,
    /// The provider host was denied by the run's network policy before any
    /// request was issued.
    DeniedHost,
    /// Credentials are missing, were rejected by the provider, or the
    /// provider denied the authenticated operation.
    Authentication,
    /// The provider rate-limited the request.
    RateLimit,
    /// The provider or selected API flavor cannot honor the requested
    /// operation or canonical filter.
    UnsupportedCapability,
    /// A bounded traversal stopped before the provider's result domain was
    /// exhausted, so no complete answer exists.
    IncompleteDomain,
    /// The endpoint could not be reached, its response could not be used, or
    /// the provider client could not be constructed.
    Transport,
}

impl ProviderFailureKind {
    /// The stable snake_case slug for this failure kind.
    pub fn as_str(&self) -> &'static str {
        match self {
            ProviderFailureKind::NotFound => "not_found",
            ProviderFailureKind::DeniedHost => "denied_host",
            ProviderFailureKind::Authentication => "authentication",
            ProviderFailureKind::RateLimit => "rate_limit",
            ProviderFailureKind::UnsupportedCapability => "unsupported_capability",
            ProviderFailureKind::IncompleteDomain => "incomplete_domain",
            ProviderFailureKind::Transport => "transport",
        }
    }
}

/// The expected argument count for an [`ExpressionError::Arity`] error.
///
/// Models the three arity shapes the builtins express today: an exact count
/// (`"requires 1 argument"`), an open lower bound (`"requires at least 1
/// argument"`), and an inclusive range (`"requires 1 or 2 arguments"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArityBound {
    /// Exactly `n` arguments.
    Exact(usize),
    /// At least `n` arguments.
    AtLeast(usize),
    /// Between `min` and `max` arguments, inclusive.
    Range(usize, usize),
}

impl std::fmt::Display for ArityBound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn plural(n: usize) -> &'static str {
            if n == 1 {
                "argument"
            } else {
                "arguments"
            }
        }
        match self {
            ArityBound::Exact(n) => write!(f, "{n} {}", plural(*n)),
            ArityBound::AtLeast(n) => write!(f, "at least {n} {}", plural(*n)),
            ArityBound::Range(min, max) => write!(f, "{min} or {max} arguments"),
        }
    }
}

/// A typed expression-evaluation error (Layer A: the *what*).
///
/// Flat by design (§3): the where (frontmatter key vs body, on-disk vs
/// effective source) is added by the `MarkdownError::Interpolation` wrapper in a
/// later phase, never by reformatting this error's message.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ExpressionError {
    /// A filesystem builtin failed to resolve its file argument.
    #[error("{}", display_file_reference(.0))]
    FileReference(FileReferenceDiagnostic),

    /// An unrecognized function name. The sole authoring-fatal variant in
    /// lenient mode — it can never resolve, so it is surfaced rather than left
    /// to leak its literal `{{ … }}` downstream.
    #[error("Unknown function: {name}")]
    UnknownFunction {
        /// The unrecognized function name.
        name: String,
    },

    /// A builtin was called with the wrong number of arguments.
    #[error("{function}() requires {expected}, got {actual}")]
    Arity {
        /// The builtin's name.
        function: &'static str,
        /// The argument count the builtin expects.
        expected: ArityBound,
        /// The argument count actually supplied.
        actual: usize,
    },

    /// A builtin argument had the wrong type.
    #[error("{function}() argument {index}: expected {expected}, got {actual_type}")]
    ArgType {
        /// The builtin's name.
        function: &'static str,
        /// The zero-based index of the rejected argument.
        index: usize,
        /// The expected type domain (`"numeric"`, `"string"`, `"array"`).
        expected: &'static str,
        /// The type actually supplied.
        actual_type: &'static str,
    },

    /// A parse failure. The recursive-descent parser stays string-typed (§4); a
    /// parse error is rarely the author's actual confusion in the observed
    /// failures, so it is carried as last-mile text behind this single variant.
    #[error("parse error: {0}")]
    Parse(String),

    /// A binary arithmetic operator (`+`, `-`, `*`, `/`, `%`) received a
    /// non-numeric operand. Operators are not function calls, so they are modeled
    /// distinctly from [`Other`] — its `name():` framing would render the
    /// misleading `Addition():` for what is really the `+` operator.
    ///
    /// [`Other`]: ExpressionError::Other
    #[error("{op} requires numeric operands")]
    Arithmetic {
        /// The operator label (`"Addition"`, `"Subtraction"`, `"Multiplication"`,
        /// `"Division"`, `"Remainder"`).
        op: &'static str,
    },

    /// A focused remote-provider query failed (`pr`, `pr_list`, `cicd`,
    /// `cicd_list`, `branch_exists_on_remote`, `remote_vendor`).
    ///
    /// Kept distinct from [`Other`] so the failure classification survives
    /// the run-local memoization layer (which previously flattened every
    /// provider failure to text and rebuilt it as `Other`) and so the
    /// frontmatter/body/`$()` surfaces can apply one fatality rule to it.
    /// Provider failures are authoring-fatal on every surface: the spec
    /// forbids replacing them with empty values or demoting them to warnings
    /// that leave the unevaluated `{{ … }}` behind.
    ///
    /// [`Other`]: ExpressionError::Other
    #[error("{function}(): {message}")]
    Provider {
        /// The provider function's name.
        function: String,
        /// The focused failure classification.
        kind: ProviderFailureKind,
        /// The actionable provider detail.
        message: String,
    },

    /// Migration catch-all for the long tail of pure builtins not yet
    /// individually classified. Always carries the function name, so it is never
    /// *less* informative than today's string.
    #[error("{function}(): {message}")]
    Other {
        /// The builtin's name.
        function: String,
        /// The builtin's raw error message.
        message: String,
    },
}

fn display_file_reference(diagnostic: &FileReferenceDiagnostic) -> String {
    match (diagnostic.kind, diagnostic.source.as_ref()) {
        (FileRefFailure::RemoteNotEnabled, None) => {
            format!(
                "{}() remote reads are not enabled for {:?}",
                diagnostic.function, diagnostic.reference
            )
        }
        _ => format!("invalid file path: {}", diagnostic.reference),
    }
}

impl ExpressionError {
    /// Whether this error halts lenient (non-`fail_fast`) composition.
    ///
    /// This is the checked-`match` replacement for the string-prefix
    /// `is_fatal_eval_error` gate (design §5). Three classes of failure are
    /// authoring-fatal even in lenient body interpolation:
    ///
    /// - [`UnknownFunction`] — an unknown symbol can never resolve, so it must be
    ///   surfaced rather than demoted to a warning that leaves the literal
    ///   `{{ … }}` in place.
    /// - A [`FileReference`] failure that is [`Malformed`], [`NotFound`], or
    ///   [`FoundElsewhere`] — a *present* file reference that fails to resolve is
    ///   almost always a real authoring mistake, not a tolerated absence. A
    ///   required property must resolve, and an optional property carrying a
    ///   reference must be either null/undefined or a *valid* reference. Authors
    ///   opt a reference out by guarding with `file_exists`/a ternary so no
    ///   resolution is attempted; a reference that is actually evaluated and
    ///   misses is surfaced rather than silently swallowed.
    ///
    /// - [`Provider`] — a focused provider failure (denied host, missing or
    ///   rejected credentials, rate limit, unsupported capability, incomplete
    ///   domain, transport failure, genuine not-found) is actionable state the
    ///   author must see; the spec requires the same fatal behavior on
    ///   frontmatter, body, and `$()` surfaces and forbids replacing it with an
    ///   empty value or an unevaluated `{{ … }}`.
    ///
    /// Every other variant (arity, arg-type, parse, arithmetic, generic
    /// [`Other`], …) is demoted to a `ComposeWarning` in lenient body
    /// interpolation. [`RemoteNotEnabled`] is
    /// also non-fatal here: it is a v1 capability gap governed by its own "remote
    /// not supported" policy, not an error in the reference itself.
    ///
    /// [`UnknownFunction`]: ExpressionError::UnknownFunction
    /// [`FileReference`]: ExpressionError::FileReference
    /// [`Provider`]: ExpressionError::Provider
    /// [`Other`]: ExpressionError::Other
    /// [`Malformed`]: FileRefFailure::Malformed
    /// [`NotFound`]: FileRefFailure::NotFound
    /// [`FoundElsewhere`]: FileRefFailure::FoundElsewhere
    /// [`RemoteNotEnabled`]: FileRefFailure::RemoteNotEnabled
    pub fn is_authoring_fatal(&self) -> bool {
        match self {
            ExpressionError::UnknownFunction { .. } => true,
            ExpressionError::Provider { .. } => true,
            // A present file reference that fails to resolve is fatal (see the
            // doc comment for the WHY). `RemoteNotEnabled` is deliberately *not*
            // in this set: it is a v1 capability gap, not a reference mistake.
            ExpressionError::FileReference(diagnostic) => matches!(
                diagnostic.kind,
                FileRefFailure::Malformed
                    | FileRefFailure::NotFound
                    | FileRefFailure::FoundElsewhere
            ),
            _ => false,
        }
    }

    /// Compatibility helper for existing tests that asserted against the old
    /// stringly evaluator error.
    pub fn contains(&self, needle: &str) -> bool {
        self.to_string().contains(needle)
    }

    /// Compatibility helper for existing tests that asserted against the old
    /// stringly evaluator error.
    pub fn starts_with(&self, needle: &str) -> bool {
        self.to_string().starts_with(needle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn io_error() -> std::io::Error {
        std::io::Error::new(std::io::ErrorKind::NotFound, "missing")
    }

    mod classify {
        use super::*;
        use biscuit_file::FileReferenceError;

        #[test]
        fn invalid_syntax_is_malformed() {
            let err = FileReferenceError::InvalidSyntax("bad".to_string());
            assert_eq!(FileRefFailure::classify(&err), FileRefFailure::Malformed);
        }

        #[test]
        fn remote_not_local_is_remote_not_enabled() {
            let err = FileReferenceError::RemoteNotLocal("https://x".to_string());
            assert_eq!(
                FileRefFailure::classify(&err),
                FileRefFailure::RemoteNotEnabled
            );
        }

        #[test]
        fn io_error_is_not_found() {
            let err = FileReferenceError::Io {
                path: PathBuf::from("/tmp/x"),
                source: io_error(),
            };
            assert_eq!(FileRefFailure::classify(&err), FileRefFailure::NotFound);
        }

        #[test]
        fn relative_path_is_not_found() {
            let err = FileReferenceError::RelativePath {
                from: PathBuf::from("/a"),
                to: PathBuf::from("/b"),
            };
            assert_eq!(FileRefFailure::classify(&err), FileRefFailure::NotFound);
        }

        #[test]
        fn missing_env_var_is_not_found() {
            let err = FileReferenceError::MissingEnvironmentVariable {
                name: "HOME".to_string(),
            };
            assert_eq!(FileRefFailure::classify(&err), FileRefFailure::NotFound);
        }

        #[test]
        fn as_str_is_snake_case_not_debug() {
            // The detail-payload contract (error-catalog §2.7) locks the wire
            // form to snake_case, never the Debug rendering.
            assert_eq!(FileRefFailure::Malformed.as_str(), "malformed");
            assert_eq!(FileRefFailure::NotFound.as_str(), "not_found");
            assert_eq!(FileRefFailure::FoundElsewhere.as_str(), "found_elsewhere");
            assert_eq!(
                FileRefFailure::RemoteNotEnabled.as_str(),
                "remote_not_enabled"
            );
            assert_ne!(FileRefFailure::NotFound.as_str(), "NotFound");
        }
    }

    mod fatality {
        use super::*;

        #[test]
        fn unknown_function_is_authoring_fatal() {
            let err = ExpressionError::UnknownFunction {
                name: "lenght".to_string(),
            };
            assert!(err.is_authoring_fatal());
        }

        #[test]
        fn file_reference_not_found_is_authoring_fatal() {
            // Ratified (real-errors finding #1): a present-but-missing reference
            // halts composition rather than warning-and-leaving the `{{ … }}`.
            let err = ExpressionError::FileReference(FileReferenceDiagnostic {
                function: "frontmatter",
                reference: "does-not-exist.md".to_string(),
                kind: FileRefFailure::NotFound,
                base_dir: PathBuf::from("/repo"),
                fallback_dir: None,
                source: None,
            });
            assert!(err.is_authoring_fatal());
        }

        #[test]
        fn remote_not_enabled_is_not_authoring_fatal() {
            // The deliberate exclusion: a remote reference in a local-only run is a
            // v1 capability gap, not an authoring mistake in the reference, so it
            // stays a lenient warning.
            let err = ExpressionError::FileReference(FileReferenceDiagnostic {
                function: "frontmatter",
                reference: "https://example.com/spec.md".to_string(),
                kind: FileRefFailure::RemoteNotEnabled,
                base_dir: PathBuf::from("/repo"),
                fallback_dir: None,
                source: None,
            });
            assert!(!err.is_authoring_fatal());
        }

        #[test]
        fn arity_is_not_authoring_fatal() {
            let err = ExpressionError::Arity {
                function: "length",
                expected: ArityBound::Exact(1),
                actual: 0,
            };
            assert!(!err.is_authoring_fatal());
        }

        #[test]
        fn arg_type_is_not_authoring_fatal() {
            let err = ExpressionError::ArgType {
                function: "min",
                index: 0,
                expected: "numeric",
                actual_type: "string",
            };
            assert!(!err.is_authoring_fatal());
        }

        #[test]
        fn parse_is_not_authoring_fatal() {
            let err = ExpressionError::Parse("unexpected token".to_string());
            assert!(!err.is_authoring_fatal());
        }

        #[test]
        fn arithmetic_is_not_authoring_fatal() {
            let err = ExpressionError::Arithmetic { op: "Addition" };
            assert!(!err.is_authoring_fatal());
        }

        #[test]
        fn other_is_not_authoring_fatal() {
            let err = ExpressionError::Other {
                function: "round".to_string(),
                message: "boom".to_string(),
            };
            assert!(!err.is_authoring_fatal());
        }

        #[test]
        fn provider_is_authoring_fatal() {
            // Every focused provider failure kind aborts lenient composition;
            // the classification — not the message text — drives the verdict.
            for kind in [
                ProviderFailureKind::NotFound,
                ProviderFailureKind::DeniedHost,
                ProviderFailureKind::Authentication,
                ProviderFailureKind::RateLimit,
                ProviderFailureKind::UnsupportedCapability,
                ProviderFailureKind::IncompleteDomain,
                ProviderFailureKind::Transport,
            ] {
                let err = ExpressionError::Provider {
                    function: "pr".to_string(),
                    kind,
                    message: "boom".to_string(),
                };
                assert!(err.is_authoring_fatal(), "{kind:?} must be fatal");
            }
        }
    }

    mod display {
        use super::*;

        // These assert the *fragments* the Phase 1 characterization matrix keys
        // on (`.contains`), so that when this typed substrate is wired into the
        // dispatch boundary the matrix verdicts stay green.

        #[test]
        fn file_reference_display_contains_invalid_file_path() {
            let err = ExpressionError::FileReference(FileReferenceDiagnostic {
                function: "frontmatter",
                reference: "features/x/spec.md".to_string(),
                kind: FileRefFailure::NotFound,
                base_dir: PathBuf::from("/repo"),
                fallback_dir: None,
                source: None,
            });
            let text = err.to_string();
            assert!(text.contains("invalid file path"), "{text}");
            assert!(text.contains("features/x/spec.md"), "{text}");
        }

        #[test]
        fn unknown_function_display_matches_prefix() {
            let err = ExpressionError::UnknownFunction {
                name: "unknown_fn".to_string(),
            };
            assert_eq!(err.to_string(), "Unknown function: unknown_fn");
        }

        #[test]
        fn arity_display_contains_requires() {
            let exact = ExpressionError::Arity {
                function: "length",
                expected: ArityBound::Exact(1),
                actual: 0,
            };
            assert_eq!(exact.to_string(), "length() requires 1 argument, got 0");

            let at_least = ExpressionError::Arity {
                function: "min",
                expected: ArityBound::AtLeast(1),
                actual: 0,
            };
            assert!(at_least.to_string().contains("requires at least 1 argument"));

            let range = ExpressionError::Arity {
                function: "number",
                expected: ArityBound::Range(1, 2),
                actual: 0,
            };
            assert!(range.to_string().contains("requires 1 or 2 arguments"));
        }

        #[test]
        fn arg_type_display_names_domain() {
            let err = ExpressionError::ArgType {
                function: "min",
                index: 0,
                expected: "numeric",
                actual_type: "string",
            };
            assert!(err.to_string().contains("numeric"), "{err}");
        }

        #[test]
        fn parse_display_contains_parse() {
            let err = ExpressionError::Parse("unexpected token".to_string());
            assert!(err.to_string().contains("parse"), "{err}");
        }

        #[test]
        fn arithmetic_display_has_no_function_parens() {
            let err = ExpressionError::Arithmetic { op: "Subtraction" };
            assert_eq!(err.to_string(), "Subtraction requires numeric operands");
        }

        #[test]
        fn other_display_keeps_function_name() {
            let err = ExpressionError::Other {
                function: "round".to_string(),
                message: "boom".to_string(),
            };
            assert_eq!(err.to_string(), "round(): boom");
        }

        #[test]
        fn provider_display_matches_other_shape() {
            // The wire shape is deliberately identical to `Other` (`fn(): msg`)
            // so messages stay single-prefixed; only the typed `kind` differs.
            let err = ExpressionError::Provider {
                function: "pr".to_string(),
                kind: ProviderFailureKind::RateLimit,
                message: "rate limited by Gitea API".to_string(),
            };
            assert_eq!(err.to_string(), "pr(): rate limited by Gitea API");
        }

        #[test]
        fn provider_failure_kind_slugs_are_snake_case() {
            assert_eq!(ProviderFailureKind::NotFound.as_str(), "not_found");
            assert_eq!(ProviderFailureKind::DeniedHost.as_str(), "denied_host");
            assert_eq!(ProviderFailureKind::Authentication.as_str(), "authentication");
            assert_eq!(ProviderFailureKind::RateLimit.as_str(), "rate_limit");
            assert_eq!(
                ProviderFailureKind::UnsupportedCapability.as_str(),
                "unsupported_capability"
            );
            assert_eq!(
                ProviderFailureKind::IncompleteDomain.as_str(),
                "incomplete_domain"
            );
            assert_eq!(ProviderFailureKind::Transport.as_str(), "transport");
        }
    }
}
