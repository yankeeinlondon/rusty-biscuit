---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-17T18:39:19-07:00
spec: 2026-07-15-performance-followup/spec.md
implemented: false
description: "A **feature** review of `2026-07-15-performance-followup/spec.md`"
feature: 2026-07-15-performance-followup/review-8.md
previous: 2026-07-15-performance-followup/review-7.md
---

# Review 8 — Performance Follow-up

## Verdict

This feature is **not ready for production**. Review 7's implementation cycle
introduced no production-code or test-code change and closed no release gate.
No new implementation defect or verification-level mismatch was found, but
acceptance criteria 5 and 6 remain open because the integrated compose-regression
threshold still has no admissible pass/fail determination.

## Tracked Production-Readiness Blocker

The required integrated compose-regression measurement remains unestablished.
The retained point estimate cannot close the feature because identical-code and
cross-run drift exceeded the 5% effect being adjudicated. The committed
`performance-compliance.md` contract still requires two independent quiet-host
runs of all six cases, complete load logs, less than 1% identical-code drift,
base means agreeing within 1.5%, and a recorded `compose_trivial` verdict against
the 5% threshold.

Review 8 checked host admissibility before capture. The host reported load
averages of **12.18** (1 minute), **18.46** (5 minutes), and **26.43**
(15 minutes), so the 1-minute reading exceeded the predeclared **2.0** ceiling
by more than six times. No benchmark was run and no threshold verdict is
claimed from an inadmissible host.

## Findings

No new implementation findings. The tracked release-evidence gap above remains
release-blocking; per the specification's deferred-measurement contract, it is
not duplicated as a new implementation finding on every review iteration.

## Review 7 Closure

Review 7 contained no implementation finding. Its implementation cycle correctly
made no Rust or test change, marked the review implemented, rechecked the three
committed benchmark pins, and recorded another declined high-load attempt in
`performance-compliance.md`. Those documentation changes do not satisfy the
outstanding measurement contract.

## Requirement Verification Levels

| User-observable requirement | Strongest verification | Assessment |
| --- | --- | --- |
| F2: repeated terminal construction reuses one OSC 10 result | Level 2 in real WezTerm on macOS and retained real Kitty on Linux, plus Level-1 manufactured-PTY request counting | Appropriate. Real emulators verify OSC parsing and response; Level 1 supplies the exact request-count oracle. |
| F3: verbose, performance, and warning paths perform one terminal detection | Level 1 spawned CLI with debug-event counting | Appropriate. This is process-local behavior, not terminal rendering or input encoding. |
| F21: redirected macOS output avoids appearance discovery | Level 1 spawned CLI with an argument-sensitive `defaults` sentinel | Appropriate. The requirement is absence of a subprocess on redirected output. |
| F17/F32: shell ordering, approval behavior, timeout, cleanup, and parked-peer notification remain compatible | Level 1 process and deterministic concurrency tests, with retained Windows behavior evidence | Appropriate. These semantics do not depend on a terminal emulator. |
| F22: directory membership, aggregate hash, diagnostics, and exit status remain compatible | Level 1 library and spawned-CLI tests with retained macOS, Linux, and Windows execution | Appropriate. No terminal behavior is asserted. |
| F23: code-theme output remains stable while separate renders observe allowed environment changes | Level-1 snapshots, headless-browser computed rendering, and retained Level-2 terminal rendering | Appropriate for browser and terminal presentation. |
| F35.5: hash explanation, persistence, diagnostics, and exit status remain compatible | Level 1 library and spawned-CLI tests | Appropriate. These are file and process semantics. |

The feature specifies no keypress, hotkey, paste, IME, or mouse behavior, so
Level-3 OS input injection is not applicable. No user-observable requirement is
verified only below the level its behavior requires.

## Verification Performed

- `FileReference` resolved the specification and canonical previous review.
  The prompt's `@prompts/./_reviews/.../review-7.md` reference does not resolve;
  the canonical file is
  `darkmatter/features/2026-07-15-performance-followup/review-7.md`.
- GitNexus classified the raw audit-commit-to-worktree comparison as high risk:
  1,419 changed symbols, 401 changed files, and 14 affected execution flows.
  The raw counts include the existing unrelated `CLAUDE.md` worktree edit; the
  relevant flows remain compose-option classification and transclusion. Focused
  impact analysis found two direct consumers of `classify_options` and the
  `run_compose_pipeline` process in its upstream radius.
- `sniff` mapped the affected scope to `darkmatter`, `darkmatter-cli`,
  `biscuit-terminal`, and `sniff`, with their documented downstream consumers.
  Review 7's implementation cycle changed only feature documentation, so no new
  downstream build scope was introduced.
- `darkmatter-cli`: 18/18 focused spawned-CLI tests passed across
  `compose_terminal_detection` and `hash_kind_save_diff`.
- `darkmatter`: `benchmark_manifest_matches_recorded_identities` passed, proving
  the committed fixture manifest still matches the fixture bytes; the
  deterministic parked-waiter notification test also passed.
- `biscuit-terminal`: 2/2 Level-1 OSC-cache tests passed, including the exact
  request-count structural guard.
- L2 and Browser suites were not rerun because no production or test code has
  changed since their retained real-emulator and headless-browser captures.
- The host failed the pre-capture load condition, so the release benchmark was
  correctly not started.
- `md get` read back every required frontmatter value exactly, and the
  specification passed `md schema validate`. The review could not be
  schema-adjudicated because the repository's existing
  `schemas/feature-review.yaml` is itself rejected as a standalone tagged
  schema: its `$schema` and `description` keys are unsupported. This is
  schema-infrastructure drift, not a defect in this feature.

## Production Readiness

**Not ready.** Complete two admissible quiet-host captures under the committed
`performance-compliance.md` contract and record a passing integrated threshold
verdict before setting `ready: true`.
