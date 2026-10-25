//! Host expression functions: functions a host such as Claudine adds to the
//! expression language, registered through the same [`RegistrationBuilder`]
//! the built-ins use and checked, narrowed, and dispatched exactly like them.
//!
//! A host declares each function's signatures in its own function catalog
//! (a tagged `kind: schema` document with `categories:` and `function(...)`
//! `types:`, loaded by [`HostFunctionCatalog::load`] with the built-in
//! catalog's strict loader) and registers, per function, the behavior facts a
//! signature cannot express and a pure handler ([`HostFunctionsBuilder`]).
//! [`HostFunctionsBuilder::build`] validates the whole set before anything is
//! evaluated.
//!
//! Two ways to make a set visible to dispatch and analysis:
//!
//! - [`install_host_functions`]: process-wide, for a runtime host.
//! - [`HostFunctions::enter`]: for the current thread only, while the
//!   returned guard lives. The editor (DMLS) enters the analysis-only set
//!   built from each document's own schema roots around its diagnostics.
//!
//! Lookup order everywhere is: thread-local test functions, the current
//! thread's entered sets, installed sets, then the built-ins.
//!
//! A separately running editor cannot run the host's Rust registration, so
//! the host writes its facts into a companion artifact
//! ([`HostFunctions::facts_artifact`]) and the editor rebuilds an
//! analysis-only set from the catalog plus that artifact
//! ([`HostFunctions::analysis_only`]). A function whose facts cannot be
//! trusted is *incomplete*: it dispatches (no unknown-function error) but
//! analysis knows nothing about it (no category, no narrowing), and
//! [`incomplete_host_calls`] reports each call so the editor can say so.
//!
//! ## Examples
//!
//! ```
//! use std::sync::Arc;
//! use darkmatter::markdown::compose::expression::host::{HostFunctionCatalog, HostFunctions};
//!
//! let catalog = HostFunctionCatalog::load(
//!     "kind: schema\ncategories:\n  - Project\ntypes:\n  twice: |-\n    function(parameters(x: number); returns(number); category(Project))\n",
//!     "host/functions.yaml",
//! )
//! .unwrap();
//! let functions = HostFunctions::builder(catalog)
//!     .function("twice", &[], |registration| registration, Arc::new(|call| {
//!         Ok(serde_json::json!(call.args().number(0).unwrap() * 2.0))
//!     }))
//!     .build()
//!     .unwrap();
//! assert!(functions.facts_artifact().contains("\"twice\""));
//! ```

pub(crate) mod artifact;

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, PoisonError, RwLock};

use serde_json::Value;

pub use artifact::{FACTS_ARTIFACT_FORMAT_VERSION, FactsArtifact, NON_NULL_FACT_TYPE, facts_artifact_path};

use super::ast::{SpannedExpr, SpannedExprKind};
use super::call_binding::{BoundCall, EvaluationMode, FunctionRegistration, RegistrationBuilder, RegistrationError};
use super::catalog::{FunctionCatalog, load_catalog};
use crate::markdown::span::SourceSpan;

/// The diagnostic code for a call to a host function whose analysis
/// metadata is incomplete.
pub const INCOMPLETE_ANALYSIS_CODE: &str = "dm.expression.incomplete_analysis";

/// A host function's handler: computes from the bound call; an `Err` is a
/// domain failure. It must be pure: analysis never calls it, and the runtime
/// calls it only for an evaluated call.
pub type HostHandler = Arc<dyn Fn(&BoundCall<'_>) -> Result<Value, String> + Send + Sync>;

/// A host's function catalog: its signatures, its path, and its version.
///
/// Cloning shares the loaded catalog.
#[derive(Clone)]
pub struct HostFunctionCatalog {
    inner: Arc<CatalogData>,
}

struct CatalogData {
    path: PathBuf,
    version: String,
    catalog: FunctionCatalog,
}

impl fmt::Debug for HostFunctionCatalog {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HostFunctionCatalog")
            .field("path", &self.inner.path)
            .field("version", &self.inner.version)
            .finish_non_exhaustive()
    }
}

/// Why a host function catalog does not load.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
pub struct HostCatalogError {
    message: String,
}

impl HostFunctionCatalog {
    /// Loads a function catalog with the built-in catalog's strict loader.
    /// `path` names the file in errors and locates its facts artifact
    /// ([`facts_artifact_path`]). File-form examples (`example(./x.yaml)`)
    /// are not available to a host catalog and fail the load.
    ///
    /// The loaded descriptors live for the rest of the process; load each
    /// catalog version once.
    ///
    /// ## Errors
    ///
    /// The loader's message, naming `path`, for a document that is not a
    /// tagged catalog or whose entries do not load.
    pub fn load(source: &str, path: impl AsRef<Path>) -> Result<Self, HostCatalogError> {
        let path = path.as_ref().to_path_buf();
        let display = path.display().to_string();
        let catalog = load_catalog(source, &display, &[]).map_err(|error| HostCatalogError {
            message: error.to_string(),
        })?;
        Ok(Self {
            inner: Arc::new(CatalogData {
                path,
                // The artifact's `catalog_hash`: BLAKE3 over the raw bytes.
                version: biscuit_hash::blake3_hash(source),
                catalog,
            }),
        })
    }

