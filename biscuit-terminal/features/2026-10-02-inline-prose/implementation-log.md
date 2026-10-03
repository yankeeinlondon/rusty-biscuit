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
packages:
    - renderable
    - biscuit-terminal
    - biscuit-terminal-cli
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
