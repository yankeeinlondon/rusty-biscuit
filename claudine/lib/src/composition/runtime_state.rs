//! Invocation-local mutations, output accumulation, and lexical sequence data.
//!
//! Frontmatter layers contain authored values and runtime `set` writes.
//! Sequence values travel separately through `LayeredOverrides`, so compose
//! cannot persist them or overwrite an authored `fm.seq` property. The live
//! runtime cell supplies event-time outputs; a snapshot supplies a reread.
//!
//! Every read copies its fields out under one lock acquisition and releases
//! the lock before returning, so no reader holds the cell while a lifecycle
//! action writes to it, and a pair read together (`mutations_and_outputs`,
//! `sequence_scope`) comes from one moment. Readers take the narrowest
//! projection that answers them; [`RuntimeState::snapshot`] is for readers of
//! the whole state, because it copies the whole output history.

use std::collections::BTreeSet;
use std::sync::Mutex;

use darkmatter::effects::{EffectEngine, EffectError};
use darkmatter::markdown::FrontmatterMap;
use darkmatter::markdown::compose::{ComposeOptions, OverrideLayer, OverrideOrigin};
use indexmap::IndexMap;
use serde_json::{Map, Value};

use super::sequence::reserved::ROOT_OVERLAY_KEYS;

/// The output member name within the sequence runtime root.
pub const OUTPUTS_KEY: &str = "outputs";

/// A poisoned cell means a task panicked mid-write; there is no partial state
/// worth recovering, so every accessor treats it as unrecoverable.
const POISONED: &str = "runtime state mutex poisoned by a panicking task";

/// Why a runtime `set` was refused.
#[derive(Debug, thiserror::Error)]
pub enum RuntimeMutationError {
    /// The target names a reserved root overlay key. Those are read-only views
    /// owned by the executor, never author-writable. The *generated* state keys
    /// (`id`, `index`, `count`, …) are deliberately absent: they are reserved
    /// inside a step's `state` object, not at the frontmatter root, so an
    /// ordinary document may still carry and mutate a root `count`.
    #[error("`{key}` is reserved and cannot be written by `set`")]
    ReservedKey {
        /// The refused key.
        key: String,
    },
    /// Darkmatter refused the key shape (empty, or a dotted path — v1 `set`
    /// writes top-level keys only).
    #[error(transparent)]
    Effect(#[from] EffectError),
}

/// A point-in-time copy of the runtime layers, safe to hand to preparation.
///
/// Taken with [`RuntimeState::snapshot`] so composition never holds a borrow of
/// the live cell while a lifecycle action may be writing to it.
#[derive(Debug, Default, Clone)]
pub struct RuntimeSnapshot {
    /// Accumulated `set` writes, in first-write order.
    pub mutations: FrontmatterMap,
    /// Committed output entries, in execution order.
    pub outputs: Vec<Value>,
    /// Innermost sequence position, separate from authored mutations.
    pub seq: Option<Value>,
    /// Active container handoffs, inherited by parallel runtime branches.
    pub proxy_chain: Vec<std::path::PathBuf>,
}

/// The shared runtime cell. See the [module docs](self).
///
/// Interior mutability is a `Mutex` rather than a `RefCell` because a parallel
/// group's member tasks each hold their *own* cell but share the seams around
/// them; `Sync` is what lets one live behind a borrow that crosses a scoped
/// thread. It is never a contention point: a parallel member writes only to its
/// private buffer, and the merge back into the sequence cell happens on one
/// thread after every sibling has finished.
#[derive(Debug, Default)]
pub struct RuntimeState {
    inner: Mutex<RuntimeSnapshot>,
}

impl RuntimeState {
    /// Create an empty cell: no mutations, no outputs.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a cell pre-loaded with an existing snapshot.
    ///
    /// This is how a parallel group gives each member a private buffer that
    /// *starts* from the group's shared view: the member reads
    /// `{{ last(seq.outputs) }}` as it stood at group start and its own writes land
    /// where only the post-group merge can see them.
    pub fn from_snapshot(snapshot: RuntimeSnapshot) -> Self {
        Self {
            inner: Mutex::new(snapshot),
        }
    }

    /// Install a child position without publishing it as frontmatter.
    pub fn set_sequence_binding(&self, value: Value) {
        self.inner.lock().expect(POISONED).seq = Some(value);
    }

