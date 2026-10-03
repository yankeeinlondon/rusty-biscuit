---
spec: /Volumes/coding/wt/rusty-biscuit/fix-magic-globs/biscuit-terminal/features/2026-10-02-inline-prose/spec.md
plan: biscuit-terminal/features/2026-10-02-inline-prose/plan.md
implemented_by: claude/opus
started_phase: 1
source_files_during_phase_2:
    - renderable/Cargo.toml
    - renderable/src/markdown.rs
    - renderable/src/tree/attrs.rs
    - renderable/src/tree/mod.rs
    - renderable/src/tree/render/browser.rs
    - renderable/src/tree/render/markdown.rs
    - renderable/src/tree/validate.rs
    - Cargo.lock
docs_updated_during_phase_2:
    - renderable/docs/tree-rendering.md
    - docs/dependencies.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/renderable/tree.md
    - .claude/skills/renderable/markdown.md
source_files_during_phase_3:
    - biscuit-terminal/lib/src/components/prose/blocks.rs
    - biscuit-terminal/lib/src/components/prose/inline_prose.rs
    - biscuit-terminal/lib/src/components/prose/markdown.rs
    - biscuit-terminal/lib/src/components/prose/tokens.rs
    - biscuit-terminal/lib/src/components/prose/prose.rs
    - biscuit-terminal/lib/src/components/prose/tree.rs
    - biscuit-terminal/lib/src/components/prose/mod.rs
    - biscuit-terminal/lib/src/components/prose/parity.rs
    - biscuit-terminal/lib/src/render_tree/render.rs
    - biscuit-terminal/lib/src/render_tree/projection.rs
    - biscuit-terminal/lib/src/components/table/table.rs
    - biscuit-terminal/lib/src/components/metrics_tree.rs
    - biscuit-terminal/lib/src/components/block_quote.rs
    - biscuit-terminal/lib/src/components/list.rs
    - biscuit-terminal/lib/src/prelude.rs
    - biscuit-terminal/lib/tests/l1/prose_grammar.rs
    - biscuit-terminal/lib/tests/l1/main.rs
    - biscuit-terminal/lib/tests/l1/prelude_exports.rs
    - biscuit-terminal/lib/tests/l1/prose_cells_parity.rs
    - "biscuit-terminal/lib/tests/l1/snapshots/l1__layout_matrix__MetricsTree__*.snap (38 files)"
    - biscuit-terminal/cli/src/commands/prose.rs
    - biscuit-terminal/cli/tests/l1/integration_test.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/biscuit-terminal/SKILL.md
source_files_during_phase_4:
    - biscuit-terminal/lib/src/components/prose/tree.rs
    - biscuit-terminal/lib/src/components/prose/render.rs
    - biscuit-terminal/lib/src/render_tree/projection.rs
    - biscuit-terminal/lib/src/render_tree/render.rs
    - biscuit-terminal/lib/src/components/list.rs
    - biscuit-terminal/lib/src/components/block_quote.rs
    - biscuit-terminal/lib/src/components/two_column.rs
    - biscuit-terminal/lib/src/components/status_block.rs
    - biscuit-terminal/lib/src/components/inline_content.rs
    - biscuit-terminal/lib/src/components/compose.rs
    - biscuit-terminal/lib/src/components/table/cell.rs
    - biscuit-terminal/lib/src/components/table/column.rs
    - biscuit-terminal/lib/src/components/table/table.rs
    - biscuit-terminal/lib/tests/l1/main.rs
    - biscuit-terminal/lib/tests/l1/prose_containers.rs
    - biscuit-terminal/lib/tests/l1/prose_cells_parity.rs
    - biscuit-terminal/lib/tests/l1/status_block_parity.rs
    - biscuit-terminal/lib/tests/l1/unordered_list_parity.rs
    - biscuit-terminal/lib/tests/l1/ordered_list_parity.rs
    - biscuit-terminal/lib/tests/l1/render_tree_component_parity.rs
    - biscuit-terminal/lib/tests/l1/inline_content_matrix.rs
    - biscuit-terminal/lib/tests/inline_content_matrix_support/mod.rs
    - biscuit-terminal/cli/src/commands/table.rs
    - biscuit-terminal/cli/tests/level2/prose_cells.rs
    - renderable/src/tree/attrs.rs
    - biscuit-icon/cli/src/commands.rs
    - biscuit-icon/cli/src/sets_table.rs
    - darkmatter/cli/src/commands/schema/about.rs
    - claudine/cli/src/commands/steer/render.rs
docs_updated_during_phase_4:
    - biscuit-terminal/docs/components/table.md
    - biscuit-terminal/docs/components/inline_content.md
    - biscuit-terminal/docs/components/list.md
    - biscuit-terminal/docs/components/block_quote.md
    - biscuit-terminal/docs/components/two_column.md
    - biscuit-terminal/lib/src/components/table/README.md
    - biscuit-terminal/README.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/biscuit-terminal/SKILL.md
    - .claude/skills/biscuit-terminal/components.md
source_files_during_phase_5:
    - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/paths.rs
    - darkmatter/lib/src/markdown/compose/expression/catalog/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/catalog/parser.rs
    - darkmatter/docs/schemas/expression-functions.yaml
    - darkmatter/lib/tests/l1/code_link.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/error_snapshots/snapshots/l1__error_snapshots__link__unrecognized_format.snap
    - darkmatter/dmls/tests/l1/lsp_session.rs
    - darkmatter/dmls/tests/fixtures/mapping_only_corpus/_reviews__review-spec-inline.md
    - claudine/cli/tests/l1/context_command.rs
    - claudine/cli/tests/fixtures/nested_span_regression/review-spec-inline.md
    - prompts/plan.md
    - prompts/_reviews/review-spec-inline.md
docs_updated_during_phase_5:
    - darkmatter/docs/topics/darkmatter-expressions.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
    - .claude/skills/darkmatter/compose.md
source_files_during_phase_6:
    - biscuit-terminal/cli/src/commands/prose.rs
    - biscuit-terminal/cli/src/commands/section.rs
    - biscuit-terminal/cli/src/commands/list.rs
    - biscuit-terminal/cli/src/commands/shared.rs
    - biscuit-terminal/cli/tests/l1/integration_test.rs
    - biscuit-terminal/lib/src/components/prose/render.rs
    - biscuit-terminal/lib/src/components/prose/tree.rs
    - biscuit-terminal/lib/tests/l1/prose_grammar.rs
docs_updated_during_phase_6:
    - biscuit-terminal/docs/components/prose.md
    - biscuit-terminal/docs/components/index.md
    - biscuit-terminal/docs/components/section.md
    - biscuit-terminal/docs/components/table.md
    - biscuit-terminal/lib/src/components/table/README.md
    - biscuit-terminal/README.md
    - renderable/docs/components.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
    - .claude/skills/biscuit-terminal/styling.md
    - .claude/skills/biscuit-terminal/components.md
    - .claude/skills/biscuit-terminal/cli.md
packages:
    - renderable
    - biscuit-terminal
    - biscuit-terminal-cli
    - biscuit-icon-cli
    - darkmatter-cli
    - claudine-cli
    - darkmatter
    - dmls
---

# Implementation Log for 2026-10-02-inline-prose (8 phases)

## Phase 1

Phase 1 is rulings, spikes, and a baseline. It changes no source code.

### Rulings (R1–R10)

The plan records rulings R1–R10 as yolo defaults the author may overturn at
review. Phase 1 adopts them as written; the spikes below add these
refinements:

- **R1 — Sentinel safety.** Adopted as written. Facts for Phase 3: the
  current parser (`components/prose/markdown.rs`) already lifts two kinds
  of content behind C0 control-character sentinels, not private-use ones:
  `HREF_PLACEHOLDER_MARK = '\u{0001}'` (`\u{0001}HREF<n>\u{0001}`, link
  targets) and `CODE_BLOCK_PLACEHOLDER_MARK = '\u{0002}'`
  (`\u{0002}CODE<n>\u{0002}`, fenced blocks). There is no pre-pass today:
  a malformed placeholder is treated as literal text
  (`tokens.rs::take_code_placeholder`), but a well-formed one with an in-range
  index typed by the user *is* resolved, so input can impersonate a reference now. Phase 3
  must add the R1 pre-pass for every sentinel the parser uses (including
  the new code-span one), so there is one escape mechanism, not one per
  lift.
