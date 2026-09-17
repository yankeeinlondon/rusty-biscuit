---
$schema: feature-review.yaml
ready: false
human_review: false
reviewed_by: codex/gpt-5.6-sol
created: 2026-09-16T09:49:29-07:00
spec: 2026-09-12-shadow-home/spec.md
implemented: true
implemented_by: claude/opus
log: claudine/fixes/2026-09-12-shadow-home/implementation-log.md
description: A **fix** review of `2026-09-12-shadow-home/spec.md`
fix: 2026-09-12-shadow-home/review-2.md
previous: 2026-09-12-shadow-home/review-1.md
next: 2026-09-12-shadow-home/review-3.md
findings:
    - "[high] Failed provider-state write-back is logged and then destroyed"
    - "[high] The native-Windows Level 2 test still cannot reach the launch"
    - "[medium] Kilo still advertises inline MCP delivery that is not wired into the runtime"
---

# Review 2: Preserve Provider Overlays Without Replacing the User Home

## Verdict

The fix is **not ready for production**. The implementation has resolved the
prior review's contaminated Level 1 fixture and shared-overlay concurrency
defects. The focused selector, transition, overlap, cleanup, and real-terminal
tests are green with a fresh target directory.

Two prior findings remain: the native-Windows Level 2 test still fails before
it can launch Claudine, and Kilo still publishes an MCP capability for which it
has no runtime injector. This review also found a new high-severity durability
defect: a failed credential write-back is reduced to a warning immediately
before the only recoverable copy is deleted.

Human review is not required. The specification already establishes the
desired behavior, and each remaining repair has a deterministic technical
outcome.

## Prior Review Closure

The previous review had no `## Blocked Findings` section, so there were no
blocked findings that could have become unblocked before the latest
implementation.

| Previous finding | Status in this iteration |
| --- | --- |
| L1 overlay suite inherits host provider selectors | **Implemented.** The shared fixture now derives and scrubs the metadata selectors and external-state variables, and a policy test checks both command surfaces. The combined focused target passed 54 of 54 tests. |
| Reused overlay roots retain data and race | **Implemented.** Each launch now has a fresh root guarded by a lifetime lease, concurrent launches are isolated, and abandoned roots are swept. The new write-back defect below is a separate durability problem introduced by preserving mutable provider state from disposable roots. |
| Native-Windows L2 test fails before the feature | **Not implemented.** The checked-in test retains the same contradictory home assertion and still cannot reach the launch. |
| Kilo advertises unwired inline MCP delivery | **Not implemented.** The published capability and runtime behavior still disagree. |

## Findings

### 1. Failed provider-state write-back is logged and then destroyed (high)

`WriteBack::apply` treats every read or write error as a warning and omits the
failure from its result (`lib/src/provider_overlay/write_back.rs:152-169`).
`OverlayLease::drop` then ignores that result and unconditionally removes the
launch root and lock (`lib/src/provider_overlay/lease.rs:93-105`).

This can lose authentication state. If a provider rotates a top-level token or
credential file in a copied overlay and the write-back encounters a permission
error, Windows sharing violation, transient scanner lock, full disk, or other
I/O failure, the provider can exit successfully while Claudine deletes the only
copy of the new credential. The old source credential may already have been
invalidated. A tracing warning is not a recovery mechanism and does not satisfy
the specification's requirement to preserve provider authentication and stable
settings.

The tests prove successful write-back and deliberate refusal when the source
changes concurrently. They do not inject a write-back failure or prove that
the changed state remains recoverable afterward.

Required change: make finalization explicit enough to retain a recovery
artifact whenever changed state cannot be persisted. Report the failure in a
durable, actionable way instead of discarding it from `Drop`; preferably perform
write-back in an explicit post-child finalization path whose outcome can be
observed. Add a Level 1 filesystem-boundary test that injects a write failure,
proves the launch cannot silently claim complete persistence, and proves the
new provider state remains recoverable.

### 2. The native-Windows Level 2 test still cannot reach the launch (high)

The prior finding remains unchanged. The Windows test captures
`HomeBaseline`, acknowledges that `dirs::home_dir()` uses the Windows known
folder instead of its fixture `USERPROFILE`, and then asserts that the resolved
home is inside the fixture workspace
(`cli/tests/level2_provider_overlay_capture.rs:441-461`). On an ordinary native
Windows account this assertion is false before source seeding, fake-provider
compilation, WezTerm startup, or Claudine launch.

The current implementation therefore has only Level 1/unit evidence for the
Windows recursive-copy boundary, despite the specification explicitly
requiring a Level 2 native-Windows launch. This is a test-design defect, not a
request for cross-OS CI proof. Once the test can launch, its fixed one-second
startup delay should also be replaced with a bounded readiness condition so
slow runners do not make it flaky.

Required change: inject a disposable provider source/overlay root at the
command-fixture boundary without changing the child's raw Windows home
variables, or run under an equivalent disposable Windows profile. The test
must reach the fake provider, capture the real terminal, and prove recursive
copy behavior without accessing the runner's actual profile.

### 3. Kilo still advertises inline MCP delivery that is not wired into the runtime (medium)

