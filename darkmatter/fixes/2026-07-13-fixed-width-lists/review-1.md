---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-18T03:57:11-07:00
spec: 2026-07-13-fixed-width-lists/spec.md
implemented: true
implemented_by: claude/default
log: darkmatter/fixes/2026-07-13-fixed-width-lists/log.md
description: "A **fix** review of `2026-07-13-fixed-width-lists/spec.md`"
fix: 2026-07-13-fixed-width-lists/review-1.md
---

# Review 1 — Fixed-Width Lists

## Verdict

This fix is **not ready for production**. The core list-prose collapse and hanging-prefix behavior
works for the representative ordered, unordered, task, nested, and blockquoted fixtures, and the
focused Level-1 suites pass. However, fixed-width cleanup currently corrupts valid link-reference
definitions, treats a ten-digit numeric prose prefix as an ordered-list marker, has no admissible
performance evidence against the specification's mandatory budgets, and has not completed the
required full Level-1/Level-2 gates.

## Findings

### High — Fixed-width cleanup corrupts link-reference definitions

The specification explicitly protects link-reference definitions (`spec.md:364`) and requires
protected child blocks to remain structurally intact (AC 9). The semantic reflow map only records
parsed code, table, and HTML spans as protected (`cleanup/reflow/semantic.rs:104-109`), while the
fallback structural-line classifier does not recognize reference definitions
(`cleanup/reflow.rs:536-546`). Consequently, the physical-line reflow path tokenizes and wraps the
definition as prose.

A freshly built CLI reproduces the defect:

```text
input:
- Before [label][ref]

    [ref]: https://example.com/a/very/long/path "A descriptive title"

md clean --fixed-width 24 output:
- Before [label][ref]

[ref]:
https://example.com/a/very/long/path
"A descriptive title"
```

The resulting lines are no longer a valid reference definition, so the cleaned document's link is
broken. The protected-child test matrix covers fences, indented code, tables, HTML, and shell
blocks, but omits reference definitions (`cleanup/tests/reflow.rs:736-748`).

**Suggested resolution:** add source-range protection for parsed reference definitions before
physical-line wrapping. Add an exact-output Level-1 fixture with a used definition through
`reflow_to_width`, `md clean --fixed-width`, compose, and DMLS; reparse the result and prove the
reference still resolves.

### High — Ten-digit numeric prose is misclassified as an ordered list

The specification states that CommonMark ordered-list markers contain one to nine digits and wider
runs remain ordinary prose (`spec.md:191-192`). `ordered_marker_prefix_len` accepts an unbounded
digit run (`cleanup/reflow.rs:779-792`). A freshly built CLI therefore changes this ordinary prose:

```text
input:
1234567890. Alpha beta gamma delta epsilon zeta.

md clean --fixed-width 24 output:
1234567890. Alpha beta
            gamma delta
            epsilon
            zeta.
```

This violates the explicit marker rule and AC 14's compatibility requirement.

**Suggested resolution:** reject ordered-marker candidates with more than nine digits and add
boundary tests for nine-digit markers versus ten-digit prose on direct-library and CLI surfaces.

### High — Mandatory performance budgets have no benchmark evidence

The new default path scans protected lines and clones/transforms the parsed event stream, while
fixed-width reflow now builds a semantic map with another parser pass. Those choices may be sound,
but the specification requires same-host measurements for top-level prose, flat lists, deeply
nested lists, and blockquoted task lists, with default cleanup within 10%, fixed-width list cleanup
within 15%, and fixed-width remaining below 2x full cleanup (`spec.md:631-644`). No focused
benchmark, baseline/candidate results, parse-count evidence, or threshold verdict was added to the
fix artifacts. The plan contains only assertions that the parse sequence is acceptable.

**Suggested resolution:** add or extend the focused cleanup Criterion benchmark, capture a
same-host pre-fix baseline and candidate comparison for all required fixtures, and record explicit
pass/fail calculations for all three budgets.

