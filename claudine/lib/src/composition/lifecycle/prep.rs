//! The returning call a lifecycle `prep` makes: the runner seam and the
//! active call chain.
//!
//! `prep` runs another document, group, or sequence to completion and then
//! continues the caller's event with the next item. The library owns the
//! call's contract (evaluation, cycle and hop limits, the error the caller's
//! event reports); the CLI owns running the target, which it installs per
//! thread with [`install`]. A thread with no runner refuses the call.
//!
//! The runner and the active chain are thread-scoped, like cancellation: a
//! concurrent group's worker enters [`inherit`]'s snapshot, so each branch
//! keeps its own chain and concurrent calls never consume one another's hop
//! allowance.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use indexmap::IndexMap;
use serde_json::Value;

use super::super::error::CompositionError;
use super::control::{MAX_PROXY_HOPS, proxy_handoff_allowed};
use crate::diagnostics::DiagnosticSnapshot;

/// One evaluated `prep` call, as the runner receives it.
#[derive(Debug, Clone, Copy)]
pub struct PrepCall<'a> {
    /// The evaluated target reference, as the directive resolved it.
    pub target: &'a str,
    /// The evaluated `with:` overlay: data, never templates.
    pub overlay: &'a IndexMap<String, Value>,
    /// The document that authored the directive; the target resolves
    /// relative to it.
    pub caller: &'a Path,
    /// The caller's captured file-resolution context.
    pub file_resolution_context: &'a biscuit_file::FileResolutionContext,
    /// The authored path of the directive, e.g. `initialize[0].prep`.
    pub property: &'a str,
}

impl PrepCall<'_> {
    /// The overlay as the target's caller values: every string a template
    /// could read is a stored literal token, so the target reads it as data
    /// and never evaluates a produced `{{ … }}` or `$( … )` again.
    pub fn overlay_as_data(&self) -> Value {
        Value::Object(
            self.overlay
                .iter()
                .map(|(key, value)| (key.clone(), crate::composition::closure::persisted_data(value)))
                .collect(),
        )
    }
}

/// Why a target did not return successfully.
#[derive(Debug, Clone, PartialEq)]
pub struct PrepFailure {
    /// What the target reported: its error, or the non-zero status it ended
    /// with.
    pub message: String,
    /// The failure's own diagnostic identity, when its error chain carried one.
    pub snapshot: Option<Box<DiagnosticSnapshot>>,
}

/// Runs a `prep` target as an invocation of its own and reports how it ended.
///
/// The target gets a fresh runtime layer and output accumulator and none of
/// the caller's loop, sequence, or group roots; its effects on disk are the
/// only thing the caller observes afterwards. A target that skipped from its
/// `initialize`, or ended its own root loop with a `break` code of `0`,
/// returned successfully.
pub trait PrepRunner: Send + Sync {
    /// Resolve and prepare `call`'s target, run it to completion, and return.
    ///
    /// ## Errors
    ///
    /// The target's failure, a non-zero exit status (including a non-zero
    /// root `break` code and an interrupted run), or a target that could not
    /// be resolved, prepared, or approved.
    fn run(&self, call: &PrepCall<'_>) -> Result<(), PrepFailure>;
}

thread_local! {
    static RUNNER: RefCell<Option<Arc<dyn PrepRunner>>> = const { RefCell::new(None) };
    static CHAIN: RefCell<Vec<PathBuf>> = const { RefCell::new(Vec::new()) };
}

/// The runner this thread's `prep` calls use, if one is installed.
pub fn current() -> Option<Arc<dyn PrepRunner>> {
    RUNNER.with(|runner| runner.borrow().clone())
}

/// Restores the thread's previous runner and chain when dropped.
#[must_use = "the runner is installed only while the scope lives"]
pub struct PrepScope {
    runner: Option<Arc<dyn PrepRunner>>,
    chain: Vec<PathBuf>,
}

impl Drop for PrepScope {
    fn drop(&mut self) {
        RUNNER.with(|runner| *runner.borrow_mut() = self.runner.take());
        CHAIN.with(|chain| *chain.borrow_mut() = std::mem::take(&mut self.chain));
    }
}

/// Install `runner` for this thread until the returned scope drops.
pub fn install(runner: Arc<dyn PrepRunner>) -> PrepScope {
    let previous = RUNNER.with(|slot| slot.borrow_mut().replace(runner));
    PrepScope { runner: previous, chain: active_chain() }
}

/// What a worker thread inherits from the thread that spawned it.
#[derive(Clone, Default)]
pub struct PrepInheritance {
    runner: Option<Arc<dyn PrepRunner>>,
    chain: Vec<PathBuf>,
}

impl PrepInheritance {
    /// Enter the inherited runner and a copy of the inherited chain on this
    /// thread.
    pub fn enter(&self) -> PrepScope {
        let previous_runner = RUNNER.with(|slot| std::mem::replace(&mut *slot.borrow_mut(), self.runner.clone()));
        let previous_chain = CHAIN.with(|slot| std::mem::replace(&mut *slot.borrow_mut(), self.chain.clone()));
        PrepScope { runner: previous_runner, chain: previous_chain }
    }
}

/// Snapshot this thread's runner and active chain for a worker thread.
pub fn inherit() -> PrepInheritance {
    PrepInheritance { runner: current(), chain: active_chain() }
}

/// The documents with an active `prep` call on this thread, outermost first.
pub fn active_chain() -> Vec<PathBuf> {
    CHAIN.with(|chain| chain.borrow().clone())
}

/// Leaves the call it entered when dropped, so calling the same target again
/// later is legal.
#[must_use = "the call stays active only while the frame lives"]
pub struct CallFrame {
    depth: usize,
}

impl Drop for CallFrame {
    fn drop(&mut self) {
        CHAIN.with(|chain| chain.borrow_mut().truncate(self.depth));
    }
}

/// Enter a call from `caller` to the resolved `target`.
///
/// The combined active chain of calls is the cycle and hop domain: a target
/// cannot call (or, through [`proxy_reaches_an_active_caller`], hand off to)
/// a document with an active call, and the chain cannot exceed
/// [`MAX_PROXY_HOPS`].
///
/// ## Errors
///
/// [`CompositionError::LifecyclePrepCycle`] when `target` is an active caller
/// or the chain is at its limit.
pub fn enter_call(call: &PrepCall<'_>, target: &Path) -> Result<CallFrame, CompositionError> {
    CHAIN.with(|chain| {
        let mut chain = chain.borrow_mut();
        let depth = chain.len();
        let caller = biscuit_file::canonicalize_simplified(call.caller).unwrap_or_else(|_| call.caller.to_path_buf());
        if chain.last() != Some(&caller) {
            chain.push(caller);
        }
        if !proxy_handoff_allowed(&chain, target) {
            let error = CompositionError::LifecyclePrepCycle {
                source_path: call.caller.to_path_buf(),
                property: call.property.to_string(),
                target: call.target.to_string(),
                chain: chain.iter().map(|path| biscuit_file::to_portable_string(path)).collect(),
                limit: MAX_PROXY_HOPS,
            };
            chain.truncate(depth);
            return Err(error);
        }
        chain.push(target.to_path_buf());
        Ok(CallFrame { depth })
    })
}

/// Whether a `proxy` to `target` would hand off to a document with an active
/// `prep` call on this thread: the call chain and the handoff chain are one
/// cycle domain.
pub fn proxy_reaches_an_active_caller(target: &Path) -> bool {
    let chain = active_chain();
    !chain.is_empty() && !proxy_handoff_allowed(&chain, target)
}

#[cfg(test)]
mod tests;
