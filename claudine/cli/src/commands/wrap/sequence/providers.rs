//! Per-task provider selection for a sequence run.
//!
//! Preflight predicts each `prompt:` task's provider from its own `agent`
//! ([`claudine::composition::sequence::preflight::providers`]). This module
//! turns those predictions into the target each task prepares under:
//!
//! - **own** — the task's static `agent` (or `--<provider>`), resolved now
//!   by the canonical non-interactive rule; a hint it cannot resolve fails
//!   the run before any task starts;
//! - **fallback** — no `agent` of its own: the nearest enclosing sequence
//!   document's `agent`, or the review screen's choice;
//! - **at launch** — an `agent` only the task's turn can evaluate; it is
//!   resolved from the document's evaluated frontmatter before its body
//!   renders.
//!
//! The launch-time rebuild stays authoritative: a document whose fresh
//! `agent` differs from the target still launches on its own `agent`, and
//! the status line reports what launched. The task runner applies the same
//! rule before composing, so the prompt's `env.AGENT`/`env.MODEL` and the
//! child's environment name that launch.

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use claudine::composition::sequence::preflight::providers::{
    self, HintState, ProviderPrediction, TaskProviderClass,
};
use claudine::composition::sequence::preflight::{
    PreflightAction, PreflightGraph, PreflightTask, TaskIdentity,
};
use claudine::composition::{
    self, AgentResolutionState, CompositionError, EffectiveSelectionHints,
    InstalledProviderSnapshot, ModelResolutionReason, ProviderPickerPlan,
    ProviderResolutionReason, ResolvedExecutionTarget, SequenceStepDraft,
    SequenceTaskProviderProblem,
};
use claudine::invocation_context::DocumentEnvironment;
use claudine::model_catalog::ModelCatalogService;
use claudine::provider::Provider;
use serde_json::{Map, Value};

use super::super::composition::{ModelResolveMode, resolve_document_model};

/// One provider-launching unit of the graph: a `prompt:` task, or a step
/// that composes its sequence document's body.
#[derive(Debug, Clone)]
pub(super) struct TaskSite {
    pub(super) identity: TaskIdentity,
    /// The zero-based root step the site belongs to, which its review row is
    /// labeled with.
    pub(super) step: usize,
    /// The row label: the task's name, or its generated label.
    pub(super) name: String,
    /// The label diagnostics locate the task with.
    pub(super) label: String,
    /// The document that authored the task.
    pub(super) task_path: PathBuf,
    /// The enclosing containers, outermost first.
    pub(super) containers: Vec<String>,
    pub(super) own: OwnHints,
    /// Enclosing sequence documents' hints, nearest first, each with the
    /// document that wrote it.
    pub(super) enclosing: Vec<(PathBuf, EffectiveSelectionHints)>,
}

/// Where a site's own `agent` comes from.
#[derive(Debug, Clone)]
pub(super) enum OwnHints {
    /// A `prompt:` task's prediction.
    Task(ProviderPrediction),
    /// A body step composes its sequence document, so that document's hints
    /// are its own. `None` when the document names no `agent`.
    Body(Option<(PathBuf, EffectiveSelectionHints)>),
    /// A body step composing a referenced sequence document (named here)
    /// whose static `agent` is `[]`, which no caller setter replaces: the
    /// launch schema would reject the composed document after earlier tasks
    /// ran, so selection fails it instead.
    BodyEmptyAgentList(PathBuf),
}

/// How a task's provider is decided.
#[derive(Debug, Clone)]
pub(super) enum TaskTarget {
    /// The task's own provider, or the one `--<provider>` pins.
    Own(ResolvedExecutionTarget),
    /// No provider of its own: an enclosing document's, or the review choice.
    /// The task's composed `agent`, if one appears at launch, still wins.
    Fallback(ResolvedExecutionTarget),
    /// The task's own `agent` is evaluated when the task is prepared; the
    /// fallback applies only if the composed document then names none.
    AtLaunch(Option<ResolvedExecutionTarget>),
    /// Missing a provider, or an ambiguous list: a terminal reviews it, and
    /// anything else fails before any task starts.
    Unresolved {
        state: AgentResolutionState,
        plan: ProviderPickerPlan,
        /// Whether the task names the list itself (so the failure is the
        /// task's, not the sequence document's).
        own_hint: bool,
    },
}

