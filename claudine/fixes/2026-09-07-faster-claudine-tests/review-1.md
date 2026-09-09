---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-09-08T21:00:26-07:00
spec: 2026-09-07-faster-claudine-tests/spec.md
implemented: true
description: A **fix** review of `2026-09-07-faster-claudine-tests/spec.md`
fix: 2026-09-07-faster-claudine-tests/review-1.md
next: 2026-09-07-faster-claudine-tests/review-2.md
---

# Review 1 — Faster Claudine Tests

## Verdict

The fix is **not ready for production**. The fixture and cost-removal work is
substantial, and the recorded local measurements show large improvements in
several cohorts, but the implementation misses part of its central L1 migration
contract, the canonical L2 gate is not green, and the required CI performance
evidence does not exist yet.

## Findings

### 1. Critical: nineteen Level-1 PTY tests are mislabeled as Level 2 and remain outside the fixture contract

The review's rigor model defines a pseudo-TTY driven with manufactured bytes as
Level 1. Nevertheless, the inventory classifies four `expectrl` binaries and
all 19 of their tests as `cli-l2-pty`, claiming that each uses both a PTY and a
terminal session (`inventory.md:1175-1192`). The tests use `/dev/ptmx` through
`expectrl`; none creates a tmux, WezTerm, Kitty, or Apple Terminal session.
`level2_dry_run_pty.rs` even states that it injects bytes directly into a raw
pseudo-terminal and names a separate tmux test as its emulator-level complement
(`level2_dry_run_pty.rs:24-34`).

This is not only taxonomy drift. The four binaries are declared with the
`terminal-tests` feature (`claudine/cli/Cargo.toml:146-148,198-216`), their test
names carry the `level2_` prefix, and therefore the ordinary `just test` L1
route does not run them. The implementation instead measures them through
`just _test_l2` and reports them as L2 evidence (`results.md:206-213`). That
inflates the L2 population while withholding 19 ordinary PTY tests from the
fast L1 suite.

It also leaves the required L1 process migration incomplete. These binaries
still construct raw commands and rebuild isolation by hand:

- `level2_pty_tests.rs:24-30,53-59`
- `level2_schema_prompt_pty.rs:60-78`
- `level2_provided_partial_file_pty.rs:62-85`
- `level2_dry_run_pty.rs:166-181`

The guard cannot report those violations because it excludes every file whose
name starts with `level2_` (`spawn_site_guard.rs:169-170,396-410`). As a result,
its claim that every remaining live-child and PTY test goes through the builder
(`spawn_site_guard.rs:204-210`) and `results.md`'s AC2 claim of zero raw spawn
sites (`results.md:279-281`) are both false under the specified level model. The
focused guard run in this review passed all 18 tests while those raw sites were
still present, demonstrating the blind spot rather than merely inferring it.

**Required change:** rename and route these four binaries as Level 1, remove the
Level-2 gate and `terminal-tests` requirement unless a production dependency
truly requires it, migrate every Claudine child to `CliProcessFixture::command_std()`
or `apply_policy_to`, and make the guard classify by actual resource boundary
rather than trusting a filename that can exempt a PTY-only test. Reconcile the
inventory, enumeration, measurements, and test counts afterward.

### 2. High: the canonical real-terminal gate is reproducibly red because host shell state is not isolated

`results.md` records `just test-l2` as 236 of 237 on every run since Phase 4.
The failing WezTerm test never observes its exit marker because an Atuin
first-run prompt appears in the spawned pane (`results.md:243-248,294-303`).
Calling this a host condition does not satisfy the specification: the scope
explicitly includes shared fixtures and inherited state, and acceptance
requires shared-setup and reachability findings to be resolved or deferred to
a linked owner. This failure has no owner in the seven residual findings
(`results.md:220-241`).

The affected requirement is genuinely Level 2—it verifies OSC 8 behavior in a
real WezTerm session—so a PTY substitute would be insufficient. The harness
must launch the pane with deterministic shell initialization, or otherwise
prevent the user's Atuin setup from intercepting the command, and the canonical
`just test-l2` route must complete green. Until then, the implementation has no
complete real-terminal regression gate.