    /// Discard an abandoned container attempt's results while retaining its writes.
    pub fn clear_outputs(&self) {
        self.inner.lock().expect(POISONED).outputs.clear();
    }

    /// Return a looping container's retried pass to its entry state: the
    /// mutation layer it entered with and the first `outputs` entries, which
    /// earlier passes committed. Position and active handoffs are kept.
    pub fn restore_pass(&self, mutations: FrontmatterMap, outputs: usize) {
        let mut inner = self.inner.lock().expect(POISONED);
        inner.mutations = mutations;
        inner.outputs.truncate(outputs);
    }

    /// Swap in a lexical accumulator without undoing child mutations, and
    /// return the one it replaced: a container enters its own scope with this
    /// and restores the enclosing one the same way, moving the enclosing
    /// outputs rather than copying them.
    pub fn replace_sequence_scope(&self, seq: Option<Value>, outputs: Vec<Value>) -> (Option<Value>, Vec<Value>) {
        let mut inner = self.inner.lock().expect(POISONED);
        (std::mem::replace(&mut inner.seq, seq), std::mem::replace(&mut inner.outputs, outputs))
    }

    /// Enter a container handoff and restore its enclosing chain on drop.
    pub(crate) fn enter_container_proxy<'a>(
        &'a self, source: &std::path::Path, target: &std::path::Path, authored: &str,
    ) -> Result<ContainerProxyGuard<'a>, super::error::CompositionError> {
        let mut inner = self.inner.lock().expect(POISONED);
        let before = inner.proxy_chain.len();
        if inner.proxy_chain.is_empty() {
            inner.proxy_chain.push(biscuit_file::canonicalize_simplified(source).unwrap_or_else(|_| source.to_path_buf()));
        }
        if !super::lifecycle::control::proxy_handoff_allowed(&inner.proxy_chain, target)
            || super::lifecycle::prep::proxy_reaches_an_active_caller(target)
        {
            let error = super::error::CompositionError::LifecycleProxyCycle {
                source_path: source.to_path_buf(), target: authored.into(),
                chain: inner.proxy_chain.iter().map(|path| biscuit_file::to_portable_string(path)).collect(),
                limit: super::lifecycle::control::MAX_PROXY_HOPS,
            };
            inner.proxy_chain.truncate(before);
            return Err(error);
        }
        inner.proxy_chain.push(target.to_path_buf());
        Ok(ContainerProxyGuard { runtime: self, before })
    }

    /// The lexical sequence with the current committed outputs.
    pub fn sequence_value(&self) -> Value {
        let (seq, outputs) = self.sequence_scope();
        sequence_value_of(seq, outputs)
    }

    /// Copy every layer out of the cell.
    ///
    /// Only for a reader that needs the whole state, such as a private branch
    /// seeded from it or a fold of mutations, position, and outputs together.
    /// A reader of some fields takes the narrow projection that copies only
    /// those, so it never copies the output history it does not read.
    pub fn snapshot(&self) -> RuntimeSnapshot {
        crate::note_work!(RuntimeSnapshot);
        let snapshot = self.inner.lock().expect(POISONED).clone();
        crate::note_work!(RuntimeOutputsCopy, snapshot.outputs.len());
        snapshot
    }

    /// The accumulated `set` writes, in first-write order.
    pub fn mutations(&self) -> FrontmatterMap {
        self.inner.lock().expect(POISONED).mutations.clone()
    }

    /// The keys of the accumulated `set` writes, without their values.
    pub fn mutation_keys(&self) -> Vec<String> {
        self.inner.lock().expect(POISONED).mutations.keys().cloned().collect()
    }

    /// The mutations and the committed outputs as they stood at one moment.
    pub fn mutations_and_outputs(&self) -> (FrontmatterMap, Vec<Value>) {
        let inner = self.inner.lock().expect(POISONED);
        crate::note_work!(RuntimeOutputsCopy, inner.outputs.len());
        (inner.mutations.clone(), inner.outputs.clone())
    }

    /// The mutations and how many outputs were committed at one moment: the
    /// entry state of a looping container's pass.
    pub fn mutations_and_output_count(&self) -> (FrontmatterMap, usize) {
        let inner = self.inner.lock().expect(POISONED);
        (inner.mutations.clone(), inner.outputs.len())
    }

    /// The committed entries from index `start` on; empty when `start` is
    /// past the end.
    pub fn outputs_from(&self, start: usize) -> Vec<Value> {
        let inner = self.inner.lock().expect(POISONED);
        let tail = &inner.outputs[start.min(inner.outputs.len())..];
        crate::note_work!(RuntimeOutputsCopy, tail.len());
        tail.to_vec()
    }

    /// The innermost sequence position and the committed outputs at one
    /// moment, without the mutations.
    pub fn sequence_scope(&self) -> (Option<Value>, Vec<Value>) {
        let inner = self.inner.lock().expect(POISONED);
        crate::note_work!(RuntimeOutputsCopy, inner.outputs.len());
        (inner.seq.clone(), inner.outputs.clone())
    }

    /// Apply one programmatic runtime write and return the value it replaced.
    ///
    /// `prior_base` supplies the effective document state so the returned prior
    /// value is what the author would have read *before* this write — the
    /// document's own value on a first write, the previous mutation afterwards.
    ///
    /// ## Errors
    ///
    /// [`RuntimeMutationError::ReservedKey`] for a reserved overlay/generated
    /// key, [`RuntimeMutationError::Effect`] for an empty or dotted key.
    pub fn set(
        &self,
        engine: &EffectEngine,
        key: &str,
        value: Value,
        prior_base: &Map<String, Value>,
    ) -> Result<Value, RuntimeMutationError> {
        let mut updates = IndexMap::new();
        updates.insert(key.to_string(), value);
        Ok(self
            .set_batch(engine, &updates, prior_base)?
            .shift_remove(key)
            .expect("single update returns one prior value"))
    }

    /// Atomically commit one mapping-based lifecycle `set` action.
    ///
    /// Every destination is validated before the cell is locked. The commit is
    /// prepared against a clone and published under one mutex acquisition, so
    /// a refusal cannot expose a partial batch. Prior values are selected by
    /// key presence: an explicitly stored runtime null overrides a non-null
    /// document value.
    pub fn set_batch(
        &self,
        engine: &EffectEngine,
        updates: &IndexMap<String, Value>,
        prior_base: &dyn crate::composition::sequence::expr::DocumentFrontmatter,
    ) -> Result<IndexMap<String, Value>, RuntimeMutationError> {
        let mut validated = FrontmatterMap::new();
        for (key, value) in updates {
            if ROOT_OVERLAY_KEYS.contains(&key.as_str()) {
                return Err(RuntimeMutationError::ReservedKey { key: key.clone() });
            }
            engine.set(&mut validated, key, value.clone())?;
        }

        let mut inner = self.inner.lock().expect(POISONED);
        let mut next = inner.mutations.clone();
        let mut prior = IndexMap::with_capacity(updates.len());
        for (key, value) in updates {
            prior.insert(
                key.clone(),
                next.get(key)
                    .cloned()
                    .or_else(|| prior_base.top_level(key).cloned())
                    .unwrap_or(Value::Null),
            );
            next.insert(key.clone(), value.clone());
        }
        inner.mutations = next;
        Ok(prior)
    }

    /// Commit one task's captured stdout as the next `outputs` entry.
    ///
    /// One trailing transport newline is removed; all other whitespace is
    /// preserved (spec → *Output capture boundary*).
    pub fn append_output(&self, text: &str) {
        self.append_output_entry(Value::String(trim_transport_newline(text).to_string()));
    }

    /// Commit a native result: text, a sequence array, or a named group object.
    pub fn append_output_entry(&self, entry: Value) {
        self.inner.lock().expect(POISONED).outputs.push(entry);
    }

    /// The committed entries as the `seq.outputs` value.
    pub fn outputs_value(&self) -> Value {
        let inner = self.inner.lock().expect(POISONED);
        crate::note_work!(RuntimeOutputsCopy, inner.outputs.len());
        Value::Array(inner.outputs.clone())
    }

    /// How many entries have been committed.
    pub fn output_count(&self) -> usize {
        self.inner.lock().expect(POISONED).outputs.len()
    }

    /// The most recent entry, when it is a plain string.
    ///
    /// Container results have no single text, so they yield `None` without flattening.
    pub fn last_output_text(&self) -> Option<String> {
        match self.inner.lock().expect(POISONED).outputs.last() {
            Some(Value::String(text)) => Some(text.clone()),
            _ => None,
        }
    }
}

