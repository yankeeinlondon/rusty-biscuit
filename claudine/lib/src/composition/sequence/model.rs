//! Typed domain model for Sequence Plus.
//!
//! This module replaces the loosely-shaped "list of raw JSON blobs" with typed
//! states, sources, and the reserved per-step overlay. It is the vocabulary the
//! later phases (source resolution, preflight, JIT execution, groups) build on;
//! phase 3 wires the state/source/overlay portion and defines the remaining
//! executable/task/output/runtime representations so those phases share one
//! model rather than re-inventing shapes.
//!
//! See `claudine/features/2026-07-11-sequence-plus/spec.md`.

use std::path::PathBuf;

use serde_json::{Map, Value};

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

/// A fully-generated per-step state — the `step_state` schema type.
///
/// Every step's authored `name` (or a normalized scalar) plus the generated
/// identity/position fields and any arbitrary authored state keys (`extra`).
/// [`Self::to_value`] renders it as the JSON object injected under `state`,
/// `previous`, and `next`.
#[derive(Debug, Clone, PartialEq)]
pub struct StepState {
    /// Authored (or scalar-normalized) display name. Always a non-empty string.
    pub name: String,
    /// Collision-free identity: `name` dasherized, suffixed `-<n>` on duplicates.
    pub id: String,
    /// The owning sequence's invocation token, copied unchanged into every step.
    pub sequence_id: String,
    /// `true` for the first step in the sequence.
    pub is_first: bool,
    /// `true` for the last step in the sequence.
    pub is_last: bool,
    /// One-based position in the sequence.
    pub index: usize,
    /// Total number of steps in the sequence.
    pub count: usize,
    /// Arbitrary authored state keys (everything that is not the `name`, an
    /// executable, a task option, or a reserved key).
    pub extra: Map<String, Value>,
}

impl StepState {
    /// Render this state as the JSON object placed under `state`/`previous`/`next`.
    ///
    /// Generated fields are emitted first, then the authored `extra` keys. An
    /// `extra` key never shadows a generated field: normalization already
    /// rejected authored keys colliding with the reserved catalog, so the
    /// generated values are authoritative here.
    pub fn to_value(&self) -> Value {
        let mut map = Map::new();
        map.insert("name".into(), Value::String(self.name.clone()));
        map.insert("id".into(), Value::String(self.id.clone()));
        map.insert("sequence_id".into(), Value::String(self.sequence_id.clone()));
        map.insert("is_first".into(), Value::Bool(self.is_first));
        map.insert("is_last".into(), Value::Bool(self.is_last));
        map.insert("index".into(), Value::Number(self.index.into()));
        map.insert("count".into(), Value::Number(self.count.into()));
        for (key, value) in &self.extra {
            map.insert(key.clone(), value.clone());
        }
        Value::Object(map)
    }
}

// ---------------------------------------------------------------------------
// Executable / task vocabulary
// ---------------------------------------------------------------------------

/// The single executable field a step or task declares. A step may declare zero
/// (it runs the source document body) or one; a task declares exactly one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutableField {
    /// `prompt: <file-ref>` — compose and execute the referenced document.
    Prompt,
    /// `shell: <string | string[]>` — run one or more approved shell commands.
    Shell,
    /// `side_effect: <action>` — run one Darkmatter safe side effect.
    SideEffect,
    /// `group: <group-ref | inline group>` — execute a group.
    Group,
    /// `task: <file-ref>` — expand an externalized `kind: task` file.
    Task,
}

impl ExecutableField {
    /// The frontmatter key this variant is spelled as.
    pub fn key(self) -> &'static str {
        match self {
            ExecutableField::Prompt => "prompt",
            ExecutableField::Shell => "shell",
            ExecutableField::SideEffect => "side_effect",
            ExecutableField::Group => "group",
            ExecutableField::Task => "task",
        }
    }

    /// Resolve a frontmatter key to its executable variant, if it is one.
    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            "prompt" => Some(ExecutableField::Prompt),
            "shell" => Some(ExecutableField::Shell),
            "side_effect" => Some(ExecutableField::SideEffect),
            "group" => Some(ExecutableField::Group),
            "task" => Some(ExecutableField::Task),
            _ => None,
        }
    }
}