impl TaskTarget {
    /// The target the task prepares under, before its turn decides anything.
    pub(super) fn planned(&self) -> Option<&ResolvedExecutionTarget> {
        match self {
            Self::Own(target) | Self::Fallback(target) => Some(target),
            Self::AtLaunch(fallback) => fallback.as_ref(),
            Self::Unresolved { .. } => None,
        }
    }
}

/// One site and how its provider is decided.
#[derive(Debug, Clone)]
pub(super) struct TaskSelection {
    pub(super) site: TaskSite,
    pub(super) target: TaskTarget,
    /// The hints its model is re-selected from when its document captures
    /// its own environment.
    pub(super) hints: EffectiveSelectionHints,
}

/// Every task's selection, keyed by its graph identity.
#[derive(Debug, Clone, Default)]
pub(super) struct TaskSelections {
    selections: Vec<TaskSelection>,
    index: HashMap<TaskIdentity, usize>,
}

impl TaskSelections {
    fn new(selections: Vec<TaskSelection>) -> Self {
        let index = selections
            .iter()
            .enumerate()
            .map(|(position, selection)| (selection.site.identity.clone(), position))
            .collect();
        Self { selections, index }
    }

    pub(super) fn get(&self, identity: &TaskIdentity) -> Option<&TaskSelection> {
        self.index.get(identity).map(|position| &self.selections[*position])
    }

    pub(super) fn iter(&self) -> impl Iterator<Item = &TaskSelection> {
        self.selections.iter()
    }

    /// The first site no rule resolved.
    pub(super) fn first_unresolved(&self) -> Option<&TaskSelection> {
        self.selections
            .iter()
            .find(|selection| matches!(selection.target, TaskTarget::Unresolved { .. }))
    }

    /// Replace each reviewed site's target with the choice made for it: a
    /// fallback, never an authored `agent`.
    pub(super) fn apply_review(&mut self, reviewed: Vec<(TaskIdentity, ResolvedExecutionTarget)>) {
        for (identity, target) in reviewed {
            if let Some(position) = self.index.get(&identity) {
                self.selections[*position].target = TaskTarget::Fallback(target);
            }
        }
    }

    /// The planned target of root step `index`, for the step's own
    /// composition: its body site's, or its task's when the task is a
    /// `prompt:` task. A container step plans per child.
    pub(super) fn root_target(&self, index: usize) -> Option<ResolvedExecutionTarget> {
        self.get(&TaskIdentity::root(index))
            .and_then(|selection| selection.target.planned().cloned())
    }
}

/// What resolving a hint reads.
pub(super) struct SelectionInputs<'a> {
    pub(super) snapshot: &'a InstalledProviderSnapshot,
    pub(super) catalog: &'a ModelCatalogService,
    pub(super) cli_model: Option<&'a str>,
    pub(super) explicit_provider: Option<Provider>,
    pub(super) favorite: Option<Provider>,
    pub(super) environment: &'a DocumentEnvironment,
    pub(super) silent: bool,
    /// A preview renders an unusable hint instead of failing on it.
    pub(super) dry_run: bool,
}

/// Every provider-launching site of `graph`, in launch order. `root_hints`
/// are the sequence document's hints with the caller's `--set` applied;
/// `caller_sets_agent` is whether that `--set` names `agent`.
pub(super) fn collect_sites(
    graph: &PreflightGraph,
    plan: &composition::SequencePlan,
    root_path: &Path,
    root_hints: &EffectiveSelectionHints,
    caller_sets_agent: bool,
) -> Vec<TaskSite> {
    let root_document = root_hints.names_agent().then(|| (root_path.to_path_buf(), root_hints.clone()));
    let mut sites = Vec::new();
    for (index, step) in graph.steps.iter().enumerate() {
        match &step.task {
            None => sites.push(TaskSite {
                identity: TaskIdentity::root(index),
                step: index,
                name: plan.steps.get(index).map_or_else(|| step.name.clone(), |s| s.name.clone()),
                label: format!("step {} (`{}`)", index + 1, step.id),
                task_path: root_path.to_path_buf(),
                containers: Vec::new(),
                own: OwnHints::Body(root_document.clone()),
                enclosing: Vec::new(),
            }),
            Some(task) => {
                visit(task, index, &step.name, &[], root_document.as_slice(), caller_sets_agent, &mut sites);
            }
        }
    }
    sites
}

