//! Provider child-process spawning, split by execution mode.
//!
//! - [`setup`] — shared command build, process-group isolation, stdin-seed
//!   write, and wall-clock ticker contracts.
//! - [`inherited`] — [`run_child`]: inherited stdio with optional noise filtering.
//! - [`captured`] — [`run_child_capture`]: stdout/stderr captured into strings
//!   with the per-run volume cap.
//! - [`semantic`] — [`run_child_stream_semantic`]: structured semantic-stream
//!   parsing with live rendering, watchdog, and signal collection.
//! - [`retained`] — the semantic mode's spawn and readiness pre-flight for a
//!   child driven by a retained-stdin control session.
//! - `descendants` (Unix) — reaps the tool processes such a child started
//!   outside its process group.
//!
//! Only the stable setup and wait contracts are shared; each mode owns its own
//! pipe/thread/parser behavior.

mod captured;
#[cfg(unix)]
mod descendants;
mod inherited;
mod retained;
mod semantic;
mod setup;

pub(crate) use captured::{CapturedChildOutput, run_child_capture};
pub(crate) use inherited::run_child;
pub(crate) use semantic::run_child_stream_semantic;

#[cfg(test)]
mod tests;
