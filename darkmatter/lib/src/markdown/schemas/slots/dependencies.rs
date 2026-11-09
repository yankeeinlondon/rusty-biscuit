//! What a resolution's evaluations consumed besides the inputs they observed:
//! every read a slot's gate allowed, with the file that defined the slot; the
//! context inputs each evaluated slot can read; every call to a
//! captured-context function with the digest of its answer; and whether any
//! volatile function ran.
//!
//! The summary is recorded while slots evaluate, never derived from the
//! template's first-pass slot list, so it covers every slot that actually
//! ran: a later root-union round, the slots of a `load_data` contract that
//! names a template (at any depth), and a contract served from the data memo
//! (which replays the summary its preparation recorded). Collection is
//! thread-local, like the observation capture: a slot and every nested
//! resolution it causes evaluate on the thread that started the outer one.
//!
//! An allowed read keeps the file that defined the reading slot, at whatever
//! depth it ran: a nested contract's read is judged for another command
//! against the contract's own file, never the outer slot's, because only
//! that file decides whether the automatic grant applies.
//!
//! A captured call is re-checked by calling the function again with the same
//! arguments, in the same slot context, against the requesting command's own
//! captured observations, and comparing the answer's digest: a new captured
//! day, address set, package lookup, or agentic-CLI scan, or an executable
//! that appeared or disappeared on the same `PATH`, is a miss; an unchanged
//! answer (the same day for `is_today`) is a hit. Only digests of answers are
//! kept, never their text.

use std::cell::RefCell;

use biscuit_file::FileResolutionContext;
use serde_json::Value;

use super::cache::{ContextCheck, GrantedRead};
use super::observations::SessionObservations;
use crate::markdown::compose::context::capture::{CapturedObservations, function_group};
use crate::markdown::compose::expression::functions::SlotClass;
use crate::markdown::compose::expression::{ExpressionError, ResolutionContext};

/// One call to a [`SlotClass::Captured`] function a slot made, with what
/// re-checking it needs.
#[derive(Debug, Clone)]
pub(crate) struct CapturedCall {
    function: &'static str,
    args: Vec<Value>,
    /// The slot context the call ran in. A cache entry is only looked up
    /// under the same request-context identity, so this context resolves
    /// alike for the command re-checking it.
    context: FileResolutionContext,
    answer: [u8; 32],
}

impl CapturedCall {
    /// Whether the call still answers the same with `observations`, the
    /// re-checking command's captured observations.
    pub(crate) fn holds(&self, observations: &CapturedObservations) -> bool {
        let mut context = ResolutionContext::new(self.context.clone());
        context.observations = observations.clone();
        crate::markdown::compose::expression::functions::call_function(self.function, &self.args, Some(&context))
            .is_some_and(|answer| answer_digest(&answer) == self.answer)
    }
}

fn answer_digest(answer: &Result<Value, ExpressionError>) -> [u8; 32] {
    let text = match answer {
        Ok(value) => format!("ok\0{}", biscuit_hash::canonical_json(value)),
        Err(error) => format!("err\0{error}"),
    };
    biscuit_hash::blake3_hash_bytes(text.as_bytes())
}

/// What a resolution's evaluations consumed beyond their observations; see
/// the module documentation.
#[derive(Debug, Clone, Default)]
pub(crate) struct SlotDependencies {
    /// Every read a slot's gate allowed, each once; a session serving the
    /// result to another command must decide each one allowed itself.
    pub(crate) reads: Vec<GrantedRead>,
    pub(crate) context: Vec<ContextCheck>,
    pub(crate) captured: Vec<CapturedCall>,
    /// A volatile function ran: the result is never served to another
    /// command.
    pub(crate) volatile: bool,
}

impl SlotDependencies {
    fn merge(&mut self, other: &Self) {
        for read in &other.reads {
            self.add_read(read.clone());
        }
        for check in &other.context {
            self.add_context(check.clone());
        }
        self.captured.extend(other.captured.iter().cloned());
        self.volatile |= other.volatile;
    }

    fn add_read(&mut self, read: GrantedRead) {
        if !self.reads.contains(&read) {
            self.reads.push(read);
        }
    }

    fn add_context(&mut self, check: ContextCheck) {
        if !self.context.contains(&check) {
            self.context.push(check);
        }
    }

    /// Whether every captured call still answers the same with the
    /// re-checking command's `observations`, captured for the groups the
    /// calls read first.
    pub(crate) fn captured_hold(&self, observations: &SessionObservations) -> bool {
        if self.captured.is_empty() {
            return true;
        }
        let observations = observations.covering(self.captured.iter().filter_map(|call| function_group(call.function)));
        self.captured.iter().all(|call| call.holds(&observations))
    }
}

thread_local! {
    static FRAMES: RefCell<Vec<SlotDependencies>> = const { RefCell::new(Vec::new()) };
}

/// Runs `run`, returning its result with everything its evaluations
/// consumed on this thread. Nested collections also report to every
/// enclosing one.
pub(crate) fn collect<T>(run: impl FnOnce() -> T) -> (T, SlotDependencies) {
    struct Pop;
    impl Drop for Pop {
        fn drop(&mut self) {
            FRAMES.with_borrow_mut(Vec::pop);
        }
    }
    FRAMES.with_borrow_mut(|frames| frames.push(SlotDependencies::default()));
    let pop = Pop;
    let result = run();
    let collected = FRAMES.with_borrow_mut(|frames| frames.last_mut().map(std::mem::take)).unwrap_or_default();
    drop(pop);
    (result, collected)
}

fn with_frames(note: impl Fn(&mut SlotDependencies)) {
    FRAMES.with_borrow_mut(|frames| frames.iter_mut().for_each(note));
}

/// Reports `dependencies` (a memoized or cached result's) to every
/// collection running on this thread, as if their evaluations ran again.
pub(crate) fn replay(dependencies: &SlotDependencies) {
    with_frames(|frame| frame.merge(dependencies));
}

/// Records a read a slot's gate allowed.
pub(crate) fn note_read(read: GrantedRead) {
    with_frames(|frame| frame.add_read(read.clone()));
}

/// Records a context input an evaluated slot can read.
pub(crate) fn note_context(check: ContextCheck) {
    with_frames(|frame| frame.add_context(check.clone()));
}

/// Records a call to the built-in `function` of slot class `class`, made in
/// a slot's evaluation (`context` has a slot gate), with its `answer`.
/// Outside a collection, and for every other class, nothing is recorded:
/// pure functions depend on their arguments alone, and input-reading ones
/// record what they read as observations.
pub(crate) fn note_call(
    class: Option<SlotClass>,
    function: &'static str,
    args: &[Value],
    context: Option<&ResolutionContext>,
    answer: &Result<Value, ExpressionError>,
) {
    let Some(context) = context.filter(|context| context.slot_gate.is_some()) else {
        return;
    };
    if FRAMES.with_borrow(Vec::is_empty) {
        return;
    }
    match class {
        Some(SlotClass::Volatile) => with_frames(|frame| frame.volatile = true),
        Some(SlotClass::Captured) => {
            let call = CapturedCall {
                function,
                args: args.to_vec(),
                context: context.file_resolution_context.clone(),
                answer: answer_digest(answer),
            };
            with_frames(|frame| frame.captured.push(call.clone()));
        }
        _ => {}
    }
}

