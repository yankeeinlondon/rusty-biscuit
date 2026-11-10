//! Typed expression-evaluation errors (Layer A of the "real errors" design).
//!
//! This is the typed substrate that replaces the stringly-typed
//! `Result<Value, String>` boundary in the expression engine. It is introduced
//! per the integrated design
//! (`claudine/features/2026-06-28-real-errors/integrated-design.md` §3–§5): a
//! typed [`ExpressionError`] (the *what*) plus a reusable
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
//!   [`ExpressionError::FileReference`]; a failed fetch from an allowed host
//!   is [`ExpressionError::RemoteRead`], and a schema file `validate_schema`
//!   could not read is [`ExpressionError::SchemaFileRead`].
//! - An unrecognized function name is [`ExpressionError::UnknownFunction`] — the
//!   failure remains authoring-fatal through callback wrappers.
//! - A call that does not bind to its declared signatures, or a handler result
//!   that breaks its return type, is [`ExpressionError::Call`].
//! - The recursive-descent parser stays string-typed behind
//!   [`ExpressionError::Parse`].
//! - The long tail of pure builtins not yet individually classified is carried
//!   by [`ExpressionError::Other`], which always keeps the function name so it is
//!   never *less* informative than today's string.

use crate::markdown::compose::context::ContextGroup;
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
    /// The run's fetch policy does not allow the URL's host, so nothing was
    /// requested. Unlike [`RemoteNotEnabled`](Self::RemoteNotEnabled), no
    /// later pass of the same run can read it.
    RemoteDenied,
}

impl FileRefFailure {
    /// Classifies a [`biscuit_file::FileReferenceError`] into a failure kind.
    ///
    /// A reference that cannot name a local path as written is [`Malformed`]:
    /// invalid syntax, an unsupported scheme or `~user` home, or an absolute
    /// path that belongs to another operating system. A remote URL that cannot
    /// resolve to a local path is [`RemoteNotEnabled`]. Everything else
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
            E::InvalidSyntax(_)
            | E::UnsupportedScheme { .. }
            | E::UnsupportedUserHome(_)
            | E::ForeignAbsolutePath { .. } => FileRefFailure::Malformed,
            E::RemoteNotLocal(_) => FileRefFailure::RemoteNotEnabled,
            // A relative reference that leaves the tree root, and a document
            // outside its tree, are understood but yield no usable path, like
            // their repository counterparts. The typed cause stays in `source`.
            E::RelativeTreeEscape { .. }
            | E::CwdOutsideBaseDir { .. }
            | E::BaseDirNotRepositoryRoot { .. } => FileRefFailure::NotFound,
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
            FileRefFailure::RemoteDenied => "remote_denied",
        }
    }
}

/// A reusable diagnostic for a file-reference resolution failure.
///
/// Shared by every filesystem builtin so they all inherit the same headline,
/// OSC8-linked paths, and did-you-mean suggestions (design §3). The fields are
/// normally come from `resolve_arg` and its [`super::ResolutionContext`]. When
/// schema projection supplies the argument, `caller` retains its authoring
/// context and selected candidate without changing the semantic argument used
/// for evaluation.
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
    /// The document directory (`cwd`) resolution started from.
    pub cwd: PathBuf,
    /// Launch-area metadata retained by the resolution diagnostic.
    pub fallback_dir: Option<PathBuf>,
    /// The underlying typed cause, when one exists (absent for a clean miss).
    pub source: Option<Arc<biscuit_file::FileReferenceError>>,
    /// Caller provenance when the argument is a projected schema-selected value.
    pub caller: Option<Arc<CallerFileDiagnosticProvenance>>,
}

/// Immutable caller evidence retained for a later read-side file failure.
#[derive(Debug, Clone)]
pub struct CallerFileDiagnosticProvenance {
    /// Top-level caller property whose semantic value reached the builtin.
    pub property: String,
    /// Captured authoring context used to construct the selected candidate.
    pub origin: biscuit_file::FileResolutionContext,
    /// Candidate installed as the property's semantic value.
    pub candidate: PathBuf,
    /// Candidate root class selected by `FileReference`.
    pub candidate_provenance: biscuit_file::RootProvenance,
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

/// How a read-side function's fetch of a URL failed.
///
/// Only [`Transport`](Self::Transport) is a failure of the source itself, so
/// only it is recoverable by `try` (see
/// [`ExpressionError::is_recoverable_read_failure`]); the other kinds are
/// failures of the request or the host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteReadKind {
    /// The allowed host could not be reached, answered with an error status,
    /// sent a body that could not be read, or sent a body that is not UTF-8.
    Transport,
    /// The run's policy refused the request, for example a redirect to a
    /// host it does not allow.
    Policy,
    /// The URL cannot be fetched as written: an unsupported scheme or a URL
    /// that does not parse.
    InvalidReference,
    /// The remote runtime could not run the fetch: its HTTP client or
    /// runtime failed to start, or it lost a registered fetch.
    Infrastructure,
}

