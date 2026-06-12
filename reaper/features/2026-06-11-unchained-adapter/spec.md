---
status: draft - ready for review
reviewed: false
---

# Unchained AI Inference Adapter

## Summary

Add an `unchained-ai/contract` sub-crate that implements the
`biscuit_contract::inference::InferenceAdapter` trait by making **direct LLM
calls** through `unchained-ai`'s provider registry and model catalog, executed
with `rig-core`. This gives deterministic consumers — Reaper first, Darkmatter
next — a low-overhead path to a single text-inference call against a concrete
provider/model, without spawning an agentic CLI and without depending on any
provider crate directly.

Where the Claudine adapter routes inference through an agentic CLI session, this
adapter talks to a model provider's completion API directly. It maps the
provider-neutral `InferenceProfile` onto `unchained-ai`'s richer
`ModelCapability` vocabulary, resolves a concrete `ProviderModel`, and runs a
completion (prose) or schema-constrained extraction (structured) through a
`rig-core` client.

> **Reader note:** `unchained-ai`'s `Prompt::execute()` is currently a stub that
> returns *"LLM execution not yet implemented"*. The provider registry, model
> enums, rich model metadata, and `rig-core` client adaptors all exist, but no
> working completion call does. This specification's scope therefore **includes
> building the real direct-LLM execution path** — a reusable completion/extraction
> surface in `unchained-ai/lib` — and then mapping the contract onto it from the
> thin `unchained-ai/contract` crate.

## Scope

In scope:

- A new `unchained-ai-contract` library crate implementing `InferenceAdapter`.
- A **real execution path** in `unchained-ai/lib`: given a `ProviderModel`, a
  prompt, an optional JSON Schema, and generation parameters, run a `rig-core`
  completion and return text or a structured JSON value. This resolves the
  current `Prompt::execute()` stub (or provides an equivalent reusable function
  that `Prompt::execute()` can delegate to).
- Mapping `InferenceProfile` (priority + reasoning effort) onto `ModelCapability`
  and a concrete, configured `ProviderModel`.
- Prose and structured output, with adapter-side JSON Schema validation as the
  contract requires.
- Mapping `unchained-ai`/`rig-core` failures onto the stable
  `InferenceErrorKind` categories.

Out of scope:

- Changes to the `biscuit-contract` public API.
- Reaper or Darkmatter consumer wiring (their own follow-up specs).
- Conversational history, streaming to the caller, attachments, embeddings, tool
  execution, or agent/tool orchestration through the contract — all excluded by
  `biscuit-contract` v1. The execution path built here is a single-turn
  completion, even though `rig-core` and `unchained-ai` support more.
- Regenerating or changing the model catalog/metadata generator.

## Goals

- Provide direct-LLM inference as a drop-in `Arc<dyn InferenceAdapter>` for
  deterministic consumers.
- Build a reusable, single-turn completion/extraction surface in `unchained-ai`
  that fills the `Prompt::execute()` gap and is consumable beyond this adapter.
- Translate the provider-neutral profile into `unchained-ai`'s
  `ModelCapability`/`ProviderModel` model, honoring it as a best-effort
  preference.
- Satisfy the structured-output contract: a successful `Structured` response
  validates against the supplied JSON Schema; variant mismatch is
  `InvalidResponse`.

## Non-Goals

- Do not add `biscuit-contract` as a dependency of `unchained-ai`'s lib, gen, or
  cli crates. The contract dependency lives only in `unchained-ai/contract`.
- Do not duplicate `unchained-ai`'s provider/model enums into `biscuit-contract`;
  the profile→capability translation lives in this adapter (and lib), as the
  inference-trait spec requires.
- Do not implement retries, caching, or telemetry export. `retry_after` is
  surfaced when the provider supplies it; the adapter does not retry.
- Do not expose `rig-core` or provider-native request/response types through the
  shared contract.

## Package Area

Add the adapter as a sibling crate in the `unchained-ai` package area, which
already splits into `lib`, `cli`, `gen`, and `model_id`:

```text
unchained-ai/
  lib/                 # existing: unchained-ai (gains the execution surface)
  cli/                 # existing: unchained
  gen/                 # existing: unchained-ai-gen
  model_id/            # existing proc-macro crate
  contract/            # NEW
    Cargo.toml
    src/
      lib.rs
      adapter.rs       # UnchainedInferenceAdapter + InferenceAdapter impl
      profile.rs       # InferenceProfile -> ModelCapability -> ProviderModel
      structured.rs    # schema-constrained extraction + JSON Schema validation
      error.rs         # ProviderError / rig errors -> InferenceError
```

New execution surface in `unchained-ai/lib` (exact module name per local style):

```text
unchained-ai/lib/src/
  execution/           # NEW: single-turn rig-core completion + extraction
    mod.rs
```

- Crate and package name: `unchained-ai-contract`, imported as
  `unchained_ai_contract`.
- Add `unchained-ai/contract` to the root Cargo workspace `members`.
- Add an `unchained-ai/contract/justfile` reusing `/just` shared recipes and add
  the crate to the `unchained-ai` area `justfile` targets. Add to the root
  curated `just` list only if root lifecycle participation is intended.
- Update `unchained-ai/docs/dependencies.md` and root `docs/dependencies.md`,
  and update the `unchained-ai` and `biscuit-contract` skills for the new
  execution surface and provider adapter, per drift-maintenance rules.

## Dependencies

`unchained-ai-contract` may depend on:

- `biscuit-contract` — the trait and data types it implements.
- `unchained-ai` (workspace lib) — provider registry, `ModelCapability`,
  `ProviderModel`, metadata, client adaptors, and the new execution surface.
- `async-trait` — to match the contract's default `#[async_trait]` (Send) bound.
- `tokio` — async runtime (already an `unchained-ai` dependency).
- `serde_json` — schema and structured payload values.
- A JSON Schema validation engine (e.g. `jsonschema`) — the adapter owns
  structured-output validation.
- `thiserror` — adapter-internal error type mapped to `InferenceError`.

The new `unchained-ai/lib` execution surface depends on the already-present
`rig-core` (v0.31, OpenAI-compatible completion clients and provider adaptors
such as the Z.ai / ZenMux `client_adaptors`).

## Execution Surface (unchained-ai/lib)

Build a single-turn completion/extraction function reusable by both
`Prompt::execute()` and this adapter. Conceptually:

```rust
// in unchained-ai/lib, exact names per local style
pub struct CompletionRequest {
    pub model: ProviderModel,
    pub system_prompt: Option<String>,
    pub prompt: String,
    pub schema: Option<serde_json::Value>, // JSON Schema => structured extraction
    pub parameters: ResolvedParameters,     // temperature, etc.
}

pub async fn complete(request: CompletionRequest)
    -> Result<CompletionOutput, ProviderError>;
```

- **Client construction.** From `ProviderModel::provider()` and
  `Provider::config()` (env vars, `auth_method`, `base_url`), build the
  appropriate `rig-core` completion client (`from_env()` per provider; reuse the
  existing OpenAI-compatible client adaptors). Missing credentials surface as a
  typed `ProviderError` (`MissingApiKey`).
- **Prose completion.** Build a `rig-core` `CompletionModel` for
  `ProviderModel::model_id()` and run a single completion with the system prompt
  and user prompt; return the text.
- **Structured extraction.** When a schema is present, prefer `rig-core`'s
  structured/extractor path for models whose metadata reports
  `has_capability("structured_output")`; otherwise fall back to a
  prompt-and-parse strategy (instruct the model to emit a single JSON value
  matching the schema). Return the raw JSON value; **schema validation is the
  caller's responsibility** (the contract adapter), keeping the lib surface
  validation-engine-free.
