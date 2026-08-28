---
status: implemented — awaiting review
created: 2026-08-27
area: darkmatter
packages:
  - darkmatter
---

# Compose must not consume backslash escapes in body text

## Summary

`md compose` (and every consumer of `markdown::cleanup`) turns

```markdown
PLAIN=C:\Temp\.tmp\repo and a\-b and 2 \* 3
```

into

```markdown
PLAIN=C:\Temp.tmp\repo and a-b and 2 * 3
```

The document composes successfully and exits zero, but the composed Markdown
no longer says what the source said. A `\*` that was deliberately literal is
now an emphasis marker for the next renderer; a Windows path with a
`.`/`-`/`_`-initial segment is silently corrupted. The same happens to
interpolated values: `{{ p }}` with `p: "C:\\x\\.y"` renders as `C:\x.y`.

This was first observed on the Windows CI runners for the Claudine
launch-anchor tests, where `{{ ctx.repo_root }}` pointed at
`C:\Users\runneradmin\AppData\Local\Temp\.tmpXXXX\repo`.

## Background

CommonMark treats `\` followed by any ASCII punctuation character as an
escape: `\.` means a literal `.`, `\*` a literal `*`. A Markdown → HTML
renderer resolves those escapes, and that is correct. Compose is a
Markdown → **Markdown** transform, though: its output is read by another
Markdown consumer, so the escapes must survive.

Compose's always-on `Cleanup` phase (`compose/pipeline/phases.rs`) delegates
to `markdown::cleanup`, which round-trips the document through
`pulldown-cmark` → `pulldown-cmark-to-cmark`. The parser hands back
`Event::Text` with escapes already resolved, and cleanup re-emits that text
verbatim (`cleanup/emphasis.rs`, the pass-through arm of
`preserve_original_emphasis`). `cmark` re-escapes only the characters it
considers dangerous (`*`, `_`, `[`, …), so `\.` and `\-` vanish outright; the
`\*`/`\_`/`\[` that `cmark` *does* re-emit are then deliberately stripped by
`unescape_emphasis_chars` and `unescape_brackets`, because those passes cannot
tell "escape `cmark` added" from "escape the author wrote".

Nothing in darkmatter's docs or tests describes escape resolution as intended
compose behaviour. Existing tests pin the opposite direction only: cleanup
must not *add* escapes (`cleanup/tests/brackets.rs`, `cleanup/tests/tables.rs`).

## The fix

Cleanup already parses with `into_offset_iter`, so the source slice for every
`Text` event is in hand, and it already uses private-use placeholder
characters to carry emphasis markers through `cmark` untouched. The fix reuses
that mechanism:

1. In `preserve_original_emphasis`, detect an escaped `Text` event. The parser's
   source range for an escaped character starts *after* the backslash (`a\-b`
   yields `Text("-b")` at `2..4`), so the backslash lives in the gap before the
   event. An event is an escape when its text starts with ASCII punctuation,
   its range really starts with that character, and the source immediately
   before the range ends in an odd run of backslashes (an even run is escaped
   backslashes, `a\\-b`). Emit `BACKSLASH_PLACEHOLDER` (`U+E002`) followed by
   the event text. Code blocks and code spans never match: their text keeps its
   backslashes, so the character before a range is never a stray `\`.
2. After `cmark` rendering and after the `unescape_emphasis_chars` and
   `unescape_brackets` passes, restore the placeholder to `\`. Ordering
   matters: an author's `\*` becomes `␣*`, `cmark` may re-escape the `*` to
   `\*`, the unescape pass reduces that to `*`, and the restore then yields
   the original `\*`.

Backslashes that are not escapes (`C:\Users`, where `U` is not punctuation)
were never at risk: the parser leaves them in the text and `cmark` does not
escape a lone backslash.

## Acceptance criteria

- AC1: `a\-b`, `a\.b`, `a\_b`, `a\*b`, `a\[b\]`, `a\\b` in paragraph text
  survive `cleanup_content` byte-for-byte.
- AC2: A Windows path with a punctuation-initial segment
  (`C:\Users\x\Temp\.tmpAbc\repo`) survives compose, both as literal body text
  and as an interpolated frontmatter value.
- AC3: Escapes inside fenced code blocks and code spans are untouched (they
  were already literal; the fix must not double them).
- AC4: The existing "cleanup must not add escapes" tests still pass.
- AC5: A hard line break (`\` + newline) is unaffected.

## Out of scope

`ctx.*` directory values are now rendered as portable `/`-separated paths
(`compose/context/capture/repo.rs`, landed separately), so they no longer
depend on this fix; this fix covers author-written and interpolated text.
