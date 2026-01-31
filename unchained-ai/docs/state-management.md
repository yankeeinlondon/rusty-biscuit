# State Management in AI Pipelines

This document describes the state management architecture for the `ai-pipeline` library, enabling type-safe data flow between pipeline steps with robust error handling.

## Overview

The pipeline state system solves a fundamental challenge: how do heterogeneous pipeline steps share data while maintaining type safety? The solution uses **typed keys** that combine runtime flexibility with compile-time guarantees.

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Step 1    │────▶│   Step 2    │────▶│   Step 3    │
│ writes: "a" │     │ reads: "a"  │     │ reads: "b"  │
└─────────────┘     │ writes: "b" │     └─────────────┘
                    └─────────────┘
                           │
                    ┌──────▼───────┐
                    │ PipelineState│
                    │  "a": String │
                    │  "b": i32    │
                    └──────────────┘
```

## Core Concepts

### StateKey<T> - Type-Safe Access

A `StateKey<T>` combines a string name with a phantom type, ensuring that values are always read with the correct type:

```rust
// Define keys as constants - the type is baked in
const USER_INPUT: StateKey<String> = StateKey::new("user_input");
const ANALYSIS: StateKey<ImageAnalysis> = StateKey::new("analysis");
const SCORE: StateKey<f64> = StateKey::new("score");

// Type-safe access - compiler enforces correct types
let input: Option<&String> = state.get(USER_INPUT);
let analysis: Option<&ImageAnalysis> = state.get(ANALYSIS);
```

Keys with the same name but different types are stored separately, preventing accidental type confusion.

### PipelineState - Heterogeneous Container

`PipelineState` stores values of any type in a single container, indexed by `(name, TypeId)` pairs:

```rust
let mut state = PipelineState::new();

// Store different types
state.set(USER_INPUT, "Hello, analyze this image".to_string());
state.set(SCORE, 0.95);
state.set(ANALYSIS, ImageAnalysis { objects: vec!["cat", "dog"] });

// Retrieve with type safety
if let Some(score) = state.get(SCORE) {
    println!("Confidence: {:.1}%", score * 100.0);
}
```

### Runnable Trait - Steps with State Access

Pipeline steps implement the `Runnable` trait, receiving mutable state access during execution:

```rust
pub trait Runnable: Send + Sync {
    type Output: Serialize + Hash + Eq + Send + Sync + 'static;

    fn execute(&self, state: &mut PipelineState) -> Result<Self::Output, StepError>;

    fn execute_readonly(&self, state: &PipelineState) -> Result<Self::Output, StepError>;
    fn supports_readonly(&self) -> bool { false }