impl RuntimeSnapshot {
    /// Whole-root replacement prevents fallback to an enclosing accumulator.
    pub fn sequence_value(&self) -> Value {
        sequence_value_of(self.seq.clone(), self.outputs.clone())
    }

    /// The committed entries as the `seq.outputs` value.
    pub fn outputs_value(&self) -> Value {
        Value::Array(self.outputs.clone())
    }
}

/// The `seq` value of position `seq` (standalone when `None`) with `outputs`
/// as its `outputs` member.
pub(crate) fn sequence_value_of(seq: Option<Value>, outputs: Vec<Value>) -> Value {
    let mut value = seq.unwrap_or_else(||
        crate::composition::lifecycle::bindings::SequenceBinding::standalone(Vec::new()).to_value());
    value.as_object_mut().expect("sequence bindings are objects")
        .insert(OUTPUTS_KEY.into(), Value::Array(outputs));
    value
}

/// Remove exactly one trailing transport newline (`\n`, or `\r\n`).
///
/// A provider or shell command terminates its final line; that terminator is a
/// transport artifact, not content. A *second* trailing newline is content the
/// author wrote and is preserved.
pub fn trim_transport_newline(text: &str) -> &str {
    match text.strip_suffix('\n') {
        Some(rest) => rest.strip_suffix('\r').unwrap_or(rest),
        None => text,
    }
}