### High — The required full Level-1 and Level-2 gates are incomplete

Acceptance criterion 16 requires area build, Level 1, Level 2, lint, and bounded GitNexus change
detection (`spec.md:711-712`). This review's `just build` and `just lint` passed, but the bounded
`just test` attempt was interrupted after 2,523 of 5,795 Darkmatter tests passed; 3,272 Darkmatter
tests plus the CLI and DMLS portions did not run. The implementation record also reports that
`just test-l2` passed 19 Darkmatter tests, then failed repeatedly in
`level2_code_block_clears_inherited_dim_before_theme_colors`, preventing 66 remaining CLI tests and
the DMLS Level-2 tier from running (`plan.md:496-504`). An unrelated failure still leaves the
specified release gate open.

**Suggested resolution:** run the complete area `just test` recipe in an environment without the
session time limit, resolve or isolate the existing Level-2 color failure through the repository's
normal test policy, then obtain a complete green `just test-l2` run.

### Medium — The structural fingerprint omits required list semantics

The fingerprint records only `ordered` versus `unordered` for `Tag::List`; it discards the starting
ordinal (`cleanup/tests/reflow.rs:31-38`). It also treats `Event::TaskListMarker(_)` only as a signal
to open an implicit paragraph and never records checked/unchecked state
(`cleanup/tests/reflow.rs:90-96`). The specification explicitly requires both the ordered-list
starting ordinal and task state in the structural fingerprint, so AC 10 is not actually proved.

**Suggested resolution:** include the `Option<u64>` start value and task Boolean in fingerprint
entries, and apply the helper to ordered and checked/unchecked task fixtures.

### Medium — Two required Level-1 boundary fixtures are missing

The configured-nesting test covers only indentation widths 2 and 4
(`cleanup/tests/reflow.rs:630-650`), although the test plan requires 2, 4, and 8. The hard-break
test verifies ordinary two-space and backslash breaks that remain within width
(`cleanup/tests/reflow.rs:694-710`), but not the required indivisible suffix overflow case. Both
are explicit test-plan requirements and support AC 7/8 boundary claims.

**Suggested resolution:** add an 8-space nested-list fixture and a hard-break case where prefix +
one atomic body token + suffix necessarily exceeds the target width, asserting that the suffix is
preserved and the overflow is limited to the documented exception.

## Requirement-to-Verification Assessment

All changed behavior is deterministic Markdown source transformation. Level 1 is the appropriate
behavioral verification level; no requirement depends on terminal-emulator rendering or OS input,
so feature-specific Level 2 or Level 3 tests are not required. The specification separately makes
the area Level-2 recipe a release gate under AC 16, and that gate remains incomplete.

