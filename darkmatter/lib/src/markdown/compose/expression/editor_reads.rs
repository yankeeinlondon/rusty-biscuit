//! The reads an editor observation may make, and how they stop.
//!
//! The editor evaluates a document's whole-value frontmatter expressions in
//! the background (`schemas::slots::observe`). Its reads go through the
//! ordinary `load_data` and `file_exists` handlers, which take the guarded
//! paths here only when the [`ResolutionContext`] carries [`EditorReads`];
//! every other surface is unchanged.
//!
//! - A call is admitted when evaluation reaches it ([`admit_call`]), before
//!   its arguments run: one the editor does not make is
//!   [`ExpressionError::ObservationPending`] before it has any effect.
//! - A local reference is resolved candidate by candidate, and each
//!   candidate is put to the slot gate before it is probed, so a refused
//!   candidate is never stat'ed or read. An open buffer counts as present.
//! - A clean miss of admitted candidates is the ordinary recoverable
//!   not-found failure; a refusal, an I/O failure, or a file that vanished
//!   after its probe is [`ExpressionError::ObservationPending`], which no
//!   recovery turns into a value.
//! - A URL is answered only from a fresh transport-cache entry, after the
//!   gate allowed it; nothing is fetched, registered, or written.
//! - Source bytes are charged to the observation's shared work budget before
//!   they are parsed, so a file larger than what is left stops the pass.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::SystemTime;

use super::functions::ReadKind;
use super::iteration::IterationPath;
use super::read_gate::{ReadLocation, ReadTarget};
use super::{ExpressionError, ResolutionContext};
use crate::markdown::compose::remote_fetch::{CacheOnlyPending, RemoteFetchRuntime};

/// Why an editor observation has no value yet.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PendingReason {
    /// The configured `permissions.file-read` and this process's approvals
    /// do not allow a read (or a probe) the expression needs.
    ReadRefused,
    /// An allowed input could not be inspected: an I/O failure, an
    /// unavailable resolution context, or a file that vanished after it was
    /// found.
    ReadUnavailable,
    /// A URL with no fresh transport-cache entry the request may use.
    RemoteUnavailable,
    /// A binding only a composition run knows (a host or context value, a
    /// property absent from the document).
    RuntimeOnly(String),
    /// An operation the editor does not run (an effectful or volatile
    /// function, a listing, a recursive reference).
    Unsupported(String),
    /// Another property this one reads is not known yet.
    Dependency(String),
    /// Properties that read each other.
    Cycle,
    /// The observation's work allowance ran out.
    Exhausted,
    /// The observation was canceled.
    Canceled,
}

impl std::fmt::Display for PendingReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReadRefused => f.write_str("the read is not approved"),
            Self::ReadUnavailable => f.write_str("the input cannot be inspected"),
            Self::RemoteUnavailable => f.write_str("no fresh cached response may be used"),
            Self::RuntimeOnly(name) => write!(f, "`{name}` is known only when the document is composed"),
            Self::Unsupported(what) => write!(f, "{what} is not evaluated in the editor"),
            Self::Dependency(name) => write!(f, "property `{name}` is not known yet"),
            Self::Cycle => f.write_str("properties read each other"),
            Self::Exhausted => f.write_str("the editor's work allowance ran out"),
            Self::Canceled => f.write_str("the observation was canceled"),
        }
    }
}

/// The pending failure of `function` for `reason`.
pub(crate) fn pending(function: &str, reason: PendingReason) -> ExpressionError {
    ExpressionError::ObservationPending { function: function.to_string(), reason }
}

/// Admits a call to `name` that an editor observation's evaluation reached,
/// before its arguments are evaluated or it runs. Pure functions and the
/// guarded readers (`load_data`, `file_exists`) run. Any other reader (a
/// listing included) and an effectful function are
/// [`PendingReason::Unsupported`], and a captured or volatile one is
/// [`PendingReason::RuntimeOnly`]. An unknown name is left for dispatch to
/// fail. A call evaluation never reaches is never judged here, so an unused
/// branch, recovery argument, or callback body does not decide the outcome.
pub(crate) fn admit_call(name: &str) -> Result<(), ExpressionError> {
    use super::functions::{SlotClass, is_editor_guarded_reader, slot_class};
    let reason = match slot_class(name) {
        Some(SlotClass::Pure) | None => return Ok(()),
        Some(SlotClass::InputReading) if is_editor_guarded_reader(name) => return Ok(()),
        Some(SlotClass::InputReading | SlotClass::Rejected) => PendingReason::Unsupported(format!("{name}()")),
        Some(SlotClass::Captured | SlotClass::Volatile) => PendingReason::RuntimeOnly(format!("{name}()")),
    };
    Err(pending(name, reason))
}

