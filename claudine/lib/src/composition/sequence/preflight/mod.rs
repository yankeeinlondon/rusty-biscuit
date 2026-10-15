//! Sequence Plus static preflight: the recursive task graph.
//!
//! Preflight answers one question before anything is launched: *what could this
//! sequence possibly execute?* It walks every step, expands every external
//! reference — `kind: task` files, `kind: group` files, `kind: group-catalog`
//! entries, and `prompt:` documents — and returns an immutable
//! [`PreflightGraph`] describing all potentially executable work.
//!
//! Three properties make the walk trustworthy:
//!
//! - **Condition-blind.** Conditional branches are traversed as *potential*
//!   work. A lifecycle action guarded by a `when:` that is false today still
//!   contributes its shell command, because the guard may read differently at
//!   its turn.
//! - **Origin-relative.** Every reference resolves from the directory of the
//!   document that authored it, never the process CWD and never the top-level
//!   sequence directory. The authoring directory travels with each node.
//! - **Positioned.** Every task records whether it is the sequence's first
//!   ([`TaskPosition`]). The first task's commands whose values all exist now
//!   are resolved once here against an early-binding-only lookup and may be
//!   approved as those bytes or by their typed shape. Every other command with a
//!   placeholder — any in a later task, and any reading a late value
//!   (`outputs`, `loop`, `group`) — is approved only by its typed shape and
//!   filled when it runs ([`PreparedShellSite`]).
//!
//! Every failure raised here is abort-all: `fail_fast` governs *execution*
//! outcomes, and preflight happens before execution exists. `--dry-run` runs
//! this identical walk.
//!
//! See `claudine/features/2026-07-11-sequence-plus/spec.md` → *Phase 1 — Static
//! Preflight*.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use darkmatter::markdown::Markdown;

use super::super::lifecycle::bindings::{
    LifecycleScope, approval_diagnostics, unavailable_root,
};
use darkmatter::markdown::compose::{ComposeContext, EffectiveState};
use serde_json::{Map, Value};

use super::super::authored_order::AuthoredOrder;
use super::super::error::CompositionError;
use super::super::lifecycle::{parse_lifecycle_config, validate_no_nested_spans_in_literals};
use super::super::json_util::json_type_name;
use super::super::types::ResolvedCompositionSource;
use super::model::{ExecutableField, SequencePlan, StepExecutable};
use super::task::TaskStage;
use super::{reserved, source as source_resolution};
use crate::invocation_context::{InvocationContext, SourceContext};

mod reapproval;
mod shape;

pub use reapproval::PreflightApproval;
pub use shape::{
    ApprovedCommand, ApprovedSetValue, CommandSiteId, GroupExecution, PreflightAction,
    PreflightGroup, PreflightSequence, PreflightStep, PreflightTask, PreparedShellSite, ShellSiteKind, SiteMode,
    TaskDiagnosticProvenance, TaskPosition,
};

use darkmatter::markdown::compose::shell_expansion::template::{
    CommandSite, CommandTemplate, DeferredShellRoots, HoleError, HoleEvaluator, HoleTiming,
    LateValues, ModeRequirement, ShellCandidate, derived_deferred_keys, parse_authored,
};
use darkmatter::markdown::compose::{ShellCommandOrigin, ShellExpansionError};
use std::collections::HashSet;

/// The roots whose values a sequence run produces: the outputs accumulator, the
/// innermost loop's values, and the enclosing group's variables. A placeholder
/// that reads one — directly or through a value derived from one — is late: it
/// is not evaluated at approval, and its command is approved only as a
/// template. `err`, `timing`, and `current` are not late values; a shell
/// command reading them stays refused.
pub const SEQUENCE_LATE_ROOTS: [&str; 3] = [
    "seq",
    crate::composition::reserved::LOOP_ROOT,
    "group",
];

/// [`SEQUENCE_LATE_ROOTS`] as Darkmatter's discovery declaration.
pub fn sequence_deferred_roots() -> DeferredShellRoots {
    DeferredShellRoots::new(SEQUENCE_LATE_ROOTS)
}

/// The globals a sequence shell placeholder may name without being a document
/// property, so its template type falls back to `string`.
const SEQUENCE_HOST_GLOBALS: [&str; 5] = ["seq", "loop", "group", "err", "timing"];

/// A shell command discovered during preflight that may run as resolved bytes,
/// with the location that contributed it. The `command` is the resolved byte
/// string approval sees and execution runs — never the authored template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredCommand {
    /// The resolved command bytes.
    pub command: String,
    /// The document that authored it.
    pub source_file: PathBuf,
    /// A human label locating the command within that document.
    pub origin: String,
}

/// A `prompt:` document reached from the task graph.
#[derive(Debug, Clone)]
pub struct PromptDocument {
    /// Canonical path to the document.
    pub path: PathBuf,
    /// The parsed document, retained so approval and JIT composition never
    /// re-read it during this preflight.
    pub markdown: Markdown,
    /// `true` when the document's own frontmatter carries a string `prompt:`,
    /// which makes composing it an inline-compose run that rewrites its body.
    pub inline_compose: bool,
    /// `true` when the document declares a `$schema`, so its required-property
    /// gaps join this run's aggregated collection pass.
    pub has_schema: bool,
}

type GraphShellAuthorization = Option<(darkmatter::markdown::compose::shell_expansion::template::ShellPreApprovals, Option<PathBuf>)>;

/// The complete statically-knowable picture of one sequence invocation.
#[derive(Debug, Clone, Default)]
pub struct PreflightGraph {
    authored_sources: HashMap<PathBuf, Option<Vec<u8>>>,
    /// Authorization granted by the graph audit, separate from discovery.
    shell_authorization: std::sync::Arc<std::sync::Mutex<GraphShellAuthorization>>,
    /// One node per step, in sequence order.
    pub steps: Vec<PreflightStep>,
    /// Every prompt document reachable from the graph, deduplicated by
    /// canonical path in first-encounter order.
    pub prompt_documents: Vec<PromptDocument>,
    /// Every task command reachable from the graph that is approved as
    /// resolved bytes, in discovery order.
    pub shell_commands: Vec<DiscoveredCommand>,
    /// Every task command with placeholders, including resolved-byte sites
    /// without a valid template shape, in discovery order: one per site, never merged.
    pub template_candidates: Vec<ShellCandidate>,
    /// Every reference to a prompt document, with what its discovery needs:
    /// the referencing task's position and its parameters. A document
    /// referenced from two positions is discovered once for each.
    pub prompt_references: Vec<PromptReference>,
}

/// One task's reference to a prompt document, as its shell discovery needs it.
#[derive(Debug, Clone, PartialEq)]
pub struct PromptReference {
    /// Whether the reference belongs to a lexically established group.
    pub in_group: bool,
    /// The canonical document path.
    pub path: PathBuf,
    /// The referencing task's position.
    pub position: TaskPosition,
    /// The task's `params` whose values exist at approval, evaluated against
    /// the step's early-binding state. The document reads them as data.
    pub params: Map<String, Value>,
    /// The task's `params` whose values read a late root, which the document's
    /// discovery treats as late roots themselves. Sorted.
    pub deferred_params: Vec<String>,
}

impl PromptReference {
    /// The late roots the document's discovery declares. A later task's
    /// document has every command with a placeholder approved as a template.
    pub fn deferred_shell_roots(&self) -> DeferredShellRoots {
        let roots = DeferredShellRoots::new(
            SEQUENCE_LATE_ROOTS
                .iter()
                .filter(|root| **root != "group" || self.in_group)
                .map(|root| (*root).to_string())
                .chain(self.deferred_params.iter().cloned()),
        );
        match self.position {
            TaskPosition::First => roots,
            TaskPosition::Later => roots.with_placeholders_template_required(),
        }
    }
}

/// Resolve and validate every late value `task`'s command sites read, before
/// any of its commands runs: the step barrier.
///
/// `evaluator` reads the run state as it stands now (`outputs`, `loop`,
/// `group`). Only late placeholders are read; every other placeholder is read
/// when its own command runs, against the state current then.
///
/// ## Errors
///
/// [`CompositionError::SequenceShellLateValue`] for the first site, in authored
/// order, whose late value is missing, is not a scalar, or fails its declared
/// type. The error never contains the value.
pub fn resolve_task_late_values(
    task: &PreflightTask,
    evaluator: &dyn HoleEvaluator,
) -> Result<LateValues, CompositionError> {
    darkmatter::markdown::compose::shell_expansion::template::resolve_late_values(
        task.late_sites(),
        evaluator,
    )
    .map_err(|failure| {
        let site = task
            .shell_sites
            .iter()
            .find(|site| site.id == failure.site)
            .expect("a late-value failure names one of the task's sites");
        late_value_failure(site, failure.error)
    })
}

/// The typed step failure for one late value.
fn late_value_failure(site: &PreparedShellSite, error: HoleError) -> CompositionError {
    let message = error.to_string();
    let shell_error = error.into_shell_error(
        &site_source_context(site),
        &site.authored,
        &site_origin(&site.property),
    );
    CompositionError::SequenceShellLateValue {
        command: site.authored.clone(),
        task: site.label.clone(),
        message,
        source: Box::new(darkmatter::markdown::MarkdownError::ShellExpansion(Box::new(
            shell_error,
        ))),
    }
}

fn site_origin(property: &str) -> ShellCommandOrigin {
    ShellCommandOrigin::Frontmatter {
        key: property.to_string(),
        line: None,
    }
}

fn site_source_context(site: &PreparedShellSite) -> biscuit_terminal::errors::SourceContext {
    biscuit_terminal::errors::SourceContext::new(
        site.source_file.clone(),
        site.source_file.clone(),
        site.authored.clone(),
    )
}

impl PreflightGraph {
    fn capture_source(&mut self, path: &Path) {
        self.authored_sources.entry(path.to_path_buf()).or_insert_with(|| std::fs::read(path).ok());
    }

