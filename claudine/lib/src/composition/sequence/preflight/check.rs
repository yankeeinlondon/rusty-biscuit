//! The recovering walk behind a sequence dry run.
//!
//! [`check_sequence_tree`] reads a sequence the way a real run's preflight
//! does — the same root plan pipeline, the same input discovery, and the same
//! graph [`Loader`] — but where a real run stops at its first failure, the
//! check records the failure at the task or group leg it belongs to and goes
//! on with that task's siblings. A bad item keeps its authored position; a
//! malformed, command-backed, or effect-dependent root list leaves its tasks
//! unread; a bad file blocks only the subtree that references it.
//!
//! The walk holds a passive session for its thread
//! ([`passive_scope`](crate::composition::passive_scope)): an expression it
//! evaluates (a source, an operator, a `template:` default, a `with:` value)
//! that calls an effectful function fails as unavailable instead of running.
//!
//! Every task the walk reaches is a [`CheckedTask`], in authored order (a
//! container before its children), identified by the same [`TaskIdentity`]
//! the graph gives it. A real run keeps calling the fail-fast entry points,
//! whose behavior and error order this module does not change.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use darkmatter::findings::Diagnostic;
use darkmatter::findings::reference::{DocumentConstruct, ListingMemo};
use darkmatter::markdown::compose::ComposeContext;
use serde_json::{Map, Value};

use super::super::grammar::{self, SequenceSourceSpec};
use super::super::model::{ExecutableField, SequencePlan, SequenceSource};
use super::super::normalize::ItemFailure;
use super::super::{SequenceSourceOptions, document, formal, removed};
use super::{InputDiscoveryOptions, Loader, PreflightAction, PreflightTask, SequenceInputInventory, TaskIdentity};
use crate::composition::error::CompositionError;
use crate::composition::types::{CallerInputLayers, ResolvedCompositionSource};
use crate::invocation_context::{InvocationContext, SourceContext};

/// What [`check_sequence_tree`] reads a sequence under.
pub struct SequenceCheckRequest<'a> {
    /// The root document as authored.
    pub source: &'a ResolvedCompositionSource,
    /// The invocation's captured launch context, which owners prepare under.
    pub compose_context: ComposeContext,
    /// The invocation every reference resolves through.
    pub invocation: &'a InvocationContext,
    /// The root's own source context.
    pub source_context: &'a SourceContext,
    /// The caller's setters with their origins, when any were given.
    pub caller_layers: Option<&'a CallerInputLayers>,
    /// Where the command was run: a reference finding's hint names paths
    /// relative to it.
    pub launch_dir: Option<&'a Path>,
}

/// How the root's `sequence` reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootList {
    /// A list that was read (inline, referenced, or a `{{ … }}` value).
    Read,
    /// A `$( … )` list, known only once its command runs.
    Command,
    /// A list whose source expression needs an effect a dry run does not
    /// have (`has_alias`, a deferred value); its root failure carries the
    /// reason ([`CompositionError::deferral_reason`]).
    Deferred,
    /// A list that could not be read; its failure is a root failure.
    Unread,
}

/// A failure, with the finding a check reports in place of the typed error
/// when one describes it better (a reference with no match).
#[derive(Debug)]
pub struct CheckFailure {
    /// The failure as a real run would raise it.
    pub error: CompositionError,
    /// The reference finding for it, when the failure is a reference with
    /// no match.
    pub finding: Option<Diagnostic>,
}

/// An enclosing task of a [`TaskSite`]: the group or sequence a task sits in.
#[derive(Debug, Clone)]
pub struct SiteFrame {
    /// The enclosing task's identity.
    pub identity: TaskIdentity,
    /// The enclosing task's executable.
    pub executable: Option<ExecutableField>,
    /// Its display name.
    pub name: String,
    /// Its authored property path in [`Self::file`].
    pub property: String,
    /// The file that authored it.
    pub file: PathBuf,
}

/// Where a checked task sits.
#[derive(Debug, Clone)]
pub struct TaskSite {
    /// The task's identity in the graph.
    pub identity: TaskIdentity,
    /// Its display name: the authored `name`, else `step-<n>`.
    pub name: String,
    /// The file that authored the task.
    pub file: PathBuf,
    /// Its authored property path in [`Self::file`] (`sequence[1]`,
    /// `sequence[0].group.tasks[2]`, `tasks[0]` in a group file).
    pub property: String,
    /// The enclosing tasks, outermost first.
    pub frames: Vec<SiteFrame>,
    /// Where a selected data row was written, when that is not
    /// [`Self::file`]: the data file and the row's property in it (`[1]`,
    /// `rows[1]`). A data selection resolves from [`Self::file`], the file
    /// that selected it, and reads there; what is wrong with the row itself
    /// is located here. `None` for a task its own file wrote.
    pub row: Option<(PathBuf, String)>,
}