/// What an editor observation reads through, shared by every property of
/// one pass.
#[derive(Debug)]
pub(crate) struct EditorReads {
    /// A cache-only runtime under the observation's fetch policy.
    remote: RemoteFetchRuntime,
    /// The wall-clock instant the observation judges cache freshness at.
    now: SystemTime,
    /// The earliest expiry of a cached body the pass used.
    expiry: Mutex<Option<SystemTime>>,
    cancel: Option<Arc<AtomicBool>>,
}

impl EditorReads {
    pub(crate) fn new(remote: RemoteFetchRuntime, now: SystemTime, cancel: Option<Arc<AtomicBool>>) -> Self {
        debug_assert!(remote.is_cache_only(), "an editor observation never fetches");
        Self { remote, now, expiry: Mutex::new(None), cancel }
    }

    /// The earliest expiry of any cached body the pass used.
    pub(crate) fn expiry(&self) -> Option<SystemTime> {
        *self.expiry.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(crate) fn is_canceled(&self) -> bool {
        self.cancel.as_ref().is_some_and(|cancel| cancel.load(Ordering::Acquire))
    }

    /// `Err` when the observation was canceled.
    pub(crate) fn check_cancel(&self, function: &str) -> Result<(), ExpressionError> {
        if self.is_canceled() { Err(pending(function, PendingReason::Canceled)) } else { Ok(()) }
    }

    fn note_expiry(&self, expires_at: SystemTime) {
        let mut expiry = self.expiry.lock().unwrap_or_else(PoisonError::into_inner);
        *expiry = Some(expiry.map_or(expires_at, |known| known.min(expires_at)));
    }
}

/// Resolves `file_ref` (authored as `raw`) for `function` on the editor's
/// guarded path: `Ok(Some(path))` for the first present candidate,
/// `Ok(None)` for a clean miss of admitted candidates (every probed
/// candidate is observed as missing), and a pending failure for a refused
/// candidate, a recursive reference, or a probe that could not complete.
/// A resolution error the author must fix (a reference leaving its tree, a
/// missing home) is the ordinary file-reference failure.
pub(crate) fn resolve_local(
    function: &'static str,
    raw: &str,
    file_ref: &biscuit_file::FileReference,
    ctx: &ResolutionContext,
    editor: &EditorReads,
) -> Result<Option<PathBuf>, ExpressionError> {
    use biscuit_file::{DetailedOutcome, ProbeDisposition, ResolutionFailure};
    editor.check_cancel(function)?;
    if file_ref.class().recursive {
        return Err(pending(function, PendingReason::Unsupported("a recursive (`%`) reference".to_string())));
    }
    let context = simplified_schema::file_context::document_file_context(&ctx.cwd, &ctx.file_resolution_context);
    let mut admitted: Vec<PathBuf> = Vec::new();
    let detailed = file_ref.resolve_detailed_with_presence(
        &context,
        |path: &Path| {
            admit_probe(function, path, ctx)?;
            admitted.push(path.to_path_buf());
            Ok::<(), ExpressionError>(())
        },
        |path: &Path| simplified_schema::resolve::buffers::text(path).is_some(),
    );
    let detailed = match detailed {
        Ok(detailed) => detailed,
        Err(refusal) => {
            // Every location admitted before the refused one was probed and
            // held nothing, so creating it would change the answer.
            for path in &admitted {
                ctx.data.observe_existence(path, false);
            }
            return Err(refusal);
        }
    };
    for candidate in detailed.candidates() {
        match candidate.disposition() {
            ProbeDisposition::Missing | ProbeDisposition::NonFile => {
                ctx.data.observe_existence(candidate.candidate().path(), false);
            }
            ProbeDisposition::Matched | ProbeDisposition::Io(_) | ProbeDisposition::SearchRoot => {}
        }
    }
    match detailed.outcome() {
        DetailedOutcome::Matched(path) => Ok(Some(path.clone())),
        DetailedOutcome::Failed(ResolutionFailure::NoMatch) => Ok(None),
        DetailedOutcome::Failed(ResolutionFailure::Io | ResolutionFailure::MissingContext) => {
            Err(pending(function, PendingReason::ReadUnavailable))
        }
        DetailedOutcome::Failed(_) => match detailed.into_convenience() {
            Err(error) => Err(super::functions::file_reference_error(
                function,
                raw,
                ctx,
                super::FileRefFailure::classify(&error),
                Some(error),
            )),
            Ok(found) => Ok(found),
        },
    }
}

/// Asks the slot gate about probing `path`: a lexical precheck first, which
/// refuses without touching the filesystem a path no rule or approval could
/// allow, then the gate's full (canonical) decision.
fn admit_probe(function: &str, path: &Path, ctx: &ResolutionContext) -> Result<(), ExpressionError> {
    let Some(gate) = &ctx.slot_gate else {
        return Ok(());
    };
    let target = ReadTarget {
        function: function.to_string(),
        kind: ReadKind::Probe,
        location: ReadLocation::Local(path.to_path_buf()),
        observed_hash: None,
    };
    if !gate.admits_lexically(&target) {
        return Err(pending(function, PendingReason::ReadRefused));
    }
    gate.check(&target).map_err(|_| pending(function, PendingReason::ReadRefused))
}

/// The bytes of the approved local source at `path` (an open buffer's text
/// first), charged to the pass's work budget before they are returned. A
/// read past what is left stops at one byte over and fails with the
/// budget's limit; a file that vanished or cannot be read is pending.
pub(crate) fn read_local(function: &str, path: &Path, ctx: &ResolutionContext, editor: &EditorReads) -> Result<Vec<u8>, ExpressionError> {
    editor.check_cancel(function)?;
    let budget = &ctx.iteration;
    let charge = |bytes: u64| {
        budget.reserve_bytes(bytes, &IterationPath::empty()).map_err(|error| ExpressionError::IterationLimit(Box::new(error)))
    };
    if let Some(text) = simplified_schema::resolve::buffers::text(path) {
        charge(text.len() as u64)?;
        return Ok(text.as_bytes().to_vec());
    }
    let remaining = remaining_bytes(ctx);
    let file = std::fs::File::open(path).map_err(|_| pending(function, PendingReason::ReadUnavailable))?;
    let mut bytes = Vec::new();
    file.take(remaining.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|_| pending(function, PendingReason::ReadUnavailable))?;
    charge(bytes.len() as u64)?;
    Ok(bytes)
}

/// The expansion bytes the pass may still use.
pub(crate) fn remaining_bytes(ctx: &ResolutionContext) -> u64 {
    let budget = &ctx.iteration;
    let limit = budget.limits().max_expansion_bytes().get();
    limit.saturating_sub(budget.usage().bytes)
}

/// The fresh cached body of `url` for `function`, its expiry noted and its
/// bytes charged; pending for a URL the policy refuses, a miss, an expired
/// or undecodable entry, and a body larger than what is left.
pub(crate) fn cached_text(function: &str, url: &url::Url, ctx: &ResolutionContext, editor: &EditorReads) -> Result<String, ExpressionError> {
    editor.check_cancel(function)?;
    let remaining = remaining_bytes(ctx);
    match editor.remote.cached_text_bounded(url, editor.now, remaining) {
        Ok(fresh) => {
            ctx.iteration
                .reserve_bytes(fresh.body.len() as u64, &IterationPath::empty())
                .map_err(|error| ExpressionError::IterationLimit(Box::new(error)))?;
            editor.note_expiry(fresh.expires_at);
            Ok(fresh.body)
        }
        // Reserving one byte past what is left fails with the budget's own
        // exhaustion, as an oversized local file does.
        Err(CacheOnlyPending::TooLarge) => Err(ctx
            .iteration
            .reserve_bytes(remaining.saturating_add(1), &IterationPath::empty())
            .map_or_else(|error| ExpressionError::IterationLimit(Box::new(error)), |()| pending(function, PendingReason::Exhausted))),
        Err(other) => {
            tracing::debug!(url = %url, why = %other.describe(), "editor observation: no usable cached body");
            Err(pending(function, PendingReason::RemoteUnavailable))
        }
    }
}

/// The most distinct values kept per `load_data` call occurrence; a call
/// that returns more is marked as overflowing, and its type stays
/// provisional.
pub(crate) const CALL_VALUE_ALTERNATIVES: usize = 8;

/// The values one evaluated `load_data` node returned during a
/// [`collecting_calls`] scope.
#[derive(Debug, Clone, Default)]
pub(crate) struct CallValues {
    /// Distinct values of its contracted or headed reads, in first-seen
    /// order, at most [`CALL_VALUE_ALTERNATIVES`]. A headerless
    /// uncontracted read's value is never kept: its type is `unknown[][]`
    /// whatever it loaded.
    pub(crate) values: Vec<serde_json::Value>,
    /// More distinct values were returned than were kept.
    pub(crate) overflow: bool,
    /// At least one execution bound no contract and read CSV or TSV with
    /// `header: false`, judged from the bound arguments
    /// ([`note_read_mode`]).
    pub(crate) headerless_uncontracted: bool,
}

thread_local! {
    /// The per-node call facts of the running [`collecting_calls`] scope,
    /// keyed by the evaluated node's address.
    static CALLS: std::cell::RefCell<Option<Vec<(usize, CallValues)>>> = const { std::cell::RefCell::new(None) };
    /// Whether the `load_data` call that just returned was a headerless
    /// uncontracted read: set by the handler as it returns, taken by the
    /// evaluator for that same node ([`take_read_mode`]).
    static HEADERLESS: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Records, from a successful `load_data` handler while a
/// [`collecting_calls`] scope runs, whether its bound arguments made it a
/// headerless uncontracted read. Called last, after every argument (and any
/// nested call in one) was evaluated, so the evaluator's next
/// [`take_read_mode`] belongs to this call.
pub(crate) fn note_read_mode(headerless_uncontracted: bool) {
    HEADERLESS.set(headerless_uncontracted);
}

/// Takes the read mode [`note_read_mode`] left, resetting it; `false` when
/// the call returned before binding a read (a `null` file) or failed.
pub(crate) fn take_read_mode() -> bool {
    HEADERLESS.replace(false)
}

/// Runs `evaluate` collecting what each `load_data` node it evaluates
/// returns. The node identities are meaningful only while the evaluated
/// expression is alive; the caller turns them into ordinals at once. The
/// scope ends even when `evaluate` panics.
pub(crate) fn collecting_calls<T>(evaluate: impl FnOnce() -> T) -> (T, Vec<(usize, CallValues)>) {
    struct End(Option<Vec<(usize, CallValues)>>);
    impl Drop for End {
        fn drop(&mut self) {
            CALLS.with_borrow_mut(|calls| *calls = self.0.take());
        }
    }
    let outer = CALLS.with_borrow_mut(|calls| calls.replace(Vec::new()));
    let end = End(outer);
    let value = evaluate();
    let collected = CALLS.with_borrow_mut(|calls| calls.take()).unwrap_or_default();
    drop(end);
    (value, collected)
}

/// Notes that the `load_data` node `node` returned `value`, headerless and
/// uncontracted or not, when a [`collecting_calls`] scope is running.
pub(crate) fn note_call(node: &simplified_schema::expression::ast::Expr, value: &serde_json::Value, headerless_uncontracted: bool) {
    CALLS.with_borrow_mut(|calls| {
        let Some(calls) = calls.as_mut() else {
            return;
        };
        let address = std::ptr::from_ref(node).addr();
        let index = match calls.iter().position(|(known, _)| *known == address) {
            Some(index) => index,
            None => {
                calls.push((address, CallValues::default()));
                calls.len() - 1
            }
        };
        let facts = &mut calls[index].1;
        if headerless_uncontracted {
            facts.headerless_uncontracted = true;
            return;
        }
        if facts.values.contains(value) {
            return;
        }
        if facts.values.len() < CALL_VALUE_ALTERNATIVES {
            facts.values.push(value.clone());
        } else {
            facts.overflow = true;
        }
    });
}

/// Whether a [`collecting_calls`] scope is running on this thread.
pub(crate) fn is_collecting_calls() -> bool {
    CALLS.with_borrow(Option::is_some)
}
