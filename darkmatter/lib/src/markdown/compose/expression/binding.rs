//! Host binding model: how a bare expression root is classified and resolved.
//!
//! A variable root is exactly one of:
//!
//! - a **reserved namespace** (`fm`, `ctx`, `env`, `current`, `local`; the set
//!   is [`reserved_root_descriptors`]);
//! - a **global** a host registered for the current scope, available (its
//!   value may be `null`) or unavailable (reading it is a typed
//!   [`BindingError::Unavailable`]);
//! - otherwise a **document property**, present or absent (absent evaluates to
//!   `null`).
//!
//! Callback and iteration parameters are **locals**, held in an immutable
//! [`LexicalScope`] and read through [`ScopedLookup`]. A local shadows a
//! document property but never a reserved namespace or a registered global;
//! `local.<name>` names the nearest local explicitly and never reaches the
//! document, so a frontmatter key `local` is read as `fm.local`.
//!
//! A host declares its globals once, in an immutable, provider-free
//! [`BindingView`]. Before anything is evaluated it supplies one
//! [`RuntimeBinding`] per declared global, and [`EvaluationSession::associate`]
//! checks the complete registration against the view. Passive validation
//! ([`validate_prepared`](super::prepared::validate_prepared)) reads the same
//! view without runtime values.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use serde_json::Value;

use super::catalog::reserved_root_descriptors;
use super::{EvaluationLookup, ExpressionError, ResolutionContext};
use crate::markdown::compose::value_origin::OverrideOrigin;
use crate::markdown::span::SourceSpan;

/// How one variable path resolved.
///
/// `Document` and `Namespace` carry `None` when the path is absent; the
/// evaluator reads that as `null`. A `Global` always has a value: an available
/// global whose value is `null`, or whose root lacks the requested member,
/// stays a `Global` holding `null` and never falls through to the document.
#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedBinding {
    /// A document property, read bare or through `fm.`.
    Document {
        /// The property value, or `None` when absent.
        value: Option<Value>,
    },
    /// A path under a reserved namespace.
    Namespace {
        /// The namespace member, or `None` when absent.
        value: Option<Value>,
    },
    /// A path under an available host-registered global.
    Global {
        /// The selected value; `null` for a `null` global or a missing member.
        value: Value,
    },
    /// A path under a lexical local (a callback or iteration parameter).
    ///
    /// Like a `Global`, a present local always has a value: a `null` local, or
    /// a missing member of a present local, stays a `Local` holding `null`
    /// and never falls through to the document.
    Local {
        /// The selected value.
        value: Value,
        /// The binding the read resolved to, never just its spelling.
        id: LocalId,
        /// Whether the bound value is authored template text or inert data.
        origin: OverrideOrigin,
    },
}

impl ResolvedBinding {
    /// Tags `value` as a reserved-namespace read when `path`'s root is a
    /// reserved namespace, otherwise as a document read.
    pub fn classify(path: &str, value: Option<Value>) -> Self {
        if is_reserved_namespace(root_of(path)) {
            Self::Namespace { value }
        } else {
            Self::Document { value }
        }
    }

    /// The resolved value, or `None` for an absent document or namespace path.
    pub fn value(&self) -> Option<&Value> {
        match self {
            Self::Document { value } | Self::Namespace { value } => value.as_ref(),
            Self::Global { value } | Self::Local { value, .. } => Some(value),
        }
    }

    /// Consumes the binding, returning [`value`](Self::value).
    pub fn into_value(self) -> Option<Value> {
        match self {
            Self::Document { value } | Self::Namespace { value } => value,
            Self::Global { value } | Self::Local { value, .. } => Some(value),
        }
    }
}

/// Whether `root` is one of the reserved namespaces no host may register.
pub fn is_reserved_namespace(root: &str) -> bool {
    reserved_root_descriptors()
        .iter()
        .any(|descriptor| descriptor.name == root)
}

/// The first dotted segment of a variable path.
fn root_of(path: &str) -> &str {
    path.split('.').next().unwrap_or(path)
}

/// Whether `root` is a single expression identifier (no dots, no brackets),
/// using the lexer's identifier rule.
fn is_root_identifier(root: &str) -> bool {
    let mut chars = root.chars();
    chars
        .next()
        .is_some_and(|first| first.is_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_alphanumeric() || ch == '_')
}

/// Opaque identity of the scope a [`BindingView`] describes, such as a
/// lifecycle event. Darkmatter never interprets it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ScopeId(String);

impl ScopeId {
    /// Creates a scope identity.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ScopeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Why a global is unavailable in a scope.
///
/// The code is stable and namespaced by its owner (`claudine.outside-group`);
/// Darkmatter carries it and never matches on it. Parameters are structured
/// detail for diagnostics.
#[derive(Debug, Clone, PartialEq)]
pub struct UnavailabilityReason {
    code: String,
    parameters: serde_json::Map<String, Value>,
}

impl UnavailabilityReason {
    /// Creates a reason with no parameters. The code is validated when the
    /// reason is declared or associated, not here.
    pub fn new(code: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            parameters: serde_json::Map::new(),
        }
    }

    /// Adds one structured parameter.
    #[must_use]
    pub fn with_parameter(mut self, key: impl Into<String>, value: Value) -> Self {
        self.parameters.insert(key.into(), value);
        self
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn parameters(&self) -> &serde_json::Map<String, Value> {
        &self.parameters
    }

    /// `owner.reason`: at least two non-empty dot-separated segments of ASCII
    /// lowercase letters, digits, `-`, or `_`.
    fn is_namespaced(&self) -> bool {
        let mut segments = 0;
        for segment in self.code.split('.') {
            if segment.is_empty()
                || !segment
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
            {
                return false;
            }
            segments += 1;
        }
        segments >= 2
    }
}

impl fmt::Display for UnavailabilityReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.code)
    }
}

/// A global's static availability in one scope.
#[derive(Debug, Clone, PartialEq)]
pub enum Availability {
    /// Always supplied; the value may be `null`.
    Available,
    /// Never supplied; reading it fails with this reason.
    Unavailable(UnavailabilityReason),
    /// Known only at invocation. Passive validation defers it; association
    /// requires an explicit runtime entry of either kind.
    ExecutionDependent,
}

impl fmt::Display for Availability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Available => f.write_str("available"),
            Self::Unavailable(reason) => write!(f, "unavailable ({reason})"),
            Self::ExecutionDependent => f.write_str("execution-dependent"),
        }
    }
}