/// A typed expression-evaluation error (Layer A: the *what*).
///
/// Document locations are added by `MarkdownError::Interpolation`; callback
/// wrappers retain original item positions and a typed source chain.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ExpressionError {
    /// A filesystem builtin failed to resolve its file argument.
    #[error("{}", display_file_reference(.0))]
    FileReference(FileReferenceDiagnostic),

    /// A builtin that takes a glob reference (`find_files()`) could not
    /// parse or resolve it. The typed cause carries the pattern and its
    /// [`ResolutionFailure`](biscuit_file::ResolutionFailure) class.
    #[error("{function}(): {source}")]
    GlobReference {
        /// The builtin's name.
        function: &'static str,
        /// Why the glob reference failed.
        #[source]
        source: Arc<biscuit_file::GlobReferenceError>,
    },

    /// An unrecognized function name. The sole authoring-fatal variant in
    /// lenient mode — it can never resolve, so it is surfaced rather than left
    /// to leak its literal `{{ … }}` downstream.
    #[error("Unknown function: {name}")]
    UnknownFunction {
        /// The unrecognized function name.
        name: String,
    },

    /// A parse failure. The recursive-descent parser stays string-typed (§4); a
    /// parse error is rarely the author's actual confusion in the observed
    /// failures, so it is carried as last-mile text behind this single variant.
    #[error("parse error: {0}")]
    Parse(String),

    /// A callback pattern binds a protected name or violates callback syntax.
    #[error("invalid callback: {message}")]
    InvalidCallback {
        /// Located pattern or callback syntax in the expression text.
        span: crate::markdown::span::SourceSpan,
        /// The actionable rejection, retaining the shared binding rule.
        message: String,
    },

    /// A callback failed at an original source position. Each enclosing
    /// callback retains its own wrapper and the typed underlying cause.
    #[error("{function}[{position}]: {source}")]
    Callback {
        /// Registered collection function.
        function: String,
        /// Original zero-based array index or object key.
        position: super::iteration::IterationPosition,
        /// Pattern or body failure, before any later item executes.
        #[source]
        source: Box<ExpressionError>,
    },

    /// A collection allowance (visits or expansion bytes) was exhausted.
    ///
    /// Request-fatal on every surface: no handler, fallback, or recovery
    /// option converts it into a value.
    #[error("{0}")]
    IterationLimit(Box<super::iteration::IterationError>),

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

    /// A cataloged `ctx.*` variable whose owning capture group the composition
    /// request never captured. This is a caller or pipeline contract violation,
    /// not a fact about the host: a captured group that found nothing projects
    /// a typed `null`/empty value instead.
    #[error(
        "context variable `ctx.{key}` belongs to the {group:?} capture group, \
         which this composition request did not capture; capture the document's \
         `ContextRequirements` (e.g. `ComposeContext::capture_for_document`) or \
         build options with `ComposeOptions::new()`"
    )]
    ContextNotCaptured {
        /// The bare catalog key (`repo_root` for `ctx.repo_root`).
        key: String,
        /// The capture group that owns `key`.
        group: ContextGroup,
    },

    /// A schema slot's evaluation tried a read its approvals do not cover
    /// (see [`read_gate`](super::read_gate)). Only slot evaluation sets a
    /// gate, so document evaluation never raises it. Never recovered into a
    /// value: `try_frontmatter` and `||` pass it through.
    #[error("{target}: {refusal}")]
    SlotReadRefused {
        /// The read that was refused.
        target: Box<super::read_gate::ReadTarget>,
        /// Why.
        refusal: super::read_gate::GateRefusal,
    },

    /// A call that breaks a function's specified contract where the
    /// specification requires a compose error and the declared signature
    /// cannot express the rule (an argument out of a domain the type does
    /// not bound, or a required effect the host cannot perform). Unlike
    /// [`Other`](Self::Other) and a [`Call`](Self::Call) type error, it is never
    /// demoted to a warning or replaced by a fallback value on any surface.
    #[error("{function}(): {message}")]
    ContractViolation {
        /// The function's canonical name.
        function: String,
        /// What the call violated.
        message: String,
    },

    /// A function that reads a capture group's retained observations (such as
    /// `package()` or `ipv4()`) ran in a request that never captured that
    /// group. Like [`ContextNotCaptured`](Self::ContextNotCaptured), this is a
    /// request contract violation, not a fact about the host.
    #[error(
        "{function}() reads the {group:?} context capture group, which this \
         composition request did not capture; capture the document's \
         `ContextRequirements` (e.g. `ComposeContext::capture_for_document`) or \
         build options with `ComposeOptions::new()`"
    )]
    FunctionContextNotCaptured {
        /// The function's canonical name.
        function: String,
        /// The capture group whose observations the function reads.
        group: ContextGroup,
    },

    /// A captured group whose projection omitted one of its own cataloged keys.
    /// A Darkmatter internal invariant failure; the caller is not at fault.
    #[error(
        "internal error: the {group:?} context capture did not project cataloged \
         variable `ctx.{key}`; this is a Darkmatter bug"
    )]
    ContextProjectionInvariant {
        /// The bare catalog key missing from the projection.
        key: String,
        /// The captured group that should have projected `key`.
        group: ContextGroup,
    },

    /// `ctx.id` or `ctx.sid` was read, but the operating system could not
    /// supply the per-execution nonce they hash. Never replaced by a fixed or
    /// time-derived nonce, which would break per-execution uniqueness.
    #[error(
        "context variable `ctx.{key}` is unavailable: its per-execution random \
         nonce could not be drawn ({detail})"
    )]
    ExecutionNonceUnavailable {
        /// `id` or `sid`.
        key: String,
        /// The entropy source's failure.
        detail: String,
    },

    /// A path under a lazy reserved root that names no member of it. The root
    /// cannot be shadowed and has no authored fallback, so the reference can
    /// never resolve; it is surfaced instead of rendering empty. This is how the
    /// removed `current.ctx.*` / `current.env.*` nesting fails after the clean
    /// break (**R33**).
    #[error(
        "`{path}` is not a member of the reserved `{root}` root: `current` mirrors \
         the `ctx` variables by name, so there is no `current.ctx.*` or \
         `current.env.*` nesting"
    )]
    ReservedRootPathUnknown {
        /// The lazy root, `current`.
        root: &'static str,
        /// The full unresolvable path, root included.
        path: String,
    },

    /// A function call that did not bind to any of the function's declared
    /// signatures (type, arity, or ambiguous overload), or a handler result
    /// that breaks its declared return type (implementation contract). The
    /// handler is never entered for a call that did not bind.
    #[error("{0}")]
    Call(Box<super::call_binding::CallBindingError>),

    /// A call the design-time check proves wrong (category 5), found before
    /// any expression of the document was evaluated. Always fatal: the
    /// document is rejected whichever branch the call is in.
    #[error("{}", .0.rejection_message())]
    CallCheck(Box<super::call_check::CallCheck>),

    /// A binding failure: a read of a global the host declared unavailable in
    /// this scope, or an invalid host registration. Never resolved by falling
    /// through to a same-named document property.
    #[error(transparent)]
    Binding(Box<super::binding::BindingError>),

    /// A literal token (`{{!data:…}}`) that is malformed, or that is not an
    /// entire frontmatter string value. A token never falls back to
    /// expression parsing, so this is authoring-fatal on every surface.
    #[error("malformed literal token: {0}")]
    MalformedLiteralToken(crate::markdown::literal_token::TokenError),

    /// A `load_data` failure other than a file-reference one: the contract,
    /// the format, the document's syntax, or the data's fit to the contract.
    /// Authoring-fatal on every surface; shared through an `Arc` because the
    /// request memo hands one failure to every call site that reaches it.
    #[error("{0}")]
    Data(Arc<super::data::DataError>),

    /// A `load_data` failure raised inside a nested composition
    /// (`as_markdown`) and re-raised at its call. Displayed, graded, and
    /// projected exactly like [`Data`](Self::Data); kept distinct so a
    /// failure inside the child is never taken for a direct read failure of
    /// the calling expression.
    #[error("{0}")]
    NestedData(Arc<super::data::DataError>),

    /// A read-side function's fetch from an allowed host failed: the request
    /// failed (a network error, an HTTP error status) or the response was not
    /// UTF-8 text. Worded and graded like [`Other`](Self::Other), so it is not
    /// authoring-fatal and renders the same; it keeps the reference as
    /// written so the report can fill its reference fields. A host the policy
    /// refuses is a [`FileReference`](Self::FileReference) failure instead.
    #[error("{function}(): {message}")]
    RemoteRead {
        /// The read-side function's name.
        function: &'static str,
        /// The URL as written (or as the calling schema property spelled it).
        reference: String,
        /// How the fetch failed.
        kind: RemoteReadKind,
        /// The remote runtime's description of the failure.
        message: String,
    },

    /// A schema file a read-side function needed beyond its argument could
    /// not be read: `validate_schema`'s `$schema` target or a named-type
    /// import is missing, malformed, or unreadable. Worded and graded like
    /// [`Other`](Self::Other), so it is not authoring-fatal and renders the
    /// same; `file` keeps the typed cause so the report can fill its
    /// reference fields. A schema that reads but is invalid stays `Other`.
    #[error("{}(): {message}", file.function)]
    SchemaFileRead {
        /// The same message the function's `Other` failure carries.
        message: String,
        /// The schema file reference that could not be read.
        file: Box<FileReferenceDiagnostic>,
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
        // Worded as the remote runtime's policy refusal, which this failure
        // replaced as untyped text.
        (FileRefFailure::RemoteDenied, _) => format!(
            "{}(): remote read denied for host '{}'",
            diagnostic.function,
            url::Url::parse(&diagnostic.reference)
                .ok()
                .and_then(|url| url.host_str().map(str::to_string))
                .unwrap_or_default()
        ),
        _ => crate::markdown::errors::with_hint_line(
            format!("invalid file path: {}", diagnostic.reference),
            diagnostic.glob_hint(),
        ),
    }
}

