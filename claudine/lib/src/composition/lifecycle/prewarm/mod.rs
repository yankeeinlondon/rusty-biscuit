//! Background preparation ("pre-warm") of lifecycle speech.
//!
//! Lifecycle `say`/`speak` phrases that can be resolved at preparation time
//! are handed to [`biscuit_speaks::Speak::prepare`] so the later event finds a
//! warm audio cache. [`collect`] finds the phrases in a parsed lifecycle,
//! [`classify`] decides which are known now and resolves them, and [`key`]
//! identifies one preparation. [`worker`] owns the one background worker an
//! invocation prepares on. Every preparation goes through the [`Preparer`]
//! seam, so tests observe calls with a fake rather than a real speech
//! provider.
//!
//! Every preparation site calls one entry point: [`collect_and_submit`] for a
//! document's lifecycle, [`submit_sequence_preflight`] for everything a
//! sequence's static graph reaches, or [`submit_task_stacks`] for a sequence
//! task's own `setup:`/`teardown:` once the task's context is established. One [`PrewarmWorker`] belongs to each
//! top-level command and travels to those sites as a [`PrewarmHandle`] on the
//! invocation context.

mod classify;
mod collect;
#[cfg(feature = "test-fixtures")]
mod fixture;
mod key;
mod preparer;
mod sequence;
mod worker;

pub use classify::PrewarmInputs;
pub(crate) use classify::{Classification, DocumentContext, classify};
pub(crate) use collect::{Phrase, lifecycle_phrases, stack_phrases};
pub(crate) use key::PrewarmKey;
pub(crate) use preparer::{Preparer, SpeakPreparer};
#[cfg(test)]
pub(crate) use preparer::{PrepareFuture, PrewarmError};
pub use sequence::{submit_sequence_preflight, submit_task_stacks};
pub use worker::{PrewarmHandle, PrewarmWorker};

use super::LifecycleConfig;
use crate::events::GlobalSettings;

/// Submit every early `say`/`speak` phrase of one prepared document.
///
/// The one entry point a document's bootstrap, stabilized reread, loop
/// refresh, retry/resume, and `proxy` target preparation call; a future
/// `prep` directive calls it too. `settings` are the invocation's bridged
/// user settings, which a document's lifecycle also plays speech with: every
/// candidate is prepared with their configuration, and the first submission
/// that carries phrases decides the gates from them. `inputs` is only built when the
/// handle is live and the lifecycle speaks, so a silent document costs
/// nothing.
///
/// Returns how many new candidates were queued; a candidate already queued in
/// this invocation is not queued again.
pub fn collect_and_submit(
    handle: &PrewarmHandle,
    settings: &GlobalSettings,
    lifecycle: &LifecycleConfig,
    inputs: impl FnOnce() -> PrewarmInputs,
) -> usize {
    if !handle.is_enabled() {
        return 0;
    }
    let phrases = lifecycle_phrases(lifecycle);
    if phrases.is_empty() {
        return 0;
    }
    handle.submit(settings, settings, phrases, &inputs())
}

#[cfg(test)]
pub(crate) mod fakes;

#[cfg(test)]
mod tests;