- **R2 — `\` before a blank line.** Adopted as written: literal backslash;
  `Soft` → one soft break per run; `Hard` → one hard break per `\n` of the
  run (`a\n\nb` → two hard breaks).
- **R3 — Layout transfer.** Adopted. S2b (below) shows every non-inline
  kind accepts `Layout`, so `ListItem`, `BlockQuote`, `TableCell`, and
  `Paragraph` all qualify and no new neutral block kind is needed. A nested
  `Root` is rejected by the validator ("Root node may appear only as the
  top-level node"), which confirms the "never a nested `Root`" clause.
- **R4 — `StyledProse` → `StyledInlineProse`.** Adopted as written.
- **R5 — Inline vs block classification.** Adopted as written; the S1
  ledger applies it.
- **R6 — Fence helper lives in `renderable`.** Adopted as written.
- **R7 — No new `bt prose` flags.** Adopted as written.
- **R8 — Perf.** Adopted. `biscuit-terminal/lib/tests/l1/perf_gate.rs`
  exists and is part of the baseline below.
- **R9 — Known-red ledger.** Adopted; see "Known-Red Ledger".
- **R10 — Spec `packages:`.** Adopted; extra consumers found by S1 are
  recorded under "Scope Record", not edited into the spec.

### S2 — Terminal Renderer Feasibility

Read-only, macOS.

#### S2a — Appearance restore and the backtick-fallback signal

- **Restore is supported; no new style stack is needed.**
  `render_tree/render.rs` passes the inherited text appearance down as the
  `effective: &Style` argument of `render_inline` / `render_inline_node`.
  Every styled inline kind (`Emphasis`, `Strong`, `Delete`, `Span` with a
  style, `InlineCode`, `Extended` mark/dim, class-driven styling via
  `apply_classes`) computes `child_effective`, opens with
  `style::text_appearance_sgr(&child_effective, term)`, and closes with
  `style::appearance_close(&open, effective, term)`, which emits a reset
  and then re-applies the *parent* appearance. The recursion is the stack.
  The `InlineCode` arm (render.rs, `NodeKind::InlineCode { value }`)
  already follows this pattern: themed background/foreground when the
  context carries `inline_code_background`, otherwise `dim`.
- **The "no styling emitted" signal needs a comparison, not `open.is_empty()`.**
  `text_appearance_sgr` returns the *full* effective appearance, so `open`
  is non-empty whenever an ancestor is styled, even if the code span itself
  adds nothing visible. Example: inside an underlined run on a
  `ColorDepth::None` terminal with underline support, `dim` is suppressed
  but `open` still carries the inherited underline. The Phase 3 fallback
  should therefore keep the backticks when
  `text_appearance_sgr(&child_effective, term) == text_appearance_sgr(effective, term)`
  (the code span changed nothing). On a plain terminal both sides are
  empty, which is the common case.
- **OSC 8 does not count.** Hyperlink escapes are added in the `Link` arm
  around the already-styled label; they never pass through
  `text_appearance_sgr`, so a code span inside a link is judged on its SGR
  alone, as the spec requires.

#### S2b — Which `NodeKind`s accept `Layout`

`renderable/src/tree/validate.rs` rejects a `Layout` only on inline kinds
(`is_inline_kind`); every other kind accepts it. The terminal renderer
applies it in `render_with_layout` for any non-inline node and drops it
on inline nodes.

| Accepts `Layout` | Rejects `Layout` (inline) |
| --- | --- |
| `Root`, `Heading`, `Section`, `Paragraph`, `BlockQuote`, `List`, `ListItem`, `Code`, `ThematicBreak`, `Table`, `TableRow`, `TableCell`, `FootnoteDefinition`, `Disclosure`, `Html`, `Unsupported` | `Text`, `Emphasis`, `Strong`, `Delete`, `Span`, `Extended`, `InlineCode`, `Link`, `Image`, `FootnoteReference`, `SoftBreak`, `HardBreak` |

Other placement constraints that matter for R3:

- `Root` is valid only at the top level, so an embedded `Prose` must give
  up its `Root` and hand its children (and its `Layout`) to the container.
- A `sequence-join` policy is `Root`-only.
- `TextLayoutHints` (pad/truncate) are allowed only on `Link`, `Image`,
  and `ListItem`; they are not a substitute for `Layout`.

### Scope Record

(S1 ledger — see below.)

### Known-Red Ledger

(Filled from the baseline.)

### Snapshot Review

(Ledger of snapshot directories likely to move — see below.)

### Drift Notes

None found in Phase 1 (read-only).

### Departures

None in Phase 1.

## Phase 2

Phase 2 builds the `renderable` pieces later phases depend on. Only the
`renderable` package changed in code; validation ran on macOS.

### What Changed

- **`BlockElement`** (`renderable/src/tree/attrs.rs`, exported from
  `renderable::tree`): `P` (default), `Div`, `Section`, `Article`, `Aside`,
  `Header`, `Footer`, with `ALL`, `is_default()`, and `block_tag()`.
  `BrowserAttrs` gains `block_element: BlockElement` (not `Option`), with
  `#[serde(default, skip_serializing_if = "BlockElement::is_default")]`.
  `P` means "absent", so there is one way to spell the default rather than
  both `None` and `Some(P)`. `is_empty()` counts it.