    /// Unchanged source bytes retain their audited nodes; live variables still
    /// resolve at each container preparation boundary.
    pub(crate) fn sources_unchanged(&self) -> bool {
        !self.authored_sources.is_empty() && self.authored_sources.iter().all(|(path, original)| {
            match (original, std::fs::read(path)) {
                (Some(original), Ok(current)) => original == &current,
                _ => false,
            }
        })
    }

    pub(crate) fn record_shell_authorization(
        &self,
        approved: darkmatter::markdown::compose::shell_expansion::template::ShellPreApprovals,
        policy_root: Option<PathBuf>,
    ) {
        if let Ok(mut authorization) = self.shell_authorization.lock() {
            *authorization = Some((approved, policy_root));
        }
    }

    pub(crate) fn template_authorization(&self, template: &CommandTemplate) -> Option<Option<PathBuf>> {
        let authorization = self.shell_authorization.lock().ok()?;
        let (approved, root) = authorization.as_ref()?;
        approved.contains_template(&darkmatter::markdown::compose::shell_expansion::template::TemplateShape::from_template(template))
            .then(|| root.clone())
    }
    /// Settle either-mode task sites against the audited membership before
    /// stack stamping or reapproval can turn their authored text into bytes.
    pub(crate) fn settle_task_shell_modes(&self, task: &mut PreflightTask) {
        let Ok(authorization) = self.shell_authorization.lock() else { return };
        let Some((approved, _)) = authorization.as_ref() else { return };
        for site in &mut task.shell_sites {
            if site.mode == SiteMode::Template { continue; }
            let candidate = self.template_candidates.iter().find(|candidate| {
                candidate.site.source_file == site.source_file
                    && candidate.site.origin == site_origin(&site.property)
                    && candidate.authored() == Some(site.authored.as_str())
            });
            let Some(candidate) = candidate else { continue };
            let exact = candidate.resolved.as_ref().is_some_and(|keys| {
                keys.iter().all(|key| approved.contains_exact(key))
            });
            if !exact && candidate.shape().is_some_and(|shape| approved.contains_template(&shape)) {
                site.mode = SiteMode::Template;
                task.preflight_approvals.retain(|record| {
                    record.stage() != site.stage || record.authored() != site.authored
                        && record.authored() != format!("$({})", site.authored)
                });
                task.approved_stack_commands.retain(|record| {
                    record.site().stage() != site.stage || record.site().authored() != site.authored
                });
            }
        }
    }

    /// The prompt document at `path`, if the graph reached one.
    pub fn prompt_document(&self, path: &Path) -> Option<&PromptDocument> {
        self.prompt_documents.iter().find(|d| d.path == path)
    }

    /// Reject a nested `{{ … }}` span inside a single-pass lifecycle literal in
    /// any referenced prompt document, before the first step runs.
    ///
    /// Passive: it reads each document's authored frontmatter as the graph
    /// already resolved and parsed it — no compose, shell, remote fetch, or
    /// prompt. A document whose lifecycle block does not parse is left to its
    /// own turn, where canonical preparation reports that error and runs this
    /// same validator. The sequence document itself is covered by Phase 1c,
    /// which prepares every step through the same validator.
    ///
    /// ## Errors
    ///
    /// The first [`CompositionError::LifecycleNestedSpanInLiteral`], in
    /// first-encounter document order.
    pub fn validate_prompt_lifecycle_literals(&self) -> Result<(), CompositionError> {
        for document in &self.prompt_documents {
            let frontmatter = Value::Object(
                document
                    .markdown
                    .frontmatter()
                    .as_map()
                    .iter()
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect(),
            );
            let Ok(lifecycle) = parse_lifecycle_config(&frontmatter, &document.path) else {
                continue;
            };
            validate_no_nested_spans_in_literals(&frontmatter, &lifecycle, &document.path)?;
        }
        Ok(())
    }
}


/// Reject invoking a non-sequence document kind directly.
///
/// `kind: group` documents in particular are never directly executable — a
/// group runs only as a sequence task (spec → *Groups*). Catching this at the
/// entry point produces an actionable error instead of "does not define a
/// `sequence` frontmatter property".
///
/// ## Errors
///
/// Returns [`CompositionError::SequenceUnsupportedConstruct`] when the source's
/// `kind:` is `group`, `group-catalog`, or `task`.
pub fn reject_non_sequence_kind(source: &ResolvedCompositionSource) -> Result<(), CompositionError> {
    let Some(Value::String(kind)) = source.markdown.frontmatter().as_map().get("kind") else {
        return Ok(());
    };
    let detail = match kind.as_str() {
        "group" | "group-catalog" => {
            "a group executes only as a sequence task; reference it from a step's `group:`"
        }
        "task" => "a task executes only inside a sequence or a group; reference it with `task:`",
        _ => return Ok(()),
    };
    Err(CompositionError::SequenceUnsupportedConstruct {
        construct: format!("running a `kind: {kind}` document directly"),
        detail: detail.to_string(),
    })
}

/// Walk `plan` and build the complete preflight graph.
///
/// ## Errors
///
/// Every construct rejection, reference failure, cycle, collision, and shell
/// late-binding violation surfaces as its typed [`CompositionError`] variant.
/// All of them are abort-all: preflight never degrades to best-effort.
pub fn build_preflight_graph(
    plan: &SequencePlan,
    source: &ResolvedCompositionSource,
    file_resolution_context: &biscuit_file::FileResolutionContext,
) -> Result<PreflightGraph, CompositionError> {
    let anchor = source.resolved_path.parent().unwrap_or_else(|| file_resolution_context.cwd());
    let context = ComposeContext::capture_for_document(anchor, &source.markdown);
    build_preflight_graph_with_context(plan, source, context, file_resolution_context)
}

/// [`build_preflight_graph`] against a caller-supplied `ctx.*`/`env.*` snapshot.
///
/// A caller that already captured a context for this composition passes it here
/// so shell early-binding resolves against the *same* snapshot the body and the
/// lifecycle stacks see — the divergence
/// [`StackExecutionContext::prepared_context`][ctx] exists to prevent — instead
/// of re-probing the environment a second time.
///
/// ## Errors
///
/// As [`build_preflight_graph`].
///
/// [ctx]: super::super::lifecycle::executor::StackExecutionContext::prepared_context
pub fn build_preflight_graph_with_context(
    plan: &SequencePlan,
    source: &ResolvedCompositionSource,
    context: ComposeContext,
    file_resolution_context: &biscuit_file::FileResolutionContext,
) -> Result<PreflightGraph, CompositionError> {
    let mut loader = Loader::new(source, context, file_resolution_context.clone());
    loader.walk_plan(plan)?;
    Ok(loader.graph)
}

/// Build preflight from one invocation's retained repository observations.
///
/// Every referenced document derives its own [`SourceContext`], so nested
/// references and early-bound `ctx.*` values follow the repository that owns
/// the authoring document rather than inheriting the top-level sequence roots.
/// `caller_setters` are the invocation's `key=value`/`--set` values, which a
/// shell command reads at approval exactly as it does at its turn.
pub fn build_preflight_graph_with_invocation(
    plan: &SequencePlan,
    source: &ResolvedCompositionSource,
    context: ComposeContext,
    invocation: &InvocationContext,
    source_context: &SourceContext,
    caller_setters: Option<&Value>,
) -> Result<PreflightGraph, CompositionError> {
    let mut loader = Loader::new_with_invocation(
        source,
        context,
        invocation,
        source_context.clone(),
    );
    loader.caller_setters = caller_setters.and_then(Value::as_object).cloned().unwrap_or_default();
    loader.walk_plan(plan)?;
    Ok(loader.graph)
}

fn load_preparation_source(path: &Path) -> Result<ResolvedCompositionSource, CompositionError> {
    let original_text = std::fs::read_to_string(path).map_err(|source| CompositionError::MarkdownLoad {
        path: path.to_path_buf(), source: super::super::error::MarkdownLoadCause::Read(source),
    })?;
    let markdown = if path.extension().and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("md") || extension.eq_ignore_ascii_case("markdown"))
    {
        Markdown::try_from(path).map_err(|source| CompositionError::MarkdownLoad {
            path: path.to_path_buf(), source: super::super::error::MarkdownLoadCause::Parse(Box::new(source)),
        })?
    } else {
        let loaded = super::data::load_document(path)?;
        let root = expect_object(&loaded.value, "container source", path)?;
        let mut frontmatter = darkmatter::markdown::Frontmatter::new();
        for (key, value) in root {
            frontmatter.insert(&key, value).map_err(|source| CompositionError::MarkdownLoad {
                path: path.to_path_buf(), source: super::super::error::MarkdownLoadCause::Parse(Box::new(source)),
            })?;
        }
        Markdown::with_frontmatter(frontmatter.with_mapping_orders(loaded.orders), "")
    };
    Ok(ResolvedCompositionSource {
        original_ref: path.to_string_lossy().into_owned(), resolved_path: path.to_path_buf(), original_text, markdown,
    })
}

/// Re-read a nested container at its preparation boundary while retaining the
/// original approval graph as the authority for every executable command.
pub(crate) fn prepare_nested_sequence(
    task: &PreflightTask, state: &EffectiveState, context: ComposeContext,
    resolution: &biscuit_file::FileResolutionContext, in_group: bool,
) -> Result<(PreflightSequence, PreflightGraph), CompositionError> {
    let source = load_preparation_source(&task.origin_path)?;
    let document = super::data::load_document(&task.origin_path)?.value;
    let raw_task = document.pointer(task.authored.pointer()).and_then(Value::as_object)
        .ok_or_else(|| CompositionError::SequenceInvalid("nested sequence disappeared before preparation".into()))?;
    let value = raw_task.get("sequence").ok_or_else(|| CompositionError::SequenceInvalid(
        "nested `sequence:` executable changed before preparation".into(),
    ))?;
    let mut loader = Loader::new(&source, context, resolution.clone());
    loader.step_frontmatter = Arc::new(state.data().clone());
    loader.ancestry.push(canonical(&task.origin_path));
    let sequence = loader.load_sequence(
        value, raw_task, &task.origin_path, &task.label, &task.diagnostic.task_property,
        &task.authored, state,
        Placement { position: task.position, in_group },
    )?;
    Ok((sequence, loader.graph))
}