/// `step_name` names a task that sits directly in a sequence step; a group
/// leg is named by its own `name:`.
fn visit(
    task: &PreflightTask,
    root_step: usize,
    step_name: &str,
    containers: &[String],
    enclosing: &[(PathBuf, EffectiveSelectionHints)],
    caller_sets_agent: bool,
    sites: &mut Vec<TaskSite>,
) {
    let name = task.name.clone().unwrap_or_else(|| step_name.to_string());
    match &task.action {
        PreflightAction::Prompt { .. } => {
            let Some(prediction) = &task.provider else { return };
            sites.push(TaskSite {
                identity: task.identity.clone(),
                step: root_step,
                name,
                label: task.label.clone(),
                task_path: task.origin_path.clone(),
                containers: containers.to_vec(),
                own: OwnHints::Task(prediction.clone()),
                enclosing: enclosing.to_vec(),
            });
        }
        PreflightAction::Sequence(sequence) => {
            // A referenced sequence document's own `agent` applies to its
            // children ahead of every outer document's.
            let own = sequence
                .frontmatter
                .as_ref()
                .and_then(|frontmatter| hints_from_map(frontmatter).ok())
                .filter(EffectiveSelectionHints::names_agent)
                .map(|hints| (sequence.origin_path.clone(), hints));
            let empty_agent_list = !caller_sets_agent
                && sequence
                    .frontmatter
                    .as_ref()
                    .and_then(|frontmatter| frontmatter.get("agent"))
                    .and_then(Value::as_array)
                    .is_some_and(Vec::is_empty);
            let inner: Vec<(PathBuf, EffectiveSelectionHints)> =
                own.into_iter().chain(enclosing.iter().cloned()).collect();
            let mut nested = containers.to_vec();
            nested.push(format!("sequence `{name}`"));
            for (index, step) in sequence.steps.iter().enumerate() {
                match &step.task {
                    // A body step composes the nearest sequence document.
                    None => sites.push(TaskSite {
                        identity: task.identity.sequence_child(index),
                        step: root_step,
                        name: step.name.clone(),
                        label: format!("{} → {}", task.label, step.name),
                        task_path: sequence.origin_path.clone(),
                        containers: nested.clone(),
                        own: if empty_agent_list {
                            OwnHints::BodyEmptyAgentList(sequence.origin_path.clone())
                        } else {
                            OwnHints::Body(inner.first().cloned())
                        },
                        enclosing: Vec::new(),
                    }),
                    Some(child) => visit(child, root_step, &step.name, &nested, &inner, caller_sets_agent, sites),
                }
            }
        }
        PreflightAction::Group(group) => {
            let mut nested = containers.to_vec();
            nested.push(format!("group `{}`", group.name));
            for leg in &group.tasks {
                visit(leg, root_step, &leg.label, &nested, enclosing, caller_sets_agent, sites);
            }
        }
        PreflightAction::Shell { .. } | PreflightAction::SideEffect { .. } => {}
    }
}