- **Validator** (`validate.rs`): a non-default `block_element` on any kind
  except `Paragraph` is an error ("the block-element browser attribute is
  permitted only on a Paragraph node, found on {kind}").
- **Browser** (`render/browser.rs`): one shared `paragraph_tag(&NodeAttrs)`
  picks the element for both the fragment writer (`render_kind`) and the
  streaming writer (`write_kind`). The progress-widget paragraph branch runs
  first and does not change. The attribute is never written out as an HTML
  attribute.
- **Fence helper** (R6): `renderable::markdown::code_span(value: &str) -> String`
  in `renderable/src/markdown.rs`. It turns line endings (`\n`, `\r\n`, lone
  `\r`) into spaces before fencing, returns `""` for an empty value, and uses
  the CommonMark fence and padding rule. Its doc comment states both limits.
- **Markdown renderer** (`render/markdown.rs`): `InlineCode` →
  `code_span` (outside a table cell: the raw value; inside one, `|` → `\|`
  first, then `code_span`). `HardBreak` → `\` + newline outside a table cell,
  and still `<br>` inside one. Both dialects share the code path.
- **Dev-dependency**: `pulldown-cmark = { version = "0.13", default-features = false }`
  for `renderable` tests. It was already locked through darkmatter, so
  `Cargo.lock` gains one edge and no package (noted in `docs/dependencies.md`).

### Requirement-to-Test Mapping

All tests are in-crate `#[cfg(test)]` unit tests in `renderable`, which makes
them L1. No name segment carries a tier marker, and `just test` runs them.

| Requirement | Test(s) |
| --- | --- |
| Every `BlockElement` renders its own tag; fragment and streaming paths agree; two tagged paragraphs are two siblings (AC 8, 31) | `tree::render::browser::tests::paragraph_block_element_matches_in_fragment_and_streaming_paths` |
| Other browser attrs still land on the chosen element | `…browser::tests::paragraph_block_element_keeps_other_browser_attributes` |
| Legacy serialized tree (no key) keeps `<p>`; explicit `"div"` honored; unknown name and wrong type are rejected at deserialization (AC 31) | `…browser::tests::paragraph_without_block_element_field_keeps_p` |
| Validator rejects the attribute on every one of the 27 non-`Paragraph` kinds (AC 8, 31) | `tree::validate::tests::block_element_on_non_paragraph_is_an_error` |
| Every element is valid on `Paragraph` | `tree::validate::tests::block_element_on_paragraph_is_valid` |
| Serde round-trip; default is sparse and serializes to `{}` | `tree::attrs::tests::block_element_round_trips_and_default_is_sparse` |
| Markdown ignores it in both dialects, with no diagnostics | `tree::render::markdown::tests::paragraph_block_element_does_not_change_markdown` |
| Fence helper: spec table, long runs, edge spaces, all-space, line endings, empty, literal content | `markdown::tests::code_span_*` (7 tests) |
| Fence output parses back through a real CommonMark parser as exactly one code span | `markdown::tests::code_span_parses_back_with_commonmark`, proptest `code_span_round_trips_any_value` |
| Directly constructed `InlineCode` in both dialects, standalone, in a paragraph, and in a table cell (pipes escaped before fencing, multiline, empty, backslash) (AC 19, 30) | `tree::render::markdown::tests::inline_code_uses_a_safe_fence_in_both_dialects` |
| Hard break is `\`+newline outside tables and `<br>` inside, in both dialects, never trailing spaces (AC 30) | `tree::render::markdown::tests::hard_break_uses_backslash_outside_tables_and_br_inside`; `soft_and_hard_breaks` updated |

The terminal side of AC 8 (a tagged paragraph is unchanged on the terminal)
is not asserted here. The terminal renderer in `biscuit-terminal` never reads
`NodeAttrs::browser`, which a grep confirmed. Phase 3 covers it through
`Prose::with_tag`.

### Validation

- `renderable`: `just test` gives 564/564 passed. `just lint` is clean.
  `cargo test --doc` gives 99 passed and 2 ignored (both ignores already
  existed).
- Downstream, run to fill in the Known-Red Ledger:
  - `biscuit-terminal`: `just test` gives 3391 passed.
  - `claudine`: `just test` gives 8104 passed.
  - `messenger`: `just test` gives 696 passed.
  - `darkmatter`: `just test --no-fail-fast` gives 8883 passed and 2 failed.
    Both failures existed before this phase (see the ledger).
- OS risk: the change is pure string and tree logic with no `cfg` and no
  filesystem access, so no `just cross-check` was run. CI covers Linux; other
  OSes add nothing here.

### Known-Red Ledger (Phase 2)

- **Nothing went red because of Phase 2.** The plan expected darkmatter
  snapshots to move with the hard-break and fence changes. They did not: no
  darkmatter, claudine, or messenger test asserts on the
  `renderable` Markdown hard-break or inline-code spelling today. The Phase 7
  snapshot review should still expect moves once `Prose` emits these nodes.
- **These failures existed before Phase 2 and are not caused by this
  feature:** `darkmatter::l1
  current_root_documentation_contract::every_required_page_explains_the_binding_time_model`
  and `darkmatter::l1
  current_root_migration_guard::the_removed_nesting_appears_only_where_the_allowlist_expects`.
  Both check text in `.claude/skills/claudine/SKILL.md` and `cli-commands.md`,
  which this phase did not touch. They were most likely caused by the branch's
  latest commit (`92c562dcc docs(skills): …`). They must be green before
  merge (R9), but they belong to that skills change, not to this plan.

### Drift Notes

- `.claude/skills/renderable/markdown.md` describes a
  `render_markdown_with_style(&self, Option<Stylesheet>)` trait method. The
  real `MarkdownRenderable` trait (`renderable/src/markdown.rs`) has
  `render_markdown_plus(&self)` instead. This drift existed before Phase 2
  and is outside its scope, so it is left as is. Phase 6 (skills) should fix
  it.
- The old Markdown `HardBreak` comment ("two trailing spaces") and the
  `InlineCode` comment were updated in the same change.

### Departures

- `BrowserAttrs::block_element` is a plain `BlockElement`, not an
  `Option<BlockElement>`. On the wire it stays optional (omitted when it is
  the default), as the plan requires. In Rust it has a single default state.
- The helper is named `renderable::markdown::code_span`. The plan left the
  signature to Phase 2.
- A `renderable` dev-dependency on `pulldown-cmark` was added. The plan
  expected no crate changes. This adds no new package, only one lockfile
  edge.

### Phase 1 Gaps Noticed

The Phase 1 log sections "Scope Record" (the S1 call-site ledger) and
"Known-Red Ledger" (the baseline) still hold placeholders. The plan's
Phase 1 checkboxes are also unticked. Phase 2 did not need either. Phases 4,
7, and 8 depend on the S1 ledger, so it must be produced before then.

## Phase 3

Phase 3 builds the shared grammar and the two components in
`biscuit-terminal`. Validation ran on macOS.

### What Changed

- **Pipeline** (`components/prose/`): `markdown::lift_fences` normalizes CRLF
  and lone CR to LF, lifts fenced blocks, and backslash-escapes every literal
  sentinel character outside them. `blocks::split_blocks` (new module) splits
  the remaining text into paragraphs. `markdown::preprocess_inline` runs per
  paragraph (code spans, then links, bold, and italics), and
  `tokens::parse_render_nodes` builds the nodes. Both entry points live in
  `blocks.rs`: `parse_blocks` (`Prose`) and `parse_inline` (`InlineProse`).
- **R1 mechanism.** A literal `\u{0001}` or `\u{0002}` in input becomes
  `\` + that character, an ordinary escape pair that every phase already
  skips and the token parser resolves to the literal character. An
  unescaped sentinel is therefore always parser-issued. Inside a code span
  the inserted backslash is removed again, which is unambiguous because
  every sentinel has exactly one inserted backslash. In a link href it is
  unescaped too. `restore_hrefs` is now a single left-to-right pass, so a
  restored href value can no longer be re-read as a placeholder. The old
  sequential `replace` had that impersonation hole.
- **Lifted content** is one table, `Vec<Lifted>` (`Fence` | `Span`), with
  `\u{0002}<n>\u{0002}` placeholders. The old `CODE<n>` spelling and
  `take_code_placeholder` are gone.
- **Splitter.** It finds blank-line runs (two or more `\n` with only spaces
  or tabs between them) and fence placeholders. It reopens recognized style
  and `<a>` declarations in each paragraph and closes them at the end. Code
  spans (when the closer comes before the next blank line), quoted
  attributes, and `<code-block>…</code-block>` are opaque to it. It drops
  leading and trailing blank lines and whitespace-only paragraphs. An
  unquoted blank line inside `< … >` does not form a tag declaration.
- **Breaks** (`tokens.rs`). A run of newlines is one `SoftBreak` (spaces
  and tabs on both sides are discarded) or, in `Hard`, one `HardBreak` per
  newline. Unescaped `\` + newline is a `HardBreak`, except before a blank
  run, where the backslash stays literal (R2). `\\` + newline is a literal
  backslash plus the mode's break. Code-span values follow the CommonMark
  rules.
- **Fenced code in `InlineProse`** (fence placeholder or `<code-block>`)
  becomes `InlineCode`, with each newline turned into a space and the
  language hint dropped. An empty body adds no node. In `Prose`, a `Code`
  node that lands inside a paragraph (through a `<code-block>` tag) is split
  out into a sibling block.
- **Components.** `Prose` gains `line_breaks`, `tag: ProseTag`,
  `with_line_breaks`, and `with_tag`, and now derives `Default`.
  `InlineProse` is new (`inline_prose.rs`). `LineBreaks` lives in
  `blocks.rs`. All three are exported from `components::prose` and the
  prelude. `Prose::to_render_nodes` is removed.
  `Prose::render_tree` returns a `Root` of `Paragraph` and `Code` blocks with
  the layout on the root, and each paragraph's `BlockElement` comes from
  `ProseTag` (default kept sparse). `InlineProse::render_tree` returns a
  neutral `Span`.
- **Browser.** `Prose::render_html_fragment` with no layout concatenates the
  blocks as siblings: `<p>…</p><pre>…`, an empty string for empty input, and
  no `<span class="prose">`. With a layout it renders the tree, so the root
  `<div style>` carries the CSS. `InlineProse` concatenates its children
  with no wrapper. Both go through one helper, `tree::concat_html`.
- **Terminal inline code** (`render_tree/render.rs`). When
  `text_appearance_sgr(child) == text_appearance_sgr(parent)`, the code adds
  no styling, so it is emitted through `renderable::markdown::code_span` as a
  backtick fence. Otherwise it is styled and the parent appearance is
  restored afterward, as before.
- **Inside-out rewrite removed.** `whole_span_link`, `push_escaped_code`,
  `escape_code_spans`, the two tests the spec names, and the matching
  `markdown.rs` module-doc rows are gone. `escape_text`'s rustdoc now says
  not to escape text placed in a code span.
- **Lib-internal call site.** `MetricsTree` builds one line per metric with
  single `\n`, so its three `Prose` constructions now use `LineBreaks::Hard`.

### Requirement-to-Test Mapping

All of these are L1 tests. The new module `lib/tests/l1/prose_grammar.rs` is
declared in `tests/l1/main.rs`, and no path segment carries a tier marker.

| Requirement (AC) | Test(s) |
| --- | --- |
| `InlineProse` on Markdown, browser, and terminal (1) | `prose_grammar::inline_prose_renders_phrasing_on_every_target`, doc test on `InlineProse` |
| Single `\n` is a soft break (2) | `single_newline_is_a_soft_break_on_every_target` |
| `\n\n`, `\n\n\n`, and `\n  \n` are boundaries (3) | `blank_line_runs_are_paragraph_boundaries_on_every_target`, `blocks::tests::blank_line_runs_split_paragraphs` |
| `\` + newline is a hard break (4) | `backslash_newline_is_a_hard_break_on_every_target` |
| `Hard` mode, and blank lines still split (5) | `hard_mode_turns_single_newlines_into_hard_breaks`, `inline_prose_hard_mode_breaks_on_every_newline_of_a_blank_run` |
| Trailing spaces are not a break (6) | `trailing_spaces_are_not_a_hard_break`, `spaces_and_tabs_around_a_soft_break_are_discarded` |
| `InlineProse` blank lines (7) | `inline_prose_blank_lines_are_one_soft_break` |
| `ProseTag` in HTML only (8) | `prose_tag_sets_each_paragraph_element_in_html_only`, `prelude_exports::prelude_exports_prose_components` |
| `<p>` and `<pre><code>` are siblings (9) | `fenced_prose_renders_paragraph_and_pre_as_siblings`, `parity::browser_paragraphs_and_code_blocks_are_siblings` |
| Margin in terminal and HTML, once (10, lib half; CLI half below) | `left_margin_renders_in_terminal_and_html`; CLI `test_prose_html_margin_left_appears_once` |
| No `class="prose"` (11) | `no_prose_class_is_generated_and_user_content_is_untouched` |
| Code-span projection examples (14) | `code_spans_project_to_inline_code_everywhere_prose_grammar_applies` |
| Link example per target, OSC 8 order, no backtick (15) | `code_link_example_renders_per_target` (exact byte string) |
| `` `[desc](ref)` `` is literal code (16) | `code_span_holding_a_link_is_literal_code`, `markdown::tests::code_span_holding_a_link_stays_literal`, `parity::markdown_code_spans_use_a_safe_fence` |
| `` `a\_b` `` is literal (17, lib half) | `code_span_backslashes_are_literal_on_every_target`, `prose::tests::escaped_text_inside_a_code_span_keeps_its_backslashes` |
| Space handling (18) | `code_span_space_handling_follows_commonmark`, `markdown::tests::code_span_value_follows_commonmark_spacing` |
| `ColorDepth::None` backtick fallback (21) | `unstyled_terminal_keeps_a_backtick_fence`, `inherited_styling_without_new_code_styling_keeps_the_fence`, `parity::terminal_code_span_keeps_backticks_without_styling` |
| Empty input; one-`Span` `InlineProse` (25) | `empty_and_whitespace_input_yield_a_valid_single_node`, `multi_node_inline_prose_is_one_span_and_markdown_has_no_separators`, `inline_prose_keeps_spaces_around_a_label` |
| CRLF and lone CR; escaped and trailing backslashes; opaque code unaffected by mode (26) | `crlf_and_lone_cr_match_lf_structure` (LF twin per variant, compared as render trees), `escaped_backslash_before_newline_is_a_literal_backslash_and_the_mode_break`, `backslash_at_end_or_before_a_boundary_is_literal`, `opaque_code_is_unaffected_by_break_mode` |
| Style across paragraphs, fence inside a style, blank line in a fence, quoted attribute, spans never cross paragraphs (27) | `bracketed_style_spanning_paragraphs_reopens_in_each`, `fenced_block_inside_a_style_is_a_sibling_and_the_style_resumes`, `quoted_attribute_newlines_belong_to_the_attribute`, `code_spans_never_cross_paragraphs_and_unmatched_runs_stay_literal`, and the `blocks::tests::*` splitter table |
| Fence in `InlineProse` becomes inline code, directly (28; table-cell variant is Phase 4) | `inline_prose_fence_becomes_one_inline_code_value` |
| Restore after code; hyperlink kept; unstyled fallback in a directly built tree (29) | `nested_code_restores_the_enclosing_style`, `nested_code_preserves_a_surrounding_hyperlink`, `unstyled_fallback_applies_to_a_directly_built_tree` |
| R1: sentinel lookalikes stay user content | `sentinel_lookalike_input_stays_user_content`, `escaped_tags_and_delimiters_stay_user_content`, `markdown::tests::{literal_sentinels_are_escaped_outside_fences_only, escaped_sentinel_inside_code_span_is_restored, literal_href_placeholder_is_not_restored}` |
| R8: performance | `perf_gate.rs` passes, unchanged |

### Validation

- `biscuit-terminal`: `just test` gives 3447 passed and 57 skipped (lib
  and CLI). `just lint` is clean. Lib doc tests: 203 passed.
  `just test-browser`: 56 passed. `cargo check --all-targets --features
  terminal-tests,browser-tests,image` is clean, and no L2 test references
  `Prose`.
- OS risk: the change is pure string and tree logic, with no `cfg` and no
  filesystem access. Line endings are normalized in the parser, and
  `crlf_and_lone_cr_match_lf_structure` covers them on every host. No
  `just cross-check` was run; CI covers Linux.

### Snapshot Review

- 38 snapshots moved, all
  `lib/tests/l1/snapshots/l1__layout_matrix__MetricsTree__*__{browser,markdown}.snap`.
  They fall into one category, **hard break form**. `MetricsTree` now uses
  `LineBreaks::Hard`, so each row ends in `<br>` inside the `<p>` instead
  of a raw newline (which browsers collapsed into a space, so the new output
  is more correct), and Markdown rows end in `\` plus a newline. A script
  rebuilt each expected `.snap.new` from its old `.snap` by applying only
  that substitution and compared the two. All 38 matched exactly, and no
  other byte changed. They were then accepted.
- Hand-written expectations that changed, each one reviewed:
  - `prose::tests::code_block_inside_span_restores_enclosing_style` and
    `parity::terminal_styled_fenced_code_splits_around_block`: the stray
    newlines inside the red runs before and after the fence are gone,
    because the paragraph edges are now trimmed.
  - `prose::tests::description_with_underscored_file_names_keeps_every_character`:
    the single `\n` is now a soft break. The code span sits inside `<dim>`,
    so its dim adds nothing and it keeps a backtick fence, as the spec
    requires.
  - `prose::tests::escaped_author_text_renders_verbatim`: the code span
    moved out of this test into
    `escaped_text_inside_a_code_span_keeps_its_backslashes`, which locks the
    new literal-backslash rule.
  - `block_quote::tests::test_multiline_prose_in_block_quote` now uses
    `LineBreaks::Hard`, because a single `\n` is soft in a block.
  - CLI `test_prose_html_*` (renamed to `…_margin_left_appears_once` and
    `…_without_margin_renders_a_paragraph`): the output is `Prose`'s own
    `<div style><p>…</p></div>`, with no `class="prose"`.

### Known-Red Ledger (Phase 3)

`biscuit-terminal` is green. Phase 3 changes the meaning of a single `\n`
and of code spans for every `Prose` consumer. These downstream suites went
red as expected, and Phase 7 clears them by call-site migration and
snapshot review:

- **darkmatter** (`just test --no-fail-fast`): 8885 run, 50 failed,
  1 timed out. Two of the failures existed before this phase and are
  unrelated (`current_root_documentation_contract`,
  `current_root_migration_guard`; see Phase 2). The others are
  `error_snapshots::*` (27), `markdown::errors::*`, `shell_blocks` and
  `shell_expansion` status blocks, darkmatter-cli L1 compose and schema
  output, and 4 `zed-dmls-cli` doctor tests. All of them render `Prose` text
  that uses single-`\n` line structure or code spans.
- **claudine** (`just test --no-fail-fast`): 8104 run, 33 failed,
  1 timed out. They fall in composition error blocks, lifecycle hints,
  magic-miss reports, authored-text header rendering, compose validation
  messages, wrap help and watchdog output, and PTY sequence tests.
- Not run in this phase: `sniff`, `messenger`, `worktree`, `model-citizen`,
  `playa`, `homelab`. Expect the same categories there.

### Drift Notes

- `render_tree/projection.rs`, `table/table.rs`, and `list.rs` comments
  named `Prose::to_render_nodes`. They now name the interim bridge.
  `fold_prose_nodes_into_blocks`'s doc no longer claims it backs `Prose`'s
  own `render_tree`.
- `Prose`'s rustdoc described a "dual-mode" layout contract (block when
  rendered, inline when embedded). It was replaced by the block/inline
  split.
- `biscuit-terminal/docs/components/prose.md` still describes the old
  behavior. Phase 6 owns that rewrite, so it is known drift until then.

### Departures

- **Interim container bridge.** Removing `Prose::to_render_nodes` would have
  broken `Table`, `BlockQuote`, the lists, and `project_renderable_content`
  before Phase 4. A crate-private `Prose::interim_container_nodes()` keeps
  their old flat shape: each paragraph's inline children, consecutive
  paragraphs joined by a literal `"\n\n"` text node, and fences as `Code`
  for the containers' existing fold and degrade logic. **Phase 4 deletes
  it.**
- **`Prose` has no `render_tree_node` override yet.** Returning the `Root`
  from it would nest a `Root` wherever a generic container projects the
  component. Phase 4 decides how embedded `Prose` hands over its root
  children and layout (R3).
- **`InlineProse` and the `TerminalRenderable` layout.** The trait requires
  `layout()` and `layout_mut()`, so `InlineProse` stores a `Layout` but
  never applies it. Rendering with a non-default layout logs a `warn`, so
  the layout is not dropped silently. The type has no inherent
  margin/width/align/wrap builders.
- **`bt prose --html` (part of Phase 6, done early).** Once `Prose` emits
  its own layout CSS, the CLI's wrapper doubled the margin. The wrapper now
  carries only `--margin-top`/`--margin-bottom`, which `Layout` cannot
  express (it has no line-height unit). Horizontal margins and alignment
  come from `Prose`. Help text and the new code-span CLI examples remain
  Phase 6 work.
- **Open spec conflict (AC 1).** `Prose` link targets pass through
  `styles::resolve_href` when the tree is built, so a relative `plan.md`
  becomes `file:///<package root>/plan.md` on every target. This behavior
  predates this feature and is what makes OSC 8 links clickable. AC 1
  expects Markdown and HTML to keep `plan.md` unchanged. Phase 3 kept the
  existing resolution and wrote the AC 1 tests with an absolute URL.
  Meeting AC 1 literally means moving resolution to the terminal target
  only. That is a design decision for review, and Phase 8 should not
  silently accept it.

## Phase 4

Phase 4 moves every container onto the shape the spec assigns it. Validation
ran on macOS.

### What Changed

- **Embedding `Prose`** (`prose/tree.rs`). The interim bridge
  `Prose::interim_container_nodes` is deleted. Its replacement,
  `Prose::embedded_nodes()` (crate-private), returns the root's `Paragraph`
  and `Code` blocks and never a `Root`. A non-default layout moves onto those
  blocks:
  - one block carries the whole layout;
  - several blocks each carry the horizontal box (left and right margin and
    padding, width, max width, alignment, word wrap), the first keeps the
    top margin and padding, and the last keeps the bottom ones.
- **One funnel** (`render_tree/projection.rs`). `project_renderable_content`
  and `RenderableTerminalContent::to_tree_nodes` both special-case `Prose`
  and return `embedded_nodes()`. `fold_prose_nodes_into_blocks` is deleted,
  and so is the `ProjectionMode::InlineOnly` variant, which had no remaining
  caller. `InlineProse` needs no special case: its `render_tree_node` is
  its neutral `Span`.
- **`Prose::is_block_level()` is `true`**. Lists only add a hanging-indent
  word wrap to non-block components, and that wrap never affected the tree
  path. Without this change it would have become a non-default layout and
  been moved onto every list item's paragraph.
- **Lists**: a `Prose` item's blocks are the `ListItem`'s children, with no
  folding.
- **`BlockQuote`**: a `String` becomes one `Paragraph`. Every component,
  `Prose` included, projects structurally, and only an all-inline
  projection is wrapped in a `Paragraph`. `paragraph_children` is deleted.
- **`TwoColumn`**: unchanged code. A `Prose` column now reaches the funnel's
  special case instead of the ANSI-stripped `Text` fallback it hit before
  (it had no `render_tree_node`). Strings and inline content are still
  grouped into a paragraph.
- **`StatusBlock`**: the body is structural. Each `Prose` item contributes
  its `embedded_nodes()`, so it keeps its inline styling and **its
  `LineBreaks` mode**. Before, `body_plain_text` re-parsed
  `Prose::new(p.content())`, which dropped the item's mode, so Phase 7's
  `LineBreaks::Hard` migrations of single-`\n` body sites would have had no
  effect. `body_plain_text` is deleted; the header and hint are unchanged.
- **`Compose`**: places a `"\n\n"` `Text` between consecutive blocks of one
  `Prose` part, because its `SequenceJoin::None` sequence has no separator
  (without it `one\n\ntwo` rendered `onetwo` on terminal and Markdown).
- **Table** (R4): `TableCellContent::StyledInlineProse(Box<InlineProse>)`
  replaces `StyledProse`; `From<InlineProse>` replaces `From<Prose>`;
  `TableColumn::header_prose: Option<InlineProse>`; `degrade_code_nodes` is
  deleted (an `InlineProse` fence is already `InlineCode`); the cell hint
  token is `styled_inline_prose` (writer in `table.rs`, reader in
  `render_tree/render.rs`, doc in `renderable/src/tree/attrs.rs`).
- **`InlineContent`**: `From<InlineProse>` replaces `From<Prose>`;
  `add_inline_prose(InlineProse)` replaces `add_prose(Prose)`.
- **`bt table`**: `--prose-row` and `--mixed-row` cells are
  `InlineProse` with `LineBreaks::Hard`, so the documented "`\n` → hard line
  break" behavior holds (Phase 3's soft single newline had broken it). Help
  text renamed.
- **Compile-only downstream migrations**. Removing `From<Prose>` broke four
  cell sites outside biscuit-terminal. Each was migrated by type only, so the
  workspace compiles for Phase 5:
  - `darkmatter/cli/src/commands/schema/about.rs` `prose_cell`: now an
    `InlineProse`. The `with_word_wrap(WrapProse(6))` it set was never
    applied to a cell, and the columns set their own wrap.
  - `biscuit-icon/cli/src/{commands.rs,sets_table.rs}`: three icon/title
    cells.
  - `claudine/cli/src/commands/steer/render.rs`: `cell()` now returns
    `InlineProse` built from a shared `cell_markup()`. The narrow stacked
    summary line, which also used `cell()`, renders through `line()` (block
    `Prose` with wrap) so it keeps wrapping. The first attempt lost that
    wrap; `steer::tests::a_narrow_terminal_wraps_details_instead_of_hiding_them`
    caught it.

### Requirement-to-Test Mapping

All tests are L1. The new module `lib/tests/l1/prose_containers.rs` is
declared in `tests/l1/main.rs`, and no path segment carries a tier marker.

| Requirement (AC) | Test(s) |
| --- | --- |
| Block containers embed each `Prose` paragraph as its own block, keep bold and inline code structural, no nested `Root`, valid tree (AC 12): `UnorderedList`, `OrderedList`, `BlockQuote`, `TwoColumn`, `StatusBlock`, `Section`, `Compose` | `prose_containers::block_containers_embed_each_prose_paragraph_as_its_own_block` |
| Fenced code stays a sibling `Code` block in every block container (AC 12, 27) | `prose_containers::block_containers_keep_fenced_code_as_a_sibling_block` |
| Paragraph separation on every target (`BlockQuote` terminal, Markdown, and HTML; `Compose` terminal and Markdown; list Markdown) | `prose_containers::embedded_paragraphs_stay_apart_on_every_target` |
| Embedded layout appears exactly once, on the `Prose` paragraph, never a nested `Root`, never on the container node (AC 31) | `prose_containers::embedded_prose_layout_appears_exactly_once_on_its_paragraph` |
| Multi-block layout keeps one outer box (top edge first, bottom edge last, horizontal box on each) (AC 31) | `prose_containers::embedded_multi_block_layout_keeps_one_outer_box` |
| Transferred margin renders inside the quote border (terminal) and once, on the `<p>` (HTML) | `prose_containers::embedded_prose_margin_renders_inside_the_container` |
| `InlineContent` takes `InlineProse`; styling kept; a fence in it is one inline code value (AC 12, 28) | `prose_containers::inline_content_takes_inline_prose`, `prose_containers::inline_content_fence_is_one_inline_code_value` |
| Table header label is `InlineProse` | `prose_containers::table_header_label_is_inline_prose` |
| `TwoColumn` `Prose` column is structural, not stripped text | `prose_containers::two_column_prose_column_projects_structurally` |
| Multiline fence in a table cell becomes one `InlineCode` (`fn main() {} let x = 1;`) with no `Code`, no hard break, no fence or language on terminal, Markdown, and HTML (AC 28, table half) | `prose_cells_parity::multiline_fence_in_a_cell_is_one_inline_code_value` (replaces `fenced_code_in_prose_degrades_to_text`) |
| `From<InlineProse>` produces `StyledInlineProse`; hint `styled_inline_prose` (R4) | `prose_cells_parity::inline_prose_into_produces_styled_inline_prose`, `…::inline_prose_into_boxed_correctly`, `…::styled_inline_prose_cell_hints`, `…::mixed_type_row_retains_formatting_and_alignment` |
| `StatusBlock` body keeps inline structure; items are sibling paragraphs on tree, Markdown, and HTML; the item's `LineBreaks` mode survives | `status_block::tests::body_prose_keeps_its_inline_structure`, `…::multiple_body_items_are_sibling_paragraphs`, `…::body_line_breaks_mode_survives_projection`, `status_block_parity::multiple_body_items_keep_blank_line_separation_in_block_quote` (rewritten) |
| List `Prose` items keep semantic emphasis in Markdown | `unordered_list_parity::prose_item_keeps_semantic_emphasis_in_markdown`, `ordered_list_parity::prose_item_keeps_semantic_emphasis_in_markdown` (replace the misnamed `…_degrades_in_markdown`) |
| `BlockQuote` `Prose` keeps emphasis in Markdown and HTML | `block_quote::tests::test_render_markdown_from_prose_keeps_semantic_emphasis`, `…::test_browser_renderable_fragment_from_prose` |
| `Compose` trailing-`\n` spacing moved outside the component | `compose::tests::test_add_file_system_with_prose`, `…::test_add_table_with_prose`, `…::test_mixed_all_types` (now pin the line break) |

`fold_prose_nodes_into_blocks`, `degrade_code_nodes`,
`interim_container_nodes`, `StyledProse`, and `styled_prose` have zero
hits in `biscuit-terminal/lib`, `biscuit-terminal/cli`, and
`renderable/src`.

### Validation

- `biscuit-terminal`: `just test` gives 3458 passed and 57 skipped (lib and
  CLI). `just lint` is clean. Lib doc tests give 203 passed with
  `--features image` (the same as Phase 3). `just test-browser` gives 56
  passed. `cargo check --all-targets` passes for the lib with
  `terminal-tests,browser-tests,image` and for the CLI with
  `terminal-tests`. L2 was not run (the plan defers it to Phase 6).
- `bt table --columns "Msg,Note" --prose-row '<b>line one\nline two</b>,```sh\nmd hash\n```'`
  renders a two-line `Msg` cell and `md hash` as one inline code value.
- `cargo check --workspace --all-targets --keep-going` is clean.
- `biscuit-icon`: `just test` gives 228 passed. claudine-cli `steer` tests:
  50 passed.
- OS risk: the changes are pure tree logic with no `cfg` and no filesystem
  access. No `just cross-check` was run; CI covers Linux.

### Snapshot Review

No snapshot moved in this phase. No `.snap.new` files were left behind.

### Known-Red Ledger (Phase 4)

Phase 4 adds no red. biscuit-terminal is green.

- **darkmatter** (`just test --no-fail-fast`): 8885 run, 50 failed,
  1 timed out. These are the same counts as Phase 3, and they include the two
  unrelated `current_root_*` failures that existed before this feature.
  `darkmatter-cli::l1 schema_about::schema_about_verbose_prints_advanced_sections_as_readable_lists`
  is one of them. `detail_list` builds items as `"…\n  - Details: …"`, and
  Phase 3 made that single `\n` soft. Phase 7 migrates it to
  `LineBreaks::Hard`.
- **claudine**: in this worktree, `just test` ran only 7359 tests and showed
  1 failure, so it is not a complete measure here. A direct
  `cargo nextest run -p claudine -p claudine-cli -p claudine-gen -p claudine-contract -p claudine-catalog-types --no-fail-fast`
  gives 8045 run and 33 failed. That matches Phase 3's 33, in the same
  categories (composition error blocks, lifecycle hints, magic-miss reports,
  authored-text headers, compose validation messages, wrap help and
  watchdog, PTY sequence tests). Spot checks:
  - `lifecycle_short_form_removed_status_block_is_escape_free_at_none` is an
    `escape_text`-in-backticks site (AC 17, Phase 7).
  - `shell_expansion_failed_plain_terminal_has_no_escape_bytes` emits no
    escape byte and fails on line structure.
- **Unrelated and not counted**:
  `claudine-cli completion::composition::tests::compose_magic_does_not_emit_a_nested_file_without_its_scope`
  failed once in the `just test` run (`["@plan.md"]` flattened). It is a
  magic-path completion test with no Prose involvement, and it did not fail
  in the direct run.
- Still not run: `sniff`, `messenger`, `worktree`, `model-citizen`,
  `playa`, `homelab`. They compile.

### Scope Record (R10)

- **`biscuit-icon-cli`** consumes `TableCellContent`/`Prose` and is not in
  the spec's `packages:` list. Its three cell sites are migrated (above), and
  Phase 7 and Phase 8 should treat it as a consumer.
- `tree-hugger-cli` builds `Prose` with layout builders (found by the
  Phase 4 grep). It compiles unchanged, but it is not in the spec's list
  either.

### Drift Notes

These comments described behavior that no longer exists, or never did. Each
was corrected in the same change; the code was taken as correct.

- `OrderedList` and `UnorderedList` `render_markdown` rustdoc, and the
  list parity module docs and test names, said a `Prose` item's `<b>`
  "degrades to plain text" in Markdown. It was already `**…**` before this
  feature, and those tests passed only because they asserted
  `!contains("<b>")`.
- `BlockQuote::render_markdown_plus` rustdoc and two `block_quote.rs`
  tests said `render_tree()` "flattens Prose styling". It did not.
- Module and helper docs in `projection.rs`, `list.rs`, `block_quote.rs`,
  `two_column.rs`, `table.rs`, `status_block.rs`, `compose.rs`,
  `render_tree_component_parity.rs`, and the L2 `prose_cells.rs` named the
  interim bridge, the fold, `Prose::to_render_nodes`, or `StyledProse`.
  Each was rewritten to the current behavior.

### Departures

- **R3, the layout target.** R3 preferred the container's own block node
  (`ListItem`, `BlockQuote`) for the layout. That node is the wrong box:
  - a margin on the `ListItem` would move the marker, which sits outside the
    Prose's box;
  - the `BlockQuote` node already carries the quote's own layout (the
    border-gap padding).

  The layout therefore goes on the Prose's own blocks. With one block (the
  common case) it appears exactly once, as AC 31 requires. With several
  blocks, the horizontal box is repeated on each block and the vertical
  edges are split, which keeps one visual box without adding a
  neutral-block `NodeKind`. A new kind would have rippled into darkmatter's
  exhaustive matches.
