---
$schema: feature-review.yaml
ready: false
findings:
  - priority: high
    title: Narrow terminals corrupt underscore-heavy metric rows
  - priority: high
    title: The new terminal rendering has no Level 2 verification
human_review: false
reviewed_by: codex/gpt-5.6-sol
created: 2026-09-15T18:45:24-07:00
spec: corrected-perf-flag/spec.md
implemented: true
implemented_by: claude/opus
log: sniff/fixes/corrected-perf-flag/implementation-log.md
next: corrected-perf-flag/review-2.md
description: "A **fix** review of `corrected-perf-flag/spec.md`"
fix: corrected-perf-flag/review-1.md
---

# Review 1 — Corrected Perf Flag

## Verdict

The fix is **not ready for production**. The report-to-tree projection is
otherwise faithful to the specification: timing and counter hierarchies remain
separate, measured and synthetic values have the requested semantics, HOT
selection is deterministic, focused reports work without `detect.total`, and
the existing stdout/stderr/JSON routes remain intact. The full Sniff Level 1
suite and lint gate pass.

The implementation nevertheless ships a known rendering defect on narrow
terminals and has no real-terminal test for the new user-visible layout. The
defect is reachable with production counter names and contradicts the outcome's
scan-friendly, unit-aligned rendering requirement. No human review is required
yet: both findings have direct technical remediations and can be verified by an
agent.

## Findings

### High — Narrow terminals corrupt underscore-heavy metric rows

The projection deliberately passes labels to `MetricsTree` unescaped even
though its implementation note records that terminals narrower than 49 columns
can acquire unintended italics and misaligned value columns
([`perf_tree.rs:178`](../../cli/src/output/perf_tree.rs#L178)). The failure is
in the shared renderer: it truncates and pads the raw label, then sends that
text through `Prose` markup without escaping it
([`metrics_tree.rs:321`](../../../biscuit-terminal/lib/src/components/metrics_tree.rs#L321)).
If truncation lands immediately after an underscore, the appended ellipsis
turns that underscore into a markup delimiter. A later underscore can close the
span across rows, consuming visible characters and invalidating the column
width calculated before markup parsing.

This is not an invented malformed input. Sniff's production stage and counter
names include long underscore-heavy segments such as
`classified_embedded_language_hint`, and the implementation log's width sweep
reproduced corruption in both Unicode and ASCII modes from 27 through 48
columns. Shipping the defect because the correction belongs in
`biscuit-terminal` does not satisfy R2's width/alignment contract.

Fix `MetricsTree::build_markup` so literal label text is escaped after
truncation and padding, before it is embedded in component markup. Add a
component regression using the exact underscore/ellipsis input across the
failing widths, then retain a Sniff regression using the production key set.
Do not escape labels in Sniff before width calculation; the existing analysis
correctly shows that doing so makes the component's visible-width accounting
wrong at every width.

Strongest verification present: a temporary Level 1 rendering spike recorded
in the implementation log. It demonstrates the failure rather than proving the
required behavior.

### High — The new terminal rendering has no Level 2 verification

The changed user-facing contract includes hierarchy, aligned duration/count
columns, Unicode/ASCII connectors, the styled HOT marker, truncation, and width
degradation in an actual terminal. Current tests construct a synthetic
`Terminal` in-process for glyph assertions
([`perf_tree.rs:869`](../../cli/src/output/perf_tree.rs#L869)) or spawn the CLI
with captured pipes and `--plain`
([`cli.rs:479`](../../cli/tests/cli.rs#L479)). Those are Level 1 tests: they
verify generated strings and routing, but no terminal emulator interprets the
SGR sequences, glyph widths, or wrapping. Sniff's two existing Level 2 tests
cover unrelated Git and CI/CD output; neither invokes `--perf`.

Add a `level2_` Sniff test through the shared terminal harness. Run a
representative `--perf` command inside the real terminal, capture the completed
pane, and assert the timing hierarchy, separate counter tree, HOT row, aligned
values, and no wrapped/corrupted rows. Include a narrow pane that exercises the
underscore truncation boundary after the component defect is fixed. No Level 3
test is needed because this feature has no keyboard, mouse, paste, or IME
interaction.

Strongest verification present: Level 1 in-process rendering and real-binary
subprocess tests. The required boundary for visual terminal rendering is Level
2, so this is a readiness gap under the review's test-rigor rules.

## Requirement Verification Levels

| Requirement | Strongest verification present | Assessment |
|---|---|---|
| R1 — Existing collection, worker propagation, and request isolation remain intact | Level 1 library unit/integration tests in the full Sniff suite | Correct level; all remain green and the collector seam is unchanged. |
| R2.1 — Root, dotted hierarchy, detection aliases, measured/synthetic values, calls, and ordering | Level 1 pure projection tests | Correct level and representative coverage, including the live production key set. |
| R2.2 — Wall-clock shares and overlap note | Level 1 projection/render tests | Correct level for calculation and text presence. Actual italic rendering of the note is part of the missing Level 2 visual proof. |
| R2.3 — Exactly one deterministic HOT measured stage | Level 1 projection tests | Correct level for selection. The styled marker's terminal appearance lacks Level 2 proof. |
| R3 — Separate generic counter tree with count ordering and unknown shares | Level 1 projection and real-binary subprocess tests | Correct level for structure and routing. Real-terminal alignment remains unverified. |
| R4 — stdout/stderr routing, one JSON document, and ANSI-free `--plain` output | Level 1 real-binary subprocess tests | Correct level; the tests distinguish rich stdout, scriptable stderr, JSON stdout, and plain output. |
| R2/R4 — Width handling, glyphs, SGR styling, unit alignment, and terminal degradation | Level 1 synthetic-terminal/string tests only | **Gap:** requires Level 2 pane capture; a narrow-width failure is already known. |
| Keyboard, hotkey, paste, IME, or mouse behavior | No changed requirement | Level 3 is not applicable. |

## Validation Performed

- `CARGO_TARGET_DIR=<isolated-temp-dir> just test` passed: 2,660 tests, 0
  failed, 24 pre-existing tier skips.
- `CARGO_TARGET_DIR=<isolated-temp-dir> just lint` passed with no lint errors.
- `git diff --check` passed.
- GitNexus could not resolve the uncommitted new projection symbols and
  returned `risk: UNKNOWN`; source search confirmed
  `render_performance_section` has one production caller through
  `CliPerf::emit`, while the new tree helpers remain confined to the output
  module and its tests.
- The first test attempt against the shared Cargo target failed because cached
  `.rmeta` files were not writable; the isolated target rerun avoided that
  environmental cache issue without changing source or test scope.

## Production Readiness

Not ready. The data projection and CLI stream contracts are sound, but the
human report is known to corrupt real production labels at narrow widths and
the visual terminal contract has only Level 1 verification.