Kilo's facts continue to declare `mcp: composable_injection` through
`KILO_CONFIG_CONTENT` (`docs/providers/facts/kilo.yaml:309-313`). Its behavior
module still states that native MCP is not wired and inherits the default
`McpBehavior::runtime_injector`, which returns `None`
(`lib/src/provider/kilo/behavior.rs:11,44-48`). Direct wrapping consequently
reaches the generic missing-injector error, and launch-plan rebuilding returns
`NoMcpInjector`; there is still no successful Kilo Level 1 MCP test.

Required change: either implement the researched inline Kilo injector and add
a Level 1 fake-provider test for the emitted JSON, or mark Kilo MCP unsupported
in the provider facts until it is implemented. The declared capability,
planner verdict, and runtime behavior must agree.

## Requirement Verification Levels

| User-facing requirement | Strongest current verification | Assessment |
| --- | --- | --- |
| Direct, composed, retry, resume, and proxy launches preserve `HOME`, `USERPROFILE`, `HOMEDRIVE`, and `HOMEPATH` | Level 2 tmux on Unix plus Level 1 fake-provider tests | Appropriate on Unix and green. Native Windows remains blocked by finding 2. |
| No launch writes an overlay or null device into the global home variables | Level 2 tmux plus Level 1/unit guards | Appropriate on Unix and green. Native-Windows Level 2 does not reach launch. |
| Explicit provider roots become sources while selectors point to the overlay shape | Level 1 fake-provider tests plus units | Appropriate process boundary and green. |
| Repository resources are masked and concurrent launches receive isolated views | Level 2 tmux for a fresh launch, Level 1 overlapping real-binary launches, and units | Appropriate for Unix behavior; overlap and transition regressions are green. Native-Windows recursive materialization remains below its required level. |
| Codex prompt/MCP injection works while SQLite remains outside the overlay | Level 2 tmux plus Level 1 fake-provider tests | Appropriate on Unix and green. |
| Gemini MCP is written under `GEMINI_CLI_HOME` | Level 1 direct/composition fake-provider tests | Appropriate and green. |
| OpenCode MCP stays inline and creates no filesystem overlay | Level 1 fake-provider test | Appropriate and green. |
| Kilo MCP follows its published composable-injection verdict | No successful Level 1 launch test; runtime has no injector | Missing required process-boundary verification and behavior; finding 3. |
| Unsupported provider/reason pairs refuse before spawn | Level 1 fake-provider and unit tests | Appropriate for the actual refusal rows and green. Kilo's published supported row remains contradictory. |
| Mutable top-level authentication/settings survive a disposable overlay | Level 1 real-binary and unit success/conflict tests | Correct level for filesystem behavior, but the I/O-failure path is untested and destructive; finding 1. |
| Nested `git`, `gpg`, and `gh` observe the original home | Level 2 tmux plus Level 1 fixture stubs | Appropriate on Unix and green; native Windows remains blocked by finding 2. |
| Native Windows recursively copies directories without hard links | Level 1/unit copy-mode tests; intended Level 2 WezTerm test exits before launch | Wrong strongest executable level for the explicit real-terminal requirement; finding 2 is high severity. |

Level 3 is not required because the specification has no user-visible behavior
that depends on a terminal emulator encoding an OS keyboard event.

## Verification Performed

- Refreshed the GitNexus index for this exact working tree. Upstream impact is
  **critical** for `build_overlay` (55 impacted symbols, four direct callers,
  two execution processes, and nine modules) and **high** for
  `build_child_env_with_launch` (seven impacted symbols, three direct callers,
  two processes, and four modules). Direct, composition, and replay paths were
  therefore reviewed separately.
- `CARGO_TARGET_DIR=<fresh temp> just test-cli --test
  level1_provider_overlay_home --test cli_process_fixture --no-fail-fast`:
  **54 passed**.
- `CARGO_TARGET_DIR=<fresh temp> just test-library provider_overlay
  --no-fail-fast`: **31 matched tests passed**.
- `CARGO_TARGET_DIR=<fresh temp> just test-cli provider_overlay
  --no-fail-fast`: **49 matched tests passed**.
- `BISCUIT_TEST_REQUIRED_BACKENDS=tmux CARGO_TARGET_DIR=<fresh temp> just
  _test_l2 claudine-cli --features terminal-tests --test
  level2_provider_overlay_capture`: **1 passed**, with tmux execution evidence.
- `CARGO_TARGET_DIR=<fresh temp> just lint`: **passed**. The linker emitted a
  non-fatal `__eh_frame section too large` warning while building a debug
  binary; Clippy completed successfully.

The repository's shared target artifacts were read-only, so an initial command
failed before compilation. All reported results use a fresh temporary target
directory. Cross-OS result collection itself remains CI work and does not
affect readiness; the unreachable native-Windows test implementation does.

## Design Assessment

Fresh launch-owned roots and a lifetime lease are simpler and safer than the
previous shared mutable overlay. Deriving fixture cleanup from provider
metadata also prevents new selectors from silently contaminating tests.

The remaining architectural weakness is performing fallible credential
persistence inside `Drop`. Rust destructors cannot return a meaningful error,
and deleting the recovery source immediately afterward turns a recoverable I/O
failure into silent state loss. An explicit finalization boundary would make
the durability contract testable and observable without weakening cleanup.
