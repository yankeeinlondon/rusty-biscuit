//! Editor observation of whole-value frontmatter expressions: what an editor
//! may evaluate in the background so it can judge a property's actual value
//! against the destination schema, and the reads it may make doing so.
//!
//! The behavior is documented in
//! [Schema slots, "In the editor"](../../../../../docs/topics/schemas/schema-slots.md).
//!
//! An observation evaluates each whole-value `{{ … }}` property with the
//! session's grants (never asking anyone), its cancel flag and work limits,
//! and one shared data runtime and work budget for the whole pass:
//!
//! - evaluation follows ordinary branch selection, short-circuiting, and
//!   `try` recovery, and every call and binding is judged only when
//!   evaluation reaches it, so an unused branch, fallback, recovery
//!   argument, or callback body decides nothing;
//! - only pure functions and the guarded readers `load_data` and
//!   `file_exists` run; any other reached call leaves the property
//!   [pending](PropertyOutcome::Pending) before it runs, as does a reached
//!   `ctx`, host or runtime binding, or property absent from the document;
//! - a bare name reads a literal property, or another whole-value property,
//!   evaluated the first time a reached read names it (memoized; properties
//!   that read each other are a [cycle](PendingReason::Cycle)); `fm.NAME`
//!   reads the authored value, and one still holding a template leaves the
//!   property pending;
//! - every read goes through the editor's guarded paths
//!   (`compose::expression::editor_reads`): a candidate the permissions and
//!   this process's approvals do not allow is never probed or read, a URL is
//!   answered only from a fresh transport-cache entry, and nothing is
//!   fetched, written, or asked;
//! - an executed, unrecovered read of a provably missing local file is
//!   [`PropertyOutcome::Missing`]; a refused or unavailable read is pending,
//!   which `try` cannot turn into its fallback;
//! - a `load_data` call that ran with no contract and bound a CSV or TSV
//!   read with `header: false` is marked headerless and uncontracted
//!   ([`ObservedCall::headerless_uncontracted`]), judged from the arguments
//!   the loader bound, never from how they are written; a property whose
//!   evaluation ran one, itself or through a property it read, is marked
//!   too ([`PropertyObservation::headerless_uncontracted`]).

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;

use biscuit_file::FileResolutionContext;
use serde_json::{Map, Value};
use simplified_schema::expression::{SpannedCallArgument, SpannedExpr, SpannedExprKind};

use super::SlotSession;
use super::grants::gate::SlotGrants;
use crate::markdown::compose::conditions::ShortcutLookup;
use crate::markdown::compose::expression::data::Observation;
use crate::markdown::compose::expression::editor_reads::{self, EditorReads};
use crate::markdown::compose::expression::iteration::IterationBudget;
use crate::markdown::compose::expression::read_gate::{GateRefusal, ReadGate, ReadTarget};
use crate::markdown::compose::expression::trace::{self, LoadOrigin};
use crate::markdown::compose::expression::binding::ResolvedBinding;
use crate::markdown::compose::expression::fm_namespace;
use crate::markdown::compose::expression::{EvaluationLookup, ExpressionError, FileRefFailure, ResolutionContext, evaluate};
use crate::markdown::compose::remote_fetch::RemoteFetchRuntime;

pub use crate::markdown::compose::expression::editor_reads::PendingReason;

/// One whole-value top-level frontmatter property.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertySite {
    /// The top-level key.
    pub key: String,
    /// The expression inside the `{{ … }}`, without the braces.
    pub expression: String,
    /// Whether the editor observes it for its own sake ([`observes_property`]);
    /// a site that is not eligible is evaluated only when an eligible one
    /// reads it.
    pub eligible: bool,
}

/// What one observation pass evaluates, in which context.
#[derive(Debug, Clone)]
pub struct ObserveRequest {
    /// The document the properties belong to.
    pub document: PathBuf,
    /// The document's own resolution context.
    pub context: FileResolutionContext,
    /// The authored frontmatter as JSON, templates still written.
    pub seed: Map<String, Value>,
    /// Every whole-value property, in document order.
    pub sites: Vec<PropertySite>,
    /// The cache root a composition would use (`md compose --cache-root`);
    /// the transport store read is the one under it
    /// (`<root>/.darkmatter/cache/v…`). `None` is the context's base folder.
    pub cache_root: Option<PathBuf>,
    /// The wall-clock instant cached responses are judged fresh at.
    pub now: SystemTime,
}