/// Re-read a group's authored executable, including external and catalog references.
pub(crate) fn prepare_nested_group(
    task: &PreflightTask, state: &EffectiveState, context: ComposeContext,
    resolution: &biscuit_file::FileResolutionContext,
) -> Result<(PreflightGroup, PreflightGraph), CompositionError> {
    let source = load_preparation_source(&task.origin_path)?;
    let document = super::data::load_document(&task.origin_path)?.value;
    let raw_task = document.pointer(task.authored.pointer()).and_then(Value::as_object)
        .ok_or_else(|| CompositionError::SequenceInvalid("group disappeared before preparation".into()))?;
    let value = raw_task.get("group").ok_or_else(|| CompositionError::SequenceInvalid(
        "group executable changed before preparation".into(),
    ))?;
    let mut loader = Loader::new(&source, context, resolution.clone());
    loader.step_frontmatter = Arc::new(state.data().clone());
    loader.ancestry.push(canonical(&task.origin_path));
    let group = loader.load_group(
        value, &task.origin_path, &task.label, &property_child(&task.diagnostic.task_property, "group"),
        &task.authored.child("group"), state, task.position,
    )?;
    Ok((group, loader.graph))
}

/// Walk state: the graph under construction plus the ancestry stack and the
/// parsed-document cache shared by independent branches.
struct Loader<'a> {
    graph: PreflightGraph,
    /// Canonical paths currently being expanded, innermost last.
    ancestry: Vec<PathBuf>,
    /// Immutable parsed data files and their authored key order, reusable
    /// across independent branches (spec → *Task Resolution*: sharing parsed
    /// data is allowed; effective parameters and runtime state stay
    /// invocation-local).
    documents: HashMap<PathBuf, (Value, Arc<darkmatter::markdown::MappingOrders>)>,
    source: &'a ResolvedCompositionSource,
    /// The invoking document's canonical path, used for write-back collisions.
    source_path: PathBuf,
    /// Captured once: rebuilding it per shell command would re-probe the
    /// environment for every string in the graph.
    context: ComposeContext,
    file_resolution_context: biscuit_file::FileResolutionContext,
    invocation: Option<&'a InvocationContext>,
    source_contexts: HashMap<PathBuf, SourceContext>,
    /// The frontmatter the current step's early-binding state was built
    /// from, kept on each approval so a task can re-resolve it.
    step_frontmatter: Arc<HashMap<String, Value>>,
    /// The keys of [`Self::step_frontmatter`] whose value reads a late root,
    /// directly or through another such key.
    step_derived: HashSet<String>,
    /// The sequence source's simplified schema, which types the placeholders
    /// of tasks it authors. Loaded on first use.
    source_schema: Option<Option<darkmatter::markdown::schemas::SimplifiedSchema>>,
    /// The invocation's caller values, above the document and below the
    /// step's reserved overlay.
    caller_setters: Map<String, Value>,
}

/// Where in the sequence a task being loaded sits.
#[derive(Clone, Copy)]
struct Placement {
    position: TaskPosition,
    /// Whether the task is a member of a group, the only place `group.*` is
    /// readable.
    in_group: bool,
}

impl<'a> Loader<'a> {
    fn new(
        source: &'a ResolvedCompositionSource,
        context: ComposeContext,
        file_resolution_context: biscuit_file::FileResolutionContext,
    ) -> Self {
        Self {
            graph: PreflightGraph::default(),
            ancestry: Vec::new(),
            documents: HashMap::new(),
            source,
            source_path: canonical(&source.resolved_path),
            context,
            file_resolution_context,
            invocation: None,
            source_contexts: HashMap::new(),
            step_frontmatter: Arc::default(),
            step_derived: HashSet::new(),
            source_schema: None,
            caller_setters: Map::new(),
        }
    }

    fn new_with_invocation(
        source: &'a ResolvedCompositionSource,
        context: ComposeContext,
        invocation: &'a InvocationContext,
        source_context: SourceContext,
    ) -> Self {
        let source_path = canonical(&source.resolved_path);
        Self {
            graph: PreflightGraph::default(),
            ancestry: Vec::new(),
            documents: HashMap::new(),
            source,
            source_path: source_path.clone(),
            context,
            file_resolution_context: source_context.file_resolution_context().clone(),
            invocation: Some(invocation),
            source_contexts: HashMap::from([(source_path, source_context)]),
            step_frontmatter: Arc::default(),
            step_derived: HashSet::new(),
            source_schema: None,
            caller_setters: Map::new(),
        }
    }

    fn walk_plan(&mut self, plan: &SequencePlan) -> Result<(), CompositionError> {
        let source_path = self.source.resolved_path.clone();
        self.graph.capture_source(&source_path);
        let mut authored = crate::composition::prepare::frontmatter_to_value(self.source.markdown.frontmatter());
        if let Value::Object(values) = &mut authored {
            // The plan owns the already-resolved sequence configuration.
            values.extend(self.caller_setters.iter().filter(|(key, _)| key.as_str() != "sequence")
                .map(|(key, value)| (key.clone(), value.clone())));
        }
        crate::composition::lifecycle::bindings::reject_removed_sequence_reads(
            &source_path, crate::composition::lifecycle::bindings::AuthoredDocument {
                frontmatter: &authored,
                body: self.source.markdown.content(), body_line_offset: self.source.markdown.frontmatter_line_count(),
            }, Some(&Value::Object(self.caller_setters.clone())), None,
        )?;
        // The invoking document roots the ancestry, so a task or prompt that
        // references the sequence source itself closes a cycle rather than
        // recursing forever.
        self.ancestry.push(self.source_path.clone());
        for (index, step) in plan.steps.iter().enumerate() {
            let label = format!("step {} (`{}`)", index + 1, step.state.id);
            let property = format!("tasks[{index}]");
            self.step_frontmatter = Arc::new(self.step_frontmatter(plan, index));
            self.step_derived = derived_deferred_keys(
                self.step_frontmatter.iter(),
                &sequence_deferred_roots(),
                &HashSet::new(),
            );
            let state = reapproval::early_binding_state(&self.step_frontmatter, self.context.clone())?;
            // The root sequence is first, so its first step is; a nested
            // sequence would apply the same rule to its own children.
            let placement = Placement {
                position: TaskPosition::First.child(index),
                in_group: false,
            };

            let task = match &step.executable {
                Some(executable) => Some(self.load_step_task(
                    executable,
                    &source_path,
                    &label,
                    &property,
                    &plan.authored.at(index),
                    &state,
                    None,
                    placement,
                )?),
                None => None,
            };

            self.graph.steps.push(PreflightStep {
                index,
                id: step.state.id.clone(),
                name: step.name.clone(),
                task,
            });
        }
        Ok(())
    }

    /// The early-binding frontmatter for one step: the invoking document's
    /// frontmatter, then the caller's values, under this step's reserved
    /// overlay.
    ///
    /// This is exactly the state a shell command sees at its turn, minus the
    /// runtime mutation layer — which is why a later task's command is
    /// approved only as a template, read again when it runs.
    fn step_frontmatter(&self, plan: &SequencePlan, index: usize) -> HashMap<String, Value> {
        let overlay = super::build_step_overlay(plan, index);
        let mut frontmatter: HashMap<String, Value> = self
            .source
            .markdown
            .frontmatter()
            .as_map()
            .iter()
            // Composition reads a stored literal token as its text.
            .map(|(k, v)| (k.clone(), crate::composition::closure::stored_text(v)))
            .collect();
        frontmatter.extend(self.caller_setters.iter().map(|(k, v)| (k.clone(), v.clone())));
        if let Value::Object(overrides) = overlay.as_set_overrides(None) {
            frontmatter.extend(overrides);
        }
        frontmatter
    }

    /// Expand a step's declared executable into a task node.
    #[allow(clippy::too_many_arguments)]
    fn load_step_task(
        &mut self,
        executable: &StepExecutable,
        origin: &Path,
        label: &str,
        property: &str,
        authored: &AuthoredOrder,
        state: &EffectiveState,
        group_defaults: Option<&GroupDefaults>,
        placement: Placement,
    ) -> Result<PreflightTask, CompositionError> {
        self.load_task(
            executable.field,
            &executable.value,
            &executable.options,
            origin,
            label,
            property,
            authored,
            state,
            group_defaults,
            placement,
        )
    }

