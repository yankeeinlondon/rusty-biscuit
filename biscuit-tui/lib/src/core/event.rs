//! The canonical event outcome for every biscuit-tui component.
//!
//! Every input component exposes a `handle_event` method (or an
//! equivalent) that returns an [`EventOutcome`]. The variants are
//! deliberately kept to a small, stable set so that calling code can
//! compose components without reasoning about bespoke per-component
//! outcome types.

/// The set of outcomes every component reports after handling an event.
///
/// ## Variants
///
/// - [`EventOutcome::Consumed`]: the event was handled internally; the
///   caller should redraw but take no further action.
/// - [`EventOutcome::Ignored`]: the component did not handle the event;
///   the caller may route it elsewhere (e.g. to a surrounding container).
/// - [`EventOutcome::Submitted`]: the user committed a value; the caller
///   should read it from the state (`state.value()`).
/// - [`EventOutcome::Cancelled`]: the user cancelled (typically `Esc`
///   or `Ctrl-C`); the caller should tear down the component.
///
/// ## Notes
///
/// Validation failures do **not** introduce a new variant. A component
/// that receives a submit keystroke while its value is invalid returns
/// [`EventOutcome::Consumed`] and populates its internal validation
/// error (retrievable via [`crate::core::ValidationState::validation_error`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventOutcome {
    /// The component handled the event internally and requires no
    /// further action from the caller other than a redraw.
    Consumed,

    /// The component did not handle the event. The caller may route
    /// the event to a surrounding container or discard it.
    Ignored,

    /// The user committed a value. The caller should read the value
    /// from the component state (e.g. `state.value()`).
    Submitted,

    /// The user cancelled. The caller should tear down the component
    /// and not read any value.
    Cancelled,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variants_compare_equal_to_themselves() {
        assert_eq!(EventOutcome::Consumed, EventOutcome::Consumed);
        assert_eq!(EventOutcome::Ignored, EventOutcome::Ignored);
        assert_eq!(EventOutcome::Submitted, EventOutcome::Submitted);
        assert_eq!(EventOutcome::Cancelled, EventOutcome::Cancelled);
    }

    #[test]
    fn distinct_variants_are_not_equal() {
        assert_ne!(EventOutcome::Consumed, EventOutcome::Ignored);
        assert_ne!(EventOutcome::Submitted, EventOutcome::Cancelled);
        assert_ne!(EventOutcome::Consumed, EventOutcome::Submitted);
        assert_ne!(EventOutcome::Ignored, EventOutcome::Cancelled);
    }

    #[test]
    fn outcome_is_copy() {
        let outcome = EventOutcome::Submitted;
        let copy = outcome;
        assert_eq!(outcome, copy);
    }
}