impl FileReferenceDiagnostic {
    /// The biscuit-file class of this failure: its typed cause's class when
    /// it has one, otherwise the class its kind implies (a clean miss is
    /// [`NoMatch`](biscuit_file::ResolutionFailure::NoMatch), a remote read
    /// that cannot run is
    /// [`UnsupportedRemote`](biscuit_file::ResolutionFailure::UnsupportedRemote)).
    pub fn resolution_failure(&self) -> biscuit_file::ResolutionFailure {
        use biscuit_file::ResolutionFailure;
        if let Some(source) = &self.source {
            return source.resolution_failure();
        }
        match self.kind {
            FileRefFailure::Malformed => ResolutionFailure::InvalidReference,
            FileRefFailure::NotFound | FileRefFailure::FoundElsewhere => ResolutionFailure::NoMatch,
            FileRefFailure::RemoteNotEnabled | FileRefFailure::RemoteDenied => ResolutionFailure::UnsupportedRemote,
        }
    }

    /// The literal-glob hint for a clean miss (a [`FileRefFailure::NotFound`]
    /// with no underlying error) of a reference whose text looks like a glob;
    /// see [`biscuit_file::ResolutionFailure::glob_hint`].
    pub fn glob_hint(&self) -> Option<&'static str> {
        match (self.kind, &self.source) {
            (FileRefFailure::NotFound, None) => {
                biscuit_file::ResolutionFailure::NoMatch.glob_hint(&self.reference)
            }
            _ => None,
        }
    }

    /// The file-reference failure of a schema file `function` could not
    /// read — a `$schema` target or named-type import that is missing,
    /// malformed, or unreadable — resolved from `cwd`; any other schema
    /// failure is returned unchanged.
    ///
    /// The reference is the one the schema spells (the path, for an I/O
    /// failure). Only this level of `error` is inspected.
    pub(crate) fn from_schema_read(
        function: &'static str,
        cwd: PathBuf,
        error: simplified_schema::SchemaError,
    ) -> Result<Self, simplified_schema::SchemaError> {
        use simplified_schema::SchemaError;
        let (reference, kind, source) = match error {
            SchemaError::Unresolved { reference, source } => (reference, FileRefFailure::classify(&source), Some(source)),
            SchemaError::NoMatch { reference, .. } => (reference, FileRefFailure::NotFound, None),
            SchemaError::Io { path, source } => (
                path.to_string_lossy().into_owned(),
                FileRefFailure::NotFound,
                Some(biscuit_file::FileReferenceError::Io { path, source }),
            ),
            other => return Err(other),
        };
        Ok(Self { function, reference, kind, cwd, fallback_dir: None, source: source.map(Arc::new), caller: None })
    }
}

