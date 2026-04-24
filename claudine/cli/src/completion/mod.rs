//! Dynamic shell completion for Claudine's CLI surface.
//!
//! This module serves two distinct completion paths:
//!
//! 1. **`claudine __complete`** (bash / zsh / fish) — the hidden subcommand
//!    that generated completion scripts shell out to on every `<TAB>`.
//!    Routing flows through [`engine::run`], which classifies the cursor
//!    slot and dispatches to a slot-specific completer. Phase 1 of the
//!    `2026-04-24-improved-shell-completions` feature implements the root
//!    menu here ([`root_menu::render`]); later phases replace the
//!    composition positional and setter-value slots.
//!
//! 2. **`CompleteEnv`** (powershell / elvish) — the legacy
//!    `COMPLETE=<shell> claudine` bootstrap that activates at process
//!    startup via [`maybe_complete`]. This path is retained unchanged for
//!    shells the new engine does not target; it builds a clap-derived
//!    command tree through [`command_factory::completion_command`].
//!
//! The old (`supplement`, `command_factory`, `file_reference`,
//! `validate`) modules remain in-tree during the transition. The legacy
//! `CompleteEnv` factory still depends on `command_factory`, and
//! `engine::run` bridges non-root slots to [`supplement::run`] until the
//! dedicated composition/setter completers land in later phases. All four
//! are scheduled for removal in Phase 5.

pub(crate) mod bootstrap;
pub(crate) mod command_factory;
pub(crate) mod composition;
pub(crate) mod engine;
pub(crate) mod file_reference;
pub(crate) mod frontmatter;
pub(crate) mod fuzzy;
pub(crate) mod root_menu;
pub(crate) mod scopes;
pub(crate) mod supplement;
pub(crate) mod validate;
pub(crate) mod walker;

use clap_complete::CompleteEnv;

/// Intercept `COMPLETE=<shell> claudine` startup and exit.
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
/// - Only PowerShell and Elvish still rely on this path; bash / zsh / fish
///   scripts shell out to `claudine __complete`, which is dispatched
///   through [`engine::run`] instead.
pub(crate) fn maybe_complete() {
    CompleteEnv::with_factory(command_factory::completion_command).complete();
}
