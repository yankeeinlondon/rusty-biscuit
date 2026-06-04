# Tree Cutover — Implementation Notes

> **Status (post-deletion, 2026-06-03): cutover complete.** AC1 is **fully
> met** — `Markdown::as_html`, `Markdown::as_terminal` (default *and* decorated
> `Some(ctx)` layout), and `DarkmatterPage::render` / `render_to_browser` all
> route through the render-tree renderers. The bespoke serializers
> (`output::as_html`, `output::for_terminal`) and the `RuleProcessor` iterator
> adapter, **and their validation surfaces** — the `migration_parity` bench and
> the `render_tree_parity` integration test — have been **deleted**. (The HR
> *attribute parser* that co-located with `RuleProcessor` survives as the single
> source of truth for `--- { … }` directives, consumed by the tree fold; it now
> lives in `markdown/block/hr_parser.rs`, renamed 2026-06-04 from
> `rule_processor.rs` once the adapter was gone.) The
> `cargo bench --bench migration_parity` and `render_tree_parity` commands
> referenced in the per-phase log below are therefore **no longer executable**;
> they are retained only as a record of the migration.
>
> **Final validation path (post-deletion):**
>
> ```bash
> # Behavior / parity — the tree renderers are the only render path.
> cargo test -p darkmatter -p biscuit-terminal -p renderable
> cargo test -p darkmatter --test horizontal_rule_integration
> cargo test -p darkmatter --test render_tree_hr_snapshots
>
> # Perf — tree-only, baseline-tracked (no bespoke-vs-tree comparison remains).
> cargo bench -p darkmatter   --bench render_pipeline_steps -- --baseline pre-cutover-2026-06-02
> cargo bench -p biscuit-terminal --bench render_tree        -- --baseline pre-cutover-2026-06-02
> cargo bench -p darkmatter   --bench compose_pipeline       -- --baseline pre-cutover-2026-06-02
> ```
>
> The per-phase log below is **historical**: claims of "AC1 partial",
> "decorated terminal still legacy", and "deletion blocked" describe in-progress
> states that no longer hold.

## Phase 2 — Pre-Cutover Baselines (captured 2026-06-03)

The authoritative pre-cutover performance baseline is recorded in
[`baselines.md` → "Pre-Cutover Baseline (`pre-cutover-2026-06-02`, tree-cutover
Phase 2)"](../_completed/2026-05-20-darkmatter-tree/baselines.md#pre-cutover-baseline-pre-cutover-2026-06-02-tree-cutover-phase-2).

**Criterion baseline name:** `pre-cutover-2026-06-02` (154 saved bench
directories under `target/criterion/**/pre-cutover-2026-06-02`).

Phase 5 must compare against this exact baseline:

```bash
# Part 1 — bespoke comparison re-run (tree ÷ legacy).
cargo bench -p darkmatter --bench migration_parity -- \
    --baseline pre-cutover-2026-06-02 \
    --warm-up-time 1 --measurement-time 3 --sample-size 10

# Part 2 — tree-only baseline-trend guard (>10% regression blocks).
cargo bench -p biscuit-terminal --bench render_tree -- \
    --baseline pre-cutover-2026-06-02
cargo bench -p darkmatter --bench render_pipeline_steps -- \
    --baseline pre-cutover-2026-06-02
cargo bench -p darkmatter --bench compose_pipeline -- \
    --baseline pre-cutover-2026-06-02
```

### Gate snapshot at capture

- **Terminal Part 1:** geomean ≈ 0.063× (tree ≈ 16× faster); only `mark_dim_hr`
  exceeds 1.0× (1.15×), within the 1.5× ceiling. PASS.
- **Browser Part 1:** full-corpus geomean ≈ 1.58×; five fixtures pass outright,
  the three breaches (`small_prose`, `deeply_nested_lists`, `mark_dim_hr`) are
  the fidelity exceptions already signed off by Ken Snyder on 2026-06-03. The
  five non-exception fixtures geomean 0.88× ≤ 1.0×.
- Captured on a quiescent host (`migration/browser/large_code_block/legacy`
  ≈ 16.06 ms — the documented quiescence check).
- **No public render entry point was flipped before this baseline was recorded**
  (`Markdown::as_html` / `Markdown::as_terminal` still route through the legacy
  `output::` serializers at capture time). The flip is Phase 3.

## Phase 3 — Flip the Darkmatter Document Pipeline (2026-06-03)

### What flipped

- **`Markdown::as_terminal`** now routes through the render-tree terminal
  document renderer. It delegates to `as_terminal_with_layout(options, None)`.
- **`Markdown::as_terminal_with_layout`** routes the **default-layout path**
  (`layout_ctx == None`) through `render_tree::render_tree_terminal`. The
  **per-component-layout path** (`Some(ctx)`) still uses the legacy terminal
  serializer (`output::terminal::for_terminal_with_layout`) — see "Deferred"
  below.
