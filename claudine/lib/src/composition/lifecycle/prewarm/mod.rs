//! Background preparation ("pre-warm") of lifecycle speech.
//!
//! Lifecycle `say`/`speak` phrases that can be resolved at preparation time
//! are handed to [`biscuit_speaks::Speak::prepare`] so the later event finds a
//! warm audio cache. [`collect`] finds the phrases in a parsed lifecycle,
//! [`classify`] decides which are known now and resolves them, and [`key`]
//! identifies one preparation. Every preparation goes through the
//! [`Preparer`] seam, so tests observe calls with a fake rather than a real
//! speech provider.

#![allow(
    dead_code,
    unused_imports,
    reason = "the collection, classification, and Preparer seams land ahead of the worker that drives them"
)]

mod classify;
mod collect;
mod key;
mod preparer;

pub(crate) use classify::{Classification, DocumentContext, PrewarmInputs, SkipReason, classify};
pub(crate) use collect::{Phrase, lifecycle_phrases, stack_phrases};
pub(crate) use key::PrewarmKey;
pub(crate) use preparer::{PrepareFuture, Preparer, PrewarmError, SpeakPreparer};

#[cfg(test)]
pub(crate) mod fakes;

#[cfg(test)]
mod tests;
