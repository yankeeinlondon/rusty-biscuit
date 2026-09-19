//! The request's refresh authority behind the lazy `current` and `current_env`
//! roots (spec R30–R33).
//!
//! `ctx` is captured once at the start of a request; `current` is the same key
//! set observed when a reference is evaluated. The two are kept apart on
//! purpose: a `current.*` read never adds an eager capture requirement, and an
//! eager capture never answers a `current.*` read.
//!
//! Three pieces make that work:
//!
//! - [`CurrentProvider`] — the invocation's capability to observe **one**
//!   cataloged key now. `md compose` installs [`AnchoredRefresh`], which
//!   answers repository facts from the request's retained observation and
//!   re-captures every mutable group at the context's retained anchor; an
//!   embedder such as Claudine installs one built from its own launch
//!   evidence. A provider that does not hold a capability answers
//!   [`CurrentRefresh::Unsupported`] and the read fails closed with a
//!   `PartialRuntimeCapture` diagnostic — no provider ever falls back to
//!   ambient discovery (decision D5).
//! - [`CurrentAuthority`] — the request-owned handle carried on
//!   `ComposeOptions` and cloned into every lookup, holding the provider and
//!   the shared diagnostic sink. Child pipelines share the provider; they never
//!   share an observation.
//! - [`CurrentScope`] — the per-lookup memo. Q2 fixes the memo scope at **one
//!   expression evaluation, per key**: repeated reads of `current.branch`
//!   inside a single `{{ … }}` span, `when=` condition, or `$()` branch agree,
//!   while the next expression observes the fact afresh.
//!
//! Request-owned facts never refresh. The invocation directory and the root
//! document's identity are fixed for the request (decisions D2/D3), so
//! `current.cwd` and `current.id` read the same captured values `ctx` does.
//! The repository root and package topology are fixed too, but by the
//! provider rather than by Darkmatter: an embedder answers them from its
//! launch repository, and the ambient provider from the request's one
//! repository observation, which
//! [`CurrentAuthority::establish_ambient_repository`] fixes when the request
//! is created (`ComposeOptions::for_document`), or at the root pipeline entry
//! for a request built through an older constructor.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

use serde_json::{Map, Value};

use super::ContextMergeDiagnostic;
use super::capture::{ContextGroup, capture_runtime_context_for_groups};
use super::catalog::context_variable_descriptors;
use super::repository_scope::RepositoryObservation;
use super::runtime::ComposeContext;
use crate::markdown::compose::ComposeWarning;
use crate::markdown::compose::expression::ExpressionError;

/// The reserved root mirroring `ctx`.
pub(crate) const CURRENT_ROOT: &str = "current";

/// The reserved root mirroring `env`.
pub(crate) const CURRENT_ENV_ROOT: &str = "current_env";

/// The `area` a fail-closed lazy read reports its partial capture under.
const CURRENT_AREA: &str = "current";

/// The warning stage fail-closed lazy reads are reported under.
const STAGE: &str = "context";

/// What one refresh of a cataloged key produced.
#[derive(Debug, Clone, PartialEq)]
pub enum CurrentRefresh {
    /// The invocation holds the capability; this is the key's value now, in the
    /// same shape `ctx.<key>` projects.
    Observed(Value),
    /// The invocation does not hold the capability for this key. The read fails
    /// closed: `null` plus a `PartialRuntimeCapture` diagnostic, never an
    /// ambient probe and never the stale eager value.
    Unsupported,
}

/// An invocation's capability to observe one mutable fact as it stands now.
///
/// Implementations refresh against the launch roots and evidence authority the
/// request already retains. They must not rediscover the working directory, the
/// repository root, or package topology (decision D3): freshness covers mutable
/// facts such as the branch, the dirty-file set, recent history, and the
/// environment. A `Repo`-group key is answered from the one repository
/// observation the request holds, so `current.repo_root` cannot switch
/// observations mid-compose.
pub trait CurrentProvider: std::fmt::Debug + Send + Sync {
    /// Observes `key` — a cataloged `ctx` variable name — as it stands now.
    fn refresh(&self, key: &str) -> CurrentRefresh;
}