| AC | Requirement | Strongest verification | Assessment |
|---|---|---|---|
| 1 | Collapse eligible list soft breaks at all nesting/quote depths | Level 1 exact library tests plus spawned CLI/compose tests | Pass for covered marker/container forms. |
| 2 | Remove layout prefix and use the existing Unicode separator | Level 1 exact strings and Unicode matrix | Pass. |
| 3 | Strip mode emits no hanging whitespace | Level 1 exact strings and idempotence assertion | Pass. |
| 4 | Preserve mode performs no list collapse or synthesis | Level 1 library, compose, and spawned CLI tests | Pass. |
| 5 | Fixed width unwraps the complete logical list paragraph | Level 1 exact library/CLI/DMLS fixture | Pass for ordinary prose. |
| 6 | Created lines use complete aligned container prefixes | Level 1 exact nested/blockquote/task output plus width assertions | Pass for covered forms. |
| 7 | Per-item digit/task/nesting/quote prefixes | Level 1 exact matrix | Partial: no 8-space fixture; ten-digit prose is misclassified. |
| 8 | Total display width obeys documented overflow exceptions | Level 1 `UnicodeWidthStr` assertions | Partial: required hard-break-suffix overflow fixture is absent. |
| 9 | Paragraph/item/child-block structure remains intact | Level 1 exact output and fingerprint helper | Fail: reference definitions are split into invalid Markdown. |
| 10 | Fingerprint preserves required list semantics | Level 1 fingerprint comparison | Fail: starting ordinal and task state are not recorded. |
| 11 | Normal/compact/loose spacing remains compatible | Level 1 mode matrix | Pass. |
| 12 | Equivalent library, compose, CLI stdout/save, and DMLS sequences agree | Level 1 cross-surface parity tests | Pass for the representative list fixture. |
| 13 | Default, preserve, and fixed-width cleanup are idempotent | Level 1 fixed-point tests and recorded CLI checks | Partial: fixed width is pinned; preserve is not retained as a dedicated regression fixture. |
| 14 | Public API, CLI schema, marker rules, dependencies, and platform behavior remain stable | Source/API inspection and Level 1 tests | Fail: the explicit one-to-nine-digit marker rule is violated. |
| 15 | Parse/performance budgets are satisfied | Required Criterion comparison | Fail: no admissible measurement exists. |
| 16 | Area build/L1/L2/lint and bounded impact gates pass | `just` recipes plus GitNexus | Fail: L1 and L2 recipes did not complete green. |

No keypress, hotkey, paste, IME, mouse, or terminal-rendering requirement exists, so there is no
L1/L2/L3 verification-level mismatch beyond the separately mandated incomplete AC 16 gate.

## Verification Performed

- GitNexus upstream impact is **CRITICAL** for both core symbols:
  `strip_incidental_newlines` has 34 direct / 178 total dependents;
  `reflow_to_width` has 5 direct / 25 total dependents and reaches two CLI process families.
- `sniff` identifies the affected area as `darkmatter`, `darkmatter-cli`, and `dmls`; it also
  confirms `darkmatter` is consumed by several workspace packages, while the changed cleanup
  entry points' indexed callers remain in the expected cleanup/compose/CLI/DMLS surfaces.
- `just build` from `darkmatter/`: **PASS** for `darkmatter`, `darkmatter-cli`, and `dmls`.
- Focused Level-1 nextest run: **PASS**, 66/66 cleanup/reflow/compose tests.
- Focused CLI nextest run: **PASS**, 2/2 list stdout/save tests.
- Focused DMLS nextest run: **PASS**, 2/2 formatting/idempotence tests.
- `just lint` from `darkmatter/`: **PASS** for all three packages.
- Worktree-scoped GitNexus change detection reports LOW risk across 25 dirty indexed files, 143
  changed symbols, and no affected execution process. Compare-to-`main` is CRITICAL and
  branch-wide (571 files, 2,009 symbols, 57 affected processes), so it is not a cleanly isolated
  oracle for this fix.
- Full `just test`: **INCOMPLETE**, interrupted at the non-interactive command limit after
  2,523/5,795 Darkmatter tests passed with no observed failure; 3,272 Darkmatter tests and the CLI
  and DMLS tiers did not run.
- `just test-l2`: not rerun; the retained implementation evidence records the unrelated repeated
  terminal-color failure and the resulting unexecuted CLI/DMLS remainder.
- Freshly built `md clean --fixed-width 24 -` reproduces both high-severity correctness defects
  shown above.
- Review-schema validation is blocked by existing schema infrastructure drift:
  `schemas/feature-review.yaml` is rejected as a standalone tagged schema because it combines
  unsupported `$schema` and `description` keys with `kind`/`types`. The required frontmatter was
  read back directly and `git diff --check` passed.

This macOS review introduced no platform-specific implementation. Windows and Linux execution was
not available; portability is therefore supported by source inspection, not fresh cross-platform
runtime evidence.

## Production Readiness

**Not ready.** Fix both output-corruption defects, complete the missing Level-1 contract tests,
produce passing performance evidence, and obtain complete green area Level-1/Level-2 gates before
setting `ready: true`.