impl TaskSite {
    /// The file that wrote the task and its property there: the data row
    /// ([`Self::row`]) when it was selected from one.
    #[must_use]
    pub fn authored(&self) -> (&Path, &str) {
        match &self.row {
            Some((file, property)) => (file, property),
            None => (&self.file, &self.property),
        }
    }
}

/// A sequence document's own preparation failure, and the task it belongs
/// to.
#[derive(Debug)]
pub struct OwnerFailure {
    /// The sequence document.
    pub document: PathBuf,
    /// The `sequence:` task that references it; `None` for the root (and a
    /// root `sequence:` reference), whose tasks all read its values.
    pub task: Option<TaskIdentity>,
    /// The failure, as a real run raises it.
    pub failure: CheckFailure,
}

/// One task the check reached.
#[derive(Debug)]
pub struct CheckedTask {
    /// Where it sits.
    pub site: TaskSite,
    /// The authored item, as the list holds it.
    pub item: Value,
    /// The executable it declares; `None` composes its document's body.
    pub executable: Option<ExecutableField>,
    /// The prompt document a `prompt:` task (or the task a `task:` file
    /// declares) composes, once resolved.
    pub prompt: Option<PathBuf>,
    /// The task's `with:` inputs, as authored.
    pub with: Map<String, Value>,
    /// The `with:` values known before the run, evaluated against the
    /// step's early-binding values as the task's turn will see them; the
    /// document reads them as data. A key absent here is in
    /// [`Self::with_late`] or failed to evaluate.
    pub with_evaluated: Map<String, Value>,
    /// The `with:` keys whose values read a value known only at the task's
    /// turn (a prior task's output, `seq.*`, …), sorted.
    pub with_late: Vec<String>,
    /// A `prompt:` task's provider prediction, as the real run's selector
    /// reads it.
    pub provider: Option<super::providers::ProviderPrediction>,
    /// A `sequence:` task's sequence document and its frontmatter, whose
    /// `agent` its children fall back to; `None` for an inline list.
    pub sequence_document: Option<(PathBuf, Map<String, Value>)>,
    /// The `seq` binding a sequence step's document reads at its turn
    /// (position, neighbors, `run_id`; outputs empty); `None` for a group leg.
    pub seq: Option<Value>,
    /// Why the task could not be read; empty when it was.
    pub failures: Vec<CheckFailure>,
    /// The task, group, catalog, and sequence files this task's own
    /// reference read, in read order: what a verdict on such a file that no
    /// task inside it owns belongs to.
    pub loaded: Vec<LoadedDocument>,
}

/// A data file a task's reference read, and the part of it the task uses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedDocument {
    /// The file as resolved.
    pub path: PathBuf,
    /// The authored property of the part the task uses (`groups[1]` for a
    /// selected catalog entry); empty for the whole file.
    pub property: String,
}

/// Everything [`check_sequence_tree`] found.
#[derive(Debug)]
pub struct SequenceCheck {
    /// How the root's list reads.
    pub list: RootList,
    /// Failures that belong to the root rather than to one task.
    pub root_failures: Vec<CheckFailure>,
    /// Every task reached, in authored order.
    pub tasks: Vec<CheckedTask>,
    /// The inventory of owners' inputs, or why discovery failed.
    pub inventory: Result<SequenceInputInventory, CompositionError>,
    /// Every task, group, catalog, and sequence file the walk read, in
    /// first-read order: what a check judges against its kind's schema.
    pub files: Vec<PathBuf>,
    /// Each sequence document whose own preparation failed for a reason
    /// other than a missing input (a value its `$schema` refuses): a real run
    /// stops there, after the graph and provider checks and before any task.
    pub owner_failures: Vec<OwnerFailure>,
}

/// The walk's record while a [`Loader`] reads in check mode.
pub(super) struct CheckState {
    tasks: Vec<CheckedTask>,
    root_failures: Vec<CheckFailure>,
    frames: Vec<SiteFrame>,
    /// The index in [`Self::tasks`] of each frame's task.
    open: Vec<usize>,
    files: Vec<PathBuf>,
    memo: ListingMemo,
    pending_finding: Option<Diagnostic>,
    launch_dir: Option<PathBuf>,
    /// `root_ordinals[i]` is the authored position of the root plan's step
    /// `i`.
    pub(super) root_ordinals: Vec<usize>,
    /// The root items the plan set aside: position, item, and failure.
    pub(super) root_item_failures: Vec<(usize, Value, CompositionError)>,
}

