---
status: ready for planning and implementation
created: 2026-06-20
area: darkmatter
packages:
    - darkmatter
reviewed: true
review_iterations: 1
---

# Unordered List Markers Inside Blockquotes

## Problem

Running:

```text
claudine compose prompts/review-feature.md -y spec="fixes/2026-06-19-invalid-parsing-state/spec.md" --codex --dry-run
```

produces composed Markdown where unordered list items **inside a blockquote**
are emitted with a `*` bullet, while unordered list items **outside** blockquotes
keep their original `-` bullet:

```text
# Review of Invalid Parsing State
> * Fix: `2026-06-19-invalid-parsing-state`
> * Review File (_output_): `@claudine/fixes/2026-06-19-invalid-parsing-state/review-2.md`
> * Review Iteration: #2

...

- look for gaps in functionality that were designed but not implemented
- features who's implementation is broken or incomplete
```

The source authored these blockquote items with `-`:

```md
> - {{capitalize(feature_or_fix)}}: `{{parent_dir(spec)}}`
> - Review File (_output_): `@{{review_file}}`
> - Review Iteration: #{{iteration}}
```

The blockquote bullets must round-trip as `-` (the authored marker), matching the
behavior of top-level lists.

## Root Cause

The bug is in the compose **cleanup / normalization** step, in
`darkmatter/lib/src/markdown/cleanup.rs`.

The cleanup phase round-trips Markdown through `pulldown-cmark-to-cmark`
(`cleanup.rs:387`), which **normalizes every unordered list bullet to `*`**. To
undo this, darkmatter performs a two-step extract/restore:

1. `extract_list_markers()` (`cleanup.rs:449`) walks the source before
   serialization and records each unordered item's original bullet, in document
   order.
2. `restore_list_markers()` (`cleanup.rs:502`) walks the serialized output
   line-by-line and rewrites each normalized `* ` back to the recorded marker.

**Step 1 works correctly**, including for blockquote items. A diagnostic run on
the failing pattern confirmed every marker — blockquote and top-level — is
recorded as `-`:

```text
EXTRACTED_MARKERS = ['-', '-', '-', '-', '-']
```

**Step 2 is the defect.** The restore matcher at `cleanup.rs:533` is:

```rust
let trimmed = line.trim_start();
...
if trimmed.starts_with("* ") {   // only matches bare list lines
```

`str::trim_start()` strips leading **whitespace only** — it does not strip the
blockquote `>` prefix. So a line like `> * Fix: one` stays `> * Fix: one` after
trimming and fails the `starts_with("* ")` test. The branch is skipped, and the
blockquote bullet keeps cmark's normalized `*`. Top-level lines (`* top one`)
match and are correctly restored to `-`.

### Latent alignment defect

`restore_list_markers`'s doc comment claims a "simple sequential 1:1 replacement"
between extracted markers and `* ` output lines. That invariant is **already
broken**: `extract_list_markers` counts blockquote items into the markers vector,
but restore skips those lines and never advances `marker_idx` for them. The
top-level items therefore consume the blockquote's marker slots, not their own.

This is invisible today only because every marker in the failing document is `-`.
With mixed markers (e.g. a blockquote using `-` followed by a top-level list using
`+`), restoration would assign the wrong characters to the wrong items.

## Requirements

1. Unordered list items inside blockquotes MUST survive cleanup with their
   authored bullet marker. `> - item` cleans to `> - item`; `> * item` cleans to
   `> * item`; `> + item` cleans to `> + item`.
2. The fix MUST also handle CommonMark blockquote prefixes with up to three
   leading spaces, nested blockquotes (`> > - item`), compact blockquote marker
   input (`>> - item`, which cleanup may normalize to `> > - item`), and
   indented content after the final `>` marker.
3. The 1:1 correspondence between `extract_list_markers` and the lines visited by
   `restore_list_markers` MUST be restored, so mixed-marker documents
   (blockquote and top-level lists using different bullets) each retain their own
   authored markers.
