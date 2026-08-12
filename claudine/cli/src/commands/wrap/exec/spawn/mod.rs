//! Provider child-process spawning, split by execution mode.
//!
//! - [`setup`] — shared command build, process-group isolation, stdin-seed
//!   write, and wall-clock ticker contracts.
//! - [`inherited`] — [`run_child`]: inherited stdio with optional noise filtering.
//! - [`captured`] — [`run_child_capture`]: stdout/stderr captured into strings
//!   with the per-run volume cap.
//! - [`semantic`] — [`run_child_stream_semantic`]: structured semantic-stream
//!   parsing with live rendering, watchdog, and signal collection.
//!
//! Only the stable setup and wait contracts are shared; each mode owns its own
//! pipe/thread/parser behavior.

mod captured;
mod inherited;
mod semantic;
mod setup;

pub(crate) use captured::{CapturedChildOutput, run_child_capture};
pub(crate) use inherited::run_child;
pub(crate) use semantic::run_child_stream_semantic;

#[cfg(test)]
mod tests;
