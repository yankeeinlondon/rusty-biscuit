---
ready: false
agent: codex
model: ""
---

# Review: Claudine Inference Adapter

## Findings

### Critical: Codex is enabled but the planned argv does not request the JSONL stream Claudine parses

The support matrix and provider gate mark Codex as enabled for v1
([support.rs:19](../../../claudine/contract/src/support.rs:19),
[support.rs:31](../../../claudine/contract/src/support.rs:31)), so a consumer can
construct `ClaudineInferenceAdapter::new(Provider::Codex)` and expect a working
`InferenceAdapter`. The current plan builder, however, selects the first
`OutputFormat::Stream` entry for every provider
([session.rs:136](../../../claudine/contract/src/session.rs:136),
[session.rs:176](../../../claudine/contract/src/session.rs:176)). In the Codex
catalog that entry is `--output-schema schema-json`
([codex.rs:291](../../../claudine/lib/src/provider/codex.rs:291),
[codex.rs:296](../../../claudine/lib/src/provider/codex.rs:296)), while the
actual Codex JSONL event stream is the `OutputFormat::Json` entry using `--json`
([codex.rs:284](../../../claudine/lib/src/provider/codex.rs:284),
[codex.rs:287](../../../claudine/lib/src/provider/codex.rs:287)).

That means Codex sessions are planned as roughly
`codex exec --output-schema schema-json ...`, not the documented
`codex exec --json ...` path the README claims for Codex
([README.md:86](../../../claudine/contract/README.md:86)). `--output-schema`
takes a schema file/path for final-output validation; it is not the telemetry
stream switch. As a result, Claudine's Codex semantic parser will not receive
the JSONL events it expects, and the adapter is likely to return empty or
garbled output for an enabled provider.

The L1 tests miss this because they never assert Codex stream-output planning;
they only check Codex reasoning and sandbox flags
([tests.rs:237](../../../claudine/contract/src/tests.rs:237),
[tests.rs:277](../../../claudine/contract/src/tests.rs:277)). Add a Codex
planning test that requires `exec --json` and rejects `--output-schema` for the
normal prose/adapter stream path, or fix the provider catalog/selection logic so
the adapter chooses the parser-compatible JSONL output flag.

### High: Codex is marked tool-free with only a read-only sandbox, which does not satisfy the no-tools contract

The spec requires the session to perform no file reads/writes, shell execution,
web fetches, or other tool calls, and says providers that cannot be verified
tool-free must be reported as `Unsupported`
([spec.md:278](spec.md:278), [spec.md:305](spec.md:305)). For Codex, the adapter
only adds `--sandbox read-only`
([session.rs:216](../../../claudine/contract/src/session.rs:216),
[session.rs:223](../../../claudine/contract/src/session.rs:223)). The provider
metadata describes Codex sandbox modes and tool-deny controls separately:
`read-only` is a sandbox mode, while tool denial is modeled via rule decisions
and related policy controls
([codex.rs:592](../../../claudine/lib/src/provider/codex.rs:592),
[codex.rs:600](../../../claudine/lib/src/provider/codex.rs:600)).

`read-only` is not the same thing as tool-free. It can still allow read-only
command execution and filesystem reads, exactly the behavior the contract says
must not occur for untrusted scraped input. The README also narrows the Codex
claim to "blocks writes/network" rather than "blocks shell execution and file
reads" ([README.md:86](../../../claudine/contract/README.md:86)). Post-hoc
`summary.tool_calls` rejection is useful defense-in-depth, but it happens after
the provider had a chance to attempt a tool.

Either add pre-turn Codex controls that actually forbid command/tool execution,
or remove Codex from `V1_ENABLED` until that policy is implemented and verified.
The tests should assert the exact Codex deny controls, not just
`--sandbox read-only`.

### High: The required real-provider verification is still incomplete

The spec's `real_` tier requires proving that a genuinely installed provider
runs tool-free in an isolated directory with the documented environment
allowlist ([spec.md:376](spec.md:376)). The current real tests only exercise
Claude (`const PROVIDER: Provider = Provider::Claude`)
([real_provider.rs:25](../../../claudine/contract/tests/real_provider.rs:25)).
They do not run the other enabled provider, Codex, so the broken Codex argv and
the weak Codex tool-denial claim above are not caught.

The real isolation probe is also indirect: `real_tool_attempt_does_not_execute`
asks Claude to run `id` and asserts that `uid=` is not returned
([real_provider.rs:102](../../../claudine/contract/tests/real_provider.rs:102),
[real_provider.rs:126](../../../claudine/contract/tests/real_provider.rs:126)).
That is useful, but it does not verify the spawned process's actual `HOME`, CWD,
environment allowlist, or visibility of real provider-home files. Those are
explicit user-observable security requirements in this feature because the
adapter runs untrusted prompt content through an agentic CLI.

Verification level by requirement:

| Requirement | Required level | Strongest present | Result |
|---|---:|---:|---|
| Fake session runner seam and canned provider stdout | L1 | L1 | OK |
| Session planning from typed provider catalog | L1 | L1 | Partial; Codex output planning is wrong |
| Prose and structured response handling | L1 | L1 | OK |
| Structured JSON Schema validation failures | L1 | L1 | OK |
| Reasoning mapping | L1 | L1 | OK for resolved/argv-supported controls |
| Priority model mapping omitted in v1 per spec | L1 | L1 | OK |
| Stable error-kind mapping | L1 | L1 | Mostly OK |
| Reject observed tool calls/prompts | L1 | L1 | OK as post-hoc rejection |
| End-to-end installed provider behavior | `real_` | `real_` gated, Claude only | Partial |
| Tool-free isolated directory and env allowlist with every enabled real provider | `real_` | indirect Claude-only probe | Gap |

This feature has no terminal emulator or keyboard UX requirement, so Level 2
and Level 3 are not relevant. The mismatch is in the `real_` tier: the
security guarantees are not verified for every enabled provider and are not
inspected directly enough to prove the environment contract.

## Verification

- `cargo metadata --no-deps --format-version 1 --color=never` completed and
  shows `claudine-contract` as a workspace member.
- `cargo test -p claudine-contract --color=never` passed: 31 unit tests, 3
  `real_provider` tests, and 2 doctests.

The default `real_provider` run should not be read as proof of real-provider
execution: the tests return early unless `CLAUDINE_CONTRACT_REAL=1` is set and
the provider binary is on `PATH`, so they reported `ok` without spawning a
provider in this review run.

## Ready For Production

No. `ready` is `false` because an enabled provider path (Codex) is not planned
with the parser-compatible JSONL output mode, Codex is not yet proven
tool-free under the spec's no-tools contract, and the required real-provider
security verification remains incomplete.