/// Where a task sits relative to the walk's innermost frame.
#[derive(Debug, Clone, Copy)]
pub(super) enum Slot {
    /// Root step at this authored position.
    Root(usize),
    /// Child at this authored position of the innermost nested sequence.
    SequenceChild(usize),
    /// Leg at this position of the innermost group.
    Leg(usize),
}

/// What a check site records about the task it is about to read.
pub(super) struct SiteSeed<'s> {
    pub(super) slot: Slot,
    pub(super) name: String,
    pub(super) file: &'s Path,
    pub(super) property: String,
    pub(super) item: &'s Value,
    pub(super) executable: Option<ExecutableField>,
    pub(super) seq: Option<Value>,
    /// Where a selected data row was written ([`TaskSite::row`]).
    pub(super) row: Option<(PathBuf, String)>,
}

impl CheckState {
    fn new(launch_dir: Option<&Path>) -> Self {
        Self {
            tasks: Vec::new(),
            root_failures: Vec::new(),
            frames: Vec::new(),
            open: Vec::new(),
            files: Vec::new(),
            memo: ListingMemo::new(),
            pending_finding: None,
            launch_dir: launch_dir.map(Path::to_path_buf),
            root_ordinals: Vec::new(),
            root_item_failures: Vec::new(),
        }
    }

    fn identity(&self, slot: Slot) -> TaskIdentity {
        match (slot, self.frames.last()) {
            (Slot::Root(index), _) | (Slot::SequenceChild(index) | Slot::Leg(index), None) => TaskIdentity::root(index),
            (Slot::SequenceChild(index), Some(frame)) => frame.identity.sequence_child(index),
            (Slot::Leg(index), Some(frame)) => frame.identity.group_leg(index),
        }
    }

    fn failure(&mut self, error: CompositionError) -> CheckFailure {
        CheckFailure { error, finding: self.pending_finding.take() }
    }

    fn push(&mut self, seed: SiteSeed<'_>) -> usize {
        let site = TaskSite {
            identity: self.identity(seed.slot),
            name: seed.name,
            file: seed.file.to_path_buf(),
            property: seed.property,
            frames: self.frames.clone(),
            row: seed.row,
        };
        self.tasks.push(CheckedTask {
            site,
            item: seed.item.clone(),
            executable: seed.executable,
            prompt: None,
            with: Map::new(),
            with_evaluated: Map::new(),
            with_late: Vec::new(),
            provider: None,
            sequence_document: None,
            seq: seed.seq,
            failures: Vec::new(),
            loaded: Vec::new(),
        });
        self.tasks.len() - 1
    }
}

/// The `construct` and `noun` a reference finding names for `field`.
fn construct_of(field: ExecutableField) -> (&'static str, &'static str) {
    match field {
        ExecutableField::Prompt => ("prompt:", "prompt file"),
        ExecutableField::Task => ("task:", "task file"),
        ExecutableField::Group => ("group:", "group file"),
        ExecutableField::Sequence => ("sequence:", "sequence file"),
        ExecutableField::Shell | ExecutableField::SideEffect => ("reference", "file"),
    }
}