- **Terminal list renderer and layout.** The terminal list renderer
  renders a `ListItem`'s paragraphs inline itself, so a layout moved onto a
  list-item paragraph applies in the browser and Markdown output but not on
  the terminal. Before this phase the interim projection dropped that layout
  on every target. The behavior is documented in `docs/components/list.md`.
- **`Prose::render_tree_node` stays `None`.** The hook returns one node,
  and the only single node that can hold several blocks is the `Root`,
  which cannot nest. Embedding goes through the projection funnel, which
  shares `block_nodes()` (the same parse and break mode) with
  `render_tree`. Nothing outside biscuit-terminal calls `render_tree_node`
  on a `Prose`.
- **Strings stay literal.** The spec's "strings converted by a table or
  `InlineContent` use `InlineProse`; by a block container use `Prose`" is
  read as covering existing string-to-Prose conversions
  (`TableColumn::new_with_bold`, `IntoProseVec for &str` in `StatusBlock`).
  It does not turn `RenderableTerminalContent::String` or
  `TableCellContent::Text` into parsed markup, which would reinterpret `_`,
  `*`, and `<` in every plain cell and list item in the workspace.
- **Compile-only migrations outside biscuit-terminal** (four sites, above)
  happened in Phase 4 rather than Phase 7 so the workspace builds for
  Phase 5. They are type-only and are listed for Phase 7's review.