/// The `Repo` group as one Darkmatter-owned request observed it: the projected
/// `ctx` values of every key in that group, plus the root and topology behind
/// them.
#[derive(Debug)]
pub(crate) struct AmbientRepository {
    values: Map<String, Value>,
    observation: Option<Arc<RepositoryObservation>>,
}

impl AmbientRepository {
    /// The eager `Repo` capture of a context that made one.
    fn from_context(context: &ComposeContext) -> Self {
        Self {
            values: ContextGroup::Repo
                .projected_keys()
                .filter_map(|key| Some((key.to_string(), context.get(key)?.clone())))
                .collect(),
            observation: context.observations().repository().cloned(),
        }
    }

    /// One discovery of the `Repo` group at `anchor`.
    fn discover(anchor: &Path) -> Self {
        let (values, _, _, _, observations) =
            capture_runtime_context_for_groups(anchor, &[ContextGroup::Repo]);
        Self {
            values,
            observation: observations.repository().cloned(),
        }
    }

    /// The root and topology behind the projected values.
    pub(crate) fn observation(&self) -> Option<&Arc<RepositoryObservation>> {
        self.observation.as_ref()
    }
}

/// The ambient provider `md compose` installs.
///
/// A `Repo`-group key reads the request's established repository observation
/// and never discovers the repository itself: a read the request never
/// established for fails closed like any unsupplied capability. Every other
/// group is re-captured at the anchor the request's `ComposeContext` retained,
/// so a refresh reads the same directory the eager capture did rather than
/// the process CWD at reference time.
#[derive(Debug, Clone)]
pub(crate) struct AnchoredRefresh {
    anchor: PathBuf,
    /// The request's repository observation, shared through
    /// [`CurrentAuthority`] by every provider the request builds.
    repository: Arc<OnceLock<AmbientRepository>>,
}

impl AnchoredRefresh {
    fn for_request(
        context: &ComposeContext,
        repository: Arc<OnceLock<AmbientRepository>>,
    ) -> Self {
        if context.capture_requirements().contains(ContextGroup::Repo) {
            // A filled memo means the request already established its
            // observation; that one stays, so the eager values do not
            // replace it.
            let _ = repository.set(AmbientRepository::from_context(context));
        }
        Self {
            anchor: context.anchor().to_path_buf(),
            repository,
        }
    }
}

impl CurrentProvider for AnchoredRefresh {
    fn refresh(&self, key: &str) -> CurrentRefresh {
        let Some(group) = ContextGroup::for_key(key) else {
            return CurrentRefresh::Unsupported;
        };
        let value = match group {
            ContextGroup::Repo => self
                .repository
                .get()
                .and_then(|repository| repository.values.get(key).cloned()),
            _ => {
                let (values, ..) = capture_runtime_context_for_groups(&self.anchor, &[group]);
                values.get(key).cloned()
            }
        };
        match value {
            Some(value) => CurrentRefresh::Observed(value),
            None => CurrentRefresh::Unsupported,
        }
    }
}

/// The request's refresh authority: its provider and its diagnostic sink.
///
/// The default holds no provider, so a request that was never given one cannot
/// observe a mutable fact and every `current.<key>` read fails closed. That is
/// the supplied-evidence contract: an embedder that supplies no capability gets
/// `null` and a diagnostic, not a host probe.
#[derive(Clone, Default)]
pub struct CurrentAuthority {
    provider: Option<Arc<dyn CurrentProvider>>,
    diagnostics: Arc<Mutex<Vec<ContextMergeDiagnostic>>>,
    /// The ambient provider's repository observation. It lives on the
    /// request handle rather than the provider because a Darkmatter-owned
    /// request builds a fresh [`AnchoredRefresh`] for every resolution
    /// context it projects; sharing the memo here is what makes "one
    /// discovery per request" hold across them.
    ambient_repository: Arc<OnceLock<AmbientRepository>>,
    /// Discovery mode: observe nothing and record nothing.
    discovering: bool,
}

impl std::fmt::Debug for CurrentAuthority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CurrentAuthority")
            .field("provider", &self.provider)
            .field("discovering", &self.discovering)
            .finish()
    }
}

impl CurrentAuthority {
    /// This authority with `provider` installed, sharing the diagnostic sink.
    #[must_use]
    pub fn with_provider(&self, provider: Arc<dyn CurrentProvider>) -> Self {
        Self {
            provider: Some(provider),
            ..self.clone()
        }
    }