/// Decide every site's target.
///
/// ## Errors
///
/// [`CompositionError::SequenceTaskProviderSelection`] for the first task, in
/// launch order, whose own hint can never select a provider: a name that is
/// unknown or not installed, a list with no installed member, a wrong-typed
/// hint, an evaluation failure, an `agent: []`, or a hint that reads its own
/// identity. A task whose `when:` will be false is checked like any other.
///
/// A provider flag makes every `agent` hint's provider irrelevant, and
/// `--model`, a provider-specific model variable, or `MODEL` every `model`
/// hint's model, so a hint reading its own identity fails only when no such
/// source decides its key. The other failures stand under any override: the
/// task's prompt still evaluates the hint when it composes.
#[allow(clippy::result_large_err)]
pub(super) fn select(
    sites: Vec<TaskSite>,
    inputs: &SelectionInputs<'_>,
) -> Result<TaskSelections, CompositionError> {
    let mut warned: BTreeSet<Provider> = BTreeSet::new();
    let mut selections = Vec::with_capacity(sites.len());
    for site in sites {
        let (target, hints) = select_one(&site, inputs, &mut warned)?;
        selections.push(TaskSelection { site, target, hints });
    }
    Ok(TaskSelections::new(selections))
}

#[allow(clippy::result_large_err)]
fn select_one(
    site: &TaskSite,
    inputs: &SelectionInputs<'_>,
    warned: &mut BTreeSet<Provider>,
) -> Result<(TaskTarget, EffectiveSelectionHints), CompositionError> {
    let mut own = own_hints(site)?;
    // Provider and model precedence apply separately: a circular hint blocks
    // only when nothing above it decides its key.
    if inputs.explicit_provider.is_none()
        && let Some(circular) = own.circular_agent.take()
    {
        return Err(circular.error(site));
    }
    let circular_model = own.circular_model.take();
    let (target, hints) = select_target(site, own, inputs, warned)?;
    if let Some(circular) = circular_model
        && !model_decided_above_hints(&target, inputs)
    {
        return Err(circular.error(site));
    }
    Ok((target, hints))
}

/// Whether a source above every `model` hint decides the model: `--model`,
/// a provider-specific model variable, or `MODEL`. For a task whose provider
/// is not known yet, only the provider-independent sources count.
fn model_decided_above_hints(target: &TaskTarget, inputs: &SelectionInputs<'_>) -> bool {
    match target {
        TaskTarget::Own(target) | TaskTarget::Fallback(target) => matches!(
            target.model_reason,
            ModelResolutionReason::ExplicitCli
                | ModelResolutionReason::ProviderEnv(_)
                | ModelResolutionReason::GenericEnv
        ),
        TaskTarget::AtLaunch(_) | TaskTarget::Unresolved { .. } => {
            inputs.cli_model.is_some()
                || inputs.environment.lookup("MODEL").is_some_and(|model| !model.is_empty())
        }
    }
}

#[allow(clippy::result_large_err)]
fn select_target(
    site: &TaskSite,
    own: OwnSelection,
    inputs: &SelectionInputs<'_>,
    warned: &mut BTreeSet<Provider>,
) -> Result<(TaskTarget, EffectiveSelectionHints), CompositionError> {
    if let Some(provider) = inputs.explicit_provider {
        let target = target_for(provider, ProviderResolutionReason::ExplicitFlag, &own.hints, inputs, warned);
        return Ok((TaskTarget::Own(target), own.hints));
    }
    match own.class {
        TaskProviderClass::Predicted => {
            let state = composition::classify_agent_resolution(&own.hints, inputs.snapshot);
            if let Some(provider) = auto_selected(&state) {
                let reason = provider_reason(&own.hints);
                let target = target_for(provider, reason, &own.hints, inputs, warned);
                return Ok((TaskTarget::Own(target), own.hints));
            }
            // A body step's hints are its sequence document's: an unusable
            // `agent` there is the document's to review or to fail with, as it
            // is for a document composed directly. So is an ambiguous list,
            // and every hint a preview only renders.
            let body = matches!(site.own, OwnHints::Body(_));
            if body || inputs.dry_run || matches!(state, AgentResolutionState::ListMultipleInstalled { .. }) {
                let plan = picker_plan(&state, &own.hints, inputs);
                return Ok((TaskTarget::Unresolved { state, plan, own_hint: !body }, own.hints));
            }
            Err(selection_error(site, &own, SequenceTaskProviderProblem::Unresolvable {
                state,
                installed: inputs.snapshot.runnable.clone(),
            }))
        }
        TaskProviderClass::DecidedAtLaunch => {
            let fallback = enclosing_target(site, &own.hints, inputs, warned);
            Ok((TaskTarget::AtLaunch(fallback.map(|(target, _)| target)), own.hints))
        }
        TaskProviderClass::Missing => match enclosing_target(site, &own.hints, inputs, warned) {
            Some((target, hints)) => Ok((TaskTarget::Fallback(target), hints)),
            None => {
                let state = site
                    .enclosing
                    .first()
                    .map_or(AgentResolutionState::NoAgent, |(_, hints)| {
                        composition::classify_agent_resolution(hints, inputs.snapshot)
                    });
                let plan = picker_plan(&state, &own.hints, inputs);
                Ok((TaskTarget::Unresolved { state, plan, own_hint: false }, own.hints))
            }
        },
    }
}

