---
ready: false
agent: codex
model: ""
---

# Review: Claudine Inference Adapter

## Findings

### Critical: The child process still gets the caller's real `HOME`, so filesystem isolation is incomplete

The spec requires the adapter to use Claudine's shadow-home/provider-home
isolation pattern when provider state is needed, copy or expose only the
minimum authentication config, and not mount the consumer's ambient environment
or repository context into the session ([spec.md:284](spec.md:284)). The
implementation creates an isolated CWD, but `build_env` includes `HOME` in the
baseline allowlist and copies it from the caller's process environment into the
child ([session.rs:70](../../../claudine/contract/src/session.rs:70),
[session.rs:80](../../../claudine/contract/src/session.rs:80)). The production
runner then clears the environment and restores those allowlisted values
verbatim ([session.rs:241](../../../claudine/contract/src/session.rs:241)).

For enabled providers such as Claude and Codex, the real `HOME` is also where
provider config, memory, MCP config, and session state live. The provider
catalog documents these reads, including Claude's `~/.claude/settings.json`,
`~/.claude.json`, and `~/.claude/CLAUDE.md`, and Codex's
`~/.codex/AGENTS.md` / `~/.codex/config.toml` surfaces. With the current plan,
untrusted scraped prompt text is run by an agentic CLI that can still see the
user's normal provider home. That violates the isolation requirement even if
the working directory is temporary.

This is a release blocker. The adapter should create a shadow home/provider
home, set `HOME` to that isolated tree, and copy only the minimum auth material
needed for the selected provider. The L1 planning tests should assert that
`HOME` points at the shadow location and not the caller's home.

### Critical: Tool-free execution is asserted but not enforced with provider policy controls

The spec says the session "must not perform file reads/writes, shell execution,
web fetches, or other tool calls" and must use provider flags/modes and
Claudine permission/protect policy in the most restrictive available mode where
available ([spec.md:278](spec.md:278)). The implementation currently relies on
a system/developer guard instruction and then rejects sessions after Claudine's
semantic parser reports tool calls or prompts
([adapter.rs:152](../../../claudine/contract/src/adapter.rs:152),
[adapter.rs:166](../../../claudine/contract/src/adapter.rs:166)). The session
plan does not add any provider permission/sandbox/tool-deny flags; the L1 plan
test only asserts that no argument contains `mcp`
([tests.rs:122](../../../claudine/contract/src/tests.rs:122)).

That is not the same guarantee. Prompt instructions are not a tool sandbox, and
post-hoc stream rejection only catches events the parser observes after the
provider has already had a chance to act. This is especially risky because the
support matrix marks Claude and Codex as enabled for untrusted input while
their provider catalog exposes agent/tool permission surfaces, including Codex
permission modes and sandbox modes
([codex.rs:592](../../../claudine/lib/src/provider/codex.rs:592)).

Either enable only providers with an actual verified text-only mode, or include
the provider-specific restrictive policy/config needed to deny tools before the
model turn starts. The support matrix and tests should name and assert those
controls, not just the guard prompt.

### High: `InferenceProfile` preferences are intentionally not implemented as specified

The spec requires `InferencePriority` to map onto provider model selection via
Claudine's model catalog/static models and requires providers with reasoning
controls to receive a proportional reasoning setting
([spec.md:225](spec.md:225), [spec.md:236](spec.md:236)). The implementation
does neither for actual sessions: model priority is ignored unless an explicit
model override is supplied, and resolved reasoning is recorded on
`SessionPlan` but never emitted onto argv
([profile.rs:1](../../../claudine/contract/src/profile.rs:1),
[session.rs:144](../../../claudine/contract/src/session.rs:144)).

The tests lock this gap in rather than catching it:
`priority_does_not_fabricate_a_model` asserts that every priority omits a model
flag, and `build_plan_records_reasoning_without_emitting_it` asserts that the
reasoning setting is not sent
([tests.rs:316](../../../claudine/contract/src/tests.rs:316),
[tests.rs:303](../../../claudine/contract/src/tests.rs:303)). That contradicts
the normative profile-mapping section of the spec. If v1 cannot safely map
priority/reasoning for these providers, the spec should be amended before the
implementation is considered complete; otherwise the adapter needs to apply the
requested selections and test the resulting session plan.

### High: The required `real_` tier does not verify isolation or environment allowlisting

The spec's `real_` tier requires proving that an installed/authenticated
provider runs tool-free in an isolated directory with the documented
environment allowlist, in addition to end-to-end prose and structured requests
([spec.md:376](spec.md:376)). The current real tests only instantiate the
public adapter and assert that prose/structured calls return data
([real_provider.rs:51](../../../claudine/contract/tests/real_provider.rs:51),
[real_provider.rs:70](../../../claudine/contract/tests/real_provider.rs:70)).
They do not inspect the spawned process environment, `HOME`, working directory,
provider config visibility, or tool-denial behavior.

Verification level by requirement:

| Requirement | Required level | Strongest present | Result |
|---|---:|---:|---|
| Fake session seam and canned stdout | L1 | L1 | OK |
| Session planning from typed provider catalog | L1 | L1 | Partial; missing policy/shadow-home assertions |
| Prose and structured response handling | L1 | L1 | OK |
| Structured JSON Schema validation failures | L1 | L1 | OK |
| Profile priority and reasoning mapping | L1 | L1 | Gap; tests assert non-implementation |
| Stable error-kind mapping | L1 | L1 | Mostly OK |
| Reject observed tool calls/prompts | L1 | L1 | OK as post-hoc rejection only |
| Object-safe `Arc<dyn InferenceAdapter>` usage | L1 | L1 | OK |
| End-to-end installed provider behavior | `real_` | `real_` gated | Partial |
| Tool-free isolated directory and env allowlist with real provider | `real_` | none | Gap |

This feature has no terminal emulator or keyboard UX requirement, so Level 2
and Level 3 are not the relevant tiers. The mismatch is that the `real_` tier
does not exercise the real-provider security guarantees that the spec requires.

### Medium: The process runner can deadlock on large stderr output

`TokioSessionRunner` reads stdout to EOF before it starts draining stderr
([session.rs:267](../../../claudine/contract/src/session.rs:267),
[session.rs:281](../../../claudine/contract/src/session.rs:281)). If a provider
writes enough stderr to fill the pipe while stdout remains open, the child can
block writing stderr, stdout never reaches EOF, and `infer` hangs. Agentic CLIs
commonly emit diagnostics, progress, and auth errors on stderr, so this is not
theoretical.

Read stdout and stderr concurrently while also awaiting process exit, or use a
single helper that drains both pipes without ordering dependency. Add an L1
runner test with a small fixture process/script that writes enough stderr to
force the old behavior to hang under a timeout.

## Verification

- `cargo metadata --no-deps --format-version 1` passes.
- `cargo test -p claudine-contract --color=never` passes: 25 unit tests, 2
  `real_provider` tests, and 2 doctests.

Note: the `real_provider` tests return early unless
`CLAUDINE_CONTRACT_REAL=1` is set and the provider binary is on `PATH`, so the
successful default run should not be read as proof that real provider execution
occurred.

## Ready For Production

No. `ready` is `false` because the adapter still fails the spec's security
isolation contract, does not implement profile mapping as specified, and lacks
the required real-provider verification for tool-free isolated execution.
