---
area: darkmatter
status: draft-spec
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
reviewed: false
review_iterations: 0
created: 2026-10-11
owner: Ken Snyder <ken@ken.net>
packages:
    - darkmatter
    - darkmatter-cli
    - dmls
related:
    - 2026-08-27-preserve-backslash-escapes
human_review: false
implemented: false
---

# Clean Fidelity and Formatting Rules

## Problem

Darkmatter's Markdown cleanup promises to "fix common formatting noise without
changing meaning" ([`docs/cli/clean.md`](../../docs/cli/clean.md)). Two
in-repo consumers call the library cleanup directly; neither invokes the other:

- the `md` CLI: `md clean` and `md FILE --save`
  ([`cli/src/commands/clean.rs`](../../cli/src/commands/clean.rs))
- DMLS `textDocument/formatting`
  ([`dmls/src/providers/formatting.rs`](../../dmls/src/providers/formatting.rs)).
  `format_text` calls `Markdown::cleanup*` in-process, plus `reflow_to_width`
  when it is configured, and returns one whole-document `TextEdit`. Its module
  docs say "Frontmatter and directive lines pass through untouched." That is
  not true today.

DMLS cleans a document **only** when the client sends
`textDocument/formatting`. In practice that means an explicit "Format
Document", or format-on-save in an editor where DMLS is the selected Markdown
formatter. DMLS advertises no range, on-type, or `willSaveWaitUntil`
formatting. Neither the VS Code nor the Zed extension makes DMLS the default
formatter or turns on format-on-save.

There are four problems.

1. **Cleanup changes the CommonMark structure of plain Markdown.** Block
   content inside list items gets pushed out of its item. Lists split in two,
   and prose and nested lists become code blocks. No Darkmatter syntax is
   needed to trigger this.
2. **Cleanup ignores Darkmatter directives.** It parses the body as plain
   CommonMark. A column-0 directive line right after a paragraph counts as a
   _lazy continuation_ of that paragraph, so the serializer moves it into the
   container (`> ::end-block`, `    ::block …`). Compose no longer sees it as a
   directive, and the document fails to compose.
3. **The defaults rewrite things an author chose.** Every default entry point
   passes `IncidentalNewlineMode::Strip`, so authored hard wraps are joined.
   DMLS has no setting to stop this. The serializer's
   `increment_ordered_list_bullets: true` option renumbers a deliberate
   `1. 1. 1.` list to `1. 2. 3.`.
4. **Nothing can be controlled per document, and the two consumers behave
   differently.** The only controls are CLI flags and a DMLS workspace config
   (`cleanup` variant, `fixed_width`). Frontmatter repair is orchestrated inside
   the CLI (`cli/src/commands/clean/frontmatter_repair.rs`), so DMLS formatting
   never repairs frontmatter.

Found in a real file: running `md clean` on `prompts/commit.md` (blob
`464e2d90e`) gives a document with an unclosed `::block`, a blockquote pushed
out of a list item, a sub-list that changed its parent, a fenced example that
moved out of its list item, and every hard-wrapped paragraph joined into one
line.

### Reproductions

Every case below was run through `md clean` on 2026-10-11. "Structure" means
the HTML from the CommonMark reference renderer (`commonmark` 0.31.2) for the
input and for the output. Whitespace-only lines are written `·` here.