/// One global a host declares for a scope.
#[derive(Debug, Clone, PartialEq)]
pub struct GlobalDeclaration {
    root: String,
    description: String,
    availability: Availability,
}

impl GlobalDeclaration {
    pub fn root(&self) -> &str {
        &self.root
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn availability(&self) -> &Availability {
        &self.availability
    }
}

/// What a root names under a [`BindingView`], without any runtime value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RootClass<'v> {
    /// A reserved namespace.
    Namespace,
    /// A declared global with its static availability.
    Global(&'v Availability),
    /// A document property.
    Document,
}

/// The immutable, provider-free declaration of one scope's globals.
///
/// Shared by passive validation and runtime association, so both classify a
/// root identically. Build one with [`BindingView::builder`].
#[derive(Debug, Clone, PartialEq)]
pub struct BindingView {
    scope: ScopeId,
    globals: BTreeMap<String, GlobalDeclaration>,
}

impl BindingView {
    /// Starts a view for `scope`.
    pub fn builder(scope: ScopeId) -> BindingViewBuilder {
        BindingViewBuilder {
            scope,
            declarations: Vec::new(),
        }
    }

    /// The Darkmatter baseline: the reserved namespaces and no host globals.
    ///
    /// The view a static consumer without host descriptors (DMLS) classifies
    /// against, so every bare root outside the reserved namespaces is a
    /// document property, exactly as compose resolves it.
    pub fn baseline() -> Self {
        Self {
            scope: ScopeId::new(BASELINE_SCOPE),
            globals: BTreeMap::new(),
        }
    }

    pub fn scope(&self) -> &ScopeId {
        &self.scope
    }

    /// The declaration for `root`, if `root` is a declared global.
    pub fn global(&self, root: &str) -> Option<&GlobalDeclaration> {
        self.globals.get(root)
    }

    /// Every declared global, ordered by root.
    pub fn globals(&self) -> impl Iterator<Item = &GlobalDeclaration> {
        self.globals.values()
    }

    /// Classifies `root`: reserved namespaces first, declared globals second,
    /// document properties last.
    pub fn classify_root(&self, root: &str) -> RootClass<'_> {
        if is_reserved_namespace(root) {
            RootClass::Namespace
        } else if let Some(declaration) = self.globals.get(root) {
            RootClass::Global(&declaration.availability)
        } else {
            RootClass::Document
        }
    }

    /// Whether a bare `root` reads a document property under this view: it
    /// [classifies](Self::classify_root) as one and is not the `null` literal.
    ///
    /// The grammar has no `null` literal; `null` lexes as a variable that
    /// resolves to nothing, so it is never an undeclared property. Static
    /// consumers add frontmatter, schema, and function membership themselves.
    pub fn names_document_property(&self, root: &str) -> bool {
        matches!(self.classify_root(root), RootClass::Document) && root != NULL_ROOT
    }
}

/// The scope identity of [`BindingView::baseline`].
const BASELINE_SCOPE: &str = "darkmatter.baseline";

/// The variable spelling authors use as the `null` literal.
pub(crate) const NULL_ROOT: &str = "null";

/// Collects global declarations for [`BindingView::build`](BindingViewBuilder::build).
#[derive(Debug)]
pub struct BindingViewBuilder {
    scope: ScopeId,
    declarations: Vec<GlobalDeclaration>,
}

impl BindingViewBuilder {
    /// Declares one global root.
    #[must_use]
    pub fn declare(
        mut self,
        root: impl Into<String>,
        description: impl Into<String>,
        availability: Availability,
    ) -> Self {
        self.declarations.push(GlobalDeclaration {
            root: root.into(),
            description: description.into(),
            availability,
        });
        self
    }

    /// Validates every declaration and freezes the view.
    ///
    /// ## Errors
    ///
    /// A root that is not a single identifier, names a reserved namespace, or
    /// is declared twice, and an unavailability reason whose code is not
    /// namespaced.
    pub fn build(self) -> Result<BindingView, BindingError> {
        let mut globals = BTreeMap::new();
        for declaration in self.declarations {
            check_root(&declaration.root)?;
            if let Availability::Unavailable(reason) = &declaration.availability {
                check_reason(&declaration.root, reason)?;
            }
            if globals.contains_key(&declaration.root) {
                return Err(BindingError::Duplicate {
                    root: declaration.root,
                });
            }
            globals.insert(declaration.root.clone(), declaration);
        }
        Ok(BindingView {
            scope: self.scope,
            globals,
        })
    }
}

fn check_root(root: &str) -> Result<(), BindingError> {
    if !is_root_identifier(root) {
        return Err(BindingError::InvalidRoot {
            root: root.to_string(),
        });
    }
    if is_reserved_namespace(root) {
        return Err(BindingError::ReservedName {
            root: root.to_string(),
        });
    }
    Ok(())
}

fn check_reason(root: &str, reason: &UnavailabilityReason) -> Result<(), BindingError> {
    if reason.is_namespaced() {
        Ok(())
    } else {
        Err(BindingError::InvalidReasonCode {
            root: root.to_string(),
            code: reason.code.clone(),
        })
    }
}

/// A lazy global's provider. It runs at most once per [`EvaluationSession`],
/// only when the root is first read.
pub type LazyProvider<'a> = Arc<dyn Fn() -> Value + Send + Sync + 'a>;

/// The runtime entry a host supplies for one declared global.
#[derive(Clone)]
pub enum RuntimeBinding<'a> {
    /// A value computed before evaluation.
    Eager(Value),
    /// A value computed on first read and cached for the session.
    Lazy(LazyProvider<'a>),
    /// Reading the global fails with this reason.
    Unavailable(UnavailabilityReason),
}

impl<'a> RuntimeBinding<'a> {
    pub fn eager(value: Value) -> Self {
        Self::Eager(value)
    }

    pub fn lazy<F>(provider: F) -> Self
    where
        F: Fn() -> Value + Send + Sync + 'a,
    {
        Self::Lazy(Arc::new(provider))
    }

    pub fn unavailable(reason: UnavailabilityReason) -> Self {
        Self::Unavailable(reason)
    }

    fn kind(&self) -> &'static str {
        match self {
            Self::Eager(_) => "eager",
            Self::Lazy(_) => "lazy",
            Self::Unavailable(_) => "unavailable",
        }
    }
}

impl fmt::Debug for RuntimeBinding<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Eager(value) => f.debug_tuple("Eager").field(value).finish(),
            Self::Lazy(_) => f.debug_tuple("Lazy").field(&"Fn(..)").finish(),
            Self::Unavailable(reason) => f.debug_tuple("Unavailable").field(reason).finish(),
        }
    }
}