impl ExpressionError {
    /// Whether this error halts lenient (non-`fail_fast`) composition.
    ///
    /// This is the checked-`match` replacement for the string-prefix
    /// `is_fatal_eval_error` gate (design §5). These classes of failure are
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
    /// - [`GlobReference`] — a glob reference that cannot be parsed or rooted
    ///   is an authoring mistake for the same reason, and one whose search
    ///   cannot read a directory it must enter would otherwise be a silently
    ///   shorter list.
    ///
    /// - [`Provider`] — a focused provider failure (denied host, missing or
    ///   rejected credentials, rate limit, unsupported capability, incomplete
    ///   domain, transport failure, genuine not-found) is actionable state the
    ///   author must see; the spec requires the same fatal behavior on
    ///   frontmatter, body, and `$()` surfaces and forbids replacing it with an
    ///   empty value or an unevaluated `{{ … }}`.
    ///
    /// - [`ContractViolation`] — the function's specification makes the call a
    ///   compose error in every position.
    ///
    /// - [`ContextNotCaptured`], [`FunctionContextNotCaptured`], and
    ///   [`ContextProjectionInvariant`] — a known `ctx.*` variable or
    ///   observation-reading function with no captured group is a request or
    ///   Darkmatter defect; rendering it as an empty value would hide it.
    ///
    /// - [`ReservedRootPathUnknown`] — a lazy reserved root cannot be shadowed
    ///   and has no authored fallback, so the reference can never resolve. Like
    ///   an unknown function, it is surfaced rather than rendered empty.
    ///
    /// - [`MalformedLiteralToken`] — a token never falls back to expression
    ///   parsing or to text, so a lenient caller cannot keep it either.
    ///
    /// - [`Binding`] — an unavailable global must never render as an empty
    ///   value or a same-named document property.
    ///
    /// - [`Call`] of kind `ImplementationContract` — a handler returned a value
    ///   its declared return type rejects, a Darkmatter defect. A call that does
    ///   not bind (type, arity, ambiguous overload) is not fatal here, like the
    ///   arity and arg-type errors it replaces.
    ///
    /// - [`CallCheck`] — a call the design-time check proves wrong rejects the
    ///   document before evaluation, so no lenient surface may keep it.
    ///
    /// - [`Data`] and [`NestedData`] — a `load_data` contract, format,
    ///   syntax, or contract-fit failure. No partial value may stand in for
    ///   data that did not load. A contract import that cannot be read is
    ///   graded like the [`FileReference`] failure it was before it had its
    ///   own variant.
    ///
    /// Every other variant (arity, arg-type, parse, arithmetic, generic
    /// [`Other`], …) is demoted to a `ComposeWarning` in lenient body
    /// interpolation. [`RemoteNotEnabled`] is
    /// also non-fatal here: it is a v1 capability gap governed by its own "remote
    /// not supported" policy, not an error in the reference itself.
    ///
    /// [`UnknownFunction`]: ExpressionError::UnknownFunction
    /// [`FileReference`]: ExpressionError::FileReference
    /// [`GlobReference`]: ExpressionError::GlobReference
    /// [`Provider`]: ExpressionError::Provider
    /// [`ContractViolation`]: ExpressionError::ContractViolation
    /// [`ContextNotCaptured`]: ExpressionError::ContextNotCaptured
    /// [`FunctionContextNotCaptured`]: ExpressionError::FunctionContextNotCaptured
    /// [`ContextProjectionInvariant`]: ExpressionError::ContextProjectionInvariant
    /// [`ReservedRootPathUnknown`]: ExpressionError::ReservedRootPathUnknown
    /// [`MalformedLiteralToken`]: ExpressionError::MalformedLiteralToken
    /// [`Binding`]: ExpressionError::Binding
    /// [`Call`]: ExpressionError::Call
    /// [`CallCheck`]: ExpressionError::CallCheck
    /// [`Data`]: ExpressionError::Data
    /// [`NestedData`]: ExpressionError::NestedData
    /// [`Other`]: ExpressionError::Other
    /// [`Malformed`]: FileRefFailure::Malformed
    /// [`NotFound`]: FileRefFailure::NotFound
    /// [`FoundElsewhere`]: FileRefFailure::FoundElsewhere
    /// [`RemoteNotEnabled`]: FileRefFailure::RemoteNotEnabled
    pub fn is_authoring_fatal(&self) -> bool {
        match self {
            ExpressionError::Callback { source, .. } => source.is_authoring_fatal(),
            ExpressionError::UnknownFunction { .. } | ExpressionError::InvalidCallback { .. } => true,
            ExpressionError::IterationLimit(_) => true,
            ExpressionError::GlobReference { .. } => true,
            ExpressionError::Provider { .. } => true,
            ExpressionError::ContractViolation { .. } => true,
            ExpressionError::ReservedRootPathUnknown { .. } => true,
            ExpressionError::MalformedLiteralToken(_) => true,
            ExpressionError::Binding(_) => true,
            ExpressionError::CallCheck(_) => true,
            ExpressionError::Data(data) | ExpressionError::NestedData(data) => match &**data {
                super::data::DataError::ContractImport(diagnostic) => file_reference_is_fatal(diagnostic),
                _ => true,
            },
            // A broken return contract is a Darkmatter defect; a call that
            // does not bind keeps the old arity/arg-type severity.
            ExpressionError::Call(error) => {
                error.kind() == super::call_binding::CallErrorKind::ImplementationContract
            }
            ExpressionError::ContextNotCaptured { .. }
            | ExpressionError::FunctionContextNotCaptured { .. }
            | ExpressionError::ContextProjectionInvariant { .. }
            | ExpressionError::ExecutionNonceUnavailable { .. } => true,
            ExpressionError::FileReference(diagnostic) => file_reference_is_fatal(diagnostic),
            _ => false,
        }
    }

