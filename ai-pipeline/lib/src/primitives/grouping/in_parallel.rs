//! Parallel execution of pipeline steps.
//!
//! This module provides `InParallel` for executing multiple steps concurrently
//! with read-only access to shared state.

use crate::primitives::runnable::Runnable;
use crate::primitives::state::PipelineState;

/// Executes a set of homogeneous tasks in parallel.
///
/// All tasks receive read-only access to the pipeline state and must
/// implement `execute_readonly`. Their outputs are collected into a `Vec<R::Output>`.
///
/// ## Example
///
/// ```ignore
/// use ai_pipeline::primitives::grouping::InParallel;
/// use ai_pipeline::primitives::state::PipelineState;
///
/// let parallel = InParallel::new(vec![
///     FetchDataStep { url: "https://a.com" },
///     FetchDataStep { url: "https://b.com" },
///     FetchDataStep { url: "https://c.com" },
/// ]);
///
/// let mut state = PipelineState::new();
/// let results: Vec<String> = parallel.execute(&mut state)?;
/// ```
pub struct InParallel<R>
where
    R: Runnable,
{
    pub tasks: Vec<R>,
}

impl<R> InParallel<R>
where
    R: Runnable,
{
    /// Creates a new parallel executor with the given tasks.
    #[must_use]
    pub fn new(tasks: Vec<R>) -> Self {
        Self { tasks }
    }

    /// Creates a new parallel executor from an iterator.
    pub fn collect_from(iter: impl IntoIterator<Item = R>) -> Self {
        Self {
            tasks: iter.into_iter().collect(),
        }
    }

    /// Returns the number of tasks.
    #[must_use]
    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    /// Returns true if there are no tasks.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }
}

impl<R> Runnable for InParallel<R>
where
    R: Runnable,
    R::Output: Clone,
{
    type Output = Vec<R::Output>;

    /// Executes all tasks with read-only state access, collecting their outputs.
    ///
    /// Currently executes sequentially; future versions may use rayon for true parallelism.
    fn execute(
        &self,
        state: &mut PipelineState,
    ) -> Result<Self::Output, crate::primitives::state::StepError> {
        let mut results = Vec::with_capacity(self.tasks.len());

        for task in &self.tasks {
            match task.execute_readonly(state) {
                Ok(output) => results.push(output),
                Err(error) => state.add_error(error),
            }
        }

        Ok(results)
    }

    fn execute_readonly(
        &self,
        state: &PipelineState,
    ) -> Result<Self::Output, crate::primitives::state::StepError> {
        let mut results = Vec::with_capacity(self.tasks.len());

        for task in &self.tasks {
            results.push(task.execute_readonly(state)?);
        }

        Ok(results)
    }

    fn name(&self) -> &str {
        "InParallel"
    }

    fn supports_readonly(&self) -> bool {
        // InParallel supports readonly if all its tasks do
        self.tasks.iter().all(|t| t.supports_readonly())
    }

    fn declares_reads(&self) -> &[&'static str] {
        // Aggregate reads from all tasks
        // Note: This returns empty since we can't easily aggregate static slices
        // Validation should be done on individual tasks
        &[]
    }

    fn declares_writes(&self) -> &[&'static str] {
        // Parallel tasks should not write during execution
        &[]
    }
}

/// A builder for creating parallel execution groups.
pub struct ParallelBuilder<R: Runnable> {
    tasks: Vec<R>,
}

impl<R: Runnable> Default for ParallelBuilder<R> {
    fn default() -> Self {
        Self::new()
    }
}

impl<R: Runnable> ParallelBuilder<R> {
    /// Creates a new empty parallel builder.
    #[must_use]
    pub fn new() -> Self {
        Self { tasks: Vec::new() }
    }

    /// Adds a task to the parallel group.
    #[must_use]
    pub fn with(mut self, task: R) -> Self {
        self.tasks.push(task);
        self
    }

    /// Builds the parallel executor.
    #[must_use]
    pub fn build(self) -> InParallel<R> {
        InParallel { tasks: self.tasks }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::state::StateKey;
    use serde::Serialize;

    const MULTIPLIER: StateKey<i32> = StateKey::new("multiplier");

    #[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
    struct DoubleOutput(i32);

    struct MultiplyStep {
        value: i32,
    }

    impl Runnable for MultiplyStep {
        type Output = i32;

        fn execute(
            &self,
            state: &mut PipelineState,
        ) -> Result<Self::Output, crate::primitives::state::StepError> {
            let multiplier = state.get(MULTIPLIER).copied().unwrap_or(1);
            Ok(self.value * multiplier)
        }

        fn execute_readonly(
            &self,
            state: &PipelineState,
        ) -> Result<Self::Output, crate::primitives::state::StepError> {
            let multiplier = state.get(MULTIPLIER).copied().unwrap_or(1);
            Ok(self.value * multiplier)
        }

        fn supports_readonly(&self) -> bool {
            true
        }

        fn name(&self) -> &str {
            "MultiplyStep"
        }

        fn declares_reads(&self) -> &[&'static str] {
            &["multiplier"]
        }
    }

    #[test]
    fn test_empty_parallel() {
        let parallel: InParallel<MultiplyStep> = InParallel::new(vec![]);

        assert!(parallel.is_empty());
        assert_eq!(parallel.len(), 0);

        let mut state = PipelineState::new();
        let results = parallel
            .execute(&mut state)
            .expect("parallel should succeed");
        assert!(results.is_empty());
    }

    #[test]
    fn test_parallel_execution() {
        let parallel = InParallel::new(vec![
            MultiplyStep { value: 1 },
            MultiplyStep { value: 2 },
            MultiplyStep { value: 3 },
        ]);

        assert_eq!(parallel.len(), 3);

        let mut state = PipelineState::new();
        state.set(MULTIPLIER, 10);

        let results = parallel
            .execute(&mut state)
            .expect("parallel should succeed");

        assert_eq!(results, vec![10, 20, 30]);
    }

    #[test]
    fn test_parallel_readonly() {
        let parallel = InParallel::new(vec![MultiplyStep { value: 5 }, MultiplyStep { value: 7 }]);

        let mut state = PipelineState::new();
        state.set(MULTIPLIER, 2);

        // Should work with read-only access too
        let results = parallel
            .execute_readonly(&state)
            .expect("readonly should succeed");

        assert_eq!(results, vec![10, 14]);
    }

    #[test]
    fn test_parallel_builder() {
        let parallel = ParallelBuilder::new()
            .with(MultiplyStep { value: 1 })
            .with(MultiplyStep { value: 2 })
            .build();

        assert_eq!(parallel.len(), 2);
    }

    #[test]
    fn test_supports_readonly() {
        let parallel = InParallel::new(vec![MultiplyStep { value: 1 }, MultiplyStep { value: 2 }]);

        assert!(parallel.supports_readonly());
    }

    #[test]
    fn test_from_iter() {
        let values = vec![1, 2, 3];
        let parallel =
            InParallel::collect_from(values.into_iter().map(|v| MultiplyStep { value: v }));

        assert_eq!(parallel.len(), 3);
    }
}
