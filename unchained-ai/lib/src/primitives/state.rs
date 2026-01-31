//! Pipeline state management with type-safe accessors.
//!
//! This module provides a heterogeneous state container (`PipelineState`) that allows
//! pipeline steps to read and write typed values using compile-time typed keys (`StateKey<T>`).
//!
//! ## Example
//!
//! ```ignore
//! use unchained_ai::primitives::state::{PipelineState, StateKey};
//!
//! const USER_NAME: StateKey<String> = StateKey::new("user_name");
//! const SCORE: StateKey<i32> = StateKey::new("score");
//!
//! let mut state = PipelineState::new();
//! state.set(USER_NAME, "Alice".to_string());
//! state.set(SCORE, 42);
//!
//! assert_eq!(state.get(USER_NAME), Some(&"Alice".to_string()));
//! assert_eq!(state.get(SCORE), Some(&42));
//! ```

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt;
use std::marker::PhantomData;

/// A type-safe key for accessing values in `PipelineState`.
///
/// The key combines a string name (for debugging/validation) with a phantom type
/// that ensures type-safe access at compile time.
///
/// ## Example
///
/// ```ignore
/// const IMAGE_ANALYSIS: StateKey<ImageDescription> = StateKey::new("image_analysis");
/// ```
#[derive(Debug)]
pub struct StateKey<T: 'static> {
    name: &'static str,
    _marker: PhantomData<T>,
}

impl<T: 'static> StateKey<T> {
    /// Creates a new state key with the given name.
    ///
    /// The name is used for debugging, error messages, and pipeline validation.
    #[must_use]
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            _marker: PhantomData,
        }
    }

    /// Returns the name of this key.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.name
    }
}

impl<T: 'static> Clone for StateKey<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: 'static> Copy for StateKey<T> {}

impl<T: 'static> PartialEq for StateKey<T> {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl<T: 'static> Eq for StateKey<T> {}

/// An error that occurred during pipeline step execution.
///
/// Errors are accumulated during pipeline execution rather than causing immediate
/// failure. Only errors marked as `fatal` will halt the pipeline.
#[derive(Debug)]
pub struct StepError {
    /// Name of the step that produced this error.
    pub step_name: String,
    /// Human-readable error message.
    pub message: String,
    /// Optional underlying error that caused this step error.
    pub source: Option<Box<dyn std::error::Error + Send + Sync>>,
    /// If true, the pipeline should stop execution after this error.
    pub fatal: bool,
}

impl StepError {
    /// Creates a new non-fatal step error.
    pub fn new(step: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            step_name: step.into(),
            message: message.into(),
            source: None,
            fatal: false,
        }
    }

    /// Marks this error as fatal, causing the pipeline to halt.
    #[must_use]
    pub fn fatal(mut self) -> Self {
        self.fatal = true;
        self
    }

    /// Attaches an underlying error as the source of this step error.
    #[must_use]
    pub fn with_source(mut self, err: impl std::error::Error + Send + Sync + 'static) -> Self {
        self.source = Some(Box::new(err));
        self
    }
}

impl fmt::Display for StepError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.step_name, self.message)?;
        if self.fatal {
            write!(f, " (fatal)")?;
        }
        Ok(())
    }
}

impl std::error::Error for StepError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_ref()
            .map(|e| e.as_ref() as &(dyn std::error::Error + 'static))
    }
}

/// A heterogeneous state container for pipeline execution.
///
/// `PipelineState` stores typed values that can be read and written by pipeline steps
/// using `StateKey<T>` for type-safe access. It also accumulates errors during execution.
///
/// ## Type Safety
///
/// Values are stored with both their key name and `TypeId`, ensuring that:
/// - Reading a key always returns the correct type
/// - Different types can share the same key name without conflict
///
/// ## Error Accumulation
///
/// Non-fatal errors are collected and execution continues. Fatal errors cause the
/// pipeline to halt after the current step completes.
#[derive(Default)]
pub struct PipelineState {
    values: HashMap<(&'static str, TypeId), Box<dyn Any + Send + Sync>>,
    errors: Vec<StepError>,
}

impl PipelineState {
    /// Creates a new empty pipeline state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets a reference to a value by its typed key.
    ///
    /// Returns `None` if the key has not been set.
    #[must_use]
    pub fn get<T: 'static>(&self, key: StateKey<T>) -> Option<&T> {
        let type_key = (key.name, TypeId::of::<T>());
        self.values.get(&type_key)?.downcast_ref()
    }

