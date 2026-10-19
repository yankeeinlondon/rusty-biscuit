//! The canonical Sequence Plus reserved-key catalog.
//!
//! A sequence step object carries three kinds of keys at once: the authored
//! executable (`prompt`/`shell`/…), the executable's options (`with`,
//! `timeout`, …), and arbitrary authored *state*. Every layer that inspects a
//! step — normalization (authored-state collisions), user setters/`with`, and
//! the runtime `set` side effect — must agree on which names are off-limits as
//! state. That agreement lives here so the three call sites cannot drift.
//!
//! See `claudine/features/2026-07-11-sequence-plus/spec.md` → *Reserved step
//! keys* and *Execution Architecture* for the ratified contract.

use super::model::ExecutableField;

/// The six mutually-exclusive executable fields. Exactly one may appear on a
/// task; a step may carry zero (it runs the document body) or one.
pub const EXECUTABLE_KEYS: &[&str] = &["prompt", "shell", "side_effect", "group", "task", "sequence"];

/// Optional task fields that live alongside an executable and are consumed as
/// task configuration, never as authored state.
///
/// `with` carries a prompt task's document inputs and `when` is every task's
/// guard; a step using either name gets the option, never step data. `params`
/// is not an option, so a step's `params:` is ordinary data (`seq.params`).
/// `loop` is an inline nested sequence's loop;
/// [`super::grammar::validate_task_grammar`] rejects it on every other task
/// before options are partitioned.
pub const TASK_OPTION_KEYS: &[&str] = &[
    "name", "with", "timeout", "operation", "flow", "when", "loop",
    "initialize", "start", "success", "failure", "blocked", "cancelled", "finalize",
];

/// Fields generated onto every `step_state`. Authors may declare them in a
/// `$schema` with `generated`, but may never provide or override a value.
pub const GENERATED_STATE_KEYS: &[&str] =
    &["id", "sequence_id", "is_first", "is_last", "index", "count", "run_id", "previous", "next", "outputs"];

/// Read-only lexical roots; explicit `fm.*` remains authored frontmatter.
pub const ROOT_OVERLAY_KEYS: &[&str] = &["seq", "group", "loop"];

/// The overlay keys whose object values render their `name` field in inline
/// string context (`{{seq}}` → the state's name), while whole-value spans and
/// dotted paths keep the typed object. Passed to Darkmatter compose as the
/// name-coercion key set (spec → *Referencing Sequence States*).
pub const NAME_COERCION_KEYS: &[&str] = &["seq", "seq.previous", "seq.next"];

/// Whether `key` is one of the six executable fields.
pub fn is_executable_key(key: &str) -> bool {
    EXECUTABLE_KEYS.contains(&key)
}

/// Whether `key` is a task-option field.
pub fn is_task_option_key(key: &str) -> bool {
    TASK_OPTION_KEYS.contains(&key)
}

/// Whether `key` is forbidden as an *authored state* key: a generated state
/// field or a root overlay key. Executable and task-option keys are excluded
/// here — they are extracted as the task, not rejected — so this is the exact
/// predicate for "an arbitrary state key collides with a reserved name".
pub fn is_reserved_state_key(key: &str) -> bool {
    GENERATED_STATE_KEYS.contains(&key) || ROOT_OVERLAY_KEYS.contains(&key)
}

/// The task-option keys that are meaningful for a given executable. A task
/// declaring an option outside this set is a typed authoring error rather than
/// a silently-ignored field (spec → *Task Resolution and Lifecycle Semantics*).
///
/// Lifecycle events and `loop` apply only to inline sequence containers; every
/// task may carry a `when` guard. External task
/// references may carry a leg name; their target still supplies the executable
/// and its options.
pub fn allowed_task_options(field: ExecutableField) -> &'static [&'static str] {
    match field {
        ExecutableField::Prompt => &["name", "with", "operation", "flow", "when"],
        ExecutableField::Shell => &["name", "timeout", "when"],
        ExecutableField::SideEffect => &["name", "when"],
        ExecutableField::Group => &["name", "when"],
        ExecutableField::Task => &["name", "when"],
        ExecutableField::Sequence => &["name", "when", "loop", "initialize", "start", "success", "failure", "blocked", "cancelled", "finalize"],
    }
}