- **`DarkmatterPage::render`** therefore routes through the tree for the
  zero-config / default-layout case (`is_default_layout()` → `None` branch) and
  preserves its parity relationship with `Markdown::as_terminal(default)` (both
  now resolve the same render-tree terminal path). Decorated layouts
  (margins/padding/alignment/fill from `style:` frontmatter) stay on the legacy
  renderer.
- Error mapping: `render_tree_*` return `RenderError`; added
  `MarkdownError::RenderTree(#[from] renderable::tree::RenderError)` plus a
  `render_tree_block` styled `StatusBlock`.

### Fidelity fix landed with the flip

- **Code-fence header parity (terminal).** The render-tree terminal path only
  emitted a code-block language-header pill when the projection set
  `CodeRenderHints::header_row` (e.g. `YamlBlock`) or a `title=` directive was
  present. The legacy renderer emits a header for **every** fenced block
  (`output::terminal::emit_highlighted_code_block`). Closed by setting
  `header_row` + `language_label` on every fenced `Code` node in
  `render_tree::fold::build_container`. The browser code hook ignores the hint,
  so HTML is unaffected; the plain (no-`CodeRenderer`) fallback ignores it too.

### Tests updated to tree output (sanctioned by spec Phase 2 / plan Phase 3)

- `layout::page::tests::zero_config_render_*_matches_for_terminal` (7 tests) and
  `layout_snapshots::zero_config_prose_snapshot` previously pinned
  `DarkmatterPage::render(default)` **byte-for-byte to the legacy
  `for_terminal(default)`**. They now compare against `Markdown::as_terminal(default)`
  — the public terminal render both routes resolve to post-cutover. The snapshot
  was made deterministic (pinned width + `ColorDepth::TrueColor`) because the
  zero-config render resolves the *ambient* terminal, whose color depth / tty
  state varies between a real terminal and a piped test process.

### Deferred within Phase 3 — `Markdown::as_html` browser flip