### 3. High: the required CI performance contract has no candidate evidence or ratified budgets

The specification requires three consecutive candidate CI runs for every
configured Linux, macOS, Windows, and WSL2 leg and numeric per-family budgets
derived from compatible CI evidence. The closure record says baseline 1 of 3,
candidate 0 of 3 (`results.md:20-31,39-78`), and
`perLegFamilySummed` is empty, so budget derivation intentionally refuses
(`results.md:146-156`; `attribution/budgets-pending.json`). The branch also
still needs to be integrated with newer `main`, with thirteen expected
conflicting files, before candidate evidence can begin (`candidate/README.md:47-69`).

The local alternating runs are useful attribution evidence and show promising
improvements, but the spec expressly says they establish no performance target.
AC6 is therefore incomplete exactly as `results.md:284` acknowledges. This is
a release blocker, not a documentation follow-up: without the missing runs and
family aggregation, the review cannot determine whether the speedup meets its
cross-platform budgets or whether a configured leg regressed.

## Requirement Verification Levels

| Requirement | Strongest verification present | Assessment |
|---|---|---|
| L1 child isolation: pinned CWD/home/PATH, scrubbed application/render/Git state, shared assert/raw policy | Level 1 child-process probes plus source guards | Appropriate for the migrated population, but **incomplete** because the 19 PTY tests above are Level 1 and remain outside it. |
| Interactive schema collection and provided-partial confirmation | Level 1 `expectrl` PTY with manufactured input bytes, mislabeled Level 2 | The behavior/ordering assertions are at an appropriate boundary, but the tier and canonical route are wrong. These tests do not verify terminal-emulator rendering. |
| Dry-run approval prompt renders like normal mode | Level 2 tmux capture in `level2_dry_run_approval_capture.rs`; Level 1 PTY interaction complement | Appropriate level exists. The PTY complement must still be routed as L1. |
| Wrapper summary badges are visible as rendered terminal UI | Level 1 `expectrl` substring checks in `level2_pty_tests.rs` | **Wrong level for a rendering claim.** The test does not capture real-terminal glyph width, styling, or layout. Add or identify direct Level-2 capture for this surface; otherwise narrow the test's stated claim to textual content. |
| OSC 8 behavior in WezTerm | Level 2 real-terminal test | Correct level, but the canonical run fails because host shell initialization intercepts it. |
| User Ctrl+C / chooser keystrokes traverse the OS-to-terminal encoder | Level 3 tests exist; no Level-3 run was performed | Correct test level exists, but execution remains pending. This fix does not change keyboard encoding, so it is not the primary blocker; it must not be cited as passing evidence. |
| Cross-platform performance budgets | CI JUnit measurement, not an L1/L2/L3 boundary | Missing: zero candidate runs and no derived budgets. |

No new Level-3 test is required for the fixture or timing changes themselves.
The existing Windows console-control tests remain correctly classified as
Level 1 because they synthesize a native console-control event and test process
termination, not physical keyboard encoding.

## Verification Performed

- Read the specification, plan, inventory, results, measurement report,
  baseline/candidate handoff, manifests, fixture policy, structural guards, and
  the affected PTY tests.
- `just test-cli --test spawn_site_guard`: **18 passed, 0 skipped**. This is
  negative evidence for finding 1: the gate remains green while the four
  filename-excluded PTY binaries contain raw Claudine spawns.
- Did not repeat the full L1/L2/CI measurements already stored in the fix. The
  missing CI artifacts cannot be recreated locally, and the repository's own
  closure record already reports the L2 failure and zero candidate runs.

## Closure Criteria

1. Reclassify the four PTY-only binaries and migrate all 19 tests into the L1
   fixture and guard contract; regenerate inventory and measurement evidence.
2. Add true Level-2 proof for the wrapper-summary rendering claim or narrow the
   claim to what the Level-1 assertions establish.
3. Isolate WezTerm shell startup from host configuration and obtain a fully
   green canonical `just test-l2` run.
4. Integrate current `main`, collect the required consecutive baseline and
   candidate CI runs on all four environments, implement the JUnit-to-family
   aggregation, ratify budgets, and demonstrate every candidate leg against
   them.