/// A read of a global the scope declares unavailable.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[error("`{path}` reads the global `{root}`, which is unavailable in scope `{scope}` ({reason})")]
pub struct UnavailableBinding {
    /// The global's root.
    pub root: String,
    /// The full path read, root included.
    pub path: String,
    /// The scope that made it unavailable.
    pub scope: ScopeId,
    /// The host's reason.
    pub reason: UnavailabilityReason,
    /// The read's span in its authored text, when the reader knows it.
    pub span: Option<SourceSpan>,
}

/// A binding failure: a configuration error found at declaration or
/// association, or a read of an unavailable global.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum BindingError {
    /// A read of a global the scope declares unavailable.
    #[error(transparent)]
    Unavailable(Box<UnavailableBinding>),

    /// A global registered under a reserved namespace name.
    #[error("`{root}` is a reserved namespace and cannot be registered as a global")]
    ReservedName {
        root: String,
    },

    /// A global root that is not a single identifier (for example a dotted path).
    #[error("`{root}` is not a valid global root; a global is registered by a single identifier")]
    InvalidRoot {
        root: String,
    },

    /// The same global declared or registered twice.
    #[error("global `{root}` is registered more than once")]
    Duplicate {
        root: String,
    },

    /// A runtime entry for a root the view does not declare.
    #[error("global `{root}` is not declared for scope `{scope}`")]
    UnknownGlobal {
        root: String,
        scope: ScopeId,
    },

    /// A declared global with no runtime entry. Unavailability is an explicit
    /// entry, never an omission.
    #[error("declared global `{root}` has no runtime entry for scope `{scope}`")]
    OmittedDeclaredGlobal {
        root: String,
        scope: ScopeId,
    },

    /// A runtime entry that contradicts a definite declaration.
    #[error(
        "runtime entry for global `{root}` contradicts its declaration for scope \
         `{scope}`: declared {declared}, supplied {supplied}"
    )]
    ContradictsDeclaration {
        root: String,
        scope: ScopeId,
        /// The declared availability.
        declared: String,
        /// The supplied entry kind.
        supplied: String,
    },

    /// An unavailability reason whose code is not namespaced.
    #[error("unavailability reason `{code}` for global `{root}` is not namespaced (expected `owner.reason`)")]
    InvalidReasonCode {
        root: String,
        code: String,
    },
}

impl BindingError {
    /// The global root the failure concerns.
    pub fn root(&self) -> &str {
        match self {
            Self::Unavailable(read) => &read.root,
            Self::ReservedName { root }
            | Self::InvalidRoot { root }
            | Self::Duplicate { root }
            | Self::UnknownGlobal { root, .. }
            | Self::OmittedDeclaredGlobal { root, .. }
            | Self::ContradictsDeclaration { root, .. }
            | Self::InvalidReasonCode { root, .. } => root,
        }
    }
}

/// A complete, checked association of a [`BindingView`], a document lookup,
/// and one runtime entry per declared global.
///
/// It is itself an [`EvaluationLookup`]: reserved namespaces resolve through
/// the document lookup, registered globals through their runtime entries, and
/// everything else is a document property. Each lazy global runs at most once
/// per session, `null` results included, so a session is one direct
/// evaluation or one subtree composition, never a cross-event store.
pub struct EvaluationSession<'a> {
    view: Arc<BindingView>,
    document: &'a dyn EvaluationLookup,
    runtime: HashMap<String, RuntimeBinding<'a>>,
    cache: Mutex<HashMap<String, Value>>,
    resolution_context: Option<ResolutionContext>,
    iteration: super::iteration::IterationBudget,
}

impl fmt::Debug for EvaluationSession<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EvaluationSession")
            .field("view", &self.view)
            .field("runtime", &self.runtime)
            .finish_non_exhaustive()
    }
}

