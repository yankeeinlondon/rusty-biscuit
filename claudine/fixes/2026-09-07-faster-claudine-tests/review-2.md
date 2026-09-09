---
$schema: feature-review.yaml
ready: false
agent: codex/gpt-5.6-sol
created: 2026-09-09T12:59:28-07:00
spec: 2026-09-07-faster-claudine-tests/spec.md
implemented: true
description: A **fix** review of `2026-09-07-faster-claudine-tests/spec.md`
fix: 2026-09-07-faster-claudine-tests/review-2.md
previous: 2026-09-07-faster-claudine-tests/review-1.md
next: 2026-09-07-faster-claudine-tests/review-3.md
---

# Review 2 — Faster Claudine Tests

## Verdict

The fix is **not ready for production**. Review 1's critical PTY-classification
finding is resolved: the four pseudo-TTY binaries and all 19 identities now run
as Level 1, use `CliProcessFixture`, and are covered by a resource-based spawn
guard. The wrapper-summary claim was also narrowed correctly, and the failing
WezTerm/Atuin case is fixed for the WezTerm and Kitty backends.

The candidate does not yet satisfy the specification's closure contract. The
fresh inventory reconciliation and the test-audit package are red, the canonical
Level-1 suite fails, deterministic shell startup is still missing from two
real-terminal backends, and the required cross-platform CI measurements and
ratified budgets do not exist.

## Findings

### 1. High: the current test population is unreconciled and the canonical Level-1 gate is red

The specification requires every discovered identity to have exactly one
reviewed family and execution route, and requires the relevant local gates to
pass (`spec.md:86-96,212-228`). Fresh verification against the current working
tree contradicts the closure claims in `results.md`:

- `npx tsx inventory-reconciler.ts` exits 1 with **18 violations**: 13 current
  identities are unassigned (`compose_frontmatter_model` and
  `contamination_probes`) and five family counts disagree with the enumeration.
- `tools/test-audit/just test` exits 1 with **six failures**. The attribution
  compatibility tests still expect the 19 former Level-2 identities, while the
  reconciler compatibility tests expose the current unassigned identities,
  undeclared exclusions, runner-only identities, and count drift.
- The canonical `claudine/just test` run exits 100. Nextest stopped after 4,639
  of 6,898 tests with 4,638 passed, one failed, nine skipped, and 2,259 not run.
  `propagated_context_fixtures::isolated_fixture_can_opt_in_to_provider_memory_discovery`
  fails because the refreshed document changes the launch plan without the
  recorded inputs needed to rebuild it.

`results.md:318` still marks AC1 verified from a historical revision, while the
inventory itself says its current counts come from an uncommitted tree
(`inventory.md:23-32`). A stale green capture cannot close a gate whose inputs
have changed, and a fail-fast partial run cannot establish the rest of L1.

**Required change:** reconcile the current enumeration, `families.json`, and
inventory until both the standalone reconciler and every test-audit compatibility
test pass. Fix the propagated-context regression, rerun the complete canonical
Level-1 suite without failures, and update `results.md` with the current revision,
identity counts, skips, and complete gate output.

### 2. High: the required CI performance evidence and budgets are still absent

The specification requires three consecutive candidate CI runs on Linux,
macOS, Windows, and WSL2, with numeric per-family budgets derived from compatible
CI evidence (`spec.md:181-199`). The implementation's own closure record remains
explicit: baseline **1 of 3**, candidate **0 of 3**, and no budgets
(`results.md:20-31,39-78,146-166`). Local alternating runs are useful attribution
evidence, but the specification expressly prevents them from setting the target.

