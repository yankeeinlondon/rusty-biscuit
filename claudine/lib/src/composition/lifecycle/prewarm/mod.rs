//! Background preparation ("pre-warm") of lifecycle speech.
//!
//! Lifecycle `say`/`speak` phrases that can be resolved at preparation time
//! are handed to [`biscuit_speaks::Speak::prepare`] so the later event finds a
//! warm audio cache. Every preparation goes through the [`Preparer`] seam, so
//! tests observe calls with a fake rather than a real speech provider.

#![allow(
    dead_code,
    unused_imports,
    reason = "the Preparer seam lands ahead of the worker that drives it"
)]

mod preparer;

pub(crate) use preparer::{PrepareFuture, Preparer, PrewarmError, SpeakPreparer};

#[cfg(test)]
pub(crate) mod fakes;

#[cfg(test)]
mod tests;