impl<'a> EvaluationSession<'a> {
    /// Checks the complete runtime registration against `view` and opens a
    /// session. Nothing is evaluated and no lazy provider runs here.
    ///
    /// ## Errors
    ///
    /// Any [`BindingError`] configuration variant: an invalid or reserved
    /// root, a duplicate entry, an entry the view does not declare, a declared
    /// global left out, an entry contradicting an `available` or
    /// `unavailable` declaration (an unavailable entry must carry the declared
    /// reason code), or a non-namespaced reason code.
    pub fn associate<I>(
        view: Arc<BindingView>,
        document: &'a dyn EvaluationLookup,
        runtime: I,
    ) -> Result<Self, BindingError>
    where
        I: IntoIterator<Item = (String, RuntimeBinding<'a>)>,
    {
        let entries = check_registration(&view, runtime)?;
        Ok(Self {
            view,
            document,
            runtime: entries,
            cache: Mutex::new(HashMap::new()),
            resolution_context: None,
            iteration: super::iteration::IterationBudget::new(Default::default()),
        })
    }

    /// Supplies the read-side functions' resolution context, in place of the
    /// document lookup's own.
    #[must_use]
    pub fn with_resolution_context(mut self, resolution_context: ResolutionContext) -> Self {
        self.resolution_context = Some(resolution_context);
        self
    }

    /// Counts this session's iteration against a fresh budget of `limits`
    /// instead of the defaults.
    #[must_use]
    pub fn with_iteration_limits(mut self, limits: super::iteration::IterationLimits) -> Self {
        self.iteration = super::iteration::IterationBudget::new(limits);
        self
    }

    /// The session-owned visit and expansion counters.
    pub fn iteration_budget(&self) -> &super::iteration::IterationBudget {
        &self.iteration
    }

    /// The view this session was associated against.
    pub fn view(&self) -> &BindingView {
        &self.view
    }
}

/// Checks a complete runtime registration against `view`: one valid entry per
/// declared global, each agreeing with its declaration.
fn check_registration<'a, I>(
    view: &BindingView,
    runtime: I,
) -> Result<HashMap<String, RuntimeBinding<'a>>, BindingError>
where
    I: IntoIterator<Item = (String, RuntimeBinding<'a>)>,
{
    let mut entries = HashMap::new();
    for (root, binding) in runtime {
        check_root(&root)?;
        if let RuntimeBinding::Unavailable(reason) = &binding {
            check_reason(&root, reason)?;
        }
        let Some(declaration) = view.global(&root) else {
            return Err(BindingError::UnknownGlobal {
                root,
                scope: view.scope.clone(),
            });
        };
        check_agreement(&view.scope, declaration, &binding)?;
        if entries.contains_key(&root) {
            return Err(BindingError::Duplicate { root });
        }
        entries.insert(root, binding);
    }
    if let Some(omitted) = view.globals().find(|global| !entries.contains_key(&global.root)) {
        return Err(BindingError::OmittedDeclaredGlobal {
            root: omitted.root.clone(),
            scope: view.scope.clone(),
        });
    }
    Ok(entries)
}

/// Reads an available global's `root`, running a lazy provider on first read.
/// The cache lock is never held while the provider runs.
fn read_global(
    cache: &Mutex<HashMap<String, Value>>,
    root: &str,
    binding: &RuntimeBinding<'_>,
    rest: &str,
) -> Value {
    match binding {
        RuntimeBinding::Eager(value) => project(value, rest),
        RuntimeBinding::Lazy(provider) => {
            if let Some(cached) = lock_cache(cache).get(root) {
                return project(cached, rest);
            }
            let value = provider();
            let mut cache = lock_cache(cache);
            // A re-entrant provider may have filled the slot first; the
            // first value wins so every read agrees.
            let cached = cache.entry(root.to_string()).or_insert(value);
            project(cached, rest)
        }
        RuntimeBinding::Unavailable(_) => Value::Null,
    }
}

/// The cache only ever gains complete entries, so a poisoned lock still
/// guards consistent data.
fn lock_cache(cache: &Mutex<HashMap<String, Value>>) -> MutexGuard<'_, HashMap<String, Value>> {
    cache.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The typed error for a read of `path` under a global `root` that `scope`
/// declares unavailable.
fn unavailable_read(
    scope: &ScopeId,
    root: &str,
    path: &str,
    reason: &UnavailabilityReason,
) -> ExpressionError {
    ExpressionError::Binding(Box::new(BindingError::Unavailable(Box::new(UnavailableBinding {
        root: root.to_string(),
        path: path.to_string(),
        scope: scope.clone(),
        reason: reason.clone(),
        span: None,
    }))))
}

/// Globals a host declares for one whole composition
/// ([`ComposeOptions::with_host_globals`](crate::markdown::compose::ComposeOptions::with_host_globals)):
/// a [`BindingView`] and one checked runtime entry per declared global.
///
/// Composition classifies every variable root against it in the order of
/// [`BindingView::classify_root`]: a reserved namespace, then a declared
/// global, then a document property. A declared global therefore wins over a
/// same-named frontmatter key for bare, dotted, and bracket reads in body and
/// frontmatter interpolation, conditions, and page blocks; `fm.<name>` still
/// reads the key. The globals reach every transcluded child, because a child
/// composes with its parent's options.
///
/// Unlike an [`EvaluationSession`], which is one evaluation, a value spans a
/// composition and every clone of its options: a lazy provider runs at most
/// once across them.
#[derive(Clone)]
pub struct HostGlobals {
    document_fallbacks: BTreeSet<String>,
    view: Arc<BindingView>,
    runtime: Arc<HashMap<String, RuntimeBinding<'static>>>,
    cache: Arc<Mutex<HashMap<String, Value>>>,
}

impl fmt::Debug for HostGlobals {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HostGlobals")
            .field("view", &self.view)
            .field("runtime", &self.runtime)
            .finish_non_exhaustive()
    }
}