/// The result of one observation pass.
#[derive(Debug, Clone)]
pub struct PropertyObservations {
    /// One entry per evaluated site (every eligible site, and each other
    /// site an eligible one read), in document order.
    pub properties: Vec<PropertyObservation>,
    /// Every input the pass observed: content reads, existence answers, the
    /// candidates a miss probed, and those probed before a refusal.
    pub observations: Vec<Observation>,
    /// The earliest expiry of a cached remote body any evaluation used.
    pub remote_expiry: Option<SystemTime>,
    /// The work allowance ran out; the sites it stopped are pending.
    pub exhausted: bool,
    /// The pass was canceled; the sites it stopped are pending.
    pub canceled: bool,
}

impl PropertyObservations {
    /// The observation of `key`, when the pass evaluated it.
    pub fn get(&self, key: &str) -> Option<&PropertyOutcome> {
        self.properties.iter().find(|property| property.key == key).map(|property| &property.outcome)
    }
}

/// One property's observation.
#[derive(Debug, Clone)]
pub struct PropertyObservation {
    /// The top-level key.
    pub key: String,
    /// What evaluating it produced.
    pub outcome: PropertyOutcome,
    /// What each `load_data` call the evaluation reached returned, in the
    /// expression's node order; empty when the property was not evaluated.
    pub calls: Vec<ObservedCall>,
    /// The evaluation ran a headerless uncontracted read, directly or
    /// through another property it read. Its value's type is then
    /// `unknown[][]` for that read, so an editor neither types nor judges it
    /// cell by cell from the observed value (composition still judges every
    /// realized cell). An unused branch's read never sets it.
    pub headerless_uncontracted: bool,
}

/// The values one `load_data` call occurrence returned while a property was
/// observed: once for a call outside a callback, once per item for a call in
/// a callback body.
#[derive(Debug, Clone, PartialEq)]
pub struct ObservedCall {
    /// The call's span in [`PropertySite::expression`].
    pub span: crate::markdown::span::SourceSpan,
    /// The call as written, without spans: what a reader typing the same
    /// call compares against.
    pub call: simplified_schema::expression::ast::Expr,
    /// Its distinct returned values, first seen first, at most eight;
    /// never a headerless uncontracted execution's value.
    pub values: Vec<Value>,
    /// It returned more distinct values than were kept, so its type stays
    /// provisional.
    pub overflow: bool,
    /// At least one execution bound no contract (`as` omitted or `null`)
    /// and a CSV or TSV read with `header: false` (written literally, as a
    /// value that converts to it, through a variable, or by a spread): that
    /// execution's type is `unknown[][]`, never its observed cells.
    pub headerless_uncontracted: bool,
}

/// What evaluating one property produced.
#[derive(Debug, Clone)]
pub enum PropertyOutcome {
    /// The value, with its data source when it is, unchanged, one
    /// `load_data` call's result (directly, through parentheses, or as the
    /// selected arm of a conditional).
    Observed {
        /// The evaluated value.
        value: Value,
        /// The loaded source of a direct load; `None` for a derived value.
        load: Option<Arc<LoadOrigin>>,
    },
    /// No value yet, and no verdict may be drawn from its absence.
    Pending(PendingReason),
    /// An executed, unrecovered read of a local file that provably does not
    /// exist: the ordinary not-found failure composition reports.
    Missing(ExpressionError),
    /// Any other failure of the evaluation (malformed data, a contract
    /// violation, invalid options, a bad spread operand), as composition
    /// would report it.
    Failed(ExpressionError),
}

/// Whether the editor must observe a whole-value expression to know its
/// type: it calls `load_data` anywhere (callback bodies and spread operands
/// included) with no contract, a literal `null`, or one that is not a
/// literal, so the contract may be `null`.
pub fn observes_property(expression: &SpannedExpr) -> bool {
    let mut found = false;
    super::evaluator::walk_nodes(expression, &mut |node| {
        let contract = match &node.kind {
            SpannedExprKind::FunctionCall { name, args } if is_load_data(name) => Some(args.get(1)),
            SpannedExprKind::CallbackCall { name, args } if is_load_data(name) => Some(args.get(1).and_then(|arg| match arg {
                SpannedCallArgument::Value(value) => Some(value),
                SpannedCallArgument::Callback(_) => None,
            })),
            _ => None,
        };
        if let Some(contract) = contract {
            found |= contract.is_none_or(|contract| !statically_non_null(contract));
        }
    });
    found
}

fn is_load_data(name: &str) -> bool {
    crate::markdown::compose::expression::functions::canonical_function_name(name) == Some("load_data")
}