/// A site's own hints, parsed by the canonical selection-hint parser.
struct OwnSelection {
    class: TaskProviderClass,
    /// The static hints; a circular key is left out.
    hints: EffectiveSelectionHints,
    /// Where the `agent` (or failing hint) was written.
    hint_path: PathBuf,
    property: String,
    /// An `agent` reading the identity it selects, which fails only when no
    /// provider flag replaces it.
    circular_agent: Option<CircularHint>,
    /// The same for `model`, replaced by `--model` or a model variable.
    circular_model: Option<CircularHint>,
}

/// A hint that reads the identity it selects, and where it was written.
struct CircularHint {
    variable: String,
    hint_path: PathBuf,
    property: String,
}

impl CircularHint {
    fn from_state(state: &HintState) -> Option<Self> {
        let HintState::Circular { source, variable } = state else { return None };
        Some(Self {
            variable: variable.clone(),
            hint_path: source.source_path.clone(),
            property: source.property.clone(),
        })
    }

    fn error(&self, site: &TaskSite) -> CompositionError {
        task_error(
            site,
            self.hint_path.clone(),
            self.property.clone(),
            SequenceTaskProviderProblem::Circular(self.variable.clone()),
        )
    }
}

#[allow(clippy::result_large_err)]
fn own_hints(site: &TaskSite) -> Result<OwnSelection, CompositionError> {
    match &site.own {
        OwnHints::Body(None) => Ok(OwnSelection {
            class: TaskProviderClass::Missing,
            hints: EffectiveSelectionHints::default(),
            hint_path: site.task_path.clone(),
            property: "agent".into(),
            circular_agent: None,
            circular_model: None,
        }),
        OwnHints::Body(Some((path, hints))) => Ok(OwnSelection {
            class: TaskProviderClass::Predicted,
            hints: hints.clone(),
            hint_path: path.clone(),
            property: "agent".into(),
            circular_agent: None,
            circular_model: None,
        }),
        OwnHints::BodyEmptyAgentList(path) => Err(task_error(
            site,
            path.clone(),
            "agent".into(),
            SequenceTaskProviderProblem::EmptyAgentList,
        )),
        OwnHints::Task(prediction) => {
            let (hint_path, property) = hint_location(prediction, site);
            // A failed or empty-list hint fails even under an override: the
            // composed document still evaluates it, so the launch would fail
            // on it after earlier tasks ran. A circular one is only a
            // selection input; [`select_one`] decides whether it matters.
            for state in [&prediction.agent, &prediction.model] {
                let problem = match state {
                    HintState::Failed { cause, .. } => SequenceTaskProviderProblem::Evaluation(cause.clone()),
                    HintState::EmptyAgentList { .. } => SequenceTaskProviderProblem::EmptyAgentList,
                    _ => continue,
                };
                let (hint_path, property) = state_location(state).unwrap_or((hint_path, property));
                return Err(task_error(site, hint_path, property, problem));
            }
            let hints = hints_from_map(&prediction.static_hints()).map_err(|error| {
                let (key, found) = match error {
                    CompositionError::AgentHintWrongType(found) => ("agent", found),
                    CompositionError::ModelHintWrongType(found) => ("model", found),
                    other => return other,
                };
                task_error(site, hint_path.clone(), property.clone(), SequenceTaskProviderProblem::WrongType { key, found })
            })?;
            Ok(OwnSelection {
                class: prediction.class(),
                hints,
                hint_path,
                property,
                circular_agent: CircularHint::from_state(&prediction.agent),
                circular_model: CircularHint::from_state(&prediction.model),
            })
        }
    }
}