impl HostGlobals {
    /// Checks the complete runtime registration against `view`, exactly as
    /// [`EvaluationSession::associate`] does. No lazy provider runs here.
    ///
    /// ## Errors
    ///
    /// Any [`BindingError`] configuration variant [`EvaluationSession::associate`]
    /// reports.
    pub fn associate<I>(view: Arc<BindingView>, runtime: I) -> Result<Self, BindingError>
    where
        I: IntoIterator<Item = (String, RuntimeBinding<'static>)>,
    {
        let runtime = check_registration(&view, runtime)?;
        Ok(Self {
            document_fallbacks: BTreeSet::new(),
            view,
            runtime: Arc::new(runtime),
            cache: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Let present document properties take precedence for selected roots.
    /// Presence includes null. This opt-in supports migration diagnostics
    /// whose unavailable fallback applies only to undeclared names; lexical
    /// globals should keep the default whole-root shadowing behavior.
    pub fn with_document_fallbacks(mut self, roots: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.document_fallbacks = roots.into_iter().map(Into::into).collect();
        self
    }

    pub(crate) fn prefers_document(&self, root: &str) -> bool {
        self.document_fallbacks.contains(root)
    }

    /// The view these globals were associated against.
    pub fn view(&self) -> &BindingView {
        &self.view
    }

    /// A canonical description of the declaration and every entry, for the
    /// compose-options fingerprints. A lazy entry is identified by this value's
    /// shared cache, so independently built globals never compare equal.
    pub(crate) fn identity(&self) -> String {
        use biscuit_hash::canonical_json;
        let mut roots: Vec<&String> = self.runtime.keys().collect();
        roots.sort_unstable();
        let mut identity = format!("scope={};document_fallbacks={:?}", self.view.scope(), self.document_fallbacks);
        for root in roots {
            let entry = match &self.runtime[root] {
                RuntimeBinding::Eager(value) => format!("eager:{}", canonical_json(value)),
                RuntimeBinding::Lazy(_) => format!("lazy:{:p}", Arc::as_ptr(&self.cache)),
                RuntimeBinding::Unavailable(reason) => format!("unavailable:{reason}"),
            };
            identity.push_str(&format!(";{root}={entry}"));
        }
        identity
    }

    /// Every declared global root.
    pub(crate) fn roots(&self) -> impl Iterator<Item = &str> {
        self.runtime.keys().map(String::as_str)
    }

    /// Whether `root` is a declared global (never a reserved namespace).
    pub(crate) fn declares(&self, root: &str) -> bool {
        self.runtime.contains_key(root)
    }

    /// Resolves `path` when its root is a declared global; `None` leaves it to
    /// the reserved namespaces or the document.
    pub(crate) fn resolve(&self, path: &str) -> Option<Result<ResolvedBinding, ExpressionError>> {
        let root = root_of(path);
        let binding = self.runtime.get(root)?;
        Some(match binding {
            RuntimeBinding::Unavailable(reason) => {
                Err(unavailable_read(self.view.scope(), root, path, reason))
            }
            binding => Ok(ResolvedBinding::Global {
                value: read_global(&self.cache, root, binding, &path[root.len()..]),
            }),
        })
    }
}

/// An `available` declaration needs a value, an `unavailable` one the same
/// reason code; `execution-dependent` accepts either.
fn check_agreement(
    scope: &ScopeId,
    declaration: &GlobalDeclaration,
    binding: &RuntimeBinding<'_>,
) -> Result<(), BindingError> {
    let agrees = match (&declaration.availability, binding) {
        (Availability::ExecutionDependent, _) => true,
        (Availability::Available, RuntimeBinding::Eager(_) | RuntimeBinding::Lazy(_)) => true,
        (Availability::Unavailable(declared), RuntimeBinding::Unavailable(supplied)) => {
            declared.code == supplied.code
        }
        _ => false,
    };
    if agrees {
        return Ok(());
    }
    let supplied = match binding {
        RuntimeBinding::Unavailable(reason) => format!("unavailable ({reason})"),
        other => other.kind().to_string(),
    };
    Err(BindingError::ContradictsDeclaration {
        root: declaration.root.clone(),
        scope: scope.clone(),
        declared: declaration.availability.to_string(),
        supplied,
    })
}

/// Selects `rest` (`a.b`, or empty for the root) from `value`; a missing
/// member is `null`, never a fall-through.
fn project(value: &Value, rest: &str) -> Value {
    let mut current = value;
    for segment in rest.split('.').filter(|segment| !segment.is_empty()) {
        match current {
            Value::Object(map) => match map.get(segment) {
                Some(next) => current = next,
                None => return Value::Null,
            },
            _ => return Value::Null,
        }
    }
    current.clone()
}

impl EvaluationLookup for EvaluationSession<'_> {
    fn get(&self, path: &str) -> Option<Value> {
        self.resolve(path).ok().and_then(ResolvedBinding::into_value)
    }

    fn resolve(&self, path: &str) -> Result<ResolvedBinding, ExpressionError> {
        let root = root_of(path);
        if is_reserved_namespace(root) {
            return self.document.resolve(path).map(|binding| match binding {
                ResolvedBinding::Document { value } => ResolvedBinding::Namespace { value },
                other => other,
            });
        }
        match self.runtime.get(root) {
            Some(RuntimeBinding::Unavailable(reason)) => {
                Err(unavailable_read(&self.view.scope, root, path, reason))
            }
            Some(binding) => Ok(ResolvedBinding::Global {
                value: read_global(&self.cache, root, binding, &path[root.len()..]),
            }),
            None => self.document.resolve(path),
        }
    }

    fn binding_view(&self) -> Option<&BindingView> {
        Some(&self.view)
    }

    fn format_resolved(&self, path: &str, value: &Value) -> String {
        self.document.format_resolved(path, value)
    }

    fn resolution_context(&self) -> Option<ResolutionContext> {
        self.resolution_context
            .clone()
            .or_else(|| self.document.resolution_context())
    }

    fn resolution_context_ref(&self) -> Option<&ResolutionContext> {
        self.resolution_context
            .as_ref()
            .or_else(|| self.document.resolution_context_ref())
    }

    fn is_valid_context_variable(&self, name: &str) -> bool {
        self.document.is_valid_context_variable(name)
    }

    fn context_variable_names(&self) -> &[&'static str] {
        self.document.context_variable_names()
    }

    fn begin_expression_scope(&self) {
        self.document.begin_expression_scope();
    }

    fn lexical_scope(&self) -> Option<&Arc<LexicalScope>> {
        self.document.lexical_scope()
    }
}

/// Identity of one local binding: the frame that owns it and its slot there.
///
/// Two reads of the same spelling in different frames, or of different
/// spellings of one slot (`x` and `local.x`), are told apart by this id, not
/// by their text. Runtime frames ([`LexicalScope`]) and passive declared-type
/// layers ([`LexicalTypes`](super::scope::LexicalTypes)) both mint ids from
/// one counter, so an id never names two bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocalId {
    frame: u64,
    slot: usize,
}

impl LocalId {
    /// The owning frame's process-unique identity.
    pub const fn frame(&self) -> u64 {
        self.frame
    }

    /// The binding's position within its frame.
    pub const fn slot(&self) -> usize {
        self.slot
    }

    /// The id of slot `slot` in a frame minted by [`next_frame_id`].
    pub(crate) const fn new(frame: u64, slot: usize) -> Self {
        Self { frame, slot }
    }
}

/// A process-unique frame identity shared by runtime frames and declared-type
/// layers.
pub(crate) fn next_frame_id() -> u64 {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

#[derive(Debug)]
struct LocalSlot {
    name: String,
    value: Value,
    origin: OverrideOrigin,
}

/// One local found by [`LexicalScope::resolve`].
#[derive(Debug, Clone, Copy)]
pub struct LocalBinding<'s> {
    /// The binding's identity.
    pub id: LocalId,
    /// The bound value; `null` is a present value.
    pub value: &'s Value,
    /// Whether the value is authored template text or inert data. Origin
    /// travels beside the value, as for every other composed value.
    pub origin: OverrideOrigin,
}

/// One immutable frame of lexical locals, retaining its parent.
///
/// Frames are shared through [`Arc`], so a body copy can keep the frame of
/// its iteration alive after the callback that created it has returned.
/// A frame never changes after construction; a nested scope is a new frame
/// whose bindings shadow its parents'. A frame copies no document state:
/// it holds only its own bindings and a reference to its parent.
#[derive(Debug)]
pub struct LexicalScope {
    id: u64,
    slots: Vec<LocalSlot>,
    parent: Option<Arc<LexicalScope>>,
}

impl LexicalScope {
    /// An outermost frame holding `bindings` in order, each [`Data`](OverrideOrigin::Data).
    pub fn new(bindings: Vec<(String, Value)>) -> Arc<Self> {
        Self::build(None, with_data_origin(bindings))
    }

    /// A nested frame holding `bindings`, shadowing this frame's, each
    /// [`Data`](OverrideOrigin::Data).
    pub fn child(self: &Arc<Self>, bindings: Vec<(String, Value)>) -> Arc<Self> {
        Self::build(Some(Arc::clone(self)), with_data_origin(bindings))
    }

    /// [`Self::new`] with an explicit origin per binding.
    pub fn with_origins(bindings: Vec<(String, Value, OverrideOrigin)>) -> Arc<Self> {
        Self::build(None, bindings)
    }

    /// [`Self::child`] with an explicit origin per binding.
    pub fn child_with_origins(self: &Arc<Self>, bindings: Vec<(String, Value, OverrideOrigin)>) -> Arc<Self> {
        Self::build(Some(Arc::clone(self)), bindings)
    }

    fn build(parent: Option<Arc<Self>>, bindings: Vec<(String, Value, OverrideOrigin)>) -> Arc<Self> {
        Arc::new(Self {
            id: next_frame_id(),
            slots: bindings
                .into_iter()
                .map(|(name, value, origin)| LocalSlot { name, value, origin })
                .collect(),
            parent,
        })
    }

    /// This frame's process-unique identity.
    pub fn frame_id(&self) -> u64 {
        self.id
    }

    /// The enclosing frame, if any.
    pub fn parent(&self) -> Option<&Arc<Self>> {
        self.parent.as_ref()
    }

    /// The nearest binding named `name`, searching this frame then its
    /// parents. A binding whose value is `null` is still found.
    pub fn resolve(&self, name: &str) -> Option<LocalBinding<'_>> {
        let mut frame = self;
        loop {
            if let Some((slot, found)) = frame.slots.iter().enumerate().find(|(_, slot)| slot.name == name) {
                return Some(LocalBinding {
                    id: LocalId::new(frame.id, slot),
                    value: &found.value,
                    origin: found.origin,
                });
            }
            frame = frame.parent.as_deref()?;
        }
    }

    /// [`Self::resolve`] without the origin.
    pub fn lookup(&self, name: &str) -> Option<(LocalId, &Value)> {
        self.resolve(name).map(|found| (found.id, found.value))
    }

    /// Every visible name, nearest binding first, shadowed names once.
    /// Reads names only: no value is projected and nothing is observed.
    pub fn names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = Vec::new();
        let mut frame = Some(self);
        while let Some(current) = frame {
            for slot in &current.slots {
                if !names.contains(&slot.name.as_str()) {
                    names.push(&slot.name);
                }
            }
            frame = current.parent.as_deref();
        }
        names
    }
}