/// Whether `expr` is a literal other than `null`.
fn statically_non_null(expr: &SpannedExpr) -> bool {
    match &expr.kind {
        SpannedExprKind::StringLiteral(_)
        | SpannedExprKind::NumberLiteral(_)
        | SpannedExprKind::BoolLiteral(_)
        | SpannedExprKind::ArrayLiteral(_)
        | SpannedExprKind::ObjectLiteral(_) => true,
        SpannedExprKind::Paren(inner) => statically_non_null(inner),
        _ => false,
    }
}

/// The gate an observation's reads ask: the session's permissions and this
/// process's approvals, judged from the document, never asking anyone.
#[derive(Debug)]
struct EditorGate {
    grants: Arc<SlotGrants>,
    reading: FileResolutionContext,
}

impl ReadGate for EditorGate {
    fn check(&self, target: &ReadTarget) -> Result<(), GateRefusal> {
        self.grants.decide_without_asking(&self.reading, target).unwrap_or_else(|| Err(GateRefusal::approval_required()))
    }

    fn admits_listed(&self, target: &ReadTarget) -> bool {
        self.grants.admits_listed(&self.reading, target)
    }

    fn admits_lexically(&self, target: &ReadTarget) -> bool {
        self.grants.may_admit_lexically(&self.reading, target)
    }
}

impl SlotSession {
    /// Observes `request`'s eligible properties (and the properties they
    /// read); see the [module documentation](self). Never prompts, fetches,
    /// or writes anything.
    pub fn observe_properties(self: &Arc<Self>, request: &ObserveRequest) -> PropertyObservations {
        let (grants, cancel, limits, data) = self.observation_parts();
        let cache_root = request.cache_root.clone().unwrap_or_else(|| request.context.base_dir().to_path_buf());
        let network = grants.book().network().clone();
        let remote = RemoteFetchRuntime::cache_only_under(grants.fetch_policy(), network, &cache_root);
        let editor = Arc::new(EditorReads::new(remote, request.now, cancel));
        let mut base = ResolutionContext::new(request.context.clone());
        base.data = data.clone();
        base.slot_gate = Some(Arc::new(EditorGate { grants, reading: request.context.clone() }));
        base.editor = Some(Arc::clone(&editor));
        base.iteration = IterationBudget::new(limits);
        let mark = data.read_mark();
        let pass = Pass {
            request,
            base,
            states: RefCell::new(HashMap::new()),
            calls: RefCell::new(HashMap::new()),
            headerless: RefCell::new(HashSet::new()),
            reading: RefCell::new(Vec::new()),
        };
        for site in request.sites.iter().filter(|site| site.eligible) {
            pass.site(&site.key);
        }
        let Pass { states, base, calls, headerless, .. } = pass;
        let (mut states, mut calls, headerless) = (states.into_inner(), calls.into_inner(), headerless.into_inner());
        let properties = request
            .sites
            .iter()
            .filter_map(|site| match states.remove(&site.key) {
                Some(State::Done(outcome)) => Some(PropertyObservation {
                    key: site.key.clone(),
                    outcome,
                    calls: calls.remove(&site.key).unwrap_or_default(),
                    headerless_uncontracted: headerless.contains(&site.key),
                }),
                Some(State::Evaluating) | None => None,
            })
            .collect::<Vec<_>>();
        let exhausted = base.iteration.exhausted().is_some();
        PropertyObservations {
            properties,
            observations: data.observations_since(mark),
            remote_expiry: editor.expiry(),
            exhausted,
            canceled: editor.is_canceled(),
        }
    }
}

enum State {
    Evaluating,
    Done(PropertyOutcome),
}

/// One observation pass. Properties are evaluated on demand, the first time
/// an eligible site or a reading expression reaches them, so the state sits
/// behind cells: a dependency is evaluated while its reader's evaluation is
/// still running.
struct Pass<'r> {
    request: &'r ObserveRequest,
    base: ResolutionContext,
    states: RefCell<HashMap<String, State>>,
    calls: RefCell<HashMap<String, Vec<ObservedCall>>>,
    /// The properties whose evaluation ran a headerless uncontracted read,
    /// directly or through a property they read.
    headerless: RefCell<HashSet<String>>,
    /// One entry per evaluation in progress, innermost last: whether it read
    /// a property in [`Self::headerless`].
    reading: RefCell<Vec<bool>>,
}

