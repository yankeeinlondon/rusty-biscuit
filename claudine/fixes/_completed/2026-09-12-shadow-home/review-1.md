---
$schema: feature-review.yaml
ready: false
human_review: false
reviewed_by: codex/gpt-5.6-sol
created: 2026-09-16T05:07:28-07:00
spec: 2026-09-12-shadow-home/spec.md
implemented: true
description: A **fix** review of `2026-09-12-shadow-home/spec.md`
fix: 2026-09-12-shadow-home/review-1.md
next: 2026-09-12-shadow-home/review-2.md
findings:
    - "[high] The L1 overlay suite inherits host provider selectors and currently fails five contracts"
    - "[high] Reused overlay roots retain excluded data and race between concurrent launches"
    - "[high] The native-Windows L2 test is designed to fail before exercising the feature"
    - "[medium] Kilo advertises inline MCP delivery that is not wired into the runtime"
---

# Review 1: Preserve Provider Overlays Without Replacing the User Home

## Verdict

The central direction is sound. Provider configuration is redirected through
typed provider-owned selectors, global home variables are preserved, failures
are typed and occur before provider spawn, OpenCode MCP remains inline, and the
Unix interactive path has real-terminal evidence. The planner/materializer
split is also easier to reason about than the former collection of shadow-home
booleans.

The fix is **not ready for production**. Its dedicated Level 1 real-binary test
target currently has five failures because the shared fixture inherits
`CODEX_SQLITE_HOME` from the reviewing Claudine session. More importantly, the
persistent overlay path is neither reconciled to the current plan nor isolated
per launch: a later `--repo` run can retain resources that should be hidden,
and concurrent launches can overwrite or remove each other's repo prompts and
session-scoped MCP configuration. The required native-Windows Level 2 test is
also intentionally structured to fail before launch on an ordinary Windows
profile, so it cannot provide its claimed evidence.

Human review is not required. The specification already decides the relevant
contracts; the remaining work is deterministic implementation and test-harness
repair.

## Findings

### 1. The L1 overlay suite inherits host provider selectors and currently fails five contracts (high)

`CliProcessFixture` scrubs Claudine/Playa variables, Git plumbing, rendering
inputs, generic wrapper variables, and provider model variables, but it does
not scrub provider overlay or external-state selectors
(`cli/tests/common/mod.rs:618-645`). In this review environment the parent has
`CODEX_SQLITE_HOME=/Users/ken/.codex`, so tests that intend to exercise the
default pre-overlay Codex root inherit that host value.

A fresh build of the dedicated Level 1 target ran all 23 tests and failed five:

- `codex_repo_overlay_uses_codex_home_and_leaves_the_user_home`;
- `codex_mcp_without_a_codex_root_injects_into_an_empty_overlay`;
- `codex_mcp_injects_servers_into_the_config_codex_home_names`;
- `a_codex_to_opencode_transition_leaves_no_codex_selector_in_the_opencode_child`;
  and
- `a_codex_to_gemini_transition_injects_into_the_gemini_overlay`.

Each failure recorded the fixture-local `HOME` and overlay but the host
`CODEX_SQLITE_HOME`. This is not evidence that production should discard an
explicit ambient state selector—the specification correctly requires that
intent to be preserved. It is evidence that the tests are non-hermetic and
their current green result depends on which process launched Nextest. It also
means the implementation's acceptance record claiming 7,140 passing L1 tests
does not reproduce in a wrapped developer session.

Required change: extend the shared fixture policy from generated provider
metadata plus the small set of profile-owned external-state selectors. Default
tests must remove all such values before setting fixture intent; tests whose
subject is ambient restoration should add the values explicitly after building
the command. Add a policy test that starts with opposite ambient values and
asserts both command surfaces produce the same clean baseline. Then rerun the
whole dedicated target with a parent that exports every selector.

### 2. Reused overlay roots retain excluded data and race between concurrent launches (high)

