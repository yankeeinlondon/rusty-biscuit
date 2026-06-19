# Darkmatter Tree Rendering Migration Plan

## Status (2026-05-21)

All seven success criteria are now met. See `review-1.md` for the
finding-by-finding resolution table, and the spec's *Acceptance Criteria*
section for the matching ✔ markers.

### Review-4 Resolutions (2026-05-21)

| Finding | Resolution |
| --- | --- |
| H1: mixed mark/dim nesting impossible | `SpannedInlineStyleProcessor` now uses a frame stack (`open_stack: Vec<OpenSpan>`) so mark and dim nest. Fold tests `span_aware_fold_nests_dim_inside_mark` and `span_aware_fold_nests_mark_inside_dim` cover both directions of the design's "Mixed Mark and Dim" fixture. |
| H2: HR attributes folded but not rendered | Terminal renderer's `ThematicBreak` arm calls `horizontal_rule_from_attrs` to map `darkmatter.hr.*` hints onto `HorizontalRule` style/alignment/weight/width/color. Browser renderer's `render_thematic_break` surfaces the same hints as `data-hr-*` HTML attributes. Level 1 tests cover both; Level 2 test asserts the waves-rule glyph (`≋` / `~`) appears in the captured frame. |
| H3: benchmark misses span-aware path | `migration_parity.rs` now routes each fixture through `fold_fixture(name, input)`, which selects `fold_markdown_spanned_with_frontmatter` for `mark_dim_hr` and the plain fold for all other fixtures. Baselines.md documents the routing. |

### Review-5 Resolutions (2026-05-21)

| Finding | Resolution |
| --- | --- |
| H1: terminal tree path ignores `TerminalOptions::color_depth` | Added `darkmatter_color_depth_to_terminal` mapping helper and applied it inside `terminal_options_from_terminal_options`, so the tree entry point honors `opts.color_depth`. The bench harness's `pinned_tree_terminal_options(color)` now takes the intended depth and the `migration/terminal_no_color` group includes a `debug_assert_eq!` that the tree context's `color_depth` is `TerminalColorDepth::None`. Level 1 tests `terminal_options_mapping_pins_color_depth_translation` (every variant) and `render_tree_terminal_color_depth_none_emits_no_color_sgrs` (no color SGRs in rendered bytes) pin both wiring and observable contract. Baselines.md flags the recorded no-color numbers for re-capture. |

### Review-10 Resolutions (2026-05-21)

| Finding | Resolution |
| --- | --- |
| H1: DMTR-5 marks parity complete without target coverage for footnotes/superscript/subscript, rich code blocks, image title/width, Mermaid modes, and MarkdownPlus | Added parity fixtures + tests in `render_tree_parity.rs`: `render_tree_parity_footnote_divergence`, `render_tree_parity_superscript_divergence`, `render_tree_parity_subscript_divergence` (parser-option-sensitive — each pins legacy ignoring/mangling vs the tree producing structural nodes), `render_tree_parity_code_block_rich` (syntax-highlight + body equivalence, with the info-string title/line-number/highlight metadata classified as an accepted divergence), `render_tree_parity_image_titled` (alt/src/title equivalence; structured-title `width` classified as accepted divergence), `render_tree_parity_mermaid_off_mode` (the only deterministic Mermaid mode; Text/Image modes classified as deferred), and `render_tree_markdown_plus_preserves_mark_span` (MarkdownPlus preserves span classes portable Markdown drops). `parser-options.md` acceptance criteria are now checked. |
| H2: the tree terminal path never wires darkmatter's code renderer (`code_renderer: None`) | Wired `TerminalCodeRenderer` into `terminal_options_from_terminal_options` (the production entry point), the Level 2 harness `write_doc_to_tempfile`, the bench `pinned_tree_terminal_options`, and the parity harness's `tree_terminal_options`. Entry-point tests `terminal_options_wire_the_darkmatter_code_renderer` and `render_tree_terminal_syntax_highlights_code_blocks` pin both the wiring and the observable syntax-highlighted output. |

### Accepted-Divergence Ledger (Review-10)