- **Component docs.** Checkpoint 4 asks for the container pages, and
  Phase 6 lists them again. `table.md`, `inline_content.md`, `list.md`,
  `block_quote.md`, `two_column.md`, the table README, and the crate README
  `StatusBlock` section now state the accepted types. `prose.md`,
  `index.md`, and `browser-renderable-trait.md` remain Phase 6 work.

## Phase 5

Phase 5 adds darkmatter's `code_link()` expression function and moves the
templates that wrapped `{{link(x)}}` in backticks onto it. Validation ran on
macOS.

### What Changed

- **Shared resolution** (`darkmatter/lib/src/markdown/compose/expression/functions/mod.rs`):
  `resolve_link_parts(function, args, ctx)` now holds everything `link_fn`
  did before it spelled the label: arity, null propagation, string checks,
  the one-argument URL rejection, file resolution, the relative label, and
  the destination. It returns the raw label and destination (`LinkParts`),
  or `None` for a null result. Every error names the `function` it was given,
  and `portable_destination` takes the name too. `link_fn` and the new
  `code_link_fn` are thin wrappers, so `link()`'s output and error text are
  byte-for-byte unchanged (`link(): link() …`). `code_link()` reports the same
  cause as `code_link(): code_link() …`.
- **Label spelling**: `format_markdown_link` (escapes `\`, `[`, `]`) and the
  new `format_markdown_code_link` share `markdown_destination` (angle-bracket
  wrapping). The code label goes through `renderable::markdown::code_span`,
  the Phase 2 helper, with no other escaping. That helper already turns line
  endings into spaces and returns `""` for an empty value, which gives
  `[](dest)`.
- **Binding** (`functions/paths.rs`): `code_link`, alias `codelink`, beside
  `link`. The underscore-free alias follows the module's convention
  (`find_files`/`findfiles`, `has_skill`/`hasskill`).
- **Catalog** (`darkmatter/docs/schemas/expression-functions.yaml`):
  `code_link` with order 78 (the free slot between `link` 77 and 79), two
  overloads, and a `display-only` example each. The expected-signature list
  in `catalog/mod.rs` gains both signatures. The registration baseline in
  `catalog/parser.rs` goes from 112 functions and 119 overloads to 113 and
  121.
- **Docs**: the generated function table in
  `darkmatter/docs/topics/darkmatter-expressions.md` was regenerated with
  `cargo run -p darkmatter --example expression_doc_generator -- --write`
  (two rows added). "Link Helpers" gains `code_link`, with examples, the
  bracket note, the fence rule, the empty case, and the reason to prefer it
  over a backtick-wrapped `link()`.
- **Templates**: `prompts/plan.md:30`, `prompts/_reviews/review-spec-inline.md:18`,
  `claudine/cli/tests/fixtures/nested_span_regression/review-spec-inline.md:18`,
  and `darkmatter/dmls/tests/fixtures/mapping_only_corpus/_reviews__review-spec-inline.md:18`
  now use `{{code_link(x)}}` / `{{ code_link(spec) }}` with no backticks.
  Line numbers are unchanged, and every test that reads the two fixtures
  (claudine `nested_span`, `wrap_compose_validation`, `shipped_prompt_contract`;
  darkmatter `lint.rs`, `parser.rs`, DMLS `nested_span_tests`,
  `frontmatter_inventory_tests`) passes, apart from one existing red listed
  below.
- **Snapshot (AC 20)**: `l1__error_snapshots__link__unrecognized_format.snap`
  moved from ``Input did not look like an HTML `<a>` tag or a Markdown `text` link.``
  to `Input did not look like an HTML <a> tag or a Markdown [text](href) link.`.
  Reviewed: `[text](href)` is now literal inline code, and the snapshot strips
  its styling, so no backticks remain. This is the expected category. A stale
  `.snap.new` from an earlier run was replaced by a fresh run before it was
  accepted.
- **Skill**: `.claude/skills/darkmatter/compose.md` helper list gains two
  `code_link` lines.

### Requirement-to-Test Mapping

| Requirement (AC) | Test(s) | Tier / target |
| --- | --- | --- |
| `code_link("plans/foo.md")`: label `plans/foo.md` as one code span; destination spelling and quoting identical to `link()`, including angle-bracket wrapping (AC 22) | `functions::tests::fn_phase5::code_link_one_arg_matches_link_destination_with_a_code_label` | L1, darkmatter lib unit |
| Two-argument file, `https`, and uppercase-scheme destinations match `link()` | `…::code_link_two_arg_file_and_url_destinations_match_link` | L1 unit |
| `code_link(url, "a]b")` keeps `]` (and `[`) unescaped, and a CommonMark parser reads one code span (AC 22) | `…::code_link_keeps_brackets_unescaped_inside_the_code_span` | L1 unit |
| `code_link(url, r"a\_b")` emits `a\_b` as is (AC 22) | `…::code_link_emits_backslashes_as_is` | L1 unit |
| A backtick in the text gets a longer fence; edge backticks and edge spaces are padded; round-trip through pulldown-cmark (AC 22) | `…::code_link_fences_backticks_and_pads_edges` | L1 unit |
| `\n`, `\r\n`, and lone `\r` become spaces | `…::code_link_turns_line_endings_into_spaces` | L1 unit |
| `code_link(url, "")` gives `[](url)`, the same as `link(url, "")` (AC 22) | `…::code_link_empty_label_is_an_ordinary_empty_link` | L1 unit |
| Null propagation matches `link()` for all three null shapes | `…::code_link_null_propagates_like_link` | L1 unit |
| Errors carry `link()`'s cause under `code_link`; `link()` error text is unchanged (URL in one-argument form, both arity errors, unparseable and malformed targets, wrong types) | `…::code_link_errors_name_code_link_with_the_link_cause` | L1 unit |
| Dispatch by `code_link` and `codelink` | `…::code_link_is_dispatched_by_name_and_alias` | L1 unit |
| Catalog and runtime parity, expected signatures, registration baseline (AC 23) | `catalog::tests::catalog_and_runtime_bindings_have_bidirectional_canonical_parity`, `catalog::tests::…expected…` signature list, `catalog::parser::tests::authored_catalog_matches_registration_baseline`, `narrative_doc_function_table_matches_catalog` | L1 unit |
| The four migrated templates' real `code_link` lines (read with `include_str!`) compose, through the normal compose path, to `` [`plans/foo.md`](…) `` and render as a link whose only child is `InlineCode` on every target: darkmatter tree, Markdown, HTML (`<code>…</code></a>`), terminal (OSC 8, no fence when styled), and the same four through `Prose` (AC 22) | `darkmatter::l1 code_link::migrated_templates_render_an_inline_code_link_on_every_target` | L1, `darkmatter/lib/tests/l1/code_link.rs` (declared in `main.rs`) |
| Composed `code_link()` and `link()` share a destination; a backtick-wrapped `{{link(x)}}` is now opaque code with no link (AC 16, 22) | `code_link::code_link_matches_link_and_a_backticked_link_is_literal_code` | L1 |
| `a\_b` from frontmatter composes literally and renders as `a\_b` in darkmatter's parser and in `Prose` on every target (AC 22) | `code_link::code_link_backslash_text_is_literal_in_prose_and_darkmatter` | L1 |
| `a]b` and `` a`b `` survive compose and both parsers | `code_link::code_link_brackets_and_backticks_survive_compose_and_render` | L1 |
| Empty text composes to `[](url)` | `code_link::code_link_empty_text_is_an_empty_link` | L1 |
| `unrecognized_format` snapshot shows `[text](href)` (AC 20) | `darkmatter::l1 error_snapshots::link::unrecognized_format_mentions_html_and_markdown` | L1 |
| `claudine context --expressions` lists `code_link(file)` and `code_link(target, desc)` once each, in the Filesystem group, directly after `link()`'s two rows (AC 23) | `claudine-cli::l1 context_command::context_expressions_lists_code_link_beside_link_in_the_filesystem_group` (the existing `context_expressions_includes_every_function` also covers them now) | L1, real CLI process |
| DMLS offers `code_link` as a completion with the catalog signature, typed detail, description, and bare-name `textEdit` (AC 23) | `dmls::l1 lsp_session::function_completion_offers_code_link` | L1, real LSP session |