    /// Expand one executable field plus its options into a task node.
    ///
    /// `property` is the source-rooted semantic path of the task and `authored`
    /// is its JSON Pointer in the *same* document — the two travel together and
    /// are both reset when the walk crosses into another file.
    #[allow(clippy::too_many_arguments)]
    fn load_task(
        &mut self,
        field: ExecutableField,
        value: &Value,
        options: &Map<String, Value>,
        origin: &Path,
        label: &str,
        property: &str,
        authored: &AuthoredOrder,
        state: &EffectiveState,
        group_defaults: Option<&GroupDefaults>,
        placement: Placement,
    ) -> Result<PreflightTask, CompositionError> {
        let origin_dir = origin
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));

        let mut preflight_approvals = Vec::new();
        let mut shell_sites = Vec::new();
        let action = match field {
            ExecutableField::Prompt => {
                let reference = expect_reference(value, "prompt", label)?;
                let document = self.load_prompt(&reference, origin)?;
                self.record_prompt_reference(
                    &document,
                    &option_map(options, "params"),
                    origin,
                    state,
                    placement,
                );
                PreflightAction::Prompt {
                    path: document,
                    reference,
                }
            }
            ExecutableField::Shell => {
                let site = SiteScope {
                    origin,
                    label,
                    property: property_child(property, field.key()),
                    state,
                    placement,
                };
                let (commands, approvals, sites) = self.resolve_shell_commands(value, &site)?;
                preflight_approvals = approvals;
                shell_sites = sites;
                PreflightAction::Shell { commands }
            }
            ExecutableField::SideEffect => PreflightAction::SideEffect {
                action: value.clone(),
                authored_set_order: authored
                    .child(field.key())
                    .child("set")
                    .owned_key_order(),
            },
            ExecutableField::Group => {
                let group = self.load_group(
                    value,
                    origin,
                    label,
                    &property_child(property, field.key()),
                    &authored.child(field.key()),
                    state,
                    placement.position,
                )?;
                PreflightAction::Group(group)
            }
            ExecutableField::Sequence => {
                PreflightAction::Sequence(Box::new(self.load_sequence(
                    value, options, origin, label, property, authored, state, placement,
                )?))
            }
            ExecutableField::Task => {
                let reference = expect_reference(value, "task", label)?;
                return self.load_external_task(
                    &reference,
                    origin,
                    label,
                    state,
                    group_defaults,
                    placement,
                );
            }
        };

        // Group `operation`/`flow` are defaults; a task-level value overrides
        // them. Both are meaningless outside a prompt task, and the option
        // catalog already rejects an explicit one — so a group default is
        // simply not propagated to non-prompt siblings rather than erroring.
        let prompt_task = matches!(action, PreflightAction::Prompt { .. });
        let operation = option_string(options, "operation")
            .or_else(|| prompt_task.then(|| group_defaults.and_then(|d| d.operation.clone())).flatten());
        let flow = option_string(options, "flow")
            .or_else(|| prompt_task.then(|| group_defaults.and_then(|d| d.flow.clone())).flatten());

        let mut task = PreflightTask {
            name: option_string(options, "name"),
            label: label.to_string(),
            action,
            params: option_map(options, "params"),
            timeout: options.get("timeout").cloned(),
            operation,
            flow,
            setup: options.get("setup").cloned(),
            teardown: options.get("teardown").cloned(),
            origin_dir,
            origin_path: origin.to_path_buf(),
            authored: authored.clone(),
            diagnostic: TaskDiagnosticProvenance {
                source_path: origin.to_path_buf(),
                action_property: property_child(property, field.key()),
                task_property: property.to_string(),
            },
            approved_stack_commands: Vec::new(),
            preflight_approvals,
            position: placement.position,
            shell_sites,
            approved_set_values: Vec::new(),
        };

        let site = SiteScope {
            origin,
            label,
            property: property.to_string(),
            state,
            placement,
        };
        self.collect_lifecycle_shell(&mut task, &site)?;
        Ok(task)
    }

    #[allow(clippy::too_many_arguments)]
    fn load_sequence(
        &mut self, value: &Value, options: &Map<String, Value>, origin: &Path,
        label: &str, property: &str, authored: &AuthoredOrder,
        state: &EffectiveState, placement: Placement,
    ) -> Result<PreflightSequence, CompositionError> {
        use super::grammar::{SequenceSourceSpec, classify_source};
        let (plan, child_origin, child_authored, events, entered) = match classify_source(value)? {
            SequenceSourceSpec::Inline(items) => {
                let mut plan = super::normalize::normalize_plan(
                    &items, super::model::SequenceSource::Inline, origin, true,
                )?;
                plan.authored = authored.child("sequence");
                (plan, origin.to_path_buf(), authored.child("sequence"), options.clone(), false)
            }
            SequenceSourceSpec::Reference(reference) => {
                let path = self.resolve_reference(&reference.reference, origin)?;
                self.enter(&path)?;
                let (document, order) = self.load_data_document(&path)?;
                let context = self.source_context_for(origin);
                let resolution = context.as_ref().map(SourceContext::file_resolution_context)
                    .unwrap_or(&self.file_resolution_context);
                let globals = self.step_frontmatter.iter().map(|(k,v)| (k.clone(),v.clone())).collect();
                let plan = source_resolution::load_referenced_sequence(
                    &reference, &path, origin, &globals, true, resolution,
                )?;
                let events = if reference.offset.is_none() && reference.operator.is_none() {
                    document.as_object().cloned().unwrap_or_default()
                } else { Map::new() };
                let child_authored = if let Some(offset) = &reference.offset {
                    offset.split('.').fold(order, |cursor, segment| cursor.child(segment))
                } else if document.is_array() { order } else { order.child("sequence") };
                (plan, path, child_authored, events, true)
            }
            _ => return Err(CompositionError::SequenceInvalid(
                "a nested `sequence:` must be a list or file reference".into(),
            )),
        };
        let mut event_root = Map::new();
        for key in super::grammar::CONTAINER_EVENT_KEYS {
            if let Some(value) = events.get(*key) { event_root.insert((*key).into(), value.clone()); }
        }
        let lifecycle = parse_lifecycle_config(&Value::Object(event_root), &child_origin)?;
        let mut steps = Vec::with_capacity(plan.steps.len());
        for (index, step) in plan.steps.iter().enumerate() {
            let child_label = format!("{label} → step {} (`{}`)", index + 1, step.state.id);
            let child_property = if entered { format!("sequence[{index}]") }
                else { property_child(property, &format!("sequence[{index}]")) };
            let task = step.executable.as_ref().map(|executable| self.load_step_task(
                executable, &child_origin, &child_label, &child_property,
                &child_authored.at(index), state, None,
                Placement { position: placement.position.child(index), in_group: placement.in_group },
            )).transpose()?;
            steps.push(PreflightStep { index, id: step.state.id.clone(), name: step.name.clone(), task });
        }
        if entered { self.leave(); }
        Ok(PreflightSequence { plan, steps, lifecycle: Box::new(lifecycle), origin_path: child_origin })
    }

    /// Expand a `task: <file-ref>` reference into the task it names.
    ///
    /// The reference is exclusive and immutable at the referencing site: the
    /// referenced file supplies the executable *and* every option, so nothing
    /// from the referencing step is merged in.
    fn load_external_task(
        &mut self,
        reference: &str,
        origin: &Path,
        label: &str,
        state: &EffectiveState,
        group_defaults: Option<&GroupDefaults>,
        placement: Placement,
    ) -> Result<PreflightTask, CompositionError> {
        let path = self.resolve_reference(reference, origin)?;
        let (document, authored) = self.load_data_document(&path)?;
        let root = expect_object(&document, "task file", &path)?;

        expect_kind(&root, "task", &path)?;
        let (field, value, options) = parse_task_object(&root, &path)?;

        let nested_label = format!(
            "{label} → task `{}`",
            biscuit_file::to_portable_string(&path)
        );
        self.enter(&path)?;
        let result = self.load_task(
            field,
            &value,
            &options,
            &path,
            &nested_label,
            "",
            &authored,
            state,
            group_defaults,
            placement,
        );
        self.leave();
        result
    }

    /// Resolve a `group:` value into a group node.
    #[allow(clippy::too_many_arguments)]
    fn load_group(
        &mut self,
        value: &Value,
        origin: &Path,
        label: &str,
        property: &str,
        authored: &AuthoredOrder,
        state: &EffectiveState,
        position: TaskPosition,
    ) -> Result<PreflightGroup, CompositionError> {
        match value {
            Value::Object(map) => {
                self.build_group(map, origin, label, property, authored, state, position)
            }
            Value::String(raw) => {
                let (name, reference) = split_catalog_reference(raw);
                let path = self.resolve_reference(&reference, origin)?;
                let (document, document_authored) = self.load_data_document(&path)?;
                let root = expect_object(&document, "group file", &path)?;

                // A catalog selects one named entry, so the group's authored
                // root is that entry's pointer, not the file root.
                let (group_map, group_authored) = match name {
                    Some(name) => {
                        let (index, map) = select_catalog_group(&root, &name, &path)?;
                        (map, document_authored.child("groups").at(index))
                    }
                    None => {
                        expect_kind(&root, "group", &path)?;
                        (root.clone(), document_authored)
                    }
                };

                let nested_label = format!(
                    "{label} → group `{}`",
                    biscuit_file::to_portable_string(&path)
                );
                self.enter(&path)?;
                let result = self.build_group(
                    &group_map,
                    &path,
                    &nested_label,
                    "",
                    &group_authored,
                    state,
                    position,
                );
                self.leave();
                result
            }
            other => Err(CompositionError::SequenceGroupInvalid {
                group: "<inline>".to_string(),
                origin: origin.to_path_buf(),
                problem: format!(
                    "`group:` must be an inline group object or a file reference, got {}",
                    json_type_name(other)
                ),
            }),
        }
    }

    /// Validate a group definition and expand its tasks.
    #[allow(clippy::too_many_arguments)]
    fn build_group(
        &mut self,
        map: &Map<String, Value>,
        origin: &Path,
        label: &str,
        property: &str,
        authored: &AuthoredOrder,
        state: &EffectiveState,
        position: TaskPosition,
    ) -> Result<PreflightGroup, CompositionError> {
        let name = match map.get("name") {
            Some(Value::String(s)) => s.clone(),
            _ => "<inline>".to_string(),
        };
        let invalid = |problem: String| CompositionError::SequenceGroupInvalid {
            group: name.clone(),
            origin: origin.to_path_buf(),
            problem,
        };

        // Container loops belong to the later loops-and-flow-control feature.
        if map.contains_key("loop") {
            return Err(CompositionError::SequenceUnsupportedConstruct {
                construct: format!("group `loop` (in group `{name}`)"),
                detail: "iteration commit semantics for group loops are not ratified yet; \
                         remove `loop:` or express the repetition as separate sequence steps"
                    .to_string(),
            });
        }

        if let Some(value) = map.get("execution") {
            return Err(super::grammar::removed_task_key("execution", value, origin));
        }
        let fail_fast = match map.get("fail_fast") {
            None => true,
            Some(Value::Bool(value)) => *value,
            Some(_) => return Err(invalid("`fail_fast` must be a boolean".into())),
        };
        let execution = GroupExecution::Parallel;
        let mut event_root = Map::new();
        for key in super::grammar::CONTAINER_EVENT_KEYS {
            if let Some(value) = map.get(*key) { event_root.insert((*key).into(), value.clone()); }
        }
        let lifecycle = parse_lifecycle_config(&Value::Object(event_root), origin)?;
        let owner = self.documents.get(origin).map(|(value, _)| value.clone())
            .unwrap_or_else(|| crate::composition::prepare::frontmatter_to_value(self.source.markdown.frontmatter()));
        crate::composition::lifecycle::bindings::reject_removed_sequence_reads(
            origin, crate::composition::lifecycle::bindings::AuthoredDocument {
                frontmatter: &owner, body: "", body_line_offset: 0,
            }, Some(&Value::Object(self.caller_setters.clone())), Some(&lifecycle),
        )?;

        let max_parallel = match map.get("max_parallel") {
            None | Some(Value::Null) => None,
            Some(value) => {
                let cap = value.as_u64().filter(|n| *n >= 1).ok_or_else(|| {
                    invalid(format!(
                        "`max_parallel` must be an integer >= 1, got {}",
                        render_scalar(value)
                    ))
                })?;
                Some(cap as usize)
            }
        };

        let variables = match map.get("variables") {
            None => Map::new(),
            Some(Value::Object(map)) => map.clone(),
            Some(_) => return Err(invalid("`variables` must be an object".into())),
        };
        for key in ["legs", "succeeded", "failed", "cancelled"] {
            if variables.contains_key(key) { return Err(invalid(format!("`variables.{key}` is reserved for group outcomes; choose another variable name"))); }
        }
        let defaults = GroupDefaults {
            operation: option_string(map, "operation"),
            flow: option_string(map, "flow"),
        };

        let raw_tasks = match map.get("tasks") {
            Some(Value::Array(items)) if !items.is_empty() => items.clone(),
            Some(Value::Array(_)) | None => {
                return Err(invalid("`tasks` must declare at least one task".to_string()));
            }
            Some(other) => {
                return Err(invalid(format!(
                    "`tasks` must be a list, got {}",
                    json_type_name(other)
                )));
            }
        };

        let mut tasks = Vec::with_capacity(raw_tasks.len());
        let mut leg_names = std::collections::HashSet::new();
        for (index, raw) in raw_tasks.iter().enumerate() {
            let task_map = raw.as_object().ok_or_else(|| {
                invalid(format!(
                    "task {} must be an object, got {}",
                    index + 1,
                    json_type_name(raw)
                ))
            })?;
            let leg_name = task_map.get("name").and_then(Value::as_str)
                .filter(|name| !name.trim().is_empty())
                .ok_or_else(|| invalid(format!("tasks[{index}].name must be a non-empty string; every leg needs a name")))?;
            if !leg_names.insert(leg_name) { return Err(invalid(format!("duplicate leg name `{leg_name}` at tasks[{index}]; use unique names"))); }
            let (field, value, options) = parse_task_object(task_map, origin)?;
            let task_label = format!("{label} → task {} of group `{name}`", index + 1);
            let task_property = property_child(property, &format!("tasks[{index}]"));
            let mut leg = self.load_task(
                field,
                &value,
                &options,
                origin,
                &task_label,
                &task_property,
                &authored.child("tasks").at(index),
                state,
                Some(&defaults),
                // Legs are not ordered: every leg of a first group is first.
                Placement {
                    position,
                    in_group: true,
                },
            )?;
            leg.name = Some(leg_name.to_string());
            tasks.push(leg);
        }

        let group = PreflightGroup {
            name,
            execution,
            fail_fast,
            lifecycle: Box::new(lifecycle),
            max_parallel,
            variables,
            tasks,
            origin_path: origin.to_path_buf(),
        };

        if group.execution == GroupExecution::Parallel {
            self.check_write_back_collisions(&group)?;
        }
        Ok(group)
    }

    /// Reject two siblings in a parallel group that would rewrite one file.
    ///
    /// The sequence source document counts as an occupied target: an
    /// inline-compose sequence rewrites its own body between steps, so a
    /// concurrent task pointed at it would race the orchestrator itself.
    fn check_write_back_collisions(
        &self,
        group: &PreflightGroup,
    ) -> Result<(), CompositionError> {
        let mut claimed: HashMap<PathBuf, String> = HashMap::new();
        if self.source_inline_compose() {
            claimed.insert(self.source_path.clone(), "the sequence source".to_string());
        }

        for task in &group.tasks {
            let owner = task.name.clone().unwrap_or_else(|| task.label.clone());
            let mut pending = vec![task];
            let mut targets = std::collections::BTreeSet::new();
            while let Some(task) = pending.pop() {
                match &task.action {
                    PreflightAction::Prompt { path, .. } => {
                        if self.graph.prompt_document(path).is_some_and(|document| document.inline_compose) {
                            targets.insert(path.clone());
                        }
                    }
                    PreflightAction::Group(nested) => pending.extend(nested.tasks.iter()),
                    PreflightAction::Sequence(nested) => {
                        pending.extend(nested.steps.iter().filter_map(|step| step.task.as_ref()));
                        if nested.steps.iter().any(|step| step.task.is_none()) {
                            let inline = self.documents.get(&nested.origin_path)
                                .is_some_and(|(document, _)| document.get("prompt").is_some_and(Value::is_string))
                                || (nested.origin_path == self.source_path && self.source_inline_compose());
                            if inline { targets.insert(nested.origin_path.clone()); }
                        }
                    }
                    PreflightAction::Shell { .. } | PreflightAction::SideEffect { .. } => {}
                }
            }
            for path in targets {
                if let Some(first) = claimed.get(&path) {
                    return Err(CompositionError::SequenceWriteBackCollision {
                        group: group.name.clone(), target: path,
                        first: first.clone(), second: owner.clone(),
                    });
                }
                claimed.insert(path, owner.clone());
            }
        }
        Ok(())
    }

    fn source_inline_compose(&self) -> bool {
        matches!(
            self.source.markdown.frontmatter().as_map().get("prompt"),
            Some(Value::String(_))
        )
    }

    /// Load a `prompt:` document as a graph leaf.
    ///
    /// A prompt remains a leaf. A referenced document with `sequence:` needs
    /// the explicit `sequence:` executable so its container owns the lifecycle.
    fn load_prompt(&mut self, reference: &str, origin: &Path) -> Result<PathBuf, CompositionError> {
        let path = self.resolve_reference(reference, origin)?;
        self.graph.capture_source(&path);
        if let Some(existing) = self.graph.prompt_document(&path) {
            let _ = existing;
            return Ok(path);
        }
        self.check_cycle(&path)?;

        // The referenced document is loaded through the ordinary Markdown load
        // error so a malformed prompt reports the same typed parse failure it
        // would when composed directly, rather than a preflight-shaped string.
        let markdown = Markdown::try_from(path.as_path()).map_err(|e| {
            CompositionError::MarkdownLoad {
                path: path.clone(),
                source: super::super::error::MarkdownLoadCause::Parse(Box::new(e)),
            }
        })?;

        let frontmatter = markdown.frontmatter();
        if frontmatter.as_map().contains_key("sequence") {
            return Err(CompositionError::SequenceNestedSequence {
                path: path.clone(),
                referenced_from: origin.to_path_buf(),
            });
        }
        let inline_compose = matches!(frontmatter.as_map().get("prompt"), Some(Value::String(_)));
        let has_schema = frontmatter.as_map().contains_key("$schema");

        self.graph.prompt_documents.push(PromptDocument {
            path: path.clone(),
            markdown,
            inline_compose,
            has_schema,
        });
        Ok(path)
    }

    /// Prepare a task's `shell:` value: one site per command, with the bytes
    /// a resolved-byte (or approval-time-only) command runs and an approval
    /// record for each one that interpolates.
    fn resolve_shell_commands(
        &mut self,
        value: &Value,
        scope: &SiteScope<'_>,
    ) -> Result<PreparedCommands, CompositionError> {
        let label = scope.label;
        let origin = scope.origin;
        let raw: Vec<String> = match value {
            Value::String(s) => vec![s.clone()],
            Value::Array(items) => items
                .iter()
                .map(|item| match item {
                    Value::String(s) => Ok(s.clone()),
                    other => Err(CompositionError::SequenceReferenceInvalid {
                        context: format!("shell task in {label}"),
                        path: origin.to_path_buf(),
                        problem: format!(
                            "every `shell` entry must be a string, got {}",
                            json_type_name(other)
                        ),
                    }),
                })
                .collect::<Result<_, _>>()?,
            other => {
                return Err(CompositionError::SequenceReferenceInvalid {
                    context: format!("shell task in {label}"),
                    path: origin.to_path_buf(),
                    problem: format!(
                        "`shell` must be a string or a list of strings, got {}",
                        json_type_name(other)
                    ),
                });
            }
        };

        let mut commands = Vec::with_capacity(raw.len());
        let mut approvals = Vec::new();
        let mut sites = Vec::with_capacity(raw.len());
        for (index, command) in raw.into_iter().enumerate() {
            let prepared = self.prepare_site(
                &command,
                ShellSiteKind::Command,
                TaskStage::Primary,
                &format!("{}[{index}]", TaskStage::Primary.key()),
                scope,
            )?;
            // A command with a late value has no bytes; its authored text
            // stands in, and its site's template mode is what runs.
            commands.push(prepared.bytes.clone().unwrap_or(command));
            approvals.extend(prepared.approval);
            sites.push(prepared.site);
        }
        Ok((commands, approvals, sites))
    }

    /// Prepare every shell site of a task's `setup:`/`teardown:` stacks: each
    /// `shell` action's `command` and `on_error`, and each `set` value written
    /// as a whole-value `$( … )`. A site with resolved bytes is recorded on the
    /// task so execution runs exactly those bytes.
    ///
    /// Traversal is condition-blind: a `when:`-guarded action contributes its
    /// command regardless of the guard, because the guard may read differently
    /// at the task's turn.
    fn collect_lifecycle_shell(
        &mut self,
        task: &mut PreflightTask,
        scope: &SiteScope<'_>,
    ) -> Result<(), CompositionError> {
        for (stage, stack) in [
            (TaskStage::Setup, task.setup.clone()),
            (TaskStage::Teardown, task.teardown.clone()),
        ] {
            let Some(stack) = stack else { continue };
            let stage_label = format!("{} `{}`", scope.label, stage.key());
            let stage_scope = SiteScope {
                label: &stage_label,
                property: property_child(&scope.property, stage.key()),
                ..*scope
            };
            for (index, (kind, raw)) in shape::collect_stack_shell_sites(&stack).into_iter().enumerate() {
                let id = format!("{}[{index}]", stage.key());
                let prepared = self.prepare_site(&raw, kind.clone(), stage, &id, &stage_scope)?;
                task.preflight_approvals.extend(prepared.approval);
                match (&kind, prepared.bytes, prepared.set_value) {
                    (ShellSiteKind::SetValue { key }, _, Some(value)) => {
                        task.approved_set_values.push(ApprovedSetValue {
                            site: CommandSiteId::new(stage, raw.clone()),
                            key: key.clone(),
                            value,
                        });
                    }
                    (ShellSiteKind::Command, Some(bytes), _) => {
                        task.approved_stack_commands
                            .push(ApprovedCommand::new(CommandSiteId::new(stage, raw.clone()), bytes));
                    }
                    _ => {}
                }
                task.shell_sites.push(prepared.site);
            }
        }
        Ok(())
    }

    /// Prepare one shell site: decide how it is approved, check every value
    /// it reads at approval, and record it for approval.
    ///
    /// A command with no placeholder is approved and runs as its literal
    /// bytes. A command with a placeholder is approved as resolved bytes only
    /// in the first task and only when every value it reads exists now;
    /// otherwise — a later task, or a late value (`outputs`, `loop`, `group`,
    /// or a step value derived from one) — it is approved only by its typed
    /// shape. Either way every approval-time value is resolved here, and a
    /// missing one (`null`) fails preflight before any step runs.
    ///
    /// A first-task command's resolved bytes are what it runs: a target-identity
    /// root (`ctx.agent`, `ctx.model`, `env.AGENT`, `env.MODEL`) is rejected
    /// rather than expanded, because this graph phase runs before per-task
    /// target selection. Per-task and JIT audits — where the selected target's
    /// environment is available — continue to permit those roots.
    fn prepare_site(
        &mut self,
        raw: &str,
        kind: ShellSiteKind,
        stage: TaskStage,
        id: &str,
        scope: &SiteScope<'_>,
    ) -> Result<PreparedSite, CompositionError> {
        let label = scope.label;
        let origin = scope.origin;
        let site = |authored: &str, template, mode| PreparedShellSite {
            id: id.to_string(),
            stage,
            kind: kind.clone(),
            authored: authored.to_string(),
            template,
            mode,
            position: scope.placement.position,
            source_file: origin.to_path_buf(),
            property: scope.property.clone(),
            label: label.to_string(),
        };

        // The command a set value runs is the one inside `$( … )`.
        let (authored, parsed) = match &kind {
            ShellSiteKind::Command => (raw.to_string(), parse_authored(raw)),
            ShellSiteKind::SetValue { key } => {
                let ctx = biscuit_terminal::errors::SourceContext::new(
                    origin.to_path_buf(),
                    origin.to_path_buf(),
                    raw.to_string(),
                );
                darkmatter::markdown::compose::authored_shell_template(raw, key, &ctx).ok_or_else(
                    || CompositionError::SequenceShellResolution {
                        command: raw.to_string(),
                        task: label.to_string(),
                        message: "a `set` value starting with `$(` must be a whole-value `$( … )` \
                                  command"
                            .to_string(),
                        source: None,
                    },
                )?
            }
        };

        if !raw.contains("{{") {
            let (bytes, set_value) = self.record_resolved_bytes(raw, raw, &kind, scope)?;
            return Ok(PreparedSite {
                site: site(&authored, None, SiteMode::ResolvedBytes { bytes: bytes.clone() }),
                bytes: Some(bytes),
                approval: None,
                set_value,
            });
        }

        self.refuse_unapprovable_reads(raw, scope)?;

        // Type each placeholder against the authoring document's schema and
        // mark the ones that read a late value.
        let roots = sequence_deferred_roots();
        let schema = self.owner_schema(origin);
        let parsed = parsed.map(|mut template| {
            template.project_types(schema.as_ref(), &SEQUENCE_HOST_GLOBALS);
            mark_late_holes(&mut template, &roots, &self.step_derived);
            template
        });
        let holes: Vec<(String, bool)> = match &parsed {
            Ok(template) => template
                .placeholders()
                .map(|hole| {
                    (
                        hole.expression.clone(),
                        matches!(hole.timing, HoleTiming::Deferred { .. }),
                    )
                })
                .collect(),
            Err(_) => darkmatter::markdown::compose::expression::ExpressionFinder::scan_plain(raw)
                .expressions
                .into_iter()
                .map(|location| {
                    let late = roots.late_roots_of(&location.expression, &self.step_derived).is_some();
                    (location.expression, late)
                })
                .collect(),
        };
        let any_late = holes.iter().any(|(_, late)| *late);
        let template_required = any_late || scope.placement.position == TaskPosition::Later;
        let template_result = parsed.clone();
        let parsed = match parsed {
            Ok(template) => Some(template),
            Err(reason) if template_required => {
                return Err(self.site_failure(
                    raw,
                    scope,
                    ShellExpansionError::TemplateGrammar {
                        ctx: Box::new(self.authored_context(origin, &authored)),
                        command: authored.clone(),
                        reason,
                        origin: site_origin(&scope.property),
                    },
                    "it must run as a template, but it is not a valid template".to_string(),
                ));
            }
            Err(_) => None,
        };

        // Every approval-time value, and the command's bytes when no value is
        // late, resolve in one pass so the placeholders keep their types: a
        // `null` refuses the command, while an empty string, zero, and
        // `false` are present.
        let approval_time: Vec<&str> = holes
            .iter()
            .filter(|(_, late)| !late)
            .map(|(expression, _)| expression.as_str())
            .collect();
        let resolved = self.resolve_approval_time(raw, !any_late, &approval_time, scope)?;
        if let Some(expression) = resolved.missing {
            return Err(self.site_failure(
                raw,
                scope,
                ShellExpansionError::MissingValue {
                    ctx: Box::new(self.authored_context(origin, &authored)),
                    command: authored.clone(),
                    expression: expression.to_string(),
                    origin: site_origin(&scope.property),
                },
                format!(
                    "it reads `{expression}`, which is missing; a shell command never runs with a \
                     missing value"
                ),
            ));
        }

        if !template_required {
            let bytes = resolved.bytes.expect("a site with no late value resolves its bytes");
            let command_start = self.graph.shell_commands.len();
            let (bytes, set_value) = self.record_resolved_bytes(raw, &bytes, &kind, scope)?;
            self.graph.template_candidates.push(ShellCandidate {
                authored_command: authored.clone(),
                site: CommandSite {
                    source_file: origin.to_path_buf(),
                    origin: site_origin(&scope.property),
                    owner: Some(canonical(origin).to_string_lossy().into_owned()),
                },
                template: template_result,
                resolved: Some(self.graph.shell_commands[command_start..].iter()
                    .map(|command| command.command.clone()).collect()),
                requirement: ModeRequirement::AnyMode,
            });
            return Ok(PreparedSite {
                site: site(&authored, parsed, SiteMode::ResolvedBytes { bytes: bytes.clone() }),
                bytes: Some(bytes),
                approval: resolved.approval.map(|record| record.into_approval(stage)),
                set_value,
            });
        }

        let template = parsed.expect("a template-required site parsed or failed above");
        self.graph.template_candidates.push(ShellCandidate {
            authored_command: authored.clone(),
            site: CommandSite {
                source_file: origin.to_path_buf(),
                origin: site_origin(&scope.property),
                owner: Some(canonical(origin).to_string_lossy().into_owned()),
            },
            template: Ok(template.clone()),
            resolved: None,
            requirement: ModeRequirement::TemplateRequired,
        });
        Ok(PreparedSite {
            site: site(&authored, Some(template), SiteMode::Template),
            bytes: None,
            approval: None,
            set_value: None,
        })
    }

    /// Refuse a shell command that reads a value no approval can cover: `err`,
    /// `timing`, or `current` (even beside a late root), `group` outside a
    /// group, a target-identity root, or an unknown function.
    fn refuse_unapprovable_reads(&self, raw: &str, scope: &SiteScope<'_>) -> Result<(), CompositionError> {
        let label = scope.label;
        // The sequence approval scope declares every value that does not exist
        // yet unavailable, and `current` is refused with them, in any branch.
        // The late roots are approved as templates instead; every other refusal
        // stands. A parse failure is left to the resolution below, which
        // reports it typed.
        let scope_id = LifecycleScope::SequenceShellApproval;
        if let Ok(diagnostics) = approval_diagnostics(raw, scope_id) {
            for diagnostic in diagnostics {
                let root = unavailable_root(&diagnostic.error).map(str::to_string);
                let late = match root.as_deref() {
                    Some("group") => scope.placement.in_group,
                    Some(root) => SEQUENCE_LATE_ROOTS.contains(&root),
                    None => false,
                };
                if late {
                    continue;
                }
                return Err(match root {
                    Some(root) => CompositionError::SequenceShellLateBinding {
                        command: raw.to_string(),
                        root,
                        task: label.to_string(),
                        source: Some(Box::new(diagnostic.error)),
                    },
                    None => CompositionError::SequenceShellResolution {
                        command: raw.to_string(),
                        task: label.to_string(),
                        message: diagnostic.error.to_string(),
                        source: Some(Box::new(super::super::preflight::approval_failure(
                            raw,
                            diagnostic.error,
                        ))),
                    },
                });
            }
        }
        if let Some(root) = shape::first_target_identity_root(raw) {
            return Err(CompositionError::SequenceShellTargetIdentity {
                command: raw.to_string(),
                root,
                task: label.to_string(),
            });
        }
        Ok(())
    }

    /// Resolve the approval-time placeholders of `raw` — and, when
    /// `with_bytes`, `raw` itself — against the step's early-binding values.
    fn resolve_approval_time<'h>(
        &mut self,
        raw: &str,
        with_bytes: bool,
        holes: &[&'h str],
        scope: &SiteScope<'_>,
    ) -> Result<ApprovalTimeValues<'h>, CompositionError> {
        // Runtime values come from the graph's launch context — the caller's
        // launch repository and package area — extended in place with any
        // group this command needs. Each origin document's own
        // `SourceContext` still supplies file resolution; it never supplies
        // launch-facing `ctx.*`.
        let source_context = self.source_context_for(scope.origin);
        let file_resolution_context = source_context
            .as_ref()
            .map(SourceContext::file_resolution_context)
            .unwrap_or(&self.file_resolution_context)
            .clone();
        if let Some(invocation) = self.invocation {
            let requirements = darkmatter::markdown::compose::ContextRequirements::for_content(raw);
            invocation.extend_launch_context(&mut self.context, &requirements);
        }
        let runtime_context = self.context.clone();
        let value = Value::Array(
            with_bytes
                .then(|| Value::String(raw.to_string()))
                .into_iter()
                .chain(holes.iter().map(|hole| Value::String(format!("{{{{ {hole} }}}}"))))
                .collect(),
        );
        let composed = reapproval::compose_approval_value(
            &value,
            scope.origin,
            scope.state,
            &runtime_context,
            &file_resolution_context,
        )?
        .map_err(|error| reapproval::shell_resolution_failure(raw, scope.label, error))?;
        let Value::Array(mut composed) = composed else {
            unreachable!("an array subtree composes to an array");
        };
        let reads = composed.split_off(usize::from(with_bytes));
        let missing = holes
            .iter()
            .zip(&reads)
            .find_map(|(hole, read)| read.is_null().then_some(*hole));
        let bytes = composed.pop().map(|resolved| match resolved {
            Value::String(text) => text,
            other => other.to_string(),
        });
        let approval = bytes.as_ref().map(|bytes| reapproval::ApprovalRecordOwned {
            authored: raw.to_string(),
            approved: bytes.clone(),
            origin: scope.origin.to_path_buf(),
            label: scope.label.to_string(),
            frontmatter: Arc::clone(&self.step_frontmatter),
            state_context: scope.state.context().clone(),
            runtime_context: runtime_context.clone(),
            file_resolution_context: file_resolution_context.clone(),
        });
        Ok(ApprovalTimeValues {
            bytes,
            missing,
            approval,
        })
    }

    /// Record a site approved as resolved bytes: the command, or for a `set`
    /// value every command it can run. Returns the command text the site runs
    /// and, for a `set` value, its fixed value.
    fn record_resolved_bytes(
        &mut self,
        raw: &str,
        resolved: &str,
        kind: &ShellSiteKind,
        scope: &SiteScope<'_>,
    ) -> Result<(String, Option<darkmatter::markdown::compose::ResolvedShellValue>), CompositionError> {
        let ShellSiteKind::SetValue { key } = kind else {
            self.graph.shell_commands.push(DiscoveredCommand {
                command: resolved.to_string(),
                source_file: scope.origin.to_path_buf(),
                origin: scope.label.to_string(),
            });
            return Ok((resolved.to_string(), None));
        };
        let source_context = self.source_context_for(scope.origin);
        let file_resolution_context = source_context
            .as_ref()
            .map(SourceContext::file_resolution_context)
            .unwrap_or(&self.file_resolution_context)
            .clone();
        let resolution = super::super::document_expression_resolution_context(
            scope.origin,
            Some(&self.context),
            &file_resolution_context,
            None,
        )?;
        // The authored value supplies the structure; the resolved one only its
        // bytes, so interpolation cannot change an executable or add a command.
        let value = darkmatter::markdown::compose::ResolvedShellValue::resolve(
            key,
            raw,
            resolved,
            (*self.step_frontmatter).clone(),
            &self.context,
            resolution,
        )
        .map_err(|error| CompositionError::SequenceShellResolution {
            command: raw.to_string(),
            task: scope.label.to_string(),
            message: crate::composition::lifecycle::actions::shell_value_error_message(&error),
            source: Some(Box::new(darkmatter::markdown::MarkdownError::ShellExpansion(Box::new(error)))),
        })?;
        for command in value.commands() {
            self.graph.shell_commands.push(DiscoveredCommand {
                command,
                source_file: scope.origin.to_path_buf(),
                origin: format!("{} `set.{key}`", scope.label),
            });
        }
        Ok((resolved.to_string(), Some(value)))
    }

    /// A typed site failure carrying Darkmatter's shell error as its cause.
    fn site_failure(
        &self,
        raw: &str,
        scope: &SiteScope<'_>,
        error: ShellExpansionError,
        message: String,
    ) -> CompositionError {
        CompositionError::SequenceShellResolution {
            command: raw.to_string(),
            task: scope.label.to_string(),
            message,
            source: Some(Box::new(darkmatter::markdown::MarkdownError::ShellExpansion(Box::new(error)))),
        }
    }

    fn authored_context(&self, origin: &Path, authored: &str) -> biscuit_terminal::errors::SourceContext {
        biscuit_terminal::errors::SourceContext::new(
            origin.to_path_buf(),
            origin.to_path_buf(),
            authored.to_string(),
        )
    }

    /// The simplified schema of the document that authored a task, which
    /// types its placeholders. Only the sequence source declares one; a
    /// `kind: task`/`kind: group` data file has none, so its placeholders fall
    /// back to `string`.
    fn owner_schema(&mut self, origin: &Path) -> Option<darkmatter::markdown::schemas::SimplifiedSchema> {
        if canonical(origin) != self.source_path {
            return None;
        }
        if self.source_schema.is_none() {
            // A schema that does not load is reported by the step's own
            // composition; here it only means no declared types.
            let loaded = crate::composition::schema::load_authored_schema(
                self.source,
                None,
                &self.file_resolution_context,
                &darkmatter::markdown::compose::CallerInputRecords::default(),
            )
            .ok()
            .flatten()
            .and_then(|effective| effective.simplified);
            self.source_schema = Some(loaded);
        }
        self.source_schema.clone().flatten()
    }

    /// Record a prompt task's reference to its document, with the position
    /// and parameters its discovery needs.
    ///
    /// A parameter whose value reads a late root (directly or through a step
    /// value derived from one) is late inside the document too. Every other
    /// parameter is evaluated now, against the step's early-binding values, so
    /// the document's commands see it as they will at the task's turn; one
    /// that cannot be evaluated here is treated as late rather than failing
    /// preflight, because `params` otherwise bind only at the task's turn.
    fn record_prompt_reference(
        &mut self,
        path: &Path,
        params: &Map<String, Value>,
        origin: &Path,
        state: &EffectiveState,
        placement: Placement,
    ) {
        let mut deferred: Vec<String> =
            derived_deferred_keys(params.iter(), &sequence_deferred_roots(), &self.step_derived)
                .into_iter()
                .filter(|key| params.contains_key(key))
                .collect();
        let mut evaluated = Map::new();
        for (key, value) in params {
            if deferred.contains(key) {
                continue;
            }
            let source_context = self.source_context_for(origin);
            let file_resolution_context = source_context
                .as_ref()
                .map(SourceContext::file_resolution_context)
                .unwrap_or(&self.file_resolution_context)
                .clone();
            match reapproval::compose_approval_value(
                value,
                origin,
                state,
                &self.context,
                &file_resolution_context,
            ) {
                Ok(Ok(resolved)) => {
                    evaluated.insert(key.clone(), resolved);
                }
                _ => deferred.push(key.clone()),
            }
        }
        deferred.sort();
        let reference = PromptReference {
            in_group: placement.in_group,
            path: path.to_path_buf(),
            position: placement.position,
            params: evaluated,
            deferred_params: deferred,
        };
        if !self.graph.prompt_references.contains(&reference) {
            self.graph.prompt_references.push(reference);
        }
    }

    /// Resolve a reference authored inside `origin` to an absolute path.
    fn resolve_reference(
        &mut self,
        reference: &str,
        origin: &Path,
    ) -> Result<PathBuf, CompositionError> {
        let source_context = self.source_context_for(origin);
        let file_resolution_context = source_context
            .as_ref()
            .map(SourceContext::file_resolution_context)
            .unwrap_or(&self.file_resolution_context);
        source_resolution::resolve_sequence_reference(reference, origin, file_resolution_context)
            .map(|p| canonical(&p))
    }

    fn source_context_for(&mut self, origin: &Path) -> Option<SourceContext> {
        let invocation = self.invocation?;
        let key = canonical(origin);
        if let Some(context) = self.source_contexts.get(&key) {
            return Some(context.clone());
        }
        let context = invocation
            .derive_source(&key)
            .expect("resolved sequence document always has a parent directory");
        self.source_contexts.insert(key, context.clone());
        Some(context)
    }

    /// Load and cache a data document (`kind: task`/`group`/`group-catalog`).
    ///
    /// The returned cursor is rooted at that document, not at the referencing
    /// one: crossing a file boundary replaces both the order source and the
    /// pointer, exactly as it already resets the semantic `property` path.
    fn load_data_document(
        &mut self,
        path: &Path,
    ) -> Result<(Value, AuthoredOrder), CompositionError> {
        self.graph.capture_source(path);
        if let Some((value, orders)) = self.documents.get(path) {
            return Ok((value.clone(), AuthoredOrder::at_root(Arc::clone(orders))));
        }
        let loaded = super::data::load_document(path)?;
        crate::composition::lifecycle::bindings::reject_removed_sequence_reads(
            path, crate::composition::lifecycle::bindings::AuthoredDocument {
                frontmatter: &loaded.value, body: "", body_line_offset: 0,
            }, Some(&Value::Object(self.caller_setters.clone())), None,
        )?;
        let orders = Arc::new(loaded.orders);
        self.documents
            .insert(path.to_path_buf(), (loaded.value.clone(), Arc::clone(&orders)));
        Ok((loaded.value, AuthoredOrder::at_root(orders)))
    }

    /// Reject a reference back to a document already being expanded, reporting
    /// the whole chain rather than only the repeated file.
    fn check_cycle(&self, path: &Path) -> Result<(), CompositionError> {
        if self.ancestry.iter().any(|p| p == path) {
            let mut chain = self.ancestry.clone();
            chain.push(path.to_path_buf());
            return Err(CompositionError::SequenceReferenceCycle { chain });
        }
        Ok(())
    }

    /// Push `path` onto the ancestry stack after a cycle check.
    fn enter(&mut self, path: &Path) -> Result<(), CompositionError> {
        if self.ancestry.iter().any(|p| p == path) {
            let mut chain = self.ancestry.clone();
            chain.push(path.to_path_buf());
            return Err(CompositionError::SequenceReferenceCycle { chain });
        }
        self.ancestry.push(path.to_path_buf());
        Ok(())
    }

    fn leave(&mut self) {
        self.ancestry.pop();
    }
}