/// Top-level frontmatter overrides together with the origin of each key.
///
/// This is the one shape runtime values take on their way into Darkmatter. A
/// key is either **authored** — a person typed it (`--set`, `key=value`), so it
/// is a template scanned once like the document's own frontmatter — or
/// **data** — the run produced it (agent or task output, a lifecycle `set:`, a
/// loop value, the step overlay, a `proxy.with:` overlay), so Darkmatter never
/// evaluates its `{{ … }}` or runs its `$( … )`. Values stay raw and typed;
/// origin travels beside them and no token or escaping is ever added.
///
/// [`apply_to`](Self::apply_to) is the only place Claudine hands overrides to
/// Darkmatter compose options.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LayeredOverrides {
    values: Map<String, Value>,
    data_keys: BTreeSet<String>,
    sequence_value: Option<Value>,
}

impl LayeredOverrides {
    /// No overrides.
    pub fn new() -> Self {
        Self::default()
    }

    /// Every key of `values` as authored.
    pub fn authored(values: Option<&Value>) -> Self {
        let mut overrides = Self::new();
        overrides.push(OverrideOrigin::Authored, values);
        overrides
    }

    /// Every key of `values` as data.
    pub fn data(values: Option<&Value>) -> Self {
        let mut overrides = Self::new();
        overrides.push(OverrideOrigin::Data, values);
        overrides
    }

    /// A `proxy.with:` overlay as data.
    ///
    /// A `null` overlay value removes the key from the target's frontmatter
    /// instead (the CLI's `merge_frontmatter_overlay`) and contributes no
    /// override.
    pub fn proxy_overlay(overlay: &indexmap::IndexMap<String, Value>) -> Self {
        let mut overrides = Self::new();
        for (key, value) in overlay {
            if !value.is_null() {
                overrides.insert(OverrideOrigin::Data, key.clone(), value.clone());
            }
        }
        overrides
    }

    /// Rebuild from an effective override object and the keys in it that are
    /// data. A listed key absent from `values` is ignored.
    pub fn from_parts(values: Option<&Value>, data_keys: &BTreeSet<String>) -> Self {
        let values = match values {
            Some(Value::Object(map)) => map.clone(),
            _ => Map::new(),
        };
        let data_keys = data_keys
            .iter()
            .filter(|key| values.contains_key(*key))
            .cloned()
            .collect();
        Self { values, data_keys, sequence_value: None }
    }

    /// Layer every key of `layer` on top: each replaces any earlier value and
    /// takes `origin`. A non-object layer contributes nothing.
    pub fn push(&mut self, origin: OverrideOrigin, layer: Option<&Value>) {
        if let Some(Value::Object(map)) = layer {
            for (key, value) in map {
                self.insert(origin, key.clone(), value.clone());
            }
        }
    }

    /// Layer `other` on top, keeping the origin of each of its keys.
    pub fn extend(&mut self, other: &LayeredOverrides) {
        if other.sequence_value.is_some() { self.sequence_value = other.sequence_value.clone(); }
        for (key, value) in &other.values {
            self.insert(other.origin_of(key), key.clone(), value.clone());
        }
    }

