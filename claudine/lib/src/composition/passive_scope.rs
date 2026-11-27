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
}

impl Drop for PassiveScope {
    fn drop(&mut self) {
        let previous = self.previous.take();
        ACTIVE.with(|active| *active.borrow_mut() = previous);
    }
}

/// Hold `session` for this thread until the returned guard drops.
pub(crate) fn enter(session: PassiveSession) -> PassiveScope {
    let previous = ACTIVE.with(|active| active.borrow_mut().replace(session));
    PassiveScope { previous }
}

/// The session this thread holds, when a check is reading.
pub(crate) fn current() -> Option<PassiveSession> {
    ACTIVE.with(|active| active.borrow().clone())
}