/// Join a source-rooted property to one of its children, treating the empty
/// parent as the document root rather than producing a leading dot.
pub(crate) fn property_child(parent: &str, child: &str) -> String {
    if parent.is_empty() {
        child.to_string()
    } else {
        format!("{parent}.{child}")
    }
}

/// A `shell:` task's commands as the runner receives them, the approval
/// records of those resolved to bytes, and one site per command.
type PreparedCommands = (Vec<String>, Vec<PreflightApproval>, Vec<PreparedShellSite>);

/// Where the shell sites of one task are being prepared.
struct SiteScope<'s> {
    origin: &'s Path,
    label: &'s str,
    /// The source-rooted property of the site's field.
    property: String,
    state: &'s EffectiveState,
    placement: Placement,
}

/// One prepared site and what its task records about it.
struct PreparedSite {
    site: PreparedShellSite,
    /// The bytes the task's runner runs today: a resolved-byte site's
    /// approved bytes, or a later command's approval-time bytes until
    /// template-mode dispatch replaces them.
    bytes: Option<String>,
    approval: Option<PreflightApproval>,
    set_value: Option<darkmatter::markdown::compose::ResolvedShellValue>,
}

/// The approval-time reads of one site.
struct ApprovalTimeValues<'h> {
    bytes: Option<String>,
    /// The first approval-time placeholder that read `null`.
    missing: Option<&'h str>,
    approval: Option<reapproval::ApprovalRecordOwned>,
}