No new name segment carries a tier marker. `code_link.rs` is declared in
`darkmatter/lib/tests/l1/main.rs`, and `test_layout` passes. The
`include_str!` reads of `prompts/` and the two fixtures make those files test
inputs that CI can see.

### Validation

- darkmatter area `just test --no-fail-fast`: 8901 run, 8850 passed,
  49 failed, 2 timed out, 12 skipped. Phase 4 had 8885 run, 50 failed, and
  1 timed out. The one fewer failure is `error_snapshots::link::unrecognized_format`
  (fixed here). The one extra timeout is
  `darkmatter-cli::l1 entry_point_parity::md_entry_points_agree_on_every_reference`.
  Both `entry_point_parity` tests pass in isolation (19 passed), so this is
  load at the 30 s limit, not a regression. Every remaining failure is in
  the existing ledger categories (Prose error blocks and error snapshots,
  `schema_about`, the two `current_root_*` tests, and the zed-dmls-cli
  `doctor` text). None involves `link`, `code_link`, the catalog, or the
  migrated files.
- Targeted runs: darkmatter lib `fn_phase5` 34/34 (10 new) and the catalog
  and registration tests; `darkmatter::l1 code_link::` 5/5; DMLS
  `function_completion` 2/2; claudine `context_command::` 28/28;
  claudine `shipped_prompt_contract::` 12/12.