`OverlayStorage::new` assigns every launch of a provider the same persistent
path (`lib/src/provider_overlay/selector.rs:62-84`). Materialization iterates
only current source entries. An excluded or live-state name is skipped, not
removed, and a source entry deleted since the prior run is never observed
(`cli/src/commands/wrap/provider_overlay.rs:307-348`). Recursive copy mode has
the same one-way update behavior (`provider_overlay.rs:366-395`).

This breaks repo isolation deterministically. For example, a Codex prompt or
Gemini MCP launch without `--repo` can place `skills` in the shared overlay. A
later `--repo` launch computes `skills` as excluded but leaves the existing
overlay entry intact, so the provider still sees the user skill that the
current plan promises to hide. The current filesystem test proves only a fresh
overlay and therefore cannot catch this mode transition.

The shared path also makes simultaneous sessions unsafe. Repo-scoped prompt
materialization removes and rebuilds a directory, while Codex/Gemini MCP
injection rewrites a config file and cleanup later removes it. Two launches for
different repositories or MCP sets can therefore expose one launch's view to
the other or delete configuration while the other provider is still running.
That contradicts the specification's per-launch, session-scoped view.

Required change: give each invocation/attempt an isolated overlay root and
leave compatible legacy storage untouched as migration input. If persistent
shared storage is retained instead, it needs a full desired-state
reconciliation plus synchronization held for the entire provider lifetime;
that is more complex and unnecessarily serializes same-provider sessions. Add
Level 1 regressions for non-repo-to-repo transitions, source deletion, two
different repositories, two MCP server sets, and overlapping child lifetimes.

### 3. The native-Windows L2 test is designed to fail before exercising the feature (high)

The specification requires native Windows to prove home preservation and
recursive copy materialization in a real terminal. The Windows test documents
that `dirs::home_dir()` ignores its fixture `USERPROFILE`, then asserts that the
resolved known-folder home nevertheless lives inside the fixture
(`cli/tests/level2_provider_overlay_capture.rs:30-43,428-443`). On a normal
Windows test account that assertion is false by construction, and the test
exits before spawning Claudine.

This is not merely missing cross-OS execution evidence. The checked-in test
cannot reach the behavior it claims to verify without writing into the test
account's real profile, which its own isolation contract prohibits. Compile
coverage and copy-mode unit tests do not replace the required Level 2
real-binary boundary.

Required change: make the invocation's resolved provider-source/overlay home an
explicit, injectable input at the command-fixture boundary without changing
the child's raw home variables, or provide an equivalent disposable Windows
profile harness. The L2 test must then launch, capture the fake provider in
background WezTerm, and prove recursive copy behavior without reading or
writing the developer/runner profile. WSL2 execution evidence can remain a CI
concern; the structurally unreachable native-Windows test cannot.

### 4. Kilo advertises inline MCP delivery that is not wired into the runtime (medium)

Kilo's generated facts say MCP is `composable_injection` through
`KILO_CONFIG_CONTENT` (`docs/providers/facts/kilo.yaml:297-313`). Its behavior
module says the opposite—native MCP is not wired—and relies on the default
`McpBehavior::runtime_injector`, which returns `None`
(`lib/src/provider/kilo/behavior.rs:1-12,44-48` and
`lib/src/provider/behavior.rs:65-85`). Consequently `overlay_reasons` records a
supported inline MCP reason, but the wrapper later fails with the generic
unsupported-runtime-injection error instead of constructing
`KILO_CONFIG_CONTENT`.

The test map's activation matrix covers Codex, Gemini, and OpenCode MCP but not
Kilo, so the claimed per-provider/per-reason verdict audit does not test this
published capability end to end.

Required change: either implement and test a Kilo injector that emits the
researched inline JSON shape, or mark Kilo MCP `unsupported` until that wiring
exists. Add a Level 1 fake-provider test that proves the selected verdict and
the actual launch behavior agree.

## Requirement Verification Levels

