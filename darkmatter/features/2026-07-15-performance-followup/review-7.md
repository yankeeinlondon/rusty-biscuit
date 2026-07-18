---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-17T17:46:03-07:00
spec: 2026-07-15-performance-followup/spec.md
implemented: true
implemented_by: claude/default
log: darkmatter/features/2026-07-15-performance-followup/log.md
description: "A **feature** review of `2026-07-15-performance-followup/spec.md`"
feature: 2026-07-15-performance-followup/review-7.md
previous: 2026-07-15-performance-followup/review-6.md
---

# Review 7 — Performance Follow-up

## Verdict

This feature is **not ready for production**. Review 6's only implementation
finding is closed, and this review found no new implementation defect or
verification-level mismatch. The release evidence is still incomplete:
acceptance criteria 5 and 6 require an admissible determination of the
integrated compose-regression threshold, and no qualifying run exists.

## Tracked Production-Readiness Blocker

The integrated compose-regression threshold remains unestablished. This is the
existing deferred measurement owned by `performance-compliance.md`, not a new
implementation finding. The retained comparison's point estimate was within
the 5% limit, but identical-code and cross-run drift exceeded the effect being
adjudicated, so it cannot support either a pass or a fail.

Review 7 rechecked host admissibility before capture. The 1-minute load was
**8.84**, above the predeclared **2.0** ceiling, with 5-minute and 15-minute
loads of 19.21 and 30.74. Starting the three-arm release build and capture in
that state would knowingly produce inadmissible evidence, so no benchmark was
run. The declined attempt is recorded in `performance-compliance.md`.

Production readiness still requires two independent quiet-host runs of the
committed base/before/after pins, all six cases, complete load logs, less than
1% identical-code drift, base means agreeing within 1.5%, and a recorded
`compose_trivial` verdict against the 5% threshold.

## Findings

No new implementation findings. The current blocker is the tracked release
evidence gap above; it remains release-blocking even though it is not duplicated
as a new finding in each review iteration.

## Review 6 Closure

The terminal-detection test documentation now identifies the genuine Level-2
evidence as `biscuit-terminal/lib/tests/level2_terminal_osc_wezterm.rs` and
separately identifies `level1_terminal_osc_cache.rs` as Level 1. The nonexistent
`level2_terminal_osc_cache.rs` reference is gone. This is a comment-only fix;
the underlying tests and their classification are unchanged.

## Requirement Verification Levels

| User-observable requirement | Strongest verification | Assessment |
| --- | --- | --- |
| F2: repeated terminal construction reuses one OSC 10 result | Level 2 in real WezTerm on macOS and retained real Kitty on Linux, plus Level-1 manufactured-PTY request counting | Appropriate. Real emulators parse and answer OSC; Level 1 supplies the exact request-count oracle. |
| F3: verbose, performance, and warning paths perform one terminal detection | Level 1 spawned CLI with debug-event counting | Appropriate. This is process-local behavior, not terminal rendering or input encoding. |
| F21: redirected macOS output avoids appearance discovery | Level 1 spawned CLI with an argument-sensitive `defaults` sentinel | Appropriate. The requirement is absence of a subprocess on a redirected path. |
| F17/F32: shell ordering, approval behavior, timeout, cleanup, and parked-peer notification remain compatible | Level 1 process and deterministic concurrency tests, with retained Windows behavior evidence | Appropriate. These requirements do not depend on a terminal emulator. |
| F22: directory membership, aggregate hash, diagnostics, and exit status remain compatible | Level 1 library and spawned-CLI tests with retained macOS, Linux, and Windows execution | Appropriate. No terminal behavior is asserted. |
| F23: code-theme output remains stable while separate renders observe allowed environment changes | Level-1 snapshots, headless-browser computed rendering, and retained Level-2 terminal rendering | Appropriate for browser and terminal presentation. |
| F35.5: hash explanation, persistence, diagnostics, and exit status remain compatible | Level 1 library and spawned-CLI tests | Appropriate. These are file and process semantics. |

The feature specifies no keypress, hotkey, paste, IME, or mouse behavior.
Level-3 OS input injection is therefore not applicable. No user-observable
requirement has only a lower verification level than its behavior requires.

## Verification Performed

- `FileReference` resolved the specification and canonical previous review.
  The prompt's `@prompts/./_reviews/.../review-6.md` reference does not resolve;
  the existing canonical file is
  `darkmatter/features/2026-07-15-performance-followup/review-6.md`.
- GitNexus classified the accumulated feature delta from audit commit
  `51c1f16e1` as high risk: 400 changed files, 1,407 changed symbols, and 14
  affected execution flows. The affected flows center on compose-option
  classification, transclusion, and terminal discovery. No production symbol
  changed after review 6; the subsequent feature-local delta is documentation.
- `darkmatter-cli`: 18 focused spawned-CLI cases passed, comprising the two
  terminal-detection tests and all 16 `hash_kind_save_diff` cases.
- `darkmatter`: the benchmark-manifest identity test and deterministic
  parked-waiter notification test both passed.
- `biscuit-terminal`: both Level-1 OSC-cache tests passed, including the exact
  request-count regression guard.
- The real-terminal and browser suites were not rerun because review 6's only
  change was documentation. Their retained real-emulator and headless-browser
  evidence remains applicable.
- The current host failed the benchmark's pre-capture admissibility check, so
  no release-threshold result was claimed.
- `md get` read back every required frontmatter value exactly. The specification
  passed `md schema validate`. The review could not be adjudicated because the
  repository's existing `schemas/feature-review.yaml` is itself rejected as a
  standalone schema: tagged schema documents do not support its `$schema` and
  `description` keys. This is schema-infrastructure drift, not a defect in this
  feature's implementation.

## Production Readiness

**Not ready.** Complete the tracked performance-compliance capture under the
predeclared quiet-host conditions and record a passing threshold verdict before
setting `ready: true`.