    /// This authority with the ambient provider installed: the
    /// [`AnchoredRefresh`] a Darkmatter-owned request gets when no embedder
    /// supplied a capability, built on `context`'s retained anchor and
    /// repository observation.
    #[must_use]
    pub(crate) fn with_ambient_refresh(&self, context: &ComposeContext) -> Self {
        self.with_provider(Arc::new(AnchoredRefresh::for_request(
            context,
            self.ambient_repository.clone(),
        )))
    }

    /// Establishes the repository observation the ambient provider answers
    /// `Repo`-group keys from.
    ///
    /// The observation is fixed for the request (decision D3): the eager
    /// `Repo` capture when the request's context made one, otherwise one
    /// discovery at the context's retained anchor. It is not gated on whether
    /// any document plans a `Repo`-group `current.*` read: a transcluded
    /// descendant may be the only reader, and it must find the observation
    /// already fixed rather than establish request state late. Once
    /// established it is shared by every provider and child pipeline of the
    /// request, and a later call is a no-op. A discovering authority observes
    /// nothing.
    pub(crate) fn establish_ambient_repository(&self, context: &ComposeContext) {
        if self.discovering || self.ambient_repository.get().is_some() {
            return;
        }
        let repository = if context.capture_requirements().contains(ContextGroup::Repo) {
            AmbientRepository::from_context(context)
        } else {
            AmbientRepository::discover(context.anchor())
        };
        let _ = self.ambient_repository.set(repository);
    }

    /// The request's repository observation, once established.
    pub(crate) fn ambient_repository(&self) -> Option<&AmbientRepository> {
        self.ambient_repository.get()
    }

    /// A passive twin of this authority for pre-flight discovery.
    ///
    /// Discovery evaluates a document's expressions to find its effects, so it
    /// reaches `current.*` and `current_env.*` references that the real compose
    /// pass would observe. Observing them here would make a passive pass probe
    /// the host and reread the environment, so the discovery authority answers
    /// every member `null` and records nothing. Reaching the reference is the
    /// metadata preflight reports (see
    /// [`DeferredCapabilities`](super::capture::DeferredCapabilities)); the
    /// value is not.
    #[must_use]
    pub(crate) fn discovering(&self) -> Self {
        Self {
            discovering: true,
            ..self.clone()
        }
    }

    /// Whether a refresh capability is installed at all.
    pub(crate) fn has_provider(&self) -> bool {
        self.provider.is_some()
    }

    /// Drains the partial-capture diagnostics raised so far in the request.
    pub(crate) fn take_diagnostics(&self) -> Vec<ContextMergeDiagnostic> {
        std::mem::take(&mut *lock(&self.diagnostics))
    }

    /// Drains the diagnostics as rendered compose warnings.
    pub(crate) fn take_warnings(&self) -> Vec<ComposeWarning> {
        self.take_diagnostics()
            .into_iter()
            .map(|diagnostic| match diagnostic {
                ContextMergeDiagnostic::PartialRuntimeCapture { area, detail } => {
                    ComposeWarning::new(STAGE, format!("Partial runtime capture for {area}: {detail}"))
                }
                other => ComposeWarning::new(STAGE, format!("{other:?}")),
            })
            .collect()
    }

    /// Observes `key` now, recording a partial capture when no capability
    /// answers it.
    fn refresh(&self, key: &str) -> Option<Value> {
        if self.discovering {
            return None;
        }
        let refreshed = match &self.provider {
            Some(provider) => provider.refresh(key),
            None => CurrentRefresh::Unsupported,
        };
        match refreshed {
            CurrentRefresh::Observed(value) => Some(value),
            CurrentRefresh::Unsupported => {
                self.record_unsupported(key);
                None
            }
        }
    }

    fn record_unsupported(&self, key: &str) {
        let detail = match self.provider.is_some() {
            true => format!(
                "`current.{key}` was not supplied by this invocation's refresh provider"
            ),
            false => format!(
                "`current.{key}` needs a refresh capability, and this request installed none"
            ),
        };
        let diagnostic = ContextMergeDiagnostic::PartialRuntimeCapture {
            area: CURRENT_AREA,
            detail,
        };
        let mut sink = lock(&self.diagnostics);
        if !sink.contains(&diagnostic) {
            sink.push(diagnostic);
        }
    }
}