The new JUnit-to-family aggregator closes the missing-join portion of review 1's
finding, but it does not prove the provenance property its output claims. A run
is identified only by the staging directory's basename
(`tools/test-audit/src/attribute/aggregate.ts:200-205`); the manifest schema has
no revision, ref, workflow run sequence, or event fields
(`tools/test-audit/src/junit/manifest.ts:11-19`); and callers default the
provenance kind to `ci` (`tools/test-audit/src/attribute/command.ts:110-130`).
`deriveBudgets` checks only that a declared count reaches three and then reports
those runs as consecutive (`tools/test-audit/src/attribute/index.ts:341-389`). It
therefore cannot reject three unrelated, reordered, or locally staged trees
that happen to be labeled CI. The aggregator also selects the first matching
manifest cell with `find` (`aggregate.ts:135-148`), so a duplicate unexpected
record is not rejected as an ambiguous sample.

**Required change:** collect and retain the required baseline and candidate CI
runs on all four legs, including intervening failures. Add immutable source and
workflow provenance to the staged metadata and mechanically verify same-source,
ordered consecutive runs before budget derivation. Reject duplicate and
unexpected manifest cells. Then ratify the per-leg/per-family budgets and run
all candidate legs against them.

### 3. High: real-terminal shell isolation fixes only WezTerm and Kitty, leaving the CI tmux route host-dependent

Review 1's Atuin failure is fixed for WezTerm and Kitty by an outer login shell
that launches an rc-suppressed inner interactive shell
(`biscuit-test-harness/src/lib.rs:449-478`). However, tmux still starts
`<shell> -l` directly in both entry points
(`biscuit-test-harness/src/tmux.rs:41-62,326-354`), and Terminal.app still emits
`exec <shell> -l` (`biscuit-test-harness/src/apple_terminal.rs:959-1004`). The
harness documentation explicitly acknowledges that both inherit whatever the
host profile chain sources (`biscuit-test-harness/README.md:108-130`).

This is the same class of contamination that caused the previous Level-2
failure. It is especially relevant to the portable tmux backend: a login profile
that chains an interactive rc file can inject Atuin, starship, or another prompt
hook before the test command. The implementation log calls this a finding and
says it was not fixed (`log.md:2465-2473`), but supplies no linked owner document
as required for a deferred shared-setup finding (`spec.md:220-222`). A green run
on a host whose profile happens not to chain its rc file is not deterministic
cross-platform evidence.

**Required change:** route tmux and Apple Terminal shell startup through the same
tested login-plus-rc-suppressed policy, or create a linked owner specification
with evidence and an explicit reason that satisfies the deferral rule. Re-run
the canonical Level-2 route on the backends used locally and in CI.

### 4. Medium: Level-1 PTY tests can false-green by skipping, and one migrated path retains an untracked readiness sleep

The reclassified binaries use `require_level!(Level::L1, pty_available(), ...)`
(for example `level1_provided_partial_file_pty.rs:154,186,213,244`). On a Unix
runner where PTY setup is missing or broken, these ordinary Level-1 tests report
a skip rather than failing the canonical L1 gate. That conflicts with the
project's Level-1 contract: missing ordinary test infrastructure is not passing
evidence. The full run already reports nine skips, so closure must distinguish
intentional platform exclusions from unavailable L1 infrastructure.

In the same binary, `confirm_and_drain` waits a fixed 300 ms before sending `y`
because there is no observable raw-mode/read-loop marker
(`level1_provided_partial_file_pty.rs:129-148`). This is exactly the readiness
sleep the specification requires replacing with bounded observation
(`spec.md:162-170`). Under contention it can still send input too early, while
always adding 1.2 seconds across the four tests. The current inventory's sleep
census does not identify this site.

**Required change:** make PTY availability/setup failure fatal for the ordinary
L1 route while retaining explicit compile-time platform exclusions. Expose and
observe a deterministic readiness condition before sending confirmation input,
record the site and disposition in the inventory, and repeat the changed tests
under representative suite load.

## Review 1 Closure Status

