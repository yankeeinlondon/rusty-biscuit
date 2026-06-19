---
status: ready for planning and implementation
created: 2026-06-18
severity: bug
provider: all
reviewed: true
related_features:
  - darkmatter/features/_completed
related_docs:
  - claudine/docs/topics/composition.md
---

# Cleanup must keep tight nested lists tight (no blank line before a sub-list)

## The Problem We Are Solving

When any composition path runs the Darkmatter **cleanup** stage — `claudine
compose` / `claudine inline-compose` / `claudine sequence` (all of which pipe the
composed body through `Markdown::compose*`), the `md clean` CLI, or any caller of
`cleanup_content` / `cleanup_content_with_indent` — a **tight nested list** is
corrupted: a spurious blank line is inserted between a parent list item and its
own indented children.

### Observed Incident

`prompts/review-feature.md`'s `## Closure` section authors a tight
nested unordered list:

```markdown
## Closure

- Save your review suggestions to "@{{ctx.area}}/{{dir}}/review-{{iteration}}.md"
- Save the following frontmatter properties on "@{{ctx.area}}/{{dir}}/review-{{iteration}}.md":
    - based on your review suggestions indicate whether you think this feature is **ready for production** …
    - set the `agent` frontmatter property to "{{env.AGENT}}"
    - set the `model` frontmatter property as "{{env.MODEL}}"
    - set the `created` frontmatter property to "{{ctx.now}}"
```

Running `claudine compose prompts/review-feature.md … --dry-run` produces a
composed body whose `## Closure` section now reads:

```markdown
- Save your review suggestions to "…/review-1.md"
- Save the following frontmatter properties on "…/review-1.md":
                                                                              ← spurious blank line
    - based on your review suggestions indicate whether you think this feature is **ready for production** …
    - set the `agent` frontmatter property to "codex"
    …
```

The nested items then render as a **code block** in the downstream consumer that
displays the composed prompt, not as a nested list.

### Reproduction

The bug is independent of composition; it lives in the cleanup stage. Reproduce
directly with the `md` CLI:

```sh
# Tight nested list in → blank-line-corrupted list out.
printf -- '- Level 1\n    - Level 2\n        - Level 3\n' > /tmp/l.md
md clean /tmp/l.md
# emits:
# - Level 1
#
#     - Level 2
#
#         - Level 3
```

Reproduced against the exact `## Closure` payload via `md clean` and confirmed
identical in the live `claudine compose … --dry-run` stdout body.

## Root Cause

`darkmatter/lib/src/markdown/cleanup.rs::normalize_list_spacing`, **Phase 2**
(the blank-line-insertion pass), uses the wrong predicate for `Normal` mode:

```rust
ListSpacingMode::Normal => {
    if let Some(prev) = prev_item_indent {
        // Blank line on indent change OR when the previous item
        // had continuation content (loose list items).
        indent != prev || had_continuation
    } else {
        false
    }
}
```

`indent != prev` is `true` for an indent change in **either** direction —
including the transition from a parent item (indent 0) into its own sub-list
(indent 2 after `pulldown-cmark-to-cmark` serialization, later scaled to 4 by
`fix_list_indentation`). So a blank line is inserted directly between a parent
item and the first child of a **tight** sub-list.

The correct behavior: **entering a deeper sub-list is not a place where a blank
line belongs.** A blank line should only be inserted when:

- the previous item had wrapped continuation content (a genuinely **loose** list),
  tracked by `had_continuation`; or
- the new item is at a **strictly shallower** indent than the previous item
  (i.e. one or more sub-lists are being closed on the way back up).

Same-level siblings and parent→child descents must both stay tight.

### Why it surfaces as a code block

The cleanup pipeline runs `normalize_list_spacing` _before_
`fix_list_indentation`. After the spurious blank line is inserted, the children
are rescaled to a 4-space indent, yielding:

```markdown
- parent:

    - child
```

`pulldown-cmark` (the parser Darkmatter itself uses) parses this as a **loose
sub-list**, _not_ a code block, because the child marker sits at `content_offset
+ 2` (column 4 for a `- ` parent whose content begins at column 2), short of the
`content_offset + 4` an indented code block would require. **That is exactly why
this regression was invisible to the existing suite** — Darkmatter's own parser
sees a (loose) list, so every parse-based assertion passed.

Renderers that key the indented-code-block threshold off the list **marker
column** rather than the item **content offset** (parent marker at column 0 ⇒
child at column 0 + 4 = code block) treat the post-cleanup structure as a code
block. The user's downstream viewer is one such renderer. Removing the spurious
blank line restores the universally-recognized tight sub-list form

