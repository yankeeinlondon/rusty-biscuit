//! The opt-in result trace of a whole-value expression: whether the value it
//! returned is, unchanged, the value of one `load_data` call, and if so which
//! source that call read.
//!
//! A destination schema failure on such a value can then name the data file
//! position it came from beside the authored expression. Only a direct load
//! qualifies: the result of the call itself, through parentheses, or through
//! the arm a ternary selected. Every other construct (an operator, a member
//! or index selection, a callback, an object or array literal, `try` and
//! `||`) derives a new value, so its result has the expression location
//! alone.
//!
//! The trace follows evaluation itself, never value equality: each evaluated
//! node either forwards the origin its last evaluated child left (a
//! parenthesized expression, a ternary, whose last child is the selected
//! arm) or replaces it (a `load_data` call sets it, every other node clears
//! it). Nothing is recorded unless a caller opened a trace with [`traced`],
//! so ordinary evaluation pays one thread-local read per node.

use std::cell::RefCell;
use std::sync::Arc;

use biscuit_file::DataFormat;

use super::data::{DataSourceIdentity, TabularSource};

/// The source a direct `load_data` result was read from: enough to place a
/// value path of the result in the original input.
#[derive(Debug, Clone)]
pub struct LoadOrigin {
    /// The resolved data source.
    pub identity: DataSourceIdentity,
    /// The format the source was parsed as.
    pub format: DataFormat,
    /// The BOM-stripped source text, shared with the data memo.
    pub text: Arc<str>,
    /// For CSV and TSV, the table the rows were built from, with each
    /// object row's column keys.
    pub table: Option<TabularSource>,
    /// Whether the parsed root was a scalar wrapped in a one-element array.
    pub wrapped: bool,
}

thread_local! {
    /// `None` when no trace is open; otherwise the origin of the value the
    /// node evaluated last.
    static TRACE: RefCell<Option<Option<Arc<LoadOrigin>>>> = const { RefCell::new(None) };
}

/// Runs `evaluate` with a trace open and returns its result with the origin
/// of the value it produced, when that value is a direct load. A trace
/// already open on this thread is restored afterward.
pub(crate) fn traced<T>(evaluate: impl FnOnce() -> T) -> (T, Option<Arc<LoadOrigin>>) {
    struct Restore(Option<Option<Arc<LoadOrigin>>>);
    impl Drop for Restore {
        fn drop(&mut self) {
            let previous = self.0.take();
            TRACE.with(|trace| *trace.borrow_mut() = previous);
        }
    }
    let outer = TRACE.with(|trace| trace.borrow_mut().replace(None));
    let restore = Restore(outer);
    let result = evaluate();
    let origin = TRACE.with(|trace| trace.borrow_mut().as_mut().and_then(Option::take));
    drop(restore);
    (result, origin)
}

/// Whether a trace is open on this thread.
pub(crate) fn is_active() -> bool {
    TRACE.with(|trace| trace.borrow().is_some())
}

/// Records that the value just produced is `origin`'s load, when a trace is
/// open; `origin` runs only then.
pub(crate) fn record_load(origin: impl FnOnce() -> LoadOrigin) {
    TRACE.with(|trace| {
        if let Some(slot) = trace.borrow_mut().as_mut() {
            *slot = Some(Arc::new(origin()));
        }
    });
}

/// Records that the value just produced is derived, when a trace is open.
pub(crate) fn derive() {
    TRACE.with(|trace| {
        if let Some(slot) = trace.borrow_mut().as_mut() {
            *slot = None;
        }
    });
}