/// Mark each placeholder of `template` that reads a late root, directly or
/// through a step value derived from one.
fn mark_late_holes(template: &mut CommandTemplate, roots: &DeferredShellRoots, derived: &HashSet<String>) {
    use darkmatter::markdown::compose::shell_expansion::template::ArgPart;
    for segment in &mut template.segments {
        for arg in &mut segment.args {
            for part in &mut arg.parts {
                if let ArgPart::Hole(hole) = part
                    && let Some(late) = roots.late_roots_of(&hole.expression, derived)
                {
                    hole.timing = HoleTiming::Deferred { roots: late };
                }
            }
        }
    }
}

/// Group-level `operation`/`flow` defaults handed down to member tasks.
struct GroupDefaults {
    operation: Option<String>,
    flow: Option<String>,
}

/// Split `{group-name}@{file-ref}` into its parts.
///
/// The split is on the *first* `@`, so `build-group@@catalogs/all.yaml` names
/// group `build-group` in the magic reference `@catalogs/all.yaml`. A leading
/// `@` is a magic file reference with no group name, and a string with no `@`
/// at all is a plain `kind: group` file reference.
fn split_catalog_reference(raw: &str) -> (Option<String>, String) {
    let trimmed = raw.trim();
    match trimmed.find('@') {
        Some(0) | None => (None, trimmed.to_string()),
        Some(at) => (
            Some(trimmed[..at].trim().to_string()),
            trimmed[at + 1..].trim().to_string(),
        ),
    }
}

