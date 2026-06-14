---
ready: false
agent: codex
model: ""
---

# Review: Claudine Inference Adapter

## Findings

### High: Codex guard instructions are passed as an unquoted config value

The spec requires adapter-owned guard instructions to be delivered separately from the untrusted prompt when the provider has a system-prompt channel, and the README claims Codex gets the guard via `-c developer_instructions=...` ([README.md](../../../claudine/contract/README.md:72), [README.md](../../../claudine/contract/README.md:95)). The Codex provider catalog models that channel as `SystemPromptDelivery::ConfigKeyInline { flag: "-c", key: "developer_instructions" }` ([codex.rs](../../../claudine/lib/src/provider/codex.rs:325), [codex.rs](../../../claudine/lib/src/provider/codex.rs:331)), and the catalog documentation gives the required shape as `-c developer_instructions="..."` ([system_prompt.rs](../../../claudine/lib/src/provider/system_prompt.rs:41)).

`system_prompt_args`, however, emits the raw multi-sentence instruction as `developer_instructions={instruction}` with no TOML/string quoting or escaping ([session.rs](../../../claudine/contract/src/session.rs:275), [session.rs](../../../claudine/contract/src/session.rs:280)). The same planner already quotes Codex's reasoning override as `model_reasoning_effort="xhigh"` ([session.rs](../../../claudine/contract/src/session.rs:264), [session.rs](../../../claudine/contract/src/session.rs:267)), so the inconsistency is visible in the implementation. The existing Codex tests assert `exec --json`, `--sandbox read-only`, and reasoning, but they do not assert the exact `developer_instructions` argv value ([tests.rs](../../../claudine/contract/src/tests.rs:307), [tests.rs](../../../claudine/contract/src/tests.rs:329), [tests.rs](../../../claudine/contract/src/tests.rs:267)).

Impact: an enabled production provider may either fail before inference due to an invalid `-c` override or run without the adapter-owned guard in the intended system/developer channel. That weakens the prompt-injection boundary for untrusted scraped content and violates a normative security requirement for Codex.

Fix by serializing `ConfigKeyInline` values using the provider's config syntax before appending them to `-c` (for Codex, a TOML string literal), or by using a file-based config override for long guard text. Add an L1 Codex planning test that asserts the exact quoted/escaped `developer_instructions` argument and a real-provider regression when `CLAUDINE_CONTRACT_REAL=1` is available.

## Verification Level Assessment

| Requirement | Required level | Strongest present | Assessment |
|-------------|----------------|-------------------|------------|
| Object-safe `Arc<dyn InferenceAdapter>` | L1 | L1 | OK |
| Prose and structured responses through fake runner | L1 | L1 | OK |
| JSON Schema validation and invalid-response cases | L1 | L1 | OK |
| Claude session planning: stream output, guard prompt, strict MCP, shadow HOME, env allowlist | L1 + real_ | L1; real_ tests exist but were skipped in this run | Mostly OK, live provider not verified here |
| Codex session planning: `exec --json`, read-only sandbox, isolated CWD/HOME, env allowlist | L1 + real_ | L1; real_ tests exist but were skipped in this run | Partial: guard argv is not validated and appears malformed |
| Tool-free execution and post-hoc tool rejection for enabled providers | L1 + real_ per enabled provider | L1 for Claude/Codex; real_ tests skipped in this run | OK at L1; live provider behavior not verified here |
| Terminal rendering / keyboard behavior | L2/L3 only if specified | Not applicable | No terminal UX requirement in this spec |

## Tests Run

- `cargo test -p claudine-contract --color=never` passed: 36 unit tests, 3 `real_provider` tests, and 2 doctests.
- The `real_provider` tests returned early because `CLAUDINE_CONTRACT_REAL=1` was not set, so this run did not exercise Claude Code or Codex binaries.

## Ready For Production

No. `ready` is `false` because Codex is enabled as a production provider, but its adapter-owned guard instruction is not encoded in the config-override form that Codex's own provider catalog documents. That leaves a normative prompt-injection/security requirement either broken or unverified for one of the two enabled providers.
