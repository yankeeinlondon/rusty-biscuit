---
status: draft - ready for review
reviewed: false
---

# Claudine Inference Adapter

## Summary

Add a `claudine/contract` sub-crate that implements the
`biscuit_contract::inference::InferenceAdapter` trait by running an agentic CLI
(Claude Code, Codex, Gemini, etc.) as a **single non-interactive session** and
returning its final assistant text as the inference result. This lets
deterministic consumers — Reaper first, Darkmatter next — delegate
judgment-requiring operations to whatever agentic CLI the user already has
installed and authenticated, without taking a direct dependency on any provider
or on the large `claudine` library.

The motivating consumer is Reaper, which evaluates pages and sites
deterministically and reaches for an injected `Arc<dyn InferenceAdapter>` only
for operations that need a language model. A Claudine-backed adapter is
attractive because it reuses the user's existing agentic-CLI authentication and
the provider-normalization, stream-parsing, model-catalog, and policy machinery
that `claudine` already owns.

> **Reader note:** an agentic CLI is not a bare LLM. In its normal
> non-interactive mode it can read files, run shell commands, and call MCP
> tools. The `InferenceAdapter` contract explicitly does **not** authorize tools,
> filesystem access, or network access beyond the selected provider, and
> consumers will feed this adapter untrusted scraped text that may contain
> prompt injection. This specification therefore treats *tool-free,
> filesystem-isolated execution* as a normative requirement, not an
> implementation detail. See [Security and Isolation](#security-and-isolation).

## Scope

This specification delivers the `claudine/contract` adapter crate, its mapping
from the provider-neutral contract onto a claudine non-interactive session, and
its tests. It depends on the already-implemented `biscuit-contract` crate and on
the existing `claudine` library.

In scope:

- A new `claudine-contract` library crate implementing `InferenceAdapter`.
- Building and spawning a provider's non-interactive entrypoint from
  `claudine`'s provider registry, capturing the final assistant text via
  `claudine`'s semantic stream parser.
- Mapping `InferenceProfile` (priority + reasoning effort) onto provider, model,
  and reasoning selection.
- Prose and structured (JSON Schema) output, including adapter-side schema
  validation as required by the contract.
- Mapping claudine/session failures onto the stable `InferenceErrorKind`
  categories.
- Tool-free, MCP-free, filesystem-isolated session execution.

Out of scope:

- Changes to the `biscuit-contract` public API.
- Reaper or Darkmatter consumer wiring (their own follow-up specs).
- Conversational history, streaming to the caller, attachments, embeddings, or
  tool execution exposed through the contract — all excluded by
  `biscuit-contract` v1.
- New provider support in `claudine` itself; this adapter uses the eight
  providers `claudine` already normalizes.

## Goals

- Provide `claudine`'s agentic-CLI execution as a drop-in
  `Arc<dyn InferenceAdapter>` for deterministic consumers.
- Keep the adapter a thin, well-isolated crate that depends on `claudine` and
  `biscuit-contract` but adds no inference logic to `claudine`'s consumers.
- Run every inference session in a locked-down, tool-free, filesystem-isolated
  configuration suitable for untrusted prompt content.
- Honor `InferenceProfile` as a best-effort preference, mapping it onto
  `claudine`'s provider/model/reasoning capabilities.
- Satisfy the structured-output contract: a successful `Structured` response
  validates against the supplied JSON Schema, and a variant mismatch is reported
  as `InvalidResponse`.

## Non-Goals

- Do not add `biscuit-contract` as a dependency of the `claudine` library or
  CLI crates. The dependency lives only in `claudine/contract`.
- Do not expose a stable wire protocol or any provider-native session JSON
  through the shared contract.
- Do not implement retries, caching, or telemetry export. `retry_after` is
  surfaced when available, but the adapter does not retry.
- Do not authorize tool use, MCP servers, or filesystem mutation as a feature.
  These are deliberately disabled.

## Package Area

Add the adapter as the first sub-crate under the existing `claudine` package
area, alongside `lib` and `cli`:

```text
claudine/
  lib/                 # existing: claudine
  cli/                 # existing: claudine-cli (binary: claudine)
  contract/            # NEW
    Cargo.toml
    src/
      lib.rs
      adapter.rs       # ClaudineInferenceAdapter + InferenceAdapter impl
      session.rs       # non-interactive session build/spawn + stream capture
      profile.rs       # InferenceProfile -> provider/model/reasoning mapping
      structured.rs    # schema-injection prompt + JSON Schema validation
      error.rs         # ClaudineError / session failure -> InferenceError
```

- Crate and package name: `claudine-contract`, imported as `claudine_contract`.
- Add `claudine/contract` to the root Cargo workspace `members`.
- Add a `claudine/contract/justfile` reusing the shared `/just` recipes, and add
  the crate to the `claudine` area `justfile` test/lint/build targets. Add it to
  the root curated `just` area list only if it should participate in root
  lifecycle commands; workspace membership alone does not imply root coverage.
- Update `claudine/docs/dependencies.md` (and the root `docs/dependencies.md`)
  for the new crate and its dependencies, and update the `claudine` and
  `biscuit-contract` skills to note the new provider adapter, per repository
  drift-maintenance rules.

## Dependencies

`claudine-contract` may depend on:

- `biscuit-contract` — the trait and data types it implements.
- `claudine` (workspace lib) — provider registry, semantic stream parser,
  model catalog, agent capabilities.
- `async-trait` — to match the contract's default `#[async_trait]` (Send) bound.
- `tokio` — process spawning and async I/O (already a `claudine` dependency).
- `serde_json` — schema and structured payload values.
- A JSON Schema validation engine (e.g. `jsonschema`) — required because the
  adapter, not `biscuit-contract`, owns structured-output validation.
- `thiserror` — for any adapter-internal error type, mapped to `InferenceError`
  at the trait boundary.

The crate must not re-export provider-native types through the contract surface.

## Adapter Construction

The adapter owns the configuration that selects and constrains a session:

```rust
pub struct ClaudineInferenceAdapter { /* provider, model override, isolation */ }

impl ClaudineInferenceAdapter {
    pub fn new(provider: claudine::provider_id::Provider) -> Self;
    pub fn with_model(self, model: impl Into<String>) -> Self;
    pub fn build(self) -> Arc<dyn InferenceAdapter>; // or impl the trait directly
}
```

- The adapter is constructed with an explicit `Provider` (one of `claudine`'s
  eight). There is no implicit default-provider discovery in v1; the consumer or
  its caller chooses, because availability and authentication are the user's
  environment, not the adapter's to guess.
- An optional explicit model overrides profile-driven model selection.
- The constructed value is `Send + Sync` and stored by consumers as
  `Arc<dyn InferenceAdapter>`.
- The public trait impl uses the contract's default `#[async_trait]` bound (not
  `?Send`), so it crosses ordinary multi-threaded runtime boundaries.

