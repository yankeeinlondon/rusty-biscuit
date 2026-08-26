---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-08-26T18:48:24+01:00
spec: 2026-08-12-ctx-launch-anchor/spec.md
implemented: false
description: "A **fix** review of `2026-08-12-ctx-launch-anchor/spec.md`"
fix: 2026-08-12-ctx-launch-anchor/review-2.md
previous: 2026-08-12-ctx-launch-anchor/review-1.md
---

# Review 2: Ctx Launch Anchor

## Verdict

The fix is **not ready for production**. Review 1's bracket-index policy gap,
portable fixture gap, and most of the route/interpolation matrix have been
repaired. Two high-severity acceptance gaps remain: AC5's accounting still does
not prove the consumer set for each document epoch, and AC13/AC14's required
cross-platform validation has not run against this implementation.

## Findings

### 1. High: consumer accounting is not scoped or asserted per document epoch

`InvocationWorkSnapshot::prepared_context_consumers` is one invocation-wide
`BTreeMap<String, usize>` (`claudine/lib/src/invocation_context.rs:101-105`), and
`record_prepared_context_consumer` increments only the consumer name
(`invocation_context.rs:1064-1072`). There is no epoch key or snapshot
association. The performance report further projects only the map keys and
drops the counts (`claudine/cli/src/perf/report.rs:355-379`). Consequently an
aggregate route can report every expected consumer even if different epochs
populated different subsets.

The new harness regression demonstrates the issue. It verifies the consumer
key set after the proxy/stabilized-read epoch, then changes only
`state.entry` and directly calls `materialize_harness_prompt` for retry and
resume (`claudine/cli/src/commands/wrap/harness_orch/prompt.rs:550-590`). Those
later assertions check only the cumulative construction count; they never
assert the consumer delta for either fresh epoch. This does not meet AC5's
explicit requirement for per-epoch work-accounting assertions proving every
consumer received that epoch's populated snapshot. It also weakens AC11's
route-wide retained-evidence proof.

**Required change:** make the observations attributable to an epoch, or take
and compare exact counter deltas around each canonical epoch boundary. Add
assertions for direct, proxy-target, retry, resume, loop, and sequence epochs
that prove exactly one construction, only permitted stabilized-read extensions,
zero ambient fallbacks, and the exact populated consumer set for that epoch.
The retry/resume proof should traverse the canonical re-entry transition rather
than obtaining the desired branch solely by mutating `state.entry` in the test.

Verification level present: partial Level 1 for invocation aggregates and
construction counts. Required level: Level 1 with per-epoch assertions.

### 2. High: AC13/AC14 has no current-worktree cross-platform evidence

The CLI regression module is no longer Unix-gated and now supplies POSIX and
Windows fake-command fixtures, closing the source-level portability defect from
Review 1. The required execution matrix remains incomplete. The implementation
log records that `build-linux` and `build-win-native` were on a stale commit
without the changed test, `build-win` stopped returning output, the Windows GNU
cross-check failed in `libduckdb-sys` before reaching Claudine, and hosted CI
could not run without publishing the uncommitted work (`log.md:41-68`). The
plan correspondingly leaves the cross-platform and hosted-CI gates unchecked
(`plan.md:415-440`).

These are understandable environment and workflow constraints, but AC13 and
AC14 make successful execution on macOS, Linux, WSL, and native Windows, then
hosted CI, part of the production-readiness contract. Portable-looking Rust and
macOS execution cannot establish Windows prefix, command-resolution, or
filesystem behavior.

**Required change:** make the current implementation available in each named
build environment, run the complete affected Level 1 and Level 2 suites on
`build-linux`, `build-win`, and `build-win-native`, rerun Level 1 after any L2
fix, then run the hosted CI workflow. Preserve the non-focus-taking L2 setup and
record the exact results.

Verification level present: macOS Level 1 and Level 2, plus a source-level
portability audit. Required level: the native multi-OS Level 1/Level 2 and
hosted-CI matrix explicitly required by AC13/AC14.

## Requirement Verification Levels