- claudine fixture readers (`nested_span`, `wrap_compose_validation`,
  `shipped_prompt_contract`, `context_command`): 58 run, 57 passed. The
  failure, `composition::lifecycle::tests::nested_span::nested_span_error_renders_property_literal_rewrite_and_escape_hint`,
  renders `\{\{ctx.area}}` with visible backslashes. It concerns line 15
  (`success.say`), not the migrated line 18. This is the AC 17
  `escape_text`-in-backticks category that Phase 7 owns.
- `just lint`: darkmatter, claudine, and biscuit-terminal are all clean.
- biscuit-terminal `just test`: 3458 passed and 57 skipped, the same as
  Phase 4. Phase 5 changed no biscuit-terminal code.
- OS risk: the new code is string formatting over `link()`'s existing
  resolution, with no `cfg` and no new filesystem access. The tests use
  `tempfile` directories and `str::lines`, which also strips CRLF from a
  Windows checkout. No `just cross-check` was run; CI covers Linux and
  macOS on the pull request and Windows on `main`.

### Snapshot Review

- `l1__error_snapshots__link__unrecognized_format.snap`: accepted (AC 20,
  diff above).
- Moved, not accepted, and left for Phase 7 (same category: a code span is
  no longer wrapped in backticks once styling is stripped):
  `error_snapshots::link::missing_href_has_href_hint`
  (``Link has no `href`/`url` attribute.`` → `Link has no href/url attribute.`),
  and the `error_snapshots::image_ref::*` snapshots.

### Known-Red Ledger (Phase 5)

Phase 5 adds no red. It removes `error_snapshots::link::unrecognized_format_mentions_html_and_markdown`
from the darkmatter list. The rest of the darkmatter list (49 failures; the
second timeout is load) and the claudine list are unchanged, and Phase 7
owns them.

### Drift Notes

- `darkmatter-expressions.md` "Link Helpers" said `link()` destinations are
  "wrapped in angle brackets or percent-encoded", and that link text escapes
  only `[` and `]`. The code never percent-encodes, and it also escapes `\`.
  The paragraph now describes the code.
- `link_fn`'s one-argument and two-argument `invalid URL` branch cannot be
  reached: `is_remote_url` returns true only for a target that already
  parsed as an `http(s)` URL. A target that does not parse fails as a file
  path. The branch is kept in the shared resolver so `link()`'s behavior
  is unchanged. The error test asserts the reachable shapes
  (`http://[::1` → "invalid file path", and a URL with a space → a
  file-reference error).

### Departures

- **No claudine end-to-end assertion on the `review-spec-inline.md`
  `success.message`.** The `message` channel posts to the external messenger,
  so nothing reaches the compose output that `shipped_prompt_contract` can
  observe. A first attempt asserted on it and was reverted. The
  template-to-render proof is in darkmatter's `code_link::migrated_templates_render_an_inline_code_link_on_every_target`,
  which composes each template's real `code_link` line.
- **Prose resolves a relative destination to `file://`.** Compose normalizes
  a `code_link()` destination relative to the document (`./plans/foo.md`, the
  same as `link()`). `Prose` then turns that into a `file://` URL against the
  process directory. This is existing `Prose` link behavior that Phase 5 did
  not change. The end-to-end test compares the path tail on the `Prose` side.
- The catalog `description` for `code_link` is one paragraph on both
  overloads, which is how `link` is written. The bracket note is in the
  description because `claudine context --expressions` and DMLS hover show
  only that text.

### Manual Step (for Ken)

`~/.claudine/prompts/plan.md` and `~/.claudine/prompts/_reviews/review-spec-inline.md`
are outside the repository and still use `` `{{link(x)}}` ``. Replace each with
`{{code_link(x)}}` (no backticks). Until then, those lines show the literal
`[text](url)` as code.

## Phase 6

Phase 6 brings the `bt` CLI and the documentation into line with the code.
Validation ran on macOS.

### What Changed

- **`bt prose`** (`cli/src/commands/prose.rs`). `render_html_with_layout`
  and `vertical_margin_css` are deleted. Every margin flag now sets the
  `Prose`'s own `Layout`: `--margin-top`/`--margin-bottom` set
  `layout.margin.top`/`.bottom` as `Length::ch`, which the browser renderer
  lowers to `lh` and the terminal renderer emits as blank rows. The terminal
  path no longer calls `emit_vertical_margins`, so each margin is applied
  once on every target. `--html` output is `Prose`'s own fragment (one
  `<div style>` from its layout, `<p>` per paragraph, no `class="prose"`).
  The help text (`--help`) describes block prose, newline rules, code spans,
  and how the layout flags apply per target. No flags were added (R7).
- **`--alignment` in HTML** (`cli/src/commands/shared.rs`). The three
  identical alignment-only wrappers in `prose.rs`, `section.rs`, and
  `list.rs` became one helper, `render_html_with_alignment`, which adds a
  `<div style="text-align: …">` only when `--alignment` is set. See
  Departures.
- **`bt section`** (`cli/src/commands/section.rs`). Each `--content` item is
  now a `Prose` (after `unescape_shell_escapes`, as `bt list` and `bt prose`
  do). Before, items were literal strings and `--html` ran them together
  (`<section><h2>…</h2>Follow these steps to deploy.Verify the build passes.</section>`);
  now each is a `<p>`. Terminal and Markdown output for plain text is
  unchanged.
- **`bt list`** needed no shape change: its items were already `Prose`, and
  `--html` already gives `<li><p>…</p></li>` with inline `<code>`.
- **Library fix: bottom margin lost under word wrap**
  (`lib/src/components/prose/render.rs`). `Prose::render_via_tree`'s
  word-wrap pass split the tree output with `str::lines`, which drops the
  trailing empty rows a bottom margin produces. `bt prose` (which wraps by
  default) printed `--margin-bottom 2` as one blank line, and any `Prose`
  with a bottom margin and a wrapping `WordWrap` lost it. It now splits on
  `'\n'`, so wrapped and unwrapped output have the same rows. Found while
  moving the vertical margins onto `Prose`.
