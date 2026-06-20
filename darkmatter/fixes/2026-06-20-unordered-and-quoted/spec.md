---
status: draft
created: 2026-06-20
area: darkmatter
packages:
    - darkmatter
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
2. The fix MUST also handle nested blockquotes (`> > - item`) and indented
   blockquote content.
3. The 1:1 correspondence between `extract_list_markers` and the lines visited by
   `restore_list_markers` MUST be restored, so mixed-marker documents
   (blockquote and top-level lists using different bullets) each retain their own
   authored markers.
4. No regression to existing behavior: top-level lists, ordered lists, fenced
   code blocks (which must not have bullets rewritten), and the blockquote
   prefix-normalization performed by `fix_blockquote_formatting` /
   `fix_blockquote_line` must all continue to behave as before.

## Proposed Approach

Make the line-matcher in `restore_list_markers()` (`cleanup.rs:502`)
blockquote-aware so it both **restores** the marker inside the `>` prefix and
**advances `marker_idx`** for those lines:

1. After `trim_start()`, also strip a leading blockquote prefix (one or more
   `>` / `> ` segments) and record the byte offset where post-prefix content
   begins.
2. Test the post-prefix content with `starts_with("* ")` instead of testing the
   whitespace-only-trimmed line.
3. On a match, rebuild the line as
   `<original-prefix-including-blockquote><marker><rest>` and advance
   `marker_idx`, exactly as the bare-list branch does today but preserving the
   `> ` prefix.

`extract_list_markers` needs no change — it already records the correct markers.

### Design note: shared prefix parsing

`restore_list_markers`, `fix_blockquote_formatting`, and `fix_blockquote_line`
each independently re-parse the blockquote prefix line-by-line. The new
prefix-stripping in `restore_list_markers` SHOULD reuse the same notion of
"blockquote prefix" those functions use (factor a small shared helper if
needed) to avoid introducing a third subtly-different prefix parser.

## Test Plan

- **Regression (primary):** `> - item` round-trips through cleanup unchanged as
  `> - item`; likewise `> * item` and `> + item` keep their authored markers.
- **Mixed markers:** a document with a `-` blockquote list followed by a
  top-level list using a different bullet — assert each list keeps its own
  authored marker (locks down the latent alignment defect).
- **Nested blockquotes:** `> > - item` keeps `-`.
- **No code-block leakage:** a `* ` line inside a fenced code block within a
  blockquote is not rewritten.
- **End-to-end:** the original `claudine compose ... --codex --dry-run` command
  emits `> - Fix:` etc. (not `> *`).