impl Pass<'_> {
    /// The outcome of the site at `key`, evaluating it the first time.
    fn site(&self, key: &str) -> PropertyOutcome {
        match self.states.borrow().get(key) {
            Some(State::Done(outcome)) => return outcome.clone(),
            Some(State::Evaluating) => return PropertyOutcome::Pending(PendingReason::Cycle),
            None => {}
        }
        self.states.borrow_mut().insert(key.to_string(), State::Evaluating);
        let outcome = self.evaluate(key);
        self.states.borrow_mut().insert(key.to_string(), State::Done(outcome.clone()));
        outcome
    }

    fn evaluate(&self, key: &str) -> PropertyOutcome {
        let editor = self.base.editor.as_ref().expect("an observation pass reads through the editor");
        if editor.is_canceled() {
            return PropertyOutcome::Pending(PendingReason::Canceled);
        }
        if self.base.iteration.exhausted().is_some() {
            return PropertyOutcome::Pending(PendingReason::Exhausted);
        }
        let Some(site) = self.request.sites.iter().find(|site| site.key == key) else {
            return PropertyOutcome::Pending(PendingReason::Dependency(key.to_string()));
        };
        let Ok(parsed) = simplified_schema::expression::parse_spanned(&site.expression) else {
            // The passive call check reports the parse failure.
            return PropertyOutcome::Pending(PendingReason::Unsupported("an expression that does not parse".to_string()));
        };
        let erased = parsed.erase();
        let empty = Value::Object(Map::new());
        let lookup = PassLookup { pass: self, reserved: ShortcutLookup::in_context(&empty, &self.base) };
        self.reading.borrow_mut().push(false);
        let ((result, load), facts) =
            editor_reads::collecting_calls(|| trace::traced(|| evaluate(&erased, &lookup)));
        let read_headerless = self.reading.borrow_mut().pop().unwrap_or(false);
        let calls = observed_calls(&parsed, &erased, facts);
        if read_headerless || calls.iter().any(|call| call.headerless_uncontracted) {
            self.headerless.borrow_mut().insert(key.to_string());
        }
        self.calls.borrow_mut().insert(key.to_string(), calls);
        match result {
            Ok(value) => PropertyOutcome::Observed { value, load },
            Err(error) => classify(error),
        }
    }

    /// The value of the bare top-level name `name`: a literal property, or
    /// another whole-value property, evaluated now if it was not yet.
    fn bare(&self, name: &str) -> Result<Value, PendingReason> {
        if self.request.sites.iter().any(|site| site.key == name) {
            return match self.site(name) {
                PropertyOutcome::Observed { value, .. } => {
                    if self.headerless.borrow().contains(name)
                        && let Some(reading) = self.reading.borrow_mut().last_mut()
                    {
                        *reading = true;
                    }
                    Ok(value)
                }
                PropertyOutcome::Pending(PendingReason::Cycle) => Err(PendingReason::Cycle),
                _ => Err(PendingReason::Dependency(name.to_string())),
            };
        }
        match self.request.seed.get(name) {
            Some(value) if holds_template(value) => Err(PendingReason::Dependency(name.to_string())),
            Some(value) => Ok(value.clone()),
            None => Err(PendingReason::RuntimeOnly(name.to_string())),
        }
    }

    /// The authored frontmatter `fm.PATH` reads: only the key it names, or
    /// every key for a whole `fm`. A value still holding a template is not
    /// known yet.
    fn authored(&self, path: &str) -> Result<Value, PendingReason> {
        let authored = |name: &str, value: &Value| {
            if holds_template(value) { Err(PendingReason::Dependency(name.to_string())) } else { Ok(value.clone()) }
        };
        let mut data = Map::new();
        match path.strip_prefix("fm.").map(|rest| rest.split('.').next().unwrap_or(rest)) {
            Some(name) => match self.request.seed.get(name) {
                Some(value) => {
                    data.insert(name.to_string(), authored(name, value)?);
                }
                None => return Err(PendingReason::RuntimeOnly(format!("fm.{name}"))),
            },
            None => {
                for (name, value) in &self.request.seed {
                    data.insert(name.clone(), authored(name, value)?);
                }
            }
        }
        Ok(Value::Object(data))
    }
}

/// The lookup an observed property evaluates against. Every binding is
/// resolved when evaluation reaches it, never up front, so a branch,
/// fallback, recovery argument, or callback body that does not run binds
/// nothing:
///
/// - a bare name and `fm.NAME` read the document ([`Pass::bare`],
///   [`Pass::authored`]);
/// - `ctx`, `current`, and a `local.*` no callback binds are known only when
///   the document is composed;
/// - `env` and `null` read as composition reads them.
///
/// A binding that is not known yet fails the evaluation with
/// [`ExpressionError::ObservationPending`], which no recovery absorbs.
struct PassLookup<'p, 'r> {
    pass: &'p Pass<'r>,
    /// Answers `env`, `null`, and the read-side functions' context.
    reserved: ShortcutLookup<'p>,
}