| Fixture | Target | Facet | Classification |
| --- | --- | --- | --- |
| `code_block_rich` | Terminal | info-string title / line numbers / highlighted lines | **Accepted divergence** — the `CodeRenderer` hook receives `(lang, value, attrs, context)` but not the code block's info-string `meta`, so these are honored by legacy but not the tree path. Data survives on `NodeKind::Code.meta`; renderer-side threading deferred until the trait grows a `meta` parameter. |
| `image_titled` | Browser | structured-title `width` extension | **Accepted divergence** — the tree fold captures only image `url` + `title`; darkmatter's structured-title `width` is not parsed, so legacy emits a `width` attribute the tree path omits. |
| `footnote` / `superscript` / `subscript` | Browser / MarkdownPlus / Terminal | structural treatment | **Intentional expansion** — tree-experimental parser options widen the fold; legacy ignores/mangles the syntax by design (`parser-options.md`). Visible text survives on both. |
| `mermaid_off` (Text / Image modes) | Terminal / Browser | diagram rendering | **Deferred** — `MermaidMode::Off` is deterministic and covered; Text/Image modes shell out to external renderers / need a tree Mermaid adapter (`entry-point-shape.md`, `baselines.md`). |

## Success Criteria

- [x] Experimental tree-backed Darkmatter entry points exist for Markdown,
  MarkdownPlus, Browser/HTML, and Terminal without changing public legacy
  methods.
- [x] Parser options are explicit, documented, and fixture-backed.
- [x] Darkmatter mark, dim, and HR-attribute processors have a span-aware fold
  path with source ranges preserved.
- [x] Folded `Document`s can carry already-extracted frontmatter metadata.
- [x] Legacy-vs-tree parity tests classify every difference by target and
  phase.
- [x] `cargo bench -p darkmatter --bench migration_parity` exists with baseline
  numbers before any public cutover.
- [x] Raw HTML stays escaped by default on the tree Browser/HTML path.

## Phase 0: Preflight and Baseline

1. Run the focused existing gates to establish the starting point:
   - `cargo test -p darkmatter --test render_tree_roundtrip`
   - `cargo test -p darkmatter --test render_tree_parity`
   - `cargo test -p darkmatter --lib markdown::render_tree`
2. Inspect the current fold/parser option construction and legacy render parser
   options in:
   - `darkmatter/lib/src/markdown/render_tree`
   - `darkmatter/lib/src/markdown/output/html.rs`
   - `darkmatter/lib/src/markdown/output/terminal.rs`
3. Record any already-failing tests before making changes. Do not start by
   changing public render methods.

## Phase 1: Parser Option Policy

1. Add a small shared function or clearly named constants for the render-tree
   fold parser options.
2. Ensure the fold enables only:
   - public now: `ENABLE_TABLES`, `ENABLE_STRIKETHROUGH`
   - tree experimental: `ENABLE_TASKLISTS`, `ENABLE_FOOTNOTES`,
     `ENABLE_SUPERSCRIPT`, `ENABLE_SUBSCRIPT`
3. Fix the legacy HTML table parser gap by adding `ENABLE_TABLES` to
   `as_html`, unless inspection shows a deliberate accepted divergence already
   exists.
4. Add parser-option fixtures for tables, task lists, footnotes, superscript,
   subscript, and GFM alerts.

Verification:

- `cargo test -p darkmatter --test render_tree_roundtrip`
- `cargo test -p darkmatter --test render_tree_parity`

## Phase 2: Internal Pipeline Result and Entry Points

1. Add internal result types in `darkmatter::markdown::render_tree`:
   - `PipelineResult<T>`
   - `PipelineRenderResult<T>`
2. Add internal/module-level entry points:
   - `to_render_document(&Markdown)`
   - `render_tree_html(&Markdown, &HtmlOptions)`
   - `render_tree_terminal(&Markdown, &TerminalOptions)`
   - `render_tree_markdown(&Markdown)`
   - a MarkdownPlus variant or dialect parameter if the local renderer shape
     makes that cleaner.
