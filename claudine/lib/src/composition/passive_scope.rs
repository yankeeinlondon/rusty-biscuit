//! The dry run's passive session, held for one thread while a sequence check
//! reads the tree.
//!
//! A sequence check ([`check_sequence_tree`](super::check_sequence_tree))
//! reaches expression evaluation through many evaluators that build their own
//! Darkmatter context: source expressions, `::template(…)` operators, formal
//! `template:` defaults, `with:` derivation, shell-site values, and owner
//! preparation. Each of them builds that context through
//! [`document_expression_resolution_context`](super::document_expression_resolution_context)
//! or the canonical preparation options, and both attach the session held
//! here, so Darkmatter's function dispatch refuses an effectful call
//! (`has_alias`, `ping`, a provider query) with
//! `ExpressionError::Unavailable` instead of starting a process.
//!
//! The scope also puts the session's observation of local files in force
//! ([`PassiveObservations::local_reads`](darkmatter::markdown::compose::passive::PassiveObservations::local_reads)),
//! so every source the walk reads (the root, a prompt, a referenced
//! sequence, task, or group file, a `$schema` file) is opened once for every
//! pass of the check, and [`read_local_source`] serves its bytes to the
//! check's other readers.
//!
//! The scope is per thread: the check walk runs on one thread and spawns
//! none. A real run never enters it.

use std::cell::RefCell;

use darkmatter::markdown::compose::passive::PassiveSession;

thread_local! {
    static ACTIVE: RefCell<Option<PassiveSession>> = const { RefCell::new(None) };
}

/// Restores the enclosing scope's session (usually none) when dropped.
#[must_use = "the scope ends when the guard is dropped"]
pub(crate) struct PassiveScope {
    previous: Option<PassiveSession>,
    _shared_reads: simplified_schema::resolve::buffers::SharedReadsScope,
}

impl Drop for PassiveScope {
    fn drop(&mut self) {
        let previous = self.previous.take();
        ACTIVE.with(|active| *active.borrow_mut() = previous);
    }
}

/// Hold `session` for this thread until the returned guard drops.
pub(crate) fn enter(session: PassiveSession) -> PassiveScope {
    let shared_reads = simplified_schema::resolve::buffers::enter_shared_reads(session.observations().local_reads().clone());
    let previous = ACTIVE.with(|active| active.borrow_mut().replace(session));
    PassiveScope { previous, _shared_reads: shared_reads }
}

/// The session this thread holds, when a check is reading.
pub(crate) fn current() -> Option<PassiveSession> {
    ACTIVE.with(|active| active.borrow().clone())
}

/// The bytes of the local file at `path`, as a check reads a source: through
/// the observation of local files in force on this thread (a dry run's,
/// shared by its passes), else with one open. A real run holds none, so it
/// reads the file as it is now.
///
/// ## Errors
///
/// The open's or the read's I/O error.
pub fn read_local_source(path: &std::path::Path) -> std::io::Result<simplified_schema::resolve::buffers::ReadSource> {
    use simplified_schema::resolve::buffers;
    match buffers::shared_reads() {
        Some(shared) => {
            let fresh = !shared.holds(path);
            let read = shared.read(path)?;
            if fresh {
                crate::note_work!(CompositionSourceRead);
            }
            Ok(read)
        }
        None => {
            crate::note_work!(CompositionSourceRead);
            buffers::open_source(path)
        }
    }
}

/// [`read_local_source`] as text.
///
/// ## Errors
///
/// The read's I/O error; [`std::io::ErrorKind::InvalidData`] when the file
/// is not UTF-8.
pub fn read_local_text(path: &std::path::Path) -> std::io::Result<String> {
    String::from_utf8(read_local_source(path)?.bytes.to_vec())
        // The message `std::fs::read_to_string` gives, which reports show.
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "stream did not contain valid UTF-8"))
}