| Review 1 criterion | Status | Evidence |
|---|---|---|
| Reclassify four PTY binaries and 19 identities into Level 1; use the fixture and guard contract | **Resolved** | The focused four-binary run passed 19/19; `spawn_site_guard` passed 19/19 and now classifies actual emulator resources rather than filename prefixes. |
| Add Level-2 wrapper-summary rendering proof or narrow the claim | **Resolved by narrowing** | The test now claims textual ordering only and explicitly disclaims glyph-width, SGR, and layout proof. No visual badge requirement is asserted. |
| Isolate WezTerm startup and obtain a green real-terminal route | **Partially resolved** | The recorded WezTerm regression passes after rc suppression, but the same shared-startup defect remains in tmux and Apple Terminal as finding 3. |
| Collect CI evidence, aggregate families, and ratify budgets | **Not resolved** | Aggregation exists, but baseline/candidate evidence and budgets are missing; provenance validation is incomplete. |

## Requirement Verification Levels

| User-observable requirement | Strongest verification present | Assessment |
|---|---|---|
| L1 child isolation and shared command policy | Level 1 child-process probes plus structural guards | Correct boundary, and the 19 migrated identities are now included. The overall claim is not closed while the inventory and full L1 gate are red. |
| Interactive schema, provided-partial, and dry-run behavior | Level 1 `expectrl` PTY tests with manufactured bytes | Correct for byte-level interaction and textual ordering; it does not prove terminal-emulator encoding or rendering. One readiness race and false-green skip path remain. |
| Dry-run approval rendering | Level 2 tmux capture plus a Level-1 PTY interaction complement | Appropriate levels are present. The tmux harness still inherits host profile state. |
| Wrapper summary textual ordering | Level 1 PTY transcript assertions | Appropriate after the claim was narrowed. There is intentionally no claim about SGR, glyph width, or final layout. |
| OSC 8 auto-detection in WezTerm | Level 2 real-WezTerm capture | Correct level and the recorded failing case is now green; backend-independent shell isolation remains incomplete. |
| Physical Ctrl+C/chooser keystrokes through a terminal encoder | Level 3 tests exist; no Level-3 run was performed in this review | Correct level exists. This performance/fixture fix does not alter keyboard encoding, so missing Level-3 execution is not a new blocker and must not be cited as passing. |
| Cross-platform performance budgets | CI JUnit evidence, outside the L1/L2/L3 model | Missing and not yet mechanically provenance-safe. |

## Verification Performed

- `tools/test-audit/just test`: **exit 1**, six failed tests.
- `npx tsx inventory-reconciler.ts`: **exit 1**, 18 violations.
- `just test-cli --test spawn_site_guard`: **19 passed, 0 skipped**.
- Focused run of `level1_dry_run_pty`, `level1_provided_partial_file_pty`,
  `level1_pty_wrapper_summary`, and `level1_schema_prompt_pty`: **19 passed,
  0 skipped** in 5.873 seconds.
- `biscuit-test-harness/just test`: **103 passed, 0 skipped**.
- `claudine/just test`: **exit 100** after one failure; fail-fast left 2,259
  tests unrun, so this is not a complete Level-1 result.

Level-2 and Level-3 suites were not rerun during this review. The current stored
Level-2 evidence was used only to confirm the WezTerm-specific repair; no
Level-3 behavior is reported as verified.

## Closure Criteria

1. Reconcile the current identities, family assignments, counts, exclusions,
   and compatibility fixtures; make both inventory gates green.
2. Fix the propagated-context fixture regression and obtain a complete green
   canonical Level-1 run with every skip explained as a platform/tier exclusion.
3. Remove false-green Level-1 PTY skips and replace the 300 ms readiness sleep
   with bounded observation.
4. Apply deterministic rc-suppressed startup to tmux and Apple Terminal, or
   record a compliant linked deferral, then rerun the canonical Level-2 route.
5. Strengthen CI artifact provenance and manifest cardinality checks; collect
   the required baseline and three consecutive candidate runs on every configured
   environment; ratify and enforce the resulting budgets.
6. Refresh `inventory.md`, `results.md`, and the stored gate evidence against the
   final candidate revision so every acceptance claim points to current data.
