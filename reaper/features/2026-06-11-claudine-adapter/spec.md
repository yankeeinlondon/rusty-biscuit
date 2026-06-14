---
status: ready for planning and implementation
reviewed: true
sub-spec: true
depends-on: reaper/features/2026-06-03-inference-trait/spec.md
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
from the provider-neutral contract onto a Claudine non-interactive session, and
its tests. It depends on the already-implemented `biscuit-contract` crate and on
the existing `claudine` library.

> **Reader note:** Claudine's provider catalog, model catalog, permissions
> policy, and semantic stream parser are library APIs, but some reusable
> non-interactive argv/session assembly currently lives in the `claudine-cli`
> wrapper implementation. This adapter crate must not depend on the CLI crate.
> Any wrapper code needed by the adapter must be moved into `claudine/lib` as a
> provider-neutral session-planning/execution surface, then consumed by both
> `claudine-cli` and `claudine-contract`.

In scope:

- A new `claudine-contract` library crate implementing `InferenceAdapter`.
- Any small `claudine/lib` extraction needed so provider non-interactive
  entrypoint resolution, prompt argument placement, model/reasoning flags,
  output-format flags, shadow-home setup, and process spawning can be reused
  without depending on `claudine-cli`.
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
- A documented provider support matrix for this adapter's tool-free
  non-interactive mode.

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
- Do not add a dependency from `claudine-contract` to `claudine-cli`, and do
  not expose CLI-only wrapper types as this crate's public API.
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
- If current `claudine-cli` wrapper code is needed, move only the reusable,
  provider-neutral pieces into `claudine/lib` under names chosen during
  implementation. Keep command-line presentation, user prompts, and wrapper
  command dispatch in `claudine-cli`.
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
  model catalog, permission/policy metadata, agent capabilities, and any
  extracted reusable non-interactive session planner.
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
    pub fn new(provider: claudine::provider::Provider) -> Self;
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
   per the provider's `prompt_arg_conventions`. No MCP injection. Tools
   disabled. Use `ProviderInfo::entrypoints`, `prompt_arg_conventions`,
   `output_formats`, `reasoning`, and system-prompt metadata from
   `claudine/lib` instead of hard-coded per-provider matches in this crate.
4. **Spawn and stream.** Spawn the process with `tokio`. Construct a parser via
   `claudine::stream::create_semantic_parser(provider, sink, config)`, feed each
   stdout line to `feed_line`, and on exit call `finish(exit_code)` to obtain a
   `StreamExecutionSummary`.
5. **Build the response.** Use `summary.assistant_text` as the model output.
   For `InferenceOutput::Prose`, return `InferenceData::Prose(text)` after
   rejecting empty assistant text as `InvalidResponse`. For
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

- **Model selection.** `InferencePriority` is **not** mapped onto a model id in
  v1. Claudine's model catalog exposes `static_models` as a flat, untiered
  `&[&str]` and `ModelCatalogService` only validates that a model id exists — no
  cost/latency/quality tier metadata exists to map `Cost`/`Latency`/`Quality`/
  `Balanced` onto a specific model. Inventing a per-provider tier opinion in
  this crate would be unverifiable and provider-version-fragile. An explicit
  model override always wins; absent one, the provider uses its own default, the
  `--model` flag is omitted, and the reported `model` is filled from the stream
  summary when available, else `None`. Tiered priority mapping is deferred until
  the catalog carries tier metadata.
- **Reasoning effort.** Map `ReasoningEffort` onto the provider's typed
  `ReasoningSupport` (named levels, numeric budget, or binary toggle):
  proportional selection where `None` → off/omitted, `Low`/`Medium`/`High` span
  the provider's documented range. The resolved control is **emitted onto argv**
  where the provider exposes a verified non-interactive config-override (Codex
  `-c model_reasoning_effort="<level>"`). Where the non-interactive reasoning
  wiring is not reliably verifiable (Claude), the preference is recorded on the
  session plan but not emitted, since an unrecognized flag could fail an
  otherwise valid session. Providers without a reasoning control ignore the
  effort; this is approximation, not `Unsupported`.
- The adapter returns `Unsupported` only when it cannot perform the operation at
  all (e.g. no non-interactive entrypoint), never merely because a preference
  could not be honored exactly.

## Structured Output

`InferenceOutput::Structured { schema }` carries a JSON Schema (Draft 2020-12).

Agentic CLIs vary in native structured-output support through Claudine's
provider capability metadata. v1 uses a uniform, provider-independent
**prompt-and-parse** strategy with adapter-side validation:

1. **Validate the schema** itself before spawning anything; an unparseable or
   invalid schema is `InvalidRequest`.
2. **Augment the prompt** with an instruction to return a single JSON value
   conforming to the schema and nothing else. The adapter may still select a
   provider output format needed for Claudine's semantic stream parsing, but it
   must not rely on provider-native JSON mode as the only correctness guard in
   v1.
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