/// One expression evaluation's view of the two lazy roots.
///
/// The memo is cleared at every expression boundary
/// ([`EvaluationLookup::begin_expression_scope`]), which is what makes repeated
/// reads of one key inside a single expression agree while the next expression
/// observes the fact afresh (Q2). Cloning a lookup starts an empty memo rather
/// than copying observations.
///
/// [`EvaluationLookup::begin_expression_scope`]: crate::markdown::compose::expression::EvaluationLookup::begin_expression_scope
#[derive(Debug, Default)]
pub(crate) struct CurrentScope {
    authority: CurrentAuthority,
    memo: Mutex<HashMap<String, Value>>,
}

impl Clone for CurrentScope {
    fn clone(&self) -> Self {
        Self::new(self.authority.clone())
    }
}

impl CurrentScope {
    pub(crate) fn new(authority: CurrentAuthority) -> Self {
        Self {
            authority,
            memo: Mutex::new(HashMap::new()),
        }
    }

    /// Discards the memo so the next expression observes the facts afresh.
    pub(crate) fn begin_expression_scope(&self) {
        lock(&self.memo).clear();
    }

    /// Resolves `path` when it addresses a lazy reserved root.
    ///
    /// ## Returns
    ///
    /// `None` when `path` is not under `current` or `current_env`, leaving the
    /// caller's ordinary resolution in charge. Otherwise the reserved root's
    /// answer, which may itself be `Ok(None)` for a member with no value.
    ///
    /// ## Errors
    ///
    /// [`ExpressionError::ReservedRootPathUnknown`] for a path the root has no
    /// member for, which is how the removed `current.ctx.*` / `current.env.*`
    /// nesting fails. A fixed member still reports the eager `ctx` failures
    /// ([`ExpressionError::ContextNotCaptured`] and friends).
    pub(crate) fn resolve(
        &self,
        path: &str,
        context: &ComposeContext,
    ) -> Option<Result<Option<Value>, ExpressionError>> {
        if let Some(rest) = strip_root(path, CURRENT_ROOT) {
            return Some(self.resolve_current(rest, context));
        }
        if let Some(rest) = strip_root(path, CURRENT_ENV_ROOT) {
            return Some(self.resolve_current_env(rest));
        }
        None
    }

    /// Whether `root` is one of the two lazy reserved roots.
    pub(crate) fn is_reserved_root(root: &str) -> bool {
        root == CURRENT_ROOT || root == CURRENT_ENV_ROOT
    }

    fn resolve_current(
        &self,
        rest: Option<&str>,
        context: &ComposeContext,
    ) -> Result<Option<Value>, ExpressionError> {
        let Some(rest) = rest else {
            return Ok(Some(current_enumeration()));
        };
        let (key, tail) = split_first_segment(rest);
        let Some(group) = ContextGroup::for_key(key) else {
            return Err(unknown_path(CURRENT_ROOT, rest));
        };
        if self.authority.discovering {
            return Ok(None);
        }
        let value = match self.memoized(CURRENT_ROOT, key) {
            Some(value) => Some(value),
            None => {
                let observed = match is_request_owned(group) {
                    // Identity, the invocation directory, and the resolution
                    // anchors are fixed for the request even for `current`
                    // (D2/D3), so they read the request's own capture and the
                    // provider is never consulted.
                    true => context.classify_ctx_key(key).into_checked(|| None)?,
                    false => self.authority.refresh(key),
                };
                if let Some(value) = &observed {
                    self.remember(CURRENT_ROOT, key, value.clone());
                }
                observed
            }
        };
        Ok(walk_tail(value, tail))
    }

    fn resolve_current_env(&self, rest: Option<&str>) -> Result<Option<Value>, ExpressionError> {
        let Some(rest) = rest else {
            // `env` has no bare-root value either; the mirror keeps that shape.
            return Ok(None);
        };
        if rest.contains('.') {
            return Err(unknown_path(CURRENT_ENV_ROOT, rest));
        }
        if self.authority.discovering {
            return Ok(None);
        }
        if let Some(value) = self.memoized(CURRENT_ENV_ROOT, rest) {
            return Ok(Some(value));
        }
        // The point of `current_env`: the live process environment at reference
        // time, not the snapshot `env` froze at capture.
        let Ok(value) = std::env::var(rest) else {
            return Ok(None);
        };
        let value = Value::String(value);
        self.remember(CURRENT_ENV_ROOT, rest, value.clone());
        Ok(Some(value))
    }