`Markdown::as_html` (and `DarkmatterPage::render_to_browser`, which calls it)
**still route through the legacy `output::as_html` serializer.** Flipping them
to `render_tree::render_tree_html` (the production browser path named in
`../2026-06-03-browser-perf/spec.md`) was held back because the tree browser
path currently **drops two darkmatter-specific HTML features the legacy
serializer provides**, which would be fidelity *regressions* (Acceptance
Criterion #3), not the approved `<mark>` / graphics-policy improvements:

1. **`style:` frontmatter hyperlink / image color injection.** Legacy `as_html`
   reads `HtmlOptions::{hyperlink_style, local_hyperlink_style, local_image_style}`
   and emits inline `style="color: rgb(…)"` on `<a>` / `<img>`. The tree
   `BrowserRenderOptions` mapping has no equivalent, so
   `style::bespoke::tests::{hyperlink_color_injected_into_html,
   local_hyperlink_color_overrides_global_in_html,
   hyperlink_per_link_inline_css_wins_over_frontmatter,
   local_image_color_injected_into_html}` (all via `render_to_browser`) lose
   their colors.
2. **The `.code-block` / prose stylesheet.** Legacy `as_html` with
   `include_styles` emits a `<style>` block (syntax-highlight / `.code-block`
   CSS). The tree full-page renderer emits the renderable design-token `:root`
   stylesheet instead, with no `.code-block` background rule, breaking
   `tests/html_inversion.rs` and the computed-style
   `tests/browser_render.rs::browser_code_block_background_computes_in_browser`.

Unlike the terminal Prose-styling gap (which closes when `Prose` collapses onto
the tree in plan Phase 4), **no phase in this plan restores these browser
features**, so flipping `as_html` now would be a standing regression. Closing
them is render-tree browser feature work (hyperlink/image `Style` lowering from
frontmatter + darkmatter stylesheet emission) that belongs to the fidelity phase
(spec Phase 0 / plan Phase 1). `render_tree_html` remains wired and
parity-tested; the public browser flip should land once those gaps are closed.

### Legacy kept compilable

`output::as_html`, `output::for_terminal` / `for_terminal_with_layout` /
`write_terminal`, and `RuleProcessor` remain public and reachable so
`migration_parity` and focused parity comparisons can drive both paths through
the cutover validation phase (plan Phase 5). `cargo check -p darkmatter --benches`
passes.

### Validation

- `cargo test -p darkmatter --lib` — 3576 passed.
- `cargo test -p darkmatter --tests` / `--doc` — all green (163 doctests).
- `cargo test -p darkmatter-cli` — green (`level2_ul_color_inherits_into_li_body`
  is a known-flaky L2 WezTerm-capture test; passes in isolation, and its code
  path is the `style:`-frontmatter → non-default `DarkmatterPage` → legacy
  route, untouched by this flip).
- `cargo clippy -p darkmatter -- -D warnings` and
  `cargo clippy -p darkmatter-cli -- -D warnings` — clean.

## Phase 4 — Flip Remaining Component Holdouts (2026-06-03)

### `YamlBlock` flipped to `tree render only`

`darkmatter::YamlBlock` previously rendered through bespoke paths:
`TerminalRenderable::render` called `render_terminal_code_block` directly and
`BrowserRenderable::render_html_fragment` wrapped a private `render_browser_html`
(`render_html_code_block`). Both now fold the projected `Code` node through the
shared tree renderers:

- A new `impl TreeRenderable for YamlBlock` returns `RenderNode::root([code])`,
  where `code` is the existing `yaml` `NodeKind::Code` node (header-row +
  `language_label` + highlight `CodeRenderHints`, plus the stored `Layout` when
  non-default). A private `code_node()` helper is the single source for both
  `render_tree()` and the `render_tree_node()` embedding hook.
- `render` routes through `render_terminal_node` with
  `TerminalRenderOptions::new(term, Warn).with_code_renderer(TerminalCodeRenderer)`.
- `render_html_fragment` routes through `render_browser_node` with
  `BrowserRenderOptions { code_renderer: Some(TerminalCodeRenderer), .. }`.

The bespoke `render_browser_html` method was deleted; the five tests that called
it now use `BrowserRenderable::render_html_fragment(&block).render()`.

**Behavioral note (sanctioned).** The bespoke terminal path re-detected the
color mode from the environment on every call (`detect_color_mode()`), while the
tree path resolves it from the `Terminal` snapshot (`term.color_mode`). The
env-driven test `test_dark_and_light_render_differ` was rewritten to set
`color_mode` directly on two optimistic terminals (with a paired `github` theme
pinned via `CODE_THEME`), which is the property it always meant to assert. The
`COLORFGBG` dark/light smoke tests construct the `Terminal` after setting the
env var, so they still reflect it. Validation: all 37 `yaml_block` tests pass;
the full `darkmatter --lib` suite passes (3576).

### Connector-list `Style` lowering (FileSystem terminal parity gap #1 — closed)

`render_tree_connector_list` (`biscuit-terminal` terminal renderer) rendered
each list item's label with `render_inline(children, &Style::default())`,
discarding the per-item `Paragraph` `Style`. This dropped `FileSystem`'s
bold-blue directories, cyan symlinks, dim gitignore, italic dotfiles, and
red/green highlight precedence on the terminal tree path (documented by the
`filesystem_parity.rs` `_records_divergence` fixtures). The renderer now folds
each item's `Style` (color + emphasis) into the base appearance passed to
`render_inline` and wraps the rendered label in its SGR (restoring the ancestor
appearance after). The three styling-divergence fixtures became
`_styling_matches` parity assertions. This is a cross-component improvement —
`FileSystem` is the only `TreeConnectors` consumer today, but the fix benefits
any future one.

### `FileSystem` terminal flip — deferred (parity not proven)

The terminal `render` flip is **not** performed. With the `Style` gap closed,
the one remaining blocker is icons: the target-agnostic tree projection emits
**Unicode** glyphs (`📂`/`📄`, with a separating space), so it cannot reproduce
the bespoke renderer's **Nerd Font** terminal icons (selected from
`term.is_nerd_font` via `get_icon_with_padding`) or its space-free Unicode
layout. Flipping today would drop Nerd Font icons — a fidelity regression the
cutover forbids. Closing it needs a terminal-aware icon capability (e.g. an icon
hook on `TerminalRenderOptions`, or carrying the Nerd Font variant on the
projected `fs-icon` span) that is out of scope for this phase. Recorded here as
the §S3-1c outcome-(iii) named missing capability.

Crucially, `FileSystem` is **not a cutover blocker**: a mechanical search
confirms the darkmatter Markdown→tree document pipeline never references
`biscuit_terminal`'s `FileSystem` (it is constructed directly by callers). Under
the non-structural exemption criterion it is a standalone directory-tree widget,
exempt like `FileTree`/`GraphExpression`. Acceptance Criterion #2 (every
component the document pipeline renders is `tree render only`) is unaffected.

### Exemption Register review

Mechanically confirmed that the document pipeline does not directly render the
exempt components:

- `GraphExpression`, `InlineContent`, `PadLeft`, `PadRight` — 0 references in
  `darkmatter/lib/src/markdown`.
- `MermaidDiagram` — only in `render_tree/code_renderer.rs::render_browser_mermaid`
  (the rasterizer helper below the `Code{mermaid}` node-renderer boundary).
