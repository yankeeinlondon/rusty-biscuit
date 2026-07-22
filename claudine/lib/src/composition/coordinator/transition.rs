//! The one transition type every producer returns and the coordinator
//! consumes.

use crate::composition::CompositionError;
use crate::composition::lifecycle::LifecycleSignal;

use super::handoff::EvaluatedProxyRequest;

/// An abort that preserves its concrete cause.
///
/// Generic over the error type so a CLI-side producer can abort with a
/// `HarnessError`, a `color_eyre::Report`, or its own enum without either
/// erasing it into a string or forcing it into [`CompositionError`] — and
/// without `claudine` growing a dependency on `claudine-cli`.
#[derive(Debug, Clone, PartialEq)]
pub struct TransitionAbort<E> {
    signal: Option<LifecycleSignal>,
    source: E,
}

impl<E> TransitionAbort<E> {
    /// `signal` is the lifecycle event the abort arose from, or `None` when it
    /// arose outside any event (e.g. during target bootstrap).
    pub fn new(signal: Option<LifecycleSignal>, source: E) -> Self {
        Self { signal, source }
    }

    #[allow(missing_docs)]
    pub fn signal(&self) -> Option<LifecycleSignal> {
        self.signal
    }

    /// The concrete error, still typed.
    pub fn source(&self) -> &E {
        &self.source
    }

    /// Take the concrete error, discarding the signal.
    pub fn into_source(self) -> E {
        self.source
    }

    /// Re-type the carried error, preserving the signal.
    ///
    /// This is how a CLI driver lifts a library abort into its own error type
    /// without flattening it: pass a real `From`-style conversion, not a
    /// `to_string`.
    pub fn map_source<F, T>(self, f: F) -> TransitionAbort<T>
    where
        F: FnOnce(E) -> T,
    {
        TransitionAbort {
            signal: self.signal,
            source: f(self.source),
        }
    }
}

/// What should happen to the active document next.
///
/// Every proxy producer returns this, and the coordinator is the only thing
/// that consumes it. There is deliberately no second, optional channel for a
/// proxy target: a [`Proxy`][Self::Proxy] that no coordinator consumes is a
/// compile-time-visible unhandled variant, not a silently dropped
/// `Option<PathBuf>` return value.
#[derive(Debug, Clone, PartialEq)]
pub enum DocumentTransition<E = CompositionError> {
    /// Nothing to do; the current attempt proceeds.
    Continue,
    /// Run the active document again in a fresh provider attempt and session.
    Retry,
    /// Continue the live provider session with a follow-up message.
    Resume {
        /// The live session to continue.
        session: String,
        /// The evaluated follow-up, when the control authored one.
        message: Option<String>,
    },
    /// Hand the run off to another document.
    Proxy(EvaluatedProxyRequest),
    /// The active document finished; the run is over.
    Complete,
    /// The run cannot continue.
    Abort(TransitionAbort<E>),
}

impl<E> DocumentTransition<E> {
    /// Whether this transition ends the source document's ownership of the
    /// run, which is what makes a later synthetic source `finalize` wrong.
    pub fn hands_off_source(&self) -> bool {
        matches!(self, Self::Proxy(_) | Self::Complete | Self::Abort(_))
    }

    /// Re-type the abort error, leaving every other variant untouched.
    pub fn map_abort<F, T>(self, f: F) -> DocumentTransition<T>
    where
        F: FnOnce(E) -> T,
    {
        match self {
            Self::Continue => DocumentTransition::Continue,
            Self::Retry => DocumentTransition::Retry,
            Self::Resume { session, message } => DocumentTransition::Resume { session, message },
            Self::Proxy(request) => DocumentTransition::Proxy(request),
            Self::Complete => DocumentTransition::Complete,
            Self::Abort(abort) => DocumentTransition::Abort(abort.map_source(f)),
        }
    }
}
