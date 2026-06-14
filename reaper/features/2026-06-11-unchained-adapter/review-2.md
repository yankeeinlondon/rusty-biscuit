---
ready: false
agent: codex
model: ""
---

# Review: Unchained AI Inference Adapter

## Findings

### High: Network and local-provider connection failures can surface as `Provider` instead of `Unavailable`

The spec requires provider 5xx, overload, and network-unreachable failures to map to `InferenceErrorKind::Unavailable` ([spec.md](spec.md:296)). This is especially important for the default no-credentials path: the capability resolver intentionally treats local providers as runnable without credentials, so a profile-driven request can resolve to Ollama even when no Ollama server is listening ([selection.rs](../../../unchained-ai/lib/src/models/selection.rs:96), [selection.rs](../../../unchained-ai/lib/src/models/selection.rs:100)).

The production rig backend wraps every `agent.prompt(...)` failure as `ProviderError::ExecutionFailed` with provider `"LLM"` ([mod.rs](../../../unchained-ai/lib/src/execution/mod.rs:491), [mod.rs](../../../unchained-ai/lib/src/execution/mod.rs:494)). The contract classifier handles timeout, rate-limit, auth, and a few overload/5xx words in `ExecutionFailed`, but it does not recognize common network strings such as `connection refused`, `connect`, `dns`, `network`, or `unreachable` on that path ([error.rs](../../../unchained-ai/contract/src/error.rs:165), [error.rs](../../../unchained-ai/contract/src/error.rs:198)). Those strings are only classified for `ProviderError::HttpError` ([error.rs](../../../unchained-ai/contract/src/error.rs:139), [error.rs](../../../unchained-ai/contract/src/error.rs:147)).

Impact: a normal profile-driven request on a machine without provider credentials and without a running local Ollama endpoint can report a generic provider failure instead of the stable `Unavailable` category the contract promises. Fix by classifying network/unreachable wording in `classify_execution_failed` too, or by preserving a typed transport error from the execution layer instead of flattening rig errors into strings. Add an L1 test for an `ExecutionFailed` reason containing a realistic connection-refused / DNS failure.

### High: Structured-output prompting incorrectly narrows JSON Schema to JSON objects

The contract and this spec allow structured output to be any JSON Schema value, and failures for non-conforming values should be validation failures, not prompt-shape drift ([spec.md](spec.md:266), [spec.md](spec.md:277)). The execution surface parses either `{...}` or `[...]` and documents "single JSON value" extraction ([mod.rs](../../../unchained-ai/lib/src/execution/mod.rs:217), [mod.rs](../../../unchained-ai/lib/src/execution/mod.rs:221)), but the structured prompt tells the model to return "a single JSON object" ([mod.rs](../../../unchained-ai/lib/src/execution/mod.rs:206), [mod.rs](../../../unchained-ai/lib/src/execution/mod.rs:209)).

Impact: callers using valid non-object schemas, such as an array schema or scalar enum schema, receive instructions that contradict their schema. The adapter may still validate an array if a provider happens to ignore the word "object", but production behavior is biased toward the wrong response shape. Change the instruction to "single JSON value" and add L1 structured tests for at least an array schema and a scalar schema so this does not regress.

### Medium: The area-level `test-real` recipe does not run the contract real-provider tests

The spec requires opt-in `real_` tests for prose and structured end-to-end calls against real provider credentials ([spec.md](spec.md:339)). Those tests exist under `unchained-ai/contract/tests/real_provider.rs`, and the contract-local `just test-real` runs them with `UNCHAINED_AI_CONTRACT_REAL=1` ([justfile](../../../unchained-ai/contract/justfile:68), [justfile](../../../unchained-ai/contract/justfile:71)).

However, the package-area `unchained-ai/justfile` omits `unchained-ai-contract` from `test-real` while including the lib, CLI, and gen crates ([justfile](../../../unchained-ai/justfile:82), [justfile](../../../unchained-ai/justfile:84)). That means `just -f unchained-ai/justfile test-real` does not exercise the new adapter's real-provider tier. Add the contract package to this recipe so area-level verification covers the feature.

## Verification Summary

| Requirement | Strongest verification observed | Status |
| --- | --- | --- |
| `unchained-ai-contract` implements object-safe `Arc<dyn InferenceAdapter>` | L1 unit + doctest | OK |
| Profile priority/reasoning maps to `ModelCapability` and concrete model with injected env | L1 unit | OK |
| Local provider is selectable without credentials | L1 unit | OK |
| Prose request succeeds through fake completion seam | L1 unit | OK |
| Structured request goes through lib structured path and validates schema | L1 unit | OK for object schemas; gap for non-object JSON Schema |
| Invalid schema, invalid JSON, schema violation, and variant mismatch map correctly | L1 unit | Mostly OK; network-unreachable mapping gap remains |
| `Prompt::execute()` / `execute_readonly()` delegate through `complete_blocking()` and avoid nested-runtime panic | L1 unit | OK |
| Real provider prose and structured requests | `real_` integration tests exist, env-gated | Not run in this review; area recipe omission noted |
| Terminal UX behavior requiring Level 2/3 | Not applicable | OK |

## Commands Run

- `sniff repo`
- `cargo test -p unchained-ai-contract --color=never`
- `cargo test -p unchained-ai --lib execution::tests --color=never`
- `cargo test -p unchained-ai --lib primitives::atomic::prompt::tests --color=never`
- `cargo test -p unchained-ai --lib models::selection::tests --color=never`

I also attempted one invalid `cargo test` invocation with two test filters; Cargo rejected it before running tests, and I reran the intended filters separately.

## Production Readiness

Not ready. The adapter shape and most L1 coverage are in place, but the error-category contract is still wrong for an important local/network failure path, and structured output still prompts for the wrong JSON shape for valid non-object schemas.