## Inference Execution

`infer` runs exactly one non-interactive session per call:

1. **Resolve target.** From the configured `Provider`, read
   `claudine::provider::provider_info(provider)` and require a non-interactive
   entrypoint (`EntrypointSpec` with `mode` of `NonInteractive`/`Both`, e.g.
   Claude's `--print`, Codex's `exec` subcommand). A provider with no
   non-interactive entrypoint yields `InferenceErrorKind::Unsupported`.
2. **Resolve model and reasoning** from `InferenceProfile` (see
   [Profile Mapping](#profile-mapping)), or use the explicit model override.
3. **Build the command** in a locked-down configuration (see
   [Security and Isolation](#security-and-isolation)): the provider binary, its
   required non-interactive flags, the resolved model, and the prompt delivered
   per the provider's `prompt_arg_conventions`. No MCP injection. Tools disabled.
4. **Spawn and stream.** Spawn the process with `tokio`. Construct a parser via
   `claudine::stream::create_semantic_parser(provider, sink, config)`, feed each
   stdout line to `feed_line`, and on exit call `finish(exit_code)` to obtain a
   `StreamExecutionSummary`.
5. **Build the response.** Use `summary.assistant_text` as the model output.
   For `InferenceOutput::Prose`, return `InferenceData::Prose(text)`. For
   `InferenceOutput::Structured`, parse and validate (see
   [Structured Output](#structured-output)).
6. **Populate metadata** (see [Metadata](#metadata)).

Cancellation: dropping the future must kill the spawned child process
(best-effort) and stop streaming. Per the contract, the adapter does not own a
timeout; the consumer wraps `infer` with `tokio::time::timeout`. The adapter may
still surface a provider/stream-reported timeout as
`InferenceErrorKind::Timeout`.

## Profile Mapping

`InferenceProfile` is a best-effort preference, not a guarantee.

- **Model selection.** Map `InferencePriority` onto a provider model using
  `claudine`'s `ModelCatalogService` and the provider's `static_models`:
  - `Cost` → the provider's cheapest/most-efficient catalog model.
  - `Latency` → the provider's fastest small model.
  - `Quality` → the provider's most capable model.
  - `Balanced` → the provider's default model.
  An explicit model override always wins. When the catalog cannot be resolved
  (offline, no dynamic source), fall back to the provider default rather than
  failing.
- **Reasoning effort.** Map `ReasoningEffort` onto the provider's reasoning
  capability as reported by `provider_info(provider).agent_capabilities()`
  (`RuntimeCapabilities.reasoning`). Providers that expose a reasoning/thinking
  control receive a proportional setting (`None` → off, `Low`/`Medium` →
  standard thinking, `High` → maximum). Providers without a reasoning control
  ignore the effort; this is approximation, not `Unsupported`.
- The adapter returns `Unsupported` only when it cannot perform the operation at
  all (e.g. no non-interactive entrypoint), never merely because a preference
  could not be honored exactly.

## Structured Output

`InferenceOutput::Structured { schema }` carries a JSON Schema (Draft 2020-12).

Agentic CLIs vary in native structured-output support
(`NonInteractiveCapabilities.structured_output_supported`). v1 uses a uniform,
provider-independent **prompt-and-parse** strategy with adapter-side validation:

1. **Validate the schema** itself before spawning anything; an unparseable or
   invalid schema is `InvalidRequest`.
2. **Augment the prompt** with an instruction to return a single JSON value
   conforming to the schema and nothing else. (Where a provider's
   `structured_output_supported` flag and entrypoint allow a native
   structured/JSON mode, the adapter may use it; the validation requirement is
   unchanged.)
3. **Extract** the JSON value from `summary.assistant_text` (tolerating
   surrounding prose/code fences is permitted, but the result must be a single
   JSON value).
4. **Validate** the extracted value against the schema with the bundled JSON
   Schema engine.
5. On success return `InferenceData::Structured(value)`. Invalid JSON, schema
   violation, or a response that is prose when structure was requested is
   `InferenceErrorKind::InvalidResponse`, never silent success.

The adapter's validation is a guard, not a trust boundary: consumers still
deserialize the JSON into their own domain type and handle failure.

## Security and Isolation

Because consumers feed untrusted scraped content and an agentic CLI is
tool-capable, every session must run in a constrained configuration:

- **No tools.** The session must not perform file reads/writes, shell
  execution, web fetches, or other tool calls. Where a provider exposes a flag
  or mode to run text-only, use it. As defense-in-depth, the session must run
  with no MCP servers and should be subject to `claudine`'s `PolicyEngine` /
  protect deny behavior so that any attempted tool use is blocked rather than
  executed.
- **Filesystem isolation.** Spawn the process with its working directory set to
  an ephemeral throwaway directory (e.g. a `tempfile` dir), not the consumer's
  repository or CWD, to minimize filesystem exposure if a tool call slips
  through.
- **No environment leakage.** Pass only the environment needed for the provider
  to authenticate and run; do not forward the consumer's full environment.
- **Provider gating.** A provider that cannot be run in a guaranteed tool-free
  manner for untrusted input must be reported as `Unsupported` (or excluded from
  selection) rather than run unsafely. The set of providers that satisfy this
  requirement is determined during implementation and documented in the crate
  README.

The adapter never returns secrets, API keys, or full provider session payloads
in `InferenceError::message`; provider detail stays in tracing only.

## Metadata

Populate `InferenceMetadata` from the session:

- `provider` — the configured provider's slug/display name.
- `model` — `summary.model` when reported, else the resolved model string.
- `agent` — the agentic-CLI identity (provider slug), since this adapter's
  inference *is* an agent invocation. This is the field that distinguishes a
  Claudine-backed response from a direct-LLM one.

Token counts, cost, latency, and finish reasons available in
`StreamExecutionSummary` are **not** surfaced through the v1 contract metadata
(excluded by `biscuit-contract`); they remain available to `claudine`'s own
tracing/reporting. Metadata is diagnostic only and must not change consumer
semantics.

## Error Mapping

Map session outcomes onto `InferenceErrorKind`:

| Condition | Kind |
|-----------|------|
| No non-interactive entrypoint for provider; provider not runnable tool-free | `Unsupported` |
| Invalid request (e.g. malformed schema, empty prompt) | `InvalidRequest` |
| Provider binary missing / not installed (`which` failure) | `Unavailable` |
| Authentication missing or rejected by provider | `Unauthorized` |
| Provider/stream signals rate limiting | `RateLimited` (+ `retry_after` if known) |
| Provider/stream signals overload/5xx/unavailability | `Unavailable` (+ `retry_after` if known) |
| Session timed out (provider/stream reported) | `Timeout` |
| Empty/garbled output, JSON parse failure, schema mismatch, variant mismatch | `InvalidResponse` |
| Any other provider/session failure | `Provider` |

Stream `SemanticEvent::Error { kind, terminal, message, .. }` and the summary's
`is_error`/`error_message`/`exit_code` drive this mapping. `retry_after` is set
only when the provider supplies a meaningful delay.

## Testing

Follow the repository testing tiers (`.claude/skills/rust-testing`).

**L1 (deterministic, default):**

- A session runner seam (trait or injected closure) so tests can supply canned
  provider stdout instead of spawning a real binary. The adapter must be
  testable without any agentic CLI installed.
- Prose request → `InferenceData::Prose` from a canned `assistant_text`.
- Structured request → validation success against a schema; and
  `InvalidResponse` for (a) invalid JSON, (b) schema violation, (c) prose
  returned when structure was requested.
- Profile mapping: assert each `InferencePriority`/`ReasoningEffort` resolves to
  the expected model/reasoning selection for a representative provider.
- Error mapping: each stable `InferenceErrorKind` is produced from the
  corresponding simulated session outcome.
- Object-safety: store and call the adapter through `Arc<dyn InferenceAdapter>`.

**`real_` tier (gated, opt-in):**

- Against a genuinely installed and authenticated provider (e.g. Claude Code),
  prove a prose and a structured request complete end-to-end, that the session
  runs tool-free in an isolated directory, and that structured output validates
  against a real JSON Schema engine. These tests are skipped when the provider
  is unavailable.

## Dependency Direction

```text
biscuit-contract
  ^                      claudine (lib)
  |                        ^
  +---- claudine-contract -+
          ^
          |
          +-- reaper (consumer, via Arc<dyn InferenceAdapter>)
          +-- darkmatter (consumer)
```

`claudine-contract` depends on both `biscuit-contract` and `claudine`. Consumers
depend only on `biscuit-contract` and inject `claudine-contract` at the
composition root, keeping deterministic crates free of provider dependencies.

## Open Questions

- **Per-provider tool-free guarantee.** Which of `claudine`'s eight providers
  can be run in a guaranteed tool-free, filesystem-isolated non-interactive mode
  using their existing flags, versus needing `PolicyEngine`/protect enforcement
  as the sole guard? Implementation must enumerate this and gate unsafe
  providers to `Unsupported`.
- **Native structured mode.** Should v1 prefer a provider's native structured
  output where `structured_output_supported` is true, or use prompt-and-parse
  uniformly for predictability? Default proposal: uniform prompt-and-parse, with
  native mode allowed but not required.

## Success Criteria

- `claudine/contract` is a workspace library crate with standard area `justfile`
  coverage and updated dependency docs and skills.
- `ClaudineInferenceAdapter` implements `InferenceAdapter`, is object-safe, and
  is injectable as `Arc<dyn InferenceAdapter>`.
- A prose and a structured request both succeed end-to-end against a fake
  session runner in L1 tests, with structured output validated against a JSON
  Schema engine owned by this crate.
- Every inference session runs tool-free, MCP-free, and filesystem-isolated;
  providers that cannot guarantee this are reported `Unsupported`.
- `InferenceProfile`, error categories, metadata optionality, and structured
  validation are enforced as specified, with no change to `biscuit-contract`.