impl Loader<'_> {
    /// Whether this walk records failures instead of returning them.
    pub(super) fn checking(&self) -> bool {
        self.check.is_some()
    }

    /// In check mode, records the open task's evaluated and late `with:`
    /// values.
    pub(super) fn note_prepared_with(&mut self, evaluated: &Map<String, Value>, late: &[String]) {
        let Some(check) = self.check.as_mut() else { return };
        let Some(&slot) = check.open.last() else { return };
        check.tasks[slot].with_evaluated = evaluated.clone();
        check.tasks[slot].with_late = late.to_vec();
    }

    /// In check mode, record `error` as the root's and continue; otherwise
    /// return it.
    pub(super) fn recover_root(&mut self, result: Result<(), CompositionError>) -> Result<(), CompositionError> {
        match (result, self.check.as_mut()) {
            (Err(error), Some(check)) => {
                let failure = check.failure(error);
                check.root_failures.push(failure);
                Ok(())
            }
            (result, _) => result,
        }
    }

    /// Note a reference `raw` (authored in `origin` as `field`) that failed
    /// to resolve, so the failure the walk records next carries its finding.
    pub(super) fn note_unresolved_reference(&mut self, raw: &str, origin: &Path, field: ExecutableField) {
        let resolution = self.source_context_for(origin)
            .map_or_else(|| self.file_resolution_context.clone(), |context| context.file_resolution_context().clone());
        let Some(check) = self.check.as_mut() else { return };
        let (construct, noun) = construct_of(field);
        let launch_dir = check.launch_dir.clone();
        let construct = DocumentConstruct { construct, noun, launch_dir: launch_dir.as_deref() };
        check.pending_finding =
            super::super::source::reference_finding(raw, origin, &resolution, construct, &mut check.memo);
    }

    /// In check mode, record `error` at the task being read and continue;
    /// otherwise (or outside every task) return it.
    pub(super) fn recover_in_task(&mut self, result: Result<(), CompositionError>) -> Result<(), CompositionError> {
        match (result, self.check.as_mut()) {
            (Err(error), Some(check)) if !check.open.is_empty() => {
                let slot = *check.open.last().expect("checked above");
                let failure = check.failure(error);
                check.tasks[slot].failures.push(failure);
                Ok(())
            }
            (result, _) => result,
        }
    }

    /// In check mode, note that the walk read the data file `path`, for the
    /// open task when there is one.
    pub(super) fn note_loaded_file(&mut self, path: &Path) {
        let Some(check) = self.check.as_mut() else { return };
        if !check.files.iter().any(|known| known == path) {
            check.files.push(path.to_path_buf());
        }
        if let Some(&slot) = check.open.last() {
            check.tasks[slot].loaded.push(LoadedDocument { path: path.to_path_buf(), property: String::new() });
        }
    }

    /// In check mode, narrow the open task's last read of `path` to the
    /// part at `property` it uses.
    pub(super) fn note_loaded_part(&mut self, path: &Path, property: &str) {
        let Some(check) = self.check.as_mut() else { return };
        let Some(&slot) = check.open.last() else { return };
        if let Some(loaded) = check.tasks[slot].loaded.iter_mut().rev().find(|loaded| loaded.path == path) {
            loaded.property = property.to_string();
        }
    }

    /// In check mode, the authored position of root plan step `index`.
    pub(super) fn root_ordinal(&self, index: usize) -> usize {
        self.check.as_ref().and_then(|check| check.root_ordinals.get(index).copied()).unwrap_or(index)
    }

    /// In check mode, record the root items the plan set aside, located in
    /// `origin` (and, for a data selection, at their rows of `source`).
    pub(super) fn record_root_item_failures(&mut self, origin: &Path, source: &super::super::model::SequenceSource) {
        let Some(check) = self.check.as_mut() else { return };
        for (index, item, error) in std::mem::take(&mut check.root_item_failures) {
            let seed = SiteSeed {
                slot: Slot::Root(index), name: item_name(&item, index), file: origin,
                property: format!("sequence[{index}]"), item: &item, executable: item_executable(&item),
                seq: None, row: source.data_row(index),
            };
            self.record_item_failure(seed, error);
        }
    }

    /// Record an item that failed before it became a task.
    pub(super) fn record_item_failure(&mut self, seed: SiteSeed<'_>, error: CompositionError) {
        let Some(check) = self.check.as_mut() else { return };
        check.pending_finding = None;
        let slot = check.push(seed);
        let failure = check.failure(error);
        check.tasks[slot].failures.push(failure);
    }

    /// Read one task through `run`. Outside check mode this is `run`. In
    /// check mode the task is recorded before its children, a failure is
    /// recorded at it (`Ok(None)`), and the walk goes on.
    pub(super) fn check_task(
        &mut self,
        seed: SiteSeed<'_>,
        run: impl FnOnce(&mut Self) -> Result<Option<PreflightTask>, CompositionError>,
    ) -> Result<Option<PreflightTask>, CompositionError> {
        let Some(check) = self.check.as_mut() else { return run(self) };
        check.pending_finding = None;
        let slot = check.push(seed);
        let frame = {
            let site = &check.tasks[slot].site;
            SiteFrame {
                identity: site.identity.clone(),
                executable: check.tasks[slot].executable,
                name: site.name.clone(),
                property: site.property.clone(),
                file: site.file.clone(),
            }
        };
        check.frames.push(frame);
        check.open.push(slot);
        let depth = self.ancestry.len();
        let result = run(self);
        // A failure that returned between entering a file and leaving it
        // would otherwise leave the file on the chain, and the next task
        // referencing it would read as a cycle.
        self.ancestry.truncate(depth);
        let check = self.check.as_mut().expect("check mode does not change during a walk");
        check.frames.pop();
        check.open.pop();
        match result {
            Ok(task) => {
                if let Some(task) = &task {
                    let record = &mut check.tasks[slot];
                    match &task.action {
                        PreflightAction::Prompt { path, .. } => record.prompt = Some(path.clone()),
                        PreflightAction::Sequence(sequence) => {
                            record.sequence_document = sequence.frontmatter.clone()
                                .map(|frontmatter| (sequence.origin_path.clone(), frontmatter));
                        }
                        _ => {}
                    }
                    record.provider = task.provider.clone();
                    record.with = task.with.clone();
                }
                Ok(task)
            }
            Err(error) => {
                let failure = check.failure(error);
                check.tasks[slot].failures.push(failure);
                Ok(None)
            }
        }
    }
}