    pub fn path(&self) -> &Path {
        &self.inner.path
    }

    /// The BLAKE3 hex digest of the catalog's raw bytes; a facts artifact
    /// records it as `catalog_hash`.
    pub fn version(&self) -> &str {
        &self.inner.version
    }

    /// Every function's canonical name, in documentation order.
    pub fn function_names(&self) -> Vec<&'static str> {
        let mut names: Vec<&'static str> = Vec::new();
        for descriptor in self.inner.catalog.descriptors {
            if !names.contains(&descriptor.name) {
                names.push(descriptor.name);
            }
        }
        names
    }

    /// A registration builder over `name`'s catalog overloads; `None` when
    /// the catalog does not declare `name`.
    fn registration_builder(&self, name: &str) -> Option<RegistrationBuilder> {
        let overloads: Vec<_> = self
            .inner
            .catalog
            .descriptors
            .iter()
            .filter(|descriptor| descriptor.name == name)
            .map(|descriptor| descriptor.function_type.clone())
            .collect();
        (!overloads.is_empty()).then(|| FunctionRegistration::builder_from_types(name, overloads))
    }

    fn build(&self, builder: RegistrationBuilder) -> Result<FunctionRegistration, RegistrationError> {
        builder.build_with(&self.inner.catalog.type_resolver())
    }
}

/// Why a set of host functions cannot be registered or installed. Every
/// variant is reported before any evaluation.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum HostFunctionError {
    #[error("host function name `{name}` is registered twice (as a canonical name or alias)")]
    DuplicateName { name: String },
    #[error("host function name `{name}` collides with a built-in function's canonical name or alias")]
    BuiltinCollision { name: String },
    #[error("host function name `{name}` collides with an already-installed host function")]
    InstalledCollision { name: String },
    #[error("catalog function `{function}` has no registration")]
    MissingRegistration { function: String },
    #[error("registered function `{function}` is not declared in the catalog")]
    MissingCatalogFunction { function: String },
    #[error("host function `{function}` is registered as lazy; host functions are evaluated eagerly")]
    LazyFunction { function: String },
    #[error(transparent)]
    Registration(#[from] RegistrationError),
}

type Configure = Box<dyn FnOnce(RegistrationBuilder) -> RegistrationBuilder>;

struct PendingFunction {
    canonical: String,
    aliases: Vec<String>,
    configure: Configure,
    handler: HostHandler,
}

/// Collects a host's registrations; [`HostFunctionsBuilder::build`]
/// validates them against the catalog and the built-ins.
pub struct HostFunctionsBuilder {
    catalog: HostFunctionCatalog,
    pending: Vec<PendingFunction>,
}

impl HostFunctionsBuilder {
    /// Registers `canonical` (its catalog name) with `aliases`. `configure`
    /// adds the registration facts to the builder over the catalog's
    /// signatures ([`RegistrationBuilder::narrows`],
    /// [`RegistrationBuilder::reject_sources`],
    /// [`RegistrationBuilder::numeric_recovery`],
    /// [`RegistrationBuilder::false_for_malformed_reference`]).
    pub fn function(
        mut self,
        canonical: impl Into<String>,
        aliases: &[&str],
        configure: impl FnOnce(RegistrationBuilder) -> RegistrationBuilder + 'static,
        handler: HostHandler,
    ) -> Self {
        self.pending.push(PendingFunction {
            canonical: canonical.into(),
            aliases: aliases.iter().map(|alias| (*alias).to_string()).collect(),
            configure: Box::new(configure),
            handler,
        });
        self
    }