| User-facing requirement | Strongest current verification | Assessment |
| --- | --- | --- |
| Direct wrapper and composition launches preserve `HOME`/`USERPROFILE`/`HOMEDRIVE`/`HOMEPATH` | Level 2 tmux on macOS/Linux; Level 1 fake providers across activation reasons | Appropriate on Unix. The L1 target is currently red because of inherited provider state (finding 1). |
| No path writes a null device or overlay into a global home variable | Level 2 tmux plus Level 1 fake-provider and unit guards | Appropriate for Unix and passing in the focused L2 run. Native-Windows L2 is unreachable (finding 3). |
| Explicit provider roots become sources and selectors point at the correct overlay shape | Level 1 fake-provider tests plus units | Appropriate boundary and passing in the focused L1 run. |
| `--repo` hides provider resources at the documented level | Level 2 fresh Codex overlay plus Level 1 fresh-overlay tests | Inadequate for persistent reuse and concurrency; production behavior is broken (finding 2). |
| Codex prompt/MCP overlays work and SQLite remains outside | Level 2 tmux and Level 1 fake-provider tests | Correct test levels, but three dedicated Level 1 cases currently fail from ambient selector leakage (finding 1). |
| Gemini MCP writes under `GEMINI_CLI_HOME` | Level 1 direct and composition fake-provider tests | Appropriate and passing. |
| OpenCode MCP remains inline and creates no filesystem overlay | Level 1 fake-provider test | Appropriate and passing. |
| Unsupported provider/reason pairs refuse before spawn | Level 1 fake-provider and unit tests | Appropriate for listed refusal rows; Kilo's contradictory supported verdict is untested (finding 4). |
| Overlay materialization failure is typed and pre-spawn | Level 1 fake-provider tests | Appropriate and passing. |
| Nested `git`, `gpg`, and `gh` observe the original home | Level 2 tmux and Level 1 fixture stubs | Appropriate on Unix; native-Windows real-terminal path is unverified because its test cannot launch (finding 3). |
| Proxy/retry/resume restore the baseline and apply only the target provider selector | Level 1 fake-provider plus unit replay tests | Appropriate level, but two transition cases currently fail from fixture contamination (finding 1). |
| Native Windows recursively copies directories without hard links | Copy-mode unit tests; intended Level 2 WezTerm test never reaches launch | Wrong strongest executable level for the explicit real-terminal requirement; finding 3 is high severity. |

Level 3 is not required. No requirement depends on the terminal emulator's
keyboard encoder or OS-level input events.

## Verification Performed

- Refreshed the GitNexus index for the current working tree. Upstream impact is
  **critical** for `build_overlay` (16 impacted symbols across five modules) and
  **high** for `build_child_env_with_launch` (the CLI `async_main` execution
  process is affected). This validates reviewing direct, composition, and
  replay routes separately.
- `CARGO_TARGET_DIR=<fresh temp> just test-cli --test
  level1_provider_overlay_home --no-fail-fast`: **18 passed, 5 failed**.
- `CARGO_TARGET_DIR=<fresh temp> just test-library provider_overlay`: **25
  passed**. These pure tests do not exercise the contaminated process boundary.
- `BISCUIT_TEST_REQUIRED_BACKENDS=tmux CARGO_TARGET_DIR=<fresh temp> just
  _test_l2 claudine-cli --features terminal-tests --test
  level2_provider_overlay_capture`: **1 passed**, with tmux execution evidence
  recorded.

The repository's existing `target/debug/deps` artifacts are read-only, so the
first targeted attempt failed before compilation. All reported test results
above use a fresh temporary target directory. The full `just test`, full
`just test-l2`, and `just lint` gates were not rerun after the dedicated L1
target reproduced a production-readiness blocker. Cross-OS result collection
itself remains CI work and does not affect this verdict; the unreachable test
design in finding 3 does.

## Design Assessment

The typed `OverlayReason`/`OverlayCapability`/`OverlayPlan` model is a material
ergonomic improvement. It centralizes selector shapes, keeps provider-specific
side effects in profiles, and makes fail-closed behavior explicit. The
environment baseline and replay patch are also the right abstractions for
provider transitions.

The main architectural mistake is retaining one mutable provider directory as
both durable cache and session-specific launch view. A plan is per launch; its
filesystem representation should have the same ownership. Per-invocation
overlay roots make cleanup, concurrency, exact reconciliation, and MCP
session-scoping simpler, while preserving the old directories as read-only
migration sources satisfies the no-destructive-migration requirement.