    // Optional: declare dependencies for validation
    fn declares_reads(&self) -> &[&'static str] { &[] }
    fn declares_writes(&self) -> &[&'static str] { &[] }
}
```

Returning `Err(StepError)` will be captured by the pipeline executor and stored in `PipelineState`.
Agent delegation primitives should also implement `AgentDelegation` to signal interactive vs
headless execution.

---

## Error Handling

The pipeline uses an **error accumulation** model rather than fail-fast. This is ideal for AI pipelines where partial results are often valuable.

### StepError - Accumulated Errors

Errors are collected during execution rather than immediately halting:

```rust
pub struct StepError {
    pub step_name: String,   // Which step failed
    pub message: String,     // Human-readable description
    pub source: Option<Box<dyn Error>>,  // Underlying cause
    pub fatal: bool,         // Should pipeline halt?
}
```

### Error Severity

- **Non-fatal errors** (default): Logged and accumulated, execution continues
- **Fatal errors**: Pipeline halts after current step completes

```rust
// Non-fatal: log and continue
state.add_error(StepError::new("ImageAnalysis", "Low confidence result"));

// Fatal: stop the pipeline
state.add_error(
    StepError::new("APICall", "Rate limit exceeded")
        .fatal()
        .with_source(api_error)
);
```

### Checking Errors After Execution

```rust
let state = pipeline.run();

if state.has_errors() {
    for err in state.errors() {
        if err.fatal {
            eprintln!("FATAL [{}]: {}", err.step_name, err.message);
        } else {
            eprintln!("WARN  [{}]: {}", err.step_name, err.message);
        }
    }
}

// Partial results may still be available
if let Some(result) = state.get(PARTIAL_RESULT) {
    println!("Got partial result despite errors: {:?}", result);
}
```

### Pipeline Validation

Catch configuration errors before execution by declaring dependencies:

```rust
let pipeline = Pipeline::new()
    .add_with_output(AnalyzeStep, ANALYSIS)      // writes: "analysis"
    .add_with_output(SummarizeStep, SUMMARY);    // reads: "analysis"

// Validates that all reads are satisfied by prior writes
pipeline.validate()?;
```

---

## Parallel Execution

For `InParallel` steps, tasks receive **read-only** state access to avoid race conditions:

```rust
let parallel = InParallel::new(vec![
    FetchFromAPI { endpoint: "users" },
    FetchFromAPI { endpoint: "posts" },
    FetchFromAPI { endpoint: "comments" },
]);

// All tasks read the same state snapshot
// Outputs are collected into Vec<R::Output> on success
let results = parallel.execute(&mut state)?;
```

This design:
- Avoids race conditions (no concurrent writes)
- Keeps it simple (no merge/conflict resolution)
- Enables true parallelism (read-only refs can be shared across threads)
- Records read-only task failures in `PipelineState` while still returning successful outputs

### Composing Parallel Results with State

Since `InParallel<R>` implements `Runnable` with `Output = Vec<R::Output>`, you have two common patterns for integrating parallel results into pipeline state:

#### Pattern A: Single Combined Result

Store the Vec and transform it in a subsequent step:

```rust
const FETCHED_VEC: StateKey<Vec<String>> = StateKey::new("fetched_vec");
const COMBINED: StateKey<String> = StateKey::new("combined");

struct CombineStep;

impl Runnable for CombineStep {
    type Output = String;

    fn execute(&self, state: &mut PipelineState) -> Result<Self::Output, StepError> {
        Ok(state
            .get(FETCHED_VEC)
            .map(|data| data.join(" + "))
            .unwrap_or_default())
    }

    fn declares_reads(&self) -> &[&'static str] { &["fetched_vec"] }
}

let pipeline = Pipeline::new()
    .add_with_output(parallel, FETCHED_VEC)
    .add_with_output(CombineStep, COMBINED);
```

#### Pattern B: Individual State Properties

Store the Vec, then unpack to separate keys in a subsequent step:

```rust
const FETCHED_VEC: StateKey<Vec<String>> = StateKey::new("fetched_vec");
const API_1_RESULT: StateKey<String> = StateKey::new("api_1");
const API_2_RESULT: StateKey<String> = StateKey::new("api_2");

struct UnpackResultsStep;

impl Runnable for UnpackResultsStep {
    type Output = ();

    fn execute(&self, state: &mut PipelineState) -> Result<Self::Output, StepError> {
        if let Some(results) = state.get(FETCHED_VEC) {
            if let Some(r) = results.get(0) {
                state.set(API_1_RESULT, r.clone());
            }
            if let Some(r) = results.get(1) {
                state.set(API_2_RESULT, r.clone());
            }
        }
        Ok(())
    }

    fn declares_reads(&self) -> &[&'static str] { &["fetched_vec"] }
    fn declares_writes(&self) -> &[&'static str] { &["api_1", "api_2"] }
}

let pipeline = Pipeline::new()
    .add_with_output(parallel, FETCHED_VEC)
    .add(UnpackResultsStep);
```

Both patterns keep the design simple: parallel tasks never write directly to state, and all mutations happen serially after parallel execution completes.

---

## Examples

### Example 1: Simple Linear Pipeline

A basic pipeline that processes user input through multiple steps:

```rust
use ai_pipeline::primitives::{Pipeline, PipelineState, StateKey, Runnable, StepError};

// Define state keys
const RAW_INPUT: StateKey<String> = StateKey::new("raw_input");
const CLEANED: StateKey<String> = StateKey::new("cleaned");
const RESPONSE: StateKey<String> = StateKey::new("response");

// Step 1: Clean user input
struct CleanInputStep;
impl Runnable for CleanInputStep {
    type Output = String;

    fn execute(&self, state: &mut PipelineState) -> Result<Self::Output, StepError> {
        Ok(state
            .get(RAW_INPUT)
            .map(|s| s.trim().to_lowercase())
            .unwrap_or_default())
    }

    fn declares_reads(&self) -> &[&'static str] { &["raw_input"] }
    fn declares_writes(&self) -> &[&'static str] { &["cleaned"] }
}

// Step 2: Generate response (placeholder for LLM call)
struct GenerateResponseStep;
impl Runnable for GenerateResponseStep {
    type Output = String;

    fn execute(&self, state: &mut PipelineState) -> Result<Self::Output, StepError> {
        let cleaned = state
            .get(CLEANED)
            .ok_or_else(|| StepError::new("GenerateResponse", "missing cleaned input").fatal())?;
        Ok(format!("You said: {}", cleaned))
    }

    fn declares_reads(&self) -> &[&'static str] { &["cleaned"] }
    fn declares_writes(&self) -> &[&'static str] { &["response"] }
}

// Build and run
fn main() {
    let pipeline = Pipeline::new()
        .add_with_output(CleanInputStep, CLEANED)
        .add_with_output(GenerateResponseStep, RESPONSE);

    pipeline.validate().expect("Invalid pipeline");

    let mut state = PipelineState::new();
    state.set(RAW_INPUT, "  Hello World!  ".to_string());

    pipeline.execute(&mut state);

    println!("{}", state.get(RESPONSE).unwrap());
    // Output: "You said: hello world!"
}
```

### Example 2: Error Handling with Fallbacks

A pipeline that gracefully handles failures and provides partial results:

```rust
use ai_pipeline::primitives::{Pipeline, PipelineState, Runnable, StateKey, StepError};

const PRIMARY_RESULT: StateKey<String> = StateKey::new("primary");
const FALLBACK_RESULT: StateKey<String> = StateKey::new("fallback");
const FINAL_RESULT: StateKey<String> = StateKey::new("final");

struct PrimaryAPIStep;
impl Runnable for PrimaryAPIStep {
    type Output = Option<String>;

    fn execute(&self, _state: &mut PipelineState) -> Result<Self::Output, StepError> {
        // Simulate API failure
        Err(StepError::new("PrimaryAPI", "Service unavailable"))
    }
}

struct FallbackStep;
impl Runnable for FallbackStep {
    type Output = String;

    fn execute(&self, state: &mut PipelineState) -> Result<Self::Output, StepError> {
        // Check if primary succeeded
        if let Some(result) = state.get(PRIMARY_RESULT).and_then(|o| o.as_ref()) {
            return Ok(result.clone());
        }

        // Use fallback
        Ok("Fallback response".to_string())
    }
}

fn main() {
    let pipeline = Pipeline::new()
        .add_with_output(PrimaryAPIStep, PRIMARY_RESULT)
        .add_with_output(FallbackStep, FINAL_RESULT);

    let state = pipeline.run();

    // Pipeline completed despite error
    assert!(state.has_errors());
    assert!(!state.has_fatal_error());

    // Fallback result is available
    assert_eq!(state.get(FINAL_RESULT), Some(&"Fallback response".to_string()));
}
```

### Example 3: Parallel Data Fetching

Fetch data from multiple sources concurrently, then combine:

```rust
use ai_pipeline::primitives::grouping::InParallel;
use ai_pipeline::primitives::{Pipeline, PipelineState, Runnable, StepError};

const FETCHED_DATA: StateKey<Vec<String>> = StateKey::new("fetched");
const COMBINED: StateKey<String> = StateKey::new("combined");

struct FetchStep {
    source: &'static str,
}

impl Runnable for FetchStep {
    type Output = String;

    fn execute(&self, _state: &mut PipelineState) -> Result<Self::Output, StepError> {
        Ok(format!("Data from {}", self.source))
    }

    fn execute_readonly(&self, _state: &PipelineState) -> Result<Self::Output, StepError> {
        Ok(format!("Data from {}", self.source))
    }

    fn supports_readonly(&self) -> bool { true }
}

struct CombineStep;
impl Runnable for CombineStep {
    type Output = String;

    fn execute(&self, state: &mut PipelineState) -> Result<Self::Output, StepError> {
        Ok(state
            .get(FETCHED_DATA)
            .map(|data| data.join(" + "))
            .unwrap_or_default())
    }

    fn declares_reads(&self) -> &[&'static str] { &["fetched"] }
}

fn main() {
    // Parallel fetch from 3 sources
    let parallel = InParallel::new(vec![
        FetchStep { source: "API-1" },
        FetchStep { source: "API-2" },
        FetchStep { source: "API-3" },
    ]);

    let pipeline = Pipeline::new()
        .add_with_output(parallel, FETCHED_DATA)
        .add_with_output(CombineStep, COMBINED);

    let state = pipeline.run();

    println!("{}", state.get(COMBINED).unwrap());
    // Output: "Data from API-1 + Data from API-2 + Data from API-3"
}
```

---

## Summary

| Component | Purpose |
|-----------|---------|
| `StateKey<T>` | Type-safe key for accessing specific value types |
| `PipelineState` | Heterogeneous container with error accumulation |
| `StepError` | Accumulated error with severity (fatal/non-fatal) |
| `Runnable` | Trait for steps with state access and dependency declaration |
| `Pipeline` | Serial executor with validation and error handling |
| `InParallel` | Parallel executor with read-only state access |

The design prioritizes:
1. **Type safety** - Compile-time guarantees where possible
2. **Flexibility** - Runtime composition of heterogeneous steps
3. **Resilience** - Error accumulation with partial results
4. **Simplicity** - Read-only parallel access avoids complexity