    fn memoized(&self, root: &str, key: &str) -> Option<Value> {
        lock(&self.memo).get(&memo_key(root, key)).cloned()
    }

    fn remember(&self, root: &str, key: &str, value: Value) {
        lock(&self.memo).insert(memo_key(root, key), value);
    }
}

fn memo_key(root: &str, key: &str) -> String {
    format!("{root}.{key}")
}

/// Whether the request owns `group` outright, so `current` reads it fixed.
fn is_request_owned(group: ContextGroup) -> bool {
    matches!(group, ContextGroup::Invocation | ContextGroup::Document)
}

/// `Some(None)` for the bare root, `Some(Some(rest))` for a member path, `None`
/// when `path` is under a different root.
fn strip_root<'a>(path: &'a str, root: &str) -> Option<Option<&'a str>> {
    if path == root {
        return Some(None);
    }
    path.strip_prefix(root)
        .and_then(|rest| rest.strip_prefix('.'))
        .map(Some)
}

fn split_first_segment(path: &str) -> (&str, Option<&str>) {
    match path.split_once('.') {
        Some((head, tail)) => (head, Some(tail)),
        None => (path, None),
    }
}

fn walk_tail(value: Option<Value>, tail: Option<&str>) -> Option<Value> {
    let Some(tail) = tail else {
        return value;
    };
    let mut current = value?;
    for segment in tail.split('.') {
        current = match current {
            Value::Object(mut map) => map.remove(segment)?,
            _ => return None,
        };
    }
    Some(current)
}

fn unknown_path(root: &'static str, path: &str) -> ExpressionError {
    ExpressionError::ReservedRootPathUnknown {
        root,
        path: format!("{root}.{path}"),
    }
}

/// The `current` root's member names, with no value observed for any of them.
///
/// A bare `current` reference enumerates the descriptor catalog: naming the
/// members must never cost the probes that observing them would.
fn current_enumeration() -> Value {
    Value::Object(
        context_variable_descriptors()
            .iter()
            .map(|descriptor| (descriptor.name.to_string(), Value::Null))
            .collect(),
    )
}

/// A poisoned lazy-root mutex means a handler panicked mid-read; the recorded
/// state is still sound, so the request continues rather than cascading.
fn lock<T: ?Sized>(value: &Mutex<T>) -> MutexGuard<'_, T> {
    value.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Deterministic refresh capabilities for the laziness and mutation tests.
///
/// A real refresh would depend on the host's repository and clock; every
/// behavior this feature promises is scripted here instead.
#[cfg(test)]
pub(crate) mod test_support {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use std::sync::atomic::{AtomicUsize, Ordering};

    use serde_json::Value;

    use super::{CurrentAuthority, CurrentProvider, CurrentRefresh, lock};

    /// A provider whose answers a test rewrites between evaluations.
    #[derive(Debug, Default)]
    pub(crate) struct ScriptedRefresh {
        values: Mutex<HashMap<String, Value>>,
        calls: Mutex<Vec<String>>,
        observations: AtomicUsize,
    }

    impl ScriptedRefresh {
        pub(crate) fn new<I, K>(values: I) -> Arc<Self>
        where
            I: IntoIterator<Item = (K, Value)>,
            K: Into<String>,
        {
            let provider = Self::default();
            for (key, value) in values {
                lock(&provider.values).insert(key.into(), value);
            }
            Arc::new(provider)
        }

        /// Changes what the next refresh of `key` observes.
        pub(crate) fn set(&self, key: &str, value: Value) {
            lock(&self.values).insert(key.to_string(), value);
        }

        /// Every key this provider was actually asked to observe, in order.
        pub(crate) fn calls(&self) -> Vec<String> {
            lock(&self.calls).clone()
        }

        /// How many refreshes produced an observation.
        pub(crate) fn observations(&self) -> usize {
            self.observations.load(Ordering::SeqCst)
        }
    }

