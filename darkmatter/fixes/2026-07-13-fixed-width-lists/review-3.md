---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-20T16:42:34-07:00
spec: 2026-07-13-fixed-width-lists/spec.md
log: darkmatter/fixes/2026-07-13-fixed-width-lists/log.md
implemented: true
implemented_by: codex/default
description: "A **fix** review of `2026-07-13-fixed-width-lists/spec.md`"
fix: 2026-07-13-fixed-width-lists/review-3.md
previous: 2026-07-13-fixed-width-lists/review-2.md
---

# Review 3 — Fixed-Width Lists

## Verdict

This fix is **not ready for production**. The Review 2 regressions for wide markers and
preserve-mode idempotence are addressed, and the parse-count budget is enforced. However, cleanup
still changes the CommonMark structure of valid loose nested lists, marker-looking indented code,
and nested lists inside blockquotes. The CLI also removed the specified `--indent 8` mode without
the corresponding specification change, the required timing budgets remain unmeasured, and the
package area's Level-2 gate is red.

## Findings

### High — A loose nested list is flattened after an additional item paragraph

`fix_list_indentation` reconstructs list depth from physical lines. It clears its list stack when
it encounters an unindented blank line, even when that blank line occurs inside a list item. A
valid loose item containing an additional paragraph followed by a nested child therefore loses the
child relationship:

```markdown
- Parent first paragraph.

  Second paragraph.

  - Child item.
```

The CommonMark parser reports the child as a nested list. `md clean --indent 4 -` instead emits:

```markdown
- Parent first paragraph.

    Second paragraph.

- Child item.
```

The child reparses as a top-level sibling. Fixed-width cleanup has the same structural defect. The
current tests exercise an additional paragraph and a nested child independently, but not their
valid combination. This violates AC 1, 7, 9, 10, and 13.

**Suggested resolution:** derive indentation normalization from parser events and source spans, or
carry equivalent container context through serialization, rather than treating blank physical
lines as proof that a list has ended. Add exact-output and structural-fingerprint Level-1 fixtures
for loose items that contain both additional paragraphs and nested lists, through default,
fixed-width, configured-indent, and second-pass cleanup.

### High — Marker-looking indented code is converted into a nested list

The line heuristic treats any recognized marker after indentation as a list item, even when the
parser classified that line as indented code. For example:

```markdown
- Parent.

      - literal code line
```

The source parses as one list item containing an indented code block. `md clean --indent 4 -`
emits the code line at four spaces:

```markdown
- Parent.
    - literal code line
```

That output reparses as a nested list. This contradicts the function documentation's claim that
indented code preserves its relative offset and violates the protected-block and structural
requirements in AC 9 and 10.

**Suggested resolution:** use parser-derived block classification when normalizing indentation and
add Level-1 exact-output, fingerprint, and idempotence tests for indented code beginning with
unordered and ordered list markers. Cover default and configured indentation as well as
fixed-width cleanup. Update the nearby documentation as part of the behavioral correction.

### High — Nested lists inside blockquotes are flattened

The specification explicitly requires nested and blockquoted list combinations. This valid input
contains two nested list levels according to the CommonMark parser:

```markdown
> - Parent.
>   - Child.
```

Both default cleanup and `--indent 2` emit:

```markdown
> - Parent.
> - Child.
```

The result is a single list containing sibling items. Existing blockquote coverage verifies prose
continuations and composed reflow prefixes, but does not verify an actual nested list within a
blockquote. This violates AC 1, 7, 9, 10, and 12.

**Suggested resolution:** preserve the parsed container stack across composed blockquote/list
prefixes. Add structural-fingerprint and exact-output Level-1 fixtures in the library and spawned
CLI, plus compose/DMLS parity fixtures, for nested ordered, unordered, and task lists inside
blockquotes.

### High — Rejecting `--indent 8` conflicts with the current specification

The CLI now rejects `--indent 8`, completion offers only 2 and 4, and the CLI documentation was
updated to match. That is internally consistent and avoids emitting a value that narrow CommonMark
markers cannot always represent. It does not, however, satisfy the reviewed specification: the
test plan explicitly requires configured 2-, 4-, and 8-space nesting, the non-goals state that
`--indent` is not changed, and AC 14 prohibits a public CLI schema change. The library's
eight-column source fixture does not exercise an accepted configured value.

**Suggested resolution:** either ratify a specification and compatibility change that removes 8,
including an explicit migration decision, or support the configured value with a serialization
policy that cannot change the parse tree. Until that decision is reflected in the specification,
AC 7 and 14 fail.

### High — Required performance timing evidence is still deferred

The deterministic parse-count tests pass: default and cleanup variants use one parse, while
fixed-width paths use two. The Criterion timing requirements in AC 15 remain outstanding.
`deferred-performance-tests.md` explicitly records the timing run as deferred, so there is no
same-host evidence for the default-cleanup 10% budget, fixed-width-list 15% budget, or the
fixed-width-versus-full-cleanup 2x ceiling.

