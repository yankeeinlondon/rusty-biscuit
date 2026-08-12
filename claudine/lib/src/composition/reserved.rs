//! Canonical reserved names used by composition layers.

/// Ambient variables injected by the loop engine.
pub(crate) const AMBIENT_VARIABLE_NAMES: &[&str] = &[
    "_loop_count",
    "_loop_is_first",
    "_loop_is_last",
    "_loop_last_output",
    "_loop_last_exit_code",
];

/// Expression roots that cannot be interpreted as frontmatter properties.
pub(crate) const EXPRESSION_RESERVED_ROOTS: &[&str] = &["true", "false", "doc", "env"];

/// Frontmatter properties that loop set actions cannot mutate.
pub(crate) const SET_ACTION_BLOCKLIST: &[&str] = &["loop", "replace"];

/// Runtime-only lifecycle roots resolved after document preparation.
pub const LATE_BINDING_ROOTS: &[&str] = &["err", "timing", "current"];
