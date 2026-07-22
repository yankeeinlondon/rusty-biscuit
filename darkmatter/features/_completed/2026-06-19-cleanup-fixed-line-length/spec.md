---
created: 2026-06-19
clarified: claude_code/claude-opus-4-8
status: clarified
---

> **Status.** This feature is already implemented and shipped across the
> library, the CLI, and the docs (the companion `plan.md` is fully checked off).
> This spec has been **reconciled** to match the shipped behavior and to record
> the correctness fixes the shipped code still needs: the structural-safety
> requirements in [Decision 1](#decision-1--structural-safety-required-fixes)
> and the Unicode-script join rule in
> [Decision 2](#decision-2--join-separator-by-unicode-script). Sections are
> labeled to distinguish **shipped** behavior from **required (not yet
> implemented)** behavior at the time of this reconciliation. The initial
> implementation also preserved list prefixes but did not unwrap complete
> list-item prose blocks before reflow. That later-discovered shipping gap and
> its list-aware completion are documented in the
> [fixed-width lists fix](../../../fixes/2026-07-13-fixed-width-lists/spec.md).

In Markdown we have a **clean** operation -- which is reachable from the terminal via `md clean <file>` and it does a number of things to
help a markdown file be as standard based as possible.

One thing it doesn't do currently is address a Markdown file who's lines have been fixed to a certain length.
LLM's commonly do this at 80 or 100 characters. I guess the idea is that this is a "feature not a bug" because
strictly from an spec standard there is nothing wrong with putting a single `\n` to truncate.

> Note: in Markdown two or more `\n` characters represents a paragraph boundary but a single `\n` has no semantic
> meaning.

The problem though is that Markdown is a format for _writing_ first (reading second) and if you are adding in extra `\n`
because you think it might look better in some editors (aka, those without word wrap turned on) you are at the same time going against
the principle of Markdown's "notational velocity" which aims to provide an author a clean surface to write content
and stay in the flow. Ironically you might also be making it look far worse if it's being displayed in a terminal
with less columns than your cutoff point.

So in summary, in theory there is nothing with a bunch of single `\n` characters added to a Markdown file but
in practice it most typically not the best choice.

## Feature

- by default Darkmatter's library should include removing all single `\n` characters.
    - we must apply Markdown nuance
    - the join separator between two rejoined lines is decided by inspecting the boundary characters, per
      [Decision 2](#decision-2--join-separator-by-unicode-script). For space-delimited scripts this means:
      if the line with the `\n` we are about to remove already ends in whitespace then we simply remove
      the `\n` character, otherwise we replace the `\n` with a single space.
    - the strip pass must **not** destroy structurally-significant single `\n` constructs (hard line breaks and
      setext-heading underlines); see [Decision 1](#decision-1--structural-safety-required-fixes).
- there should be an option in the library and the CLI to also:
    - be **neutral** to the input and ignore whatever single `\n` pattern is being used in document
    - allow a fixed length to specified (`ch` as unit):
        - the caller specifies the fixed length they prefer
        - again we must avoid the naive implementation which ignores the inputs single `\n` and just adds new `\n` at the requested fixed length

> **Requirement (un-wrap must precede re-wrap).** Fixed-width reflow must respect the input's existing
> single-`\n` pattern: incidental newlines are stripped first so the reflow starts from canonical
> paragraph/block form, never from the source's own wrapping. The compose pipeline (which includes the
> `clean` operation) must stay performant, so this work must not add allocations or passes beyond what the
> existing cleanup phase budgets for.
>
> *Implementation note (non-normative).* The shipped code uses the obvious two-pass strategy (strip all
> incidental `\n`, then re-wrap to the requested fixed width). A single-pass solution producing
> byte-identical output is permitted if it stays within the same perf budget; pass count is not part of
> the contract.

### CLI

- `md clean <file>` - will by default remove all single `\n` found in the content
- `md clean <file> --fixed-width <#>` - allows the caller to specify a new width to use as the fixed width
- `md clean <file> --ignore-incidental-newlines` - will ignore the single `\n` characters and make no mutations to the input in this regard
    - this is the **only** opt-out (no environment variable, no frontmatter switch); see
      [Decision 3a](#decision-3a--default-on-backward-compatibility)
    - `--ignore-incidental-newlines` hard-conflicts with `--fixed-width`; see
      [Decision 3b](#decision-3b--flag-naming--interaction)

## Decision 1 — Structural safety (REQUIRED fixes)

*Status: not yet implemented in the shipped code; both items below need fail-first (currently-failing) tests
added to the cleanup test suite.*

The default strip pass (`strip_incidental_newlines`) currently destroys two structurally-significant
single-`\n` constructs. Both must be **preserved** — the boundary at that newline must be forced to
`Preserve`, never dropped or replaced with a separator.

- **Hard line breaks.** A line ending in two-or-more trailing spaces, **or** a trailing backslash `\`, is a
  CommonMark hard break. The newline at that boundary must be **preserved**. The shipped
  `ends_with(char::is_whitespace)` check is wrong: it treats a two-trailing-space hard break as "drop the
  newline," silently deleting the break. That is a bug to fix.
- **Setext headings.** A prose line immediately followed by a line of `===` (or `---`) underline is a setext
  heading; the newline **before** the underline must be preserved. The shipped `is_structural_line`
  classifier protects `---`, `***`, `___`, `#`, `|`, and `::`, but **not** `===`. The fix adds an
  `===` / `---` setext-underline lookahead to the line classification so the preceding prose line is not
  joined into the underline.

## Decision 2 — Join separator by Unicode Script (REQUIRED fix)

*Status: not yet implemented in the shipped code. This REPLACES the literal-whitespace join rule described
in the Feature section above.*

The literal whitespace rule ("drop the `\n` if the line ends in whitespace, else insert a space") is correct
only for space-delimited scripts and corrupts spaceless scripts (e.g. it injects stray spaces between Han
characters). When rejoining, the separator is decided by inspecting the **last scalar** of the line before
the `\n` and the **first scalar** of the line after:

1. **No separator** iff **both** boundary chars are letters (Unicode General_Category `L*`) whose Unicode
   **Script** is in the curated spaceless set: **Han, Hiragana, Katakana, Bopomofo, Thai, Lao, Khmer,
   Myanmar, Tibetan** (the set is extensible).
2. **Hangul is deliberately excluded** from the spaceless set — Korean uses eojeol word spaces, so Hangul is
   treated as space-delimited. This matches Prettier, CSS Text Level 3, and goldmark precedent.
3. At a **script-transition boundary** (one side in the spaceless set, the other not — e.g. Han followed by
   Latin), emit **no separator** (neutral reconstruction). Un-wrapping is explicitly **not** "pangu" spacing:
   never inject a Han↔Latin readability space.
4. **ZWSP (U+200B)** on either side → **no separator**.
5. Otherwise (the space-delimited case) → the existing behavior: drop the `\n` if the prior line already ends
   in whitespace, else insert a single `U+0020` space.

Implementation constraints:

- Key off the Unicode **Script** property — **not** East-Asian-Width (which cannot distinguish emoji from Han
  or Thai from Latin) and **not** full UAX #14 line breaking.
- Add `unicode-script = "0.5"` to `darkmatter/lib/Cargo.toml` `[dependencies]`. It is already resolved in
  `Cargo.lock` (0.5.8, transitively via `biscuit-terminal → resvg → usvg → rustybuzz`), so this is zero added
  build cost.
- The decision is O(1) per join boundary (two trie lookups, no allocation), preserving the pipeline
  performance this spec stresses.

## Acceptance Criteria

**Shipped (already implemented and tested):**

- `md clean <file>` strips incidental single `\n` by default; runs of two or more `\n` (paragraph boundaries)
  pass through unchanged.
- Fenced/indented code blocks, inline code spans, tables, HTML blocks, blockquote prefixes, list markers, and
  transclusion directives (`::file`, `::code`, `::shell`, `::disclosure`, …) are preserved verbatim across the
  strip.
- `md clean <file> --fixed-width <#>` reflows prose to the target column count (`ch` measured as typographic
  width, not byte length) while preserving block structure; the longest reflowed prose line is `<= width`.
- `md clean <file> --ignore-incidental-newlines` makes no incidental-newline mutation; the rest of cleanup
  (whitespace, list spacing, indent) still applies.
- `--fixed-width` and `--ignore-incidental-newlines` are mutually exclusive at the clap level and exit
  non-zero with a clear error when combined.
- `ComposeOptions` exposes `with_incidental_newline_mode(IncidentalNewlineMode)` and `with_fixed_width(usize)`;
  the default mode is `IncidentalNewlineMode::Strip`.
- `--fixed-width` reflow measures column count by Unicode display width (`UnicodeWidthStr::width`), not char
  count or bytes (wide chars count as 2).
- A token wider than the target width (e.g. a long word or URL) is emitted on its own line and is allowed to
  overflow; words/URLs are **never** broken mid-token. (Conventional `textwrap` behavior.)
- Reflow runs **after** the incidental-newline strip, so it reflows canonical collapsed paragraph form, never
  the source's own wrapping.
- Reflow only touches unprotected prose lines; code blocks, tables, lists (prefix preserved, body wrapped),
  headings, blockquotes, HTML blocks, and `::` directives are preserved. The original list-body implementation
  was incomplete; complete logical list-paragraph reflow and composite hanging prefixes landed in the
  [fixed-width lists fix](../../../fixes/2026-07-13-fixed-width-lists/spec.md).

**Required (not yet implemented — needs fail-first tests):**

- A prose line ending in two-or-more trailing spaces, or a trailing `\`, keeps its `\n` (hard break preserved).
- A prose line immediately followed by a `===` or `---` setext underline keeps the `\n` before the underline
  (setext heading preserved).
- Join-separator selection follows the Unicode-Script rule. Tests must cover: Han↔Han (no space), Thai↔Thai
  (no space), Hangul↔Hangul (space inserted), Han↔Latin (no space, neutral reconstruction), emoji↔emoji (not
  mis-joined as CJK), and CJK punctuation (`。` / `、`) before a line.
- ZWSP (U+200B) on either boundary side yields no separator.
- CJK / spaceless (Thai, Lao, …) runs under `--fixed-width` are treated as a **single atomic token** — they
  are not broken between ideographs and may overflow the target width, exactly like a long URL. This is the
  **v1 contract** (intra-ideograph / kinsoku-aware CJK line breaking is explicitly out of scope for v1). A
  regression test must pin this: a wide CJK run measured at display-width 2/char that exceeds the target width
  stays on one line.

## Decision 3a — Default-on backward compatibility

Strip-incidental-newlines remains the **default** (`IncidentalNewlineMode::Strip`), matching this spec's
stated intent. This is a backward-compat change: existing `md clean` and compose-pipeline cleanup output
changes repo-wide. The flip is accepted, but **must be documented**: a CHANGELOG/README note must flag that
"`md clean` now collapses incidental single newlines by default; opt out with `--ignore-incidental-newlines`."
The CLI flag is the only opt-out — no environment variable, no frontmatter switch.

## Decision 3b — Flag naming + interaction

- The opt-out flag is named `--ignore-incidental-newlines` (FINAL). This corrects the spec's earlier
  misspelled `--ignore-incidental-carraige-returns`.
- `--ignore-incidental-newlines` **hard-conflicts** with `--fixed-width` (mutually exclusive; a clap-level
  conflict producing a clear error), because reflowing to a fixed width while refusing to strip the source's
  own wrapping is contradictory.

## Out of Scope (v1)

- Intra-ideograph CJK line breaking under `--fixed-width` (kinsoku rules) — spaceless runs are atomic in v1.
- "Pangu" Han↔Latin readability spacing — un-wrapping stays neutral at script transitions (already stated in
  [Decision 2](#decision-2--join-separator-by-unicode-script)).
- Mid-token / mid-URL breaking during reflow.
- Hangul treated as spaceless — Hangul is space-delimited (see
  [Decision 2](#decision-2--join-separator-by-unicode-script)).