- **Parameters.** Apply generation parameters from
  `ProviderModel::metadata().default_parameters` (`temperature`, `top_p`, …) as
  a starting point, overridden by the resolved profile (see
  [Profile Mapping](#profile-mapping)).
- Wiring `Prompt::execute()` to delegate to this surface is in scope so the stub
  no longer returns a fatal "not yet implemented".

This keeps `rig-core` and provider knowledge in `unchained-ai/lib`, where the
clients already live, and leaves `unchained-ai/contract` thin.

## Adapter Construction

```rust
pub struct UnchainedInferenceAdapter { /* model override, defaults */ }

impl UnchainedInferenceAdapter {
    pub fn new() -> Self;                                  // profile-driven model
    pub fn with_model(self, model: ProviderModel) -> Self; // pin a concrete model
    pub fn build(self) -> Arc<dyn InferenceAdapter>;
}
```

- The adapter is `Send + Sync` and stored as `Arc<dyn InferenceAdapter>`.
- The public trait impl uses the contract's default `#[async_trait]` bound.
- An explicit `with_model` pins a concrete `ProviderModel` and bypasses
  profile-driven model selection (profile still drives reasoning/parameters).

## Profile Mapping

`InferenceProfile` is a best-effort preference. Translate it into
`unchained-ai`'s vocabulary, then resolve to a configured concrete model.

1. **Priority + reasoning → `ModelCapability`.** Combine the two contract
   dimensions into one capability tier:

   | `InferencePriority` | base capability |
   |---------------------|-----------------|
   | `Cost` | `NormalCheap` / `FastCheap` stack |
   | `Latency` | `Fast` |
   | `Quality` | `Smart` |
   | `Balanced` | `Normal` |

   Then apply `ReasoningEffort`: `None` keeps the non-thinking variant;
   `Low`/`Medium` select the `*Thinking`/`*Think` variant; `High` selects the
   `*Ultrathink` variant where one exists (e.g. `Quality` + `High` →
   `SmartUltrathink`). When no thinking variant exists for a tier, drop to the
   nearest available variant — approximation, not failure.

2. **`ModelCapability` → `ProviderModel`.** `ModelCapability` tiers are ordered
   stacks (first match wins). Resolve to the first model in the stack **whose
   provider is configured** — i.e. the provider's `config().env_vars` are
   present in the environment. This makes the adapter pick a usable model based
   on the credentials the user actually has, rather than a fixed default.

3. **Reasoning parameters.** For the chosen model, translate `ReasoningEffort`
   into the provider's reasoning control where metadata/`supported_parameters`
   indicate one (e.g. an Anthropic thinking budget or an OpenAI reasoning-effort
   parameter). Where none exists, the effort influences only model-variant
   choice. This is provider-specific and best-effort in v1.

4. **Model override.** `with_model` pins the `ProviderModel` directly; only
   reasoning/parameters are then profile-driven.

If no model in the resolved stack has a configured provider, return
`InferenceErrorKind::Unavailable` (nothing runnable), or `Unauthorized` if a
model was selectable but its credentials were rejected.

## Structured Output

`InferenceOutput::Structured { schema }` carries a JSON Schema (Draft 2020-12).

1. **Validate the schema** before any provider call; an invalid schema is
   `InvalidRequest`.
2. **Execute** via the lib execution surface's structured path: native
   structured/extractor support where the model reports it, prompt-and-parse
   otherwise.
3. **Validate** the returned JSON value against the schema using the adapter's
   bundled JSON Schema engine.
4. On success return `InferenceData::Structured(value)`. Invalid JSON, schema
   violation, or prose returned when structure was requested is
   `InferenceErrorKind::InvalidResponse`.

Adapter validation is a guard, not a trust boundary; consumers still deserialize
into their own domain type and handle failure. Consumers must also treat prompt
context and model output as untrusted (scraped text may contain prompt
injection); the adapter authorizes no tools, filesystem, or network access
beyond the selected provider's completion endpoint.

## Metadata

Populate `InferenceMetadata` from the resolved model:

- `provider` — `ProviderModel::provider()` display name (e.g. "OpenAI").
- `model` — `ProviderModel::wire_id()` or `model_id()`.
- `agent` — `None`; direct-LLM inference has no agent identity. This is what
  distinguishes an Unchained-backed response from a Claudine-backed one.

Token counts, cost, latency, and finish reasons are excluded from v1 contract
metadata even though `rig-core` may report them; they belong to a future
observability contract. Metadata is diagnostic only.

## Error Mapping

| Condition | Kind |
|-----------|------|
| Malformed schema, empty prompt, unparseable model override | `InvalidRequest` |
| Requested capability/modality unsupported by every resolvable model | `Unsupported` |
| No model in the stack has configured credentials | `Unavailable` |
| `ProviderError::MissingApiKey` / auth rejected (401/403) | `Unauthorized` |
| Provider returns 429 / rate limit | `RateLimited` (+ `retry_after` from headers when present) |
| Provider 5xx / overload / network unreachable | `Unavailable` (+ `retry_after` if known) |
| Request deadline exceeded (provider/transport reported) | `Timeout` |
| Invalid JSON, schema mismatch, variant mismatch, empty completion | `InvalidResponse` |
| Any other `rig-core`/provider failure | `Provider` |

`ProviderError` and `rig-core` errors are mapped at the trait boundary;
`InferenceError::message` must not contain API keys, auth headers, or full
provider payloads — those stay in tracing/the error chain.

## Testing

Follow the repository testing tiers (`.claude/skills/rust-testing`).

**L1 (deterministic, default):**

- Profile mapping: assert each `InferencePriority` × `ReasoningEffort`
  combination resolves to the expected `ModelCapability` and, given a fixed set
  of "configured" providers (injected env view), the expected `ProviderModel`.
- Structured validation: success against a schema; `InvalidResponse` for invalid
  JSON, schema violation, and prose-when-structure-requested — driven through a
  fake completion seam so no network call occurs.
- Error mapping: each stable `InferenceErrorKind` from the corresponding
  simulated provider/transport outcome.
- Object-safety: store and call through `Arc<dyn InferenceAdapter>`.
- The execution surface must accept an injected client/transport seam so its
  logic is testable without real providers.

**`real_` tier (gated, opt-in):**

- Against a real provider with credentials in the environment, prove a prose and
  a structured request complete end-to-end and that structured output validates
  against a real JSON Schema engine. Skipped when no provider credentials are
  present.

## Dependency Direction

```text
biscuit-contract
  ^                       unchained-ai (lib, incl. execution surface)
  |                         ^
  +---- unchained-ai-contract +
          ^
          |
          +-- reaper (consumer, via Arc<dyn InferenceAdapter>)
          +-- darkmatter (consumer)
```

`unchained-ai-contract` depends on both `biscuit-contract` and `unchained-ai`.
Consumers depend only on `biscuit-contract` and inject this adapter at the
composition root.

## Open Questions

- **Reasoning parameter coverage.** Which providers/models in the catalog expose
  a usable reasoning/thinking parameter via metadata, and which only reflect
  effort through model-variant choice? The mapping must degrade gracefully where
  no control exists.
- **Structured extraction mechanism.** Confirm the exact `rig-core` v0.31
  structured-output/extractor API to use for the native path, versus
  prompt-and-parse fallback, and which catalog models truthfully report
  `structured_output`.
- **`Prompt::execute()` coupling.** Should `Prompt::execute()` delegate to the
  new execution surface in this same effort, or should the surface ship first
  and `Prompt` rewiring follow? Default proposal: build the surface and rewire
  `Prompt::execute()` to delegate, removing the fatal stub.

## Success Criteria

- `unchained-ai/contract` is a workspace library crate with standard area
  `justfile` coverage and updated dependency docs and skills.
- `unchained-ai/lib` gains a working single-turn `rig-core` completion/extraction
  surface; `Prompt::execute()` no longer returns "not yet implemented".
- `UnchainedInferenceAdapter` implements `InferenceAdapter`, is object-safe, and
  is injectable as `Arc<dyn InferenceAdapter>`.
- A prose and a structured request both succeed end-to-end against a fake
  completion seam in L1 tests, with structured output validated against a JSON
  Schema engine owned by this crate.
- `InferenceProfile` translates into `ModelCapability`/`ProviderModel`, error
  categories and metadata behave as specified, and `biscuit-contract` is
  unchanged.
