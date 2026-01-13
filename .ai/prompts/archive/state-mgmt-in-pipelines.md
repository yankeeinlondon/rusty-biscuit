Problem Statement

The current `Runnable<R>` trait in the **AI Pipeline** package is stateless - each operation executes independently and returns a result. To build composable pipelines where steps can read/write shared state, we need a state management pattern that balances:

 1. **Type safety:** Catch type mismatches at compile time where possible
 2. **Runtime flexibility:** Allow heterogeneous pipeline composition
 3. **Error accumulation:** Continue processing when non-fatal errors occur

## Proposed Design

1. State Container with Typed Keys

    ```rust
    use std::any::{Any, TypeId};
    use std::collections::HashMap;
    use std::marker::PhantomData;

    /// Type-safe key for accessing state values
    pub struct StateKey<T: 'static> {
        name: &'static str,
        _marker: PhantomData<T>,
    }

    impl<T: 'static> StateKey<T> {
        pub const fn new(name: &'static str) -> Self {
            Self { name, _marker: PhantomData }
        }
    }

    /// Pipeline state container - holds heterogeneous typed values
    pub struct PipelineState {
        values: HashMap<(&'static str, TypeId), Box<dyn Any + Send + Sync>>,
        errors: Vec<StepError>,
    }

    impl PipelineState {
        pub fn new() -> Self {
            Self { values: HashMap::new(), errors: Vec::new() }
        }

        /// Get a value by typed key (type-safe)
        pub fn get<T: 'static>(&self, key: StateKey<T>) -> Option<&T> {
            let type_key = (key.name, TypeId::of::<T>());
            self.values.get(&type_key)?.downcast_ref()
        }

        /// Set a value by typed key (type-safe)
        pub fn set<T: 'static + Send + Sync>(&mut self, key: StateKey<T>, value: T) {
            let type_key = (key.name, TypeId::of::<T>());
            self.values.insert(type_key, Box::new(value));
        }

        /// Check if a key exists
        pub fn contains<T: 'static>(&self, key: StateKey<T>) -> bool {
            let type_key = (key.name, TypeId::of::<T>());
            self.values.contains_key(&type_key)
        }

        /// Add an error (continues execution unless fatal)
        pub fn add_error(&mut self, error: StepError) {
            self.errors.push(error);
        }

        pub fn errors(&self) -> &[StepError] {
            &self.errors
        }

        pub fn has_fatal_error(&self) -> bool {
            self.errors.iter().any(|e| e.fatal)
        }
    }
    ```

2. Error Accumulation

      ```rust
      /// Error from a pipeline step
      pub struct StepError {
        pub step_name: String,
        pub message: String,
        pub source: Option<Box<dyn std::error::Error + Send + Sync>>,
        pub fatal: bool,  // If true, pipeline should stop
      }

      impl StepError {
        pub fn new(step: impl Into<String>, message: impl Into<String>) -> Self {
            Self {
                step_name: step.into(),
                message: message.into(),
                source: None,
                fatal: false,
            }
        }

        pub fn fatal(mut self) -> Self {
            self.fatal = true;
            self
        }

        pub fn with_source(mut self, err: impl std::error::Error + Send + Sync +
      'static) -> Self {
            self.source = Some(Box::new(err));
            self
        }
      }
      ```

3. Updated Runnable Trait

      ```rust
       /// A step in the pipeline that can read/write state
       pub trait Runnable: Send + Sync {
           /// The output type of this step
           type Output: Serialize + Hash + Eq + Send + Sync + 'static;

           /// Execute this step with access to pipeline state
           fn execute(&self, state: &mut PipelineState) -> Self::Output;

           /// Optional: Name for error reporting
           fn name(&self) -> &str {
               std::any::type_name::<Self>()
           }

           /// Optional: Declare what state keys this step reads (for validation)
           fn declares_reads(&self) -> &[&'static str] {
               &[]
           }

           /// Optional: Declare what state keys this step writes (for validation)
           fn declares_writes(&self) -> &[&'static str] {
               &[]
           }
       }
       ```

4. Type-Erased Wrapper for Heterogeneous Pipelines

      ```rust
      /// Type-erased runnable for heterogeneous pipeline composition
      trait DynRunnable: Send + Sync {
        fn execute_dyn(&self, state: &mut PipelineState);
        fn name(&self) -> &str;
        fn declares_reads(&self) -> &[&'static str];
        fn declares_writes(&self) -> &[&'static str];
      }

      /// Wrapper that erases the output type and optionally stores result in state
      struct StepWrapper<R: Runnable> {
           runnable: R,
           output_key: Option<StateKey<R::Output>>,
      }

      impl<R: Runnable> DynRunnable for StepWrapper<R>
      where
           R::Output: Send + Sync + 'static,
      {
           fn execute_dyn(&self, state: &mut PipelineState) {
               let output = self.runnable.execute(state);
               if let Some(key) = self.output_key {
                   state.set(key, output);
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
      ```

