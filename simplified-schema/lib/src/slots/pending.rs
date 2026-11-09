//! A template found by a resolution entry point, with what it takes to
//! resolve it: the slots, the exact schema texts the first pass read, and a
//! replay of that same resolution.

use std::fmt;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use biscuit_file::FileResolutionContext;

use super::host::{SlotAdmissionError, SlotHost, SlotRequest, SlotResolutionError, SlotRunRequest};
use super::reads::with_template_reads;
use super::substitute::{SlotValues, with_values};
use crate::errors::{SchemaError, UnresolvedSlots};
use crate::resolve::{AuthoredSchemaValue, ResolvedSchema};
use crate::resolve::buffers::{self, SourceBuffers};
use crate::simplified::{DefinitionTable, SchemaRoot, Slot};

/// An expanded template: the declaration after import expansion, slots in
/// place, and the stored definitions it reaches.
#[derive(Debug, Clone, PartialEq)]
pub struct Template {
    pub(crate) root: SchemaRoot,
    pub(crate) definitions: Arc<DefinitionTable>,
    /// The template is one arm of a root union, not the whole root.
    pub(crate) union_arm: bool,
    /// For a root-union arm, the top-level properties every arm of that
    /// union declares, read from the arms as written before any slot is
    /// evaluated ([`PendingSchema::declared_properties`]).
    pub(crate) every_arm_declares: Arc<[String]>,
}

impl Template {
    /// `root` and `definitions` as a whole schema's template.
    pub(crate) fn new(root: SchemaRoot, definitions: Arc<DefinitionTable>) -> Self {
        Self { root, definitions, union_arm: false, every_arm_declares: Arc::default() }
    }

    /// Marks this template as one arm of a root union whose arms all
    /// declare `every_arm_declares`.
    pub(crate) fn mark_union_arm(&mut self, every_arm_declares: Vec<String>) {
        self.union_arm = true;
        self.every_arm_declares = every_arm_declares.into();
    }

    /// The top-level properties this template declares as a whole object;
    /// nothing for one arm of a root union, which speaks for that arm only.
    pub(crate) fn single_object_names(&self) -> Vec<String> {
        match &self.root {
            SchemaRoot::Single(shape) if !self.union_arm => shape.properties.keys().cloned().collect(),
            _ => Vec::new(),
        }
    }
}

/// The resolution that met the template, run again under slot values.
pub(crate) type Replay = Arc<dyn Fn() -> Result<ResolvedSchema, SchemaError> + Send + Sync>;

/// A template a resolution entry point met, ready to resolve
/// ([`UnresolvedSlots::pending`]).
///
/// [`Self::resolve`] runs the entry point's own resolution again with the
/// slots' values in force and the schema texts the first pass read served
/// in place of the files, so the result never mixes two revisions of a file
/// and is exactly what a concrete schema with those values resolves to.
/// The pending schema is never changed; each call starts from the template.
#[derive(Clone)]
pub struct PendingSchema {
    slots: Vec<Slot>,
    template: Option<Template>,
    sources: Arc<[(PathBuf, Arc<str>)]>,
    /// Where the inline value holding the template's own slots was written.
    authored: Option<AuthoredText>,
    buffers: SourceBuffers,
    replay: Replay,
    /// Top-level properties [`Self::without_properties`] left out.
    omitted: Arc<[String]>,
}

impl PendingSchema {
    /// The template `unresolved` reports, replayed by `replay` under the
    /// open buffers `buffers` and the texts `sources` the first pass read,
    /// its inline value written in `authored`.
    pub(crate) fn new(
        unresolved: &UnresolvedSlots,
        mut buffers: SourceBuffers,
        sources: Vec<(PathBuf, Arc<str>)>,
        authored: Option<AuthoredText>,
        replay: Replay,
    ) -> Self {
        for (path, text) in &sources {
            buffers.insert(path, Arc::clone(text));
        }
        Self {
            slots: unresolved.slots.clone(),
            template: unresolved.template.clone(),
            sources: sources.into(),
            authored,
            buffers,
            replay,
            omitted: Arc::default(),
        }
    }

