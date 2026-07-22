---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-17T21:42:42-07:00
spec: 2026-07-15-performance-followup/spec.md
implemented: true
description: "A **feature** review of `2026-07-15-performance-followup/spec.md`"
feature: 2026-07-15-performance-followup/review-9.md
previous: 2026-07-15-performance-followup/review-8.md
next: 2026-07-15-performance-followup/review-10.md
---

# Review 9 — Performance Follow-up

## Verdict

This feature is **not ready for production**. Review 8's implementation cycle
changed only feature documentation and closed no release gate. No new
implementation defect or verification-level mismatch was found, but acceptance
criteria 5 and 6 remain open because the integrated compose-regression threshold
still has no admissible pass/fail determination.

## Tracked Production-Readiness Blocker

The required integrated compose-regression measurement remains unestablished.
The retained point estimate cannot close the feature because identical-code and
cross-run drift exceeded the 5% effect being adjudicated. The committed
`performance-compliance.md` contract still requires two independent quiet-host
runs of all six cases, complete load logs, less than 1% identical-code drift,
base means agreeing within 1.5%, and a recorded `compose_trivial` verdict against
the 5% threshold.

Review 9 checked host admissibility before capture. The host reported load
averages of **55.05** (1 minute), **70.90** (5 minutes), and **56.41**
(15 minutes), so the 1-minute reading exceeded the predeclared **2.0** ceiling
by more than 27 times. No benchmark was run and no threshold verdict is claimed
from an inadmissible host.

## Findings

No new implementation findings. The tracked release-evidence gap above remains
release-blocking; per the specification's deferred-measurement contract, it is
not duplicated as a new implementation finding on every review iteration.

## Review 8 Closure

Review 8 contained no implementation finding. Its implementation cycle correctly
made no Rust or test change, marked the review implemented, rechecked the three
committed benchmark pins, and recorded two declined high-load checks in
`performance-compliance.md`. Those documentation changes do not satisfy the
outstanding measurement contract.

## Requirement Verification Levels

| User-observable requirement | Strongest verification | Assessment |
| --- | --- | --- |
| F1: Darkmatter avoids NTP work while the bare Sniff API retains full-report behavior | Level 1 injectable decision seams and no-network compose tests | Appropriate. This is API/subprocess-selection behavior and does not depend on a real terminal. |
| F2: repeated terminal construction reuses one OSC 10 result | Level 2 in real WezTerm on macOS and retained real Kitty on Linux, plus Level-1 manufactured-PTY request counting | Appropriate. Real emulators verify OSC parsing and response; Level 1 supplies the exact request-count oracle. |
| F3: verbose, performance, and warning paths perform one terminal detection | Level 1 spawned CLI with debug-event counting | Appropriate. This is process-local behavior, not terminal rendering or input encoding. |
| F4: TOC output remains compatible while line lookup scales | Level 1 unit/property coverage plus same-byte CLI benchmark artifacts | Appropriate. The requirement concerns deterministic output and runtime, not terminal-emulator behavior. |
| F11–F14 and F33: interpolation, replacement, and remote-discovery behavior remains byte-compatible | Level 1 unit/integration tests plus same-byte target/control benchmarks | Appropriate. These are parser and compose semantics with no terminal dependency. |
| F17/F32: shell ordering, approval behavior, timeout, cleanup, and parked-peer notification remain compatible | Level 1 process and deterministic concurrency tests, with retained Linux and Windows behavior evidence | Appropriate. These semantics do not depend on a terminal emulator. |
| F22: directory membership, aggregate hash, diagnostics, and exit status remain compatible | Level 1 library and spawned-CLI tests with retained macOS, Linux, and Windows execution | Appropriate. No terminal behavior is asserted. |
| F23: code-theme output remains stable while separate renders observe allowed environment changes | Level-1 snapshots, headless-browser computed rendering, and retained Level-2 terminal rendering | Appropriate for browser and terminal presentation. |
| F35.5: hash explanation, persistence, diagnostics, and exit status remain compatible | Level 1 library and spawned-CLI tests | Appropriate. These are file and process semantics. |
| Feature closeout: integrated compose regression is no more than 5% | Benchmark evidence contract, not an L1/L2/L3 test | **Open.** Neither retained run satisfies the predeclared admissibility contract, so there is no production-grade pass/fail result. |

The feature specifies no keypress, hotkey, paste, IME, or mouse behavior, so
Level-3 OS input injection is not applicable. No user-observable requirement is
verified only below the level its behavior requires.

## Verification Performed

- `FileReference` resolved the specification. The prompt's
  `@prompts/./_reviews/.../review-8.md` reference does not resolve; the canonical
  previous review is
  `darkmatter/features/2026-07-15-performance-followup/review-8.md`, which was
  updated instead.
- The review-8 implementation delta changes only `log.md`,
  `performance-compliance.md`, and `review-8.md`. It changes no Rust symbol,
  test, manifest, or build recipe, so there is no new symbol blast radius for
  GitNexus to analyze and no new downstream package gate scope.
- `sniff` confirmed the repository package/package-area catalogs and the
  Darkmatter package-area context. The affected implementation scope remains
  documentation-only for this review cycle.
- The three required benchmark revisions remain committed objects:
  `51c1f16e10ffe825b56987573ba4eabc659c768e`,
  `e15b1cc22b113a9b24058207d760cd879fa62eb6`, and
  `92a3d502eb65c30205a9a255dd13dd8dc6d0aabf`.
- The canonical `just sanity` gate was stopped after **105.319 s** because it
  exceeded the non-interactive subprocess limit on a host whose 1-minute load
  was 55.05. At interruption, **2,601/2,601 executed tests had passed**, 69 were
  skipped, and 2,570 had not run. This is partial evidence, not a completed
  green gate.
- L2 and Browser suites were not rerun because no production or test code has
  changed since their retained real-emulator and headless-browser captures.
- The host failed the pre-capture load condition, so the release benchmark was
  correctly not started. The declined attempt is recorded in
  `performance-compliance.md`.
- The specification passed `md schema validate`. The review itself cannot be
  schema-adjudicated because the repository's existing
  `schemas/feature-review.yaml` is rejected as a standalone tagged schema: its
  `$schema` and `description` keys are unsupported. This is existing schema
  infrastructure drift, not a defect in this feature.
- Final required frontmatter was read back with `md get`, and
  `git diff --check` passed.

## Production Readiness

**Not ready.** Complete two admissible quiet-host captures under the committed
`performance-compliance.md` contract and record a passing integrated threshold
verdict before setting `ready: true`.