```markdown
- parent:
    - child
```

so the structure renders as a nested list everywhere, regardless of which
threshold convention a renderer uses.

## Why The Existing Tests Missed It

`test_nested_list_preserves_4_space_indentation` (and siblings) assert with
loose substring matches:

```rust
assert!(cleaned.contains("\n    - Level 2"), …);
```

`\n\n    - Level 2` _contains_ the substring `\n    - Level 2` (matched against
the second newline), so the assertion passes even when a blank line has been
inserted immediately before the child. The suite was asserting that the _indent
width_ was preserved; it never asserted the _absence_ of a blank line between a
parent and its tight children.

## Scope

### Reader note — focused compatibility fix

This spec does **not** redefine `ListSpacingMode::Normal` as a complete
CommonMark tight-list serializer. It makes the smallest behavior change needed
to stop cleanup from corrupting a parent item followed immediately by its own
child list. In Normal mode, descents into a sub-list become tight; the existing
shallower-return behavior is preserved for this fix so callers that rely on
visual separation after nested blocks do not get an unrelated output change.

### In scope

Darkmatter library only — `darkmatter/lib/src/markdown/cleanup.rs`:

- `normalize_list_spacing` Phase 2: correct the `Normal`-mode blank-line
  predicate so descending into a sub-list does not insert a blank line.
- `#[cfg(test)] mod tests` in the same file: strengthen the nested-list
  regression tests to assert the _absence_ of a blank line between a parent and
  its tight children (negative substring / structural assertion), and add a
  dedicated regression test modeled on the `## Closure` payload (parent item
  ending with `:`, two top-level siblings where only the second has children,
  followed by prose).
- Existing Normal-mode spacing tests that currently encode the bug must be
  updated. In particular,
  `normal_blank_lines_around_level_transition` should be renamed or rewritten so
  it no longer expects `lessons:\n\n    - @docs` when entering a sub-list. It may
  continue to assert the shallower-return blank (`commits.md\n\n2.`), matching
  this spec's compatibility decision.

### Out of scope

- Claudine source changes. `prepare_direct` / `prepare_inline` already call
  `Markdown::compose_with(…)`, so fixing Darkmatter cleanup fixes every
  composition surface (`compose`, `inline-compose`, `sequence`, `--dry-run`) with
  no claudine edit. Claudine gets only a new end-to-end regression test (see
  Testing Requirements).
- Changes to `pulldown-cmark-to-cmark`'s 2-space default indentation or to
  `fix_list_indentation`'s 2→N rescaling. Both are correct; the defect is purely
  the spurious blank line.
- The `Compact` and `Loose` modes. `Compact` already strips all inter-item
  blanks; `Loose` intentionally inserts blanks between every item. Only `Normal`
  (the default for `cleanup_content` and therefore for the compose pipeline) is
  affected.
- The unrelated false-positive `ctx.*` interpolation warnings observed in the
  same prompt — see "Related Finding" below; that is a separate bug warranting
  its own spec.

## Normative Behavior

After the fix, for `ListSpacingMode::Normal`:

- **Tight sub-list (parent → own child, deeper indent):** no blank line.
  ```markdown
  - parent:
      - child
  ```
- **Tight siblings (same indent, no continuation):** no blank line (unchanged).
  ```markdown
  - a
  - b
  ```
- **Loose list (item with continuation prose):** blank line before the next item
  (unchanged — driven by `had_continuation`).
  ```markdown
  - **First**

      Paragraph under first.

  - **Second**
  ```
- **Closing a sub-list (strictly shallower indent than the previous item):**
  blank line (preserves current behavior for this focused fix; not regressed).
  ```markdown
  - parent:
      - child

  - sibling
  ```
- **List followed by prose:** blank line (unchanged — driven by the existing
  `is_cont && in_list_run` branch).

The single-source-of-truth rule for the `Normal`-mode item predicate becomes:

> Insert a blank line before a list item iff the previous item had continuation
> content (`had_continuation`) _or_ the new item is at a strictly shallower
> indent than the previous item (`indent < prev`). Descents into a sub-list
> (`indent > prev`) and same-level siblings (`indent == prev`) stay tight.

## Proposed Implementation

`darkmatter/lib/src/markdown/cleanup.rs`, inside `normalize_list_spacing`'s
Phase 2 `match mode` arm, change the `Normal` branch from

```rust
indent != prev || had_continuation
```

to

```rust
// Descents and same-level siblings stay tight; loose items and shallower
// returns keep their separating blank.
indent < prev || had_continuation
```

No other state machine field (`prev_item_indent`, `in_list_run`,
`had_continuation`, `prev_was_blank`) changes. The `Compact` and `Loose` arms
are untouched.