    /// This template with the top-level properties `discarded` names left
    /// out, as if they had never been written: their slots, and every slot
    /// of a stored definition only they reach, are gone from
    /// [`Self::slots`], and [`Self::resolve`] neither evaluates them nor
    /// keeps the properties. A host whose layering replaces those properties
    /// with another layer's resolves only what survives.
    ///
    /// Unchanged when no `discarded` name is a top-level property of the
    /// template, or when the template is a root union or one of its arms (a
    /// union's arms are layered one by one, so no property is discarded
    /// from all of them here).
    #[must_use]
    pub fn without_properties(&self, discarded: &[String]) -> Self {
        let Some(template) = self.template.as_ref().filter(|template| !template.union_arm) else {
            return self.clone();
        };
        let SchemaRoot::Single(shape) = &template.root else {
            return self.clone();
        };
        let mut omitted: Vec<String> = self.omitted.to_vec();
        omitted.extend(discarded.iter().filter(|name| shape.properties.contains_key(*name)).cloned());
        if omitted.len() == self.omitted.len() {
            return self.clone();
        }
        omitted.sort();
        omitted.dedup();
        let mut root = template.root.clone();
        let mut definitions = template.definitions.as_ref().clone();
        super::omit_properties(&mut root, &mut definitions, &omitted);
        let surviving = super::collect_slots(&root, &definitions);
        let slots = self
            .slots
            .iter()
            .filter(|slot| surviving.iter().any(|kept| super::SlotKey::of(kept) == super::SlotKey::of(slot)))
            .cloned()
            .collect();
        Self {
            slots,
            template: Some(Template::new(root, Arc::new(definitions))),
            sources: Arc::clone(&self.sources),
            authored: self.authored.clone(),
            buffers: self.buffers.clone(),
            replay: Arc::clone(&self.replay),
            omitted: omitted.into(),
        }
    }

    /// The top-level properties [`Self::without_properties`] left out,
    /// sorted; empty for the template as resolution met it. A host that
    /// caches resolutions keys on them: the same files and slots resolve to
    /// a different schema without these properties.
    pub fn omitted_properties(&self) -> &[String] {
        &self.omitted
    }

    /// Whether [`Self::template`] is one arm of a root union rather than the
    /// whole root: its top-level properties are then that arm's alone.
    pub fn is_union_arm(&self) -> bool {
        self.template.as_ref().is_some_and(|template| template.union_arm)
    }

    /// The top-level properties the template's whole schema declares in
    /// every alternative, so that a lower layer's declaration of them never
    /// survives a whole-property merge: a single object's properties, or,
    /// when the template is one arm of a root union, the names every arm of
    /// that union declares (each arm read as written, a referenced arm as
    /// it resolves). Exactly what the resolved schema's root declares in
    /// each of its arms.
    ///
    /// Passive: reading it evaluates nothing and reads no file.
    pub fn declared_properties(&self) -> Vec<String> {
        match &self.template {
            Some(template) if template.union_arm => template.every_arm_declares.to_vec(),
            Some(template) => template.single_object_names(),
            None => Vec::new(),
        }
    }

    /// Resolves a template with no slot left ([`Self::slots`] is empty, as
    /// after [`Self::without_properties`] discarded every one): replays the
    /// resolution with no values, so nothing is evaluated or asked of a
    /// host.
    ///
    /// ## Errors
    ///
    /// [`SchemaError::UnresolvedSlots`] when a slot remains; otherwise the
    /// resolution's own failure.
    pub fn resolve_without_slots(&self) -> Result<ResolvedSchema, SchemaError> {
        if !self.slots.is_empty() {
            let mut unresolved = UnresolvedSlots::new(self.slots.clone());
            unresolved.pending = Some(self.clone());
            return Err(SchemaError::UnresolvedSlots(Box::new(unresolved)));
        }
        self.replay_with(&SlotValues::default())
    }

    /// The recorded resolution, run again with `values` in force, the
    /// recorded schema texts served, and the omitted properties left out.
    /// A file the replay reads that the first pass did not (an import a
    /// template type's returned definition names) is read once, however many
    /// properties use it.
    fn replay_with(&self, values: &SlotValues) -> Result<ResolvedSchema, SchemaError> {
        buffers::with(&self.buffers, || {
            buffers::with_reads_retained(|| {
                with_values(values, || super::omitting(&self.omitted, || (self.replay)()))
            })
        })
    }