fn with_data_origin(bindings: Vec<(String, Value)>) -> Vec<(String, Value, OverrideOrigin)> {
    bindings
        .into_iter()
        .map(|(name, value)| (name, value, OverrideOrigin::Data))
        .collect()
}

/// Why a name cannot be bound as a local.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtectedNameKind {
    /// A reserved root (`fm`, `ctx`, `env`, `current`, `local`).
    ReservedRoot,
    /// The `null` literal's spelling.
    NullLiteral,
    /// A global the host declared in the actual [`BindingView`].
    HostGlobal,
}

impl fmt::Display for ProtectedNameKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::ReservedRoot => "a reserved root",
            Self::NullLiteral => "the `null` literal",
            Self::HostGlobal => "a registered host global",
        })
    }
}

/// A pattern name that collides with a protected name.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("`{name}` cannot be bound as a local name: it is {kind}")]
pub struct ProtectedLocalName {
    /// The offending name.
    pub name: String,
    /// What it collides with.
    pub kind: ProtectedNameKind,
}

fn protected_kind(view: Option<&BindingView>, name: &str) -> Option<ProtectedNameKind> {
    if is_reserved_namespace(name) {
        Some(ProtectedNameKind::ReservedRoot)
    } else if name == NULL_ROOT {
        Some(ProtectedNameKind::NullLiteral)
    } else if view.is_some_and(|view| view.global(name).is_some()) {
        Some(ProtectedNameKind::HostGlobal)
    } else {
        None
    }
}

/// Rejects the first of `names` that a pattern may not bind.
///
/// `view` is the [`BindingView`] actually in force (`None` declares no
/// globals). Ordinary shadowing of a document property or an outer local is
/// allowed and silent.
///
/// ## Errors
///
/// [`ProtectedLocalName`] for a reserved root, `null`, or a declared global.
pub fn check_local_names<'n>(
    names: impl IntoIterator<Item = &'n str>,
    view: Option<&BindingView>,
) -> Result<(), ProtectedLocalName> {
    for name in names {
        if let Some(kind) = protected_kind(view, name) {
            return Err(ProtectedLocalName {
                name: name.to_string(),
                kind,
            });
        }
    }
    Ok(())
}

/// An [`EvaluationLookup`] that reads locals from a [`LexicalScope`] and
/// delegates everything else to an inner lookup.
///
/// A read resolves in this order: a reserved namespace or declared global (so
/// a local can never shadow one), then the nearest local named by the bare
/// root or by `local.<name>`, then the inner lookup. A present local answers
/// even when null, and a missing member of it is `null`, never a fall-through.
/// `local.<name>` with no such local is absent and never reaches the
/// document. Every other hook forwards to the inner lookup.
pub struct ScopedLookup<'a> {
    inner: &'a dyn EvaluationLookup,
    scope: Arc<LexicalScope>,
}

impl<'a> ScopedLookup<'a> {
    /// Layers `scope` over `inner`.
    pub fn new(inner: &'a dyn EvaluationLookup, scope: Arc<LexicalScope>) -> Self {
        Self { inner, scope }
    }

    fn protected(&self, root: &str) -> bool {
        is_reserved_namespace(root)
            || self
                .inner
                .binding_view()
                .is_some_and(|view| matches!(view.classify_root(root), RootClass::Global(_) | RootClass::Namespace))
    }

    fn local(&self, path: &str) -> Option<ResolvedBinding> {
        let root = root_of(path);
        let (name, rest) = if root == LOCAL_ROOT {
            let absent = || Some(ResolvedBinding::Namespace { value: None });
            let Some(tail) = path[root.len()..].strip_prefix('.') else {
                return absent();
            };
            let name = root_of(tail);
            (name, &tail[name.len()..])
        } else if self.protected(root) {
            return None;
        } else {
            (root, &path[root.len()..])
        };
        match self.scope.resolve(name) {
            Some(found) => Some(ResolvedBinding::Local {
                value: project(found.value, rest),
                id: found.id,
                origin: found.origin,
            }),
            None if root == LOCAL_ROOT => Some(ResolvedBinding::Namespace { value: None }),
            None => None,
        }
    }
}

/// The reserved root that names the innermost locals explicitly.
pub const LOCAL_ROOT: &str = "local";

/// Whether `path` is `local` or reads below it.
///
/// Document lookups use this to answer absent: the `local` namespace is
/// populated only by a [`ScopedLookup`], never by frontmatter.
pub fn is_local_path(path: &str) -> bool {
    root_of(path) == LOCAL_ROOT
}

