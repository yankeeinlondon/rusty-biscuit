//! Discovery of a sequence's document inputs, before its graph is frozen.
//!
//! A sequence's caller inputs are declared by every whole document that owns
//! a step list: the invoked root, a whole document the root's `sequence:`
//! references, and a whole document a nested `sequence:` task references
//! (inline, inside a group leg, or inside a `kind: task` file). Each owner's
//! root `$schema` is its document schema, so each one's required properties
//! join one collection pass, even an owner with no steps or no prompt tasks.
//! [`discover_sequence_inputs`] walks the authored structure once to find
//! them, together with every prompt document a task references, and prepares
//! each owner's values passively: no lifecycle action, shell, provider, or
//! approval runs.
//!
//! An inline container's `$schema` is a check on the values it sees at its
//! turn, not a declaration of caller inputs, and `$state` types step data;
//! neither joins the inventory. An inline container is recorded only so a
//! caller can tell it apart from an owning document.
//!
//! The walk shares the graph [`Loader`]'s reference resolution, cycle
//! detection, and parsed-document cache rather than loading files itself.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use darkmatter::markdown::compose::ComposeContext;
use serde_json::{Map, Value};

use super::super::grammar::{self, SequenceReference, SequenceSourceSpec};
use super::super::model::ExecutableField;
use super::super::{declarations, formal, reserved, source as source_resolution};
use super::{
    Loader, expect_kind, expect_object, expect_reference, option_map, parse_task_object, property_child,
    select_catalog_group, split_catalog_reference,
};
use crate::composition::error::{CompositionError, SequenceMissingPropertiesEntry, SequenceMissingSubject};
use crate::composition::prepare::PrepareOptions;
use crate::composition::types::{CallerInputLayers, CompositionMode, ResolvedCompositionSource};
use crate::invocation_context::{InvocationContext, SourceContext};

/// How an owner was reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputOwnerRole {
    /// The invoked sequence document.
    Root,
    /// The whole document the root's `sequence:` references.
    RootReference,
    /// A whole document a nested `sequence:` task references.
    Nested,
}

/// A whole sequence document that owns a step list, and its inputs.
#[derive(Debug, Clone)]
pub struct InputOwner {
    /// How the owner was reached.
    pub role: InputOwnerRole,
    /// The owning document.
    pub path: PathBuf,
    /// The file holding the reference that reached the owner; `None` for the
    /// root.
    pub referenced_from: Option<PathBuf>,
    /// The reference's container property path in [`Self::referenced_from`]
    /// (`sequence`, `sequence[1].sequence`); empty for the root.
    pub property: String,
    /// The document's missing required inputs, as a document entry of the
    /// combined report; `None` when it has none.
    pub missing: Option<SequenceMissingPropertiesEntry>,
    /// The document's prepared values (its own properties under the caller's
    /// setters, coerced and defaulted, verdict withheld), which its
    /// `template:` reads. The caller's setters and their recorded origins are
    /// not changed by preparing them.
    pub values: Arc<Map<String, Value>>,
}

/// A prompt document a task references, with the context it was reached in.
#[derive(Debug, Clone, PartialEq)]
pub struct InputPrompt {
    /// The prompt document.
    pub path: PathBuf,
    /// The file that authored the `prompt:` task.
    pub source_path: PathBuf,
    /// The task's property path in that file.
    pub property: String,
    /// Index in [`SequenceInputInventory::owners`] of the nearest owner.
    pub owner: usize,
    /// The task's `with:` values, as authored.
    pub with: Map<String, Value>,
}

/// An inline nested `sequence:` task: a container that owns no document
/// inputs, whose `$schema` (when present) is checked at its turn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineContainer {
    /// The file that authored the container.
    pub source_path: PathBuf,
    /// The container task's property path in that file.
    pub property: String,
    /// Index in [`SequenceInputInventory::owners`] of the enclosing owner.
    pub owner: usize,
    /// Whether the container declares a check-only `$schema`.
    pub checks: bool,
}

