//! Evaluates one admitted schema slot on the ordinary expression engine.
//!
//! A slot evaluates in its defining file's context (the request context
//! derived for that file, never a recapture of process state), with no
//! document data: `fm` and document properties were refused at admission.
//! It reads through the command's [`DataRuntime`], so its `load_data` reads
//! share the command's memo and read set, and every read is asked of the
//! evaluator's [`ReadGate`] first. Each slot gets a fresh work budget of the
//! default iteration limits, so a runaway expression stops with the ordinary
//! `dm.iteration.*` failure.
//!
//! Besides the value, an evaluation reports what it depended on: every input
//! observed (hashed at read) and a digest of each context input it read
//! (each captured `ctx` group, each `env.NAME`, or the whole environment for
//! a computed `env[...]`). Digests only; no value text is kept.

use std::path::Path;
use std::sync::Arc;

use biscuit_file::FileResolutionContext;
use serde_json::Value;
use simplified_schema::expression::{SpannedExpr, SpannedExprKind};

use crate::markdown::compose::DataRuntime;
use crate::markdown::compose::remote_fetch::RemoteFetchRuntime;
use crate::markdown::compose::conditions::ShortcutLookup;
use crate::markdown::compose::context::capture::CapturedObservations;
use crate::markdown::compose::expression::data::Observation;
use crate::markdown::compose::expression::iteration::{IterationBudget, IterationLimits};
use crate::markdown::compose::expression::read_gate::{GateRefusal, ReadGate, ReadTarget};
use crate::markdown::compose::expression::{ExpressionError, ResolutionContext, evaluate};

/// A gate that refuses every read: the evaluator's default until approvals
/// decide otherwise.
#[derive(Debug, Clone, Copy, Default)]
// Constructed through `SlotEvaluator::new`; the slot session wires the
// evaluator, so until it does only tests construct one.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) struct DenyAllReads;

impl ReadGate for DenyAllReads {
    fn check(&self, _target: &ReadTarget) -> Result<(), GateRefusal> {
        Err(GateRefusal::approval_required())
    }
}

/// Gives each slot the gate its reads are asked of. A slot's automatic
/// grants depend on where the slot was written, so the gate is chosen per
/// slot, by the file (or, for an inline schema, the folder) that defines it.
pub(crate) trait SlotGates: Send + Sync + std::fmt::Debug {
    /// The gate for a slot defined at `defining`.
    fn gate_for(&self, defining: &Path) -> Arc<dyn ReadGate>;
}

/// One gate for every slot, wherever it was written.
#[derive(Debug)]
struct SameGate(Arc<dyn ReadGate>);

impl SlotGates for SameGate {
    fn gate_for(&self, _defining: &Path) -> Arc<dyn ReadGate> {
        Arc::clone(&self.0)
    }
}

/// A context input a slot read, beside the files it observed.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum ContextInput {
    /// A captured `ctx` group, by name.
    Group(String),
    /// `env.NAME`.
    Env(String),
    /// The whole environment (a bare `env` or a computed `env[...]`).
    WholeEnv,
}

/// One context input and the digest of what it held (`None`: an unset
/// `env.NAME`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct ContextDigest {
    pub(crate) input: ContextInput,
    pub(crate) digest: Option<[u8; 32]>,
}

/// One slot's value and what it depended on.
#[derive(Debug, Clone)]
// The resolution cache reads the dependencies; until it lands only tests do.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) struct SlotEvaluation {
    pub(crate) value: Value,
    /// Every input the evaluation observed, in order.
    pub(crate) observations: Vec<Observation>,
    /// Every context input it read, sorted.
    pub(crate) context_digests: Vec<ContextDigest>,
}

/// The shared setup every slot of one resolution evaluates with.
#[derive(Debug, Clone)]
pub(crate) struct SlotEvaluator {
    data: DataRuntime,
    observations: CapturedObservations,
    gates: Arc<dyn SlotGates>,
    remote: Option<RemoteFetchRuntime>,
    limits: IterationLimits,
}

// The slot session builds evaluators; until it lands only tests do.
#[cfg_attr(not(test), allow(dead_code))]
impl SlotEvaluator {
    /// An evaluator reading through `data`, refusing every read.
    pub(crate) fn new(data: DataRuntime) -> Self {
        Self {
            data,
            observations: CapturedObservations::default(),
            gates: Arc::new(SameGate(Arc::new(DenyAllReads))),
            remote: None,
            limits: IterationLimits::default(),
        }
    }

    /// Asks `gate` about every read instead.
    #[must_use]
    pub(crate) fn with_gate(mut self, gate: Arc<dyn ReadGate>) -> Self {
        self.gates = Arc::new(SameGate(gate));
        self
    }

