//! Invocation cancellation shared by a primitive and its admitted descendants.

use std::cell::{Cell, RefCell};
use std::sync::{Arc, Mutex};
use super::context::LifecycleErrorInfo;

/// The first observed reason a primitive must stop.
#[derive(Debug, Clone, PartialEq)]
pub enum CancellationCause {
    /// A user interrupt applies to every running primitive.
    Interrupt,
    /// An atomic group failed; only opted-in siblings are selected.
    SiblingFailure {
        /// Originating group.
        group: String,
        /// First failing leg.
        leg: String,
        /// Original failure, including its typed diagnostic.
        error: LifecycleErrorInfo,
    },
}

impl CancellationCause {
    /// Diagnostic exposed to cancelled cleanup as `err`.
    pub fn error(&self) -> LifecycleErrorInfo {
        match self {
            Self::Interrupt => LifecycleErrorInfo::from_action_failure("interrupted", "user interrupted the run"),
            Self::SiblingFailure { group, leg, error } => {
                let mut info = error.clone();
                info.msg = format!("group `{group}` leg `{leg}` failed: {}", error.msg);
                info.snapshot = crate::diagnostics::DiagnosticSnapshot::from_code("composition.cancelled", info.msg.clone())
                    .map(|mut snapshot| {
                        snapshot.detail = serde_json::json!({"group":group, "leg":leg, "cause":error.to_value()});
                        Box::new(snapshot)
                    });
                info
            }
        }
    }
}

/// A child observes its parent's cancellation without cancelling its siblings.
#[derive(Debug, Clone, Default)]
pub struct CancellationToken {
    cause: Arc<Mutex<Option<CancellationCause>>>,
    parent: Option<Box<CancellationToken>>,
}

impl CancellationToken {
    /// Make a new lexical child of this cancellation scope.
    pub fn child(&self) -> Self {
        Self { cause: Arc::default(), parent: Some(Box::new(self.clone())) }
    }

    /// Record only the first cancellation delivered to this primitive.
    pub fn cancel(&self, cause: CancellationCause) {
        if self.cause().is_some() { return; }
        let mut recorded = self.cause.lock().expect("cancellation cause poisoned");
        if recorded.is_none() { *recorded = Some(cause); }
    }

    /// The local cause, or the enclosing primitive's cause.
    pub fn cause(&self) -> Option<CancellationCause> {
        self.cause.lock().expect("cancellation cause poisoned").clone()
            .or_else(|| self.parent.as_ref().and_then(|parent| parent.cause()))
    }

    /// Bind process adapters on this worker to its lexical cancellation scope.
    pub fn enter(&self) -> CancellationScope {
        CancellationScope(ACTIVE.with(|active| active.replace(Some(self.clone()))))
    }
}

thread_local! {
    static CLEANUP: Cell<usize> = const { Cell::new(0) };
    static ACTIVE: RefCell<Option<CancellationToken>> = const { RefCell::new(None) };
}

/// Restore the caller's scope, including on an early return.
pub struct CancellationScope(Option<CancellationToken>);

impl Drop for CancellationScope {
    fn drop(&mut self) { ACTIVE.with(|active| { active.replace(self.0.take()); }); }
}

/// Current worker scope, inherited explicitly when admitting concurrent legs.
pub fn current() -> CancellationToken {
    ACTIVE.with(|active| active.borrow().clone().unwrap_or_default())
}

/// Cancellation requested for this worker; user interrupts remain process-wide.
pub fn cause() -> Option<CancellationCause> {
    ACTIVE.with(|active| active.borrow().as_ref().and_then(CancellationToken::cause))
        .or_else(|| crate::interrupt::interrupted().then_some(CancellationCause::Interrupt))
}

/// Whether running child work must settle before cleanup.
pub fn requested() -> bool { !CLEANUP.with(|depth| depth.get() > 0) && cause().is_some() }

/// Cleanup remains runnable after its child cancellation signal is latched.
/// Its own deadlines and a repeat user interrupt still apply.
pub struct CleanupScope;

impl CleanupScope {
    /// Suspend the cooperative stop check for this worker's terminal events.
    pub fn enter() -> Self {
        CLEANUP.with(|depth| depth.set(depth.get() + 1));
        Self
    }
}

impl Drop for CleanupScope {
    fn drop(&mut self) { CLEANUP.with(|depth| depth.set(depth.get() - 1)); }
}