    /// Gets a mutable reference to a value by its typed key.
    ///
    /// Returns `None` if the key has not been set.
    pub fn get_mut<T: 'static>(&mut self, key: StateKey<T>) -> Option<&mut T> {
        let type_key = (key.name, TypeId::of::<T>());
        self.values.get_mut(&type_key)?.downcast_mut()
    }

    /// Sets a value for the given typed key.
    ///
    /// If the key already has a value of the same type, it is replaced and returned.
    pub fn set<T: 'static + Send + Sync>(&mut self, key: StateKey<T>, value: T) -> Option<T> {
        let type_key = (key.name, TypeId::of::<T>());
        self.values
            .insert(type_key, Box::new(value))
            .and_then(|old| old.downcast().ok().map(|b| *b))
    }

    /// Removes a value for the given typed key, returning it if it existed.
    pub fn remove<T: 'static>(&mut self, key: StateKey<T>) -> Option<T> {
        let type_key = (key.name, TypeId::of::<T>());
        self.values
            .remove(&type_key)
            .and_then(|old| old.downcast().ok().map(|b| *b))
    }

    /// Checks if a value exists for the given typed key.
    #[must_use]
    pub fn contains<T: 'static>(&self, key: StateKey<T>) -> bool {
        let type_key = (key.name, TypeId::of::<T>());
        self.values.contains_key(&type_key)
    }

    /// Returns the number of values stored in the state.
    #[must_use]
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Returns true if no values are stored.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Adds an error to the accumulated error list.
    pub fn add_error(&mut self, error: StepError) {
        self.errors.push(error);
    }

    /// Returns a slice of all accumulated errors.
    #[must_use]
    pub fn errors(&self) -> &[StepError] {
        &self.errors
    }

    /// Returns true if any fatal error has been recorded.
    #[must_use]
    pub fn has_fatal_error(&self) -> bool {
        self.errors.iter().any(|e| e.fatal)
    }

    /// Returns true if any errors (fatal or non-fatal) have been recorded.
    #[must_use]
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Clears all accumulated errors.
    pub fn clear_errors(&mut self) {
        self.errors.clear();
    }

    /// Takes ownership of all accumulated errors, leaving the list empty.
    pub fn take_errors(&mut self) -> Vec<StepError> {
        std::mem::take(&mut self.errors)
    }
}

impl fmt::Debug for PipelineState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PipelineState")
            .field("values_count", &self.values.len())
            .field("errors_count", &self.errors.len())
            .field("has_fatal_error", &self.has_fatal_error())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NAME: StateKey<String> = StateKey::new("name");
    const COUNT: StateKey<i32> = StateKey::new("count");
    const SAME_NAME_DIFFERENT_TYPE: StateKey<i64> = StateKey::new("name");

    #[test]
    fn test_basic_get_set() {
        let mut state = PipelineState::new();

        assert!(state.get(NAME).is_none());

        state.set(NAME, "Alice".to_string());
        assert_eq!(state.get(NAME), Some(&"Alice".to_string()));

        state.set(COUNT, 42);
        assert_eq!(state.get(COUNT), Some(&42));
    }

    #[test]
    fn test_same_name_different_types() {
        let mut state = PipelineState::new();

        state.set(NAME, "Hello".to_string());
        state.set(SAME_NAME_DIFFERENT_TYPE, 123i64);

        // Both should coexist without conflict
        assert_eq!(state.get(NAME), Some(&"Hello".to_string()));
        assert_eq!(state.get(SAME_NAME_DIFFERENT_TYPE), Some(&123i64));
        assert_eq!(state.len(), 2);
    }

    #[test]
    fn test_set_returns_old_value() {
        let mut state = PipelineState::new();

        let old = state.set(COUNT, 1);
        assert!(old.is_none());

        let old = state.set(COUNT, 2);
        assert_eq!(old, Some(1));

        assert_eq!(state.get(COUNT), Some(&2));
    }

    #[test]
    fn test_remove() {
        let mut state = PipelineState::new();
        state.set(NAME, "Bob".to_string());

        let removed = state.remove(NAME);
        assert_eq!(removed, Some("Bob".to_string()));
        assert!(state.get(NAME).is_none());
    }

    #[test]
    fn test_get_mut() {
        let mut state = PipelineState::new();
        state.set(COUNT, 10);

        if let Some(count) = state.get_mut(COUNT) {
            *count += 5;
        }

        assert_eq!(state.get(COUNT), Some(&15));
    }

    #[test]
    fn test_error_accumulation() {
        let mut state = PipelineState::new();

        assert!(!state.has_errors());
        assert!(!state.has_fatal_error());

        state.add_error(StepError::new("step1", "minor issue"));
        assert!(state.has_errors());
        assert!(!state.has_fatal_error());

        state.add_error(StepError::new("step2", "critical failure").fatal());
        assert!(state.has_fatal_error());

        assert_eq!(state.errors().len(), 2);
    }

    #[test]
    fn test_take_errors() {
        let mut state = PipelineState::new();
        state.add_error(StepError::new("step1", "error1"));
        state.add_error(StepError::new("step2", "error2"));

        let errors = state.take_errors();
        assert_eq!(errors.len(), 2);
        assert!(state.errors().is_empty());
    }

    #[test]
    fn test_step_error_display() {
        let err = StepError::new("my_step", "something went wrong");
        assert_eq!(err.to_string(), "[my_step] something went wrong");

        let fatal_err = StepError::new("crash", "boom").fatal();
        assert_eq!(fatal_err.to_string(), "[crash] boom (fatal)");
    }

    #[test]
    fn test_state_key_copy_clone() {
        let key1 = NAME;
        let key2 = key1; // Copy
        let key3 = key1.clone(); // Clone

        assert_eq!(key1.name(), key2.name());
        assert_eq!(key2.name(), key3.name());
    }
}