    /// Compiles and validates every registration.
    ///
    /// ## Errors
    ///
    /// A [`HostFunctionError`]: a name or alias used twice in the set or
    /// colliding with a built-in, a catalog function without a registration
    /// or a registration without a catalog function, a lazy registration, or
    /// any [`RegistrationError`] (a fact on a parameter no overload declares,
    /// a narrowing fact on a function that does not return `boolean`, …).
    pub fn build(self) -> Result<HostFunctions, HostFunctionError> {
        let catalog_names = self.catalog.function_names();
        validate_names(self.pending.iter().map(|pending| (pending.canonical.as_str(), pending.aliases.as_slice())))?;
        for name in &catalog_names {
            if !self.pending.iter().any(|pending| pending.canonical == *name) {
                return Err(HostFunctionError::MissingRegistration {
                    function: (*name).to_string(),
                });
            }
        }
        let mut functions = Vec::with_capacity(self.pending.len());
        for pending in self.pending {
            let Some(builder) = self.catalog.registration_builder(&pending.canonical) else {
                return Err(HostFunctionError::MissingCatalogFunction {
                    function: pending.canonical,
                });
            };
            let registration = self.catalog.build((pending.configure)(builder))?;
            if registration.evaluation() == EvaluationMode::Lazy {
                return Err(HostFunctionError::LazyFunction {
                    function: pending.canonical,
                });
            }
            functions.push(HostFunction {
                canonical: pending.canonical,
                aliases: pending.aliases,
                state: FunctionState::Complete {
                    registration: Box::new(registration),
                    handler: Some(pending.handler),
                },
            });
        }
        Ok(HostFunctions::new(self.catalog, functions))
    }
}

/// Rejects a canonical name or alias used twice in the set, or used by a
/// built-in. Names compare lowercased, as dispatch resolves them.
fn validate_names<'a>(
    functions: impl IntoIterator<Item = (&'a str, &'a [String])>,
) -> Result<(), HostFunctionError> {
    let mut seen = std::collections::HashSet::new();
    for (canonical, aliases) in functions {
        for name in std::iter::once(canonical).chain(aliases.iter().map(String::as_str)) {
            let lower = name.to_ascii_lowercase();
            if super::functions::is_builtin_name(&lower) {
                return Err(HostFunctionError::BuiltinCollision { name: name.to_string() });
            }
            if !seen.insert(lower) {
                return Err(HostFunctionError::DuplicateName { name: name.to_string() });
            }
        }
    }
    Ok(())
}

pub(crate) enum FunctionState {
    /// Facts known; `handler` is `None` in an analysis-only set.
    Complete {
        registration: Box<FunctionRegistration>,
        handler: Option<HostHandler>,
    },
    /// Dispatchable, but analysis knows nothing: no category, no narrowing.
    Incomplete { reason: String },
}

pub(crate) struct HostFunction {
    canonical: String,
    aliases: Vec<String>,
    state: FunctionState,
}

struct HostSet {
    catalog: HostFunctionCatalog,
    functions: Vec<HostFunction>,
    /// Lowercased canonical names and aliases to an index in `functions`.
    names: HashMap<String, usize>,
}

/// A validated set of host functions over one catalog. Cloning shares it.
#[derive(Clone)]
pub struct HostFunctions {
    inner: Arc<HostSet>,
}

impl fmt::Debug for HostFunctions {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HostFunctions")
            .field("catalog", &self.inner.catalog)
            .field("functions", &self.function_names())
            .finish_non_exhaustive()
    }
}

impl HostFunctions {
    /// Starts registering the functions `catalog` declares.
    pub fn builder(catalog: HostFunctionCatalog) -> HostFunctionsBuilder {
        HostFunctionsBuilder {
            catalog,
            pending: Vec::new(),
        }
    }

    fn new(catalog: HostFunctionCatalog, functions: Vec<HostFunction>) -> Self {
        let mut names = HashMap::new();
        for (index, function) in functions.iter().enumerate() {
            for name in std::iter::once(&function.canonical).chain(&function.aliases) {
                names.insert(name.to_ascii_lowercase(), index);
            }
        }
        Self {
            inner: Arc::new(HostSet {
                catalog,
                functions,
                names,
            }),
        }
    }

    pub fn catalog(&self) -> &HostFunctionCatalog {
        &self.inner.catalog
    }

    /// Every canonical name in the set.
    pub fn function_names(&self) -> Vec<&str> {
        self.inner.functions.iter().map(|function| function.canonical.as_str()).collect()
    }

    /// The incomplete functions and why each is incomplete.
    pub fn incomplete_functions(&self) -> Vec<(&str, &str)> {
        self.inner
            .functions
            .iter()
            .filter_map(|function| match &function.state {
                FunctionState::Incomplete { reason } => Some((function.canonical.as_str(), reason.as_str())),
                FunctionState::Complete { .. } => None,
            })
            .collect()
    }

    /// The registration analysis reads for `name` (canonical or alias,
    /// case-insensitive); `None` for an incomplete or unknown function.
    pub fn registration(&self, name: &str) -> Option<&FunctionRegistration> {
        let index = self.inner.names.get(&name.to_ascii_lowercase())?;
        match &self.inner.functions[*index].state {
            FunctionState::Complete { registration, .. } => Some(registration),
            FunctionState::Incomplete { .. } => None,
        }
    }

