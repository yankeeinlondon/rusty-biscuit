---
name: biscuit-contract
description: Expert knowledge for the biscuit-contract Rust library - the shared, provider-neutral contract for one text-inference operation. Use when working in the biscuit-contract package area, depending on or implementing the InferenceAdapter trait, building a provider (Claudine, Unchained AI) or consumer (Reaper, Darkmatter) adapter, or adding the biscuit-contract dependency.
---

# biscuit-contract

`biscuit-contract` defines the single shared, provider-neutral trait and data
types that Reaper, Darkmatter, Claudine, and Unchained AI agree on for one
text-inference operation (a prompt that returns either prose or JSON). The
crate exists so deterministic consumers can depend on a tiny contract and
inject `Arc<dyn InferenceAdapter>` at operation boundaries, free of any
runtime, HTTP client, JSON Schema engine, or provider crate.

## Start Here

- Use `biscuit_contract::inference::InferenceAdapter` to inject a
  text-inference operation. It is object-safe, `Send + Sync`, and async via
  `#[async_trait]`.
- Use the request/response types (`InferenceRequest`, `InferenceResponse`,
  `InferenceOutput`, `InferenceData`, `InferenceMetadata`) to call the trait
  and interpret results.
- Use `InferenceProfile` (priority + reasoning effort) to express
  provider-neutral preferences.
- Use `InferenceError` / `InferenceErrorKind` to switch on a stable error
  category without depending on a provider's error type.

## Public API

| Item | Purpose |
|------|---------|
| `InferenceAdapter` | The single async trait. Object-safe; store as `Arc<dyn InferenceAdapter>`. |
| `InferenceRequest` | Owned prompt + output shape + profile. |
| `InferenceResponse` | Owned data (prose or structured) + diagnostic metadata. |
| `InferenceOutput` | `Prose` or `Structured { schema: Value }`. |
| `InferenceData` | `Prose(String)` or `Structured(Value)`. |
| `InferenceProfile` | `InferencePriority` + `ReasoningEffort`. |
| `InferenceMetadata` | Optional `provider` / `model` / `agent` strings for diagnostics. |
| `InferenceError` | `thiserror`-derived error with stable `InferenceErrorKind` and optional `retry_after`. |
| `InferenceErrorKind` | `InvalidRequest`, `Unsupported`, `Unavailable`, `Unauthorized`, `RateLimited`, `Timeout`, `InvalidResponse`, `Provider`. |

## Contract Rules

- A successful `Structured` response must satisfy the supplied JSON Schema
  (Draft 2020-12). Validation is the adapter's responsibility; the contract
  does not bundle a JSON Schema engine.
- A response variant that differs from the request variant is reported as
  `InferenceErrorKind::InvalidResponse`, not silently downgraded.
- `retry_after` is populated only when the provider supplies a meaningful
  value (typically for `RateLimited` or `Unavailable`). The contract never
  retries on the consumer's behalf.
- The request owns its data, so the future returned by `infer` is
  independent of caller lifetimes and can be dispatched or queued.

## Dependency Discipline

The crate stays tiny on purpose. Only three production dependencies are
allowed: `async-trait` (object-safety), `serde_json` (schema + structured
payloads), and `thiserror` (error impl). `tokio` is permitted in
`[dev-dependencies]` only — for `#[tokio::test]`. Forbidden classes: async
runtimes (production), HTTP clients, JSON Schema engines, TUI/terminal/
browser/audio crates, and any consumer or provider crate (Reaper,
Darkmatter, Claudine, Unchained AI). See
[`biscuit-contract/docs/dependencies.md`](../../../biscuit-contract/docs/dependencies.md)
for the full rationale and the update procedure.

## Usage Pattern

```rust
use std::sync::Arc;
use async_trait::async_trait;
use biscuit_contract::inference::{
    InferenceAdapter, InferenceData, InferenceError, InferenceMetadata,
    InferenceOutput, InferenceRequest, InferenceResponse,
};

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
                biscuit_contract::inference::InferenceErrorKind::InvalidRequest,
                "echo adapter only accepts Prose requests",
            )),
        }
    }
}

fn make_adapter() -> Arc<dyn InferenceAdapter> {
    Arc::new(EchoAdapter)
}
```

## Provider Adapters

- **`claudine-contract`** (`claudine/contract`) is the first provider adapter
  implementing `InferenceAdapter`. It runs an agentic CLI (Claude Code, Codex)
  as a single non-interactive, tool-free, filesystem-isolated session. It is
  the only crate that depends on both `biscuit-contract` and `claudine` (lib);
  consumers inject it as `Arc<dyn InferenceAdapter>`. See the `claudine` skill
  and `claudine/contract/README.md` (provider support matrix, security posture).

## When You Need More

- Full API and rationale: `biscuit-contract/README.md`
- Dependency rules: `biscuit-contract/docs/dependencies.md`
- Spec / plan: `reaper/features/2026-06-03-inference-trait/spec.md` and
  `reaper/features/2026-06-03-inference-trait/plan.md`
- Claudine adapter: `reaper/features/2026-06-11-claudine-adapter/spec.md`

## Out of Scope

`biscuit-contract` is intentionally not the place for:

- model routing or provider selection
- prompt templates, scraping, Markdown parsing, retries, caching
- telemetry export or agent orchestration
- provider-native request/response values

Provider implementations live in their own package areas: `claudine-contract`
(`claudine/contract`) is the first, implementing the trait over a tool-free
agentic-CLI session. An Unchained AI adapter and consumer wiring in Reaper and
Darkmatter are follow-up work owned by their own areas.