/// The executable half of a step or task: which field was declared, its raw
/// (unparsed) value, and the raw task options that accompanied it.
///
/// Phase 3 detects and validates exclusivity; the executable value and options
/// are parsed and executed in later phases (source resolution, task execution),
/// so they are retained verbatim here.
#[derive(Debug, Clone, PartialEq)]
pub struct StepExecutable {
    /// Which of the five executable fields was declared.
    pub field: ExecutableField,
    /// The declared executable value, verbatim.
    pub value: Value,
    /// The task-option keys/values (`params`, `timeout`, …) declared alongside.
    pub options: Map<String, Value>,
}

/// A reference to an externalized `kind: task` file (`task: <file-ref>`).
///
/// The reference is immutable at the referencing site in v1 — no field
/// patching — so it carries only the untouched file reference text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalTaskRef {
    /// The `<file-ref>` text, resolved via `FileReference` in later phases.
    pub reference: String,
}

/// How a group is named/located when a step declares `group:`.
#[derive(Debug, Clone, PartialEq)]
pub enum GroupRef {
    /// The step's `group:` value is the full inline group object.
    Inline(Map<String, Value>),
    /// A `kind: group` file, referenced with the normal file-reference grammar.
    File {
        /// The untouched `<file-ref>` text.
        reference: String,
    },
    /// A named entry in a `kind: group-catalog` file: `{name}@{file-ref}`.
    Catalog {
        /// The group name (the portion before the first `@`).
        name: String,
        /// The catalog `<file-ref>` text (the portion after the first `@`).
        reference: String,
    },
}

// ---------------------------------------------------------------------------
// Sources
// ---------------------------------------------------------------------------

/// Provenance of a resolved sequence definition — where its steps came from.
///
/// Provenance is not just bookkeeping: it selects both the normalization
/// strictness ([`Self::strictness`]) and whether a zero-step result is an
/// authoring error or a graceful no-op ([`Self::is_dynamic`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SequenceSource {
    /// Defined inline in the invoking document's frontmatter.
    Inline,
    /// A formal sequence document — a root `sequence:` list reached without an
    /// offset or operator, whether referenced or invoked directly.
    External {
        /// Absolute path to the resolved source file.
        path: PathBuf,
    },
    /// An arbitrary data file (or a sequence document reached through an
    /// offset/operator, or any JSONL/NDJSON file) used as foreign data.
    DataFile {
        /// Absolute path to the resolved source file.
        path: PathBuf,
    },
    /// A whole-value `{{ … }}` expression in the invoking frontmatter.
    Expression,
    /// A whole-value `$( … )` shell expansion in the invoking frontmatter.
    Shell,
}

impl SequenceSource {
    /// Which normalization contract this provenance earns.
    ///
    /// Lists authored *for* sequences (inline and formal sequence documents)
    /// stay strict, so a missing `name` is caught as the typo it is. Foreign
    /// data — arbitrary files, expressions, shell output — is normalized
    /// leniently because its shape was never under the sequence author's
    /// control.
    pub fn strictness(&self) -> Strictness {
        match self {
            Self::Inline | Self::External { .. } => Strictness::Strict,
            Self::DataFile { .. } | Self::Expression | Self::Shell => Strictness::Lenient,
        }
    }

    /// `true` when a zero-step result is a legitimate runtime outcome rather
    /// than an authoring mistake. A clean repository makes
    /// `{{ ctx.dirty_files }}` empty; `sequence: []` is still a typo.
    pub fn is_dynamic(&self) -> bool {
        matches!(self, Self::DataFile { .. } | Self::Expression | Self::Shell)
    }

    /// The resolved file this source was read from, when it was a file at all.
    pub fn path(&self) -> Option<&std::path::Path> {
        match self {
            Self::External { path } | Self::DataFile { path } => Some(path),
            Self::Inline | Self::Expression | Self::Shell => None,
        }
    }
}

/// How strictly an item list is normalized into step states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strictness {
    /// Objects must carry a string `name`; scalars must be strings.
    Strict,
    /// Number/boolean scalars coerce to string names, objects without a `name`
    /// receive their one-based ordinal as a name, and `null` items are still a
    /// typed error.
    Lenient,
}

// ---------------------------------------------------------------------------
// Plan
// ---------------------------------------------------------------------------

