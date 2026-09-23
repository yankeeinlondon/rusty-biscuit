---
$schema: feature-review.yaml
ready: true
agent: codex/default
created: 2026-07-21T09:04:14-07:00
spec: 2026-07-14-invalid-frontmatter/spec.md
implemented: false
description: "A **feature** review of `2026-07-14-invalid-frontmatter/spec.md`"
feature: 2026-07-14-invalid-frontmatter/review-4.md
previous: 2026-07-14-invalid-frontmatter/review-3.md
---

# Review 4 — Invalid Frontmatter

## Verdict

This feature is **not ready for production**. Review 3's two functional defects
are fixed: every later-pass JSON diagnostic and repair now maps back to authored
document coordinates, including after multiple shrinking or expanding edits,
and frontmatter reconstruction preserves authored delimiter bytes. The focused
Level-1 suites, the complete `darkmatter-cli` Level-1 suite, scoped builds, and
warnings-denying lint all pass.

The remaining blocker is acceptance evidence, not a newly found code defect.
The specification requires a retained no-regression timing comparison for the
no-frontmatter and already-clean-frontmatter hot paths, and the ratified matrix
requires native macOS, Linux, and Windows compile/test evidence. The current
candidate still has neither a valid main/branch/main Criterion bracket nor
Linux/Windows runtime results. Marking the prior finding “deferred” does not
satisfy those production acceptance requirements.

Sniff identifies the affected scope as `biscuit-file`, `darkmatter`, and
`darkmatter-cli`, with `dmls` as a downstream consumer in the `biscuit-file`
and `darkmatter` package areas. GitNexus reports LOW risk for both changed
production symbols: `repair_frontmatter` has one direct caller (`run_clean`),
and `line_col_of_offset` directly feeds JSON diagnostic projection plus its
compatibility test.

## Findings

### High — Required performance and cross-platform acceptance evidence remains absent

DECISION: this is considered non-blocking for now!

The spec's Performance section requires no measurable regression for documents
without frontmatter and documents with already-clean frontmatter. The ratified
acceptance matrix also requires successful native tests on macOS, Linux, and
Windows for both affected package areas. `deferred-performance.md` explicitly
records that no admissible Criterion comparison was run and that no current
Linux or Windows runtime result was retained.

Counter tests establish the intended short circuits—zero schema/trigger work
without frontmatter and one parse for clean frontmatter—and the corrected
Criterion vehicle compiles. Those are valuable structural checks, but they do
not measure regression. This review adds current macOS build/test evidence but
cannot manufacture the missing quiet-host bracket or native Linux/Windows
evidence from static portability inspection.

Close the existing deferred procedure: retain a clean main/candidate/main
Criterion bracket for each hot path separately, reject it if the two main runs
drift beyond noise, and retain successful scoped native gates for both package
areas on all three supported operating systems. Until those binding acceptance
rows are green, `ready` must remain false.

## Test-level assessment

| User-facing requirement | Strongest verification | Assessment |
|---|---|---|
| Source normalization, deterministic whitespace cleanup, and reserved-indicator repair through file/stdin/`--save` | Level 1 library, property, and spawned CLI | Appropriate and passing, including BOM, CRLF, lone CR, idempotency, and untouched-byte cases. |
| Schema-proven quoting and compose-parity schema resolution | Level 1 library and spawned CLI | Appropriate and passing for baseline, inline/file/root-union schemas, trigger discovery, overrides, and stdin isolation. |
| Report-only diagnostics on STDERR with stable exit code | Level 1 spawned CLI | Appropriate and passing. The contract does not require terminal-specific style, glyph, or width behavior. |
| Version-1 JSON fields, failure envelope, authored spans, and applied-repair audit | Level 1 spawned CLI plus adversarial byte probes | Appropriate and passing after both expanding and multiple shrinking earlier edits, including CRLF/lone-CR line and column projection. |
| Body fenced YAML remains outside analysis | Level 1 spawned CLI with byte assertions | Appropriate and passing. |
| Idempotency, safety gates, and untouched-byte preservation | Level 1 property and integration tests | Appropriate and passing; authored delimiter slices are now preserved. |
| Hot-path no-regression | Level 1 counters and a compiling Criterion vehicle | **Gap.** Required comparative timing has not been run or retained. |
| macOS, Linux, and Windows operation | macOS build/L1 execution plus static portability review | **Gap.** Current native Linux and Windows runtime evidence is absent. |

No feature requirement concerns terminal-emulator input encoding, OS keyboard
or mouse delivery, or renderer-specific SGR/glyph behavior. Level 2 and Level 3
tests are therefore not required for this feature contract.

## Verification performed

- Focused feature gates: **125 passed**, 0 skipped. One CLI test retried once
  after nextest reported leaked handles; it passed on retry and passed cleanly
  in the subsequent canonical Level-1 package run.
- `biscuit-file`: both crates built; **685 Level-1 tests passed** with 4
  intentionally skipped; package-area lint passed.
- `darkmatter-cli` canonical Level-1 filter: **641 passed**, 71 higher-tier
  tests skipped.
- `darkmatter`, `darkmatter-cli`, and `dmls`: scoped builds passed and
  `cargo clippy --all-targets -- -D warnings` passed.
- The broader Darkmatter area run was stopped at the non-interactive 60-second
  limit after **2,573 passed**, 140 skipped, and no failures; it is not claimed
  as a completed gate.
- An accidentally unfiltered CLI run executed higher-tier tests and found one
  unrelated existing Level-2 schema-theme failure (711 passed, 1 failed). It
  is outside this feature's requirements and was excluded by the canonical
  Level-1 rerun above.
- Direct JSON probing confirmed authored spans after five earlier shrinking
  whitespace repairs followed by schema-proven quoting.

Criterion timing and native Linux/Windows gates were not run in this review.