    /// Makes the set visible to dispatch and analysis on the current thread
    /// until the guard drops, ahead of installed sets and built-ins; a set
    /// entered later is consulted first.
    pub fn enter(&self) -> HostFunctionScope {
        SCOPED.with(|scoped| scoped.borrow_mut().push(self.inner.clone()));
        HostFunctionScope {
            set: self.inner.clone(),
            _thread: PhantomData,
        }
    }
}

/// Leaves the current thread's view of a [`HostFunctions`] set when dropped.
#[must_use = "the host functions leave the thread's scope when the guard drops"]
pub struct HostFunctionScope {
    set: Arc<HostSet>,
    /// The scope is thread-local, so the guard stays on its thread.
    _thread: PhantomData<*const ()>,
}

impl Drop for HostFunctionScope {
    fn drop(&mut self) {
        SCOPED.with(|scoped| {
            let mut scoped = scoped.borrow_mut();
            if let Some(position) = scoped.iter().rposition(|set| Arc::ptr_eq(set, &self.set)) {
                scoped.remove(position);
            }
        });
    }
}

thread_local! {
    static SCOPED: RefCell<Vec<Arc<HostSet>>> = const { RefCell::new(Vec::new()) };
}

static INSTALLED: RwLock<Vec<Arc<HostSet>>> = RwLock::new(Vec::new());
/// Spares every lookup the lock while nothing is installed.
static ANY_INSTALLED: AtomicBool = AtomicBool::new(false);

/// Installs `functions` for every thread of the process, for the rest of
/// its life. Installing is additive: several sets may be installed.
///
/// ## Errors
///
/// [`HostFunctionError::InstalledCollision`] when a name or alias of
/// `functions` is already installed; nothing is installed then.
pub fn install_host_functions(functions: HostFunctions) -> Result<(), HostFunctionError> {
    let mut installed = INSTALLED.write().unwrap_or_else(PoisonError::into_inner);
    for name in functions.inner.names.keys() {
        if installed.iter().any(|set| set.names.contains_key(name)) {
            return Err(HostFunctionError::InstalledCollision { name: name.clone() });
        }
    }
    installed.push(functions.inner);
    ANY_INSTALLED.store(true, Ordering::Release);
    Ok(())
}

/// One host function a name resolved to.
pub(crate) struct HostEntry {
    set: Arc<HostSet>,
    index: usize,
}

impl HostEntry {
    fn function(&self) -> &HostFunction {
        &self.set.functions[self.index]
    }

    pub(crate) fn canonical(&self) -> &str {
        &self.function().canonical
    }

    pub(crate) fn state(&self) -> &FunctionState {
        &self.function().state
    }
}

/// The host function the lowercased `name` resolves to: the current
/// thread's entered sets (latest first), then installed sets.
pub(crate) fn lookup(name: &str) -> Option<HostEntry> {
    let scoped = SCOPED.with(|scoped| {
        scoped.borrow().iter().rev().find_map(|set| {
            set.names.get(name).map(|index| HostEntry {
                set: set.clone(),
                index: *index,
            })
        })
    });
    if scoped.is_some() || !ANY_INSTALLED.load(Ordering::Acquire) {
        return scoped;
    }
    let installed = INSTALLED.read().unwrap_or_else(PoisonError::into_inner);
    installed.iter().find_map(|set| {
        set.names.get(name).map(|index| HostEntry {
            set: set.clone(),
            index: *index,
        })
    })
}

/// A call to a host function whose analysis metadata is incomplete.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncompleteHostCall {
    /// The name as written in the call.
    pub function: String,
    /// The whole call's span.
    pub span: SourceSpan,
    /// Why the function's metadata is incomplete.
    pub reason: String,
}

impl IncompleteHostCall {
    /// The editor message for this call.
    pub fn message(&self) -> String {
        format!(
            "analysis of host function `{}` is incomplete: {}; no argument category or narrowing is known for this call",
            self.function, self.reason
        )
    }
}

/// Every call in `expr`, in every branch, to a host function that is
/// incomplete in the current scope (entered and installed sets).
pub fn incomplete_host_calls(expr: &SpannedExpr) -> Vec<IncompleteHostCall> {
    let mut found = Vec::new();
    super::call_check::visit_call_nodes(expr, &mut |call| {
        let SpannedExprKind::FunctionCall { name, .. } = &call.kind else {
            return;
        };
        let lower = name.to_ascii_lowercase();
        #[cfg(any(test, feature = "effects-instrumentation"))]
        if super::functions::test_hooks::lookup(&lower).is_some() {
            return;
        }
        if let Some(entry) = lookup(&lower)
            && let FunctionState::Incomplete { reason } = entry.state()
        {
            found.push(IncompleteHostCall {
                function: name.clone(),
                span: call.span.clone(),
                reason: reason.clone(),
            });
        }
    });
    found
}

