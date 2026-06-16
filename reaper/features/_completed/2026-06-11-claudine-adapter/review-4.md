---
ready: false
agent: codex
model: ""
---

# Review: Claudine Inference Adapter

## Findings

### Critical: Codex is marked enabled, but the planned argv does not request the JSONL stream the Codex parser expects

`provider_support(Provider::Codex)` returns `Enabled` because Codex is in the v1 allowlist ([support.rs:19](../../../claudine/contract/src/support.rs:19), [support.rs:39](../../../claudine/contract/src/support.rs:39)). The session planner then always selects the provider catalog entry whose universal format is `OutputFormat::Stream` ([session.rs:176](../../../claudine/contract/src/session.rs:176), [session.rs:181](../../../claudine/contract/src/session.rs:181)). For Codex, that catalog entry is `--output-schema schema-json`, not `--json` ([codex.rs:291](../../../claudine/lib/src/provider/codex.rs:291), [codex.rs:294](../../../claudine/lib/src/provider/codex.rs:294), [codex.rs:296](../../../claudine/lib/src/provider/codex.rs:296)).

That does not match the parser or the README. Claudine's Codex semantic parser is explicitly for `codex exec --json` JSONL ([codex.rs:1](../../../claudine/lib/src/stream/providers/codex.rs:1)), and the adapter README documents Codex's enabled entrypoint as `codex exec --json` ([README.md:86](../../../claudine/contract/README.md:86)). The existing Codex tests check reasoning and sandbox flags only; they do not assert `--json` or feed a canned Codex JSONL transcript through `ClaudineInferenceAdapter` ([tests.rs:237](../../../claudine/contract/src/tests.rs:237), [tests.rs:277](../../../claudine/contract/src/tests.rs:277)).

This means a consumer can construct `ClaudineInferenceAdapter::new(Provider::Codex)` and get a supposedly supported adapter whose command shape is not the parser-compatible command shape. Fix by either selecting Codex's JSONL output flag for the adapter stream path, or reject Codex as `Unsupported` until the catalog can distinguish parser streams from provider-native structured-output schema mode. Add an L1 Codex plan test that asserts `exec --json`, plus an L1 adapter test with canned Codex JSONL assistant text.

### High: Codex is enabled without a verified pre-execution no-tools control

The spec requires tool-capable providers to run "tool-free" and says attempted tool use should be blocked rather than executed. It also says providers that cannot be run in a verified tool-free manner must be `Unsupported`. Codex is enabled in v1 ([support.rs:19](../../../claudine/contract/src/support.rs:19)), but the only Codex execution control emitted by the adapter is `--sandbox read-only` ([session.rs:216](../../../claudine/contract/src/session.rs:216), [session.rs:223](../../../claudine/contract/src/session.rs:223)).

`read-only` is a sandbox mode, not a tool-denial policy. Claudine's Codex capability metadata separately lists sandbox modes and tool allow/deny controls via rules/trust configuration ([codex.rs:592](../../../claudine/lib/src/provider/codex.rs:592), [codex.rs:595](../../../claudine/lib/src/provider/codex.rs:595), [codex.rs:596](../../../claudine/lib/src/provider/codex.rs:596), [codex.rs:600](../../../claudine/lib/src/provider/codex.rs:600)). The adapter does not write a Codex rules/config file into the shadow home, and `home.rs` confirms Codex writes no policy file ([home.rs:31](../../../claudine/contract/src/home.rs:31), [home.rs:40](../../../claudine/contract/src/home.rs:40), [home.rs:49](../../../claudine/contract/src/home.rs:49)). Post-hoc rejection catches observed tool calls after the fact, but the parser shows `command_execution` is a real tool event representing shell execution ([codex.rs:206](../../../claudine/lib/src/stream/protocol/codex.rs:206), [codex.rs:573](../../../claudine/lib/src/stream/protocol/codex.rs:573), [codex.rs:580](../../../claudine/lib/src/stream/protocol/codex.rs:580)).

The real-provider tier does not compensate for this because it only exercises Claude (`PROVIDER: Provider = Provider::Claude`) ([real_provider.rs:25](../../../claudine/contract/tests/real_provider.rs:25), [real_provider.rs:27](../../../claudine/contract/tests/real_provider.rs:27)). Either implement a real Codex deny-all tool policy before the model turn starts and verify it, or remove Codex from `V1_ENABLED`.

## Verification Level Assessment

| Requirement | Required level | Strongest present | Assessment |
|-------------|----------------|-------------------|------------|
| Object-safe `Arc<dyn InferenceAdapter>` | L1 | L1 | OK |
| Prose and structured responses through fake runner | L1 | L1 | OK |
| JSON Schema validation and invalid-response cases | L1 | L1 | OK |
| Claude session planning: stream output, guard prompt, strict MCP, shadow HOME, env allowlist | L1 + real_ | L1, real_ test exists but was skipped in this run | Mostly OK; not externally verified here |
| Codex enabled provider works end-to-end | L1 + real_ | Partial L1 planning only | Gap |
| Tool-free execution for enabled providers | L1 + real_ per enabled provider | Claude only in real_ tests; Codex only has `--sandbox read-only` planning | Gap |
| Terminal rendering / keyboard behavior | L2/L3 only if specified | Not applicable | No terminal UX requirement in this spec |

## Tests Run

- `cargo test -p claudine-contract --color=never` passed: 31 unit tests, 3 `real_provider` tests, 2 doctests. The `real_provider` tests returned early because `CLAUDINE_CONTRACT_REAL=1` was not set, so this did not exercise a real provider.
- `cargo test -p claudine-contract --test real_provider --color=never` passed, but also returned early for the same env gate.

## Ready For Production

No. `ready` is `false` because Codex is exposed as a supported production provider while its planned argv is not parser-compatible and its tool-denial story does not meet the spec's pre-execution no-tools requirement.