    /// Set one key with `origin`, replacing any earlier value.
    pub fn insert(&mut self, origin: OverrideOrigin, key: String, value: Value) {
        match origin {
            OverrideOrigin::Authored => self.data_keys.remove(&key),
            OverrideOrigin::Data => self.data_keys.insert(key.clone()),
        };
        self.values.insert(key, value);
    }

    /// Initialize standalone sequence data when no lexical root was supplied.
    pub fn initialize_outputs(&mut self) {
        if self.sequence_value.is_none() {
            self.sequence_value = Some(crate::composition::lifecycle::bindings::SequenceBinding::standalone(Vec::new()).to_value());
        }
    }

    /// Runtime sequence data never enters the frontmatter override map.
    pub fn with_sequence_value(mut self, value: Option<Value>) -> Self {
        self.sequence_value = value;
        self
    }

    /// The read-only sequence value carried alongside the override layers.
    pub fn sequence_value(&self) -> Option<&Value> {
        self.sequence_value.as_ref()
    }

    /// Where `key`'s value came from. An absent key reads as authored.
    pub fn origin_of(&self, key: &str) -> OverrideOrigin {
        if self.data_keys.contains(key) {
            OverrideOrigin::Data
        } else {
            OverrideOrigin::Authored
        }
    }

    /// The effective values, whatever their origin.
    pub fn values(&self) -> &Map<String, Value> {
        &self.values
    }

    /// The effective values as one JSON object.
    pub fn to_value(&self) -> Value {
        Value::Object(self.values.clone())
    }

    /// The keys whose values are data.
    pub fn data_keys(&self) -> &BTreeSet<String> {
        &self.data_keys
    }

    /// Hand these overrides to Darkmatter, authored keys as templates and data
    /// keys as inert values.
    pub fn apply_to(&self, options: ComposeOptions) -> ComposeOptions {
        let mut authored = Map::new();
        let mut data = Map::new();
        for (key, value) in &self.values {
            match self.origin_of(key) {
                OverrideOrigin::Authored => authored.insert(key.clone(), value.clone()),
                OverrideOrigin::Data => data.insert(key.clone(), value.clone()),
            };
        }
        options.with_override_layers([
            OverrideLayer::authored(Value::Object(authored)),
            OverrideLayer::data(Value::Object(data)),
        ])
    }
}

/// Fold authored setters, runtime mutations, and reserved runtime bindings.
/// Later frontmatter layers win; sequence data stays outside that map. A
/// reserved overlay replaces the complete position while outputs come from
/// the same runtime snapshot as the mutations.
pub fn layered_set_overrides(
    base: LayeredOverrides,
    runtime: Option<&RuntimeSnapshot>,
    reserved_overlay: Option<&Value>,
) -> LayeredOverrides {
    let mut merged = base;
    if let Some(runtime) = runtime {
        for (key, value) in &runtime.mutations {
            merged.insert(OverrideOrigin::Data, key.clone(), value.clone());
        }
    }
    let mut seq = reserved_overlay.and_then(|overlay| overlay.get("seq")).cloned()
        .or_else(|| merged.sequence_value.clone()).unwrap_or_else(||
            runtime.map_or_else(|| crate::composition::lifecycle::bindings::SequenceBinding::standalone(Vec::new()).to_value(), RuntimeSnapshot::sequence_value));
    if let Some(Value::Object(overlay)) = reserved_overlay {
        for (key, value) in overlay {
            if key != "seq" { merged.insert(OverrideOrigin::Data, key.clone(), value.clone()); }
        }
    }
    seq.as_object_mut().expect("sequence bindings are objects").insert("outputs".into(),
        runtime.map_or_else(|| Value::Array(Vec::new()), RuntimeSnapshot::outputs_value));
    merged.sequence_value = Some(seq);
    merged
}

#[cfg(test)]
mod tests;

/// Restores a lexical handoff chain without discarding runtime mutations.
pub(crate) struct ContainerProxyGuard<'a> {
    runtime: &'a RuntimeState,
    before: usize,
}

impl Drop for ContainerProxyGuard<'_> {
    fn drop(&mut self) {
        self.runtime.inner.lock().expect(POISONED).proxy_chain.truncate(self.before);
    }
}