/// The display name of an authored item at `index`: its `name`, else
/// `step-<n>`.
pub(super) fn item_name(item: &Value, index: usize) -> String {
    match item {
        Value::String(name) if !name.trim().is_empty() => name.clone(),
        Value::Object(map) => map.get("name").and_then(Value::as_str).filter(|name| !name.trim().is_empty())
            .map_or_else(|| format!("step-{}", index + 1), str::to_string),
        _ => format!("step-{}", index + 1),
    }
}

/// The executable an authored item declares, when it names exactly one.
pub(super) fn item_executable(item: &Value) -> Option<ExecutableField> {
    let map = item.as_object()?;
    let mut fields = map.keys().filter_map(|key| ExecutableField::from_key(key));
    let field = fields.next()?;
    fields.next().is_none().then_some(field)
}

/// Read the sequence `request.source` as its real run's preflight does,
/// recording every failure at the task it belongs to instead of stopping.
///
/// Nothing runs: no lifecycle event, command, provider, approval, or
/// effectful expression function. A `$( … )` list is not expanded
/// ([`RootList::Command`]), and a `{{ … }}` list that calls such a function
/// is not decided ([`RootList::Deferred`]).
#[must_use]
pub fn check_sequence_tree(request: SequenceCheckRequest<'_>) -> SequenceCheck {
    let SequenceCheckRequest { source, compose_context, invocation, source_context, caller_layers, launch_dir } = request;
    // Every evaluator the walk reaches builds its expression context under
    // this session, so an effectful function is refused, not called.
    let session = darkmatter::markdown::compose::passive::PassiveSession::new();
    let _passive = crate::composition::passive_scope::enter(session.clone());
    let mut state = CheckState::new(launch_dir);
    let fail = |state: &mut CheckState, error: CompositionError| {
        let failure = state.failure(error);
        state.root_failures.push(failure);
    };

    if let Err(error) = super::reject_non_sequence_kind(source) {
        fail(&mut state, error);
        return finish(state, RootList::Unread, Err(no_discovery(&source.resolved_path)));
    }
    let resolution = source_context.file_resolution_context();
    let mut options = SequenceSourceOptions::new(resolution).with_compose_context(&compose_context).with_invocation(invocation);
    if let Some(layers) = caller_layers {
        options = options.with_caller_layers(layers);
    }
    let setters = caller_layers.and_then(|layers| layers.set_overrides.as_ref())
        .and_then(Value::as_object).cloned().unwrap_or_default();
    check_root(source, &setters, &mut state);
    let (list, recovered) = root_plan(source, &options, &mut state);

    // Discovery prepares every owner passively, and its parses are reused by
    // the walk. A failed discovery leaves only the inputs unchecked: the walk
    // reports the failure at its task.
    let mut discovery = InputDiscoveryOptions::new(resolution, &compose_context)
        .with_invocation(invocation, source_context)
        .leaving_verdicts()
        .recovering();
    if let Some(layers) = caller_layers {
        discovery = discovery.with_caller_layers(layers);
    }
    // A file with no `sequence` is still discovered first by a real run, so
    // an input it misses is reported before the refusal.
    let undefined = !source.markdown.frontmatter().as_map().contains_key("sequence");
    let mut inventory = match list {
        RootList::Read => super::discover_sequence_inputs(source, discovery),
        RootList::Unread if undefined => super::discover_sequence_inputs(source, discovery),
        RootList::Command | RootList::Deferred | RootList::Unread => Err(no_discovery(&source.resolved_path)),
    };
    // A missing input has no value until the real run asks for it, and then
    // every owner and task reads the answer. Discovery runs again with each
    // one unavailable, so a value derived from it (an owner's
    // `plan: "{{ spec }}-plan"`, a `with:` reading that) is left as written
    // rather than computed from nothing; the missing entries are the same.
    let missing: Vec<String> = match &inventory {
        Ok(found) => found
            .owners
            .iter()
            .filter_map(|owner| owner.missing.as_ref())
            .flat_map(|entry| entry.missing.iter().map(|property| property.name.clone()))
            .collect(),
        Err(_) => Vec::new(),
    };
    if !missing.is_empty() {
        for name in missing {
            session.defer_absent(&name, darkmatter::markdown::compose::passive::DeferralReason::MissingInput { name: name.clone() });
        }
        inventory = super::discover_sequence_inputs(source, discovery);
    }

    let Some((plan, ordinals, item_failures)) = recovered else {
        return finish(state, list, inventory);
    };
    let mut loader = Loader::new_with_invocation(source, compose_context.clone(), invocation, source_context.clone());
    let mut state = state;
    if let Ok(inventory) = &inventory {
        loader.graph.nested_sources = inventory.sources.clone();
        loader.owner_values = inventory.owners.iter()
            .map(|owner| (super::canonical(&owner.path), Arc::clone(&owner.values)))
            .collect();
    }
    loader.caller_setters = setters;
    loader.caller_layers = caller_layers.cloned();
    state.root_ordinals = ordinals;
    state.root_item_failures = item_failures;
    loader.check = Some(Box::new(state));
    let walked = loader.walk_plan(&plan);
    let mut state = *loader.check.take().expect("the walk stays in check mode");
    if let Err(error) = walked {
        fail(&mut state, error);
    }
    // The real graph's `finish`: a prediction reading a key some lifecycle
    // may write before the task launches is decided at launch, not now.
    let written = loader.runtime_written_keys();
    for prediction in state.tasks.iter_mut().filter_map(|task| task.provider.as_mut()) {
        prediction.defer_runtime_written(&written);
    }
    // A real run judges every referenced prompt's lifecycle literals once
    // the graph is built (`validate_prompt_lifecycle_literals`); a check
    // records the refusal at each task that composes the prompt.
    for task in &mut state.tasks {
        let Some(document) = task.prompt.as_deref().and_then(|path| loader.graph.prompt_document(path)) else { continue };
        if let Err(error) = super::validate_lifecycle_literals(&document.markdown, &document.path) {
            task.failures.push(CheckFailure { error, finding: None });
        }
    }
    finish(state, list, inventory)
}