/// An executable reference that reads a template while its owner is missing
/// inputs: it is resolved once those inputs are supplied, and reported only
/// through that owner's missing entry until then.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingReference {
    /// Index in [`SequenceInputInventory::owners`] of the owner missing inputs.
    pub owner: usize,
    /// The file that authored the reference.
    pub source_path: PathBuf,
    /// The executable's property path (`sequence[0].prompt`).
    pub property: String,
    /// The reference as authored, or the `template:` default that fills it.
    pub reference: String,
}

/// An owner whose own preparation failed for a reason other than a missing
/// input, under [`InputDiscoveryOptions::leaving_verdicts`].
#[derive(Debug)]
pub struct FailedOwner {
    /// Index in [`SequenceInputInventory::owners`].
    pub owner: usize,
    /// The failure, as a discovery without `leaving_verdicts` raises it.
    pub error: CompositionError,
}

/// Everything input collection needs to know before the graph is frozen, in
/// first-encounter order.
#[derive(Debug, Default)]
pub struct SequenceInputInventory {
    /// Every owning document; the root is first.
    pub owners: Vec<InputOwner>,
    /// Every distinct prompt reference (document plus `with:`).
    pub prompts: Vec<InputPrompt>,
    /// Every inline container.
    pub inline_containers: Vec<InlineContainer>,
    /// References left pending by an owner's missing inputs.
    pub pending: Vec<PendingReference>,
    /// The owners whose own preparation failed for a reason other than a
    /// missing input, under [`InputDiscoveryOptions::leaving_verdicts`];
    /// their values were then read as authored.
    pub failed_owners: Vec<FailedOwner>,
    /// The parses discovery made, which the graph walk that follows reuses
    /// ([`super::build_preflight_graph_after_discovery`]).
    pub(crate) sources: super::NestedSources,
}

impl SequenceInputInventory {
    /// Every owner's missing-input entry, in discovery order.
    #[must_use]
    pub fn missing_entries(&self) -> Vec<SequenceMissingPropertiesEntry> {
        self.owners.iter().filter_map(|owner| owner.missing.clone()).collect()
    }

    /// The combined report of the owners' missing inputs, when any owner is
    /// missing one.
    #[must_use]
    pub fn missing_error(&self) -> Option<CompositionError> {
        let failures = self.missing_entries();
        (!failures.is_empty()).then(|| CompositionError::SequenceMissingProperties {
            failure_count: failures.len(),
            failures,
            // The combined report belongs to the root, the first owner.
            site: Box::new(self.owners.first().map_or_else(crate::composition::SequenceSite::unlocated, |root| {
                crate::composition::SequenceSite::file(&root.path)
            })),
        })
    }
}

/// The invocation inputs discovery prepares owners under.
#[derive(Clone, Copy)]
pub struct InputDiscoveryOptions<'a> {
    file_resolution_context: &'a biscuit_file::FileResolutionContext,
    compose_context: &'a ComposeContext,
    caller_layers: Option<&'a CallerInputLayers>,
    invocation: Option<(&'a InvocationContext, &'a SourceContext)>,
    leave_verdicts: bool,
    recover: bool,
    prompt_memo: Option<&'a super::PromptMemo>,
}

impl<'a> InputDiscoveryOptions<'a> {
    /// Discover through `file_resolution_context`, preparing owners against
    /// the invocation's captured `compose_context`, with no caller input.
    #[must_use]
    pub fn new(
        file_resolution_context: &'a biscuit_file::FileResolutionContext,
        compose_context: &'a ComposeContext,
    ) -> Self {
        Self {
            file_resolution_context, compose_context, caller_layers: None, invocation: None, leave_verdicts: false,
            recover: false, prompt_memo: None,
        }
    }

    /// Resolve every reference from the repository context of the document
    /// that authored it, as the graph walk does: `source_context` is the
    /// invoked document's, derived from `invocation`.
    #[must_use]
    pub fn with_invocation(mut self, invocation: &'a InvocationContext, source_context: &'a SourceContext) -> Self {
        self.invocation = Some((invocation, source_context));
        self
    }

    /// Record a failure of an owner's own preparation, other than a missing
    /// input, instead of failing discovery
    /// ([`SequenceInputInventory::failed_owners`]), reading that owner's
    /// values as authored. A caller that reports other checks first (the
    /// graph's structure, provider selection) discovers again without this
    /// to report the failure in its turn.
    #[must_use]
    pub fn leaving_verdicts(mut self) -> Self {
        self.leave_verdicts = true;
        self
    }

