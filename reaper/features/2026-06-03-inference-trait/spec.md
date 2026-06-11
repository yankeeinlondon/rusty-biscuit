---
status: ready for planning and implementation
reviewed: true
---

# Inference Trait Contract

## Summary

Create a lightweight `biscuit-contract` package area containing the shared,
provider-neutral contract for one text-inference operation. Deterministic
libraries can depend on this crate and optionally delegate non-deterministic
work to an injected adapter without depending on a specific agent or LLM
provider.

Reaper is the motivating consumer: it will evaluate pages and sites with
deterministic logic first and use an adapter only for operations that require
judgment. Darkmatter should be able to use the same contract for Markdown and
document-oriented inference. The first expected adapter providers are
Claudine, backed by agent calls, and Unchained AI, backed by direct LLM calls.

> **Reader note:** the initial draft represented model choice as variants such
> as `Cheap`, `Fast`, and `Smart(ThinkingLevel)`. Cost, latency, quality, and
> reasoning effort are independent preferences, and combining them in one enum
> creates ambiguous and increasingly numerous variants. This review replaces
> that enum with an inference profile containing an optimization priority and a
> separate reasoning effort. Adapters map those preferences onto their own
> richer model catalogs.

## Scope

This specification delivers only the shared contract crate and its tests.
Provider implementations in Claudine and Unchained AI, and consumer wiring in
Reaper and Darkmatter, require follow-up specifications owned by those package
areas. Reaper currently has documentation but no library crate, so requiring a
Reaper integration here would make this specification impossible to complete
independently.

The v1 operation accepts a text prompt and returns either prose or JSON. Binary
attachments, conversational history, streaming, tool execution, and embeddings
are intentionally outside this contract. They should not be added to the
rolled-up `infer` method as optional fields without a separate design review.

## Goals

- Define one stable inference trait that Reaper, Darkmatter, Claudine, and
  Unchained AI can share.
- Keep deterministic consumer crates free of direct agent, model, and provider
  dependencies.
- Let provider crates implement one operation rather than adding a trait method
  for every domain task.
- Make the adapter object-safe so consumers can store and inject
  `Arc<dyn InferenceAdapter>`.
- Give structured inference a meaningful success contract: successful JSON
  must satisfy the supplied JSON Schema.
- Keep the crate small, portable, and limited to contracts rather than
  orchestration.

## Non-Goals

- Do not put Reaper, Darkmatter, Claudine, or Unchained AI domain types in
  `biscuit-contract`.
- Do not define model routing, provider selection, prompt templates, scraping,
  Markdown parsing, retries, caching, telemetry export, or agent orchestration.
- Do not require deterministic operations to use inference.
- Do not define a stable network or subprocess wire protocol. The v1 contract
  is an in-process Rust API.
- Do not expose provider-native request or response values through the shared
  API.

## Package Area

Add the library package area using the repository's standard layout:

```text
biscuit-contract/
  README.md
  justfile
  docs/
    dependencies.md
  lib/
    Cargo.toml
    src/
      inference.rs
      lib.rs
```

The library crate and package name are both `biscuit-contract`, imported as
`biscuit_contract`. Add `biscuit-contract/lib` to the root Cargo workspace.
Add the package area to the root `justfile` curated area list only if it is
intended to participate in the root lifecycle commands; workspace membership
alone does not imply root `just` coverage.

The implementation must also update the root dependency documentation and the
local skill catalog for the new shared package area, as required by repository
drift-maintenance rules.

## Contract

The public API models one complete inference call. It owns its request data so
the async future is independent of caller lifetimes and can be dispatched or
queued by an adapter.

The normative shape is:

```rust
use async_trait::async_trait;
use serde_json::Value;
use std::time::Duration;
use thiserror::Error;

#[async_trait]
pub trait InferenceAdapter: Send + Sync {
    async fn infer(
        &self,
        request: InferenceRequest,
    ) -> Result<InferenceResponse, InferenceError>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct InferenceRequest {
    pub prompt: String,
    pub output: InferenceOutput,
    pub profile: InferenceProfile,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InferenceOutput {
    Prose,
    Structured { schema: Value },
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InferenceProfile {
    pub priority: InferencePriority,
    pub reasoning: ReasoningEffort,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum InferencePriority {
    Cost,
    Latency,
    Quality,
    #[default]
    Balanced,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ReasoningEffort {
    None,
    Low,
    #[default]
    Medium,
    High,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InferenceResponse {
    pub data: InferenceData,
    pub metadata: InferenceMetadata,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InferenceData {
    Prose(String),
    Structured(Value),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InferenceMetadata {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub agent: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InferenceErrorKind {
    InvalidRequest,
    Unsupported,
    Unavailable,
    Unauthorized,
    RateLimited,
    Timeout,
    InvalidResponse,
    Provider,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{message}")]
pub struct InferenceError {
    pub kind: InferenceErrorKind,
    pub message: String,
    pub retry_after: Option<Duration>,
}
```

Exact module paths, constructors, and convenience conversions may follow local
style, but implementation must preserve these semantics. Use `async-trait`,
which is already used in the workspace, so the public trait remains object-safe
and adapters can be stored as `Arc<dyn InferenceAdapter>`. Do not leave the
async representation as a planning-time choice. The public trait and every impl
must use matching default `#[async_trait]` bounds, not `#[async_trait(?Send)]`,
because the shared adapter is intended to cross ordinary multi-threaded runtime
boundaries.

`InferenceRequest` and response types do not derive `Serialize` or
`Deserialize` in v1. Adding those derives would implicitly establish a wire
format and enum representation that this specification does not design.

`InferenceError` must implement `std::error::Error` and `Display`. Use
`thiserror`, the repository-standard library error helper, rather than forcing
consumers to special-case a diagnostic-only struct.

## Preference Semantics

`InferenceProfile` contains best-effort provider-neutral preferences, not hard
service-level guarantees.

- `priority` tells the adapter which of cost, latency, or output quality to
  optimize when it has a choice. `Balanced` delegates the trade-off to the
  adapter.
- `reasoning` requests the desired reasoning effort independently of model
  priority. `None` means no deliberate reasoning mode is requested; it does not
  assert that a provider performs no internal reasoning.
- An adapter may approximate a profile when its provider has no exact mapping.
- An adapter returns `Unsupported` only when it cannot perform the operation at
  all, not merely because it cannot honor every preference exactly.

Unchained AI must translate this profile into its existing, richer
`ModelCapability` vocabulary in its own adapter crate or module. The shared
crate must not depend on or duplicate Unchained AI's provider/model enums.

## Structured Output Contract

`InferenceOutput::Structured` carries a JSON Schema represented by
`serde_json::Value`. The schema dialect is JSON Schema Draft 2020-12. Callers
should include `$schema` explicitly; when omitted, adapters must interpret the
schema as Draft 2020-12.

The schema may describe any valid JSON value, not only an object. Schema
generation remains the consumer's responsibility; consumers may use
`schemars`, handwritten schemas, or another generator without adding that
dependency to `biscuit-contract`.

For a structured request:

1. The adapter must reject an invalid schema as `InvalidRequest` before calling
   a provider.
2. The adapter may use native structured-output support or prompt-and-parse
   fallback behavior.
3. A successful response must be `InferenceData::Structured` and validate
   against the supplied schema.
4. Invalid JSON, schema mismatch, or a response data variant that differs from
   the request is `InvalidResponse`, not success.
5. Consumers must still deserialize the JSON into their domain-owned type and
   handle deserialization failure. Adapter validation is not a trust boundary.

The contract crate does not perform schema validation itself. Each adapter is
responsible for validation because it owns the inference execution path and
must not return success before the guarantee is met. This avoids forcing a
JSON Schema engine into consumers that only use prose inference.

## Errors, Retries, and Timeouts

`InferenceError` is concrete so consumers can handle stable categories without
depending on provider errors. `message` must be suitable for diagnostics but
must not contain secrets, API keys, authorization headers, or full provider
payloads. Provider-specific error objects remain in the adapter's own tracing
or error chain and are not exposed through the shared contract.

