//! Dynamic shell completion for Claudine's composition commands.
//!
//! This module owns the early `CompleteEnv` hook that intercepts completion
//! subprocesses before any of the normal CLI startup paths run. It is wired
//! into [`crate::main`] immediately after `color_eyre::install()` and before
//! argv normalization, so completion requests never touch config loading,
//! telemetry initialization, or the wrapper launch pipeline.
//!
//! The completion surface targets the shared `args: Vec<String>` positional
//! on `compose`, `inline-compose`, and `sequence`. Token classification,
//! candidate discovery, and validation land in sibling modules during
//! Phases 2 and 3 of the `2026-04-17-file-completion` feature. Phase 1 only
//! scaffolds the entry path: wiring up `CompleteEnv`, attaching mode-tagged
//! `ArgValueCompleter`s, and preserving the existing lenient wrapper parse
//! semantics for completion.

pub(crate) mod bootstrap;
pub(crate) mod command_factory;
pub(crate) mod file_reference;
pub(crate) mod supplement;
pub(crate) mod validate;

use clap_complete::CompleteEnv;

/// Intercept completion subprocesses and exit before normal CLI startup.
///
/// When `COMPLETE` is set in the environment, `clap_complete` emits either
/// a shell registration snippet or a completion reply and then exits via
/// [`CompleteEnv::complete`]. When `COMPLETE` is absent, this call is a
/// no-op and control returns to the regular CLI flow.
///
/// ## Notes
///
/// - Must be called before `argv::normalize(...)` so the normalizer's
///   `COMPLETE` guard is never exercised on the happy completion path.
/// - Must be called before any stdout output — clap_complete reserves the
///   completion stdout channel.
pub(crate) fn maybe_complete() {
    CompleteEnv::with_factory(command_factory::completion_command).complete();
}
