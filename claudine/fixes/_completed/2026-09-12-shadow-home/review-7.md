---
$schema: feature-review.yaml
ready: true
human_review: false
reviewed_by: codex/gpt-5.6-sol
created: 2026-09-17T00:38:15-07:00
spec: 2026-09-12-shadow-home/spec.md
implemented: false
log: claudine/fixes/2026-09-12-shadow-home/implementation-log.md
description: A **fix** review of `2026-09-12-shadow-home/spec.md`
fix: 2026-09-12-shadow-home/review-7.md
previous: 2026-09-12-shadow-home/review-6.md
---

# Review 7: Preserve Provider Overlays Without Replacing the User Home

## Verdict

The fix is **ready for production**. Review #6's only finding is implemented:
the Level 2 lifecycle fixture now removes every registry-derived provider
overlay selector and every profile-owned selector before launching Claudine.
Its new real-tmux regression proves that hostile inherited Codex and Gemini
roots are present in the pane, do not reach the staged child, and cannot alter
filesystem-backed MCP launch behavior.

No implementation, test-rigor, ergonomics, or performance finding remains in
the specification's scope. No human review is required because the remaining
platform result collection is an external CI/CD concern, not a readiness gate.

## Prior Review Closure

Review #6 contains no `## Blocked Findings` section. Reviews #1 through #5 also
record no blocked findings, so no previously blocked item became actionable
before the last implementation.

| Review #6 finding | Status in this iteration |
| --- | --- |
| Level 2 lifecycle fixtures inherit provider roots from the tmux server | **Implemented.** `staged_env_prefix` derives the same complete selector list as `CliProcessFixture` and removes it with portable Unix `env -u` operations. The regression injects every selector through `tmux new-session -e`, independently proves the pane saw `CODEX_HOME` and `GEMINI_CLI_HOME`, then verifies both providers launch, receive MCP configuration, and receive none of the hostile ambient selector values. |

## Findings

None.

## Requirement Verification Levels

| User-facing requirement | Strongest verification present | Assessment |
| --- | --- | --- |
| Direct, composed, sequence, retry, resume, and proxy launches preserve launch-time home variables | Level 2 tmux on Unix plus Level 1 fake-provider launches | Appropriate and green. |
| No launch writes an overlay or null device into a global home variable | Level 2 tmux plus Level 1 process and unit guards | Appropriate and green. |
| Explicit provider roots remain sources while selectors name the provider-visible overlay shape | Level 1 fake-provider launches and units | Appropriate and green, including absent values, non-UTF-8 Unix values, paths with spaces, and native-Windows path construction. |
| Repository resources remain masked and concurrent launches remain isolated | Level 2 tmux for the interactive Unix launch, Level 1 real-binary launches, and units | Appropriate and green. |
| Codex prompt/MCP injection works while SQLite remains outside the overlay | Level 2 tmux plus Level 1 fake-provider launches | Appropriate and green. |
| Gemini MCP writes beneath `GEMINI_CLI_HOME` | Level 1 direct/composition fake-provider launches | Appropriate and green. |
| OpenCode and Kilo MCP remain inline without unnecessary filesystem overlays | Level 1 direct and provider-transition launches plus injector units | Appropriate and green. |
| Unsupported provider/reason pairs refuse before provider spawn | Level 1 fake-provider launches plus units | Appropriate and green. |
| Materialization failures are typed, pre-spawn failures with no null-home fallback | Level 1 real-binary tests plus units | Appropriate and green. |
| Mutable provider state survives successful write-back | Level 1 real-binary launch plus units | Appropriate and green. |
| Changed provider state remains recoverable after persistence inspection, write, or notice failures | Level 1 deterministic fault-injection units plus a real-binary permission-failure test | Appropriate and green. |
| Nested `git`, `gpg`, and `gh` observe the original home | Level 2 tmux plus Level 1 fixture tools | Appropriate and green on the exercised paths. |
| Native Windows recursively materializes directories without links | Level 1 copy-mode units plus a reachable Level 2 WezTerm test | Correct level and harness shape. Cross-OS execution results are external to readiness. |
| Level 2 fixtures use only private provider configuration and cannot read developer credentials | Level 2 tmux with controlled session selectors plus Level 1 fixture-policy tests | Appropriate and green. The regression proves its hostile inherited-root premise independently of tmux server history. |

Level 3 is not required because no requirement depends on operating-system
keyboard or mouse injection, terminal input encoding, hotkeys, paste, or IME.

## Verification Performed

- Refreshed GitNexus through the repository recipe and queried the provider
  overlay/isolation graph. The integration-test helpers are not indexed as
  symbols, so their impact remained `UNKNOWN`; text search resolved the
  uncertainty to 13 same-file `staged_env_prefix` call sites and no production
  callers.
- Focused canonical Level 2 run from a fresh Cargo target:
  `level2_lifecycle_staged_invocation_ignores_inherited_provider_selectors`:
  **1 passed, 96 skipped**, backend proof `tmux run=1 skip=0 panic=0`.
- The implementation's non-vacuity check reports that removing the selector
  scrub makes both the Gemini and Codex arms fail before provider spawn with
  `provider.overlay_failed`.
- The implementation's full Level 2 binary run reports **97 passed, 0 skipped**
  with backend proof `tmux run=97 skip=0 panic=0`; its Claudine lint gate is
  clean.
- The implementation's L1 run reports **7251 passed, 1 failed, 12 skipped**.
  The one failure belongs to the concurrently staged
  `2026-09-15-initialize-after-proxy` work in a separate test binary and is not
  reachable from this review's test-only change.

## Design Assessment

The typed provider-overlay design remains coherent and complete: it preserves
the immutable user-home baseline, honors explicit provider roots, restores
provider-owned selectors between attempts, keeps mutable state outside mirrored
storage, and refuses unsupported isolation before spawn. Review #6's closure
reuses the registry-derived selector authority rather than adding a second list,
so a future provider selector is inherited by both L1 and L2 fixture isolation.

The implementation adds no runtime cost. The additional work is confined to
the Level 2 test harness, where one `env -u` argument per selector is negligible
and directly protects the validity of all staged lifecycle rows.
