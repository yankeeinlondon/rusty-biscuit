---
status: ready for planning and implementation
reviewed: true
reviewed_by: codex/default
reviewed_on: 2026-07-18
review_iterations: 6
created: 2026-07-13
area: darkmatter
packages:
  - darkmatter
  - darkmatter-cli
  - dmls
amends:
  - ../../features/_completed/2026-06-19-cleanup-fixed-line-length/spec.md
---

# List-Aware Incidental-Newline Cleanup and Fixed-Width Reflow

## Problem

Darkmatter's cleanup contract applies to prose, not only to top-level paragraphs. A paragraph
inside an ordered, unordered, or task-list item has the same incidental soft line breaks as a
top-level paragraph and MUST honor the same three modes:

- default cleanup removes incidental soft line breaks;
- fixed-width cleanup removes the source wrapping and then creates canonical wrapping at the
  requested display width; and
- `--ignore-incidental-newlines` leaves source soft line breaks alone.

The shipped implementation does not satisfy that contract for list-item prose. It classifies any
physical line beginning with four spaces or a tab as indented code before considering whether the
line is paragraph continuation content inside a list item. Conventional authored list wrapping
therefore bypasses both `strip_incidental_newlines` and `reflow_to_width`:

```md
- Ratified design: `claudine/features/2026-07-12-rendezvous-dashboard/spec.md`
    (see the "Decisions" section, especially the implementation stamps).
```

Default cleanup currently preserves the physical break:

```md
- Ratified design: `claudine/features/2026-07-12-rendezvous-dashboard/spec.md`
    (see the "Decisions" section, especially the implementation stamps).
```

It MUST instead produce one logical source line:

```md
- Ratified design: `claudine/features/2026-07-12-rendezvous-dashboard/spec.md` (see the "Decisions" section, especially the implementation stamps).
```

The failure is indentation-dependent. A lazy ordered-list continuation using fewer than four
spaces can be collapsed, but the existing code retains the continuation indentation as content.
That turns one soft break into several literal spaces:

```md
1. First source line
   second source line
```

can become conceptually equivalent to:

```md
1. First source line    second source line
```

With `--fixed-width`, only the first physical line of an affected item is wrapped. The untouched
continuation lines retain the source author's arbitrary wrapping and indentation. Composite
containers expose a second correctness defect: the current prefix parser recognizes a blockquote
prefix or a list prefix, but not both, so a wrapped blockquoted list can lose the hanging list
indent on continuation lines.

These are correctness defects, not merely cosmetic differences:

- fixed-width output is not actually fixed-width;
- extra indentation can become visible whitespace in source and rendered plain text;
- a continuation line emitted without the complete container prefix can end a list item or move
  prose to a different container; and
- indented code, nested lists, and second paragraphs cannot be distinguished safely by counting
  leading spaces alone.

The completed fixed-line-length feature already states that list-item bodies are reflowed within
their continuation indentation. This fix closes the gap between that ratified contract and the
shipped implementation.

## Evidence and Existing Flow

The relevant production flow is:

```text
strip_incidental_newlines
    -> cleanup_content_internal (parse, serialize, list/indent normalization)
    -> reflow_to_width when fixed width is requested
```

The same primitives feed four user-visible surfaces:

1. `Markdown::cleanup*` and `Markdown::cleanup_with_fixed_width`;
2. `md clean`, including `--save`;
3. compose's inline-post Cleanup operation; and
4. DMLS whole-document formatting.

The current unit test for mixed list markers starts with each complete item on one physical line.
It proves that newly created wraps align under `-`, `1.`, and `- [ ]`, but it does not exercise
source items that are already wrapped. The test therefore misses the unwrap-before-rewrap failure.

GitNexus impact analysis classifies the eventual implementation as broad:

- `strip_incidental_newlines`: **CRITICAL**, 34 direct and 178 total dependents at depth three when
  tests are included; and
- `reflow_to_width`: **CRITICAL**, 5 direct and 25 total dependents at depth three across cleanup,
  compose, CLI, and DMLS formatting, including two CLI execution-flow families.

The implementation MUST consequently preserve all non-list behavior and prove parity at every
public surface rather than treating this as a CLI-only patch.

## Goals