3. Keep fold diagnostics and render diagnostics separate.
4. Map legacy options narrowly:
   - Browser defaults to `RawHtmlPolicy::Escape`.
   - Code highlighting, Mermaid, HR CSS variables, and terminal images are
     documented parity gaps for the internal path.

Verification:

- Add smoke tests proving a `Markdown` value renders through each tree entry
  point.
- Confirm no public `Markdown` methods or public output functions have changed.

## Phase 3: Frontmatter Above the Fold

1. Add a construction path that accepts Darkmatter's already-extracted
   frontmatter metadata.
2. Populate `renderable::tree::DocumentMetadata`.
3. Keep `pulldown-cmark` metadata block options disabled.
4. Add a fixture proving frontmatter metadata is attached to the folded
   document while the body fold sees only Markdown content.

Verification:

- Focused fold tests for frontmatter metadata.
- Existing frontmatter extraction tests still pass.

## Phase 4: Span-Aware Processor Chain

1. Add internal transport types:
   - `SpannedInlineEvent<'a>`
   - `SpannedEventProvenance`
2. Add a `SpanningAdapter` from `(Event, Range<usize>)` into spanned inline
   events.
3. Implement `SpannedInlineStyleProcessor` with exact UTF-8 byte ranges for:
   - mark
   - dim
   - escaped delimiters
   - unclosed delimiters
   - nested mark/dim with normal Markdown inline events
4. Implement `SpannedRuleProcessor` for HR-attribute paragraphs, preserving
   parsed vs generated provenance.
5. Fold Darkmatter inline events into:
   - `Span` class `mark`
   - `Span` with dim `Style`
   - `ThematicBreak` with namespaced `darkmatter.hr` hints

Verification:

- Fold tests for every fixture listed in `span-aware-processor-design.md`.
- Legacy renderer tests remain on the old processors and continue passing.

## Phase 5: Parity Harness

1. Expand or add legacy-vs-tree parity fixtures for:
   - prose/headings
   - nested inline styles
   - links/images
   - block quotes/lists
   - tables
   - code blocks
   - Mermaid deterministic modes
   - mark/dim
   - HR attributes
   - raw HTML
   - parser-option-sensitive constructs
2. Use `PipelineResult` so each mismatch can identify fold diagnostics,
   render diagnostics, or output differences.
3. Add a ledger keyed by `(fixture, target, phase, facet)` for accepted
   divergences.
4. Keep semantic assertions for expected formatting differences; reserve byte
   equality for cases where it is a real contract.

Verification:

- `cargo test -p darkmatter --test render_tree_parity`
- New focused parity tests for mark/dim/HR attributes and raw HTML.

## Phase 6: Benchmark Harness

1. Add `darkmatter/lib/benches/migration_parity.rs`.
2. Add the `[[bench]]` entry to `darkmatter/lib/Cargo.toml`.
3. Implement paired Criterion groups:
   - `migration/terminal`
   - `migration/terminal_no_color`
   - `migration/browser`
   - `migration/markdown`
   - `migration/fold_only`
   - `migration/full_pipeline`
4. Use deterministic fixture generators and pinned options from
   `benchmark-harness-shape.md`.
5. Run and record baseline numbers in this feature directory.

Verification:

- `cargo bench -p darkmatter --bench migration_parity`

## Phase 7: Cutover Readiness Review

1. Review parity ledgers target by target.
2. Review benchmark notes for each target.
3. Confirm raw HTML remains escaped by default.
4. Confirm public legacy methods are still legacy-backed unless the target has
   an explicit cutover note.
5. Choose the first public cutover target only after the Browser/HTML parity
   and benchmark notes are acceptable.

Recommended cutover order remains:

1. Browser/HTML
2. MarkdownPlus
3. Portable Markdown
4. Terminal

## Final Verification Gate Before Any Public Cutover

- `cargo test -p darkmatter`
- `cargo test -p renderable`
- `cargo test -p biscuit-terminal`
- `cargo clippy -p darkmatter --all-targets -- -D warnings`
- `cargo bench -p darkmatter --bench migration_parity`

If any public render behavior changes, update the relevant Darkmatter docs,
fixture snapshots, and this feature directory's parity/benchmark notes in the
same change.
