//! Runnable trait for pipeline steps.
//!
//! The `Runnable` trait defines the interface for pipeline steps that can read from
//! and write to shared state during execution.

use serde::Serialize;
use std::hash::Hash;

use super::state::PipelineState;

/// A step in the pipeline that can read/write state during execution.
///
/// Implementors define an `Output` type and an `execute` method that receives
/// mutable access to the pipeline state. The trait also supports optional
/// declaration of state dependencies for validation.
///
/// ## Example
///
/// ```ignore
/// use ai_pipeline::primitives::{Runnable, PipelineState, StateKey};
///
/// const INPUT: StateKey<String> = StateKey::new("input");
///
/// struct UppercaseStep;
///
/// impl Runnable for UppercaseStep {
///     type Output = String;
///
///     fn execute(&self, state: &mut PipelineState) -> Self::Output {
///         state.get(INPUT)
///             .map(|s| s.to_uppercase())
///             .unwrap_or_default()
///     }
///
///     fn declares_reads(&self) -> &[&'static str] {
///         &["input"]
///     }
/// }
/// ```
pub trait Runnable: Send + Sync {
    /// The output type produced by this step.
    type Output: Serialize + Hash + Eq + Send + Sync + 'static;

    /// Executes this step with mutable access to the pipeline state.
    ///
    /// Steps can read from and write to state during execution.
    /// The return value is the step's output, which may optionally
    /// be stored in state by the pipeline executor.
    fn execute(&self, state: &mut PipelineState) -> Self::Output;

    /// Executes this step with read-only access to the pipeline state.
    ///
    /// This is used for parallel execution where multiple steps need
    /// concurrent read access. The default implementation panics;
    /// steps that support parallel execution should override this.
    ///
    /// ## Panics
    ///
    /// The default implementation panics. Override this method to support
    /// parallel execution.
    fn execute_readonly(&self, _state: &PipelineState) -> Self::Output {
        panic!(
            "Step '{}' does not support read-only execution for parallel contexts",
            self.name()
        )
    }

    /// Returns the name of this step for error reporting and debugging.
    ///
    /// The default implementation returns the type name.
    fn name(&self) -> &str {
        std::any::type_name::<Self>()
    }

    /// Declares the state keys this step reads from.
    ///
    /// Used for pipeline validation to ensure all required state
    /// is available before a step executes.
    fn declares_reads(&self) -> &[&'static str] {
        &[]
    }

    /// Declares the state keys this step writes to.
    ///
    /// Used for pipeline validation to track what state becomes
    /// available after a step executes.
    fn declares_writes(&self) -> &[&'static str] {
        &[]
    }

    /// Returns true if this step supports read-only execution.
    ///
    /// Steps that return true can be used in parallel execution contexts
    /// via `InParallel`.
    fn supports_readonly(&self) -> bool {
        false
    }
}

/// Extension trait providing utility methods for `Runnable` types.
pub trait RunnableExt: Runnable {
    /// Wraps this runnable to store its output in state under the given key.
    fn with_output_key(self, key: super::state::StateKey<Self::Output>) -> WithOutputKey<Self>
    where
        Self: Sized,
        Self::Output: Clone,
    {
        WithOutputKey {
            inner: self,
            output_key: key,
        }
    }
}

impl<T: Runnable> RunnableExt for T {}

/// A wrapper that stores the runnable's output in state under a specific key.
pub struct WithOutputKey<R: Runnable>
where
    R::Output: Clone,
{
    inner: R,
    output_key: super::state::StateKey<R::Output>,
}

impl<R: Runnable> Runnable for WithOutputKey<R>
where
    R::Output: Clone,
{
    type Output = R::Output;

    fn execute(&self, state: &mut PipelineState) -> Self::Output {
        let output = self.inner.execute(state);
        // Store a clone in state, return the original
        state.set(self.output_key, output.clone());
        output
    }

    fn execute_readonly(&self, state: &PipelineState) -> Self::Output {
        self.inner.execute_readonly(state)
    }

    fn name(&self) -> &str {
        self.inner.name()
    }

    fn declares_reads(&self) -> &[&'static str] {
        self.inner.declares_reads()
    }

    fn declares_writes(&self) -> &[&'static str] {
        self.inner.declares_writes()
    }

    fn supports_readonly(&self) -> bool {
        self.inner.supports_readonly()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::state::StateKey;

    const INPUT: StateKey<String> = StateKey::new("input");
    const OUTPUT: StateKey<String> = StateKey::new("output");

    struct EchoStep;

    impl Runnable for EchoStep {
        type Output = String;

        fn execute(&self, state: &mut PipelineState) -> Self::Output {
            state
                .get(INPUT)
                .cloned()
                .unwrap_or_else(|| "default".to_string())
        }

        fn declares_reads(&self) -> &[&'static str] {
            &["input"]
        }

        fn declares_writes(&self) -> &[&'static str] {
            &["output"]
        }
    }

    struct ReadOnlyStep;

    impl Runnable for ReadOnlyStep {
        type Output = i32;

        fn execute(&self, _state: &mut PipelineState) -> Self::Output {
            42
        }

        fn execute_readonly(&self, _state: &PipelineState) -> Self::Output {
            42
        }

        fn supports_readonly(&self) -> bool {
            true
        }
    }

    #[test]
    fn test_execute_reads_from_state() {
        let mut state = PipelineState::new();
        state.set(INPUT, "hello".to_string());

        let step = EchoStep;
        let output = step.execute(&mut state);

        assert_eq!(output, "hello");
    }

    #[test]
    fn test_execute_with_missing_state() {
        let mut state = PipelineState::new();

        let step = EchoStep;
        let output = step.execute(&mut state);

        assert_eq!(output, "default");
    }

    #[test]
    fn test_declares_reads_writes() {
        let step = EchoStep;

        assert_eq!(step.declares_reads(), &["input"]);
        assert_eq!(step.declares_writes(), &["output"]);
    }

    #[test]
    fn test_supports_readonly() {
        let echo = EchoStep;
        let readonly = ReadOnlyStep;

        assert!(!echo.supports_readonly());
        assert!(readonly.supports_readonly());
    }

    #[test]
    fn test_execute_readonly() {
        let state = PipelineState::new();
        let step = ReadOnlyStep;

        let output = step.execute_readonly(&state);
        assert_eq!(output, 42);
    }

    #[test]
    #[should_panic(expected = "does not support read-only execution")]
    fn test_execute_readonly_panics_for_unsupported() {
        let state = PipelineState::new();
        let step = EchoStep;

        step.execute_readonly(&state);
    }

    #[test]
    fn test_name_returns_type_name() {
        let step = EchoStep;
        assert!(step.name().contains("EchoStep"));
    }
}