/// Find the one group named `name` in a `kind: group-catalog` document.
///
/// ## Returns
///
/// The group's position in the catalog's `groups:` list — which is what anchors
/// its authored-order pointer — and the group itself.
fn select_catalog_group(
    root: &Map<String, Value>,
    name: &str,
    path: &Path,
) -> Result<(usize, Map<String, Value>), CompositionError> {
    expect_kind(root, "group-catalog", path)?;

    let groups = match root.get("groups") {
        Some(Value::Array(items)) => items,
        other => {
            return Err(CompositionError::SequenceReferenceInvalid {
                context: "group catalog".to_string(),
                path: path.to_path_buf(),
                problem: format!(
                    "a `kind: group-catalog` document must declare a `groups:` list, got {}",
                    other.map_or("nothing", json_type_name)
                ),
            });
        }
    };

    let available: Vec<String> = groups
        .iter()
        .filter_map(|g| g.get("name").and_then(Value::as_str))
        .map(str::to_string)
        .collect();

    let matches: Vec<(usize, &Value)> = groups
        .iter()
        .enumerate()
        .filter(|(_, g)| g.get("name").and_then(Value::as_str) == Some(name))
        .collect();

    let lookup_failure = |problem: &str| CompositionError::SequenceGroupCatalogLookup {
        name: name.to_string(),
        path: path.to_path_buf(),
        problem: problem.to_string(),
        available: available.clone(),
    };

    match matches.as_slice() {
        [(index, only)] => only
            .as_object()
            .cloned()
            .map(|group| (*index, group))
            .ok_or_else(|| lookup_failure("is not an object")),
        [] => Err(lookup_failure("is not defined")),
        _ => Err(lookup_failure("is defined more than once")),
    }
}

