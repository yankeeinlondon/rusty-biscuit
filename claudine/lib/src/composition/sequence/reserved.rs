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
///
/// `$schema`, `$state`, and `template` are an inline nested sequence's own
/// declarations ([`SEQUENCE_DECLARATION_KEYS`] plus its check-only `$schema`);
/// they are extracted as options so a parent never reads them as step data,
/// and [`super::declarations`] refuses them beside any other source.
pub const TASK_OPTION_KEYS: &[&str] = &[
    "name", "with", "timeout", "operation", "flow", "when", "loop",
    "initialize", "start", "success", "failure", "blocked", "cancelled", "finalize",
    "$schema", "$state", "template",
];

/// The declarations a sequence owner carries beside its inline list: the
/// step-state schema and the template defaults.
///
/// They describe the plan, never a value, so no value route may write them:
/// not a step's state, a template default, a sequence setter, a prompt task's
/// `with:`, or a runtime `set` during sequence execution
/// ([`is_declaration_write`]). Root `$schema` is not one of them: it is the
/// document schema and follows the ordinary document rules.
pub const SEQUENCE_DECLARATION_KEYS: &[&str] = &["$state", "template"];

/// Fields generated onto every `step_state`. Authors may declare them in a
/// `$state` schema with `generated`, but may never provide or override a
/// value.
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
/// field, a root overlay key, or a sequence declaration. Executable and
/// task-option keys are excluded here — they are extracted as the task, not
/// rejected — so this is the exact predicate for "an arbitrary state key
/// collides with a reserved name".
pub fn is_reserved_state_key(key: &str) -> bool {
    GENERATED_STATE_KEYS.contains(&key)
        || ROOT_OVERLAY_KEYS.contains(&key)
        || SEQUENCE_DECLARATION_KEYS.contains(&key)
}

/// The declaration a value write would replace, if any: `key` itself or the
/// root of a dotted (`template.desc`) or indexed (`$state[0]`) destination.
///
/// The one predicate behind every value route's refusal (sequence setters,
/// `with:`, runtime `set`), so a route cannot write a declaration through a
/// nested path the others refuse.
pub fn declaration_written_by(key: &str) -> Option<&'static str> {
    SEQUENCE_DECLARATION_KEYS.iter().copied().find(|declaration| {
        key.strip_prefix(declaration)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with('.') || rest.starts_with('['))
    })
}

/// Whether writing `key` would replace a sequence declaration
/// ([`declaration_written_by`]).
pub fn is_declaration_write(key: &str) -> bool {
    declaration_written_by(key).is_some()
}

/// The task-option keys that are meaningful for a given executable. A task
/// declaring an option outside this set is a typed authoring error rather than
/// a silently-ignored field (spec → *Task Resolution and Lifecycle Semantics*).
///
/// Lifecycle events, `loop`, and the declarations apply only to inline
/// sequence containers (the task grammar refuses them beside a sequence
/// reference, which shares this executable); every
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
        ExecutableField::Sequence => &[
            "name", "when", "loop", "initialize", "start", "success", "failure", "blocked", "cancelled", "finalize",
            "$schema", "$state", "template",
        ],
    }
}
