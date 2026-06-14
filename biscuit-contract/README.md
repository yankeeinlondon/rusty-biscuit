# biscuit-contract

`biscuit-contract` defines the shared, provider-neutral trait and data types
that Reaper, Darkmatter, Claudine, and Unchained AI use to perform one
text-inference operation (a prompt that returns either prose or JSON) without
depending on a specific agent, model, or LLM provider. Consumers inject an
`Arc<dyn InferenceAdapter>` at an operation boundary and remain free of
provider crates; adapters in Claudine and Unchained AI implement the trait
and validate structured output against the supplied JSON Schema before
returning success.

## Scope

This crate ships the shared contract only. Provider implementations in
Claudine and Unchained AI, and consumer wiring in Reaper and Darkmatter, are
follow-up work owned by their respective package areas and do not live here.

## Public Surface

- `InferenceAdapter` — object-safe `Send + Sync` async trait with a single
  `infer` method. The crate proves object safety at compile time so consumers
  can store and inject `Arc<dyn InferenceAdapter>`.
- `InferenceRequest` / `InferenceResponse` — owned request and response data
  carrying the prompt, the requested output shape, an inference profile, and
  diagnostic metadata.
- `InferenceOutput` / `InferenceData` — `Prose` and `Structured { schema: Value }`
  variants; the adapter is responsible for confirming structured output
  satisfies the supplied schema before returning success.
- `InferenceProfile` — best-effort, provider-neutral preferences combining an
  `InferencePriority` and a `ReasoningEffort`.
- `InferenceError` / `InferenceErrorKind` — stable error category and a
  `thiserror`-derived concrete error with an optional `retry_after` hint.

## Usage

```rust
use std::sync::Arc;
use async_trait::async_trait;
use biscuit_contract::inference::{
    InferenceAdapter, InferenceData, InferenceError, InferenceErrorKind,
    InferenceMetadata, InferenceOutput, InferenceRequest, InferenceResponse,
};

/// A trivial adapter that echoes prose and rejects structured requests.
struct EchoAdapter;

#[async_trait]
impl InferenceAdapter for EchoAdapter {
    async fn infer(
        &self,
        request: InferenceRequest,
    ) -> Result<InferenceResponse, InferenceError> {
        match request.output {
            InferenceOutput::Prose => Ok(InferenceResponse {
                data: InferenceData::Prose(format!("echo: {}", request.prompt)),
                metadata: InferenceMetadata::default(),
            }),
            InferenceOutput::Structured { .. } => Err(InferenceError::new(
                InferenceErrorKind::InvalidRequest,
                "echo adapter only accepts Prose requests",
            )),
        }
    }
}

fn make_adapter() -> Arc<dyn InferenceAdapter> {
    Arc::new(EchoAdapter)
}
```

## Testing

```bash
just test      # L1 tests
just lint      # clippy -D warnings on the library
just doctest   # runs the InferenceAdapter doc test
just build     # cargo build -p biscuit-contract
```

## Specifications

- Functional spec: `reaper/features/2026-06-03-inference-trait/spec.md`
- Execution plan: `reaper/features/2026-06-03-inference-trait/plan.md`