| # | Severity | Input | `md clean` output | Effect |
| --- | --- | --- | --- | --- |
| E1 | content → code | `4. step\n\n    > lead:\n    > \n    > - a\n    > - b\n\n5. next\n` | `4. step\n····\n>    lead:\n> \n>     - a\n>       - b\n\n5. next\n` | The blockquote leaves item 4, the ordered list splits in two, and the list inside the quote becomes an indented **code block** (`<pre><code>- a  - b`) |
| D1 | content → code | `1. top\n    - Example:\n\n        ```\n        code\n        ```\n\n        after\n2. next\n` | the fence moves from column 8 to column 5, with `········` lines around it | The fence leaves the `Example:` item, and the paragraph `after` becomes a **code block** |
| C1 | structure | `- item:\n\n    > {{message}}\n\n- next\n` | `- item:\n····\n>   {{message}}\n\n- next\n` | The blockquote leaves the item, and the list splits into two lists around a top-level quote |
| A1 | compose fails | `::block when="a"\n\n> note\n::end-block\n` | `…\n> note\n> ::end-block\n` | The CommonMark HTML is the same, but `md compose` now fails: "Add a matching `::end-block` directive to close the region" |
| B1 | directive moved | `1. first\n::block when="m"\n    - nested\n::end-block\n2. second\n` | `1. first\n    ::block when="m"\n\n    - nested\n        ::end-block\n\n2. second\n` | The directive lines are re-indented, one to column 8 (indented-code territory), and blank lines are added next to them |
| F1 | whitespace | `> a\n>\n> b\n` | `> a\n> \n> b\n` | A trailing space is added. Cleanup removes authored trailing whitespace elsewhere, yet adds its own here and in the whitespace-only lines of C1/D1/E1 |

### Root causes

These are hypotheses for the implementer to confirm; they are not yet proven.

- **Directive-blind parse.** `cleanup_content_internal`
  ([`lib/src/markdown/cleanup/mod.rs`](../../lib/src/markdown/cleanup/mod.rs))
  hides only _opaque directive bodies_ (`protect_opaque_directive_bodies`).
  Directive _lines_ reach pulldown-cmark as ordinary text. CommonMark's
  lazy-continuation rule then puts them inside the preceding paragraph's
  container, and `pulldown-cmark-to-cmark` writes that container's prefix in
  front of them. Compose's recognizer (`scan_darkmatter_directives`,
  [`compose/directives_api.rs`](../../lib/src/markdown/compose/directives_api.rs))
  does not accept a `> ` prefix, so the directive no longer exists.
- **Container prefixes lost for non-paragraph children of list items.**
  `pulldown-cmark-to-cmark` and the string passes that follow it
  (`fix_list_indentation`, `restore_list_markers`,
  `extract_additional_paragraph_contexts` in
  [`cleanup/lists.rs`](../../lib/src/markdown/cleanup/lists.rs)) re-indent
  paragraph lines by list-item context. They do not keep the item's
  continuation indent on blockquote, fenced-code, or nested-list lines inside
  that item. Those lines drop to the wrong column (E1, C1, D1), and the indent
  that was meant for them is left behind as a whitespace-only line.
- **No output check.** Nothing compares cleanup's output with its input, so a
  change in meaning is written to disk, or returned as an LSP edit, without
  anyone noticing.

## Design

The fix has two parts. **Invariants** are guarantees that every run of
cleanup keeps, whatever the configuration. **Rules** are the formatting a
document opts into or out of under `style.markdown`.

### Invariants (always on, not configurable)

1. **Directive lines are fixed points.** Every line that compose's directive
   recognizer accepts comes out of cleanup byte-for-byte unchanged, at the
   same column, in the same order, and in the same container. The same holds
   for opaque directive bodies such as `::shell`, and for `{{ … }}` expressions,
   which are never escaped. Cleanup never adds or removes a container prefix
   (`>`, list indent) on a directive line. It never inserts blank lines next to
   one either: in B1, a blank line after `::block` would turn the tight list
   into a loose one once compose removes the directive. Ordinary Markdown
   _between_ `::block` and `::end-block` is still formatted.
2. **CommonMark structure is invariant.** Rules may change spelling only. No
   rule may change the block tree, meaning which container each block is in,
   block kinds, the ordered-list start number, or code-block content.
3. **No trailing whitespace is introduced.** A whitespace-only line or a
   trailing space appears in the output only if it was in the input at the same
   logical position. Bytes inside fenced or indented code are preserved.