| Requirement | Strongest verification present | Assessment |
|---|---|---|
| AC1 direct/loop launch-area and lifecycle parity | Level 1 real CLI | Appropriate level; the paired direct, loop, `warn:`, and `when:` cases are present. |
| AC2 opposing-area body, frontmatter, preflight, and lifecycle | Level 1 real CLI with conflicting launch/source areas | Appropriate level. |
| AC3 external-source and outside-repository matrix | Level 1 real CLI on macOS | Appropriate semantic level; native portability remains open under AC13/AC14. |
| AC4 real CLI capture owner | Level 1 binary execution | Appropriate level. |
| AC5 exact per-epoch snapshot reuse | Level 1 invocation-wide counters and canonical materialization tests | **Gap:** consumer observations are not proven for each epoch. |
| AC6 target identity across direct/proxy/re-entry/loop/sequence | Level 2 direct/proxy comparison plus Level 1 route and canonical-owner tests | Appropriate semantic level; per-epoch retry/resume accounting remains an AC5 gap. |
| AC7 sequence graph/task/source parity | Level 1 graph-preflight and canonical-owner tests | Appropriate level; bracket/member/index spellings now fail closed as required. |
| AC8 proxy/re-entry and loop parity | Level 2 direct/proxy execution plus Level 1 re-entry and loop tests | Appropriate behavioral level; the incomplete per-epoch proof is recorded under AC5. |
| AC9 system-prompt and overlay relocation | Level 1 canonical-owner relocation tests | Appropriate level because this is composition semantics, not terminal rendering. |
| AC10 source-owned file and schema resolution | Level 1 conflicting launch/source fixtures | Appropriate level. |
| AC11 no extra discovery | Level 1 invocation work counters | Partial: capture/extension/fallback counts exist, but epoch-to-consumer correlation is absent. |
| AC12 capture-owner guard | Level 1 production-source inventory with seeded violation | Appropriate level. |
| AC13 cross-platform path behavior | macOS Level 1 plus portable fixture source | **Gap:** no current implementation executed on Linux, WSL, or native Windows. |
| AC14 complete validation matrix | macOS Level 1/Level 2/lint and affected Darkmatter gates | **Gap:** the three remote environments and hosted CI remain unverified. |

Level 1 is the appropriate behavioral tier for launch-context values, target
identity, ownership, and file resolution because terminal rendering and input
encoding do not affect those semantics. AC14 independently requires the full
Level 2 gate. No Level 3 test is required because the fix has no keyboard,
paste, IME, mouse, or terminal-input encoder requirement.

## Review 1 Closure Status

- Finding 1, bracket-index target identity: closed by canonical static
  member/index reconstruction, fail-closed dynamic indexes, and Level 1 coverage
  across all four protected roots and expression positions.
- Finding 2, route and interpolation matrix: closed for the specified semantic
  surfaces by real-CLI and canonical-owner tests, with existing Level 2 proxy
  execution providing additional route evidence.
- Finding 3, consumer accounting: partially closed. Named observations and
  fallback accounting exist, but the required per-epoch proof remains Finding 1
  above.
- Finding 4, portability and validation: fixture portability is closed; the
  required execution matrix remains Finding 2 above.

## Verification Run

- `cd claudine && just test` — passed on the macOS review host: 21 catalog,
  4,046 library, 48 contract, 2,384 CLI, and 154 generator tests passed.
- `cd claudine && just lint` — passed, including 18 error guards, the lifecycle
  documentation guard, and Clippy for all five package groups.
- `cd claudine && just test-l2` — passed through the canonical background
  self-spawn recipe: 230 CLI and three generator tests passed.
- `cd darkmatter && just test` — passed: 6,247 library, 653 CLI, and 640 DMLS
  tests passed.
- `cd darkmatter && just lint` — passed for `darkmatter`, `darkmatter-cli`, and
  `dmls`.
- Focused launch-anchor tests and `git diff --check` passed. Darkmatter schema
  validation accepted the spec and both review documents.

## Production Readiness

Not production ready. Close AC5 with per-epoch consumer evidence and complete
the AC13/AC14 native-platform and hosted-CI validation matrix before requesting
another production-readiness review.
