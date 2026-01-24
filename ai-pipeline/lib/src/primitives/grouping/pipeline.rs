//! Pipeline composition for serial execution of heterogeneous steps.
//!
//! This module provides `Pipeline`, which allows composing multiple steps
//! with different output types into a single executable unit.

use std::collections::HashSet;

use crate::primitives::runnable::Runnable;
use crate::primitives::state::{PipelineState, StateKey};

/// A type-erased runnable for heterogeneous pipeline composition.
///
/// This trait allows storing different `Runnable` implementations
/// in the same collection, enabling pipelines with steps that have
/// different output types.
pub trait DynRunnable: Send + Sync {
    /// Executes this step, optionally storing the result in state.
    fn execute_dyn(&self, state: &mut PipelineState);

    /// Returns the name of this step.
    fn name(&self) -> &str;

    /// Returns the state keys this step reads from.
    fn declares_reads(&self) -> &[&'static str];

    /// Returns the state keys this step writes to.
    fn declares_writes(&self) -> &[&'static str];
}

/// Wrapper that erases the output type and optionally stores results in state.
struct StepWrapper<R: Runnable> {
    runnable: R,
    output_key: Option<StateKey<R::Output>>,
}

impl<R: Runnable> DynRunnable for StepWrapper<R>
where
    R::Output: Clone,
{
    fn execute_dyn(&self, state: &mut PipelineState) {
        match self.runnable.execute(state) {
            Ok(output) => {
                if let Some(key) = self.output_key {
                    state.set(key, output);
                }
            }
            Err(error) => {
                state.add_error(error);
            }
        }
    }

    fn name(&self) -> &str {
        self.runnable.name()
    }

    fn declares_reads(&self) -> &[&'static str] {
        self.runnable.declares_reads()
    }

    fn declares_writes(&self) -> &[&'static str] {
        self.runnable.declares_writes()
    }
}

/// A pipeline that executes heterogeneous steps serially.
///
/// Steps are executed in order, with each step having mutable access
/// to the shared pipeline state. Execution stops if a fatal error is
/// encountered.
///
/// ## Example
///
/// ```ignore
/// use ai_pipeline::primitives::grouping::Pipeline;
/// use ai_pipeline::primitives::state::{PipelineState, StateKey};
///
/// const RESULT: StateKey<String> = StateKey::new("result");
///
/// let pipeline = Pipeline::new()
///     .with(FirstStep)
///     .add_with_output(SecondStep, RESULT);
///
/// pipeline.validate().expect("validation failed");
///
/// let mut state = PipelineState::new();
/// pipeline.execute(&mut state);
///
/// let result = state.get(RESULT);
/// ```
pub struct Pipeline {
    steps: Vec<Box<dyn DynRunnable>>,
}

impl Default for Pipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl Pipeline {
    /// Creates a new empty pipeline.
    #[must_use]
    pub fn new() -> Self {
        Self { steps: Vec::new() }
    }

    /// Adds a step that discards its output.
    ///
    /// The step is executed but its return value is not stored in state.
    #[must_use]
    pub fn with<R>(mut self, runnable: R) -> Self
    where
        R: Runnable + 'static,
        R::Output: Clone,
    {
        self.steps.push(Box::new(StepWrapper {
            runnable,
            output_key: None,
        }));
        self
    }

    /// Adds a step and stores its output in state under the given key.
    #[must_use]
    pub fn add_with_output<R>(mut self, runnable: R, output_key: StateKey<R::Output>) -> Self
    where
        R: Runnable + 'static,
        R::Output: Clone,
    {
        self.steps.push(Box::new(StepWrapper {
            runnable,
            output_key: Some(output_key),
        }));
        self
    }

    /// Returns the number of steps in the pipeline.
    #[must_use]
    pub fn len(&self) -> usize {
        self.steps.len()
    }

    /// Returns true if the pipeline has no steps.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// Validates that all declared reads have corresponding prior writes.
    ///
    /// This catches configuration errors before execution, ensuring that
    /// steps don't try to read state that hasn't been written.
    ///
    /// ## Errors
    ///
    /// Returns a list of validation error messages if any step reads
    /// a key that no prior step writes.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut available: HashSet<&str> = HashSet::new();
        let mut errors = Vec::new();

        for step in &self.steps {
            // Check that all reads are satisfied
            for &read_key in step.declares_reads() {
                if !available.contains(read_key) {
                    errors.push(format!(
                        "Step '{}' reads '{}' but no prior step writes it",
                        step.name(),
                        read_key
                    ));
                }
            }

            // Add writes to available set
            for &write_key in step.declares_writes() {
                available.insert(write_key);
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Validates with initial state keys that are available before execution.
    ///
    /// Use this when the pipeline will be executed with pre-populated state.
    pub fn validate_with_initial(&self, initial_keys: &[&'static str]) -> Result<(), Vec<String>> {
        let mut available: HashSet<&str> = initial_keys.iter().copied().collect();
        let mut errors = Vec::new();

        for step in &self.steps {
            for &read_key in step.declares_reads() {
                if !available.contains(read_key) {
                    errors.push(format!(
                        "Step '{}' reads '{}' but no prior step writes it",
                        step.name(),
                        read_key
                    ));
                }
            }

            for &write_key in step.declares_writes() {
                available.insert(write_key);
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Executes all steps in order, accumulating errors.
    ///
    /// Execution stops early if a fatal error is encountered.
    /// Non-fatal errors are accumulated and execution continues.
    pub fn execute(&self, state: &mut PipelineState) {
        for step in &self.steps {
            if state.has_fatal_error() {
                break;
            }
            step.execute_dyn(state);
        }
    }

    /// Executes all steps, returning the final state.
    ///
    /// This is a convenience method that creates a new state and executes.
    #[must_use]
    pub fn run(&self) -> PipelineState {
        let mut state = PipelineState::new();
        self.execute(&mut state);
        state
    }

    /// Executes with initial state, returning the final state.
    #[must_use]
    pub fn run_with(&self, state: PipelineState) -> PipelineState {
        let mut state = state;
        self.execute(&mut state);
        state
    }
}

impl std::fmt::Debug for Pipeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pipeline")
            .field("steps_count", &self.steps.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::state::StepError;

    const INTERMEDIATE: StateKey<String> = StateKey::new("intermediate");
    const OUTPUT: StateKey<String> = StateKey::new("output");

    struct ProduceStep {
        value: String,
    }

    impl Runnable for ProduceStep {
        type Output = String;

        fn execute(&self, _state: &mut PipelineState) -> Result<Self::Output, StepError> {
            Ok(self.value.clone())
        }

        fn name(&self) -> &str {
            "ProduceStep"
        }

        fn declares_writes(&self) -> &[&'static str] {
            &["intermediate"]
        }
    }

    struct TransformStep;

    impl Runnable for TransformStep {
        type Output = String;

        fn execute(&self, state: &mut PipelineState) -> Result<Self::Output, StepError> {
            Ok(state
                .get(INTERMEDIATE)
                .map(|s| s.to_uppercase())
                .unwrap_or_default())
        }

        fn name(&self) -> &str {
            "TransformStep"
        }

        fn declares_reads(&self) -> &[&'static str] {
            &["intermediate"]
        }

        fn declares_writes(&self) -> &[&'static str] {
            &["output"]
        }
    }

    struct FailingStep {
        fatal: bool,
    }

    impl Runnable for FailingStep {
        type Output = ();

        fn execute(&self, _state: &mut PipelineState) -> Result<Self::Output, StepError> {
            let mut error = StepError::new("FailingStep", "intentional failure");
            if self.fatal {
                error = error.fatal();
            }
            Err(error)
        }

        fn name(&self) -> &str {
            "FailingStep"
        }
    }

    #[test]
    fn test_empty_pipeline() {
        let pipeline = Pipeline::new();

        assert!(pipeline.is_empty());
        assert_eq!(pipeline.len(), 0);
        assert!(pipeline.validate().is_ok());
    }

    #[test]
    fn test_single_step() {
        let pipeline = Pipeline::new().add_with_output(
            ProduceStep {
                value: "hello".to_string(),
            },
            INTERMEDIATE,
        );

        assert_eq!(pipeline.len(), 1);

        let state = pipeline.run();
        assert_eq!(state.get(INTERMEDIATE), Some(&"hello".to_string()));
    }

    #[test]
    fn test_chained_steps() {
        let pipeline = Pipeline::new()
            .add_with_output(
                ProduceStep {
                    value: "hello".to_string(),
                },
                INTERMEDIATE,
            )
            .add_with_output(TransformStep, OUTPUT);

        assert_eq!(pipeline.len(), 2);
        assert!(pipeline.validate().is_ok());

        let state = pipeline.run();
        assert_eq!(state.get(INTERMEDIATE), Some(&"hello".to_string()));
        assert_eq!(state.get(OUTPUT), Some(&"HELLO".to_string()));
    }

    #[test]
    fn test_validation_fails_missing_dependency() {
        let pipeline = Pipeline::new().add_with_output(TransformStep, OUTPUT);

        let result = pipeline.validate();
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("TransformStep"));
        assert!(errors[0].contains("intermediate"));
    }

    #[test]
    fn test_validation_with_initial_keys() {
        let pipeline = Pipeline::new().add_with_output(TransformStep, OUTPUT);

        // Should fail without initial keys
        assert!(pipeline.validate().is_err());

        // Should pass with initial keys
        assert!(pipeline.validate_with_initial(&["intermediate"]).is_ok());
    }

    #[test]
    fn test_error_accumulation() {
        let pipeline = Pipeline::new()
            .with(FailingStep { fatal: false })
            .with(FailingStep { fatal: false });

        let state = pipeline.run();

        assert!(state.has_errors());
        assert!(!state.has_fatal_error());
        assert_eq!(state.errors().len(), 2);
    }

    #[test]
    fn test_fatal_error_stops_execution() {
        let pipeline = Pipeline::new()
            .with(FailingStep { fatal: true })
            .with(FailingStep { fatal: false });

        let state = pipeline.run();

        assert!(state.has_fatal_error());
        // Only the first error, second step was not executed
        assert_eq!(state.errors().len(), 1);
    }

    #[test]
    fn test_run_with_initial_state() {
        let pipeline = Pipeline::new().add_with_output(TransformStep, OUTPUT);

        let mut initial_state = PipelineState::new();
        initial_state.set(INTERMEDIATE, "world".to_string());

        let state = pipeline.run_with(initial_state);

        assert_eq!(state.get(OUTPUT), Some(&"WORLD".to_string()));
    }
}
