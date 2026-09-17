---
$schema: feature-review.yaml
ready: false
human_review: false
reviewed_by: codex/gpt-5.6-sol
created: 2026-09-16T20:54:45-07:00
spec: 2026-09-12-shadow-home/spec.md
implemented: false
description: A **fix** review of `2026-09-12-shadow-home/spec.md`
fix: 2026-09-12-shadow-home/review-6.md
previous: 2026-09-12-shadow-home/review-5.md
findings:
    - "[high] Level 2 lifecycle fixtures inherit provider roots from the tmux server"
---

# Review 6: Preserve Provider Overlays Without Replacing the User Home

## Verdict

The fix is **not ready for production**. Review #5's only unblocked finding is
implemented: the lifecycle fixture now uses provider-overlay terminology and no
longer creates empty `.codex` or `.gemini` source roots. Its three MCP consumers
pass through the real tmux backend without those directories.

One high-severity test-isolation gap remains. The same Level 2 launcher preserves
ambient provider selectors from the tmux server, so the fixture can read a real
provider root, fail according to the developer's environment, and cease to prove
the missing-default-root behavior it now documents. No human decision is needed;
the specification explicitly requires a private config tree that cannot read the
developer's credentials.

## Prior Review Closure

Review #5 contains no `## Blocked Findings` section. Reviews #1 through #4 also
record no blocked findings, so no previously blocked item became actionable
before the last implementation.

| Review #5 finding | Status in this iteration |
| --- | --- |
| Level 2 fixture still documents and provisions the retired shadow-HOME behavior | **Implemented.** `seed_mcp_catalog` no longer creates `.codex` or `.gemini`; its comment now states that runtime injection uses a per-launch provider overlay and that a missing default source root produces an empty overlay. The proxy-equivalence documentation now names `CODEX_HOME` and `config.toml`. The focused row and all three `seed_mcp_catalog` consumers pass on tmux without seeded provider roots. |

## Findings

### 1. Level 2 lifecycle fixtures inherit provider roots from the tmux server (high)

`cli/tests/level2_lifecycle_control.rs:389-409` describes the staged invocation
as host-independent, but `staged_env_prefix` resets only `NO_COLOR`, `MODEL`,
`HOME`, and `PATH`. It does not remove `CODEX_HOME`, `GEMINI_CLI_HOME`, or the
other provider selectors. A tmux pane inherits its environment from the tmux
server, and overlay planning intentionally treats an inherited selector as
explicit user intent and resolves it as the provider source root.

This directly contradicts the specification's Level 2 requirement that the
fixture use a private home and config tree and never read the developer's real
credentials. It also makes the corrected premise at
`cli/tests/level2_lifecycle_control.rs:7422-7430` environment-dependent: the
absence of `.codex` and `.gemini` under the staged `HOME` does not establish a
missing default source root when the tmux server exports a provider root.

The gap is non-vacuous. With a fresh isolated tmux server inheriting
`GEMINI_CLI_HOME=/tmp/claudine-review6-missing-gemini-root-20260916`,
`level2_lifecycle_equivalence_target_mcp_injection_matches_direct_run` fails
before provider spawn with `provider.overlay_failed` at `source_root (mcp)` and
records no provider events. The identical isolated-server run with the selector
absent passes. A run against the already-running host tmux server also passes
because that server happens not to carry either selector, demonstrating the
machine/server-history dependency.

Required change: give staged Level 2 lifecycle invocations the same
provider-selector isolation contract as `CliProcessFixture`. Remove all metadata
overlay selectors and profile-owned selector variables before launching
Claudine, while allowing a test to opt a value back in explicitly after the
baseline is established. Add a regression that starts with at least
`CODEX_HOME` and `GEMINI_CLI_HOME` present and proves the staged child cannot
observe or use them. The regression must use a fresh or explicitly controlled
tmux server environment so an old server cannot mask the premise.

## Requirement Verification Levels

| User-facing requirement | Strongest verification present | Assessment |
| --- | --- | --- |
| Direct, composed, sequence, retry, resume, and proxy launches preserve launch-time home variables | Level 2 tmux on Unix plus Level 1 fake-provider launches | Appropriate level. The production behavior is green, but the lifecycle L2 fixture is not hermetic with respect to provider selectors. |
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
| Native Windows recursively materializes directories without links | Level 1 copy-mode units plus a reachable Level 2 WezTerm test | Correct verification level and harness shape. Cross-OS execution evidence itself is not a readiness input. |
| Level 2 fixtures use only private provider configuration and cannot read developer credentials | Level 2 tmux, but without provider-selector isolation | **Gap.** The correct test level exists, but its environment boundary is invalid; an inherited selector can redirect source discovery outside the fixture. |

Level 3 is not required because no requirement depends on operating-system
keyboard or mouse injection, terminal input encoding, hotkeys, paste, or IME.

## Verification Performed

- Bound GitNexus to the indexed `feat-better-static-analysis` worktree at
  `afd762637`; the index is current. The provider-overlay query identifies
  `OverlayPlanner::resolve_source_root` as the source-root authority, and the
  `OverlayPlanner` context is exact.
- Focused clean control with a fresh isolated tmux server:
  `level2_lifecycle_equivalence_target_mcp_injection_matches_direct_run`:
  **1 passed, 95 skipped**, backend proof `tmux run=1 skip=0 panic=0`.
- Focused non-vacuity mutation with the same isolated-server shape and a missing
  inherited `GEMINI_CLI_HOME`: **1 failed, 95 skipped** with
  `provider.overlay_failed` before provider spawn. The same mutation against the
  pre-existing host tmux server passed because its global environment lacked the
  selector.
- Review #5 implementation evidence reports all three `seed_mcp_catalog`
  consumers passing on tmux, `just test` at **7204 passed, 9 skipped**, and
  `just lint` clean.
- The first local focused command against the shared Cargo target failed before
  reviewed code compiled because that target is read-only. All independent
  results above use a fresh temporary target directory.

## Design Assessment

The production provider-overlay design remains sound: the typed plan preserves
the user home, honors explicit provider roots, restores selectors between
provider transitions, and fails closed. Review #5's requested terminology and
fixture cleanup is correct and does not warrant further product changes.

The remaining work is localized to the Level 2 lifecycle harness. Reusing the
registry-derived selector list already used by `CliProcessFixture` would keep
the fix small and prevent new providers from silently reintroducing the same
test-isolation defect.