/// The file and property of the hint that decides `prediction`: its
/// `agent`'s, or its `model`'s when the task names no `agent`.
fn hint_location(prediction: &ProviderPrediction, site: &TaskSite) -> (PathBuf, String) {
    state_location(&prediction.agent)
        .or_else(|| state_location(&prediction.model))
        .unwrap_or_else(|| (site.task_path.clone(), "agent".into()))
}

fn state_location(state: &HintState) -> Option<(PathBuf, String)> {
    match state {
        HintState::Absent => None,
        HintState::Static { source, .. }
        | HintState::DecidedAtLaunch { source, .. }
        | HintState::Failed { source, .. }
        | HintState::Circular { source, .. }
        | HintState::EmptyAgentList { source } => Some((source.source_path.clone(), source.property.clone())),
    }
}

/// The typed failure for a task whose `agent`, decided at launch, selects no
/// provider when its document is prepared.
pub(super) fn launch_error(selection: &TaskSelection, problem: SequenceTaskProviderProblem) -> CompositionError {
    let (hint_path, property) = match &selection.site.own {
        OwnHints::Task(prediction) => hint_location(prediction, &selection.site),
        OwnHints::Body(_) | OwnHints::BodyEmptyAgentList(_) => (selection.site.task_path.clone(), "agent".into()),
    };
    task_error(&selection.site, hint_path, property, problem)
}

fn selection_error(site: &TaskSite, own: &OwnSelection, problem: SequenceTaskProviderProblem) -> CompositionError {
    task_error(site, own.hint_path.clone(), own.property.clone(), problem)
}

fn task_error(site: &TaskSite, hint_path: PathBuf, property: String, problem: SequenceTaskProviderProblem) -> CompositionError {
    CompositionError::SequenceTaskProviderSelection {
        task: site.label.clone(),
        task_path: site.task_path.clone(),
        hint_path,
        property,
        containers: site.containers.clone(),
        problem: Box::new(problem),
    }
}

/// The nearest enclosing sequence document whose `agent` resolves without a
/// prompt, as the target and the hints its model is chosen from: the
/// document's `agent`, and the task's own `model` above the document's.
fn enclosing_target(
    site: &TaskSite,
    own: &EffectiveSelectionHints,
    inputs: &SelectionInputs<'_>,
    warned: &mut BTreeSet<Provider>,
) -> Option<(ResolvedExecutionTarget, EffectiveSelectionHints)> {
    site.enclosing.iter().find_map(|(_, document)| {
        let state = composition::classify_agent_resolution(document, inputs.snapshot);
        let provider = auto_selected(&state)?;
        let hints = EffectiveSelectionHints {
            model: own.model.clone().or_else(|| document.model.clone()),
            ..document.clone()
        };
        let target = target_for(provider, provider_reason(document), &hints, inputs, warned);
        Some((target, hints))
    })
}

/// The provider a classified state selects without prompting: only
/// [`AgentResolutionState::Selected`] and
/// [`AgentResolutionState::ListOneInstalled`] do; every other state prompts on
/// a terminal and fails without one.
pub(super) fn auto_selected(state: &AgentResolutionState) -> Option<Provider> {
    match state {
        AgentResolutionState::Selected { provider } => Some(*provider),
        AgentResolutionState::ListOneInstalled { selected, .. } => Some(*selected),
        _ => None,
    }
}

fn provider_reason(hints: &EffectiveSelectionHints) -> ProviderResolutionReason {
    if hints.agent_was_list {
        ProviderResolutionReason::FrontmatterList
    } else {
        ProviderResolutionReason::FrontmatterSingle
    }
}