    impl CurrentProvider for ScriptedRefresh {
        fn refresh(&self, key: &str) -> CurrentRefresh {
            lock(&self.calls).push(key.to_string());
            match lock(&self.values).get(key).cloned() {
                Some(value) => {
                    self.observations.fetch_add(1, Ordering::SeqCst);
                    CurrentRefresh::Observed(value)
                }
                None => CurrentRefresh::Unsupported,
            }
        }
    }

    /// An authority backed by a scripted provider.
    pub(crate) fn authority(provider: &Arc<ScriptedRefresh>) -> CurrentAuthority {
        CurrentAuthority::default().with_provider(provider.clone())
    }

    /// A provider whose every refresh of one key observes a different value.
    ///
    /// This is how a single compose can distinguish the two halves of the Q2
    /// memo scope: reads that agree came from one expression evaluation, and
    /// reads that differ came from separate ones. Nothing outside the compose
    /// has to mutate anything mid-run.
    #[derive(Debug)]
    pub(crate) struct SequenceRefresh {
        key: String,
        observations: AtomicUsize,
    }

    impl SequenceRefresh {
        pub(crate) fn new(key: &str) -> Arc<Self> {
            Arc::new(Self {
                key: key.to_string(),
                observations: AtomicUsize::new(0),
            })
        }

        pub(crate) fn observations(&self) -> usize {
            self.observations.load(Ordering::SeqCst)
        }
    }