    /// Whether `try(expr, fallback)` may replace this failure of `expr` with
    /// its fallback: a typed failure to read a source that the request was
    /// allowed to read.
    ///
    /// The recoverable failures are:
    ///
    /// - a [`FileReference`] miss ([`NotFound`] or [`FoundElsewhere`]) that is
    ///   either clean (no typed cause) or an I/O failure reading the file;
    /// - a `load_data` source that does not parse ([`DataError::Malformed`]),
    ///   does not fit its contract ([`DataError::ContractMismatch`]), or
    ///   misses under the same file-reference rule;
    /// - a [`RemoteRead`] whose kind is [`RemoteReadKind::Transport`].
    ///
    /// Everything else is a failure of the request, the expression, or the
    /// host and propagates: a reference that leaves its tree or lacks an
    /// anchor (also labeled [`NotFound`], so the label alone proves nothing),
    /// a remote read that is not enabled or is denied, an unsupported
    /// format, invalid options, a contract or its import, a failure inside
    /// a nested composition ([`NestedData`]), a refused slot read, and every
    /// non-read variant. A callback wrapper is judged by its cause.
    ///
    /// The match is exhaustive on purpose: a new variant must be classified
    /// here before it compiles. This is not a weaker form of
    /// [`is_authoring_fatal`](Self::is_authoring_fatal): the two policies are
    /// independent.
    ///
    /// [`FileReference`]: ExpressionError::FileReference
    /// [`RemoteRead`]: ExpressionError::RemoteRead
    /// [`NestedData`]: ExpressionError::NestedData
    /// [`NotFound`]: FileRefFailure::NotFound
    /// [`FoundElsewhere`]: FileRefFailure::FoundElsewhere
    /// [`DataError::Malformed`]: super::data::DataError::Malformed
    /// [`DataError::ContractMismatch`]: super::data::DataError::ContractMismatch
    pub fn is_recoverable_read_failure(&self) -> bool {
        use super::data::DataError;
        match self {
            Self::Callback { source, .. } => source.is_recoverable_read_failure(),
            Self::FileReference(diagnostic) => is_recoverable_miss(diagnostic),
            Self::Data(data) => match &**data {
                DataError::Malformed { .. } | DataError::ContractMismatch { .. } => true,
                DataError::FileReference(diagnostic) => is_recoverable_miss(diagnostic),
                DataError::ContractImport(_)
                | DataError::NotAContainer { .. }
                | DataError::RowShapeConflict { .. }
                | DataError::ThisImportRejected { .. }
                | DataError::UnsupportedFormat { .. }
                | DataError::InvalidOptions { .. }
                | DataError::ContractSchema { .. } => false,
            },
            Self::RemoteRead { kind, .. } => *kind == RemoteReadKind::Transport,
            Self::NestedData(_)
            | Self::GlobReference { .. }
            | Self::UnknownFunction { .. }
            | Self::Parse(_)
            | Self::InvalidCallback { .. }
            | Self::IterationLimit(_)
            | Self::Arithmetic { .. }
            | Self::Provider { .. }
            | Self::ContextNotCaptured { .. }
            | Self::SlotReadRefused { .. }
            | Self::ContractViolation { .. }
            | Self::FunctionContextNotCaptured { .. }
            | Self::ContextProjectionInvariant { .. }
            | Self::ExecutionNonceUnavailable { .. }
            | Self::ReservedRootPathUnknown { .. }
            | Self::Call(_)
            | Self::CallCheck(_)
            | Self::Binding(_)
            | Self::MalformedLiteralToken(_)
            | Self::SchemaFileRead { .. }
            | Self::Other { .. } => false,
        }
    }

    /// The `load_data` failure this error carries, after callback wrappers:
    /// a direct one ([`Data`](Self::Data)) or one re-raised by a nested
    /// composition ([`NestedData`](Self::NestedData)). Every reader that
    /// projects or grades a data failure goes through this one accessor.
    pub fn data_failure(&self) -> Option<&Arc<super::data::DataError>> {
        match self.without_callbacks() {
            Self::Data(data) | Self::NestedData(data) => Some(data),
            _ => None,
        }
    }

    /// The file-reference diagnostic this error carries, after callback
    /// wrappers: a read-side function's own failure, or a `load_data`
    /// source or contract import that could not be resolved or read.
    ///
    /// Renderers read a contract import's diagnostic through this view. They
    /// never rebuild it as an [`FileReference`](Self::FileReference) error,
    /// which `try` would then recover.
    pub fn file_reference_diagnostic(&self) -> Option<&FileReferenceDiagnostic> {
        use super::data::DataError;
        match self.without_callbacks() {
            Self::FileReference(diagnostic) => Some(diagnostic),
            Self::Data(data) | Self::NestedData(data) => match &**data {
                DataError::FileReference(diagnostic) | DataError::ContractImport(diagnostic) => Some(diagnostic),
                _ => None,
            },
            _ => None,
        }
    }

    /// Original collection positions, outermost first, including an exhausted
    /// visit's position when the body has not started yet.
    pub fn callback_path(&self) -> super::iteration::IterationPath {
        use super::iteration::{IterationFrame, IterationPath};
        match self {
            Self::Callback { function, position, source } => IterationPath::empty()
                .pushed(IterationFrame { collection: function.clone(), position: position.clone() })
                .joined(&source.callback_path()),
            Self::IterationLimit(error) => error.path.clone(),
            _ => IterationPath::empty(),
        }
    }

    /// The underlying failure after all collection callback wrappers.
    pub fn without_callbacks(&self) -> &Self {
        match self {
            Self::Callback { source, .. } => source.without_callbacks(),
            other => other,
        }
    }