/// Parse a task object into its single executable field, that field's value,
/// and its validated option map.
fn parse_task_object(
    map: &Map<String, Value>,
    origin: &Path,
) -> Result<(ExecutableField, Value, Map<String, Value>), CompositionError> {
    let invalid = |problem: String| CompositionError::SequenceReferenceInvalid {
        context: "task".to_string(),
        path: origin.to_path_buf(),
        problem,
    };

    super::grammar::validate_task_grammar(map, origin)?;
    let executables: Vec<&str> = map
        .keys()
        .map(String::as_str)
        .filter(|k| reserved::is_executable_key(k))
        .collect();
    let field = match executables.as_slice() {
        [only] => ExecutableField::from_key(only).expect("filtered to executable keys"),
        [] => {
            return Err(invalid(format!(
                "a task must declare exactly one of {}",
                reserved::EXECUTABLE_KEYS.join(", ")
            )));
        }
        many => {
            return Err(invalid(format!(
                "a task declares exactly one executable; found {}",
                many.join(", ")
            )));
        }
    };

    let allowed = reserved::allowed_task_options(field);
    let mut options = Map::new();
    for (key, value) in map {
        if reserved::is_executable_key(key) || key == "kind" {
            continue;
        }
        if !reserved::is_task_option_key(key) {
            return Err(invalid(format!(
                "`{key}` is not a task field; expected one of {}",
                reserved::TASK_OPTION_KEYS.join(", ")
            )));
        }
        if !allowed.contains(&key.as_str()) {
            return Err(invalid(format!(
                "`{key}` is meaningless for a `{}` task",
                field.key()
            )));
        }
        options.insert(key.clone(), value.clone());
    }

    Ok((field, map[field.key()].clone(), options))
}

/// Require a document's `kind:` to be `expected`.
fn expect_kind(
    root: &Map<String, Value>,
    expected: &str,
    path: &Path,
) -> Result<(), CompositionError> {
    match root.get("kind") {
        Some(Value::String(kind)) if kind == expected => Ok(()),
        other => Err(CompositionError::SequenceReferenceInvalid {
            context: format!("`kind: {expected}` document"),
            path: path.to_path_buf(),
            problem: match other {
                Some(Value::String(kind)) => format!("declares `kind: {kind}`"),
                Some(other) => format!("`kind` must be a string, got {}", json_type_name(other)),
                None => format!("does not declare `kind: {expected}`"),
            },
        }),
    }
}

fn expect_object(
    document: &Value,
    context: &str,
    path: &Path,
) -> Result<Map<String, Value>, CompositionError> {
    document
        .as_object()
        .cloned()
        .ok_or_else(|| CompositionError::SequenceReferenceInvalid {
            context: context.to_string(),
            path: path.to_path_buf(),
            problem: format!("root must be an object, got {}", json_type_name(document)),
        })
}

fn expect_reference(value: &Value, key: &str, label: &str) -> Result<String, CompositionError> {
    match value {
        Value::String(s) if !s.trim().is_empty() => Ok(s.trim().to_string()),
        other => Err(CompositionError::SequenceReferenceInvalid {
            context: format!("`{key}:` in {label}"),
            path: PathBuf::new(),
            problem: format!(
                "must be a non-empty file reference, got {}",
                json_type_name(other)
            ),
        }),
    }
}

fn option_string(map: &Map<String, Value>, key: &str) -> Option<String> {
    match map.get(key) {
        Some(Value::String(s)) => Some(s.clone()),
        _ => None,
    }
}

fn option_map(map: &Map<String, Value>, key: &str) -> Map<String, Value> {
    match map.get(key) {
        Some(Value::Object(m)) => m.clone(),
        _ => Map::new(),
    }
}

/// Render a scalar for an error message without the quoting `Display` on
/// `Value` would add for strings.
fn render_scalar(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// Canonicalize when the filesystem allows it, so two spellings of one file
/// compare equal in the ancestry stack and the collision map.
///
/// Legacy-spelling canonicalization is load-bearing: `source_path` later
/// re-enters reference resolution, and the grammar rejects a verbatim
/// `\\?\` spelling as a device-prefix path.
fn canonical(path: &Path) -> PathBuf {
    biscuit_file::canonicalize_simplified(path).unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests;

#[cfg(test)]
pub(crate) mod runtime_fixture;