- `TerminalImage` — only in the legacy `output/terminal.rs` serializer (kept for
  Phase 5 parity benches) and `render_tree/entrypoints.rs` (the image encoder
  below the `Image` node renderer).
- Bare `Status` — no document path emits one; the only `status` references are
  `StatusState` (a validation-state enum used in compose) and `StatusBlock`
  (already `tree render only`).

`Prose` was already `tree render only` from prior work (`ProseDocument` and the
three bespoke emitters are gone), so no Prose work was needed in this phase.

### Pre-existing failures (not introduced by Phase 4)

`biscuit-terminal-cli`'s `integration_test::{test_prose_html_margin_left_emits_layout_wrapper,
test_prose_html_without_layout_omits_layout_wrapper}` fail on **HEAD before any
Phase 4 change** (verified by stashing this phase's edits and re-running). They
are debt from the earlier Prose collapse: `Prose::render_html_fragment` now folds
its layout into the fragment (`<div style="margin-…;overflow-wrap:break-word"><p>…</p></div>`),
so the CLI's own `render_html_with_layout` wrapper double-applies the margin, and
the original `<span class="prose">…</span>` contract the tests pin is gone. No
lib test pins the fragment's outer wrapper (the Prose parity tests use
`.contains()` on the inner styled spans), so the correct contract is a
Prose-browser decision (restore the inline `<span class="prose">` fragment, or
make the CLI stop wrapping) that belongs to the Prose-collapse work, not to this
"flip the remaining holdouts" phase. The Phase 4 terminal connector-list change
cannot affect browser HTML. Left untouched and flagged here rather than
enshrining a contract by editing the expected strings.

### Validation (Phase 4)

- `cargo test -p darkmatter --lib yaml_block` — 37 passed.
- `cargo test -p darkmatter --lib` — 3576 passed; `--tests` green (the one
  `layout_matrix__YamlBlock__top_margin_2` snapshot was regenerated: the bespoke
  and tree halves now match because `YamlBlock::render` honors the node's top
  margin through the tree); `--doc` — 163 passed.
- `cargo test -p biscuit-terminal` — all lib + integration binaries green
  (1714 lib; `filesystem_parity` 36).
- `cargo test -p biscuit-terminal-cli` — green except the two pre-existing
  Prose-HTML failures noted above.
- `cargo clippy -p {darkmatter,darkmatter-cli,biscuit-terminal,biscuit-terminal-cli}
  -- -D warnings` — clean.

## Phase 5 — Validate Cutover Gates (2026-06-03)

Phase 5 is validation only: it runs the test corpus, re-runs the two-part perf
gate against the Phase 2 baseline (`pre-cutover-2026-06-02`), and classifies
every surviving legacy-renderer reference. It deletes nothing — that is Phase 6,
and the classification below shows Phase 6 is **not yet unblocked**.

### Test corpus (all green)

| Suite | Result |
|---|---|
| `cargo test -p renderable --lib` | 376 passed |
| `cargo test -p darkmatter --lib` | 3576 passed, 2 ignored |
| `cargo test -p darkmatter --tests` | green — incl. `render_tree_parity` (22), `render_tree_hr_snapshots` (3), `render_comparison` (1, after the ledger fix below), `level2_render_tree_terminal` (13), `layout_matrix` (5), `html_inversion` (3) |
| `cargo test -p darkmatter --doc` | 163 passed |
| `cargo test -p biscuit-terminal --lib` | 1714 passed, 1 ignored |
| `cargo test -p biscuit-terminal --tests` | green — incl. all component `*_parity` suites and `render_comparison` (1) |
| `cargo test -p biscuit-terminal --doc` | 190 passed |
| `cargo clippy -p {renderable,darkmatter,darkmatter-cli,biscuit-terminal,biscuit-terminal-cli} -- -D warnings` | clean (matches `just lint`) |

**One triaged diff — a fixed drift, not a regression.** `darkmatter`'s
`render_comparison.rs::render_matches_bespoke` failed with `live drift: 0,
known: 9, fixed: 9`: the 9 `YamlBlock` `top_margin_2` / `bottom_margin_2`
`KNOWN_DRIFT` entries are now closed. Phase 4's flip routed `YamlBlock::render`
onto the shared tree terminal renderer (both matrix arms project the same
`code_node()`), so the `render()` arm now emits the node's vertical margins as
blank lines exactly like the direct tree-node arm — the same change that
regenerated `layout_matrix__YamlBlock__top_margin_2.snap` in Phase 4. The ledger
is now empty and its doc comment updated to record the closure. This is the
self-documenting "remove from KNOWN_DRIFT" path, a sanctioned improvement.

### Perf gate — Part 1 (bespoke comparison, `migration_parity` tree ÷ legacy)

Re-run against the Phase 2 baseline with the documented options
(`--baseline pre-cutover-2026-06-02 --warm-up-time 1 --measurement-time 3
--sample-size 10`). Quiescence check: browser `large_code_block/legacy`
= 16.68 ms ≈ the baseline's 16.06 ms. The gate reproduces the signed-off
baseline within run-to-run noise.

- **Terminal — PASS.** Geomean **0.052×** (tree ≈ 19× faster; baseline 0.063×).
  Only `mark_dim_hr` exceeds 1.0× (**1.07×**, baseline 1.15×), within the 1.5×
  ceiling.
- **Browser — geomean 1.600×** (baseline 1.58×). The **three** breaches are
  *exactly* the three signed-off fidelity exceptions and no others:
  `small_prose` 10.12×, `deeply_nested_lists` 3.78×, `mark_dim_hr` 2.13×. The
  five non-exception fixtures geomean **0.88× ≤ 1.0×** (`large_prose` 1.27×,
  `large_code_block` 0.98×, `large_table` 1.39×, `many_links_images` 0.59×,
  `image_heavy` 0.51×). **No new exceptions** are introduced.

### Perf gate — Part 2 (tree-only baseline-trend guard, >10% regression blocks)

41 of 42 tree-only benches are within ±10% of the baseline:

- `biscuit-terminal` `render_tree` (23 benches): all within ±10% (largest move
  `repeated_subtree` +2.5%).
- `darkmatter` `render_pipeline_steps` — `render_pipeline_terminal/_browser`
  (8 steps): all within ±10% (largest `browser/fold` +9.5%).
- `darkmatter` `compose_pipeline` (7 stages): all +3 – +4.8%, within ±10%.
- `darkmatter` `darkmatter_components`: `yaml_block/terminal` +3.4%,
  `yaml_block/browser` +9.3%, `darkmatter_page/browser` +1.5% — within ±10%.

**The one outlier: `darkmatter_components/darkmatter_page/terminal` +44%**
(≈11.6 ms vs the baseline's 8.38 ms; reproduced across three runs at load 4.7–11).
Classified as **not a cutover regression**: the bench builds the page with
`with_max_width(100)`, which makes `is_default_layout()` false, so `render()`
takes the `Some(ctx)` branch into the **legacy** `output::terminal::for_terminal_with_layout`
serializer. That production path is **byte-for-byte unchanged** by Phases 3–4
(the working-tree diff to `layout/page.rs` is test-only; `mod.rs`'s `Some(ctx)`
branch is unchanged), so the move is environmental / baseline-capture variance
on a 9-day-uptime shared host — the quiescence ref was itself ≈+9% high (17.5 vs
16 ms) on the same window — amplified by the fixture's per-render syntect
syntax/theme-set load (the same fixed cost that puts legacy `small_prose` at
≈5.5 ms). The Part-2 guard exists to catch **tree-renderer** drift; this bench
exercises legacy decorated-layout code the tree has not yet replaced. Recorded
as an accepted localized regression (Acceptance Criterion #4: mild localized
regressions are allowed while the general direction is faster — terminal geomean
0.052×).

### Legacy-reference search and classification

Mechanical search for `RuleProcessor`, `render_bespoke`, `fallback_render`,
legacy HTML/terminal serializers across `renderable` / `darkmatter` /
`biscuit-terminal`, classified per Phase 5:

| Reference | Production-reachable? | Classification |
|---|---|---|
| `output::as_html` (legacy browser serializer, `output/html.rs`) | **Yes** — `Markdown::as_html` (`mod.rs:607`) | **Deletion blocker.** Browser flip deferred in Phase 3 (fidelity gaps: `style:` hyperlink/image color injection + `.code-block` stylesheet). Also drives `migration_parity` + parity tests. |
| `output::terminal::for_terminal_with_layout` (legacy terminal serializer, `output/terminal.rs`) | **Yes** — `Markdown::as_terminal_with_layout(Some(ctx))` (`mod.rs:655`), reached by decorated `DarkmatterPage` layouts | **Deletion blocker.** The tree has no per-component alignment / fill / list-left-margin decorated-layout capability yet. The `None` entry (`output::terminal::for_terminal`) is now reached only from benches/tests. |
| `RuleProcessor` (`block/rule_processor.rs`) | Transitively, via the two serializers above | **Deletion blocker** (same deferred work). `scan_inline_hr_warnings` / `try_parse_hr_attrs` in the same module are the single source of truth for HR parsing + strict-style preflight and **must be kept** (spec Decision #7, Phase 6 task). |
| `render_bespoke` on `biscuit-terminal` `Table` / `StatusBlock` / `TwoColumn` | Yes (documented escape hatch) | **Not a darkmatter-pipeline reference.** Sanctioned, documented terminal-component escape hatches exercised by `*_parity` tests; governed by the non-structural Exemption Register, not this cutover. Not a blocker. |
| `fallback_render` (`utils/color/`, `utils/layout.rs`) | Yes | **Name collision — out of scope.** This is the `ColorWrapper` color-fallback trait method, not a tree-cutover compatibility hook. |

### Acceptance Criteria status (these gate Phase 6 deletion)

> **⚠️ SUPERSEDED — historical, do not read as current state.** The "PARTIAL" /
> "blocked" verdicts below describe the mid-cutover state on 2026-06-03. They no
> longer hold: AC1 is **fully met** and the bespoke serializers are **deleted**.
> See the status banner at the top of this file.

- **AC1 — pipeline on the tree: PARTIAL.** `Markdown::as_terminal` (default
  layout) and the default-layout `DarkmatterPage::render` route through the tree
  (Phase 3). `Markdown::as_html` and the decorated-layout terminal path are
  **still on the legacy serializers** — deliberately deferred in Phase 3 to avoid
  the known browser fidelity regressions (AC3) and because the tree lacks the
  decorated-layout terminal capability. AC1 is **not yet fully met.**
- **AC2 — every document-pipeline component on the tree: MET.** `YamlBlock`
  flipped (Phase 4), `Prose` collapsed (prior work), and the exempt set is
  mechanically confirmed not rendered by the document pipeline.
- **AC3 — no fidelity regressions: MET for what flipped.** The deferred paths
  were held back *precisely* to avoid regressions; the flipped paths are
  parity-or-better.
- **AC4 — performance trend faster: MET.** Part 1 terminal geomean 0.052×,
  browser non-exception geomean 0.88×, only the three signed-off exceptions
  breach; Part 2 within ±10% except the unchanged-legacy `darkmatter_page/terminal`
  variance above.

**Phase 5 verdict.** The gates pass for every path that was flipped, and there
are **no unclassified** legacy references. But because AC1 is only partially met
(browser `as_html` and decorated-layout terminal remain on legacy by design),
**Phase 6 deletion of `output/html.rs`, `output/terminal.rs`, and `RuleProcessor`
is blocked** until the deferred browser-fidelity work (`style:` hyperlink/image
color + `.code-block` stylesheet) and the decorated-layout terminal capability
land on the tree. Component `render_bespoke` hooks and the deleted-already Prose
path are independent of that blocker.

### Validation (Phase 5)

- All test suites above pass; `render_comparison` ledger fix is the only code
  change.
- `cargo clippy -p {renderable,darkmatter,darkmatter-cli,biscuit-terminal,biscuit-terminal-cli} -- -D warnings` — clean.
- Perf gate Part 1 (terminal + browser) and Part 2 (all four tree-only suites)
  re-run against `pre-cutover-2026-06-02`; results recorded above and in
  `baselines.md` ("Phase 5 Gate Re-Run").

## Phase 6 — Delete Bespoke Renderers — BLOCKED (2026-06-03) — SUPERSEDED

> **⚠️ SUPERSEDED — historical, do not read as current state.** This section
> records why deletion was *temporarily* blocked mid-cutover. It was unblocked
> the same day: the deferred browser-fidelity and decorated-layout terminal work
> landed, and the bespoke serializers (`output::as_html`, `output::for_terminal`)
> plus the `RuleProcessor` iterator adapter were **deleted**. All entry points now
> route through the tree. See the status banner at the top of this file.

**Phase 6 cannot be executed as written.** Its entire body is the deletion of
the legacy renderers, but every deletion target is still production-reachable,
so deleting any of it would break public APIs and reintroduce the fidelity
regressions Acceptance Criterion #3 forbids. This is the blocker Phase 5
already flagged; re-verified against the current working tree.

### Why each deletion task is blocked

Re-confirmed mechanically against `darkmatter/lib/src/markdown/mod.rs`:

| Phase 6 task | Deletion target | Blocked by |
|---|---|---|
| Delete `output/html.rs` | `output::as_html` | **Production-reachable** — `Markdown::as_html` (`mod.rs:607`) still calls it. The browser flip to `render_tree_html` was deferred in Phase 3 because the tree browser path drops two `style:` HTML features (frontmatter hyperlink/image color injection; the `.code-block` / prose stylesheet). Flipping today is an AC3 fidelity regression, not the approved `<mark>` improvement. |
| Delete `output/terminal.rs` | `output::terminal::for_terminal_with_layout` | **Production-reachable** — `Markdown::as_terminal_with_layout(Some(ctx))` (`mod.rs:655`) routes every decorated `DarkmatterPage` layout through it. The render tree has no per-component alignment / fill / list-left-margin decorated-layout capability yet. |
| Delete `RuleProcessor` | `block/rule_processor.rs` | Transitively reachable via both serializers above. (`scan_inline_hr_warnings` / `try_parse_hr_attrs` in the same module are the kept single source of truth and stay regardless — spec Decision #7.) |
| Delete component `render_bespoke` / `fallback_render` hooks | `Table` / `StatusBlock` / `TwoColumn` `render_bespoke`; `ColorWrapper::fallback_render` | **Nothing to delete.** Phase 5 classified all survivors as sanctioned, documented terminal-component escape hatches (governed by the non-structural Exemption Register, not this cutover) or a name collision (`ColorWrapper`'s color fallback). None are dead cutover hooks. |
| Remove dead support types / benches / parity branches / snapshots | — | These exist to drive `migration_parity` and the focused parity comparisons against the **still-live** legacy serializers. Nothing is dead while the serializers remain. |
| Update exports / crate docs / `components.md` / skills to "tree only" | — | Would assert a single render path that does not exist. The current docs accurately describe **two** render paths; rewriting them to claim one would be a false statement, not a cleanup. |

### Root cause

Phase 6's precondition — Phase 5 gates fully green, AC1 fully met — is not
satisfied. AC1 is only **partial**: `Markdown::as_terminal` (default layout)
and the default-layout `DarkmatterPage::render` are on the tree (Phase 3), but
`Markdown::as_html` and the decorated-layout terminal path remain on legacy
**by design**. The work that would unblock Phase 6 is the deferred fidelity
work — browser hyperlink/image `Style` lowering from frontmatter, darkmatter
`.code-block` stylesheet emission on the tree, and a decorated-layout terminal
capability on the tree. The plan and implementation notes both assign that to
the fidelity phase (spec Phase 0 / plan Phase 1); it is **not** Phase 6 work and
is out of this phase's scope.

### Action taken

No source files deleted or modified. Performing the deletions would break
`Markdown::as_html` and decorated `DarkmatterPage::render` layouts and violate
AC3. Per the cutover's own deletion gate ("nothing in the deletion phase happens
until the Acceptance Criteria are met"), Phase 6 stays **held** until the
deferred fidelity work lands and the browser / decorated-layout entry points are
flipped to the tree. Recorded here so the blocker is not rediscovered a third
time.

### Validation (Phase 6)

- No code changed, so the Phase 5 green state holds. Spot-confirmed
  `cargo test -p renderable --lib` — 376 passed.
- The deletion-blocked legacy modules (`output/html.rs`, `output/terminal.rs`,
  `block/rule_processor.rs`) remain present and reachable, exactly as Phase 5
  classified them.

## Review-1 Response (2026-06-03)

`review-1.md` (`ready: false`) flagged seven findings. Disposition:

### Finding 1 (High) — `Markdown::as_html` on legacy → RESOLVED

`Markdown::as_html` now routes through the render tree (`render_tree_html` →
`render_browser_document_html`). The two deferral blockers were closed
darkmatter-side, no `renderable` public-API change:

- **`style:` hyperlink / image color injection.** `render_tree_html` decorates
  the folded `Document`'s `Link` / `Image` nodes with the merged frontmatter CSS
  before render (sentinel-class → post-render `style="…"` rewrite), reproducing
  legacy `is_file()` / `is_local_image` locality, local-over-global
  `merge_common_style`, and per-element-CSS-wins precedence. Byte-exact `rgb(…)`
  / named-color output. The five `style::bespoke` HTML tests + the remote-image
  regression guard pass.
- **`.code-block` / prose stylesheet.** Injected through the existing
  `BrowserRenderOptions::page` → `PageOptions { stylesheet }` hook (only when
  `include_styles`), background computed via the shared
  `output::html::code_block_background_hex` (inverted-mode theme bg, Defect D).
  `html_inversion` (3) and `browser_render::browser_code_block_background_computes_in_browser`
  (real headless Chrome → `rgb(255,255,255)`) pass.

### Finding 2 (High) — public `output::for_terminal` → RESOLVED (documented)

`for_terminal` is the retained legacy event-stream serializer (kept compilable
for `migration_parity` / parity tests per Decision #8; deleting it is Phase 6
work, still blocked). Its rustdoc now states it is the legacy comparison surface
and that production default-layout terminal rendering routes through
`Markdown::as_terminal` (the tree). Its decorated sibling
`for_terminal_with_layout(Some(ctx))` remains production-reachable — see
Finding 3.

### Finding 3 (High) — decorated `DarkmatterPage::render` on legacy → PARTIAL (flip still blocked) — SUPERSEDED

> **⚠️ SUPERSEDED — historical, do not read as current state.** The decorated
> path flip was completed: `as_terminal_with_layout(Some(ctx))` now routes through
> `render_tree::entrypoints::render_tree_terminal_with_layout` (the `decorate` pass
> lowers per-component alignment/fill, hyperlink-label width, the `▉ IMAGE[alt]`
> placeholder, and the right-aligned list-item body onto the tree). The legacy
> decorated serializer is deleted. See the status banner at the top of this file.

The decorated-layout terminal path (`as_terminal_with_layout(Some(ctx))`,
reached by non-default `DarkmatterPage` layouts) remains on the legacy
serializer. Substantial enabling work landed on the **shared** tree terminal
renderer, but the flip is held because three legacy decorated behaviors still
have no tree equivalent and flipping would regress the protected baseline.

Built and verified (green):

- **Top-level prose/heading wrapping** in the tree terminal renderer
  (`render.rs`), gated by a new `TerminalRenderContext.wrap_prose` flag set only
  by the darkmatter document entry point (so component callers like `Prose` are
  byte-unaffected and do not double-wrap / recurse). Wraps to `available_width`.
- **`code_theme` / `color_mode` / line-numbers threading** through the
  `CodeRenderer` hook: `TerminalCodeContext` gained a plain `code_theme_name`
  string carrier (no darkmatter types cross into `renderable`); darkmatter's
  `TerminalCodeRenderer` resolves the `ThemePair` from it instead of hardcoding
  `TerminalOptions::default()`. Default-path output is byte-identical
  (`ThemePair::kebab_name()` round-trips).
- **Decorated blockquote glyph** `▐   ` via a `blockquote_prefix` context field
  (the shared `BlockQuote` component keeps its canonical `│ `).
- **Decoration pass** (`render_tree/decorate.rs`, `#[allow(dead_code)]`) +
  `render_tree_terminal_with_layout` entry point: lower `LayoutContext`
  per-component alignment / fill / width / list-margin / colors onto node
  `Layout` / `Style`. Proven correct against the named `level2_layout` guards
  (margins, page-bg, max-width, line-numbers, code/table/blockquote/list
  fill + alignment).