    /// Asks the gate `gates` chooses for each slot.
    #[must_use]
    pub(crate) fn with_gates(mut self, gates: Arc<dyn SlotGates>) -> Self {
        self.gates = gates;
        self
    }

    /// Fetches a slot's remote reads through `remote`, the slots' own
    /// runtime (never the document's): without one a URL read fails as
    /// remote reads not enabled, after the gate allowed it.
    #[must_use]
    pub(crate) fn with_remote(mut self, remote: RemoteFetchRuntime) -> Self {
        self.remote = Some(remote);
        self
    }

    /// Reads the request's captured observations (its clock reading) instead
    /// of a set of its own.
    #[must_use]
    pub(crate) fn with_observations(mut self, observations: CapturedObservations) -> Self {
        self.observations = observations;
        self
    }

    /// Gives each slot `limits` instead of the default allowances.
    #[must_use]
    pub(crate) fn with_limits(mut self, limits: IterationLimits) -> Self {
        self.limits = limits;
        self
    }

    /// Evaluates `expression` (already parsed as `parsed`) in `context`,
    /// the slot's own derived context, asking the gate for a slot defined at
    /// `defining`.
    ///
    /// ## Errors
    ///
    /// The expression's typed failure, unchanged: a `load_data` failure keeps
    /// its [`DataError`](crate::markdown::compose::expression::data::DataError),
    /// a refused read is [`ExpressionError::SlotReadRefused`], an exhausted
    /// budget is [`ExpressionError::IterationLimit`].
    pub(crate) fn evaluate(
        &self,
        parsed: &SpannedExpr,
        context: FileResolutionContext,
        defining: &Path,
    ) -> Result<SlotEvaluation, ExpressionError> {
        let mut resolution = ResolutionContext::new(context);
        resolution.data = self.data.clone();
        resolution.observations = self.observations.clone();
        resolution.slot_gate = Some(self.gates.gate_for(defining));
        resolution.remote_fetch = self.remote.clone();
        resolution.iteration = IterationBudget::new(self.limits);
        let erased = parsed.erase();
        let lookup = ShortcutLookup::in_context(&Value::Null, &resolution);
        let (value, observations) =
            crate::markdown::compose::expression::data::runtime::capture_observations(|| evaluate(&erased, &lookup));
        let value = value?;
        let mut context_digests: Vec<ContextDigest> = lookup
            .ctx_group_digests()
            .into_iter()
            .map(|(group, digest)| ContextDigest { input: ContextInput::Group(group), digest: Some(digest) })
            .collect();
        context_digests.extend(env_digests(parsed, resolution.file_resolution_context.env()));
        context_digests.sort_by(|left, right| left.input.cmp(&right.input));
        context_digests.dedup();
        Ok(SlotEvaluation { value, observations, context_digests })
    }
}

/// The digest of each environment input `expr` can read: `env.NAME` by its
/// value, and the whole sorted environment for a bare or computed `env`.
fn env_digests(expr: &SpannedExpr, environment: &std::collections::HashMap<String, String>) -> Vec<ContextDigest> {
    let digest = |text: &str| biscuit_hash::blake3_hash_bytes(text.as_bytes());
    let mut names = Vec::new();
    let mut whole = false;
    env_reads(expr, &mut names, &mut whole);
    let mut digests: Vec<ContextDigest> = names
        .into_iter()
        .map(|name| {
            let value = environment.get(&name).map(|value| digest(value));
            ContextDigest { input: ContextInput::Env(name), digest: value }
        })
        .collect();
    if whole {
        let sorted: std::collections::BTreeMap<_, _> = environment.iter().collect();
        let canonical = serde_json::to_string(&sorted).unwrap_or_default();
        digests.push(ContextDigest { input: ContextInput::WholeEnv, digest: Some(digest(&canonical)) });
    }
    digests
}

/// Collects `env.NAME` reads into `names`, and sets `whole` for a bare `env`
/// (which a computed `env[...]` reads too).
fn env_reads(expr: &SpannedExpr, names: &mut Vec<String>, whole: &mut bool) {
    let mut visit = |node: &SpannedExpr| {
        if let SpannedExprKind::Variable(path) = &node.kind {
            match path.strip_prefix("env.") {
                Some(rest) => names.push(rest.split('.').next().unwrap_or(rest).to_string()),
                None if path == "env" => *whole = true,
                None => {}
            }
        }
    };
    walk_nodes(expr, &mut visit);
}