- **No tools.** No tool may take effect and no tool-derived output may ever be
  trusted or returned. The adapter must disable MCP injection and run each
  provider with its most restrictive available pre-turn control. Where a
  provider exposes a hard deny-all (Claude's `permissions.deny: ["*"]`), use it
  so no tool can even be attempted. Where a provider exposes no deny-all
  primitive (Codex: its execution-rules cannot express a catch-all and only gate
  sandbox-escape), use the tightest sandbox it does expose (`--sandbox
  read-only`, which blocks every write and network call) combined with the
  filesystem isolation below and the mandatory **post-hoc tool-call rejection**:
  any session that recorded a tool call, permission prompt, or user-input prompt
  fails with `InvalidResponse` instead of returning its output. The layered
  combination — restrictive pre-turn mode + isolation + post-hoc rejection —
  must guarantee that even a permitted read-only command attempt has nothing
  sensitive to act on and can never have its output returned.
- **Filesystem isolation.** Spawn the process with its working directory set to
  an ephemeral throwaway directory (e.g. a `tempfile` dir), not the consumer's
  repository or CWD. Use Claudine's shadow-home/provider-home isolation pattern
  where credentials or provider state are required, and copy or expose only the
  minimum provider config needed for authentication. Do not mount the
  consumer's repository, prompt source files, or ambient CWD into the session.
- **No environment leakage.** Pass an explicit allowlist of environment
  variables: provider authentication variables, model-selection variables when
  needed, path variables required to locate the provider binary, and minimal OS
  variables required for the process to start on the host. Do not forward the
  consumer's full environment, and redact all allowlisted secret names/values
  from diagnostics.
- **Prompt injection boundary.** Treat the request prompt as untrusted data.
  System/developer instructions that enforce JSON-only output, tool denial, and
  filesystem isolation must be delivered separately from the scraped prompt
  when the provider supports a system-prompt channel; otherwise prepend them as
  adapter-owned instructions before the user text with clear delimiters.
- **Provider gating.** A provider that cannot be run in a verified tool-free
  manner for untrusted input must be reported as `Unsupported` rather than run
  unsafely. Implementation must produce a support matrix in the crate README
  that lists each Claudine provider, its non-interactive entrypoint, the flags
  or policy controls used to disable tools/MCP/filesystem access, and whether
  `claudine-contract` enables or rejects it in v1.

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
| Session completed but tool calls, permission prompts, or user-input prompts were observed | `InvalidResponse` |
| Any other provider/session failure | `Provider` |

Stream `SemanticEvent::Error { kind, terminal, message, .. }` and the summary's
`is_error`/`error_kind`/`error_message`/`exit_code`, `rate_limit`,
`stderr_diagnostics`, `tool_calls`, `permission_prompts`, and
`user_input_prompts` drive this mapping. `retry_after` is set only when the
provider supplies a meaningful delay.

## Testing

Follow the repository testing tiers (`.claude/skills/rust-testing`).

**L1 (deterministic, default):**

- A session runner seam (trait or injected closure) so tests can supply canned
  provider stdout instead of spawning a real binary. The adapter must be
  testable without any agentic CLI installed.
- Session planning tests that assert provider entrypoints, prompt placement,
  output-format flags, model flags, reasoning flags, working directory, shadow
  HOME, MCP disabling, and environment allowlisting are built from Claudine's
  typed provider catalog rather than ad hoc provider matches.
- Prose request → `InferenceData::Prose` from a canned `assistant_text`.
- Structured request → validation success against a schema; and
  `InvalidResponse` for (a) invalid JSON, (b) schema violation, (c) prose
  returned when structure was requested.
- Profile mapping: assert each `InferencePriority`/`ReasoningEffort` resolves to
  the expected model/reasoning selection for a representative provider.
- Error mapping: each stable `InferenceErrorKind` is produced from the
  corresponding simulated session outcome.
- Security rejection: simulated summaries containing tool calls, permission
  prompts, or interactive user-input prompts fail the request instead of
  returning a successful inference response.
- Object-safety: store and call the adapter through `Arc<dyn InferenceAdapter>`.

**`real_` tier (gated, opt-in):**

- Against a genuinely installed and authenticated provider (e.g. Claude Code),
  prove a prose and a structured request complete end-to-end, that the session
  runs tool-free in an isolated directory with the documented environment
  allowlist, and that structured output validates against a real JSON Schema
  engine. These tests are skipped when the provider is unavailable.

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

None. The original questions about dependency direction, structured output, and
provider support are resolved normatively above: the adapter depends only on
`claudine/lib`, structured output uses prompt-and-parse plus adapter-side
validation in v1, and each provider is enabled only after implementation
documents a verified tool-free support matrix.

## Success Criteria

- `claudine/contract` is a workspace library crate with standard area `justfile`
  coverage and updated dependency docs and skills.
- `ClaudineInferenceAdapter` implements `InferenceAdapter`, is object-safe, and
  is injectable as `Arc<dyn InferenceAdapter>`.
- A prose and a structured request both succeed end-to-end against a fake
  session runner in L1 tests, with structured output validated against a JSON
  Schema engine owned by this crate.
- Every inference session runs tool-free, MCP-free, and filesystem-isolated;
  providers that cannot guarantee this are reported `Unsupported`, and the
  README documents the per-provider support matrix.
- `InferenceProfile`, error categories, metadata optionality, and structured
  validation are enforced as specified, with no change to `biscuit-contract`.
