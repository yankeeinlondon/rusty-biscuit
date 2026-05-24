//! Word-wrap policy.
//!
//! The [`WordWrap`] data type lives in [`renderable::wrap_policy`] and is
//! re-exported here for backwards compatibility. The terminal-specific
//! `wrap_lines` function stays in this crate (`utils::word_wrap`).

pub use renderable::wrap_policy::WordWrap;