**Suggested resolution:** run the specified baseline and candidate Criterion samples on the same
admissible host and record the commands, medians, deltas, and pass/fail verdicts. The stack-based
pass appears linear, but code shape is not a substitute for the required measurement.

### High — The package area's Level-2 acceptance gate is failing

`just test-l2` passed all 19 library Level-2 tests, then stopped in `darkmatter-cli`:
`level2_align_code_block_center_indents_more_than_left` failed three of four attempts before
passing, and `level2_code_block_clears_inherited_dim_before_theme_colors` failed all four attempts.
A focused rerun of the latter also failed all four attempts. Consequently 66 of 69 CLI Level-2
tests did not run and the DMLS Level-2 suite was not reached. These failures do not appear specific
to list cleanup, but AC 16 requires the affected package area's Level-2 gate to pass.

**Suggested resolution:** resolve or explicitly re-scope the independent terminal-rendering
failures, then rerun the complete area Level-2 recipe and record a green result.

## Requirement-to-Verification Assessment

The transformation requirements are deterministic source-to-source behavior, so Level 1 is the
appropriate primary verification tier. No requirement depends on a terminal emulator's keyboard
encoder, paste/IME handling, mouse input, or OS key injection; Level 3 is not applicable. AC 16
explicitly requires Level 2 and is not satisfied.

| Requirement | Strongest relevant verification | Assessment |
| --- | --- | --- |
| AC 1 — list prose cleanup and nested/blockquoted combinations | Level 1 library and spawned CLI | **Fail:** loose nested and blockquoted nested structures are flattened. |
| AC 2 — continuation indentation is not emitted as prose spaces | Level 1 exact-output tests | Pass for represented prose-continuation fixtures. |
| AC 3 — configured fixed-width output | Level 1 exact-output tests | Pass for represented 2- and 4-space fixtures; the specified 8-space configuration is unavailable. |
| AC 4 — preserve incidental newlines | Level 1 library and spawned CLI, including second pass | Pass for represented authored soft breaks. |
| AC 5 — ordered, unordered, task, nested, additional-paragraph, blockquote, and wide-marker coverage | Level 1 matrix | **Fail:** the matrix omits the failing combinations above. |
| AC 6 — tabs and mixed indentation | Level 1 exact-output and fingerprint tests | Pass for represented fixtures. |
| AC 7 — configured nesting widths and composed prefixes | Level 1 library and spawned CLI | **Fail:** configured 8 is rejected and nested blockquoted prefixes lose depth. |
| AC 8 — display-width wrapping | Level 1 width assertions | Pass for represented fixtures. |
| AC 9 — semantic boundaries and protected blocks | Level 1 fingerprints and exact output | **Fail:** marker-looking indented code and valid nested-list boundaries change meaning. |
| AC 10 — structural fingerprints | Level 1 fingerprint tests | **Fail:** missing combinations permit parse-tree changes. |
| AC 11 — whitespace and paragraph-spacing matrix | Level 1 exact-output tests | **Fail:** a blank line within a loose item incorrectly terminates indentation context. |
| AC 12 — compose and DMLS parity | Level 1 parity tests | **Fail:** parity fixtures omit nested lists inside blockquotes; shared incorrect output is not semantic parity. |
| AC 13 — cleanup-mode behavior and idempotence | Level 1 mode and second-pass tests | **Fail:** the loose nested-list case is structurally incorrect on the first pass. |
| AC 14 — no public API or CLI schema change | Level 1 CLI parser/completion tests | **Fail:** accepted `--indent 8` behavior was removed without changing the specification. |
| AC 15 — parse and performance budgets | Level 1 parse counters; Criterion timing absent | **Fail:** parse counts pass, but all three timing verdicts are missing. |
| AC 16 — scoped build, Level 1, Level 2, lint, and impact evidence | Build/L1/lint pass; Level 2 fails | **Fail:** the CLI Level-2 recipe is red and DMLS Level 2 was not reached. |

## Verification Performed

- `just build`: passed for `darkmatter`, `darkmatter-cli`, and `dmls`.
- `just test`: passed (5,889/5,889 library, 617/617 CLI, 568/568 DMLS).
- `just lint`: passed for all three affected packages.
- `just test-l2`: failed in `darkmatter-cli` after 19/19 library tests passed; the focused failing
  terminal-style test also failed on all four attempts.
- Manual CLI and CommonMark-parser comparisons reproduced each structural defect above.
- `sniff` identified the affected package area as `darkmatter`, covering `darkmatter`,
  `darkmatter-cli`, and `dmls`, with downstream consumers of the public library and CLI.
- GitNexus reports `fix_list_indentation` at **critical** upstream impact (141 affected symbols,
  including two direct callers) and `cleanup_content_with_indent` at **high** impact (14 affected
  symbols, including 12 direct callers). The feature-range change report remained confined to the
  expected cleanup, CLI, test, documentation, and planning surfaces, but that does not reduce the
  independent call-graph risk of incorrect structural output.

## Production Readiness

`ready: false` is required until the three structural transformations preserve their input parse
trees, the `--indent 8` contract is reconciled with the specification, the mandated timing evidence
is recorded, and the complete Level-2 gate passes.
