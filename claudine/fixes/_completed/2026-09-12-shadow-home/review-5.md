---
$schema: feature-review.yaml
ready: false
human_review: false
reviewed_by: codex/gpt-5.6-sol
created: 2026-09-16T20:30:09-07:00
spec: 2026-09-12-shadow-home/spec.md
implemented: true
implemented_by: claude/opus
log: claudine/fixes/2026-09-12-shadow-home/implementation-log.md
description: A **fix** review of `2026-09-12-shadow-home/spec.md`
fix: 2026-09-12-shadow-home/review-5.md
previous: 2026-09-12-shadow-home/review-4.md
next: 2026-09-12-shadow-home/review-6.md
findings:
    - "[low] Level 2 fixture still documents and provisions the retired shadow-HOME behavior"
---

# Review 5: Preserve Provider Overlays Without Replacing the User Home

## Verdict

The fix is **not ready for production**. Review #4's only finding is
implemented: the unwritable-storage L1 test can no longer return success when
its permission premise is unavailable. It now fails with an actionable premise
message, and the privilege-independent file-collision case continues to prove
the typed pre-spawn failure without relying on Unix permission enforcement.

One low-severity specification gap remains in active Level 2 test code. The
lifecycle fixture still documents and provisions the retired shadow-`HOME`
architecture, including a claim that a missing default provider root refuses a
launch. That claim contradicts both the current implementation and the test map.
No human review is required; the specification and implemented behavior already
determine the correction.

## Prior Review Closure

Review #4 contains no `## Blocked Findings` section, and reviews #1–#3 likewise
record no blocked findings. No previously blocked item became actionable before
the last implementation.

| Review #4 finding | Status in this iteration |
| --- | --- |
| Unwritable-storage L1 test silently passes when its premise is unavailable | **Implemented.** `an_unwritable_storage_root_stops_the_launch_with_the_typed_diagnostic` replaces the successful early return with `assert!(!writable, ...)`, after restoring fixture permissions. A deliberate non-vacuity mutation made that assertion fail, and the canonical focused run executes all contract assertions with zero skips. |

## Findings

### 1. Level 2 fixture still documents and provisions the retired shadow-HOME behavior (low)

`cli/tests/level2_lifecycle_control.rs:7425-7430` says Codex and Gemini runtime
MCP injection runs through a shadow `HOME`, says the old builder mirrors the
original provider directory, and says the launch refuses when that directory is
missing. The same file describes Codex as writing a “shadow-home TOML” at
`cli/tests/level2_lifecycle_control.rs:7510-7512`.

Those comments are stale in three material ways. The implementation preserves
`HOME`, redirects through `CODEX_HOME` or `GEMINI_CLI_HOME`, and treats a missing
default source root as an empty overlay. The latter behavior is explicitly
mapped to `a_missing_default_source_root_launches_with_an_empty_overlay` and
`codex_mcp_without_a_codex_root_injects_into_an_empty_overlay` in `test-map.md`.
The fixture nevertheless creates empty `.codex` and `.gemini` directories under
the obsolete explanation.

This is active-code documentation drift, not a historical reference. It
violates the specification's requirement that code use the provider-overlay
abstraction and the repository rule that behavior-changing edits update nearby
comments. It also leaves irrelevant setup in a high-value proxy-equivalence
fixture, which can conceal whether a future regression incorrectly makes a
default source root mandatory.

Required change: rewrite both comments in provider-overlay terms and remove the
empty provider-root setup if it has no current purpose. If the directories are
still necessary for a different invariant, name and assert that invariant
instead. The fixture must not claim that `HOME` moves or that a missing default
root is refused.

## Requirement Verification Levels

| User-facing requirement | Strongest verification present | Assessment |
| --- | --- | --- |
| Direct, composed, sequence, retry, resume, and proxy launches preserve launch-time home variables | Level 2 tmux on Unix plus Level 1 fake-provider launches | Appropriate and green for exercised paths. Cross-OS result collection is external to readiness. |
| No launch writes an overlay or null device into a global home variable | Level 2 tmux plus Level 1 process and unit guards | Appropriate and green. |
| Explicit provider roots remain sources while selectors name the provider-visible overlay shape | Level 1 fake-provider launches and units | Appropriate and green, including absent values, non-UTF-8 Unix values, paths with spaces, and native-Windows path construction. |
| Repository resources remain masked and concurrent launches remain isolated | Level 2 tmux for the interactive Unix launch, Level 1 real-binary launches, and units | Appropriate and green. |
| Codex prompt/MCP injection works while SQLite remains outside the overlay | Level 2 tmux plus Level 1 fake-provider launches | Appropriate and green. |
| Gemini MCP writes beneath `GEMINI_CLI_HOME` | Level 1 direct/composition fake-provider launches | Appropriate and green. |
| OpenCode and Kilo MCP remain inline without unnecessary filesystem overlays | Level 1 direct and provider-transition launches plus injector units | Appropriate and green. |
| Unsupported provider/reason pairs refuse before provider spawn | Level 1 fake-provider launches plus units | Appropriate and green. |
| Materialization failures are typed, pre-spawn failures with no null-home fallback | Level 1 real-binary tests plus units | Appropriate and green. The repaired permission case cannot report a false pass, and the file-collision case is privilege-independent. |
| Mutable provider state survives successful write-back | Level 1 real-binary launch plus units | Appropriate and green. |
| Changed provider state remains recoverable after persistence inspection, write, or notice failures | Level 1 deterministic fault-injection units plus a real-binary permission-failure test | Appropriate and green. Each injected failure retains changed bytes and survives a later sweep. |
| Nested `git`, `gpg`, and `gh` observe the original home | Level 2 tmux plus Level 1 fixture tools | Appropriate and green on the exercised paths. |
| Native Windows recursively materializes directories without links | Level 1 copy-mode units plus a reachable Level 2 WezTerm test | Correct verification level and harness shape. Cross-OS execution evidence itself is not a readiness input. |

Level 3 is not required because no requirement depends on operating-system
keyboard or mouse injection, terminal input encoding, hotkeys, paste, or IME.

## Verification Performed

- Bound GitNexus to this exact worktree; its index is current at `4b7e7d6`.
  The provider-overlay query returned the typed plan, materialization, selector
  restoration, and session-key definitions; `materialize_root_level_state` has
  an exact incoming edge from `build_overlay`.
- `CARGO_TARGET_DIR=<fresh temp> just test-cli --test
  level1_provider_overlay_home`: **33 passed, 0 skipped**.
- `CARGO_TARGET_DIR=<same fresh temp> just test-library provider_overlay`: **42
  passed**; this includes deterministic metadata, source-read, fingerprint,
  atomic-write, marker-write, and later-sweep recovery cases.
- The implementation cycle's broader gates report `just test`: **7200 passed,
  9 skipped**, and `just lint`: **clean**.
- The first focused command against the default target directory failed before
  reviewed code compiled because that directory is read-only. The reported
  independent results use a fresh temporary target directory.

## Design Assessment

The production design remains sound. Provider capability metadata selects a
typed overlay strategy without another central provider dispatch, launch-owned
leases isolate concurrent executions, provider transitions restore the
immutable environment baseline, and uncertain write-back state fails closed by
retaining the launch root. The filesystem work is bounded to the provider's
small classified resource set, so no ergonomic or performance change is
warranted.

The remaining finding is narrowly editorial and fixture-focused, but it is part
of the specification's terminology contract and should be corrected before the
specification is marked complete.
