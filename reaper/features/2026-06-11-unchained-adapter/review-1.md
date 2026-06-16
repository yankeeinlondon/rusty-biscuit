---
ready: false
agent: codex
model: ""
---

# Review 1 — Unchained AI Inference Adapter

## Verdict

Not ready for production.

The implementation compiles and the narrow L1 tests pass, but several spec
requirements are either not implemented or only verified partially. The largest
production risk is that profile-derived generation parameters, including
reasoning controls, are computed and then dropped before the real provider call.

## Findings

### High — Reasoning and most generation parameters are never applied to real provider calls

Spec requirements:

- `InferenceProfile.reasoning` should become provider reasoning controls when
  model metadata advertises a known parameter.
- Metadata defaults and caller/profile overrides should be applied to the
  `rig-core` completion request.

Implementation:

- `parameters_for_reasoning` builds `ResolvedParameters.extra` for
  `reasoning_effort` / `thinking` in
  `unchained-ai/contract/src/profile.rs:66`.
- The adapter merges those parameters into the `CompletionRequest` in
  `unchained-ai/contract/src/adapter.rs:137`.
- The production Rig path only applies `temperature` and `max_tokens` in
  `unchained-ai/lib/src/execution/mod.rs:396`. It ignores `top_p`, `top_k`,
  `frequency_penalty`, `presence_penalty`, and `extra`, so OpenAI
  `reasoning_effort` and Anthropic `thinking` never reach the provider.

Strongest verification present: L1 only, and it does not assert that the real
backend receives/applies the computed parameter set. The fake backend ignores
`BackendRequest`, so these tests cannot catch the drop.

Impact: requests asking for `ReasoningEffort::High` may select a reasoning-ish
model, but provider-level reasoning settings are silently lost. Default model
parameters are also only partially honored.

### High — The adapter does not use the lib structured-output execution surface

Spec requirement:

- Structured requests should execute through the `unchained-ai/lib` structured
  path by passing the schema as data/instructions; schema validation remains in
  the adapter.

Implementation:

- The adapter augments the prompt itself at
  `unchained-ai/contract/src/adapter.rs:116`, but then constructs
  `CompletionRequest { schema: None }` at
  `unchained-ai/contract/src/adapter.rs:144`.
- Because `schema` is always `None`, `complete()` returns
  `CompletionOutput::Text` and the adapter reparses JSON in
  `unchained-ai/contract/src/adapter.rs:174`.
- The lib structured path in `unchained-ai/lib/src/execution/mod.rs:185` is
  therefore not exercised by the contract adapter.

Strongest verification present: L1 tests cover adapter-side parsing/validation
and separate lib-side structured parsing, but no test verifies that structured
adapter calls flow through the shared lib structured request shape.

Impact: the adapter and native `Prompt` structured paths can drift, defeating
one of the spec's main goals: one reusable single-turn structured execution
surface.

### High — Required L1 coverage for profile mapping is incomplete

Spec requirement:

- Assert each `InferencePriority` × `ReasoningEffort` combination resolves to
  the expected `ModelCapability` and expected `ProviderModel` under an injected
  env view.

Implementation/tests:

- Current tests cover only selected combinations in
  `unchained-ai/contract/src/profile.rs:126` and one adapter-level quality case
  in `unchained-ai/contract/src/adapter.rs:362`.
- They do not cover all 16 priority/reasoning combinations, and they do not
  pair each combination with expected concrete model resolution.

Strongest verification present: partial L1.

Impact: this is a spec-mandated behavior matrix. Regressions in the untested
profile tiers can change model selection silently.

### Medium — `retry_after` is never populated for rate limits or overloads

Spec requirement:

- Provider 429/rate-limit and 5xx/overload errors should map to stable
  `InferenceErrorKind` categories and include `retry_after` when the provider
  supplies one.

Implementation:

- `InferenceError` supports `retry_after` via
  `biscuit-contract/lib/src/inference.rs:167`.
- The adapter error mapper always constructs errors through
  `InferenceError::new` in `unchained-ai/contract/src/error.rs:11` and never
  calls `with_retry_after`.
- The current `ProviderError` variants in
  `unchained-ai/lib/src/rigging/providers/provider_errors.rs:5` also do not
  carry retry-after metadata.

Strongest verification present: L1 verifies category mapping for simulated
rate-limit text, but not retry-after propagation.

Impact: callers cannot make contract-level retry decisions even when a provider
response includes retry guidance.

### Medium — `Unsupported` is part of the specified mapping but is never produced

Spec requirement:

- "Requested capability/modality unsupported by every resolvable model" maps to
  `InferenceErrorKind::Unsupported`.

Implementation:

- `resolve_model` only reports `ProviderError::NoRunnableModel` when no stack
  entry is runnable in `unchained-ai/lib/src/models/selection.rs:75`.
- The adapter maps that to `Unavailable` in
  `unchained-ai/contract/src/error.rs:42`.
- No `InferenceErrorKind::Unsupported` mapping exists in
  `unchained-ai/contract/src/error.rs`.

Strongest verification present: none for this requirement.

Impact: unsupported capability/modality and temporary lack of runnable
providers are indistinguishable to callers, even though the contract exposes
separate stable categories.

### Medium — Real-provider tests can report success without exercising a provider

Spec requirement:

- The `real_` tier should prove prose and structured requests complete
  end-to-end against a real provider when credentials are configured, and skip
  cleanly when they are not.

Implementation:

- `real_provider.rs` skips when `UNCHAINED_AI_CONTRACT_REAL` is unset and also
  skips when no credentials are present in
  `unchained-ai/contract/tests/real_provider.rs:109`.
- The default `cargo test -p unchained-ai-contract` run reports both real tests
  as `ok` in the no-credential path; this is a clean skip behavior but looks
  like executed coverage in cargo output.

Strongest verification present: `real_` tier exists and skips cleanly here; no
evidence in this review run that it exercised a real provider.

Impact: maintainers can mistake a default green test run for real-provider
validation. Consider printing a clearer skip marker, using the repo's
resource-gating helper where practical, or documenting in the review/CI result
whether credentials were present.

## Verification Summary

- `cargo check -p unchained-ai-contract --color=never` passed.
- `cargo test -p unchained-ai-contract --color=never` passed: 35 unit tests, 2
  real-provider tests in skip path, 2 doctests.
- `cargo test -p unchained-ai --lib --color=never` passed: 201 tests.

User-observable terminal behavior levels: not applicable. This feature has no
terminal rendering, keyboard, paste, IME, mouse, or scrolling UX requirements.
The relevant verification levels are L1 fake-backend tests and the opt-in
`real_` provider tier.

## Notes

The implementation has a solid shape: the adapter crate exists, object-safety is
covered, schema validation is adapter-owned, the synchronous Prompt bridge avoids
recursive runtime blocking, and environment-driven model resolution uses an
injectable view. Closing the findings above should focus on preserving that
shape while wiring the computed request data all the way into the production
Rig call and completing the required behavior matrix tests.