    /// Whether this is a missing runtime context failure
    /// ([`ContextNotCaptured`], [`FunctionContextNotCaptured`],
    /// [`ContextProjectionInvariant`], or [`ExecutionNonceUnavailable`]).
    ///
    /// Such a failure means the request snapshot cannot answer a known
    /// `ctx.*` read, so no stage may tolerate it into partial output.
    ///
    /// [`ContextNotCaptured`]: ExpressionError::ContextNotCaptured
    /// [`FunctionContextNotCaptured`]: ExpressionError::FunctionContextNotCaptured
    /// [`ContextProjectionInvariant`]: ExpressionError::ContextProjectionInvariant
    /// [`ExecutionNonceUnavailable`]: ExpressionError::ExecutionNonceUnavailable
    pub fn is_missing_runtime_context(&self) -> bool {
        if let Self::Callback { source, .. } = self {
            return source.is_missing_runtime_context();
        }
        matches!(
            self,
            ExpressionError::ContextNotCaptured { .. }
                | ExpressionError::FunctionContextNotCaptured { .. }
                | ExpressionError::ContextProjectionInvariant { .. }
                | ExpressionError::ExecutionNonceUnavailable { .. }
        )
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

/// A present file reference that fails to resolve is fatal (see
/// [`ExpressionError::is_authoring_fatal`] for the WHY). `RemoteNotEnabled`
/// is deliberately *not* in this set: it is a v1 capability gap, not a
/// reference mistake.
fn file_reference_is_fatal(diagnostic: &FileReferenceDiagnostic) -> bool {
    matches!(diagnostic.kind, FileRefFailure::Malformed | FileRefFailure::NotFound | FileRefFailure::FoundElsewhere)
}

/// A miss of a reference that was understood and rooted: no typed cause, or
/// an I/O failure on the file itself. Tree escapes and missing anchors share
/// the [`FileRefFailure::NotFound`] label, so the cause decides.
fn is_recoverable_miss(diagnostic: &FileReferenceDiagnostic) -> bool {
    matches!(diagnostic.kind, FileRefFailure::NotFound | FileRefFailure::FoundElsewhere)
        && matches!(diagnostic.source.as_deref(), None | Some(biscuit_file::FileReferenceError::Io { .. }))
}

impl From<super::ParseError> for ExpressionError {
    fn from(error: super::ParseError) -> Self {
        if error.diagnostic_code().is_some() {
            Self::InvalidCallback { span: error.position..error.position, message: error.to_string() }
        } else {
            Self::Parse(error.to_string())
        }
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
        fn unlocatable_as_written_is_malformed() {
            for err in [
                FileReferenceError::UnsupportedScheme {
                    scheme: "ftp".to_string(),
                    reference: "ftp:spec.md".to_string(),
                },
                FileReferenceError::UnsupportedUserHome("~other/spec.md".to_string()),
                FileReferenceError::ForeignAbsolutePath {
                    path: r"C:\spec.md".to_string(),
                },
            ] {
                assert_eq!(
                    FileRefFailure::classify(&err),
                    FileRefFailure::Malformed,
                    "`{err}` must not be reported as not found"
                );
            }
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
            assert_eq!(FileRefFailure::RemoteDenied.as_str(), "remote_denied");
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
                cwd: PathBuf::from("/repo"),
                fallback_dir: None,
                source: None,
                caller: None,
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
                cwd: PathBuf::from("/repo"),
                fallback_dir: None,
                source: None,
                caller: None,
            });
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
        fn contract_violation_is_authoring_fatal_and_renders_like_other() {
            let violation = ExpressionError::ContractViolation {
                function: "recent_commits".to_string(),
                message: "count must be at least 1, got 0".to_string(),
            };
            let other = ExpressionError::Other {
                function: "recent_commits".to_string(),
                message: "count must be at least 1, got 0".to_string(),
            };
            assert!(violation.is_authoring_fatal());
            assert!(!other.is_authoring_fatal());
            assert!(!violation.is_missing_runtime_context());
            assert_eq!(violation.to_string(), other.to_string());
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

    mod recovery {
        use super::*;
        use crate::markdown::compose::expression::data::{DataError, DataSourceIdentity, DataSourceLocation};
        use crate::markdown::compose::expression::iteration::{IterationError, IterationLimitKind, IterationPath, IterationPosition};
        use biscuit_file::FileReferenceError;

        fn miss(source: Option<FileReferenceError>) -> FileReferenceDiagnostic {
            FileReferenceDiagnostic {
                function: "load_data",
                reference: "optional.yaml".to_string(),
                kind: FileRefFailure::NotFound,
                cwd: PathBuf::from("/repo"),
                fallback_dir: None,
                source: source.map(Arc::new),
                caller: None,
            }
        }

        fn with_kind(kind: FileRefFailure) -> FileReferenceDiagnostic {
            FileReferenceDiagnostic { kind, ..miss(None) }
        }

        fn identity() -> DataSourceIdentity {
            DataSourceIdentity::Local(PathBuf::from("/repo/optional.yaml"))
        }

        fn data(error: DataError) -> ExpressionError {
            ExpressionError::Data(Arc::new(error))
        }

        fn callback(error: ExpressionError) -> ExpressionError {
            ExpressionError::Callback {
                function: "map".to_string(),
                position: IterationPosition::Index(0),
                source: Box::new(error),
            }
        }

        fn malformed() -> DataError {
            DataError::Malformed {
                identity: identity(),
                format: biscuit_file::DataFormat::Yaml,
                message: "bad".to_string(),
                location: DataSourceLocation::default(),
            }
        }

        fn remote_read(kind: RemoteReadKind) -> ExpressionError {
            ExpressionError::RemoteRead {
                function: "load_data",
                reference: "https://data.test/a.json".to_string(),
                kind,
                message: "boom".to_string(),
            }
        }

        fn call_error() -> ExpressionError {
            let error = crate::markdown::compose::expression::functions::call_function("upper", &[serde_json::json!({})], None)
                .expect("registered")
                .unwrap_err();
            assert!(matches!(error, ExpressionError::Call(_)), "{error:?}");
            error
        }

        /// One case per variant of [`ExpressionError`] and of
        /// [`DataError`]: the variant, and whether `try` may recover it.
        fn cases() -> Vec<(&'static str, ExpressionError, bool)> {
            let other = |function: &str| ExpressionError::Other { function: function.to_string(), message: "x".into() };
            vec![
                ("FileReference clean miss", ExpressionError::FileReference(miss(None)), true),
                (
                    "FileReference found elsewhere",
                    ExpressionError::FileReference(with_kind(FileRefFailure::FoundElsewhere)),
                    true,
                ),
                ("FileReference malformed", ExpressionError::FileReference(with_kind(FileRefFailure::Malformed)), false),
                (
                    "FileReference remote not enabled",
                    ExpressionError::FileReference(with_kind(FileRefFailure::RemoteNotEnabled)),
                    false,
                ),
                ("FileReference remote denied", ExpressionError::FileReference(with_kind(FileRefFailure::RemoteDenied)), false),
                (
                    "GlobReference",
                    ExpressionError::GlobReference {
                        function: "find_files",
                        source: Arc::new(biscuit_file::GlobReferenceError::RejectedPrefix {
                            pattern: "https://x/*".into(),
                            prefix: "a remote URL",
                            reason: "nothing local to walk",
                        }),
                    },
                    false,
                ),
                ("UnknownFunction", ExpressionError::UnknownFunction { name: "lenght".into() }, false),
                ("Parse", ExpressionError::Parse("unexpected".into()), false),
                ("InvalidCallback", ExpressionError::InvalidCallback { span: 0..1, message: "x".into() }, false),
                ("Callback over a non-read failure", callback(other("upper")), false),
                (
                    "IterationLimit",
                    ExpressionError::IterationLimit(Box::new(IterationError {
                        kind: IterationLimitKind::Visits,
                        limit: 1,
                        used: 1,
                        requested: 1,
                        path: IterationPath::empty(),
                    })),
                    false,
                ),
                ("Arithmetic", ExpressionError::Arithmetic { op: "Addition" }, false),
                (
                    "Provider transport",
                    ExpressionError::Provider {
                        function: "pr".into(),
                        kind: ProviderFailureKind::Transport,
                        message: "x".into(),
                    },
                    false,
                ),
                (
                    "ContextNotCaptured",
                    ExpressionError::ContextNotCaptured { key: "repo_root".into(), group: ContextGroup::Git },
                    false,
                ),
                (
                    "SlotReadRefused",
                    ExpressionError::SlotReadRefused {
                        target: Box::new(crate::markdown::compose::expression::read_gate::ReadTarget {
                            function: "load_data".into(),
                            kind: crate::markdown::compose::expression::functions::ReadKind::Content,
                            location: crate::markdown::compose::expression::read_gate::ReadLocation::Local(PathBuf::from("/x")),
                            observed_hash: None,
                        }),
                        refusal: crate::markdown::compose::expression::read_gate::GateRefusal::Denied,
                    },
                    false,
                ),
                ("ContractViolation", ExpressionError::ContractViolation { function: "f".into(), message: "x".into() }, false),
                (
                    "FunctionContextNotCaptured",
                    ExpressionError::FunctionContextNotCaptured { function: "package".into(), group: ContextGroup::Git },
                    false,
                ),
                (
                    "ContextProjectionInvariant",
                    ExpressionError::ContextProjectionInvariant { key: "k".into(), group: ContextGroup::Git },
                    false,
                ),
                (
                    "ExecutionNonceUnavailable",
                    ExpressionError::ExecutionNonceUnavailable { key: "id".into(), detail: "x".into() },
                    false,
                ),
                ("ReservedRootPathUnknown", ExpressionError::ReservedRootPathUnknown { root: "current", path: "current.x".into() }, false),
                ("Call", call_error(), false),
                (
                    "CallCheck",
                    ExpressionError::CallCheck(Box::new(crate::markdown::compose::expression::call_check::CallCheck {
                        function: "load_data".into(),
                        span: 0..1,
                        arguments: Vec::new(),
                        argument_spans: Vec::new(),
                        problem: None,
                        binding_error: None,
                        always_false: None,
                        data_problem: None,
                    })),
                    false,
                ),
                (
                    "Binding",
                    ExpressionError::Binding(Box::new(crate::markdown::compose::expression::binding::BindingError::ReservedName {
                        root: "ctx".into(),
                    })),
                    false,
                ),
                (
                    "MalformedLiteralToken",
                    ExpressionError::MalformedLiteralToken(crate::markdown::literal_token::TokenError::Embedded),
                    false,
                ),
                // `DataError`, every variant.
                ("Data(FileReference) clean miss", data(DataError::FileReference(miss(None))), true),
                ("Data(FileReference) malformed", data(DataError::FileReference(with_kind(FileRefFailure::Malformed))), false),
                ("Data(ContractImport)", data(DataError::ContractImport(miss(None))), false),
                ("Data(NotAContainer)", data(DataError::NotAContainer { written: "string".into() }), false),
                (
                    "Data(RowShapeConflict)",
                    data(DataError::RowShapeConflict {
                        written: "{ a: string }[]".into(),
                        conflict: crate::markdown::compose::expression::data::RowShapeConflict::HeaderlessObjectRows,
                    }),
                    false,
                ),
                (
                    "Data(ThisImportRejected)",
                    data(DataError::ThisImportRejected { written: "Port[]".into(), name: "Port".into() }),
                    false,
                ),
                (
                    "Data(UnsupportedFormat)",
                    data(DataError::UnsupportedFormat { identity: identity(), extension: Some("dat".into()) }),
                    false,
                ),
                ("Data(Malformed)", data(malformed()), true),
                (
                    "Data(ContractMismatch)",
                    data(DataError::ContractMismatch {
                        identity: identity(),
                        format: biscuit_file::DataFormat::Yaml,
                        written: "string[]".into(),
                        failure: simplified_schema::coercion::ConversionError { failures: Vec::new() },
                        text: Arc::from("[1]"),
                        table: None,
                    }),
                    true,
                ),
                ("Data(InvalidOptions)", data(DataError::InvalidOptions { key: "delimiter", reason: "x".into() }), false),
                (
                    "Data(ContractSchema)",
                    data(DataError::ContractSchema {
                        written: "strng[]".into(),
                        source: Arc::new(simplified_schema::SchemaError::Convert {
                            property: "as".into(),
                            message: "unknown type".into(),
                        }),
                    }),
                    false,
                ),
                ("NestedData(Malformed)", ExpressionError::NestedData(Arc::new(malformed())), false),
                ("RemoteRead transport", remote_read(RemoteReadKind::Transport), true),
                ("RemoteRead policy", remote_read(RemoteReadKind::Policy), false),
                ("RemoteRead invalid reference", remote_read(RemoteReadKind::InvalidReference), false),
                ("RemoteRead infrastructure", remote_read(RemoteReadKind::Infrastructure), false),
                (
                    "SchemaFileRead",
                    ExpressionError::SchemaFileRead {
                        message: "x".into(),
                        file: Box::new(FileReferenceDiagnostic { function: "validate_schema", ..miss(None) }),
                    },
                    false,
                ),
                ("Other", other("round"), false),
            ]
        }

        #[test]
        fn every_variant_is_classified_and_callbacks_delegate() {
            for (name, error, recoverable) in cases() {
                assert_eq!(error.is_recoverable_read_failure(), recoverable, "{name}");
                let wrapped = callback(callback(error));
                assert_eq!(wrapped.is_recoverable_read_failure(), recoverable, "{name} inside two callbacks");
            }
        }

        #[test]
        fn only_an_io_cause_keeps_a_not_found_miss_recoverable() {
            use biscuit_file::ContextAnchor;
            let io = || std::io::Error::new(std::io::ErrorKind::PermissionDenied, "refused");
            let causes = vec![
                (FileReferenceError::Io { path: PathBuf::from("/repo/optional.yaml"), source: io() }, true),
                (FileReferenceError::InvalidSyntax("x".into()), false),
                (FileReferenceError::UnsupportedScheme { scheme: "ftp".into(), reference: "ftp:x".into() }, false),
                (FileReferenceError::ForeignAbsolutePath { path: r"C:\x".into() }, false),
                (FileReferenceError::MissingEnvironmentVariable { name: "HOME".into() }, false),
                (FileReferenceError::CurrentDirectory(io()), false),
                (FileReferenceError::BareRepository, false),
                (FileReferenceError::VaultNotConfigured, false),
                (FileReferenceError::UnsupportedUserHome("~x".into()), false),
                (FileReferenceError::MissingHomeContext, false),
                (FileReferenceError::OutsideRepository { sigil: '@', reference_cwd: PathBuf::from("/x") }, false),
                (
                    FileReferenceError::RepositoryEscape {
                        sigil: '@',
                        reference: "@/../x".into(),
                        repository_root: PathBuf::from("/repo"),
                        escaped_candidate: PathBuf::from("/x"),
                    },
                    false,
                ),
                (
                    FileReferenceError::BoundaryEscape {
                        reference: "../x".into(),
                        boundary: PathBuf::from("/repo"),
                        escaped_candidate: PathBuf::from("/x"),
                    },
                    false,
                ),
                (
                    FileReferenceError::RepositoryRootNotContainingSource {
                        repository_root: PathBuf::from("/repo"),
                        source_path: PathBuf::from("/x"),
                    },
                    false,
                ),
                (FileReferenceError::CwdOutsideBaseDir { base_dir: PathBuf::from("/repo"), cwd: PathBuf::from("/x") }, false),
                (FileReferenceError::ResolutionUnavailable { reference: "x".into() }, false),
                (
                    FileReferenceError::RelativeContextDirectory { anchor: ContextAnchor::WorkingDirectory, path: PathBuf::from("x") },
                    false,
                ),
                (
                    FileReferenceError::BaseDirNotRepositoryRoot {
                        base_dir: PathBuf::from("/a"),
                        repository_root: PathBuf::from("/repo"),
                    },
                    false,
                ),
                (
                    FileReferenceError::RelativeTreeEscape {
                        base_dir: PathBuf::from("/repo"),
                        candidate: PathBuf::from("/x"),
                        reference: "../x".into(),
                    },
                    false,
                ),
                (FileReferenceError::RelativePath { from: PathBuf::from("/a"), to: PathBuf::from("/b") }, false),
                (FileReferenceError::RemoteNotLocal("https://x".into()), false),
                (FileReferenceError::InvalidUrl("x".into()), false),
            ];
            for (cause, recoverable) in causes {
                let name = format!("{cause:?}");
                let diagnostic = miss(Some(cause));
                assert_eq!(diagnostic.kind, FileRefFailure::NotFound);
                assert_eq!(
                    ExpressionError::FileReference(diagnostic.clone()).is_recoverable_read_failure(),
                    recoverable,
                    "{name}"
                );
                assert_eq!(data(DataError::FileReference(diagnostic)).is_recoverable_read_failure(), recoverable, "{name}");
            }
        }

        #[test]
        fn a_contract_import_is_not_recovered_where_the_same_source_miss_is() {
            let source = ExpressionError::from(Arc::new(DataError::FileReference(miss(None))));
            let import = ExpressionError::from(Arc::new(DataError::ContractImport(miss(None))));
            assert!(source.is_recoverable_read_failure());
            assert!(!import.is_recoverable_read_failure());
            assert_eq!(source.to_string(), import.to_string(), "the two read the same");
            assert_eq!(source.is_authoring_fatal(), import.is_authoring_fatal());
            assert_eq!(
                source.file_reference_diagnostic().map(|d| &d.reference),
                import.file_reference_diagnostic().map(|d| &d.reference)
            );
        }

        #[test]
        fn a_nested_data_failure_reads_and_grades_like_a_direct_one_but_is_not_recovered() {
            let direct = data(malformed());
            let nested = ExpressionError::NestedData(Arc::new(malformed()));
            assert!(direct.is_recoverable_read_failure());
            assert!(!nested.is_recoverable_read_failure());
            assert_eq!(direct.to_string(), nested.to_string());
            assert!(nested.is_authoring_fatal());
            assert!(Arc::ptr_eq(callback(nested.clone()).data_failure().unwrap(), nested.data_failure().unwrap()));
            assert!(direct.data_failure().is_some());
            assert!(other_has_no_data_failure());
        }

        fn other_has_no_data_failure() -> bool {
            ExpressionError::FileReference(miss(None)).data_failure().is_none()
        }

        #[test]
        fn the_file_reference_view_covers_sources_and_imports_only() {
            assert!(callback(ExpressionError::FileReference(miss(None))).file_reference_diagnostic().is_some());
            assert!(data(DataError::FileReference(miss(None))).file_reference_diagnostic().is_some());
            assert!(ExpressionError::NestedData(Arc::new(DataError::ContractImport(miss(None)))).file_reference_diagnostic().is_some());
            assert!(data(malformed()).file_reference_diagnostic().is_none());
            assert!(remote_read(RemoteReadKind::Transport).file_reference_diagnostic().is_none());
        }

        #[test]
        fn remote_read_display_is_unchanged_by_its_kind() {
            for kind in [
                RemoteReadKind::Transport,
                RemoteReadKind::Policy,
                RemoteReadKind::InvalidReference,
                RemoteReadKind::Infrastructure,
            ] {
                let error = remote_read(kind);
                assert_eq!(error.to_string(), "load_data(): boom");
                assert!(!error.is_authoring_fatal());
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
                cwd: PathBuf::from("/repo"),
                fallback_dir: None,
                source: None,
                caller: None,
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
