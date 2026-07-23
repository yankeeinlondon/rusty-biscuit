# Error text is silently mangled by Prose Markdown emphasis

## Problem

Dynamic error text rendered through `Prose` is subject to Markdown emphasis
pre-processing. When an error message contains **two** `_` characters — which
happens routinely when a message quotes two filesystem paths that each contain
an underscore-prefixed directory — the pair is consumed as an `_italic_` span:
both underscores vanish from the displayed text and everything between them is
italicized.

The result is an error message that misreports the very paths it is
complaining about. Observed in the wild:

```sh
💻❯ compose prompts/review.md spec='features/_completed/2026-07-11-provider-errors-as-data/spec.md' -y --codex

Error: lifecycle initialize proxy: path resolution failed for "prompts/reviews/review-spec-inline.md":
proxy target does not exist: /Users/ken/.claudine/worktrees/rusty-biscuit/claudine/prompts/prompts/reviews/review-spec-inline.md
```

The actual `HarnessError` text contained `prompts/_reviews/…` in both
positions. The first path's `_` opened emphasis, the second path's `_` closed
it, and the copy-pasted output shows `prompts/reviews/…` — a path that was
never referenced. The italic styling between the two underscores is invisible
in a copy-paste, so the user (and any agent reading the transcript) is handed
a factually wrong path and sent down the wrong diagnostic trail.

## Reproduction

```sh
bt prose 'failed for "prompts/_reviews/a.md": target does not exist: /x/prompts/_reviews/a.md' | command cat -v
```

Output (note the `^[[3m` … `^[[0m` italic span and both underscores gone):

```
failed for "prompts/^[[3mreviews/a.md": target does^[[0m ^[[3mnot exist: /x/prompts/^[[0mreviews/a.md
```

## Root cause

Two layers combine:

1. **Claudine renders raw error text as Prose markup.**
   `claudine-cli/src/log.rs` `error()` does
   `Prose::new(format!("<red><bold>Error:</bold></red> {msg}"))` — `msg` is
   interpolated verbatim into a Prose document, so every Markdown-significant
   character in the dynamic text (`_`, `*`, `[`, `<`, backticks) is live
   syntax. The same pattern exists on other dynamic-text surfaces (see
   Affected surfaces).

2. **Prose's emphasis flanking rule is deliberately simplified.**
   `biscuit-terminal/lib/src/components/prose/markdown.rs` converts
   `_text_` → `<i>text</i>` and treats a `_` as literal only when it is
   alphanumeric on *both* sides (`is_intra_word`). In `prompts/_reviews` the
   underscore follows `/`, so it qualifies as a delimiter. Full CommonMark
   flanking rules would not have paired these two underscores (the closer is
   preceded by punctuation and followed by a letter, so it is not
   right-flanking), but the simplification was an explicit design choice
   (predictability over spec parity — see
   `biscuit-terminal/features/2026-05-05-prose-plus/spec.md`), and this spec
   does not propose reopening it.

## Affected surfaces

The injection point is any site that interpolates *uncontrolled* text (error
messages, paths, provider output) into a `Prose` source string. Known
claudine sites:

- `claudine-cli/src/log.rs` — `error()`, and the other level helpers
  (`warn`, `info`, …) to the extent they receive dynamic text.
- `render_top_level_error` fallback path in `claudine-cli/src/main.rs`
  (routes `report.to_string()` — an arbitrary eyre chain — into
  `log::error`).
- Any `output::error_walker` / status-line surface that folds typed error
  `Display` output into Prose markup rather than a pre-escaped node.

An audit pass is part of this fix: `rg 'Prose::new\(format!' claudine/` and
classify each hit as static template (safe) vs. dynamic interpolation
(needs escaping).

## Required behavior

1. Dynamic text interpolated into a Prose-rendered error/status surface must
   reach the terminal byte-for-byte (modulo wrapping): no Markdown emphasis,
   link, or code-span interpretation, and no consumption of `_`, `*`, `[`,
   `]`, `(`, `)`, `<`, `>`, or backticks.
2. The static template portions (e.g. the `<red><bold>Error:</bold></red>`
   label) must keep their Prose styling.
3. Output must remain correct at every `ColorDepth`, including `None`
   (`--plain` / `NO_COLOR`): escaping must not leak backslashes or sentinel
   characters into plain output.
4. The fix must not change how *intentional* Prose/Markdown-authored content
   renders (help text, hook descriptions, composed document previews).

## Proposed direction

Prose already honors backslash escapes for its Markdown phase (`\_`, `\*`,
`\[`, `\]`, `\(`, `\)`) and tag-open escapes for the block-tag grammar. The
work lands in the area that owns each concern:

- **`biscuit-terminal` owns the escaping.** Prose owns the grammar, so Prose
  exports the literal-text facility — the character set can then never drift
  from the parser. Either shape is acceptable, chosen during implementation:
  - a public `prose_escape(text: &str) -> String` (name per crate
    conventions) that escapes every Markdown- and tag-significant character
    the Prose pipeline recognizes, kept adjacent to the tokenizer it mirrors
    and tested against it; or
  - a `Prose::text_raw(...)`-style constructor/segment API that carries a
    span through parsing as opaque literal text (analogous to the existing
    fenced-code placeholder lift), which is the more robust long-term shape
    since it cannot be defeated by future grammar growth.
- **`claudine` owns the call sites.** `claudine-cli` routes every dynamic
  interpolation on an error/status surface through the biscuit-terminal
  facility (`log::error` escapes `msg` before embedding it in the styled
  template), and performs the call-site audit below. No claudine-local copy
  of the escape set: claudine must not hard-code knowledge of another
  crate's grammar.

Out of scope:

- Changing Prose's emphasis flanking rules (documented design choice; a
  spec-parity change has monorepo-wide blast radius).
- The proxy-target path-resolution semantics that produced the underlying
  error in the observed incident (source-dir-relative vs repo-root-relative
  `proxy:` targets in `prompts/review.md`) — that is authoring-side and
  separate; this spec covers only the display corruption.

## Acceptance criteria

1. `claudine` reproduces the incident error (a `HarnessError` whose text
   contains two `_`-bearing paths) with both underscores displayed literally
   and no italic span, at `ColorDepth::None` and at a styled depth.
2. In `biscuit-terminal`: unit tests on the literal-text facility cover two
   underscore paths in one message, `*`-bearing text, `[text](ref)`-shaped
   text, backtick-bearing text, and text containing a literal `<tag>`-like
   token — each asserting byte-for-byte round-trip through a Prose render.
3. The `Error:` label keeps its red/bold styling on TTY output.
4. In `claudine`: audit of `Prose::new(format!` interpolation sites in
   `claudine-cli` completed; each dynamic-text site either routed through
   the biscuit-terminal facility or recorded as intentionally
   Markdown-aware.