5. Pipeline Executor

      ```rust
       pub struct Pipeline {
           steps: Vec<Box<dyn DynRunnable>>,
       }

       impl Pipeline {
           pub fn new() -> Self {
               Self { steps: Vec::new() }
           }

           /// Add a step that discards its output
           pub fn add<R: Runnable + 'static>(mut self, runnable: R) -> Self {
               self.steps.push(Box::new(StepWrapper {
                   runnable,
                   output_key: None,
               }));
               self
           }

           /// Add a step and store its output in state under the given key
           pub fn add_with_output<R: Runnable + 'static>(
               mut self,
               runnable: R,
               output_key: StateKey<R::Output>,
           ) -> Self {
               self.steps.push(Box::new(StepWrapper {
                   runnable,
                   output_key: Some(output_key),
               }));
               self
           }

           /// Validate that all declared reads have corresponding writes
           pub fn validate(&self) -> Result<(), Vec<String>> {
               let mut available: HashSet<&str> = HashSet::new();
               let mut errors = Vec::new();

               for step in &self.steps {
                   // Check reads are satisfied
                   for &read_key in step.declares_reads() {
                       if !available.contains(read_key) {
                           errors.push(format!(
                               "Step '{}' reads '{}' but no prior step writes it",
                               step.name(), read_key
                           ));
                       }
                   }
                   // Add writes to available
                   for &write_key in step.declares_writes() {
                       available.insert(write_key);
                   }
               }

               if errors.is_empty() { Ok(()) } else { Err(errors) }
           }

           /// Execute all steps, accumulating errors
           pub fn execute(&self, state: &mut PipelineState) {
               for step in &self.steps {
                   if state.has_fatal_error() {
                       break;
                   }
                   step.execute_dyn(state);
               }
           }
      }
      ```


 ### Usage Example

 // Define state keys
 const PROMPT_RESULT: StateKey<String> = StateKey::new("prompt_result");
 const IMAGE_ANALYSIS: StateKey<ImageDescription> =
 StateKey::new("image_analysis");
 const FINAL_SUMMARY: StateKey<String> = StateKey::new("final_summary");

 // Create a prompt that writes to state
 struct AnalyzeImagePrompt {
     image: PromptImage,
 }

 impl Runnable for AnalyzeImagePrompt {
     type Output = ImageDescription;

     fn execute(&self, state: &mut PipelineState) -> Self::Output {
         // Call LLM, return structured output
         ImageDescription { /* ... */ }
     }

     fn declares_writes(&self) -> &[&'static str] {
         &["image_analysis"]
     }
 }

 // Create a prompt that reads from state
 struct SummarizePrompt;

```rust
impl Runnable for SummarizePrompt {
  type Output = String;

  fn execute(&self, state: &mut PipelineState) -> Self::Output {
      let analysis = state.get(IMAGE_ANALYSIS)
          .expect("image_analysis should exist");

      // Use analysis to generate summary
      format!("Summary of: {:?}", analysis)
  }

  fn declares_reads(&self) -> &[&'static str] {
      &["image_analysis"]
  }

  fn declares_writes(&self) -> &[&'static str] {
        &["final_summary"]
    }
  }

  // Build and execute pipeline
  fn main() {
    let pipeline = Pipeline::new()
        .add_with_output(
            AnalyzeImagePrompt { image: /*...*/ },
            IMAGE_ANALYSIS,
        )
        .add_with_output(
            SummarizePrompt,
            FINAL_SUMMARY,
        );

    // Validate before execution
    pipeline.validate().expect("Pipeline validation failed");

    let mut state = PipelineState::new();
    pipeline.execute(&mut state);

    // Check for errors
    if !state.errors().is_empty() {
        for err in state.errors() {
            eprintln!("Error in {}: {}", err.step_name, err.message);
        }
    }

    // Access results
    if let Some(summary) = state.get(FINAL_SUMMARY) {
        println!("Result: {}", summary);
    }
}
```

## Files to Modify

1. `ai-pipeline/lib/src/primitives/state.rs` (new)

    - `PipelineState` struct
    - `StateKey<T>` type
    - `StepError` type

2. `ai-pipeline/lib/src/primitives/runnable.rs` (modify)

    - Update Runnable trait to take &mut PipelineState
    - Add `declares_reads()` and `declares_writes()` methods

3. `ai-pipeline/lib/src/primitives/grouping/pipeline.rs` (modify)

    - Add DynRunnable trait
    - Add StepWrapper for type erasure
    - Update Pipeline to use new pattern

4. `ai-pipeline/lib/src/primitives/atomic/prompt.rs` (modify)

    - Update `Prompt<V>` to implement new **Runnable** trait

5. `ai-pipeline/lib/src/primitives/grouping/concurrency.rs` (modify)

    - Update `InParallel` - needs careful thought around state access
    - Consider: parallel steps get snapshot, merge writes after

6. ai-pipeline/lib/src/primitives/mod.rs (modify)

    - Export new state module

## Parallel Execution Strategy: Read-Only Access

For InParallel, use read-only parallel access:

- Parallel tasks receive a shared reference to state (&PipelineState)
- Tasks can read existing state values but cannot write during execution
- Each task's output is collected into a Vec<R>
- After all parallel tasks complete, outputs can be written to state as a batch

```rust
pub struct InParallel<R: Runnable> {
  pub tasks: Vec<R>,
}

impl<R: Runnable> Runnable for InParallel<R> {
  type Output = Vec<R::Output>;

  fn execute(&self, state: &mut PipelineState) -> Self::Output {
      // Tasks get read-only access, outputs collected
      self.tasks
          .iter()
          .map(|task| task.execute_readonly(state))
          .collect()
  }
}
```

This approach:

- Avoids race conditions - no concurrent writes
- Keeps it simple - no merge/conflict resolution needed
- Enables true parallelism - read-only refs can be shared across threads
