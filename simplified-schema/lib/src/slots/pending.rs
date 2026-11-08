//! A template found by a resolution entry point, with what it takes to
//! resolve it: the slots, the exact schema texts the first pass read, and a
//! replay of that same resolution.

use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;

use biscuit_file::FileResolutionContext;

use super::host::{SlotHost, SlotRequest, SlotResolutionError, SlotRunRequest};
use super::substitute::{SlotValues, with_values};
use crate::errors::{SchemaError, UnresolvedSlots};
use crate::resolve::ResolvedSchema;
use crate::resolve::buffers::{self, SourceBuffers};
use crate::simplified::{DefinitionTable, SchemaRoot, Slot};

/// An expanded template: the declaration after import expansion, slots in
/// place, and the stored definitions it reaches.
#[derive(Debug, Clone, PartialEq)]
pub struct Template {
    pub(crate) root: SchemaRoot,
    pub(crate) definitions: Arc<DefinitionTable>,
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
    buffers: SourceBuffers,
    replay: Replay,
}

impl PendingSchema {
    /// The template `unresolved` reports, replayed by `replay` under the
    /// open buffers `buffers` and the texts `sources` the first pass read.
    pub(crate) fn new(
        unresolved: &UnresolvedSlots,
        mut buffers: SourceBuffers,
        sources: Vec<(PathBuf, Arc<str>)>,
        replay: Replay,
    ) -> Self {
        for (path, text) in &sources {
            buffers.insert(path, Arc::clone(text));
        }
        Self {
            slots: unresolved.slots.clone(),
            template: unresolved.template.clone(),
            sources: sources.into(),
            buffers,
            replay,
        }
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

    /// Admits every slot, evaluates each through `host`, and resolves the
    /// template with the values: an ordinary concrete [`ResolvedSchema`].
    ///
    /// ## Errors
    ///
    /// [`SlotResolutionError`]: refused slots (none is evaluated), failed
    /// evaluations (all attempted), a value that cannot fill its position
    /// ([`SchemaError::SlotResult`]), or cancellation.
    pub fn resolve(&self, host: &dyn SlotHost, request: &SlotRunRequest) -> Result<ResolvedSchema, SlotResolutionError> {
        resolve_with(host, &request.context, self.slots.clone(), |values| {
            match buffers::with(&self.buffers, || with_values(values, || (self.replay)())) {
                Ok(resolved) => Attempt::Done(resolved),
                Err(SchemaError::UnresolvedSlots(unresolved)) => Attempt::Unresolved(unresolved.slots),
                Err(error) => Attempt::Failed(error),
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
    /// same resolution.
    fn eq(&self, other: &Self) -> bool {
        self.slots == other.slots && Arc::ptr_eq(&self.replay, &other.replay)
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
        }
    }
}