4. **Fidelity guard.** After the rules run, the output is compared with the
   input (see [Decision: guard on structure,
   fail closed](#decision-guard-on-structure-fail-closed)). If they are not
   equivalent, cleanup returns the input **unchanged** and reports one
   diagnostic, `clean.fidelity-fallback`, naming the first diverging block (line
   and column in the input, plus the kind of difference).
   - `md clean` prints the diagnostic to stderr and includes it in the `--json`
     envelope. `--save` writes nothing.
   - DMLS returns no edits and shows the diagnostic through
     `window/showMessage` at `Warning` level, so a format-on-save that declined
     is visible rather than silent.
   - Invariants 1–3 are the real fix. The guard is the backstop that turns any
     _remaining_ serializer bug into "not cleaned" instead of "corrupted".

### Rules: `style.markdown`

These keys are added to the baseline schema
([`docs/schemas/darkmatter.yaml`](../../docs/schemas/darkmatter.yaml)) under
the existing `style` root, so DMLS completes and validates them like any other
`style.*` key:

```yaml
style:
    markdown:
        frontmatter: "boolean(default(true)) -> Apply non-destructive frontmatter repairs."
        table-layout: "boolean(default(true)) -> Pad table cells so columns line up."
        fixed-width:
            - "number(integer; min(1); max(1000)) -> Unwrap, then re-wrap prose to this display width."
            - "boolean(default(false)) -> false: never re-wrap; true: re-wrap to 120."
        unwrap: "boolean(default(false)) -> Join single line breaks inside paragraphs."
        indentation: "enum(preserve, 2, 4, 8; default(4)) -> Spaces per nested-list level; preserve leaves list indentation as authored."
        ordered-lists: "boolean(default(true)) -> Renumber ordered lists that are neither uniform nor sequential."
        list-padding: "boolean(default(true)) -> Ensure one blank line before and after every list."
        list-interior-padding: "enum(preserve, none, only-prose, prose-and-child-elements; default(preserve)) -> Blank lines between a list item and its children."
        default-language: "string(default(text); suggest(text, plain)) -> Language added to unlabeled code fences; an empty string adds none."
```

The exact schema DSL spelling is the implementer's to get right; the types and
defaults above are the decision.

| Key | Default | Behavior | Example |
| --- | --- | --- | --- |
| `frontmatter` | `true` | Applies the deterministic, safety-proven repairs that `md clean` performs today. `false` leaves the frontmatter block byte-for-byte. | the repairs listed under "Frontmatter Repair" in `docs/cli/clean.md` |
| `table-layout` | `true` | Pads cells so each column has one width, and aligns the delimiter row. Never changes how the table renders. | `\| a \| bb \|` rows padded to equal widths |
| `unwrap` | `false` | Joins single newlines inside paragraphs, including list-item and blockquote paragraphs. Hard breaks (trailing two spaces or `\`) stay. | `one\ntwo` → `one two` |
| `fixed-width` | `false` | An integer, or `true` (meaning 120), first unwraps and then re-wraps prose to that display width, counting container prefixes. It implies `unwrap`. `false` means no re-wrapping. | `fixed-width: 80` |
| `indentation` | `4` | `2`, `4`, or `8` re-indents nested lists to N spaces per level, using CommonMark-valid marker padding where an 8-space step needs it so the list tree is the same. `preserve` leaves list indentation as authored. | 2-space nesting → 4-space |
| `ordered-lists` | `true` | Leaves a list alone when every number equals the first (`1,1,1`, `3,3,3`) or the numbers count up by one from the first (`1,2,3`, `3,4,5`). Otherwise renumbers sequentially from the first number. Render-neutral, because CommonMark uses only the start number. | `1,3,8` → `1,2,3`; `1,1,1` unchanged |
| `list-padding` | `true` | Ensures exactly one blank line before and after each list block (never more), unless the neighbor is a directive line (invariant 1) or the list is the first or last block in its container. | `Intro:\n- a` → `Intro:\n\n- a` |
| `list-interior-padding` | `preserve` | `preserve` leaves the blank lines inside list items as authored. `none` removes blank lines between an item and its children. `only-prose` puts one blank line before an item's paragraph children but none before a nested list. `prose-and-child-elements` puts one blank line before every child block. | see below |
| `default-language` | `"text"` | Adds this info string to a fence that has none. `""` adds nothing. | ```` ``` ```` → ```` ```text ```` |

`list-interior-padding` example, under `only-prose`:

```md
- item

    A paragraph child gets a blank line before it.
    - a nested list does not
```

A blank line inside a list item makes the list _loose_ in CommonMark, so it
renders with paragraph spacing. That is the point of the setting, and why the
default is `preserve`. The fidelity guard ignores tight/loose state for that
reason.

**With every rule at its default**, cleanup:
- repairs frontmatter non-destructively
- leaves directives untouched
- keeps every hard wrap
- fixes misnumbered ordered lists
- makes list indentation 4 spaces
- pads tables
- pads lists with one blank line before and after
- labels unlabeled fences `text`

**With every body rule off**, only the frontmatter is repaired: `unwrap: false`,
`fixed-width: false`, `ordered-lists: false`, `list-padding: false`,
`table-layout: false`, `default-language: ""`, and
`list-interior-padding: preserve`, and `indentation: preserve`.

### Resolution and precedence

The value of each rule is resolved independently, highest first:

1. An explicit CLI flag for that rule, for that run (e.g. today's `--indent`
   and `--fixed-width`).
2. The document's own `style.markdown.<key>`.
3. DMLS workspace config, for DMLS runs only. The current `formatting.cleanup`
   and `formatting.fixed_width` are replaced by a `formatting.markdown` object
   with the same keys as `style.markdown`.
4. The baseline schema default.

If the frontmatter is unrepairable, or `style.markdown` fails validation,
cleanup does not guess. It reports the schema diagnostic and formats the body
with levels 1, 3, and 4 only.

### Passes not covered by a rule

Cleanup has passes beyond the rules above: blockquote spacing fixes, bracket
unescaping, emphasis placeholder restoration, the `PREFER_ITALICS` environment
variable, and removal of the blank line after frontmatter. Each pass that
remains must be one of two things:

- **artifact repair:** it only undoes something the parse and serialize round
  trip added, such as escape and emphasis restoration
- **mapped to a rule above**

Anything else is removed. The implementer records each pass and which of the
three outcomes it got in the implementation log.

### Frontmatter repair moves into the library

`frontmatter: true` has to mean the same thing in DMLS as in the CLI. The repair
orchestration in `cli/src/commands/clean/frontmatter_repair.rs` moves into the
`darkmatter` library behind one entry point that both consumers call. The CLI
keeps only argument handling and output. DMLS gets frontmatter repair as part
of formatting.

## Decision: Guard on Structure, Fail Closed

**Choice:** compare a normalized structural fingerprint of the input and the
output, and fall back to the whole unchanged document when they differ.

**Fingerprint:** the pulldown-cmark block tree with:

- container kinds and their nesting (list with ordered flag and start, item,
  blockquote, heading level, table, HTML block, thematic break, footnote
  definition)
- code-block content bytes (info string excluded, since `default-language`
  may add one)
- inline text with whitespace runs collapsed and soft breaks treated as spaces
  (so `unwrap` and `fixed-width` pass)
- table cell text with surrounding whitespace trimmed (so `table-layout`
  passes)
- **ignoring** tight/loose list state, list-marker characters, and ordered
  item numbers after the first
- plus the ordered sequence of directive lines from the directive recognizer,
  each with its raw text and container path

**Why fail closed, for the whole document:** cleanup is a convenience, and
corruption is not an acceptable trade for it. Falling back block by block would
mean splicing original and cleaned text, which creates the same seam bugs this
fix removes. A document that comes back uncleaned with a diagnostic is a clear,
correct result. Each fallback the corpus check finds is a bug to fix, not a
state to tolerate.

**Parse budget:** the cleanup parse-count tests (`tests/parse_count.rs`)
currently assert exactly one parse per cleanup entry point. The guard adds one
parse of the output; the input's events already exist. The budget becomes
**two**, and the guard's parse must go through `cleanup_parser` so the count
stays measurable.

## Out of Scope

- **Refactoring the CLI flags.** That is a future change. Existing flags keep
  working, and a flag that maps to a rule overrides the document for that run.
  `--ignore-incidental-newlines` stays as it is. It becomes redundant under the
  new `unwrap: false` default but is harmless.
- **Heading structure** (no skipped levels). It is deferred and is not added
  to the schema until it is built, so no key exists that silently does
  nothing.
- Prettier, or any other third-party formatter, mangling Darkmatter files. That
  is configured on the editor side. This spec covers only what Darkmatter
  itself writes.

## Acceptance Criteria

1. **Reproductions.** E1, D1, C1, A1, B1, and F1 are L1 tests in
   `lib/src/markdown/cleanup/tests/`. With all rules at their defaults, each
   output equals its input, except D1, which gains only `text` on its fence and
   stays at column 8. E1, C1, and D1 also keep their structure under
   each non-default `indentation` value (`2`, `8`, `preserve`), `unwrap: true`, `fixed-width: 80`, and each
   `list-interior-padding` value.
2. A1 and B1, after cleanup, give the same `md compose` output as their
   inputs, with `when` both true and false.
3. **Directive fixed points.** For every rule, there is a test with a directive
   line following each of: a blockquote paragraph, a list-item paragraph
   (ordered and unordered, nested two levels), and a table. Each test shows the
   directive line unchanged and no blank line inserted next to it.
4. **One test per rule.** Each `style.markdown` key has a test of its default
   and of each non-default value, using the examples in the rules table. That
   includes `fixed-width: true` resolving to 120, `fixed-width` implying
   `unwrap`, `default-language: ""` adding nothing, and `ordered-lists`
   leaving `1,1,1`, `3,3,3`, and `3,4,5` alone while fixing `1,3,8` and
   `2,2,5`.
5. **Precedence.** Tests show that a CLI flag beats the document, the
   document beats DMLS config, and DMLS config beats the schema default. They
   also show that an invalid `style.markdown` falls back as described in
   [Resolution and precedence](#resolution-and-precedence).
6. **Fidelity guard.** `clean.fidelity-fallback` has tests that use an
   artificially broken pass (a test-only hook). They show that `md clean` writes
   nothing under `--save` and reports the diagnostic in `--json`, and that DMLS
   `formatting` returns an empty edit list and sends one warning.
7. **Frontmatter parity.** Frontmatter repair is a library entry point. The
   same broken-frontmatter fixture is repaired identically by `md clean` and by
   DMLS `formatting`. With `frontmatter: false`, both leave it byte-for-byte.
8. **Idempotence.** `clean(clean(x)) == clean(x)` for every fixture in criteria
   1–4.
9. **Real-world fixture.** `prompts/commit.md` at blob `464e2d90e` is checked
   in as a cleanup fixture. With defaults, its output composes without error,
   passes the guard, and keeps every hard wrap.
10. **Corpus check.** Run cleanup with defaults (without saving) over every
    tracked `*.md` file in the repository. The implementation log records how
    many guard fallbacks occurred. Each one is either fixed in this change or
    written up as a new `_unscheduled` fix with its minimal reproduction. The
    run is not added as a CI gate.
11. The parse-count tests assert a budget of two, and still pass.
12. **Docs** are updated in the same change:
    - a `docs/` topic page for `style.markdown` covering each key, a compact
      example per key, the precedence order (as a Mermaid diagram), and the
      invariants
    - [`docs/cli/clean.md`](../../docs/cli/clean.md): the invariants, the
      fallback diagnostic, and that the document's `style.markdown` drives
      cleanup unless a flag overrides it
    - [`docs/lsp/features.md`](../../docs/lsp/features.md) and the DMLS config
      docs: `formatting.markdown`, frontmatter repair during formatting, and
      that formatting may decline with a warning
    - the `formatting.rs` module docs describe the guarantee that now actually
      holds
    - the `darkmatter` skill (`.claude/skills/darkmatter/`) where it describes
      cleanup
