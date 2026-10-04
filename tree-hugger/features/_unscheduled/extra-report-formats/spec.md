# Spec: TSV, Markdown and Tags Output for Symbol Reports

## Problem

After `2026-10-03-symbol-filters-and-reports`, symbol reports support terminal
text, `--format plain|table`, JSON and JSON Lines. Spreadsheet tools, docs and
pull requests, and editors that read tags files have no direct format.

## What was proposed

Proposed in the pre-split draft of `2026-10-03-symbol-improvements` (item D6),
and moved here by the author on 2026-10-03. Extend `--format` with:

- **`tsv`:** a header `file`, `line`, `column`, `kind`, `name`,
  `qualified_name`, `visibility`, `signature`; encode tabs, CR, LF and
  backslashes as literal escapes, leaving absent values empty.
- **`markdown`:** nested lists, escaping signature/name content so it can be
  pasted into docs and PRs without creating unintended links or markup.
  `--tree` would also be valid with Markdown output.
- **`ctags`:** a standard tags file (see the open question below).

All three are machine-oriented: no ANSI styles, OSC 8 links or width wrapping.

## Why deferred

No current consumer has asked for these formats, and the tags contract needs
an author decision before editor integration is advertised.

## Prerequisites

- `2026-10-03-symbol-filters-and-reports` (`--format`, qualified names,
  signature modes and the one-output-mode rule).

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
  helpers, plus focused TSV escaping cases.