/// The root refusals a real run makes before it reads any task list
/// (`commands/sequence.rs`), whatever the list turns out to be: a setter
/// naming a declaration, the root's grammar, its lifecycle, and a read of
/// removed sequence state anywhere in the root. The walk does not repeat them.
fn check_root(source: &ResolvedCompositionSource, setters: &Map<String, Value>, state: &mut CheckState) {
    let path = &source.resolved_path;
    let mut failures = Vec::new();
    for key in setters.keys() {
        failures.extend(super::super::declarations::reject_declaration_write(key, path, "a setter").err());
    }
    let mut authored: Map<String, Value> = source.markdown.frontmatter().as_map().iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    failures.extend(grammar::validate_sequence_root(&authored, path).err());
    authored.extend(setters.clone());
    let authored = Value::Object(authored);
    let lifecycle = crate::composition::lifecycle::parse_lifecycle_config(&authored, path)
        .map_err(|error| failures.push(error))
        .ok();
    let document = crate::composition::lifecycle::bindings::AuthoredDocument {
        frontmatter: &authored,
        body: source.markdown.content(),
        body_line_offset: source.markdown.frontmatter_line_count(),
    };
    let supplied = Value::Object(setters.clone());
    failures.extend(
        crate::composition::lifecycle::bindings::reject_removed_sequence_reads(path, document, Some(&supplied), lifecycle.as_ref())
            .err(),
    );
    for error in failures {
        let failure = state.failure(error);
        state.root_failures.push(failure);
    }
}

/// The error a check records in place of an inventory it did not discover;
/// the root's `sequence` list is what could not be read.
fn no_discovery(root: &Path) -> CompositionError {
    CompositionError::sequence_invalid("the task list could not be read", crate::composition::SequenceSite::at(root, "sequence"))
}

fn finish(
    mut state: CheckState,
    list: RootList,
    inventory: Result<SequenceInputInventory, CompositionError>,
) -> SequenceCheck {
    // Root steps were recorded as the walk reached them and failed items as
    // the plan was read; authored order is the identities' order.
    state.tasks.sort_by_key(|task| identity_order(&task.site.identity));
    let mut check = SequenceCheck {
        list, root_failures: state.root_failures, tasks: state.tasks, inventory, files: state.files,
        owner_failures: Vec::new(),
    };
    // A file with no list (or one not read) stops the real run before any
    // owner's own verdict.
    if list == RootList::Read {
        check.owner_failures = owner_failures(&mut check);
    }
    check
}

