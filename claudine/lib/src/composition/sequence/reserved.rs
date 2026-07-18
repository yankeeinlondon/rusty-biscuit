//! The canonical Sequence Plus reserved-key catalog.
//!
//! A sequence step object carries three kinds of keys at once: the authored
//! executable (`prompt`/`shell`/…), the executable's options (`params`,
//! `timeout`, …), and arbitrary authored *state*. Every layer that inspects a
//! step — normalization (authored-state collisions), user setters/`params`, and
//! the runtime `set` side effect — must agree on which names are off-limits as
//! state. That agreement lives here so the three call sites cannot drift.
//!
//! See `claudine/features/2026-07-11-sequence-plus/spec.md` → *Reserved step
//! keys* and *Execution Architecture* for the ratified contract.

use super::model::ExecutableField;

/// The five mutually-exclusive executable fields. Exactly one may appear on a
/// task; a step may carry zero (it runs the document body) or one.
pub const EXECUTABLE_KEYS: &[&str] = &["prompt", "shell", "side_effect", "group", "task"];

/// Optional task fields that live alongside an executable and are consumed as
/// task configuration, never as authored state.
pub const TASK_OPTION_KEYS: &[&str] =
    &["name", "setup", "teardown", "params", "timeout", "operation", "flow"];

/// Fields generated onto every `step_state`. Authors may declare them in a
/// `$schema` with `generated`, but may never provide or override a value.
pub const GENERATED_STATE_KEYS: &[&str] =
    &["id", "sequence_id", "is_first", "is_last", "index", "count"];

/// Reserved keys at the frontmatter root (the per-step overlay). Never authored
/// as state, never a `--set`/`params`/`set`-side-effect target.
pub const ROOT_OVERLAY_KEYS: &[&str] = &["state", "previous", "next", "outputs", "sequence_id"];

/// The overlay keys whose object values render their `name` field in inline
/// string context (`{{state}}` → the state's name), while whole-value spans and
/// dotted paths keep the typed object. Passed to Darkmatter compose as the
/// name-coercion key set (spec → *Referencing Sequence States*).
pub const NAME_COERCION_KEYS: &[&str] = &["state", "previous", "next"];

/// Whether `key` is one of the five executable fields.
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
/// `name`, `setup`, and `teardown` apply to every task variant; the table below
/// lists only the variant-specific additions.
///
/// - `prompt`: `params`, `operation`, `flow` — it composes another document.
/// - `shell`: `timeout` — a per-command duration.
/// - `side_effect`: no additions.
/// - `group`: no additions (a group configures itself in its own object).
/// - `task`: no additions — an external `task:` reference is immutable at the
///   referencing site (no v1 patching/overriding).
pub fn allowed_task_options(field: ExecutableField) -> &'static [&'static str] {
    match field {
        ExecutableField::Prompt => &["name", "setup", "teardown", "params", "operation", "flow"],
        ExecutableField::Shell => &["name", "setup", "teardown", "timeout"],
        ExecutableField::SideEffect => &["name", "setup", "teardown"],
        ExecutableField::Group | ExecutableField::Task => &["name"],
    }
}