    /// Skip a task or group leg whose reading fails, and go on with its
    /// siblings, instead of failing discovery: for a check, whose graph walk
    /// reports that failure at the task
    /// ([`super::check_sequence_tree`]). A failure outside every task (the
    /// root's list or declarations) still fails discovery.
    #[must_use]
    pub fn recovering(mut self) -> Self {
        self.recover = true;
        self
    }

    /// Read each prompt document through `memo`, which a check shares with
    /// its other passes, so a prompt is read once per check.
    #[must_use]
    pub(crate) fn with_prompt_memo(mut self, memo: &'a super::PromptMemo) -> Self {
        self.prompt_memo = Some(memo);
        self
    }

    /// Prepare every owner under the invocation's caller inputs.
    #[must_use]
    pub fn with_caller_layers(mut self, layers: &'a CallerInputLayers) -> Self {
        self.caller_layers = Some(layers);
        self
    }
}

/// Inventory the document inputs of the sequence `source`.
///
/// The walk follows the authored structure: the root's list (or the whole
/// document its `sequence:` references), nested sequence tasks, group legs
/// (inline, file, or catalog entry), and `kind: task` files. A `template:`
/// default is merged into an item before its executable is read. A
/// reference that reads a template while its owner is missing an input is
/// left [pending](PendingReference) rather than resolved; every other
/// reference is resolved, and discovery continues past an owner's missing
/// inputs so independent owners report theirs in the same pass.
///
/// Declaration syntax and placement are judged as the walk reads them, behind
/// a false `when:` too.
///
/// ## Errors
///
/// A malformed declaration or task, an unresolvable reference that reads no
/// pending input, a cycle, and an owner's preparation failure other than a
/// missing input (an invalid supplied value, an unloadable schema).
pub fn discover_sequence_inputs(
    source: &ResolvedCompositionSource,
    options: InputDiscoveryOptions<'_>,
) -> Result<SequenceInputInventory, CompositionError> {
    let mut loader = match options.invocation {
        Some((invocation, source_context)) => {
            Loader::new_with_invocation(source, options.compose_context.clone(), invocation, source_context.clone())
        }
        None => Loader::new(source, options.compose_context.clone(), options.file_resolution_context.clone()),
    };
    if let Some(Value::Object(setters)) = options.caller_layers.and_then(|layers| layers.set_overrides.as_ref()) {
        loader.caller_setters = setters.clone();
    }
    loader.prompt_memo = options.prompt_memo.cloned();
    let root_path = source.resolved_path.clone();
    loader.ancestry.push((loader.source_path.clone(), None));
    let mut discovery = Discovery { loader, options, inventory: SequenceInputInventory::default() };
    discovery.inventory.sources = discovery.loader.graph.nested_sources.clone();
    let root = discovery.push_owner(source, InputOwnerRole::Root, None, String::new())?;
    let authored: Map<String, Value> = source.markdown.frontmatter().as_map().iter()
        .map(|(key, value)| (key.clone(), crate::composition::closure::stored_text(value)))
        .collect();
    let Some(sequence) = authored.get("sequence") else {
        return Ok(discovery.inventory);
    };
    grammar::validate_sequence_root(&authored, &root_path)?;
    match grammar::classify_source(sequence)? {
        SequenceSourceSpec::Inline(items) => {
            let keys = formal::formal_keys(&authored, &root_path)?;
            let scope = Scope { owner: root, template: keys.template, values: Arc::clone(&discovery.inventory.owners[root].values) };
            discovery.walk_owner_list(&items, &root_path, "sequence", &scope)?;
        }
        SequenceSourceSpec::Reference(reference) => {
            discovery.walk_reference(&reference, &root_path, "sequence", root, InputOwnerRole::RootReference)?;
        }
        // A dynamic source has no authored rows until it runs.
        SequenceSourceSpec::Expression(_) | SequenceSourceSpec::Shell(_) => {}
    }
    Ok(discovery.inventory)
}