/// A single normalized step: its ordering index, display name, the authored
/// value (post-template, for provenance), and the generated [`StepState`].
#[derive(Debug, Clone, PartialEq)]
pub struct SequenceStep {
    /// Zero-based order in the sequence list (internal scheduling index; the
    /// author-facing one-based position lives in [`StepState::index`]).
    pub index: usize,
    /// Display name (mirrors [`StepState::name`]).
    pub name: String,
    /// The authored value verbatim: a JSON string for a scalar step, a JSON
    /// object for an object step (after any external `template` application).
    pub raw_state: Value,
    /// The generated per-step state injected into composition.
    pub state: StepState,
    /// The step's executable and task options, if it declares one. `None` runs
    /// the source document body (today's "headed" behavior).
    pub executable: Option<StepExecutable>,
}

/// A validated, normalized sequence plan ready for overlay construction.
#[derive(Debug, Clone, PartialEq)]
pub struct SequencePlan {
    /// Where the sequence definition came from.
    pub source: SequenceSource,
    /// Ordered, normalized steps.
    pub steps: Vec<SequenceStep>,
    /// The invoking document's `fail_fast` setting (defaults to `true`).
    pub document_fail_fast: bool,
    /// The generated invocation correlation token, copied into every step's
    /// state and the root overlay.
    pub sequence_id: String,
}

// ---------------------------------------------------------------------------
// Overlay
// ---------------------------------------------------------------------------

/// The reserved per-step overlay injected into each composition run.
///
/// Always present: `state`, `sequence_id`. Present-as-`null` on the
/// boundaries: `previous` on the first step, `next` on the last step — an
/// absent neighbor never coerces to an empty-named state. The retired root keys
/// (`previous_state`, `next_state`, `step`, `total_steps`, `is_first`,
/// `is_last`) are gone; their information moved inside each `step_state`.
#[derive(Debug, Clone, PartialEq)]
pub struct SequenceStepOverlay {
    /// The current step's state.
    pub state: StepState,
    /// The previous step's state, or `None` on the first step.
    pub previous: Option<StepState>,
    /// The next step's state, or `None` on the last step.
    pub next: Option<StepState>,
    /// The sequence invocation token (also present inside every state).
    pub sequence_id: String,
}

impl SequenceStepOverlay {
    /// Build a `serde_json::Value::Object` suitable for `set_overrides`.
    ///
    /// Merge order: user `--set` first, then the overlay (reserved keys always
    /// win). `previous`/`next` are emitted as `null` when their neighbor is
    /// absent so `{{ previous }}` reads as empty rather than an unknown
    /// variable.
    pub fn as_set_overrides(&self, user_set: Option<Value>) -> Value {
        let mut map = Map::new();

        if let Some(Value::Object(user_map)) = user_set {
            for (key, value) in user_map {
                map.insert(key, value);
            }
        }

        map.insert("state".into(), self.state.to_value());
        map.insert(
            "previous".into(),
            self.previous.as_ref().map_or(Value::Null, StepState::to_value),
        );
        map.insert(
            "next".into(),
            self.next.as_ref().map_or(Value::Null, StepState::to_value),
        );
        map.insert("sequence_id".into(), Value::String(self.sequence_id.clone()));
        // `outputs` is deliberately absent: it lives in the runtime layer,
        // which sits *below* this overlay. Emitting the (always-empty) overlay
        // copy would clobber the accumulator every step, so a later step would
        // read `[]` instead of what earlier steps produced.

        Value::Object(map)
    }
}

// ---------------------------------------------------------------------------
// Outputs and runtime mutation
// ---------------------------------------------------------------------------

/// One entry in the accumulating `outputs` array. A single task contributes a
/// string; a parallel group contributes one nested array in declaration order,
/// giving the whole array the shape `(string | string[])[]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputEntry {
    /// A single task's captured, undecorated final stdout.
    Single(String),
    /// A parallel group's per-task outputs, one per task in declaration order.
    Nested(Vec<String>),
}

impl OutputEntry {
    /// Render as the JSON value pushed onto `outputs`.
    pub fn to_value(&self) -> Value {
        match self {
            OutputEntry::Single(s) => Value::String(s.clone()),
            OutputEntry::Nested(items) => {
                Value::Array(items.iter().cloned().map(Value::String).collect())
            }
        }
    }
}

/// A single runtime state mutation performed by the `set` side effect: a
/// top-level key, its new typed value, and the prior value it replaced.
#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeMutation {
    /// The top-level state key that was written.
    pub key: String,
    /// The value written.
    pub value: Value,
    /// The value present before the write (`null` when the key was absent).
    pub prior: Value,
}