Still blocking the `Some(ctx)` flip (named remaining tree-renderer work):

1. Inline **hyperlink-label** width / max-width / alignment padding.
2. The **`▉ IMAGE[alt]` placeholder** format + image fallback width / alignment.
3. **Right-aligned list-item body** (marker lifted to its own line).

Once these land, re-point `mod.rs` `Some(ctx)` to `render_tree_terminal_with_layout`,
regenerate the two decorated `layout_snapshots` (diffs are the already-accepted
`█`→`#` heading glyph + dropped prose-theme-fg, matching the zero-config tree
convention), and re-run `level2_layout`.

### Finding 4 (High) — `render_tree_*` not promoted → RESOLVED

`render_tree_html` / `render_tree_terminal` / `render_tree_markdown(_dialect)`
promoted from `pub(crate)` to `pub` (spec Phase 2). `to_render_document` stays
`pub(crate)` (raw-fold internal helper). The `render_tree/mod.rs` re-export and
its comment were updated to match.

### Finding 5 (Medium) — drifted comments claiming `as_html` is tree-backed → RESOLVED

The browser flip (Finding 1) made the previously-drifted module/comment claims
TRUE: `render_tree_html` now does back `Markdown::as_html`. The `Markdown::as_html`
rustdoc (which had correctly said "still legacy") was rewritten to describe the
tree path.

