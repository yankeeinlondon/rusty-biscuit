//! The invocation-local runtime state cell.
//!
//! One [`RuntimeState`] is created per composition *invocation* — a standalone
//! `compose`/`inline-compose` run, a `--loop` run across all of its iterations,
//! or a whole sequence across all of its steps — and is shared by every
//! lifecycle action, loop rematerialization, and (from phase 8) every sequence
//! step. It owns the two things that outlive a single provider attempt:
//!
//! - **accumulated mutations** — what the `set` side effect writes
//! - **`outputs`** — the append-only task-output accumulator
//!
//! Neither touches the process environment, the process working directory, or
//! the filesystem: `set` is the in-memory counterpart of `set_frontmatter`.
//!
//! ## Layer precedence
//!
//! A just-in-time composition folds four layers into one `set_overrides`
//! object, lowest precedence first (spec → *Execution Architecture*):
//!
//! 1. the live source document's frontmatter (Darkmatter's own base — not
//!    represented here)
//! 2. prompt/task `params` and sequence user setters
//! 3. accumulated runtime mutations
//! 4. the reserved per-step overlay (`state`/`previous`/`next`/`sequence_id`)
//!
//! [`layered_set_overrides`] is the single place that ordering is encoded.

use std::cell::RefCell;

use darkmatter::effects::{EffectEngine, EffectError};
use darkmatter::markdown::FrontmatterMap;
use serde_json::{Map, Value};

use super::sequence::reserved::ROOT_OVERLAY_KEYS;

/// The reserved frontmatter key holding the accumulating output array.
pub const OUTPUTS_KEY: &str = "outputs";

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
}

/// The shared runtime cell. See the [module docs](self).
#[derive(Debug, Default)]
pub struct RuntimeState {
    inner: RefCell<RuntimeSnapshot>,
}

impl RuntimeState {
    /// Create an empty cell: no mutations, no outputs.
    pub fn new() -> Self {
        Self::default()
    }

    /// Copy the current layers out of the cell.
    pub fn snapshot(&self) -> RuntimeSnapshot {
        self.inner.borrow().clone()
    }

    /// Apply one `set` write and return the value it replaced.
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
        if ROOT_OVERLAY_KEYS.contains(&key) {
            return Err(RuntimeMutationError::ReservedKey { key: key.to_string() });
        }
        let mut inner = self.inner.borrow_mut();
        // Darkmatter owns the mutation primitive (and the key-shape rule); this
        // layer owns only Claudine's reserved-key policy.
        let prior_mutation = engine.set(&mut inner.mutations, key, value)?;
        Ok(match prior_mutation {
            Value::Null => prior_base.get(key).cloned().unwrap_or(Value::Null),
            existing => existing,
        })
    }

    /// Commit one task's captured stdout as the next `outputs` entry.
    ///
    /// One trailing transport newline is removed; all other whitespace is
    /// preserved (spec → *Output capture boundary*).
    pub fn append_output(&self, text: &str) {
        self.append_output_entry(Value::String(trim_transport_newline(text).to_string()));
    }

    /// Commit an already-shaped entry — a string for an atomic task, an array
    /// of strings for a completed parallel group.
    pub fn append_output_entry(&self, entry: Value) {
        self.inner.borrow_mut().outputs.push(entry);
    }

    /// The committed entries as the `outputs` frontmatter value.
    pub fn outputs_value(&self) -> Value {
        Value::Array(self.inner.borrow().outputs.clone())
    }

    /// How many entries have been committed.
    pub fn output_count(&self) -> usize {
        self.inner.borrow().outputs.len()
    }

    /// The most recent entry, when it is a plain string.
    ///
    /// A completed parallel group commits an array entry; there is no single
    /// text for it, so this yields `None` rather than a flattened join.
    pub fn last_output_text(&self) -> Option<String> {
        match self.inner.borrow().outputs.last() {
            Some(Value::String(text)) => Some(text.clone()),
            _ => None,
        }
    }
}

impl RuntimeSnapshot {
    /// The committed entries as the `outputs` frontmatter value.
    pub fn outputs_value(&self) -> Value {
        Value::Array(self.outputs.clone())
    }
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

/// Fold the just-in-time composition layers into one `set_overrides` object.
///
/// Later arguments win. `outputs` is always present in the result — a document
/// composed outside any sequence still resolves `{{ last(outputs) }}` — so this
/// is the single guarantee behind the spec's "single-document `compose` /
/// `inline-compose` use the same key" rule.
pub fn layered_set_overrides(
    user_setters: Option<&Value>,
    runtime: Option<&RuntimeSnapshot>,
    reserved_overlay: Option<&Value>,
) -> Value {
    let mut merged = Map::new();
    merge_object_into(&mut merged, user_setters);
    if let Some(runtime) = runtime {
        for (key, value) in &runtime.mutations {
            merged.insert(key.clone(), value.clone());
        }
    }
    merged.insert(
        OUTPUTS_KEY.to_string(),
        runtime.map_or_else(|| Value::Array(Vec::new()), RuntimeSnapshot::outputs_value),
    );
    merge_object_into(&mut merged, reserved_overlay);
    Value::Object(merged)
}

/// Guarantee an `outputs` key on a caller-built override object without
/// disturbing any layer the caller already folded in.
///
/// Preparation applies this to every composition so library-only callers and
/// tests — which never build layers through [`layered_set_overrides`] — still
/// see an initialized accumulator rather than an undefined root.
pub fn with_initialized_outputs(overrides: Option<Value>) -> Value {
    let mut merged = Map::new();
    merge_object_into(&mut merged, overrides.as_ref());
    merged
        .entry(OUTPUTS_KEY.to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    Value::Object(merged)
}

fn merge_object_into(target: &mut Map<String, Value>, source: Option<&Value>) {
    if let Some(Value::Object(map)) = source {
        for (key, value) in map {
            target.insert(key.clone(), value.clone());
        }
    }
}

#[cfg(test)]
mod tests;
