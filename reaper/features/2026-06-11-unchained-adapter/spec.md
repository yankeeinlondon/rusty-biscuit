---
status: ready for planning and implementation
reviewed: true
sub-spec: true
depends-on: reaper/features/2026-06-03-inference-trait/spec.md
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
completion through a `rig-core` client. Structured requests use the same
single-turn completion path with JSON-only instructions, JSON parsing, and
adapter-side schema validation.

> **Reader note:** `unchained-ai`'s `Prompt::execute()` is currently a stub that
> returns *"LLM execution not yet implemented"*. The provider registry, model
> enums, rich model metadata, and `rig-core` client adaptors all exist, but no
> working completion call does. This specification's scope therefore **includes
> building the real direct-LLM execution path** — a reusable completion and
> structured-output surface in `unchained-ai/lib` — and then mapping the
> contract onto it from the thin `unchained-ai/contract` crate.

## Scope

In scope:

- A new `unchained-ai-contract` library crate implementing `InferenceAdapter`.
- A **real execution path** in `unchained-ai/lib`: given a `ProviderModel`, a
  prompt, an optional JSON Schema, and generation parameters, run a `rig-core`
  completion and return text or a structured JSON value. This resolves the
  current `Prompt::execute()` stub (or provides an equivalent reusable function
  that `Prompt::execute()` can delegate to).
- Mapping `InferenceProfile` (priority + reasoning effort) onto `ModelCapability`
  and a concrete, runnable `ProviderModel`.
- A reusable `ModelCapability` stack resolver in `unchained-ai/lib`. The enum
  exists today, but the repo does not currently expose an ordered-stack
  resolver that can select a runnable `ProviderModel` from the user's configured
  providers.
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
- Build a reusable, single-turn completion and structured-output surface in
  `unchained-ai` that fills the `Prompt::execute()` gap and is consumable beyond
  this adapter.
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
      structured.rs    # JSON parsing + JSON Schema validation
      error.rs         # ProviderError / rig errors -> InferenceError
```

New execution surface in `unchained-ai/lib` (exact module name per local style):

```text
unchained-ai/lib/src/
  execution/           # NEW: single-turn rig-core completion + JSON parsing
    mod.rs
  models/
    selection.rs       # NEW: ModelCapability ordered-stack resolver
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

Build a single-turn completion and structured-output function reusable by both
`Prompt::execute()` and this adapter. Conceptually:

```rust
// in unchained-ai/lib, exact names per local style
pub struct CompletionRequest {
    pub model: ProviderModel,
    pub system_prompt: Option<String>,
    pub prompt: String,
    pub schema: Option<serde_json::Value>, // JSON Schema => JSON-only completion
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
- **Structured completion.** When a schema is present, v1 uses a uniform
  prompt-and-parse strategy: combine the adapter-owned system instruction, the
  caller prompt, and the schema into a request that asks for one JSON value and
  no prose. Parse the model text into a `serde_json::Value` and return that raw
  value. **Schema validation is the caller's responsibility** (the contract
  adapter), keeping the lib surface validation-engine-free. Native
  `rig-core` extractor/JSON-mode support may be added later as an internal
  optimization after the exact API and catalog signals are verified; it is not
  required for this v1 adapter.
- **Parameters.** Apply generation parameters from
  `ProviderModel::metadata().default_parameters` (`temperature`, `top_p`, …) as
  a starting point, overridden by the resolved profile (see
  [Profile Mapping](#profile-mapping)).
- **Synchronous `Prompt` bridge.** `Prompt::execute()` and
  `execute_readonly()` are synchronous `Runnable` methods today, while provider
  calls are async. Add a small blocking bridge in `unchained-ai/lib` (for
  example `complete_blocking`) that runs the async `complete` path on a
  dedicated current-thread runtime or worker thread. It must not call
  `Handle::block_on` from inside an already-running async runtime, because that
  can panic or stall the caller. `Prompt` rewiring must delegate through that
  bridge instead of open-coding runtime behavior.

This keeps `rig-core`, runtime bridging, and provider knowledge in
`unchained-ai/lib`, where the clients already live, and leaves
`unchained-ai/contract` thin.

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
`unchained-ai`'s vocabulary, then resolve to a runnable concrete model.

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

2. **`ModelCapability` → `ProviderModel`.** Add an ordered-stack resolver in
   `unchained-ai/lib` and use it from the adapter. The resolver owns the
   canonical stack for each `ModelCapability` and chooses the first model whose
   provider is runnable. A provider is runnable when `Provider::is_local()` is
   true, or at least one of `Provider::config().env_vars` is present and
   non-empty in the injected environment view. For local providers such as
   Ollama, selection may succeed without credentials; connection failure is
   reported later as `Unavailable`.

   The resolver must accept an injected environment/configuration view so L1
   tests do not mutate process-global environment variables. The same resolver
   should be reusable by `Prompt::execute()` so adapter routing and native
   `Prompt` routing do not drift.

3. **Reasoning parameters.** For the chosen model, translate `ReasoningEffort`
   into the provider's reasoning control where metadata/`supported_parameters`
   indicate one (e.g. an Anthropic thinking budget or an OpenAI reasoning-effort
   parameter). Where none exists, the effort influences only model-variant
   choice. This is provider-specific and best-effort in v1. If the metadata
   does not expose a known reasoning parameter, omit the parameter rather than
   inventing provider-specific request fields.

4. **Model override.** `with_model` pins the `ProviderModel` directly; only
   reasoning/parameters are then profile-driven.

If no model in the resolved stack has a runnable provider, return
`InferenceErrorKind::Unavailable` (nothing runnable). If an explicit
`with_model` target is selected but credentials are missing or empty, return
`Unauthorized` because the caller requested a concrete provider/model that
cannot be authenticated. If credentials are present but rejected by the provider,
also return `Unauthorized`.

## Structured Output

`InferenceOutput::Structured { schema }` carries a JSON Schema (Draft 2020-12).

1. **Validate the schema** before any provider call; an invalid schema is
   `InvalidRequest`.
2. **Execute** via the lib execution surface's prompt-and-parse structured path.
   The adapter must pass the schema as data/instructions for JSON-only output;
   it must not rely on model metadata such as `structured_output` as the only
   correctness guard.
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
| No model in the capability stack has a runnable provider | `Unavailable` |
| Explicit model selected but credentials are missing/empty | `Unauthorized` |
| Auth rejected by provider (401/403) | `Unauthorized` |
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
- Capability resolution tests must use an injected environment view and include
  at least one credential-backed provider and one local-provider case.
- `Prompt::execute()` / `execute_readonly()` tests must prove the synchronous
  bridge calls the same execution surface and does not require a real provider.

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

None. The review resolves the original implementation gaps normatively:
`unchained-ai/lib` must add the missing `ModelCapability` stack resolver,
structured output uses prompt-and-parse plus adapter-side validation in v1, and
`Prompt::execute()` delegates in this same effort through a documented
synchronous bridge over the async execution surface.

## Success Criteria

- `unchained-ai/contract` is a workspace library crate with standard area
  `justfile` coverage and updated dependency docs and skills.
- `unchained-ai/lib` gains a working single-turn `rig-core` completion and
  structured-output surface; `Prompt::execute()` and `execute_readonly()` no
  longer return "not yet implemented" and use the same model-resolution and
  execution path as the adapter.
- `UnchainedInferenceAdapter` implements `InferenceAdapter`, is object-safe, and
  is injectable as `Arc<dyn InferenceAdapter>`.
- A prose and a structured request both succeed end-to-end against a fake
  completion seam in L1 tests, with structured output validated against a JSON
  Schema engine owned by this crate.
- `InferenceProfile` translates into `ModelCapability`/`ProviderModel` through
  the reusable stack resolver, error categories and metadata behave as
  specified, and `biscuit-contract` is unchanged.