4. `restore_list_markers` MUST NOT rewrite apparent bullets inside protected
   code content. This includes ordinary fenced code blocks and fenced code blocks
   nested inside blockquotes, where cmark output can look like:

   ```md
   > ```
   > * this is code
   > ```
   ```

   The fence-state check therefore must operate on the line body after any
   blockquote prefix has been stripped, not only on `line.trim_start()`.
5. No regression to existing behavior: top-level lists, ordered lists, fenced
   code blocks (which must not have bullets rewritten), and the blockquote
   prefix-normalization performed by `fix_blockquote_formatting` /
   `fix_blockquote_line` must all continue to behave as before.

## Proposed Approach

Make the line-matcher in `restore_list_markers()` (`cleanup.rs:502`)
blockquote-aware so it both **restores** the marker inside the `>` prefix and
**advances `marker_idx`** for those lines:

1. Add a small cleanup-local helper that splits a rendered line into
   `(prefix, body)` where `prefix` includes leading indentation plus every
   blockquote marker segment and its following spacing, and `body` starts at the
   first content byte after that prefix. If no blockquote marker is present, the
   helper should return the existing leading whitespace prefix plus the
   whitespace-trimmed body so top-level behavior remains unchanged.
2. Test the post-prefix content with `starts_with("* ")` instead of testing the
   whitespace-only-trimmed line.
3. On a match, rebuild the line as
   `<original-prefix-including-blockquote><marker><rest>` and advance
   `marker_idx`, exactly as the bare-list branch does today but preserving the
   `> ` prefix.
4. Use the same `body` from the split helper when toggling fenced-code state.
   Recognize both backtick and tilde fences with at least three repeated marker
   characters, because cleanup output is Markdown and CommonMark accepts both.

`extract_list_markers` needs no change — it already records the correct markers.

### Design note: shared prefix parsing

`restore_list_markers`, `fix_blockquote_formatting`, and `fix_blockquote_line`
each independently re-parse the blockquote prefix line-by-line. The new
prefix splitting in `restore_list_markers` MUST match the normalized prefix
shape produced by `fix_blockquote_formatting`, because restoration runs after
that normalization step.

Do not import `markdown::compose::parse_utils::strip_blockquote_prefix` directly
from cleanup. It strips the prefix but discards the prefix bytes, while
`restore_list_markers` must rebuild the line with the exact rendered prefix. A
small cleanup-local helper is the least invasive design for this fix. If later
work needs the same prefix/body split in compose and cleanup, lift the helper to
a shared `markdown` parsing utility in a separate refactor.

Reader note: the intended behavior is marker preservation, not blockquote
renormalization. `fix_blockquote_formatting` remains the owner of blockquote
spacing cleanup; `restore_list_markers` should consume that output and only
replace the normalized unordered-list marker.

## Open Questions

None. The implementation decision is to keep the fix local to
`cleanup.rs` and avoid a broader Markdown-prefix utility refactor until another
caller needs the same prefix/body split.

## Test Plan

- **Regression (primary):** `> - item` round-trips through cleanup unchanged as
  `> - item`; likewise `> * item` and `> + item` keep their authored markers.
  These should be Level 1 library tests around `cleanup_content`.
- **Mixed markers:** a document with a `-` blockquote list followed by a
  top-level list using a different bullet — assert each list keeps its own
  authored marker (locks down the latent alignment defect).
- **Nested and indented blockquotes:** `> > - item`, `>> - item`, and
  `   > - item` keep the authored marker after cleanup's existing blockquote
  normalization.
- **No code-block leakage:** a `* ` line inside a fenced code block within a
  blockquote is not rewritten. Include both backtick and tilde fences if cmark
  emits both forms for the relevant inputs; otherwise assert the emitted fence
  form and document why only that form is covered.
- **Existing coverage stays green:** run the current cleanup/list marker tests
  so top-level marker restoration, ordered lists, emphasis preservation, and
  non-blockquoted fenced code behavior remain locked down.
- **End-to-end:** the original `claudine compose ... --codex --dry-run` command
  emits `> - Fix:` etc. (not `> *`) if `claudine` is available in the local
  environment. If it is not available, the Level 1 cleanup fixture using the same
  blockquote list shape is sufficient for implementation acceptance.