impl EvaluationLookup for ScopedLookup<'_> {
    fn get(&self, path: &str) -> Option<Value> {
        self.resolve(path).ok().and_then(ResolvedBinding::into_value)
    }

    fn resolve(&self, path: &str) -> Result<ResolvedBinding, ExpressionError> {
        match self.local(path) {
            Some(binding) => Ok(binding),
            None => self.inner.resolve(path),
        }
    }

    fn binding_view(&self) -> Option<&BindingView> {
        self.inner.binding_view()
    }

    /// A local is formatted without the inner lookup's name coercion, which is
    /// keyed by document path spelling.
    fn format_resolved(&self, path: &str, value: &Value) -> String {
        match self.local(path) {
            Some(ResolvedBinding::Local { .. }) => super::default_format(value),
            _ => self.inner.format_resolved(path, value),
        }
    }

    fn get_string(&self, path: &str) -> String {
        match self.local(path) {
            Some(ResolvedBinding::Local { value, .. }) => super::default_format(&value),
            Some(_) => String::new(),
            None => self.inner.get_string(path),
        }
    }

    fn resolution_context(&self) -> Option<ResolutionContext> {
        self.inner.resolution_context()
    }

    fn resolution_context_ref(&self) -> Option<&ResolutionContext> {
        self.inner.resolution_context_ref()
    }

    fn is_valid_context_variable(&self, name: &str) -> bool {
        self.inner.is_valid_context_variable(name)
    }

    fn context_variable_names(&self) -> &[&'static str] {
        self.inner.context_variable_names()
    }

    fn begin_expression_scope(&self) {
        self.inner.begin_expression_scope();
    }

    fn lexical_scope(&self) -> Option<&Arc<LexicalScope>> {
        Some(&self.scope)
    }
}

#[cfg(test)]
mod lexical_tests {
    use super::*;
    use crate::markdown::compose::expression::reserved_root_descriptors;
    use serde_json::json;
    use std::collections::HashMap;

    struct Doc(HashMap<String, Value>);

    impl EvaluationLookup for Doc {
        fn get(&self, path: &str) -> Option<Value> {
            self.0.get(path).cloned()
        }
    }

    fn doc() -> Doc {
        Doc([("x".to_string(), json!("document")), ("y".to_string(), json!(1))].into())
    }

    #[test]
    fn implicit_and_explicit_reads_share_one_identity() {
        let doc = doc();
        let scope = LexicalScope::new(vec![("x".into(), json!({ "a": 1 }))]);
        let lookup = ScopedLookup::new(&doc, scope);
        let implicit = lookup.resolve("x.a").unwrap();
        let explicit = lookup.resolve("local.x.a").unwrap();
        assert_eq!(implicit, explicit);
        assert!(matches!(implicit, ResolvedBinding::Local { ref value, .. } if *value == json!(1)));
    }

    #[test]
    fn a_null_local_shadows_the_document_and_missing_members_do_not_fall_through() {
        let doc = doc();
        let scope = LexicalScope::new(vec![("x".into(), Value::Null)]);
        let lookup = ScopedLookup::new(&doc, scope);
        assert_eq!(lookup.get("x"), Some(Value::Null));
        assert_eq!(lookup.get("x.missing"), Some(Value::Null));
        assert_eq!(lookup.get("y"), Some(json!(1)));
        assert_eq!(lookup.resolve("fm.x").unwrap().into_value(), None);
    }

    #[test]
    fn nested_frames_shadow_and_siblings_stay_isolated() {
        let doc = doc();
        let outer = LexicalScope::new(vec![("x".into(), json!("outer"))]);
        let inner = outer.child(vec![("x".into(), json!("inner"))]);
        let sibling = outer.child(vec![("z".into(), json!(true))]);
        let inner_lookup = ScopedLookup::new(&doc, Arc::clone(&inner));
        assert_eq!(inner_lookup.get("x"), Some(json!("inner")));
        let sibling_lookup = ScopedLookup::new(&doc, sibling);
        assert_eq!(sibling_lookup.get("x"), Some(json!("outer")));
        assert_eq!(sibling_lookup.get("z"), Some(json!(true)));
        assert_ne!(inner.lookup("x").unwrap().0, outer.lookup("x").unwrap().0);
    }

    #[test]
    fn explicit_and_implicit_reads_share_an_id_and_fm_stays_a_document_read() {
        let doc = Doc([("fm.x".to_string(), json!("fm-value"))].into());
        let scope = LexicalScope::new(vec![("x".into(), json!(1))]);
        let lookup = ScopedLookup::new(&doc, scope);
        let (implicit, explicit) = (lookup.resolve("x").unwrap(), lookup.resolve("local.x").unwrap());
        let (ResolvedBinding::Local { id: left, .. }, ResolvedBinding::Local { id: right, .. }) = (&implicit, &explicit)
        else {
            panic!("both are locals: {implicit:?} {explicit:?}");
        };
        assert_eq!(left, right);
        assert_eq!(lookup.resolve("fm.x").unwrap().into_value(), Some(json!("fm-value")));
    }

    #[test]
    fn local_with_no_such_binding_is_absent_and_never_reads_the_document() {
        let doc = Doc([("local".to_string(), json!("doc")), ("local.y".to_string(), json!("doc-y"))].into());
        let scope = LexicalScope::new(vec![("x".into(), json!(1))]);
        let lookup = ScopedLookup::new(&doc, scope);
        for path in ["local", "local.y", "local.x.missing.deeper"] {
            let value = lookup.resolve(path).unwrap().into_value();
            assert!(matches!(value, None | Some(Value::Null)), "{path}: {value:?}");
        }
        assert_eq!(lookup.get("local.y"), None);
    }

    #[test]
    fn a_missing_member_of_a_present_local_does_not_read_the_document() {
        let doc = Doc([("x.a".to_string(), json!("doc-a"))].into());
        let scope = LexicalScope::new(vec![("x".into(), json!({ "b": 1 }))]);
        let lookup = ScopedLookup::new(&doc, scope);
        assert_eq!(lookup.get("x.a"), Some(Value::Null));
        assert_eq!(lookup.get("local.x.a"), Some(Value::Null));
    }

    #[test]
    fn nested_scopes_restore_the_outer_meaning_when_left() {
        let doc = doc();
        let outer = LexicalScope::new(vec![("x".into(), json!("outer")), ("keep".into(), json!(7))]);
        {
            let inner = outer.child(vec![("x".into(), json!("inner"))]);
            let inner_lookup = ScopedLookup::new(&doc, inner);
            assert_eq!(inner_lookup.get("x"), Some(json!("inner")));
            assert_eq!(inner_lookup.get("local.x"), Some(json!("inner")));
            assert_eq!(inner_lookup.get("keep"), Some(json!(7)), "an unshadowed outer local stays visible");
        }
        let outer_lookup = ScopedLookup::new(&doc, outer);
        assert_eq!(outer_lookup.get("x"), Some(json!("outer")));
    }

