---
ready: true
agent: codex
model: ""
---

# Review: Claudine Inference Adapter

## Findings

No blocking findings.

The implementation now matches the normative security shape in the spec: only curated providers are enabled, sessions are planned through Claudine's provider catalog, the child runs with a throwaway working directory and shadow `HOME`, the process environment is cleared and rebuilt from an allowlist, Claude gets deny-all policy plus strict empty MCP config, Codex gets `--sandbox read-only`, and both providers are protected by post-hoc rejection of tool calls, permission prompts, or user-input prompts.

The prior Codex issues are addressed: the plan selects `codex exec --json` rather than `--output-schema`, command-execution events are rejected before output is trusted, and Codex developer instructions are emitted as a TOML-quoted `-c developer_instructions=...` override.

## Non-Blocking Notes

- The spec named `claudine/docs/dependencies.md`, which does not exist in this package area. The implementation instead added `claudine/contract/docs/dependencies.md` and updated the root `docs/dependencies.md`. That satisfies the dependency-documentation intent, but the root generated dependency index still has a stale `jsonschema` catalog row showing `v0.28` while the new crate uses `0.42`. Refreshing generated dependency docs would clean this up.
- I did not run the opt-in live-provider suite with `CLAUDINE_CONTRACT_REAL=1`, so the real provider calls were not exercised in this review environment. The gated tests are present and cover every v1-enabled provider when installed/authenticated.

## Verification Level Matrix

| Requirement | Strongest verification present | Review |
| --- | --- | --- |
| `claudine-contract` workspace crate implements `InferenceAdapter` and can be used as `Arc<dyn InferenceAdapter>` | L1 unit + doctest | OK |
| Prose request returns final assistant text from Claudine semantic parser | L1 canned Claude and Codex streams | OK |
| Structured request validates JSON Schema and rejects invalid JSON/schema/prose/multiple JSON values | L1 unit tests with real `jsonschema` validator | OK |
| Provider session planning uses non-interactive entrypoint, parser-compatible output format, prompt placement, model/reasoning controls | L1 plan assertions | OK |
| Filesystem isolation and environment allowlist | L1 plan/env/shadow-home tests for Claude and Codex | OK |
| Tool/MCP denial controls for enabled providers | L1 plan tests; opt-in `real_` tool-attempt test | OK |
| Post-hoc rejection for tool calls, permission prompts, and user-input prompts | L1 security tests, including Codex `command_execution` | OK |
| Error mapping to stable `InferenceErrorKind` values | L1 simulated session outcomes | OK |
| Live installed/authenticated provider completes prose and structured requests | `real_` tier, env-gated | OK; not run locally |
| Terminal rendering, keyboard input, hotkeys, paste, mouse, scroll behavior | Not applicable | No L2/L3 requirement |

## Verification Run

- `sniff repo` completed.
- `cargo metadata --no-deps --format-version 1` includes `claudine-contract` as a workspace member.
- `cargo test -p claudine-contract --color=never` passed: 40 unit tests, 4 env-gated `real_provider` tests, and 2 doctests. The `real_provider` tests returned immediately because `CLAUDINE_CONTRACT_REAL=1` was not set.

## Production Readiness

Ready for production. The feature has no remaining blocking functionality or test-level gaps against the spec.
