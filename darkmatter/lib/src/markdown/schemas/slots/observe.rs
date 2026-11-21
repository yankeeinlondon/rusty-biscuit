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
//! - only pure functions and the guarded readers `load_data` and
//!   `file_exists` run; anything else leaves the property
//!   [pending](PropertyOutcome::Pending), as do `ctx`, a host or runtime
//!   binding, and a property absent from the document;
//! - a bare name reads a literal property, or another whole-value property
//!   evaluated first (memoized; properties that read each other are a
//!   [cycle](PendingReason::Cycle)); `fm.NAME` reads the authored value, and
//!   one still holding a template leaves the property pending;
//! - every read goes through the editor's guarded paths
//!   (`compose::expression::editor_reads`): a candidate the permissions and
//!   this process's approvals do not allow is never probed or read, a URL is
//!   answered only from a fresh transport-cache entry, and nothing is
//!   fetched, written, or asked;
//! - an executed, unrecovered read of a provably missing local file is
//!   [`PropertyOutcome::Missing`]; a refused or unavailable read is pending,
//!   which `try` cannot turn into its fallback.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;

use biscuit_file::FileResolutionContext;
use serde_json::{Map, Value};
use simplified_schema::expression::{SpannedCallArgument, SpannedExpr, SpannedExprKind};

use super::SlotSession;
use super::admission::{PropertyRoot, property_reads};
use super::grants::gate::SlotGrants;
use crate::markdown::compose::conditions::ShortcutLookup;
use crate::markdown::compose::expression::data::Observation;
use crate::markdown::compose::expression::editor_reads::{self, EditorReads};
use crate::markdown::compose::expression::iteration::IterationBudget;
use crate::markdown::compose::expression::read_gate::{GateRefusal, ReadGate, ReadTarget};
use crate::markdown::compose::expression::trace::{self, LoadOrigin};
use crate::markdown::compose::expression::{ExpressionError, FileRefFailure, ResolutionContext, evaluate};
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
    /// Its distinct returned values, first seen first, at most eight.
    pub values: Vec<Value>,
    /// It returned more distinct values than were kept, so its type stays
    /// provisional.
    pub overflow: bool,
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

/// Whether `expression` calls `load_data` anywhere with no contract (or a
/// literal `null`) and a literal `header: false`: a headerless read whose
/// type is `unknown[][]` whatever it loads. An editor types and judges such
/// a property provisionally, never cell by cell from its observed value.
pub fn reads_headerless_uncontracted(expression: &simplified_schema::expression::ast::Expr) -> bool {
    use simplified_schema::expression::ast::{Expr, ObjectEntry};
    let is_null = |expr: &Expr| matches!(expr, Expr::Variable(name) if name == "null");
    crate::markdown::compose::expression::member_check::expr_nodes(expression).into_iter().any(|node| {
        let Expr::FunctionCall { name, args } = node else {
            return false;
        };
        if !is_load_data(name) || !args.get(1).is_none_or(is_null) {
            return false;
        }
        matches!(args.get(2), Some(Expr::ObjectLiteral(entries)) if entries.iter().any(|entry| matches!(
            entry,
            ObjectEntry::Field { key, value: Expr::BoolLiteral(false) } if key == "header"
        )))
    })
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
        let mut pass = Pass { request, base, states: HashMap::new(), calls: HashMap::new() };
        for site in request.sites.iter().filter(|site| site.eligible) {
            pass.site(&site.key);
        }
        let Pass { mut states, base, mut calls, .. } = pass;
        let properties = request
            .sites
            .iter()
            .filter_map(|site| match states.remove(&site.key) {
                Some(State::Done(outcome)) => Some(PropertyObservation {
                    key: site.key.clone(),
                    outcome,
                    calls: calls.remove(&site.key).unwrap_or_default(),
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

struct Pass<'r> {
    request: &'r ObserveRequest,
    base: ResolutionContext,
    states: HashMap<String, State>,
    calls: HashMap<String, Vec<ObservedCall>>,
}

impl Pass<'_> {
    /// The outcome of the site at `key`, evaluating it the first time.
    fn site(&mut self, key: &str) -> PropertyOutcome {
        match self.states.get(key) {
            Some(State::Done(outcome)) => return outcome.clone(),
            Some(State::Evaluating) => return PropertyOutcome::Pending(PendingReason::Cycle),
            None => {}
        }
        self.states.insert(key.to_string(), State::Evaluating);
        let outcome = self.evaluate(key);
        self.states.insert(key.to_string(), State::Done(outcome.clone()));
        outcome
    }

    fn evaluate(&mut self, key: &str) -> PropertyOutcome {
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
        let reads = property_reads(&parsed);
        if let Some(blocked) = reads.blocked {
            return PropertyOutcome::Pending(blocked);
        }
        let mut data = Map::new();
        for root in &reads.roots {
            if let Err(reason) = self.bind(root, &mut data) {
                return PropertyOutcome::Pending(reason);
            }
        }
        let erased = parsed.erase();
        let data = Value::Object(data);
        let lookup = ShortcutLookup::in_context(&data, &self.base);
        let ((result, load), facts) =
            editor_reads::collecting_calls(|| trace::traced(|| evaluate(&erased, &lookup)));
        self.calls.insert(key.to_string(), observed_calls(&parsed, &erased, facts));
        match result {
            Ok(value) => PropertyOutcome::Observed { value, load },
            Err(error) => classify(error),
        }
    }

    /// Adds what `root` reads to `data`, or why it is not known yet.
    fn bind(&mut self, root: &PropertyRoot, data: &mut Map<String, Value>) -> Result<(), PendingReason> {
        match root {
            PropertyRoot::Bare(name) => {
                if self.request.sites.iter().any(|site| &site.key == name) {
                    return match self.site(name) {
                        PropertyOutcome::Observed { value, .. } => {
                            data.insert(name.clone(), value);
                            Ok(())
                        }
                        PropertyOutcome::Pending(PendingReason::Cycle) => Err(PendingReason::Cycle),
                        _ => Err(PendingReason::Dependency(name.clone())),
                    };
                }
                match self.request.seed.get(name) {
                    Some(value) if holds_template(value) => Err(PendingReason::Dependency(name.clone())),
                    Some(value) => {
                        data.insert(name.clone(), value.clone());
                        Ok(())
                    }
                    None => Err(PendingReason::RuntimeOnly(name.clone())),
                }
            }
            PropertyRoot::Fm(name) => match self.request.seed.get(name) {
                Some(value) if holds_template(value) => Err(PendingReason::Dependency(name.clone())),
                Some(value) => {
                    data.insert(name.clone(), value.clone());
                    Ok(())
                }
                None => Err(PendingReason::RuntimeOnly(format!("fm.{name}"))),
            },
            PropertyRoot::WholeFm => {
                for (name, value) in &self.request.seed {
                    if holds_template(value) {
                        return Err(PendingReason::Dependency(name.clone()));
                    }
                    data.insert(name.clone(), value.clone());
                }
                Ok(())
            }
        }
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
            Some((ordinal, ObservedCall { span, call, values: values.values, overflow: values.overflow }))
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
