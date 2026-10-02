//! Host binding model: how a bare expression root is classified and resolved.
//!
//! A variable root is exactly one of:
//!
//! - a **reserved namespace** (`doc`, `ctx`, `env`, `current`, `current_env`;
//!   the set is [`reserved_root_descriptors`]);
//! - a **global** a host registered for the current scope, available (its
//!   value may be `null`) or unavailable (reading it is a typed
//!   [`BindingError::Unavailable`]);
//! - otherwise a **document property**, present or absent (absent evaluates to
//!   `null`).
//!
//! A host declares its globals once, in an immutable, provider-free
//! [`BindingView`]. Before anything is evaluated it supplies one
//! [`RuntimeBinding`] per declared global, and [`EvaluationSession::associate`]
//! checks the complete registration against the view. Passive validation
//! ([`validate_prepared`](super::prepared::validate_prepared)) reads the same
//! view without runtime values.

use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use serde_json::Value;

use super::catalog::reserved_root_descriptors;
use super::{EvaluationLookup, ExpressionError, ResolutionContext};
use crate::markdown::span::SourceSpan;

/// How one variable path resolved.
///
/// `Document` and `Namespace` carry `None` when the path is absent; the
/// evaluator reads that as `null`. A `Global` always has a value: an available
/// global whose value is `null`, or whose root lacks the requested member,
/// stays a `Global` holding `null` and never falls through to the document.
#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedBinding {
    /// A document property, read bare or through `doc.`'s owner.
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
            Self::Global { value } => Some(value),
        }
    }

    /// Consumes the binding, returning [`value`](Self::value).
    pub fn into_value(self) -> Option<Value> {
        match self {
            Self::Document { value } | Self::Namespace { value } => value,
            Self::Global { value } => Some(value),
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
        Ok(Self {
            view,
            document,
            runtime: entries,
            cache: Mutex::new(HashMap::new()),
            resolution_context: None,
        })
    }

    /// Supplies the read-side functions' resolution context, in place of the
    /// document lookup's own.
    #[must_use]
    pub fn with_resolution_context(mut self, resolution_context: ResolutionContext) -> Self {
        self.resolution_context = Some(resolution_context);
        self
    }

    /// The view this session was associated against.
    pub fn view(&self) -> &BindingView {
        &self.view
    }

    /// Reads an available global's `root`, running a lazy provider on first
    /// read. The cache lock is never held while the provider runs.
    fn global_root(&self, root: &str, binding: &RuntimeBinding<'a>, rest: &str) -> Value {
        match binding {
            RuntimeBinding::Eager(value) => project(value, rest),
            RuntimeBinding::Lazy(provider) => {
                if let Some(cached) = self.lock_cache().get(root) {
                    return project(cached, rest);
                }
                let value = provider();
                let mut cache = self.lock_cache();
                // A re-entrant provider may have filled the slot first; the
                // first value wins so every read in the session agrees.
                let cached = cache.entry(root.to_string()).or_insert(value);
                project(cached, rest)
            }
            RuntimeBinding::Unavailable(_) => Value::Null,
        }
    }

    /// The cache only ever gains complete entries, so a poisoned lock still
    /// guards consistent data.
    fn lock_cache(&self) -> MutexGuard<'_, HashMap<String, Value>> {
        self.cache.lock().unwrap_or_else(PoisonError::into_inner)
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
                Err(ExpressionError::Binding(Box::new(BindingError::Unavailable(Box::new(
                    UnavailableBinding {
                        root: root.to_string(),
                        path: path.to_string(),
                        scope: self.view.scope.clone(),
                        reason: reason.clone(),
                        span: None,
                    },
                )))))
            }
            Some(binding) => Ok(ResolvedBinding::Global {
                value: self.global_root(root, binding, &path[root.len()..]),
            }),
            None => self.document.resolve(path),
        }
    }

    fn binding_view(&self) -> Option<&BindingView> {
        Some(&self.view)
    }

    fn format_resolved(&self, path: &str, value: &Value) -> String {
        let root = root_of(path);
        if !is_reserved_namespace(root) && self.runtime.contains_key(root) {
            super::default_format(value)
        } else {
            self.document.format_resolved(path, value)
        }
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
}