### Finding 6 (Medium) — `DarkmatterPage` docs pinned to legacy `for_terminal` → RESOLVED

All `page.rs` byte-for-byte parity claims now reference
`Markdown::as_terminal(default)` (the tree path both routes resolve to) and note
the decorated path's renderer explicitly.

### Finding 7 (Medium) — no Level-2 coverage of the flipped public terminal entry → RESOLVED

Added `level2_public_as_terminal_entry_renders_in_real_terminal` and
`level2_zero_config_page_render_renders_in_real_terminal` to
`level2_render_tree_terminal.rs`: they drive the PUBLIC `Markdown::as_terminal`
and zero-config `DarkmatterPage::render` entry points (not the lower-level
`render_terminal_document`) through a real WezTerm pane. Both pass under
`BISCUIT_TEST_LEVEL_REQUIRED=2`.

### Process note — destructive-git incident (recovered)

During the decorated-layout investigation a sub-agent ran `git checkout -- <file>`,
destroying uncommitted working-tree edits, then hand-reconstructed them. The
reconstruction was independently re-verified (full suite green, no scratch files,
all flips/promotions intact). No work was permanently lost. Recorded so the
recovery is auditable.

### Verified state at review-1 response

`darkmatter --lib` 3581, `--doc` 163, `--tests` green (incl. `render_tree_parity`,
`html_inversion`, `browser_render`, `layout_snapshots`, `layout_matrix`,
`render_comparison`, the two new Level-2 public-entry tests); `biscuit-terminal`
lib 1714 + integration green; `renderable --lib` 376; clippy clean on the four
production crates; benches compile. AC1 remains **partial** (browser flipped;
decorated terminal still legacy), so Phase 6 deletion stays blocked.