/// The owner failures discovery recorded, each at the task whose
/// `sequence:` reference reached the owner: the n-th owner reached from one
/// file and property belongs to the n-th task there.
fn owner_failures(check: &mut SequenceCheck) -> Vec<OwnerFailure> {
    let Ok(inventory) = &mut check.inventory else { return Vec::new() };
    let mut seen: std::collections::HashMap<(PathBuf, String), usize> = std::collections::HashMap::new();
    let occurrence: Vec<usize> = inventory.owners.iter().map(|owner| {
        let count = seen.entry((owner.referenced_from.clone().unwrap_or_default(), owner.property.clone())).or_default();
        *count += 1;
        *count - 1
    }).collect();
    // A verdict on the owner's own list alone is the walk's to report, at
    // each task, when it found the list's problems: the real run's graph
    // refuses those before any owner is judged.
    let walk_failed = !check.root_failures.is_empty() || check.tasks.iter().any(|task| !task.failures.is_empty());
    let judges_only_the_list = |error: &CompositionError| matches!(
        error,
        CompositionError::SchemaValidation { problems, .. }
            if !problems.is_empty() && problems.iter().all(|problem| problem == "/sequence" || problem.starts_with("/sequence/"))
    );
    std::mem::take(&mut inventory.failed_owners).into_iter()
        .filter(|failed| !(walk_failed && judges_only_the_list(&failed.error)))
        .map(|failed| {
        let owner = &inventory.owners[failed.owner];
        let task = match (owner.role, owner.referenced_from.as_deref()) {
            (super::InputOwnerRole::Nested, Some(from)) => check.tasks.iter()
                .filter(|task| task.site.file == from && format!("{}.sequence", task.site.property) == owner.property)
                .nth(occurrence[failed.owner])
                .map(|task| task.site.identity.clone()),
            _ => None,
        };
        OwnerFailure { document: owner.path.clone(), task, failure: CheckFailure { error: failed.error, finding: None } }
    }).collect()
}

/// The root ordinal of `identity` (`tasks[3]…` → 3): every task under one root
/// step shares it, and the walk records those in authored order already.
fn identity_order(identity: &TaskIdentity) -> usize {
    identity.as_str().strip_prefix("tasks[")
        .and_then(|rest| rest.split(']').next())
        .and_then(|index| index.parse().ok())
        .unwrap_or(usize::MAX)
}

