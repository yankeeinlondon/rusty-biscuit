//! Uniform read access to a component's active validation error.
//!
//! All components that implement submit-time validation store the
//! error message as `Option<String>` on their state struct. This trait
//! exposes that storage through a single read accessor so that
//! containers (notably [`InputTable`](crate::components)) can query
//! child cells without knowing their concrete type.

/// Read accessor for a component's active validation error.
///
/// ## Notes
///
/// Validation semantics (the two-tier rejection/suppression model)
/// live on each component's `handle_event` implementation; this trait
/// is purely about exposing the *current* error value.
pub trait ValidationState {
    /// Returns the current validation error, if any.
    ///
    /// Returns `Some(&str)` when the component has an active
    /// submit-time validation failure (e.g. a `required` `ChooseOne`
    /// whose user submitted without a selection). Returns `None`
    /// otherwise.
    fn validation_error(&self) -> Option<&str>;

    /// Clears any active validation error.
    ///
    /// Components call this internally when the user performs an
    /// action that resolves the error (e.g. making a selection on a
    /// `required` `ChooseOne`).
    fn clear_validation_error(&mut self);
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture {
        error: Option<String>,
    }

    impl ValidationState for Fixture {
        fn validation_error(&self) -> Option<&str> {
            self.error.as_deref()
        }

        fn clear_validation_error(&mut self) {
            self.error = None;
        }
    }

    #[test]
    fn validation_error_returns_none_when_clean() {
        let fixture = Fixture { error: None };
        assert!(fixture.validation_error().is_none());
    }

    #[test]
    fn validation_error_returns_the_stored_message() {
        let fixture = Fixture {
            error: Some("selection required".into()),
        };
        assert_eq!(fixture.validation_error(), Some("selection required"));
    }

    #[test]
    fn clear_validation_error_empties_the_slot() {
        let mut fixture = Fixture {
            error: Some("bad".into()),
        };
        fixture.clear_validation_error();
        assert!(fixture.validation_error().is_none());
    }
}
