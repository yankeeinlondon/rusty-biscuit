//! Canonical reserved names used by composition layers.

/// The host-declared global that carries the loop values inside a loop.
pub(crate) const LOOP_ROOT: &str = "loop";

/// The retired loop names and the `loop` member each became.
///
/// A read of one of these names is removed grammar, never a frontmatter
/// lookup; see [`super::lifecycle::bindings::removed_loop_read`].
pub(crate) const RETIRED_LOOP_NAMES: [(&str, &str); 5] = [
    ("_loop_count", "count"),
    ("_loop_is_first", "is_first"),
    ("_loop_is_last", "is_last"),
    ("_loop_last_output", "last_output"),
    ("_loop_last_exit_code", "last_exit_code"),
];

/// Frontmatter properties that loop set actions cannot mutate.
pub(crate) const SET_ACTION_BLOCKLIST: &[&str] = &["seq", "group", "loop", "replace"];