/// The owner and template a list's items are read under.
struct Scope<'s> {
    owner: usize,
    template: Option<&'s Map<String, Value>>,
    values: Arc<Map<String, Value>>,
}

struct Discovery<'a> {
    loader: Loader<'a>,
    options: InputDiscoveryOptions<'a>,
    inventory: SequenceInputInventory,
}

/// An item's executable, as discovery reads it.
enum Executable {
    Found(ExecutableField, Value),
    /// Filled by a `template:` expression its owner cannot evaluate yet.
    Pending(ExecutableField, String),
    None,
}

impl Discovery<'_> {
    /// The options an owner is prepared under, resolving from the caller's
    /// context; [`Self::push_owner`] swaps in a non-root owner's own.
    fn options_for(&self, defer: bool) -> PrepareOptions {
        let mut prepare = PrepareOptions::new(self.options.file_resolution_context.clone());
        if let Some(layers) = self.options.caller_layers {
            prepare = layers.apply_to(prepare);
        }
        prepare.prepared_context = Some(self.options.compose_context.clone());
        prepare.defer_schema_verdict = defer;
        prepare
    }

    /// Prepare `source`'s values and record it as an owner.
    ///
    /// A nested or referenced owner's authored values resolve from its own
    /// document, through the context the graph derives for it
    /// ([`Loader::source_context_for`]); the root's from the caller's context,
    /// which is its own.
    fn push_owner(
        &mut self, source: &ResolvedCompositionSource, role: InputOwnerRole,
        referenced_from: Option<PathBuf>, property: String,
    ) -> Result<usize, CompositionError> {
        let document = match role {
            InputOwnerRole::Root => None,
            InputOwnerRole::RootReference | InputOwnerRole::Nested => self.loader.source_context_for(&source.resolved_path)
                .map(|context| context.file_resolution_context().clone()),
        };
        let prepare = |defer| {
            // The caller's values keep their recorded origins either way.
            let mut options = self.options_for(defer);
            if let Some(document) = &document {
                options = options.resolving_document_from(document.clone());
            }
            super::super::document::prepare_sequence_document_values(source, options, CompositionMode::ChainedDocument)
        };
        let (values, missing) = match prepare(false) {
            Ok(prepared) => (prepared.values, None),
            Err(CompositionError::MissingProperties { source_path, missing, frontmatter_description, pointer_paths }) => {
                // The values the rest of discovery reads: everything but the
                // missing inputs, converted as the verdict would convert them.
                let values = prepare(true)?.values;
                let entry = SequenceMissingPropertiesEntry {
                    subject: SequenceMissingSubject::Document { property: property.clone() },
                    source_path, missing, frontmatter_description, pointer_paths,
                };
                (values, Some(entry))
            }
            Err(error) if self.options.leave_verdicts => {
                self.inventory.failed_owners.push(FailedOwner { owner: self.inventory.owners.len(), error });
                let authored = source.markdown.frontmatter().as_map().iter()
                    .map(|(key, value)| (key.clone(), crate::composition::closure::stored_text(value)))
                    .collect();
                (authored, None)
            }
            Err(other) => return Err(other),
        };
        self.inventory.owners.push(InputOwner {
            role, path: source.resolved_path.clone(), referenced_from, property, missing, values: Arc::new(values),
        });
        Ok(self.inventory.owners.len() - 1)
    }

    fn owner_missing(&self, owner: usize) -> bool {
        self.inventory.owners[owner].missing.is_some()
    }

    /// Walk the items of an owner's own `sequence:` list. Each item's grammar
    /// is judged here, which judges every inline list nested in it too.
    /// The items' references resolve from `declaring_path`.
    fn walk_owner_list(
        &mut self, items: &[Value], declaring_path: &Path, list_property: &str, scope: &Scope<'_>,
    ) -> Result<(), CompositionError> {
        let mut readable = Vec::with_capacity(items.len());
        for (index, item) in items.iter().enumerate() {
            if let Value::Object(map) = item {
                let judged = grammar::validate_task_grammar(map, declaring_path, &format!("{list_property}[{index}]"));
                if self.recovered(judged)?.is_none() {
                    // Kept as a placeholder so later items keep their positions.
                    readable.push(Value::Null);
                    continue;
                }
            }
            readable.push(item.clone());
        }
        self.walk_items(&readable, declaring_path, declaring_path, list_property, scope)
    }

    /// `result`, or `Ok(None)` for a failure a recovering discovery skips.
    fn recovered<T>(&self, result: Result<T, CompositionError>) -> Result<Option<T>, CompositionError> {
        match result {
            Ok(value) => Ok(Some(value)),
            Err(_) if self.options.recover => Ok(None),
            Err(error) => Err(error),
        }
    }

    fn walk_items(
        &mut self, items: &[Value], declaring_path: &Path, origin: &Path, list_property: &str, scope: &Scope<'_>,
    ) -> Result<(), CompositionError> {
        for (index, item) in items.iter().enumerate() {
            let property = format!("{list_property}[{index}]");
            let shorthand;
            let map = match item {
                Value::Object(map) => map,
                Value::String(name) if scope.template.is_some() => {
                    shorthand = Map::from_iter([("name".to_string(), Value::String(name.clone()))]);
                    &shorthand
                }
                _ => continue,
            };
            let Some(executable) = self.recovered(self.executable_of(map, scope, declaring_path))? else { continue };
            match executable {
                Executable::Found(field, value) => {
                    let walked = self.walk_executable(field, &value, map, origin, &property, scope);
                    self.recovered(walked)?;
                }
                Executable::Pending(field, reference) => self.inventory.pending.push(PendingReference {
                    owner: scope.owner, source_path: declaring_path.to_path_buf(),
                    property: property_child(&property, field.key()), reference,
                }),
                Executable::None => {}
            }
        }
        Ok(())
    }

    /// The executable an item declares, or the one its owner's `template:`
    /// fills in for it.
    fn executable_of(
        &self, map: &Map<String, Value>, scope: &Scope<'_>, declaring_path: &Path,
    ) -> Result<Executable, CompositionError> {
        if let Some((key, value)) = map.iter().find(|(key, _)| reserved::is_executable_key(key)) {
            let field = ExecutableField::from_key(key).expect("filtered to executable keys");
            return Ok(Executable::Found(field, value.clone()));
        }
        let Some((key, default)) = scope.template
            .and_then(|template| template.iter().find(|(key, _)| reserved::is_executable_key(key)))
        else {
            return Ok(Executable::None);
        };
        let field = ExecutableField::from_key(key).expect("filtered to executable keys");
        if let Value::String(raw) = default && reads_template(raw) && self.owner_missing(scope.owner) {
            return Ok(Executable::Pending(field, raw.clone()));
        }
        let filled = formal::fill_template_default(
            default, map, &*scope.values, declaring_path, self.options.file_resolution_context,
        )?;
        Ok(Executable::Found(field, filled))
    }

    /// Record a reference as pending when it reads a template while its owner
    /// is missing an input.
    fn pending(&mut self, reference: &str, scope: &Scope<'_>, origin: &Path, property: &str) -> bool {
        let pending = reads_template(reference) && self.owner_missing(scope.owner);
        if pending {
            self.inventory.pending.push(PendingReference {
                owner: scope.owner, source_path: origin.to_path_buf(),
                property: property.to_string(), reference: reference.to_string(),
            });
        }
        pending
    }

    /// Follow one executable. `options` is the task's map (its options, and
    /// for a list item its state too).
    /// Walk one executable. A failure raised without a site is located at
    /// the executable's field ([`CompositionError::locate_sequence`]), as the
    /// graph walk locates it.
    fn walk_executable(
        &mut self, field: ExecutableField, value: &Value, options: &Map<String, Value>, origin: &Path,
        property: &str, scope: &Scope<'_>,
    ) -> Result<(), CompositionError> {
        let action_property = property_child(property, field.key());
        self.walk_executable_unlocated(field, value, options, origin, property, &action_property, scope)
            .map_err(|error| error.locate_sequence(origin, &action_property, None))
    }

    #[allow(clippy::too_many_arguments)]
    fn walk_executable_unlocated(
        &mut self, field: ExecutableField, value: &Value, options: &Map<String, Value>, origin: &Path,
        property: &str, action_property: &str, scope: &Scope<'_>,
    ) -> Result<(), CompositionError> {
        match field {
            ExecutableField::Prompt => {
                let reference = expect_reference(value, field, property, origin, action_property)?;
                if self.pending(&reference, scope, origin, action_property) {
                    return Ok(());
                }
                let path = self.loader.load_prompt(&reference, origin)?;
                let prompt = InputPrompt {
                    path, source_path: origin.to_path_buf(), property: property.to_string(),
                    owner: scope.owner, with: option_map(options, "with"),
                };
                if !self.inventory.prompts.iter().any(|known| known.path == prompt.path && known.with == prompt.with) {
                    self.inventory.prompts.push(prompt);
                }
            }
            ExecutableField::Sequence => match grammar::classify_source(value)? {
                SequenceSourceSpec::Inline(items) => {
                    let declarations = declarations::inline_container_declarations(options, origin, property)?;
                    self.inventory.inline_containers.push(InlineContainer {
                        source_path: origin.to_path_buf(), property: property.to_string(),
                        owner: scope.owner, checks: declarations.check.is_some(),
                    });
                    // The container's own template, never its parent's,
                    // fills its rows, from the values its owner sees.
                    let inner = Scope { owner: scope.owner, template: declarations.template, values: Arc::clone(&scope.values) };
                    self.walk_items(&items, origin, origin, action_property, &inner)?;
                }
                SequenceSourceSpec::Reference(reference) => {
                    if !self.pending(&reference.reference, scope, origin, action_property) {
                        self.walk_reference(&reference, origin, action_property, scope.owner, InputOwnerRole::Nested)?;
                    }
                }
                SequenceSourceSpec::Expression(_) | SequenceSourceSpec::Shell(_) => {
                    return Err(CompositionError::sequence_invalid(
                        "a nested `sequence:` must be a list or file reference",
                        crate::composition::SequenceSite::at(origin, action_property),
                    ));
                }
            },
            ExecutableField::Group => self.walk_group(value, origin, action_property, scope)?,
            ExecutableField::Task => {
                let reference = expect_reference(value, field, property, origin, action_property)?;
                if self.pending(&reference, scope, origin, action_property) {
                    return Ok(());
                }
                let path = self.loader.resolve_reference(&reference, origin, ExecutableField::Task)?;
                let (document, _) = self.loader.load_data_document(&path, ExecutableField::Task)?;
                let root = expect_object(&document.value, "task file", &path)?;
                expect_kind(root, "task", &path)?;
                let (field, value, options) = parse_task_object(root, &path, "")?;
                self.loader.enter(&path)?;
                let leaf = Scope { owner: scope.owner, template: None, values: Arc::clone(&scope.values) };
                let result = self.walk_executable(field, &value, &options, &path, "", &leaf);
                self.loader.leave();
                result?;
            }
            ExecutableField::Shell | ExecutableField::SideEffect => {}
        }
        Ok(())
    }

    fn walk_group(
        &mut self, value: &Value, origin: &Path, property: &str, scope: &Scope<'_>,
    ) -> Result<(), CompositionError> {
        let leaf = Scope { owner: scope.owner, template: None, values: Arc::clone(&scope.values) };
        match value {
            Value::Object(group) => self.walk_legs(group, origin, property, &leaf),
            Value::String(raw) => {
                if self.pending(raw, scope, origin, property) {
                    return Ok(());
                }
                let (name, reference) = split_catalog_reference(raw);
                let path = self.loader.resolve_reference(&reference, origin, ExecutableField::Group)?;
                let (document, _) = self.loader.load_data_document(&path, ExecutableField::Group)?;
                let root = expect_object(&document.value, "group file", &path)?;
                let (group, group_property) = match &name {
                    Some(name) => {
                        let (index, group) = select_catalog_group(&document, root, name, &path)?;
                        (group, format!("groups[{index}]"))
                    }
                    None => {
                        expect_kind(root, "group", &path)?;
                        (root, String::new())
                    }
                };
                grammar::group_object_grammar(group, &path, &group_property)?;
                self.loader.enter_reference(&path, name)?;
                let result = self.walk_legs(group, &path, &group_property, &leaf);
                self.loader.leave();
                result
            }
            // The graph reports the shape; discovery has nothing to follow.
            _ => Ok(()),
        }
    }

    fn walk_legs(
        &mut self, group: &Map<String, Value>, origin: &Path, property: &str, scope: &Scope<'_>,
    ) -> Result<(), CompositionError> {
        for (index, leg) in group.get("tasks").and_then(Value::as_array).into_iter().flatten().enumerate() {
            let Value::Object(leg) = leg else { continue };
            let leg_property = property_child(property, &format!("tasks[{index}]"));
            let walked = parse_task_object(leg, origin, &leg_property)
                .and_then(|(field, value, options)| self.walk_executable(field, &value, &options, origin, &leg_property, scope));
            self.recovered(walked)?;
        }
        Ok(())
    }

    /// Follow a `sequence:` file reference authored at `property` in
    /// `origin`. A whole document becomes an owner of its own; a selection is
    /// data of the enclosing `owner`.
    fn walk_reference(
        &mut self, reference: &SequenceReference, origin: &Path, property: &str, owner: usize, role: InputOwnerRole,
    ) -> Result<(), CompositionError> {
        let path = self.loader.resolve_reference(&reference.reference, origin, ExecutableField::Sequence)?;
        self.loader.enter(&path)?;
        let result = self.walk_referenced(reference, &path, origin, property, owner, role);
        self.loader.leave();
        result
    }

    fn walk_referenced(
        &mut self, reference: &SequenceReference, path: &Path, origin: &Path, property: &str, owner: usize,
        role: InputOwnerRole,
    ) -> Result<(), CompositionError> {
        let (document, _) = self.loader.load_data_document(path, ExecutableField::Sequence)?;
        let whole = reference.offset.is_none() && reference.operator.is_none();
        if whole && let Some(root) = document.value.as_object() && root.contains_key("sequence") {
            grammar::validate_sequence_root(root, path)?;
            let keys = formal::formal_keys(root, path)?;
            let source = source_resolution::whole_document_source(path, root, &document.orders)?;
            let owner = self.push_owner(&source, role, Some(origin.to_path_buf()), property.to_string())?;
            let items = super::super::data::expect_list(&root["sequence"], Some("sequence"))?;
            // The template reads only this owner's prepared values, as the
            // plan does; the referrer's never enter them.
            let scope = Scope { owner, template: keys.template, values: Arc::clone(&self.inventory.owners[owner].values) };
            // A whole document's rows resolve their references from it, a
            // root reference's included, as the graph resolves them.
            return self.walk_owner_list(&items, path, "sequence", &scope);
        }
        // A selection is the enclosing owner's data: its rows are read the
        // way the graph reads them, with no document of their own. A root
        // reference's selection runs as the root's rows, from the root's
        // directory; a nested container's from the selected file's.
        let rows_origin = if role == InputOwnerRole::RootReference { origin.to_path_buf() } else { path.to_path_buf() };
        let values = Arc::clone(&self.inventory.owners[owner].values);
        let options = super::super::SequenceSourceOptions::new(self.options.file_resolution_context);
        // A recovering discovery skips a selected row that does not read, or
        // whose executable fails, as it skips an authored one; the graph walk
        // reports it.
        let recovered = if self.options.recover {
            source_resolution::plan_referenced_sequence_recovering(
                reference, path, &document.value, Arc::clone(&document.orders), origin, &*values, true, &options,
            )?.0
        } else {
            super::super::normalize::RecoveredPlan::whole(source_resolution::plan_referenced_sequence(
                reference, path, &document.value, Arc::clone(&document.orders), origin, &*values, true, &options,
            )?)
        };
        let scope = Scope { owner, template: None, values };
        for (step, ordinal) in recovered.plan.steps.iter().zip(&recovered.ordinals) {
            if let Some(executable) = &step.executable {
                let walked = self.walk_executable(
                    executable.field, &executable.value, &executable.options, &rows_origin,
                    &format!("sequence[{ordinal}]"), &scope,
                );
                self.recovered(walked)?;
            }
        }
        Ok(())
    }
}

/// Whether `raw` interpolates, so its value depends on the owner's inputs.
fn reads_template(raw: &str) -> bool {
    raw.contains("{{")
}

#[cfg(test)]
mod tests;