1. Apply incidental-soft-break cleanup uniformly to prose in ordered, unordered, and task-list
   items at every nesting depth.
2. In default strip mode, remove authored continuation-layout indentation with the soft break. Do
   not synthesize hanging indentation when no physical continuation line remains.
3. In fixed-width mode, unwrap the complete logical list paragraph before wrapping it to the
   requested display width.
4. When fixed-width wrapping creates a continuation line, align its body with the first line's
   body by emitting the complete required container prefix.
5. Preserve list structure and every non-prose child block, including nested lists, additional
   paragraphs, blockquotes, fenced and indented code, tables, HTML blocks, and Darkmatter
   directives.
6. Keep `--ignore-incidental-newlines` neutral with respect to list soft breaks.
7. Keep equivalent full-cleanup sequences byte-equivalent across direct library use, compose, the
   CLI, and DMLS formatting. Preserve the narrower, established transform-only contract of
   `cleanup_to_fixed_width` and `Markdown::cleanup_with_fixed_width`, which is strip then reflow and
   does not independently select a list-spacing or indentation policy.
8. Preserve the existing public API, CLI flags, list-spacing modes, marker rules, Unicode join
   policy, and fixed-width overflow policy.

## Non-goals

This fix does not:

- change `--fixed-width` or `--ignore-incidental-newlines` flag names, ranges, defaults, or their
  mutual exclusion;
- change `--indent`, `ListSpacingMode`, ordered-list renumbering, or unordered-marker restoration;
- broaden `cleanup_to_fixed_width` or `Markdown::cleanup_with_fixed_width` into the full cleanup
  serializer/list-normalization pipeline;
- introduce a separate list formatter or a new public Markdown AST;
- split words, URLs, inline-code spans, or spaceless-script runs that already overflow the target
  width;
- implement kinsoku or other intra-token line breaking;
- reflow fenced code, indented code, tables, HTML blocks, or directive payload bodies;
- alter CommonMark hard-break semantics; or
- guarantee that `--ignore-incidental-newlines` prevents unrelated cleanup normalization such as
  canonical list indentation or trailing-newline normalization.

## Terminology

### Soft break

An incidental newline is a CommonMark soft line break inside one prose block. In the parser this is
an `Event::SoftBreak`. It is not equivalent to every single `\n` byte.

The following are not soft breaks for this feature:

- blank-line paragraph boundaries;
- `Event::HardBreak` from two trailing spaces or an unescaped trailing backslash;
- the boundary between two list items;
- the boundary between a list-item paragraph and a nested child block;
- lines inside fenced or indented code, HTML blocks, tables, or protected Darkmatter blocks; and
- structural Markdown or Darkmatter directive lines.

### List prose block

A list prose block is a CommonMark paragraph contained by a list item. It includes the tight first
paragraph for which pulldown-cmark may suppress explicit paragraph start/end events, loose first or
subsequent paragraphs, lazy continuation lines, and paragraphs inside list items nested in
blockquotes.

### Container prefix

The container prefix is the source syntax from column zero to the first body character. Depending
on context it can contain:

- blockquote markers and their spaces;
- list nesting indentation;
- an ordered or unordered marker;
- a task-list checkbox; and
- the separator after the final marker.

For `>   10. [ ] Body`, the full prefix is every byte before `Body`, not merely `10. `. CommonMark
ordered-list markers contain one to nine digits; wider numeric runs are not list markers and remain
ordinary prose.

### Hanging continuation prefix

The hanging continuation prefix preserves all enclosing container syntax and replaces the current
item's marker, optional task box, and separator with display-width-equivalent ASCII spaces. Its
display width equals the first-line container prefix, so continuation text begins in the same
column as the first body character.

## Required Behavior

### Mode matrix

| Mode | Existing list soft breaks | Newly emitted continuation indentation |
|---|---|---|
| Default (`Strip`) | Collapse within each list prose block | None; the item paragraph becomes one physical source line |
| `--fixed-width N` | Collapse first, then wrap the complete logical prose block | Emit the full hanging continuation prefix on every created line |
| `--ignore-incidental-newlines` | Preserve | Emit none; regular cleanup may still canonicalize existing list syntax |

### Default stripping

For an eligible list soft break, stripping MUST:

1. remove the line ending;
2. remove the next physical line's container and continuation-layout prefix;
3. join the two content boundaries using the existing `join_separator` Unicode-script policy; and
4. emit no other whitespace.

The replacement is therefore either no character or one `U+0020` content separator. Authored
indentation used only to make source wrapping attractive MUST NOT survive as literal content.

This applies equally to:

- `-`, `*`, and `+` unordered items;
- `.` and `)` ordered markers with any CommonMark-valid digit width (one to nine digits);
- checked and unchecked GFM task items;
- lazy and explicitly indented continuation lines;
- nested items at every configured indentation width; and
- list items inside one or more blockquotes.

Default stripping MUST NOT add hanging spaces. Once a list paragraph occupies one source line, the
editor or renderer owns visual word wrapping and hanging-layout presentation.

### Fixed-width list reflow

Fixed-width cleanup MUST operate on the complete logical prose block, never on each authored
physical line independently. Existing soft breaks and their layout indentation are removed before
the word-wrapping decision is made.

For a target width of 24:

```md
- Alpha beta gamma delta
    epsilon zeta eta theta.
```

MUST become:

```md
- Alpha beta gamma delta
  epsilon zeta eta
  theta.
```

The requested width includes the prefix. Except for the existing atomic-token overflow case,
`UnicodeWidthStr::width(physical_line) <= width` MUST hold for every reflowed list line.

Continuation indentation is derived per item from the post-cleanup prefix:

```md
10. Alpha beta gamma
    delta epsilon.

- [ ] Alpha beta gamma
      delta epsilon.
```

The marker width can change between siblings, especially when an ordered list crosses from `9.`
to `10.`. Each item's continuation prefix MUST be computed from that item's actual serialized
marker; it MUST NOT reuse the first item's width.

Synthesized hanging indentation MUST use ASCII spaces, not tabs. Tabs have context-dependent tab
stops and would make display-column guarantees editor-dependent.

### Nested lists

A child list marker is a new block, not continuation prose for its parent. Cleanup MUST preserve the
boundary and reflow each item's own prose independently:

```md
- Parent alpha beta gamma delta epsilon.
    - Child alpha beta gamma delta epsilon.
```

At the default four-space nesting width, fixed-width continuation prefixes are:

```md
- Parent text that wraps
  under its own body.
    - Child text that wraps
      under its own body.
```

The exact nesting indentation continues to come from the existing cleanup/`--indent` policy. The
reflow pass consumes the actual post-cleanup indentation and MUST NOT hard-code two or four spaces.

### Blockquoted lists

Container prefixes compose. A blockquote containing a list is not treated as either a plain quote
or a plain list:

```md
> - Alpha beta gamma delta epsilon zeta.
```

When wrapping is required, continuation lines MUST retain the quote marker and hang under the list
body:

```md
> - Alpha beta gamma
>   delta epsilon zeta.
```

The same rule applies to nested blockquotes and ordered/task lists inside them. A continuation such
as `> delta ...` is not acceptable because it can move the line out of the list item.

### Additional paragraphs within an item

Blank lines remain paragraph boundaries. A second paragraph belongs to the same item only through
its container indentation, which must be preserved:

```md
- First paragraph.

    Second paragraph with enough text to wrap.
```

Default strip mode collapses soft breaks within each paragraph but preserves the blank line and
the second paragraph's required block indentation. Fixed-width mode wraps the second paragraph
with the same block indentation on its first and continuation lines. It does not add the list
marker again.

This rule uses the indentation produced by the existing cleanup pipeline, including an explicit
`--indent` choice.

### Hard breaks

A hard break inside list prose remains a hard break. The following newline MUST survive both the
strip phase and fixed-width reflow:

```md
- First line ends here.  
  Second line remains in the same item.
```

If fixed-width processing wraps either side, every emitted physical line still carries the prefix
required to remain inside the original list/blockquote containers.

The hard-break suffix is inseparable from the preceding content. If the complete prefix, one
otherwise atomic body token, and the required two-space or backslash suffix cannot fit together,
that physical line MAY overflow. This is the same unavoidable-overflow class as an atomic token or
prefix that cannot fit; the hard break MUST NOT be deleted merely to satisfy the width.

### Protected child blocks

List containment does not turn non-prose blocks into reflowable prose. The following remain
verbatim apart from pre-existing cleanup normalization:

- fenced and genuinely indented code blocks;
- HTML blocks;
- GFM tables;
- thematic breaks and headings;
- link-reference definitions where the parser treats them structurally;
- Darkmatter `::` directive lines; and
- `::shell-block` bodies and any future directive body explicitly classified as opaque by the
  shared Darkmatter structural-protection model.

The distinction between a four-space list continuation and indented code MUST come from parsed
block context. A raw `line.starts_with("    ")` check is not sufficient.

## Design Decisions

### Decision 1 — Parsed Markdown structure is authoritative

[CommonMark list-item containment](https://spec.commonmark.org/0.31.2/#list-items) depends on the
marker width, spaces following the marker, enclosing containers, lazy continuation rules, and
relative indentation. Reimplementing those rules in a second line-oriented parser would create a
permanent drift risk.

The implementation MUST use pulldown-cmark's offset event stream, with the same parser options as
cleanup, to identify:

- `Event::SoftBreak` boundaries eligible for collapse;
- active `List`, `Item`, `BlockQuote`, `Paragraph`, `CodeBlock`, `Table`, and HTML contexts; and
- source spans for list prose and protected blocks.

Darkmatter-specific structural protection remains an overlay because pulldown-cmark does not know
the semantics of `::` directives. The existing fence/HTML/shell-block protections may be retained,
but indentation heuristics MUST NOT override parsed evidence that a line is list prose.

The shared structural-protection classification is authoritative for Darkmatter directives. In
v1, `::shell-block` is the only opaque directive-body family in this cleanup path. Disclosure
bodies and conditional `::block` bodies contain Markdown and remain eligible for ordinary
paragraph cleanup; their opener, separator, and closer lines remain structural and protected.

The implementation MAY keep the existing line scanner for non-list prose to minimize regression
risk. Ambiguous list continuation and code classification MUST be resolved from parser structure,
not from a hard-coded indentation threshold.

### Decision 2 — One shared soft-break decision model

Standalone `strip_incidental_newlines` must preserve source syntax outside the removed soft breaks,
while full cleanup already owns an event-stream serialization phase. These paths MUST share one
internal decision model rather than independently rediscovering list boundaries.

The model needs, at minimum, for each candidate boundary:

- the source span of the soft line ending;
- the source span of the next line's syntactic container/continuation prefix;
- whether Darkmatter structural rules protect the boundary; and
- the existing zero-or-one-character join-separator decision.

For the source-preserving primitive, apply non-overlapping edits from the end of the document or
stream into a pre-sized output buffer. Do not repeatedly mutate the middle of a `String`.

For full cleanup, the preferred implementation is to lower eligible `Event::SoftBreak` values to
the chosen text separator before `pulldown-cmark-to-cmark` serialization. Container indentation is
not inline event content, so this naturally prevents authored hanging indentation from leaking
into prose. It also allows the existing cleanup parse to be reused instead of adding another full
parse to the default path.

If a different implementation is chosen, it MUST demonstrate byte-equivalent output and must not
add an unconditional second Markdown parse to every default cleanup operation without benchmark
evidence and explicit review.

### Decision 3 — Reflow logical prose blocks, not physical lines

`reflow_to_width` MUST receive or derive a semantic map of reflowable prose blocks. Its protected
classification must identify actual parsed code/HTML/table blocks; it must not mark every
four-space line as protected.

For each list prose block, reflow derives two prefixes:

- `first_prefix`: the complete prefix used on the block's first output line; and
- `continuation_prefix`: the complete prefix used on subsequent output lines.

For an item's first paragraph, `first_prefix` contains the item marker and
`continuation_prefix` replaces that marker region with equal-width spaces. For a subsequent
paragraph, both prefixes contain the paragraph's required container indentation and neither
contains an item marker.

Prefix parsing MUST compose blockquote and list containers rather than returning early after the
first recognized prefix family.

### Decision 4 — Preserve the existing pass order and public surface

The externally visible order remains:

```text
soft-break collapse when enabled
    -> existing Markdown cleanup/list normalization
    -> fixed-width logical-prose reflow when requested
```

No public function, method, enum, CLI argument, or DMLS setting is renamed or removed. Existing
entry points continue to be authoritative:

- `strip_incidental_newlines`;
- `cleanup_to_fixed_width`;
- `reflow_to_width`;
- `Markdown::strip_incidental_newlines`;
- `Markdown::cleanup*` and `Markdown::cleanup_with_fixed_width`;
- `ComposeOptions::{with_incidental_newline_mode, with_fixed_width}`; and
- DMLS `formatting.fixed_width`.

There are two intentionally different orchestration levels:

1. `cleanup_to_fixed_width` and `Markdown::cleanup_with_fixed_width` retain their established
   transform-only contract: collapse incidental newlines, then call `reflow_to_width`. They do not
   choose a list-spacing mode, force a nesting indentation, align tables, or perform the complete
   cleanup serialization pipeline.
2. `md clean`, compose Cleanup, and DMLS formatting first run their selected full cleanup variant
   and then call `reflow_to_width`. Their parity oracle is the equivalent direct library sequence
   using the same cleanup variant, indentation, incidental-newline mode, and width.

`reflow_to_width` remains the backend for already-cleaned content. It MAY defensively coalesce list
paragraph soft breaks needed to keep a logical item intact, but callers that require whole-document
unwrap-before-rewrap behavior MUST use `cleanup_to_fixed_width` or run the selected cleanup variant
before calling it.

> **Reader's note.** An earlier draft required every public entry point to be byte-equivalent. That
> would silently broaden the two transform-only helpers into full Markdown cleanup and change
> unrelated table, spacing, marker, and indentation behavior. The reviewed contract instead
> requires byte parity only between equivalent pipelines while keeping the existing helper
> semantics explicit.

Private types may be added under `markdown/cleanup/reflow.rs` or a focused child module. Do not add
a generic pass trait or a public list-layout abstraction for this single engine.

### Decision 5 — Width and overflow contracts do not change

The target is Unicode display columns, including every container-prefix column. The existing
atomic-token rule remains: a token wider than the available body width is emitted intact and may
overflow. A complete prefix that already meets or exceeds the requested width cannot be made to
fit; the following body token is likewise emitted intact rather than split. A required hard-break
suffix participates in the same indivisible line atom, so prefix + one token + suffix MAY overflow
when no valid narrower representation exists.

Only synthesized prefix whitespace is new. Word tokenization, inline-code protection,
spaceless-script joining, and long-token behavior remain outside this fix unless a fail-first list
fixture proves that the list integration itself regressed them.

### Decision 6 — Deterministic cross-platform output

The implementation remains pure Rust and filesystem-neutral. CRLF and lone-CR input continue to be
recognized as line endings and normalized consistently with the existing cleanup contract. Newly
synthesized line endings are `\n`, and hanging indentation uses ASCII spaces on macOS, Windows, and
Linux.

## Change Surface

The expected implementation surface is deliberately narrow:

- `darkmatter/lib/src/markdown/cleanup/reflow.rs`
  - semantic soft-break/list-prose classification;
  - source-edit application for standalone stripping;
  - logical-block fixed-width reflow; and
  - composite container-prefix construction.
- `darkmatter/lib/src/markdown/cleanup/mod.rs`
  - only the orchestration needed to reuse parsed events and preserve pass order.
- `darkmatter/lib/src/markdown/cleanup/tests/reflow.rs`
  - fail-first unit and semantic-regression fixtures.
- `darkmatter/lib/src/markdown/compose/tests/rendering.rs`
  - compose mode parity.
- `darkmatter/cli/tests/clean.rs`
  - CLI/default/fixed-width/opt-out coverage, including `--save` through an existing or focused
    fixture.
- `darkmatter/dmls/src/providers/formatting.rs`
  - tests only unless parity exposes a real routing defect.
- documentation and the Darkmatter skill surfaces listed below.

Changes to list-spacing, marker-restoration, or indentation-normalization code in `lists.rs` are
not expected. If implementation requires them, the plan MUST explain why reflow cannot consume the
already normalized output and add focused non-regression tests for those passes.

## Test Plan

### L1 library tests — stripping

Add fail-first tests proving:

1. unordered items with two-, four-, and eight-space authored continuation indentation collapse to
   one line without retained layout spaces;
2. ordered items with lazy three-space and explicit four-space continuations collapse identically;
3. `10.` and `10)` markers behave like `1.` without a fixed marker-width assumption;
4. checked and unchecked task-item continuations collapse;
5. unindented lazy continuation prose collapses when pulldown-cmark identifies it as the same item
   paragraph;
6. nested parent and child paragraphs collapse independently without joining across the child
   marker;
7. blockquoted and nested-blockquoted list prose removes repeated quote/list continuation prefixes;
8. a second paragraph preserves its blank boundary and required item indentation while collapsing
   its own soft breaks;
9. hard breaks inside list prose survive;
10. list-contained fenced code, indented code, tables, HTML, and directives remain protected;
11. CRLF list continuations produce the same canonical output as LF input; and
12. the Unicode separator policy remains identical inside and outside lists, including Han, Thai,
    Hangul, emoji, punctuation, and ZWSP boundaries.

At least one test MUST assert an exact string containing a single ordinary space at an ASCII join
boundary. A width-only assertion would not catch the current retained-indentation defect.

### L1 library tests — fixed width

Add exact-output tests for:

1. a pre-wrapped unordered item is fully unwrapped and rewrapped;
2. ordered markers with different digit widths receive different hanging prefixes;
3. checked and unchecked task markers align continuation text after the checkbox;
4. nested lists honor 2-, 4-, and 8-space configured nesting widths;
5. blockquoted ordered, unordered, and task lists retain composite prefixes;
6. first and subsequent paragraphs wrap independently;
7. hard breaks remain and both sides retain valid container prefixes, including a fixture where
   the indivisible hard-break suffix causes the documented overflow;
8. a long token is not split even when the prefix leaves less available width;
9. wide Unicode content is measured with `UnicodeWidthStr` including the prefix;
10. protected child blocks remain byte-equivalent; and
11. running fixed-width cleanup twice is idempotent.

Every non-overflow fixture MUST assert the display width of every emitted physical line, including
prefixes, rather than checking body text alone.

### Semantic structure regression helper

Add a test helper that parses source and output with the cleanup parser options and compares a
structural fingerprint after ignoring soft-break events and source offsets. It MUST at least pin:

- list and item count/order;
- ordered versus unordered list kind;
- ordered-list starting ordinal;
- nesting depth;
- paragraph boundaries;
- blockquote boundaries;
- checked/unchecked task state; and
- code/table/HTML child-block boundaries.

The structural fingerprint cannot recover every source spelling from pulldown-cmark. Exact-output
assertions MUST separately pin ordered delimiter (`.` versus `)`), unordered marker, task-box
spelling, and canonical indentation.

Use this helper on the nested-list, second-paragraph, blockquoted-list, and protected-code fixtures.
Exact output remains required; the fingerprint supplements rather than replaces string assertions.

### CLI tests

Add stdin-driven tests proving:

- default `md clean -` collapses a conventionally four-space-wrapped list item;
- `md clean --fixed-width N -` reflows the complete item and satisfies total-line display width;
- `md clean --ignore-incidental-newlines -` retains canonical source soft breaks in list prose; and
- `--save` writes corrected list wrapping and reports the delta.

Existing argument-conflict and width-range tests remain unchanged.

### Compose tests

For list prose, prove all three library modes through `ComposeOptions`:

- default `Strip` collapses;
- `Preserve` retains; and
- `with_fixed_width(N)` forces collapse before reflow even when `Preserve` was also selected.

The output MUST match the corresponding direct library cleanup sequence.

### DMLS tests

Extend formatting parity with a pre-wrapped list fixture. DMLS output for `fixed_width = N` MUST be
byte-identical to the `Markdown::cleanup` plus `reflow_to_width` sequence used by `md clean`.

### Performance regression

The semantic model MUST reuse the event stream already collected by full cleanup. Default cleanup
MUST NOT add an unconditional second Markdown parse. Fixed-width mode may perform the existing
post-cleanup parse needed to derive logical reflow blocks, but this fix MUST NOT add another parse
on top of that sequence.

Extend the focused cleanup benchmark when necessary and compare representative top-level prose,
flat lists, deeply nested lists, and blockquoted task lists against the pre-fix baseline. The
review must report parse count and timing evidence. On the same host and Criterion profile, default
cleanup MUST remain within 10% of its pre-fix median, fixed-width cleanup MUST remain within 15% on
the list-heavy fixtures, and the original fixed-width budget of less than 2x full cleanup on the
same input remains in force. A larger regression requires profiling and explicit approval rather
than being accepted solely because correctness tests pass.

## Documentation and Drift Maintenance

Implementation is incomplete until the following are updated:

- `darkmatter/docs/cli/clean.md` — state explicitly that prose includes list-item paragraphs and
  show default-strip and fixed-width hanging-indent examples;
- `darkmatter/cli/README.md` — make the same behavior discoverable from the CLI overview;
- `darkmatter/docs/darkmatter-compose-pipeline.md` — describe list-aware soft-break collapse and
  reflow in the Cleanup stage;
- `.claude/skills/darkmatter/SKILL.md` and its `compose.md` topic — record that cleanup applies to
  list prose and that fixed-width continuation prefixes include list/blockquote containers; and
- the completed fixed-line-length spec or its status note — link to this fix so the historical claim
  that list bodies are wrapped is no longer presented as fully shipped before this work lands.

Relevant `///`, `//!`, and inline comments in `reflow.rs`, `cleanup/mod.rs`, compose cleanup, and
DMLS formatting MUST be checked against the new behavior. Delete or correct any comment that says
reflow is physical-line based or that all four-space lines are indented code. Code behavior remains
the authority when unrelated stale comments are found.

## Validation

From the `darkmatter/` package area:

```bash
just build
just test
just test-l2
just lint
```

Do not substitute `cargo test` for nextest-backed recipes. `cargo fmt` is not part of this fix.

Because the affected free functions have HIGH/CRITICAL blast radius, implementation review MUST
also run GitNexus `detect_changes` and confirm that affected flows are limited to the expected
cleanup, compose, CLI, and DMLS formatting surfaces.

## Acceptance Criteria

The fix is complete when all of the following are true:

1. Default cleanup collapses every eligible soft break in ordered, unordered, and task-list prose,
   including nested and blockquoted lists.
2. Collapsing a list soft break removes its continuation-layout prefix and inserts only the
   existing zero-or-one-character Unicode join separator.
3. Default strip mode does not synthesize hanging indentation or other layout whitespace.
4. `--ignore-incidental-newlines` performs no list soft-break collapse or fixed-width synthesis.
5. Fixed-width cleanup unwraps each complete logical list prose block before wrapping it.
6. Every created continuation line begins with the complete hanging container prefix and its body
   aligns with the first line's body in Unicode display columns.
7. Ordered-marker digit growth, task boxes, configured nesting widths, and composed blockquote/list
   prefixes are handled per item without hard-coded prefix widths.
8. Except for the established indivisible atomic-token, prefix, and hard-break-suffix overflow
   cases, every reflowed physical line's total display width is at most the requested width.
9. Blank-line paragraph boundaries, sibling-item boundaries, nested-list boundaries, hard breaks,
   and protected child blocks remain structurally intact.
10. Structural fingerprint tests show no change to list/item nesting or protected block ownership.
11. Default, compact, and loose list-spacing modes retain their existing behavior.
12. Equivalent full-cleanup sequences agree across direct library use, compose, CLI stdout, CLI
    `--save`, and DMLS formatting. The transform-only fixed-width helpers retain the narrower
    contract defined by Decision 4.
13. Cleanup is idempotent in default, preserve, and fixed-width modes.
14. No public API, CLI schema, dependency, or platform-specific behavior changes.
15. The default cleanup path reuses its existing parse, and fixed-width mode adds no parse beyond
    the established cleanup-plus-reflow sequence; focused benchmark evidence shows no unexplained
    regression.
16. Area build, L1, L2, and lint recipes pass, and GitNexus change detection reports only the
    expected cleanup-related surfaces.

## Out of Scope Follow-ups

The following may deserve separate work but MUST NOT expand this fix:

- general inline-aware wrapping improvements unrelated to list containers;
- kinsoku-aware or dictionary-based breaking for spaceless scripts;
- configurable hanging-indent style independent of fixed-width source formatting;
- preserving authored tabs in synthesized continuation prefixes;
- changing how pulldown-cmark increments ordered markers; and
- rendering-layer visual list indentation, which belongs to terminal/browser components rather
  than source cleanup.