/// The target for `provider`, its model resolved by the shared resolver
/// without refreshing any catalog; the unknown-model warning fires once per
/// provider.
fn target_for(
    provider: Provider,
    provider_reason: ProviderResolutionReason,
    hints: &EffectiveSelectionHints,
    inputs: &SelectionInputs<'_>,
    warned: &mut BTreeSet<Provider>,
) -> ResolvedExecutionTarget {
    let first = warned.insert(provider);
    let (model, model_reason) = resolve_document_model(
        inputs.catalog,
        provider,
        hints,
        inputs.cli_model,
        ModelResolveMode { refresh: false, warn: first && !inputs.silent },
        inputs.environment,
    );
    ResolvedExecutionTarget { provider, provider_reason, model, model_reason }
}

fn picker_plan(
    state: &AgentResolutionState,
    hints: &EffectiveSelectionHints,
    inputs: &SelectionInputs<'_>,
) -> ProviderPickerPlan {
    super::super::composition::scoped_picker_plan_for_state(state, hints, inputs.snapshot, inputs.favorite)
        .unwrap_or(ProviderPickerPlan { options: Vec::new(), default_index: 0 })
}

/// Parse a frontmatter-shaped mapping's selection hints.
#[allow(clippy::result_large_err)]
fn hints_from_map(map: &Map<String, Value>) -> Result<EffectiveSelectionHints, CompositionError> {
    let mut frontmatter = darkmatter::markdown::Frontmatter::new();
    for key in ["agent", "model"] {
        if let Some(value) = map.get(key) {
            // A value the frontmatter cannot hold is a hint of the wrong type.
            frontmatter
                .insert(key, value.clone())
                .map_err(|_| CompositionError::AgentHintWrongType(crate::commands::compose::json_type_name(value).into()))?;
        }
    }
    composition::parse_selection_hints_from_frontmatter(&frontmatter)
}

/// The review rows: an editable row for each unresolved site, and a read-only
/// row for every other, showing the provider it will run (or that its turn
/// decides), in [`TaskSelections::iter`] order. A row is labeled with its
/// root step's position ([`SequenceStepDraft::step_index`]); it maps back by
/// its position in that order, never by label or name.
pub(super) fn review_drafts(selections: &TaskSelections) -> Vec<SequenceStepDraft> {
    selections
        .iter()
        .map(|selection| {
            let (plan, read_only, provider, model, model_reason) = match &selection.target {
                TaskTarget::Unresolved { plan, .. } => {
                    let provider = plan.options.get(plan.default_index).map(|option| option.provider);
                    (plan.clone(), None, provider, None, ModelResolutionReason::ProviderDefault)
                }
                TaskTarget::Own(target) | TaskTarget::Fallback(target) => (
                    ProviderPickerPlan { options: Vec::new(), default_index: 0 },
                    Some(target.provider.to_string()),
                    Some(target.provider),
                    target.model.clone(),
                    target.model_reason.clone(),
                ),
                TaskTarget::AtLaunch(_) => (
                    ProviderPickerPlan { options: Vec::new(), default_index: 0 },
                    Some("decided at launch".to_string()),
                    None,
                    None,
                    ModelResolutionReason::ProviderDefault,
                ),
            };
            SequenceStepDraft {
                step_index: selection.site.step,
                step_name: selection.site.name.clone(),
                provider_plan: plan,
                proposed_model: model,
                model_reason,
                provider_locked: false,
                model_locked: false,
                resolved_provider: provider,
                read_only,
            }
        })
        .collect()
}

/// One line per site for a preflight or dry-run listing: its name and the
/// provider it is predicted to run, or how its provider is decided.
pub(super) fn plan_rows(selections: &TaskSelections) -> Vec<String> {
    selections
        .iter()
        .map(|selection| {
            let decided = match &selection.target {
                TaskTarget::Own(target) => format!("<b>{}</b>", target.provider),
                TaskTarget::Fallback(target) => format!("<b>{}</b> <dim>(from the sequence)</dim>", target.provider),
                TaskTarget::AtLaunch(_) => "<i>decided at launch</i>".to_string(),
                TaskTarget::Unresolved { .. } => "<i>no provider</i>".to_string(),
            };
            format!("{} → {decided}", selection.site.name)
        })
        .collect()
}