The adjacent doc comment on `normalize_list_spacing` ("blank lines at
indentation level transitions") must be corrected to read "blank lines when
_returning_ from a sub-list (shallower transition) or for loose lists" so it
describes the fixed behavior rather than the bug.

### Order-of-operations note (do not change)

`normalize_list_spacing` runs against the 2-space-indented output of
`pulldown-cmark-to-cmark`, _before_ `fix_list_indentation` rescales to the
target width. The `indent` values the predicate sees are therefore 0, 2, 4, …
The `indent < prev` comparison is unitless (it compares level offsets), so it is
correct regardless of whether the pipeline later rescales to 2, 4, or any other
width.

## Files Most Likely to Change

- `darkmatter/lib/src/markdown/cleanup.rs`
  - `normalize_list_spacing` — the one-line predicate fix plus comment.
  - Doc comment on `normalize_list_spacing` — correct the "level transitions"
    wording.
  - `#[cfg(test)] mod tests` — strengthen existing nested-list assertions and
    add the `## Closure`-shaped regression.
  - `normal_blank_lines_around_level_transition` — update the existing test that
    currently expects the buggy parent→child blank.
- `claudine/lib/src/composition/prepare.rs` `#[cfg(test)] mod tests` — add an
  end-to-end test that composes a minimal document containing a tight nested
  list and asserts `prepared.prompt` does _not_ contain a blank line between the
  parent item and its first child.

## Testing Requirements

### Darkmatter unit tests (`darkmatter/lib/src/markdown/cleanup.rs`)

1. **Strengthen `test_nested_list_preserves_4_space_indentation`** (and the
   `cleanup_with_indent` variants) so each asserts the _absence_ of a blank line
   before every nested child, e.g.
   `assert!(!cleaned.contains("\n\n    - Level 2"))` in addition to the existing
   positive indent assertion. The negative assertion is what would have caught
   this regression.
2. **New regression `tight_nested_list_stays_tight_after_cleanup`** — modeled on
   the `## Closure` payload: a heading, two top-level `- ` siblings where only the
   second has a tight 4-space-indented sub-list of four children (one child
   containing inline code + bold), followed by a blank line and a `**bold:**`
   paragraph, then another top-level list. Assert the cleaned output contains
   `- Save the following…:\n    - based on` (parent directly followed by child,
   no blank line) and that the children remain at the configured indent width.
3. **`loose_list_keeps_blank_lines_in_normal_mode`** — regression guard that the
   `had_continuation` path still inserts blanks for genuinely loose lists, so the
   fix cannot be over-applied. (The existing
   `normal_loose_list_preserves_blank_lines_between_items` already covers this;
   confirm it still passes unchanged.)
4. **`tight_siblings_stay_tight`** — guard that same-indent siblings without
   continuation remain blank-free (the `indent == prev` case), so the `!=` → `<`
   change is provably not a behavior change for siblings.
5. **`closing_a_sublist_inserts_blank`** — guard that returning to a strictly
   shallower indent still inserts a blank, so the fix only affects the descent
   direction.
6. **Rewrite the existing `normal_blank_lines_around_level_transition` test** —
   it currently asserts `lessons:\n\n    - @docs`, which is the bug. Keep a
   shallower-return assertion if desired, but add/replace the descent assertion
   with `lessons:\n    - @docs`.

All cleanup tests pass under `cargo test -p darkmatter --lib markdown::cleanup`.

### Claudine end-to-end test (`claudine/lib/src/composition/prepare.rs`)

Add `direct_composition_preserves_tight_nested_list` building a source document
with frontmatter and a body containing a tight nested unordered list shaped like
the `## Closure` section; assert `prepared.prompt` contains the parent item
immediately followed (single newline, no blank line) by its first indented
child. This is the smoke test that ties the darkmatter fix to the user-facing
incident so the specific failure mode never silently regresses.

## Verification

```sh
# 1. Darkmatter cleanup tests (includes the new + strengthened assertions).
cargo test -p darkmatter --lib markdown::cleanup

# 2. Claudine composition regression.
cargo test -p claudine --lib composition::prepare

# 3. Behavioural check: tight nested list stays tight.
printf -- '- parent:\n    - child one\n    - child two\n' > /tmp/tight.md
md clean /tmp/tight.md   # must emit NO blank line between `parent:` and `- child one`

# 4. Behavioural check: the original incident prompt composes cleanly.
claudine compose prompts/review-feature.md -y \
  spec="features/<some-feature>/spec.md" --codex design="" iteration=1 --dry-run \
  | grep -A2 'Save the following'   # the two lines following must be the children, not a blank line
```

## Acceptance Criteria

- [x] `normalize_list_spacing`'s `Normal`-mode predicate inserts a blank line
      before a list item iff `had_continuation || indent < prev` (equivalently,
      a parent→child descent never inserts a blank).
- [x] `md clean` on `- Level 1\n    - Level 2\n        - Level 3` produces
      `- Level 1\n    - Level 2\n        - Level 3` with **no** blank lines
      between levels.
- [x] `md clean` on the `## Closure`-shaped payload produces
      `…properties on "…":\n    - based on…` (single newline between parent and
      first child).
- [x] The `## Closure` section of `claudine compose prompts/review-feature.md …
      --dry-run` renders as a nested list (no code block) in a renderer that
      previously showed a code block.
- [x] Existing loose-list tests
      (`normal_loose_list_preserves_blank_lines_between_items`,
      `loose_with_nested_list`, `compact_with_nested_list`) pass unchanged.
- [x] Existing tight-sibling tests (`tight_list_stays_tight_in_normal_mode`)
      pass unchanged.
- [x] `normal_blank_lines_around_level_transition` no longer expects a blank
      line before an indented child list; it either gets renamed to describe only
      the shallower-return case or is split into descent and ascent tests.
- [x] Strengthened nested-list tests assert the _absence_ of a blank line before
      children (negative assertions), not merely the indent width.
- [x] New claudine end-to-end test
      `direct_composition_preserves_tight_nested_list` passes.
- [x] The `normalize_list_spacing` doc comment describes the fixed behavior
      (blanks on shallower returns / loose lists), not "level transitions".

## Related Finding (Out of Scope — Separate Bug)

While reproducing the list bug, the same `review-feature.md` invocation emits a
cluster of **false-positive** `[frontmatter-interpolation]` warnings to stderr
(visible in both `--dry-run` and live; the `--dry-run` body on stdout is
unaffected, which is why the two were easy to conflate):

```
warning: [frontmatter-interpolation] key 'start': unknown context variable 'ctx.area'
  did you mean: area — Scoped area name: …
  example: ctx.area → claudine
warning: [frontmatter-interpolation] key 'success': failed to evaluate 'review - file': Subtraction requires numeric operands
```

Two distinct sub-bugs, both unrelated to the nested-list fix:

1. **`ctx.area` / `ctx.current_package_area` flagged unknown despite resolving.**
   `darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs::collect_context_warnings`
   calls `state.is_valid_context_variable(name)`; for runtime-detected context
   variables such as `ctx.area` this returns `false` at the warning-check pass
   even though the variable resolves (the did-you-mean line — sourced from the
   static `CONTEXT_VARIABLE_DESCRIPTORS` catalog — simultaneously confirms it is
   a known variable and even prints a live example). The runtime-validity check
   and the descriptor catalog disagree.
2. **`{{review-file}}` parsed as subtraction.** `review-feature.md` writes
   `{{review-file}}` (hyphen) in the `success.message` frontmatter but the
   declared key is `review_file` (underscore). The expression lexer parses
   `review-file` as `review - file`, hence "Subtraction requires numeric
   operands". The prompt author's typo should ideally surface as an
   unknown-variable did-you-mean for `review_file`, not as a subtraction error.

These belong in a separate fix spec and are called out here only so the
diagnosis is not lost. Neither is caused by — or affected by — the
`normalize_list_spacing` fix this spec describes.

## References

- Source: `darkmatter/lib/src/markdown/cleanup.rs:1222` — `normalize_list_spacing`
  (Phase 2, the `Normal`-mode `indent != prev || had_continuation` predicate).
- Source: `darkmatter/lib/src/markdown/cleanup.rs:748` — `fix_list_indentation`
  (runs _after_ `normalize_list_spacing`; rescales 2-space cmark output to the
  target width; correct as-is).
- Source: `darkmatter/lib/src/markdown/cleanup.rs:341` — `cleanup_content_internal`
  (ordering: `pulldown-cmark-to-cmark` serialize → `normalize_list_spacing` →
  `fix_list_indentation`).
- Source: `claudine/lib/src/composition/prepare.rs:106` — `prepare_direct`,
  where composition runs the full Darkmatter pipeline (including Cleanup) and
  takes `composed.content()` as the prompt body.
- Source: `darkmatter/lib/src/markdown/cleanup.rs:2246` —
  `test_nested_list_preserves_4_space_indentation`, whose permissive substring
  assertion let the regression through; to be strengthened.
- Incident prompt: `prompts/review-feature.md:100-114` — the `## Closure`
  section with the tight nested list that surfaced the bug.