    impl CurrentProvider for SequenceRefresh {
        fn refresh(&self, key: &str) -> CurrentRefresh {
            if key != self.key {
                return CurrentRefresh::Unsupported;
            }
            let nth = self.observations.fetch_add(1, Ordering::SeqCst) + 1;
            CurrentRefresh::Observed(Value::String(format!("observation-{nth}")))
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::test_support::{ScriptedRefresh, authority};
    use super::*;

    fn context() -> ComposeContext {
        ComposeContext::fixed_for_testing()
    }

    fn scope(provider: &Arc<ScriptedRefresh>) -> CurrentScope {
        CurrentScope::new(authority(provider))
    }

    #[test]
    fn a_member_read_observes_the_key_and_repeats_within_one_scope() {
        let provider = ScriptedRefresh::new([("branch", json!("main"))]);
        let scope = scope(&provider);

        assert_eq!(scope.resolve("current.branch", &context()).unwrap().unwrap(), Some(json!("main")));
        provider.set("branch", json!("feature"));
        assert_eq!(
            scope.resolve("current.branch", &context()).unwrap().unwrap(),
            Some(json!("main")),
            "a repeat read inside one expression scope must agree with the first",
        );
        assert_eq!(provider.calls(), ["branch"]);

        scope.begin_expression_scope();
        assert_eq!(
            scope.resolve("current.branch", &context()).unwrap().unwrap(),
            Some(json!("feature")),
            "the next expression observes the fact afresh",
        );
    }

    #[test]
    fn an_unreached_key_is_never_observed() {
        let provider = ScriptedRefresh::new([("branch", json!("main")), ("os", json!("macos"))]);
        let scope = scope(&provider);

        assert_eq!(scope.resolve("current.os", &context()).unwrap().unwrap(), Some(json!("macos")));
        assert_eq!(provider.calls(), ["os"], "only the referenced key is observed");
    }

    #[test]
    fn a_missing_capability_is_null_plus_a_partial_capture_and_never_the_eager_value() {
        let provider = ScriptedRefresh::new([("branch", json!("main"))]);
        let installed = authority(&provider);
        let scope = CurrentScope::new(installed.clone());

        assert_eq!(scope.resolve("current.repo_root", &context()).unwrap().unwrap(), None);
        let diagnostics = installed.take_diagnostics();
        assert!(
            matches!(
                diagnostics.as_slice(),
                [ContextMergeDiagnostic::PartialRuntimeCapture { area: "current", detail }]
                    if detail.contains("current.repo_root")
            ),
            "{diagnostics:?}",
        );
    }

    #[test]
    fn a_request_with_no_provider_fails_closed_for_every_mutable_fact() {
        let authority = CurrentAuthority::default();
        let scope = CurrentScope::new(authority.clone());

        assert!(!authority.has_provider());
        assert_eq!(scope.resolve("current.branch", &context()).unwrap().unwrap(), None);
        assert_eq!(authority.take_warnings().len(), 1);
    }

    #[test]
    fn request_owned_facts_read_the_eager_capture_and_never_the_provider() {
        let provider = ScriptedRefresh::new([("today", json!("2030-01-01"))]);
        let scope = scope(&provider);

        // `today` is refreshable, `cwd` is request-owned.
        assert_eq!(scope.resolve("current.today", &context()).unwrap().unwrap(), Some(json!("2030-01-01")));
        assert!(scope.resolve("current.cwd", &context()).unwrap().is_err());
        assert_eq!(provider.calls(), ["today"], "a request-owned key never reaches the provider");
    }

    #[test]
    fn the_removed_nesting_is_an_unknown_path_on_both_roots() {
        let provider = ScriptedRefresh::new([("today", json!("2030-01-01"))]);
        let scope = scope(&provider);

        for path in ["current.ctx.today", "current.env.HOME", "current_env.ctx.today"] {
            assert!(
                matches!(
                    scope.resolve(path, &context()).unwrap(),
                    Err(ExpressionError::ReservedRootPathUnknown { .. })
                ),
                "{path} must be an unknown path, not an alias",
            );
        }
        assert!(provider.calls().is_empty(), "an unknown path observes nothing");
    }

    #[test]
    fn bare_current_enumerates_descriptor_keys_without_observing_a_value() {
        let provider = ScriptedRefresh::new([("branch", json!("main"))]);
        let scope = scope(&provider);

        let enumerated = scope.resolve("current", &context()).unwrap().unwrap().unwrap();
        let members = enumerated.as_object().expect("an object of member names");
        assert!(members.contains_key("branch") && members.contains_key("recent_commits"));
        assert!(!members.contains_key("ctx") && !members.contains_key("env"));
        assert!(members.values().all(Value::is_null), "no member value is materialized");
        assert!(provider.calls().is_empty());
    }

    #[test]
    #[serial_test::serial(current_env_scope)]
    fn current_env_rereads_the_live_environment_once_per_scope() {
        let key = "DARKMATTER_CURRENT_ENV_SCOPE_TEST";
        let scope = CurrentScope::default();
        unsafe { std::env::set_var(key, "first") };

        let path = format!("current_env.{key}");
        assert_eq!(scope.resolve(&path, &context()).unwrap().unwrap(), Some(json!("first")));
        unsafe { std::env::set_var(key, "second") };
        assert_eq!(
            scope.resolve(&path, &context()).unwrap().unwrap(),
            Some(json!("first")),
            "one expression scope sees one value",
        );

        scope.begin_expression_scope();
        assert_eq!(scope.resolve(&path, &context()).unwrap().unwrap(), Some(json!("second")));

        unsafe { std::env::remove_var(key) };
        scope.begin_expression_scope();
        assert_eq!(
            scope.resolve(&path, &context()).unwrap().unwrap(),
            None,
            "a missing `current_env` value resolves like a missing `env` value",
        );
    }

    #[test]
    fn paths_outside_the_two_roots_are_left_to_ordinary_resolution() {
        let scope = CurrentScope::default();
        for path in ["ctx.branch", "env.HOME", "currently", "current_environment", "doc.title"] {
            assert!(scope.resolve(path, &context()).is_none(), "{path}");
        }
    }

    #[test]
    fn a_cloned_scope_starts_an_empty_memo() {
        let provider = ScriptedRefresh::new([("branch", json!("main"))]);
        let scope = scope(&provider);
        assert_eq!(scope.resolve("current.branch", &context()).unwrap().unwrap(), Some(json!("main")));

        provider.set("branch", json!("feature"));
        let cloned = scope.clone();
        assert_eq!(
            cloned.resolve("current.branch", &context()).unwrap().unwrap(),
            Some(json!("feature")),
            "a clone shares the provider but never an observation",
        );
        assert_eq!(provider.observations(), 2);
    }

    #[test]
    fn a_dotted_tail_walks_the_observed_value() {
        let provider = ScriptedRefresh::new([("gpu", json!({ "name": "M4" }))]);
        let scope = scope(&provider);

        assert_eq!(scope.resolve("current.gpu.name", &context()).unwrap().unwrap(), Some(json!("M4")));
        assert_eq!(scope.resolve("current.gpu.missing", &context()).unwrap().unwrap(), None);
    }
}