/// Visits every node of `expr`, callback bodies included.
fn walk_nodes(expr: &SpannedExpr, visit: &mut impl FnMut(&SpannedExpr)) {
    use simplified_schema::expression::SpannedCallArgument;
    visit(expr);
    match &expr.kind {
        SpannedExprKind::Variable(_)
        | SpannedExprKind::StringLiteral(_)
        | SpannedExprKind::NumberLiteral(_)
        | SpannedExprKind::BoolLiteral(_) => {}
        SpannedExprKind::ArrayLiteral(items) => items.iter().for_each(|item| walk_nodes(item, visit)),
        SpannedExprKind::ObjectLiteral(entries) => entries.iter().for_each(|(_, value)| walk_nodes(value, visit)),
        SpannedExprKind::UnaryNot(inner)
        | SpannedExprKind::UnaryMinus(inner)
        | SpannedExprKind::Paren(inner)
        | SpannedExprKind::MemberAccess { base: inner, .. } => walk_nodes(inner, visit),
        SpannedExprKind::Binary { left, right, .. }
        | SpannedExprKind::Comparison { left, right, .. }
        | SpannedExprKind::LogicalOr { left, right }
        | SpannedExprKind::Index { base: left, index: right } => {
            walk_nodes(left, visit);
            walk_nodes(right, visit);
        }
        SpannedExprKind::Ternary { condition, then_branch, else_branch } => {
            walk_nodes(condition, visit);
            walk_nodes(then_branch, visit);
            walk_nodes(else_branch, visit);
        }
        SpannedExprKind::FunctionCall { args, .. } => args.iter().for_each(|arg| walk_nodes(arg, visit)),
        SpannedExprKind::CallbackCall { args, .. } => {
            for arg in args {
                match arg {
                    SpannedCallArgument::Value(value) => walk_nodes(value, visit),
                    SpannedCallArgument::Callback(callback) => walk_nodes(&callback.body, visit),
                }
            }
        }
    }
}

/// The context a slot defined in `defining_file` evaluates in: the request
/// context derived for that file, or the request context itself for a slot
/// with no file (an inline document `$schema`, in-memory content).
pub(crate) fn slot_context(request: &FileResolutionContext, defining_file: Option<&Path>) -> FileResolutionContext {
    match defining_file {
        Some(file) => request.for_source(file),
        None => request.clone(),
    }
}

/// The context inputs `expr` can read, found passively: each `ctx` group a
/// `ctx.KEY` read projects from (every group for a bare `ctx`), each
/// `env.NAME`, and the whole environment for a bare or computed `env`.
pub(crate) fn context_inputs(expr: &SpannedExpr) -> Vec<ContextInput> {
    use crate::markdown::compose::context::capture::ContextGroup;
    let mut names = Vec::new();
    let mut whole = false;
    env_reads(expr, &mut names, &mut whole);
    let mut inputs: Vec<ContextInput> = names.into_iter().map(ContextInput::Env).collect();
    if whole {
        inputs.push(ContextInput::WholeEnv);
    }
    let mut groups = Vec::new();
    walk_nodes(expr, &mut |node| {
        let SpannedExprKind::Variable(path) = &node.kind else {
            return;
        };
        if path == "ctx" {
            groups.extend(ContextGroup::all());
        } else if let Some(rest) = path.strip_prefix("ctx.") {
            let key = rest.split('.').next().unwrap_or(rest);
            groups.extend(ContextGroup::all().into_iter().filter(|group| group.projected_keys().any(|projected| projected == key)));
        }
    });
    inputs.extend(groups.into_iter().map(|group| ContextInput::Group(format!("{group:?}"))));
    inputs.sort();
    inputs.dedup();
    inputs
}

/// The digest `input` has now in `context`, computed the way an evaluation
/// computes it: a `ctx` group captured afresh, an `env.NAME` by its value
/// (`None` when unset), the whole environment by its sorted canonical JSON.
pub(crate) fn context_digest_now(input: &ContextInput, context: &FileResolutionContext) -> Option<[u8; 32]> {
    use crate::markdown::compose::context::capture::ContextGroup;
    use crate::markdown::compose::expression::ctx::CtxLookup;
    let environment = context.env();
    match input {
        ContextInput::Env(name) => environment.get(name).map(|value| biscuit_hash::blake3_hash_bytes(value.as_bytes())),
        ContextInput::WholeEnv => {
            let sorted: std::collections::BTreeMap<_, _> = environment.iter().collect();
            let canonical = serde_json::to_string(&sorted).unwrap_or_default();
            Some(biscuit_hash::blake3_hash_bytes(canonical.as_bytes()))
        }
        ContextInput::Group(name) => {
            let group = ContextGroup::all().into_iter().find(|group| format!("{group:?}") == *name)?;
            let lookup = CtxLookup::new(context.cwd(), environment);
            let key = group.projected_keys().next()?;
            let _ = lookup.resolve_ctx(&format!("ctx.{key}"));
            lookup.group_digests().into_iter().find(|(captured, _)| captured == name).map(|(_, digest)| digest)
        }
    }
}
