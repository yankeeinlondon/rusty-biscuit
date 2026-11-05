# Spike: what `plain` output looks like today

Date: 2026-11-05. Time-boxed (~1 h) spike for the "Terminal degradation and
`plain`", "Code in `plain`", "Links and images that cannot be clickable", and
"Width" sections of `spec.md`. No library or CLI source was changed; the
throwaway harness lives outside the repository (see [Method](#method)).

## Summary

- **Code meaning survives everywhere.** Every path keeps inline backticks and
  `` ```yaml `` / `` ```mermaid `` fences with no styling (the darkmatter
  code renderer returns `None` on `ColorDepth::None`, so the generic plain
  fence fallback in `render.rs::render_code` runs). Mermaid stays a fence.
- **No path is escape-free.** Darkmatter's page path leaks a hard-coded
  truecolor SGR on the block-quote bar; `Markdown::as_terminal` with a width
  on a non-TTY switches to the *optimistic* profile and emits OSC 8 links,
  even with `HyperlinkMode::Never` (which nothing on the render-tree path
  reads).
- **Links and images follow none of the decided rules.** Every path prints
  `[text](url)` (and `[url](url)` for an autolink); wrapping can hyphenate
  inside that URL. Images print `[alt]` (generic) or `▉ IMAGE[alt]` (page).
- **Width is inconsistent.** The generic fold does not wrap prose unless
  `context.wrap_prose` is set but always fills rules to the full width; the
  zero-config `DarkmatterPage` ignores its terminal's width and renders at
  the ambient width (80 on a pipe); widths above 65 535 are silently clamped
  (`u16`) on the darkmatter paths.
- **"Unlimited" as a large number is not viable.** At width 100 000 the
  horizontal rule alone is 100 000 glyphs (300 KB of a 302 KB output).
- No panics or hangs; every render took under 35 ms (the first call includes
  one-off theme loading; the rest took 0.6 to 11 ms).

## Method

A scratch binary with path dependencies on the worktree's `darkmatter`,
`biscuit-terminal`, and `renderable` crates rendered one corpus. It ran with
`TERM=dumb`, no `COLORTERM`/`TERM_PROGRAM`/`COLUMNS`, and stdin/stdout not a
TTY.

The corpus has a soft-broken paragraph, a long paragraph, inline code,
bold/italic, a `yaml` fence, a 3-column table, `---`, a block quote, a nested
list, an ordered list, a link (text differs from URL), an autolink (text
equals URL), a relative image, a `::disclosure` block, and a `mermaid` fence.

The "plain" terminal profile was `Terminal::new_optimistic(w)` with
`is_tty = false`, `color_depth = None`, `osc_link_support = false`, no
italics/underline, and `GraphicsRoute::from(ImageSupport::None)`.
`supports_unicode` was left `true`.

| Path | What it calls |
|---|---|
| A, generic fold | `Markdown::as_document()` → `render_terminal_document` with `TerminalRenderOptions::new(&plain, Warn)` (no code or diagram hook) |
| A2, generic fold with wrapping | A, plus `context.wrap_prose = true` |
| B, `as_terminal` with a width | `Markdown::as_terminal` with `color_depth: Some(None)`, `image_mode: Never`, `hyperlink_mode: Never`, `mermaid_mode: Off`, `max_width: Some(w)` |
| B0, `as_terminal` without a width | B with `max_width: None` (ambient width) |
| C, zero-config page | `DarkmatterPage::new(&plain).with_color_depth(None).with_hyperlink_mode(Never).with_image_mode(Never).render(md)` |
| C2, page with a frame | C plus `.with_margin_x(2)`, which takes the `render_page_terminal_document` branch |
| D, `Prose` alone | `Prose::new(corpus).with_word_wrap(WordWrap::WrapProse(None, None)).render(&plain)` |
| E, Markdown fold | `render_tree_markdown(md)` (renderable `MarkdownRenderOptions`, which has no width) |

## Measurements

Max line is measured in characters after removing escape sequences.

| Path | Width | Bytes | Time | ESC | OSC 8 | Fences kept | `` `md compose` `` kept | Max line | Lines with trailing spaces | Non-ASCII glyphs |
|---|---|---|---|---|---|---|---|---|---|---|
| A | 40 | 1 662 | 33 ms (theme load) | 0 | 0 | yes | yes | **334** | 0 | box drawing, `╌`×40 |
| A2 | 40 | 1 664 | 0.8 ms | 0 | 0 | yes | yes | 40 | 0 | box drawing, `╌`×40 |
| B | 40 | 1 769 | 11 ms | **14** | **4** | yes | yes | 40 | 0 | box drawing, `╌`, `▉`, `▐` |
| C | 40 | 2 161 | 2 ms | **4** | 0 | yes | yes | **80** | 0 | box drawing, `╌`×**80**, `▉`, `▐` |
| C2 | 40 | 2 033 | 8 ms | **16** | **4** | yes | yes | 40 | **64** (156 B) | box drawing, `╌`, `▉`, `▐` |
| D | 40 | 1 249 | 3.5 ms | 0 | 0 | yes | yes | 40 | 0 | none |
| A | 80 | 2 100 | 0.6 ms | 0 | 0 | yes | yes | 334 | 0 | box drawing, `╌`×80 |
| B | 80 | 2 181 | 0.8 ms | 12 | 4 | yes | yes | 80 | 0 | box drawing, `▉`, `▐` |
| C | 80 | 2 161 | 0.7 ms | 4 | 0 | yes | yes | 80 | 0 | box drawing, `▉`, `▐` |
| C2 | 80 | 2 370 | 0.8 ms | 12 | 4 | yes | yes | 80 | 50 (128 B) | box drawing, `▉`, `▐` |
| D | 80 | 1 245 | 0.8 ms | 0 | 0 | yes | yes | 79 | 0 | none |
| A / A2 | 100 000 | **301 856** | 2.5 ms | 0 | 0 | yes | yes | **100 000** | 0 | `╌`×100 000 |
| B | 100 000 | 198 516 | 2 ms | 10 | 4 | yes | yes | **65 535** | 0 | `╌`×65 535 |
| C | 100 000 | 2 161 | 1.1 ms | 4 | 0 | yes | yes | **80** | 0 | `╌`×80 |
| C2 | 100 000 | 198 681 | 2.2 ms | 10 | 4 | yes | yes | 65 535 | 44 (116 B) | `╌`×65 531 |
| D | 100 000 | 1 243 | 0.8 ms | 0 | 0 | yes | yes | 334 | 0 | none |
| B0 | ambient | 2 161 | 1 ms | 4 | 0 | yes | yes | 80 | 0 | same as C |
| E | n/a | 1 133 | 0.7 ms | 0 | 0 | yes | yes | 334 | 0 | none |

## What each element looks like

| Element | Generic fold (A/A2) | Darkmatter page (C) | Decided `plain` rule |
|---|---|---|---|
| Soft-broken paragraph | joined into one line; wrapped only when `wrap_prose` is set | joined, wrapped | joined, wrapped when a width is known |
| Inline code | `` `md compose` `` | same | keep backticks ✓ |
| Bold / italic | markers dropped (`bold`) | same | styling removed ✓ (the rulings do not say whether `**` stays) |
| Code fence | `` ```yaml `` … `` ``` `` | same | keep fence and language ✓ |
| Mermaid | fence kept | fence kept | (fence) ✓ |
| Heading | `# Plain Corpus` | same | not ruled; `#` keeps the level visible |
| Table | Unicode box drawing, sized to content, cells wrap at narrow widths | same | not ruled |
| `---` | `╌` repeated to the full width | same, at the page width | not ruled; width-filling |
| Block quote | `│ ` prefix | `▐   ` prefix wrapped in `ESC[38;2;100;100;100m` | no escapes allowed |
| Lists | `-` and `1.` in ASCII, nested indent 2 | same | ✓ |
| Link (text ≠ URL) | `[the docs](https://example.com/docs)` | same | `the docs`; `-v`: `the docs <https://example.com/docs>` |
| Autolink (text = URL) | `[https://…/same](https://…/same)` | same | `https://…/same` once |
| Image | `[A diagram]` | `▉ IMAGE[A diagram]` | `IMAGE[A diagram]`; `-v`: `image: A diagram <./diagram.png>` |
| Disclosure | summary line, then the body behind a `│ ` prefix | same | not ruled; `│` is a box glyph |

At width 40 with wrapping on, the generic fold breaks inside a link and
**hyphenates inside a URL**:

```text
See [the
docs](https://example.com/docs) and
[https://example.com/same](https://exam-
ple.com/same).
```

`Prose` on its own (D) rewrote the relative image destination to an absolute
`file:///…/diagram.png` and wrapped that URL across three lines. (D is not a
document renderer: its paragraph wrapping also flattened the table, the list,
and the disclosure directive into prose. Only its wrapping behavior is
relevant here.)

## Gaps and fixes, by file

### `biscuit-terminal/lib/src/render_tree/render.rs`

1. **Link fallback** (the `NodeKind::Link` arm, about line 1160). Replace
   `[desc](url)` with the decided rule: text by default; `text <url>` at
   verbosity ≥ 1; the URL once when the text equals the URL. The fold needs
   the verbosity, for example as a `TerminalRenderContext` field filled from
   `OutputOptions::verbosity`.
2. **Image fallback** (`NodeKind::Image`, about lines 1175–1180). Make both
   `ImagePlaceholder::Bracket` (`[alt]`) and `Block` (`▉ IMAGE[alt]`) produce
   `IMAGE[alt]`, or `image: alt <path>` with `-v`. The `▉` glyph goes away.
   The decided rule leaves no reason to keep two placeholder variants.
3. **Block-quote prefix SGR** (`render_blockquote_with_prefix`, about line
   950). It formats `\x1b[38;2;100;100;100m{prefix}\x1b[39m` regardless of
   `color_depth`. Gate it on the color depth, like `render_code` already does.
4. **Unbreakable URLs.** Wrapping must treat a printed URL (`<url>` under
   `-v`, or a URL shown once) as one unbreakable token, never hyphenated. A2
   at width 40 hyphenates `https://exam-ple.com/same` today.
5. **Width-filling rules.** The horizontal rule repeats `╌` to the full
   available width; see [Unlimited width](#what-unlimited-must-mean). In
   `plain`, render it as `---`, which matches the "meaning survives" rule
   applied to code. This needs an owner decision.
6. **Glyphs in `plain`** (an open question, not one of the rulings). Tables
   (`┌─┬┐│`), the disclosure and quote bars (`│`, `▐`), and the rule (`╌`) are
   all non-ASCII. If `plain` means "styling off" only, the table borders can
   stay because they are structure. If `plain` must be byte-safe for scripts
   (for example as a replacement for `md get --raw`), the profile should also
   set `supports_unicode = false`, and the renderer must honor it. Today the
   tree renderer never reads `supports_unicode` (the only use is one test at
   line 4114). Recommendation: tables keep box drawing, which is readable and
   structural; quote and disclosure bars become `> `; the rule becomes `---`.

### `darkmatter/lib/src/markdown/render_tree/entrypoints.rs`

7. **`capability_terminal`** (line 674). With a `max_width` on a non-TTY it
   returns `Terminal::new_optimistic(width)` (TrueColor and OSC 8). That is
   why path B emits OSC 8 links on a pipe. `plain` must never route through
   this. The planned `plain` profile has to pin `osc_link_support = false`
   after the capability selection, or `max_width` must stop selecting
   capabilities (the page path already separates the two in
   `render_page_terminal_document`).
8. **`TerminalOptions::hyperlink_mode` is dead** on the render-tree path.
   `terminal_options_from_terminal_options` never reads it, and
   `HyperlinkMode::Never` had no effect in B or C2. Either map it onto
   `context.terminal.osc_link_support` or delete it in favor of the profile.
9. **`blockquote_prefix = "▐   "`** (line 774). This is the glyph the leaking
   SGR in item 3 wraps. In `plain`, use `> `.

### `darkmatter/lib/src/layout/page.rs`

10. **The zero-config page ignores its terminal's width.**
    `DarkmatterPage::new(&term)` captures `term.width()`, but
    `is_default_layout()` routes to `render_tree_terminal(md, &options)` with
    `max_width: None`, which re-detects the ambient terminal
    (`Terminal::default()`, 80 on a pipe). C rendered at 80 for requested
    widths of 40 and 100 000. `--width N` therefore cannot reach `md render`
    through the zero-config path without a fix.
11. **`u16` width** (`clamp_width`, `TerminalOptions::max_width: Option<u16>`,
    `terminal_width: u16`). It silently clamps to 65 535, which cannot
    express "unlimited".
12. **Trailing whitespace from frame decoration** (`apply_row_decoration`).
    With `margin_x(2)`, every line gets two trailing spaces and blank rows
    are padded (64 lines, 156 B at width 40). With no page background, a
    right margin and padding should emit nothing; in `plain`, strip trailing
    whitespace from every line.

### `biscuit-terminal/lib/src/terminal.rs`

13. **`Terminal::width()` falls back to 80** when the width is not fixed and
    stdout is not a TTY (through `terminal_width()`). The decided rule says an
    unknown width is unlimited. Width resolution has to return a type that can
    say "unknown/unlimited" (see below) rather than `u32`.
14. Add a single plain-profile constructor (for example
    `Terminal::plain(width)` or a `TerminalProfile::plain()` in
    `OutputOptions`) so that no caller hand-assembles the profile as this
    spike did, and so that items 3, 7, and 8 cannot regress.

### `renderable/src/tree/render/markdown.rs`

15. **No width.** `MarkdownRenderOptions` has `dialect`, `strictness`, and an
    unused `style`. It writes soft breaks exactly as authored (E keeps the
    three-line paragraph and the 334-character line).
    `--output markdown --width N` on a non-document command (whose prose
    comes from `as_prose()`) therefore cannot be honored by the fold today.
    The only existing reflow, `darkmatter::markdown::cleanup::reflow::reflow_to_width`,
    lives in darkmatter, and the dispatch in biscuit-terminal cannot depend on
    darkmatter. **Recommendation:** add a `width` field to
    `MarkdownRenderOptions` with three values. `Source` (today's behavior and
    the default) writes soft breaks as authored. `Unlimited` joins each
    paragraph's soft breaks into one line, which matches what the terminal
    fold does. `Columns(n)` joins and wraps paragraph text at word
    boundaries, never inside a code span, link destination, or autolink.
    Code blocks and tables are not reflowed.
16. Side observations on E, not width related: a nested list is written as a
    loose list (a blank line after `- Item one`); an autolink becomes
    `[url](url)`; a disclosure summary moves onto its own line after
    `::disclosure`.

## What "unlimited" must mean

The spike shows that a large number is the wrong representation. At 100 000,
the rule alone produced 300 KB; darkmatter clamps the value to 65 535; and any
component that pads to the width (code bands in color, page margins and
background, `Width::Fill` component policies) scales with it.

**Recommended rule:** "Unlimited means no wrapping and no filling. Every
component renders at its intrinsic width."

| Component | Under a known width `N` | Under unlimited |
|---|---|---|
| Prose paragraphs | joined, wrapped at `N` | joined, never wrapped |
| Code blocks | lines unchanged, with the per-line constraint at `N` | lines unchanged |
| Code band (color terminals) | padded to `N` | padded to the longest line in the block, plus its padding |
| Tables | planned to fit `N`, cells wrap | natural column widths, no cell wrapping |
| Horizontal rule | `N` glyphs (`---` in `plain`) | a fixed short form: `---` in `plain`; in a terminal, the width of the widest rendered block, or a constant such as 3 |
| Page right margin and background band | padded to `N` | omitted; nothing to fill to |
| Page left margin and padding, list and quote indentation | applied | applied (they are not fills) |
| `max-width` and `Width::Fill` policies | resolved against `N` | `Fill` resolves to the intrinsic width; `max-width` still caps |

Represent width as an enum (for example
`enum Width { Columns(u32), Unlimited, Source }`) all the way into
`TerminalRenderContext`, instead of a `u32`/`u16` sentinel, so that each
width-filling component has to choose its unlimited behavior explicitly. A
compile error is a better guard here than a large number.

## Does the trait need a `plain` override?

**No.** Every path already turns styling off when it is handed a no-color
profile. The defects found are capability leaks: an SGR that ignores the
color depth, a width that selects an optimistic profile, and a hyperlink
mode that nothing reads. None of them is a reason for a separate renderer.
The spec's "`plain` = the `terminal` path with every capability off" holds.
It depends on three conditions:

1. one canonical plain profile (item 14);
2. the leaks fixed (items 3, 7, 8, 12);
3. glyph policy decided (item 6), carried as a profile capability
   (`supports_unicode`, or a narrower "ascii decorations" flag) rather than
   as a per-target override.

A value that overrides `render_terminal` (the Darkmatter document) gets a
matching `plain` only if its override also honors the profile. Today
`DarkmatterPage` does not (items 3, 10, 12), so the Darkmatter override must
be fixed along with the generic fold.

## Not covered by this spike

- Code-block lines longer than the width. The corpus's code lines are short,
  so "Width is a straight per-line constraint" (truncate, hard-wrap, or leave
  as is?) is unverified and still needs a ruling.
- The `md` binary itself (all calls were library calls) and the colored
  terminal path.
- `supports_unicode = false`, because the tree renderer ignores it.
