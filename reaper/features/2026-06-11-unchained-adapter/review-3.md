---
ready: true
agent: codex
model: ""
---

# Review 3 — Unchained AI Inference Adapter

## Verdict

Ready for production.

I found no remaining production-blocking gaps against the spec. The prior review items have been addressed: structured adapter requests now pass the schema through the shared `unchained-ai/lib` structured path, generated/default/reasoning parameters are forwarded to the backend request and rig `additional_params`, the full priority/reasoning matrix is pinned in L1 tests, local/network failure wording is mapped to `Unavailable`, retry-after hints are covered, and the area-level `test-real` recipe includes `unchained-ai-contract`.

## Findings

No findings.

## Verification Matrix

| Requirement | Strongest verification observed | Status |
| --- | --- | --- |
| `unchained-ai-contract` is a workspace crate implementing object-safe `Arc<dyn InferenceAdapter>` | L1 unit + doctest coverage in `unchained-ai-contract` | OK |
| Empty prompts and invalid schemas fail before provider execution | L1 unit tests in adapter/structured modules | OK |
| Prose requests succeed through a fake completion seam | L1 unit test | OK |
| Structured requests pass schema to the shared lib execution path, parse JSON, and validate with adapter-owned JSON Schema | L1 unit tests, including adapter flow assertion | OK |
| Non-object structured schemas are treated as JSON values, not forced objects | L1 unit tests for array/scalar structured output and prompt wording | OK |
| Profile priority/reasoning maps to `ModelCapability` and concrete `ProviderModel` through injected env | L1 full 16-case matrix plus resolver tests for credential and local providers | OK |
| Reasoning/default/sampling parameters reach the backend request and rig `additional_params` | L1 execution tests | OK |
| Error mapping covers stable categories, network/local-provider failures, and retry-after hints | L1 error-mapper tests | OK |
| `Prompt::execute()` / `execute_readonly()` delegate through `complete_blocking()` and avoid nested-runtime panic | L1 unit tests inside and outside Tokio runtime | OK |
| Real provider prose and structured requests complete end-to-end when credentials are present | `real_` integration tests exist and are env-gated | OK; not exercised in this review environment |
| Terminal rendering/input behavior requiring Level 2 or Level 3 | Not applicable; this feature has no terminal UX requirement | OK |

## Commands Run

- `cargo test --color=never -p unchained-ai-contract --lib --tests`
- `cargo test --color=never -p unchained-ai --lib`

Both passed. The `real_provider` tests ran in their default skip-clean path because `UNCHAINED_AI_CONTRACT_REAL=1` was not set in this review environment.

## Residual Risk

The direct provider path still depends on provider-specific behavior in `rig-core` and live provider APIs. The gated `real_` tier is the right verification level for that surface, but it should be run in an environment with at least one real provider credential before release sign-off.