- **Docs.** `docs/components/prose.md` is rewritten section by section to
  the spec's Documentation table (intro with both components; Rendering
  Model with a Mermaid diagram and a two-component table; Programmatic Use
  with the choosing rule; new Paragraphs and Line Breaks, Block Tag, and
  Layout sections; Markdown Subset with the code-span value rules and the
  link-with-code-text sentence; Escape Mechanism; Supported Tags; Cross-Target
  Rendering; Prose in Other Components (container table); InlineProse in
  Table Cells; Key API for both types; Graceful Degradation → Inline Code;
  CLI with code-span and layout examples taken from real output). It names
  no spec or feature. Other pages: `index.md` (InlineProse row; Prose
  described as block), `section.md` (Prose content items; the stale "not
  exposed as a CLI command" section replaced with `bt section`; sample output
  corrected to show the blank line between items), and the
  `#prose-in-table-cells` → `#inlineprose-in-table-cells` anchor in
  `table.md`, `lib/src/components/table/README.md`, and the crate README.
  `renderable/docs/components.md` gains an `InlineProse` row, marks `Prose`
  as Block, and renames `StyledProse` to `StyledInlineProse`.
- **`inline-prose.md` not created.** `InlineProse` is a thin type over the
  shared grammar (`new`, `content`, `with_line_breaks`, `to_render_nodes`,
  escape helpers); `prose.md` documents both side by side, so the grammar is
  documented once and nothing needs a second page.
- **Skills.** `biscuit-terminal/styling.md` (both components, a Newlines
  table, code spans as `InlineCode`, no escaping inside code spans, the stale
  `Prose` struct sketch replaced with the real builders), `components.md`
  (catalog rows for `InlineProse` and block `Prose`), `cli.md` (block prose,
  code-span example, the `--html` shape and the alignment wrapper).
  `SKILL.md`, the `renderable` skill (hard break, `code_span`), and the
  `darkmatter` skill (`code_link`) were already current from Phases 2–5.
- **Module docs.** `markdown.rs`, `prose.rs`, and `inline_prose.rs` were
  already current. `tree.rs` said links keep an "un-resolved `href`" that
  each target re-resolves; the parser resolves it (see Drift Notes).
- README and `docs/dependencies.md`: no crate changes, so no dependency
  edits (the README change is the anchor only).

### Requirement-to-Test Mapping

All new tests are L1 and live in files already declared by their binaries
(`lib/tests/l1/prose_grammar.rs` in the lib's `l1`; `cli/tests/l1/integration_test.rs`
in the CLI's `l1`). No path segment carries a tier marker.

| Requirement | Test(s) |
| --- | --- |
| `bt prose --margin-left 4 --html`: margin exactly once, `<p>`, no `class="prose"` (AC 10, 11) | `integration_test::test_prose_html_margin_left_appears_once`, `…::test_prose_html_without_margin_renders_a_paragraph` (Phase 3, still green) |
| `--html` with all margins: one element, every margin on it, no CLI wrapper, paragraphs as `<p>`, no `class="prose"` (AC 10, 11) | `integration_test::test_prose_html_layout_is_one_element_with_every_margin` |
| `--alignment` still reaches HTML; margins stay on `Prose`, once | `integration_test::test_prose_html_alignment_is_text_align_and_margins_stay_on_prose` |
| Terminal vertical margins are identical with and without word wrap (CLI) | `integration_test::test_prose_terminal_vertical_margins_match_with_and_without_wrap` |
| Library regression: bottom margin survives the word-wrap pass; vertical margins lower to `lh` (fails with `lines()`, verified by reverting the fix) | `prose_grammar::vertical_margins_survive_word_wrap_on_the_terminal` |
| Code span with `--html` and `--md`; `` `[desc](ref)` `` literal on both (AC 10/11 CLI half, AC 16) | `integration_test::test_prose_code_span_renders_as_inline_code_on_html_and_markdown` |
| `bt section` content items are `Prose` paragraphs with inline code | `integration_test::test_section_html_renders_each_content_item_as_a_paragraph` |

### Validation

- `biscuit-terminal` `just test`: 3464 passed, 57 skipped (lib and CLI).
- `biscuit-terminal` `just lint`: clean (lib and CLI).
- `biscuit-terminal` `just test-l2`: 2 + 76 passed; no window took focus
  (the harness backends run unfocused).
- Docs grep over `biscuit-terminal/docs`, the crate README, lib and CLI
  sources, the `biscuit-terminal`/`renderable`/`darkmatter` skills, and
  `renderable/docs` for `class="prose"`, `<span class="prose">`, "Styled
  inline text component", visible-backtick wording, "exactly one link",
  "including inside a code span", "degrades to escaped literal",
  `render_html_with_layout`, `Prose::to_render_nodes`, `whole_span_link`,
  `StyledProse`: no hits apart from tests asserting their absence.
- OS risk: the changes are string and layout logic with no `cfg`, paths, or
  filesystem access; the CLI tests compare `\n`-only output, which Rust does
  not translate on Windows. No `just cross-check` was run.

### Snapshot Review

No snapshot moved in this phase; no `.snap.new` files were left in
`biscuit-terminal`.

### Known-Red Ledger (Phase 6)

Phase 6 adds no red; `biscuit-terminal` is green. The darkmatter and
claudine entries from Phase 5 are unchanged (no source outside
`biscuit-terminal` changed). The word-wrap fix affects only `Prose` output
that ends in blank rows (a bottom margin or padding with a wrapping
`WordWrap`); a workspace search found no consumer that sets either on a
`Prose`.

### Drift Notes

- `prose/tree.rs` module docs said links carry an "un-resolved `href`
  (each target re-resolves per its own rules)". `tokens.rs` resolves every
  link target through `styles::resolve_href` while parsing, so a file path
  is already a `file://` URL in the tree. The comment now says so.
- The Phase 3 log (Departures, "`bt prose --html`") said vertical margins
  needed a CLI wrapper because `Layout` "has no line-height unit". It has
  one in effect: `Length::Ch` on a vertical side lowers to `lh`. The wrapper
  is gone.
- `docs/components/section.md` said `Section` was "not directly exposed as a
  standalone CLI command"; `bt section` exists. Its sample output also
  omitted the blank line the renderer puts between content items.
- `renderable/docs/components.md` described `Prose` as Inline and table
  cells as `StyledProse`.

### Departures

- **`--alignment` keeps an alignment-only wrapper.** The plan says `bt prose`
  stops wrapping its fragment. Phase 3 removed the alignment declaration
  along with the margins, so `bt prose --alignment center --html` lost its
  centering (`main` emitted `text-align`). `renderable`'s `layout_to_css`
  expresses alignment only as auto margins on a box with `max_width`
  (documented in `renderable/docs/layout-and-style.md`), while the terminal
  centers the lines of a full-width block. Changing that contract is a
  `renderable` design change outside this phase, so `bt prose` uses the same
  alignment-only wrapper `bt section` and `bt list` already used, now one
  shared helper. Margins never go on the wrapper. If `renderable` later
  lowers alignment without `max_width` to `text-align`, the helper and its
  three callers can be deleted.
- **`bt section` content is markup now.** The plan names `section.rs` for
  "old shapes"; making each `--content` item a `Prose` is the change that
  gives each item its own paragraph in HTML. A `--content` value containing
  `_`, `**`, `<tag>`, or backticks is now parsed as markup, as `bt list` and
  `bt prose` arguments already were.
- **Open finding for Phase 8, not fixed here (`renderable`).** A text node
  ending in a literal backslash followed by a soft break (`Prose` input
  `a\\` + newline + `b`, i.e. an escaped backslash before a newline) renders
  to Markdown as `a\` + newline + `b`, which Markdown reads as a hard break.
  `renderable`'s Markdown text output does not escape a trailing backslash.
  Terminal and HTML are correct (`a\ b`). This predates the feature (CommonMark
  has always read backslash-newline as a hard break) but the spec's "Markdown
  output preserves the resulting meaning" does not hold for this input.

### Process Note

Several early shell commands in this phase ran `cd biscuit-terminal` from
the worktree root, and the shell's `CDPATH` resolved that to the **main
checkout** (`/Volumes/coding/personal/rusty-biscuit/biscuit-terminal`). One
edit to `cli/src/commands/prose.rs` landed there. It was confirmed to be
the only change in that checkout, reverted with `git checkout --`, and
reapplied in the worktree; the main checkout's `git status` is clean. Use
absolute paths (or `./biscuit-terminal`) in this worktree.

## Phase 7

Phase 7 migrates every consumer's `Prose` call sites to the new shapes and
reviews the snapshots they move. Validation ran on macOS.

Phase 1 never produced the S1 call-site ledger (the "Scope Record"
placeholder above). Phase 7 therefore classified sites per area as it went:
each area agent applied rule R5 to its own crates and reported the sites it
converted, the sites it kept as `Prose` because they were ambiguous, and
every snapshot it moved. Inventory at the start of the phase (`Prose::new`
occurrences, all code including tests): biscuit-terminal lib 352, claudine
cli 310, sniff cli 201, darkmatter lib 129, biscuit-terminal cli 97,
claudine lib 52, darkmatter cli 51, worktree cli 38, messenger cli 36,
model-citizen cli 19, playa cli 16, homelab cli 9, unchained-ai cli 6,
repo scripts 28, claudine gen 4, biscuit-icon 7, biscuit-tui 2,
tree-hugger cli 1, dmls 1.