    /// The slots the first pass met, in declaration order. A root union
    /// whose later arms hold slots of their own reports those as resolution
    /// reaches them.
    pub fn slots(&self) -> &[Slot] {
        &self.slots
    }

    /// The expanded template the slots were found in, with its stored
    /// definitions: for inspection only, never lowered.
    pub fn template(&self) -> Option<(&SchemaRoot, &Arc<DefinitionTable>)> {
        self.template.as_ref().map(|template| (&template.root, &template.definitions))
    }

    /// Every schema file the first pass read (the entry file and each
    /// transitive import or referenced file), with the exact text read, in
    /// read order. A host keys a resolution cache on these texts; resolution
    /// reads them again from here, never from the disk.
    pub fn read_sources(&self) -> &[(PathBuf, Arc<str>)] {
        &self.sources
    }

    /// Where the inline `$schema` value this template was resolved from is
    /// written, when the host supplied that text and it proves the value's
    /// position ([`crate::resolve::resolve_authored_yaml_schema`]): the
    /// slots written in the value then carry byte ranges in this text. A
    /// slot of the value has no [`Slot::origin`] when the value is a
    /// document's; a trigger payload's slots name this file.
    pub fn authored_source(&self) -> Option<&AuthoredSchemaValue> {
        self.authored.as_ref().map(|authored| authored.value.as_ref())
    }

    /// The text `slot`'s spans are byte ranges of, and the file it is, when
    /// known: the read schema file its origin names, else the authored
    /// value's text for a slot written there. `None` when no text proves
    /// the slot's position.
    pub fn slot_text(&self, slot: &Slot) -> Option<(Option<&std::path::Path>, &Arc<str>)> {
        slot_text(&self.sources, self.authored.as_ref(), slot)
    }

    /// The authored text, for a failure of this template to keep.
    pub(crate) fn authored_text(&self) -> Option<&AuthoredText> {
        self.authored.as_ref()
    }

    /// Admits every slot, evaluates each through `host`, and resolves the
    /// template with the values: an ordinary concrete [`ResolvedSchema`].
    ///
    /// ## Errors
    ///
    /// [`SlotResolutionError`]: refused slots (none is evaluated), a read a
    /// template type's returned definition needs that `host`'s
    /// [`SlotHost::template_reads`] refused, failed evaluations (all
    /// attempted), a value that cannot fill its position
    /// ([`SchemaError::SlotResult`]), or cancellation.
    pub fn resolve(&self, host: &dyn SlotHost, request: &SlotRunRequest) -> Result<ResolvedSchema, SlotResolutionError> {
        let reads = host.template_reads();
        resolve_with(host, &request.context, self.slots.clone(), |values| {
            let (replayed, refusal) = with_template_reads(Rc::clone(&reads), || self.replay_with(values));
            match (replayed, refusal) {
                // A refusal outranks whatever the expansion made of it.
                (_, Some(refused)) => Attempt::Refused(vec![refused]),
                (Ok(resolved), None) => Attempt::Done(resolved),
                (Err(SchemaError::UnresolvedSlots(unresolved)), None) => Attempt::Unresolved(unresolved.slots),
                (Err(error), None) => Attempt::Failed(error),
            }
        })
    }
}

impl fmt::Debug for PendingSchema {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PendingSchema")
            .field("slots", &self.slots)
            .field("sources", &self.sources.iter().map(|(path, _)| path).collect::<Vec<_>>())
            .finish_non_exhaustive()
    }
}

impl PartialEq for PendingSchema {
    /// Two handles are equal when they hold the same slots and replay the
    /// same resolution without the same properties.
    fn eq(&self, other: &Self) -> bool {
        self.slots == other.slots && Arc::ptr_eq(&self.replay, &other.replay) && self.omitted == other.omitted
    }
}

