---
kind: feature
name: extra-report-formats
date: 2026-10-11
status: draft-spec
area: tree-hugger
packages:
  - tree-hugger-cli
depends-on:
  - 2026-10-03-symbol-filters-and-reports
  - 2026-10-05-deferred-kind-precision
  - 2026-10-06-symbol-type-relations
---

# Spec: TSV, Markdown, Prose and Tags Output for Symbol Reports

## Problem

After `2026-10-03-symbol-filters-and-reports`, symbol reports support terminal
text, `--format plain|table`, JSON and JSON Lines. Spreadsheet tools, docs and
pull requests, callers that render rich text to Markdown or HTML, and editors
that read tags files have no direct format.

## What was proposed

Proposed in the pre-split draft of `2026-10-03-symbol-improvements` (item D6),
and moved here by the author on 2026-10-03; `prose` added by the author on
2026-10-05. Extend `--format` with:

- **`tsv`:** a header `file`, `line`, `column`, `kind`, `name`,
  `qualified_name`, `visibility`, `signature`; encode tabs, CR, LF and
  backslashes as literal escapes, leaving absent values empty.
- **`markdown`:** nested lists, escaping signature/name content so it can be
  pasted into docs and PRs without creating unintended links or markup.
  `--tree` would also be valid with Markdown output.
- **`prose`:** the report as biscuit-terminal `Prose` markup, unrendered.
  Prose takes a Markdown-like syntax (links, `**bold**`, `_italics_`, code
  spans) plus styling Markdown lacks (`<dim>`, color and background tags,
  `<rgb #hex>`, `<a href>`), and renders through the shared render tree to
  the terminal, HTML, and Markdown or MarkdownPlus. A caller can store the
  output, or embed it in its own Prose content, and render it to whichever
  target it needs.
- **`ctags`:** a standard tags file (see the open question below).

`tsv`, `markdown` and `ctags` are machine-oriented: no ANSI styles, OSC 8
links or width wrapping. `prose` also carries no escape sequences or
wrapping; its styling and links are Prose tags, applied when the caller
renders it.

### The `prose` format

The terminal renderer (`cli/src/render.rs`) already builds every listing line
as Prose markup and renders it through biscuit-terminal. `--format prose`
writes that same markup instead of rendering it, so there is one layout with
two outputs, not a second renderer.

- The layout follows the default terminal output: file groups, `--tree`
  outlines, signature modes and doc lines.
- Source-derived text (names, signatures, doc comments, paths) is escaped
  with `Prose::escape_text`, as the terminal path already does through
  `literal`, so markup in source is shown, never interpreted. Link targets
  use `Prose::quoted_attr`.
- Locations link to their source with `<a href>`, as the terminal path's
  OSC 8 links do.
- Test invariant: rendering the `prose` output with `Prose` for
  a plain terminal gives the same text as `--format plain`.
- `markdown` should be the `prose` output folded through Prose's Markdown
  renderer, so the two cannot drift apart. The plan confirms that renderer's
  escaping meets the `markdown` rule above, or records where it does not.

## Scheduling

Scheduled 2026-10-11, last of the seven, so every format can carry what the
earlier features add: precise kinds (which the tags kind mapping depends on),
relations and metrics.

It was first deferred because no consumer had asked for these formats. The
author ruled on 2026-10-05 that a missing consumer is not a reason to defer:
formats have to exist before callers can adopt them. The tags contract below
still needs an author decision before editor integration is advertised.

## Prerequisites

- `2026-10-03-symbol-filters-and-reports` (`--format`, qualified names,
  signature modes and the one-output-mode rule).
- `2026-10-05-deferred-kind-precision` and
  `2026-10-06-symbol-type-relations` (the kinds and relations the formats
  report).

## Open question carried over: tags output compatibility

Choose the tags-file contract before advertising editor integration.

- **Minimal format:** name, portable relative file path and numeric line
  address. Easy to implement/test; loses kind and container information.
- **Extended format:** numeric addresses plus documented kind/scope fields and
  format/sort headers. Better editor navigation; needs escaping and a reviewed
  mapping from language-specific kinds to tag kinds.
- **Recommendation (pre-split draft):** extended format, because kinds and
  containers are a principal goal. Document escaping for
  tabs/newlines/backslashes, mark files unsorted unless sorted by tag name, and
  omit synthetic or locationless tags. Do not claim full Universal Ctags
  compatibility until the chosen subset is checked against its format contract
  and an editor-reader fixture.

## Notes

- **Testing:** one golden per format over the 16-language `multi/` fixture
  tree, compared after the existing `corpus` ANSI-stripping and path-redaction
  helpers, plus focused TSV escaping cases, a `prose` case whose source text
  contains Prose markup (`<red>`, `**x**`, `_y_`), and the `prose`-to-`plain`
  round trip above.