    #[test]
    fn a_retained_frame_outlives_its_creator() {
        let parent = LexicalScope::new(vec![("p".into(), json!("parent"))]);
        let retained = {
            let creator = parent.child_with_origins(vec![("item".into(), json!("kept"), OverrideOrigin::Authored)]);
            drop(parent);
            creator.child(vec![("i".into(), json!(0))])
        };
        let found = retained.resolve("item").expect("the grandparent frame is still alive");
        assert_eq!((found.value, found.origin), (&json!("kept"), OverrideOrigin::Authored));
        assert_eq!(retained.resolve("p").unwrap().value, &json!("parent"));
        assert_eq!(retained.names(), ["i", "item", "p"]);
        let doc = doc();
        assert_eq!(ScopedLookup::new(&doc, retained).get("p"), Some(json!("parent")));
    }

    #[test]
    fn a_local_read_carries_its_origin_and_formats_without_name_coercion() {
        let doc = doc();
        let scope = LexicalScope::with_origins(vec![("x".into(), json!({ "a": "t" }), OverrideOrigin::Authored)]);
        let lookup = ScopedLookup::new(&doc, scope);
        assert!(matches!(
            lookup.resolve("x.a").unwrap(),
            ResolvedBinding::Local { origin: OverrideOrigin::Authored, .. }
        ));
        assert_eq!(lookup.get_string("x.a"), "t");
        assert_eq!(lookup.get_string("local.missing"), "");
    }

    #[test]
    fn locals_never_shadow_a_reserved_root_or_a_registered_global() {
        let view = Arc::new(
            BindingView::builder(ScopeId::new("lexical"))
                .declare("seq", "sequence", Availability::Available)
                .build()
                .unwrap(),
        );
        let mut data: HashMap<String, Value> = HashMap::new();
        for descriptor in reserved_root_descriptors() {
            data.insert(format!("{}.k", descriptor.name), json!(format!("namespace-{}", descriptor.name)));
        }
        let document = Doc(data);
        let session = EvaluationSession::associate(
            view,
            &document,
            [("seq".to_string(), RuntimeBinding::eager(json!({ "k": "global" })))],
        )
        .unwrap();
        let mut bindings: Vec<(String, Value)> = reserved_root_descriptors()
            .iter()
            .filter(|descriptor| descriptor.name != LOCAL_ROOT)
            .map(|descriptor| (descriptor.name.to_string(), json!({ "k": "local" })))
            .collect();
        bindings.push(("seq".into(), json!({ "k": "local" })));
        let lookup = ScopedLookup::new(&session, LexicalScope::new(bindings));

        for descriptor in reserved_root_descriptors().iter().filter(|descriptor| descriptor.name != LOCAL_ROOT) {
            let read = lookup.resolve(&format!("{}.k", descriptor.name)).unwrap();
            assert!(
                matches!(&read, ResolvedBinding::Namespace { .. }),
                "{}: a local shadowed the reserved root: {read:?}",
                descriptor.name
            );
            assert_eq!(read.into_value(), Some(json!(format!("namespace-{}", descriptor.name))));
        }
        assert!(matches!(lookup.resolve("seq.k").unwrap(), ResolvedBinding::Global { .. }));
        assert_eq!(lookup.get("seq.k"), Some(json!("global")));
    }

    #[test]
    fn pattern_names_that_collide_with_protected_names_are_rejected() {
        let view = BindingView::builder(ScopeId::new("lexical"))
            .declare("seq", "sequence", Availability::Available)
            .build()
            .unwrap();
        for descriptor in reserved_root_descriptors() {
            let error = check_local_names([descriptor.name], None).unwrap_err();
            assert_eq!((error.name.as_str(), error.kind), (descriptor.name, ProtectedNameKind::ReservedRoot));
        }
        assert_eq!(check_local_names(["a", "null"], None).unwrap_err().kind, ProtectedNameKind::NullLiteral);
        assert_eq!(check_local_names(["seq"], Some(&view)).unwrap_err().kind, ProtectedNameKind::HostGlobal);
        // Only the actual view's globals are protected; shadowing a document key is fine.
        assert!(check_local_names(["seq", "title", "item"], None).is_ok());
        assert!(check_local_names(["title", "item"], Some(&view)).is_ok());
    }

    #[test]
    fn a_host_cannot_register_local_and_other_globals_are_unchanged() {
        let rejected = BindingView::builder(ScopeId::new("lexical"))
            .declare("local", "shadow", Availability::Available)
            .build()
            .unwrap_err();
        assert_eq!(rejected, BindingError::ReservedName { root: "local".into() });

        let view = Arc::new(
            BindingView::builder(ScopeId::new("lexical"))
                .declare("seq", "sequence", Availability::Available)
                .declare("gone", "unavailable", Availability::Unavailable(UnavailabilityReason::new("host.gone")))
                .build()
                .unwrap(),
        );
        let document = doc();
        let association = EvaluationSession::associate(
            Arc::clone(&view),
            &document,
            [("local".to_string(), RuntimeBinding::eager(json!(1)))],
        );
        assert_eq!(association.unwrap_err(), BindingError::ReservedName { root: "local".into() });

        let session = EvaluationSession::associate(
            view,
            &document,
            [
                ("seq".to_string(), RuntimeBinding::eager(json!(3))),
                ("gone".to_string(), RuntimeBinding::unavailable(UnavailabilityReason::new("host.gone"))),
            ],
        )
        .unwrap();
        assert_eq!(session.get("seq"), Some(json!(3)));
        assert!(matches!(session.resolve("gone"), Err(ExpressionError::Binding(_))));
        let scoped = ScopedLookup::new(&session, LexicalScope::new(vec![("gone".into(), json!("local"))]));
        assert!(matches!(scoped.resolve("gone"), Err(ExpressionError::Binding(_))));
    }

    #[test]
    fn a_session_forwards_the_scope_of_the_lookup_it_wraps() {
        let document = doc();
        let scoped = ScopedLookup::new(&document, LexicalScope::new(vec![("x".into(), json!(1))]));
        let view = Arc::new(BindingView::builder(ScopeId::new("lexical")).build().unwrap());
        let session = EvaluationSession::associate(view, &scoped, []).unwrap();
        assert!(session.lexical_scope().is_some());
        assert_eq!(session.get("local.x"), Some(json!(1)));
    }
}