/// What one attempt to build the schema under slot values produced.
pub(crate) enum Attempt<T> {
    /// The concrete result.
    Done(T),
    /// Slots the values did not cover, found as resolution reached them.
    Unresolved(Vec<Slot>),
    /// The schema the values make is not valid.
    Failed(SchemaError),
    /// The host refused a read a template type's returned definition needs.
    Refused(Vec<(Slot, SlotAdmissionError)>),
}

/// The evaluation loop every resolution shares: admit the round's slots,
/// evaluate each, and `attempt` the schema; slots `attempt` reports that the
/// values did not cover (a root-union arm reached later) start another
/// round. Each round adds at least one slot, so the loop ends.
pub(crate) fn resolve_with<T>(
    host: &dyn SlotHost,
    context: &FileResolutionContext,
    initial: Vec<Slot>,
    mut attempt: impl FnMut(&SlotValues) -> Attempt<T>,
) -> Result<T, SlotResolutionError> {
    let mut values = SlotValues::default();
    let mut round = initial;
    loop {
        let mut unique: Vec<Slot> = Vec::with_capacity(round.len());
        for slot in round {
            if values.get(&slot).is_none() && !unique.iter().any(|seen| super::SlotKey::of(seen) == super::SlotKey::of(&slot)) {
                unique.push(slot);
            }
        }
        if host.is_canceled() {
            return Err(SlotResolutionError::Canceled);
        }
        let refused: Vec<_> = unique
            .iter()
            .filter_map(|slot| host.admit(slot).err().map(|error| (slot.clone(), error)))
            .collect();
        if !refused.is_empty() {
            return Err(SlotResolutionError::Refused(refused));
        }
        let mut failed = Vec::new();
        for slot in &unique {
            if host.is_canceled() {
                return Err(SlotResolutionError::Canceled);
            }
            let request = SlotRequest { slot, defining_file: slot.origin.as_deref(), context };
            match host.resolve(&request) {
                Ok(value) => {
                    values.insert(slot, value);
                }
                Err(failure) => failed.push((slot.clone(), failure)),
            }
        }
        if !failed.is_empty() {
            return Err(SlotResolutionError::Failed(failed));
        }
        match attempt(&values) {
            Attempt::Done(result) => {
                if host.is_canceled() {
                    return Err(SlotResolutionError::Canceled);
                }
                return Ok(result);
            }
            Attempt::Unresolved(slots) => {
                if slots.iter().all(|slot| values.get(slot).is_some()) {
                    // Every reported slot has a value, yet the schema is
                    // still a template: fail rather than loop.
                    return Err(SlotResolutionError::Schema(Box::new(SchemaError::UnresolvedSlots(Box::new(
                        UnresolvedSlots::new(slots),
                    )))));
                }
                round = slots;
            }
            Attempt::Failed(error) => return Err(SlotResolutionError::Schema(Box::new(error))),
            Attempt::Refused(refused) => return Err(SlotResolutionError::Refused(refused)),
        }
    }
}

/// The text an inline `$schema` value was written in
/// ([`PendingSchema::authored_source`]), and whose slots it holds.
#[derive(Debug, Clone)]
pub(crate) struct AuthoredText {
    pub(crate) value: Arc<AuthoredSchemaValue>,
    /// The value is a document's: its slots have no origin. A trigger
    /// payload's slots name `value.file` instead.
    pub(crate) document: bool,
}

/// The text `slot`'s spans are ranges of, among the read `sources` and the
/// `authored` inline value, with the file it is: see
/// [`PendingSchema::slot_text`].
pub(crate) fn slot_text<'a>(
    sources: &'a [(PathBuf, Arc<str>)],
    authored: Option<&'a AuthoredText>,
    slot: &Slot,
) -> Option<(Option<&'a std::path::Path>, &'a Arc<str>)> {
    let authored_text = |authored: &'a AuthoredText| (authored.value.file.as_deref(), &authored.value.text);
    match slot.origin.as_deref() {
        Some(origin) => sources
            .iter()
            .find(|(path, _)| path == origin)
            .map(|(path, text)| (Some(path.as_path()), text))
            .or_else(|| {
                authored
                    .filter(|authored| !authored.document && authored.value.file.as_deref() == Some(origin))
                    .map(authored_text)
            }),
        None => authored.filter(|authored| authored.document).map(authored_text),
    }
}