/// The root's plan, read as [`super::super::resolve_sequence_plan_with`]
/// reads it but setting unreadable items aside: the plan of readable items,
/// their authored positions, and the items that failed.
#[allow(clippy::type_complexity)]
fn root_plan(
    source: &ResolvedCompositionSource,
    options: &SequenceSourceOptions<'_>,
    state: &mut CheckState,
) -> (RootList, Option<(SequencePlan, Vec<usize>, Vec<(usize, Value, CompositionError)>)>) {
    let fail = |state: &mut CheckState, error: CompositionError| {
        let failure = state.failure(error);
        state.root_failures.push(failure);
    };
    // As `resolve_sequence_plan_with`: a failure the source layers raised
    // without knowing the file is the root's `sequence` value's.
    let located = |error: CompositionError| error.locate_sequence(&source.resolved_path, "sequence", None);
    let frontmatter: Map<String, Value> = source.markdown.frontmatter().as_map().iter()
        .map(|(key, value)| (key.clone(), crate::composition::closure::stored_text(value)))
        .collect();
    // A real run refuses a file with no `sequence` once it reads the plan
    // (`phase1c.rs`), so nothing beneath the root is read.
    let Some(sequence) = frontmatter.get("sequence").cloned() else {
        fail(state, CompositionError::sequence_undefined(&source.resolved_path));
        return (RootList::Unread, None);
    };
    if let Some(value) = frontmatter.get("fail_fast") {
        fail(state, removed::removed_key(removed::RemovedOwner::Root, "fail_fast", value, &source.resolved_path));
    }
    let spec = match grammar::classify_source(&sequence) {
        Ok(spec) => spec,
        Err(error) => {
            fail(state, error);
            return (RootList::Unread, None);
        }
    };
    match spec {
        SequenceSourceSpec::Inline(items) => {
            let authored = Arc::new(frontmatter.clone());
            let recovered = formal::formal_keys(&authored, &source.resolved_path).and_then(|keys| {
                let values = Arc::new(document::owner_values(&authored, &keys, options, None, || {
                    Ok(std::borrow::Cow::Borrowed(source))
                })?);
                let state_schema = keys.state_schema
                    .map(|declaration| super::super::declarations::PreparedSequenceSchema::prepare(
                        declaration, &source.resolved_path, super::super::declarations::STATE_KEY,
                        options.file_resolution_context(),
                    ))
                    .transpose()?;
                let mut recovered = formal::normalize_formal_plan_recovering(
                    items.clone(),
                    formal::FormalOwner { keys, declaring_path: &source.resolved_path, list_property: "sequence", values: &*values },
                    state_schema.as_ref(), SequenceSource::Inline, &source.resolved_path, true,
                    options.file_resolution_context(),
                )?;
                recovered.plan.document_scope = Some(Arc::new(document::SequenceDocumentScope {
                    declaring_path: source.resolved_path.clone(),
                    property: "sequence".into(),
                    authored: Arc::clone(&authored),
                    values,
                }));
                Ok(recovered)
            });
            let mut recovered = match recovered {
                Ok(recovered) => recovered,
                Err(error) => {
                    let list = if error.deferral_reason().is_some() { RootList::Deferred } else { RootList::Unread };
                    fail(state, error);
                    return (list, None);
                }
            };
            if let Err(error) = super::super::validate_formal_sequence_file(source, options.file_resolution_context()) {
                fail(state, error);
            }
            recovered.plan.authored = super::super::super::authored_order::AuthoredOrder::new(
                Arc::new(source.markdown.frontmatter().mapping_orders().clone()), "/sequence",
            );
            let failures = recovered.failures.into_iter()
                .map(|ItemFailure { index, error }| (index, items.get(index).cloned().unwrap_or(Value::Null), error))
                .collect();
            (RootList::Read, Some((recovered.plan, recovered.ordinals, failures)))
        }
        SequenceSourceSpec::Shell(_) => (RootList::Command, None),
        // A real run reads these lists through `resolve_sequence_plan_with`,
        // which fails the whole list at its first bad item; the check reads
        // the same selected items one by one.
        SequenceSourceSpec::Expression(expression) => {
            let items = super::super::expression_source_items(
                &expression, &frontmatter, &source.resolved_path, options.file_resolution_context(),
            );
            let items = match items {
                Ok(items) => items,
                Err(error) => return unread_list(state, located(error)),
            };
            let recovered = super::super::normalize::normalize_plan_recovering_at(
                &items, SequenceSource::Expression, &source.resolved_path, true, "sequence",
            );
            read_list(recovered, &items)
        }
        SequenceSourceSpec::Reference(reference) => {
            let path = match super::super::source::resolve_sequence_reference(
                &reference.reference, ExecutableField::Sequence, &source.resolved_path, options.file_resolution_context(),
            ) {
                Ok(path) => path,
                Err(error) => {
                    let mut memo = std::mem::take(&mut state.memo);
                    let launch_dir = state.launch_dir.clone();
                    state.pending_finding = super::super::source::reference_finding(
                        &reference.reference, &source.resolved_path, options.file_resolution_context(),
                        DocumentConstruct { construct: "sequence:", noun: "sequence file", launch_dir: launch_dir.as_deref() },
                        &mut memo,
                    );
                    state.memo = memo;
                    return unread_list(state, located(error));
                }
            };
            // A file that does not parse has no row boundaries to recover;
            // offsets and operators apply before any row is read.
            let planned = super::super::data::load_document(&path).and_then(|loaded| {
                super::super::source::plan_referenced_sequence_recovering(
                    &reference, &path, &loaded.value, Arc::new(loaded.orders), &source.resolved_path,
                    &frontmatter, true, options,
                )
            });
            match planned {
                Ok((recovered, items)) => read_list(recovered, &items),
                Err(error) => unread_list(state, located(error)),
            }
        }
    }
}

/// A recovered root list read from `items`, the selected items as authored,
/// which the failures index.
#[allow(clippy::type_complexity)]
fn read_list(
    recovered: super::super::normalize::RecoveredPlan,
    items: &[Value],
) -> (RootList, Option<(SequencePlan, Vec<usize>, Vec<(usize, Value, CompositionError)>)>) {
    let failures = recovered.failures.into_iter()
        .map(|ItemFailure { index, error }| (index, items.get(index).cloned().unwrap_or(Value::Null), error))
        .collect();
    (RootList::Read, Some((recovered.plan, recovered.ordinals, failures)))
}

/// A root list that could not be read: `error` is its root failure.
#[allow(clippy::type_complexity)]
fn unread_list(
    state: &mut CheckState,
    error: CompositionError,
) -> (RootList, Option<(SequencePlan, Vec<usize>, Vec<(usize, Value, CompositionError)>)>) {
    let list = if error.deferral_reason().is_some() { RootList::Deferred } else { RootList::Unread };
    let failure = state.failure(error);
    state.root_failures.push(failure);
    (list, None)
}