`retry_after` is populated only when the provider supplies a meaningful delay,
normally for `RateLimited` or `Unavailable`. The contract does not retry.

The request does not contain a timeout. Callers own end-to-end deadlines and
may wrap `infer` with `tokio::time::timeout` or an equivalent runtime mechanism.
Dropping the returned future requests cancellation, but adapters only provide
best-effort cancellation because an upstream provider may already be
processing the request.

## Metadata

`provider`, `model`, and `agent` are optional because not every backend exposes
all three identities. Adapters should populate every value they can report
reliably. These fields are diagnostic metadata and must not be used by
consumers to change domain semantics.

Token counts, latency, finish reasons, raw provider IDs, and billing data are
excluded from v1. Their units and availability differ enough that adding them
requires a separate observability contract rather than an untyped metadata
map.

## Consumer Contract

Reaper and Darkmatter will own their prompts, schemas, domain result types, and
the policy deciding whether inference is necessary. Their eventual APIs should
accept an optional `Arc<dyn InferenceAdapter>` at an operation or service
boundary; absence of an adapter must not affect deterministic operations.

Consumers must treat prompt context and adapter responses as untrusted data.
In particular, scraped page text and Markdown content may contain prompt
injection. The adapter contract does not authorize tools, filesystem access,
network access beyond the selected provider, or mutation of consumer state.

## Provider Contract

Claudine and Unchained AI will implement `InferenceAdapter` in their own
package areas in follow-up work.

- Claudine maps the profile to an agent selection/execution strategy and
  returns `agent`, `provider`, and `model` when available.
- Unchained AI maps the profile to its model catalog and inference settings and
  returns the concrete provider/model when available.
- Both implementations enforce the requested output variant and structured
  schema before returning success.
- Neither implementation needs to know which consumer or domain produced the
  request.

## Dependency Direction

The intended dependency direction is:

```text
biscuit-contract
  ^
  |
  +-- reaper
  +-- darkmatter
  +-- claudine
  +-- unchained-ai
```

`biscuit-contract` may depend on `async-trait`, `serde_json`, and `thiserror`.
It must not depend on Tokio, an HTTP client, a JSON Schema engine, or any
consumer/provider crate.

No crate feature flags are required in v1. Adding optional provider,
serialization, schema-validation, or runtime features would create implicit
sub-contracts and should be handled by a follow-up design.

## Compatibility and Evolution

The crate starts at version `0.1.0`. Public enum additions are breaking changes
until a deliberate extensibility strategy is specified, so v1 should include
only the variants listed here. Do not add provider-specific extension maps as
an escape hatch; they would move coupling into string keys and weaken the
shared contract.

## Testing

All contract-crate tests are L1 and deterministic. At minimum, tests must:

- prove that a fake adapter can be stored and called through
  `Arc<dyn InferenceAdapter>`;
- cover prose and structured requests through that trait object;
- assert defaults for `InferenceProfile`;
- assert construction and matching of every stable error category;
- use a fake adapter to demonstrate that an implementation can report response
  variant mismatch and a deliberately simulated schema violation as
  `InvalidResponse`.

Real provider conformance tests belong to the Claudine and Unchained AI
follow-up specifications and must use the repository's `real_` test tier.
Those follow-up tests must validate real structured-output behavior against a
JSON Schema engine owned by the provider adapter or test harness, not by
`biscuit-contract`.

## Open Questions

None. The original ownership, object-safety, async representation, error shape,
schema representation, capability, metadata, and timeout questions are resolved
normatively above.

## Success Criteria

- `biscuit-contract` is a workspace library with the standard package-area
  documentation and `justfile` coverage expected for a new area.
- The crate exposes the object-safe contract and data types defined above with
  no dependency on a consumer, provider, async runtime, HTTP client, or schema
  validator.
- A deterministic L1 test calls prose and structured fake adapters through
  `Arc<dyn InferenceAdapter>`.
- Structured success, response-variant matching, profile semantics, error
  categories, timeout ownership, and metadata optionality are documented as
  enforceable contracts rather than implementation-time choices.
- Follow-up provider and consumer specifications can implement against this
  contract without changing its public API.