impl PassLookup<'_, '_> {
    fn binding(&self, path: &str) -> Result<Option<Value>, PendingReason> {
        let mut segments = path.split('.');
        let root = segments.next().unwrap_or(path);
        match root {
            "null" | "env" => Ok(self.reserved.get(path)),
            "ctx" | "current" | "local" => Err(PendingReason::RuntimeOnly(root.to_string())),
            "fm" => Ok(fm_namespace::resolve_fm_namespace(path, &self.pass.authored(path)?)),
            _ => {
                let mut value = self.pass.bare(root)?;
                for segment in segments {
                    value = match value {
                        Value::Object(mut members) => match members.remove(segment) {
                            Some(member) => member,
                            None => return Ok(None),
                        },
                        _ => return Ok(None),
                    };
                }
                Ok(Some(value))
            }
        }
    }
}

impl EvaluationLookup for PassLookup<'_, '_> {
    fn get(&self, path: &str) -> Option<Value> {
        self.binding(path).ok().flatten()
    }

    fn resolve(&self, path: &str) -> Result<ResolvedBinding, ExpressionError> {
        match self.binding(path) {
            Ok(value) => Ok(ResolvedBinding::classify(path, value)),
            Err(reason) => Err(editor_reads::pending(path, reason)),
        }
    }

    fn resolution_context(&self) -> Option<ResolutionContext> {
        self.reserved.resolution_context()
    }

    /// `fm['key']` reads that one authored key, as `fm.key` does, rather
    /// than the whole `fm` that holds this property's own template.
    fn resolve_index(&self, root: &str, key: &str) -> Option<Result<Value, ExpressionError>> {
        if root != "fm" {
            return None;
        }
        let path = format!("fm.{key}");
        Some(match self.pass.request.seed.get(key) {
            Some(value) if holds_template(value) => Err(editor_reads::pending(&path, PendingReason::Dependency(key.to_string()))),
            Some(value) => Ok(value.clone()),
            None => Err(editor_reads::pending(&path, PendingReason::RuntimeOnly(path.clone()))),
        })
    }

    fn resolution_context_ref(&self) -> Option<&ResolutionContext> {
        self.reserved.resolution_context_ref()
    }
}

/// The collected per-node `facts` of `erased` (evaluated from `parsed`) as
/// authored call spans, in node order.
fn observed_calls(
    parsed: &SpannedExpr,
    erased: &simplified_schema::expression::ast::Expr,
    facts: Vec<(usize, editor_reads::CallValues)>,
) -> Vec<ObservedCall> {
    use crate::markdown::compose::expression::member_check::{expr_nodes, spanned_nodes};
    if facts.is_empty() {
        return Vec::new();
    }
    let nodes = expr_nodes(erased);
    let spanned = spanned_nodes(parsed);
    let mut calls: Vec<(usize, ObservedCall)> = facts
        .into_iter()
        .filter_map(|(address, values)| {
            let ordinal = nodes.iter().position(|node| std::ptr::from_ref(*node).addr() == address)?;
            let span = spanned.get(ordinal)?.span.clone();
            let call = (*nodes.get(ordinal)?).clone();
            Some((
                ordinal,
                ObservedCall {
                    span,
                    call,
                    values: values.values,
                    overflow: values.overflow,
                    headerless_uncontracted: values.headerless_uncontracted,
                },
            ))
        })
        .collect();
    calls.sort_by_key(|(ordinal, _)| *ordinal);
    calls.into_iter().map(|(_, call)| call).collect()
}

/// Whether `value` still holds authored `{{ … }}` or `$( … )` syntax.
fn holds_template(value: &Value) -> bool {
    match value {
        Value::String(text) => text.contains("{{") || text.contains("$("),
        Value::Array(items) => items.iter().any(holds_template),
        Value::Object(members) => members.values().any(holds_template),
        Value::Null | Value::Bool(_) | Value::Number(_) => false,
    }
}

/// The outcome a failed evaluation is.
fn classify(error: ExpressionError) -> PropertyOutcome {
    match error.without_callbacks() {
        ExpressionError::ObservationPending { reason, .. } => PropertyOutcome::Pending(reason.clone()),
        ExpressionError::SlotReadRefused { .. } => PropertyOutcome::Pending(PendingReason::ReadRefused),
        ExpressionError::IterationLimit(_) => PropertyOutcome::Pending(PendingReason::Exhausted),
        _ if error.file_reference_diagnostic().is_some_and(|diagnostic| diagnostic.kind == FileRefFailure::NotFound) => {
            PropertyOutcome::Missing(error)
        }
        _ => PropertyOutcome::Failed(error),
    }
}

#[cfg(test)]
#[path = "observe_tests.rs"]
mod tests;
