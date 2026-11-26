//! The recovering walk behind a sequence dry run.
//!
//! [`check_sequence_tree`] reads a sequence the way a real run's preflight
//! does — the same root plan pipeline, the same input discovery, and the same
//! graph [`Loader`] — but where a real run stops at its first failure, the
//! check records the failure at the task or group leg it belongs to and goes
//! on with that task's siblings. A bad item keeps its authored position; a
//! malformed or command-backed root list leaves its tasks unread; a bad file
//! blocks only the subtree that references it.
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
    /// No `sequence` key: the default plan composes the document's body.
    Body,
    /// A list that was read (inline, referenced, or a `{{ … }}` value).
    Read,
    /// A `$( … )` list, known only once its command runs.
    Command,
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
    /// Why the task could not be read; empty when it was.
    pub failures: Vec<CheckFailure>,
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
        };
        self.tasks.push(CheckedTask {
            site,
            item: seed.item.clone(),
            executable: seed.executable,
            prompt: None,
            with: Map::new(),
            failures: Vec::new(),
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

    /// In check mode, note that the walk read the data file `path`.
    pub(super) fn note_loaded_file(&mut self, path: &Path) {
        if let Some(check) = self.check.as_mut() && !check.files.iter().any(|known| known == path) {
            check.files.push(path.to_path_buf());
        }
    }

    /// In check mode, the authored position of root plan step `index`.
    pub(super) fn root_ordinal(&self, index: usize) -> usize {
        self.check.as_ref().and_then(|check| check.root_ordinals.get(index).copied()).unwrap_or(index)
    }

    /// In check mode, record the root items the plan set aside, located in
    /// `origin`.
    pub(super) fn record_root_item_failures(&mut self, origin: &Path) {
        let Some(check) = self.check.as_mut() else { return };
        for (index, item, error) in std::mem::take(&mut check.root_item_failures) {
            let seed = SiteSeed {
                slot: Slot::Root(index), name: item_name(&item, index), file: origin,
                property: format!("sequence[{index}]"), item: &item, executable: item_executable(&item),
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
        let result = run(self);
        let check = self.check.as_mut().expect("check mode does not change during a walk");
        check.frames.pop();
        check.open.pop();
        match result {
            Ok(task) => {
                if let Some(task) = &task {
                    let record = &mut check.tasks[slot];
                    if let PreflightAction::Prompt { path, .. } = &task.action {
                        record.prompt = Some(path.clone());
                    }
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
/// Nothing runs: no lifecycle event, command, provider, or approval. A
/// `$( … )` list is not expanded ([`RootList::Command`]).
#[must_use]
pub fn check_sequence_tree(request: SequenceCheckRequest<'_>) -> SequenceCheck {
    let SequenceCheckRequest { source, compose_context, invocation, source_context, caller_layers, launch_dir } = request;
    let mut state = CheckState::new(launch_dir);
    let fail = |state: &mut CheckState, error: CompositionError| {
        let failure = state.failure(error);
        state.root_failures.push(failure);
    };

    if let Err(error) = super::reject_non_sequence_kind(source) {
        fail(&mut state, error);
        return finish(state, RootList::Unread, Err(no_discovery()));
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
    let inventory = match list {
        RootList::Body | RootList::Read => super::discover_sequence_inputs(source, discovery),
        RootList::Command | RootList::Unread => Err(no_discovery()),
    };

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

/// The error a check records in place of an inventory it did not discover.
fn no_discovery() -> CompositionError {
    CompositionError::SequenceInvalid("the task list could not be read".into())
}

fn finish(
    mut state: CheckState,
    list: RootList,
    inventory: Result<SequenceInputInventory, CompositionError>,
) -> SequenceCheck {
    // Root steps were recorded as the walk reached them and failed items as
    // the plan was read; authored order is the identities' order.
    state.tasks.sort_by_key(|task| identity_order(&task.site.identity));
    SequenceCheck { list, root_failures: state.root_failures, tasks: state.tasks, inventory, files: state.files }
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
    let frontmatter: Map<String, Value> = source.markdown.frontmatter().as_map().iter()
        .map(|(key, value)| (key.clone(), crate::composition::closure::stored_text(value)))
        .collect();
    let Some(sequence) = frontmatter.get("sequence").cloned() else {
        return (RootList::Body, None);
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
                    fail(state, error);
                    return (RootList::Unread, None);
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
        SequenceSourceSpec::Expression(_) | SequenceSourceSpec::Reference(_) => {
            match super::super::resolve_sequence_plan_with(source, *options) {
                Ok(Some(plan)) => {
                    let ordinals = (0..plan.steps.len()).collect();
                    (RootList::Read, Some((plan, ordinals, Vec::new())))
                }
                Ok(None) => (RootList::Body, None),
                Err(error) => {
                    if let SequenceSourceSpec::Reference(reference) = &grammar::classify_source(&sequence).expect("classified above") {
                        let mut memo = std::mem::take(&mut state.memo);
                        let launch_dir = state.launch_dir.clone();
                        state.pending_finding = super::super::source::reference_finding(
                            &reference.reference, &source.resolved_path, options.file_resolution_context(),
                            DocumentConstruct { construct: "sequence:", noun: "sequence file", launch_dir: launch_dir.as_deref() },
                            &mut memo,
                        );
                        state.memo = memo;
                    }
                    fail(state, error);
                    (RootList::Unread, None)
                }
            }
        }
    }
}
