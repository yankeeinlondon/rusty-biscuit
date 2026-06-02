# Phase 1 Findings: Confirm Current Failure Surface

This document records the Phase 1 investigation for the `::shell` /
`::shell-block` indentation-preservation fix. No production source files were
changed in this phase; reproduction was performed with throwaway tests that
were removed afterward.

## Reproduction (through the real compose pipeline)

### `::shell` — indented under a list item (BUG CONFIRMED)

Input:

```md
- intro

    ::shell printf 'darkmatter\ndarkmatter-cli\ndmls\n'

- next
```

Actual output (directive's 4-space indent is discarded; output lands at
column 1):

```md
- intro

darkmatter
darkmatter-cli
dmls

- next
```

The lines beginning with `-` would be absorbed as siblings of the outer list,
exactly as described in `spec.md`.

### `::shell` — at column 1 (BASELINE, correct)

Input `intro\n\n::shell printf '...'\n\nnext\n` produces the three output lines
at column 1, unchanged. This is the no-indent baseline that must remain
byte-for-byte unchanged after the fix.

### `::shell-block` — indented opener (HARD PARSE FAILURE — deeper than `::shell`)

An indented `::shell-block` opener does **not** merely splice at column 1 — it
fails to parse at all:

```
Parse { line: 3, message: "Expected identifier, found ':'",
        excerpt: "    ::shell-block" }
```

Root cause: `shell_blocks/parser.rs:12-14` reads the opener line from
`pair.span.start`, which begins at the line start and therefore includes the
leading whitespace. `parse_opener_params` (`shell_blocks/parser.rs:29-30`) then
does `opener.strip_prefix("::shell-block")`, which fails because of the leading
spaces; `unwrap_or(opener)` keeps the full `    ::shell-block`, and the
key=value parameter parser is handed `::shell-block`, hitting `:` →
`Expected identifier, found ':'`.

Implication for Phase 4: the shell-block fix must trim the opener's leading
whitespace **before** `strip_prefix`/param parsing, in addition to applying the
captured indent at the splice boundary. The indent itself is already available
on `BlockPair::opening_text` (`block_pairs.rs:102/115`), which is captured as
`content[line_start..text_line_end]` and therefore retains the leading
whitespace.

A separate, related interaction: when both the opener **and** the body lines are
indented, body parsing also fails (`split_logical_commands` on indented command
lines). Phase 4 must decide how the body indent is stripped before command
parsing. This is noted here so it is not a surprise during Phase 4.

### `::shell-block` — at column 1 (BASELINE, correct)

`intro\n\n::shell-block\nprintf '...'\n::end-block\n\nnext\n` produces the
three output lines at column 1 via `render_block_output`, unchanged. This is
the no-indent baseline that must remain byte-for-byte unchanged after the fix.

## Code-level confirmation of the splice path

### `::shell`

- `shell_expansion/parser.rs:50` — `let trimmed = line.trim();` then
  `trimmed.strip_prefix("::shell ")`. Indented directives **are** detected, but
  the leading whitespace is discarded — `ShellDirective` has no `indent` field.
- `shell_expansion/parser.rs:104` — `span: line_start..line_with_newline_end`.
  The span covers the **entire line, including its leading whitespace**, so the
  replacement string overwrites the indent.
- `compose/mod.rs:1243` — `replacements.push((directive.span.clone(),
  execution.combined_output()))`. The raw multi-line output is spliced with no
  indent prefix.
- `shell_expansion/mod.rs:601-609` — `apply_replacements_in_reverse` is a
  pure byte-range splice; it must stay span-only, so the indent must be applied
  to the replacement string before it reaches this function.

### `::shell-block`

- `block_pairs.rs:104/141` — `block_start = line_start`; `span:
  block_start..span_end`. The span begins at column 1, including the opener's
  indent.
- `shell_blocks/mod.rs:176-177` — `render::render_block_output(&results)` is
  spliced raw at `pair.span`, no indent prefix.
- `shell_blocks/parser.rs:12-30` — opener param parsing fails on an indented
  opener (see hard-failure section above).

## Existing tests: expected to change vs. must remain unchanged

### Must remain byte-for-byte unchanged (root-level / column-1 baseline)

Every existing shell test uses column-1 directives; none use an indented
directive. They establish the no-indent baseline and must continue to pass
unchanged:

- `shell_expansion/mod.rs` integration tests (52 `#[test]`), e.g.
  `pipeline_replaces_shell_directive_with_output`,
  `pipeline_report_counts_are_correct`,
  `pipeline_interpolation_feeds_into_shell_expansion`.
- `shell_expansion/parser.rs` tests (39 `#[test]`), including the span
  assertions at lines ~351-385 (`span.start == 0`, `span.end` includes the
  newline). Phase 3 keeps the span unchanged (indent is applied to the
  replacement string, not by altering the span), so these stay valid.
- `shell_blocks/mod.rs` tests (20 `#[test]`): `single_command_block`,
  `multiple_commands` (`"hello\n\nworld\n"`), `multiple_sibling_blocks`,
  `all_commands_empty_output` (asserts `""`), `mixed_empty_and_non_empty_outputs`.
- `shell_blocks/render.rs` tests (8 `#[test]`): all `render_block_output`
  assertions. `render_block_output` is indent-agnostic; Phase 4 applies indent
  at the splice boundary, so these remain unchanged.

### Expected to change / be added

- New `ShellDirective` `indent: String` field (Phase 3) forces every
  struct-literal construction site to populate it. Construction sites found:
  - `shell_expansion/parser.rs:100` (the real parser — computes the indent)
  - `shell_expansion/mod.rs:519`, `:557` (effective/derived directives)
  - `shell_expansion/discovery.rs:487`
  - `shell_blocks/mod.rs:107` (synthesized per-command directive — use the
    block opener indent or `String::new()` as decided in Phase 4)
  - `shell_expansion/executor.rs` test helpers and tests (`:766`, `:913`,
    `:999`, `:1048`, `:1068`, `:1091`, `:1111`, `:1140`) — use `String::new()`
  - `frontmatter_shell_expansion.rs:1060` — **out of scope**, use
    `String::new()` (frontmatter expansion is scalar-only per `spec.md` §6)
- New indented-directive tests added in Phases 3-5 (4-space list, block-quote
  marker, tab, blank-line, root-level no-indent) for both `::shell` and
  `::shell-block`.

## Phase 2 reuse target

`toc_linking/render.rs:12-29` `indent_text(text, indent, inferred_indent)` is
the existing byte-preserving helper to extract into a shared compose utility in
Phase 2. It prefixes every line, returns text unchanged when indent or text is
empty, and is exercised by the `indented_output_*` / `no_indent_at_root` tests
in `toc_linking/render.rs` (lines ~181-247) — the regression target for the
Phase 2 extraction.
