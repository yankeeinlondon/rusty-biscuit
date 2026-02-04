# Pipeline Primitives

Detailed reference for the pipeline execution model in `unchained-ai/lib/src/primitives/`.

## PipelineState (`state.rs`)

Heterogeneous container keyed by `StateKey<T>` (string name + `TypeId`). The same string key can coexist for different types.

```rust
use unchained_ai::primitives::state::{PipelineState, StateKey};

let mut state = PipelineState::new();
let key = StateKey::<String>::new("username");
state.set(&key, "alice".to_string());
let value: Option<&String> = state.get(&key);
```

### StepError

Carries step name, message, optional source error, and a `fatal` flag. Non-fatal errors are accumulated; fatal errors halt the pipeline.

```rust
StepError {
    step: "my_step",
    message: "something went wrong",
    source: None,
    fatal: false, // pipeline continues
}
```

## Runnable Trait (`runnable.rs`)

The step interface for pipeline execution.

```rust
pub trait Runnable {
    type Output;

    fn execute(&self, state: &mut PipelineState) -> Result<Self::Output, StepError>;

    // Read-only variant for parallel execution
    fn execute_readonly(&self, state: &PipelineState) -> Result<Self::Output, StepError>;

    // Static key declarations for dependency validation
    fn declares_reads(&self) -> Vec<&'static str> { vec![] }
    fn declares_writes(&self) -> Vec<&'static str> { vec![] }

    fn supports_readonly(&self) -> bool { false }
}
```

### AgentDelegation Trait

Extends `Runnable` with interactivity signal for agent CLI delegation:

```rust
pub trait AgentDelegation: Runnable {
    fn is_interactive(&self) -> bool;
}
```

### RunnableExt

Extension trait providing `with_output_key(key)` to wrap a step and persist its output into state.

## Grouping (`grouping/`)

### Pipeline (`pipeline.rs`)

Serial composition of heterogeneous steps via type-erased `DynRunnable`. Validates that reads are satisfied by prior writes before execution.

```rust
use unchained_ai::primitives::grouping::Pipeline;

let pipeline = Pipeline::new()
    .with(step_a)              // discard output
    .add_with_output(step_b, "result");  // store output in state
pipeline.execute(&mut state)?;
```

### InParallel (`in_parallel.rs`)

Parallel execution for homogeneous tasks. All tasks receive read-only state access. Currently sequential but the read-only contract enables future true parallelism.

```rust
use unchained_ai::primitives::grouping::InParallel;

let parallel = InParallel::new(vec![task_a, task_b, task_c]);
let results: Vec<Output> = parallel.execute(&mut state)?;
```

## Atomic Primitives (`atomic/`)

### Prompt\<V\> (`prompt.rs`)

Multi-modal prompt container with builder API. Supports:

- **Text**: plain strings or `file://` paths
- **Images**: `image://` URLs or binary data (PNG/JPEG/GIF/WebP/SVG detected via magic bytes)
- **Audio**: `audio://` URLs or binary data (MP3/WAV/OGG/FLAC detected via magic bytes)

Builder methods:
- `.using_model(ModelCapability)` - set capability-based model selection
- `.with_image(url_or_bytes)` - attach image content
- `.with_audio(url_or_bytes)` - attach audio content
- `.prefer_multi_modal(bool)` - hint for multi-modal model selection
- `.with_structured_response::<T>()` - request structured JSON output

Validation: `validate()` performs async HEAD requests on external URLs.

**Not yet implemented**: `execute()` returns a fatal `StepError` (LLM execution not wired to rig-core).

### OpenCodeDelegation (`agent_delegation.rs`)

Delegates pipeline steps to the `opencode` CLI. Implements `AgentDelegation` trait.

- **Modes**: Interactive (TUI/REPL) or Headless (single CLI call)
- **Sessions**: `New`, `ContinueLast`, or `SessionId(String)`
- **State passing**: Serializes `PipelineState` as JSON and embeds JSON schema in prompt
- **Output parsing**: Parses JSON event stream from OpenCode stdout to extract assistant output
- **Interactivity**: `is_interactive()` returns `true` for interactive mode

### Placeholders

- `UserContent` - empty struct, future: insert content into pipeline without LLM interaction
- `Transcribe` - empty struct, future: audio transcription step

## Functional Grouping (`functional_grouping/`)

Scaffold modules for future pipeline operators:
- `concat.rs` - combining step outputs
- `splinter.rs` - splitting/fanning out

## Foreign Agents (`foreign_agent/`)

Incomplete trait skeleton for integrating external agentic systems (Claude Code, OpenCode, Firecrawl). Defines types:
- `ForeignAgentLocality` (LocalCli / CloudApi)
- `ConcurrencyCap`, `UsageCap`, `AuthMethod`, `AgenticPlanType`

The `ForeignAgent` trait definition is not yet complete.
