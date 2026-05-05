# Data Visualization Review

Reviewed on 2026-03-28 against the current `biscuit-terminal` and `biscuit-visualized` implementations, plus the data-visualization plan and tech design.

## Validation

- `just test` in `biscuit-visualized`: passed
- `just test` in `biscuit-terminal`: passed
- Direct CLI probe: `cargo run -q -p biscuit-terminal-cli -- graph-expression --meta 'a -> b'` produced no stderr metadata

## Highest-Signal Findings

### 1. `bt graph-expression --meta` is accepted but not implemented

- `biscuit-terminal/cli/src/commands.rs:1383-1458` takes `_meta: bool` and never uses it.
- The Mermaid commands go through `display_mermaid(...)` and emit `RenderMeta`; `graph-expression` bypasses that path and just `print!`s `graph.display(&terminal)`.
- This is a shipped behavior bug, not just a missing test. I verified it directly: `--meta` produces no stderr output.

Recommendation:

- Implement the shared diagram display helper that the design called for, or add a graph-specific equivalent that emits the same metadata contract as Mermaid.
- Add a CLI integration test for `bt graph-expression --meta` so this cannot regress again.

### 2. `--title` for `state-diagram` and `erd` is only echoed in JSON, not rendered

- `biscuit-terminal/cli/src/commands.rs:1341-1343` explicitly skips adding the state-diagram title to the Mermaid source.
- `biscuit-terminal/cli/src/commands.rs:1528-1533` explicitly skips adding the ERD title to the Mermaid source.
- In both cases the title still appears in `--json`, which makes the flag look implemented even though the rendered diagram never sees it.

Recommendation:

- Either render titles consistently, likely via frontmatter or another Mermaid-supported mechanism, or remove the flag from the affected commands until it is real.
- Add tests that assert the title affects the emitted instructions, not just the JSON wrapper.

### 3. Graph `--inverse` is only partially implemented

- `biscuit-terminal/cli/src/commands.rs:1430-1438` handles inverse mode by choosing `GraphExpression::parse(...)` and then only toggling transparency off.
- `biscuit-terminal/lib/src/components/graph_expression.rs:102-115` creates that non-terminal graph with no color theme.
- `biscuit-visualized/src/src/graph/render.rs:458-475` renders opaque graphs by injecting a hard-coded white background.

Impact:

- On dark terminals this mostly works by accident: white background plus default dark text is visually inverse enough.
- On light terminals it is not actually inverse; it is just the normal light rendering with an opaque white box.

Recommendation:

- Give graph rendering the same explicit “terminal theme” vs “inverse theme” treatment Mermaid already has.
- Replace the boolean white-or-transparent background behavior with a graph surface theme that controls both foreground and background colors.

### 4. Mixed directed and undirected graph-expression syntax is parsed, but rendered incorrectly

- `biscuit-visualized/src/src/graph/expression.rs` accepts both `->` and `--` in the same parsed expression.
- `biscuit-visualized/src/src/graph/render.rs:272-280` decides whether the graph is directed from only the first edge.
- `biscuit-visualized/src/src/graph/dot.rs:176-200` then emits every edge with one operator.

Example:

- `a -> b; c -- d` parses successfully, but the emitted DOT will coerce all edges to the first edge kind.

Recommendation:

- Reject mixed edge kinds up front with a structured `GraphError`, or extend the model so mixed syntax is represented honestly.
- Add unit coverage for this case either way.

### 5. The planned shared graph/Mermaid display path never landed

- The design explicitly called for a shared CLI display helper so Mermaid and graph rendering would share width/layout/meta handling.
- Current state:
  - Mermaid uses `display_mermaid(...)`
  - Graph uses a separate ad hoc path in `render_graph_expression(...)`
- The missing shared path is the root cause of the graph `--meta` bug and the general behavior mismatch between Mermaid and graph commands.

Recommendation:

- Finish the shared display abstraction, or add a graph-specific fallible render result analogous to `MermaidRenderResult`.
- This will improve ergonomics and remove duplicated command behavior.

## Test Coverage Gaps

### Missing tests in `biscuit-terminal`

- No CLI test for `bt graph-expression --meta`.
  Evidence: `biscuit-terminal/cli/tests/integration_test.rs:1224-1309` covers JSON and fallback behavior, but not metadata.
- No test proving `--title` changes rendered instructions for `state-diagram` or `erd`.
  Current tests only assert that the JSON response echoes the title.
- No test for graph inverse behavior.
  There is currently nothing that would catch the “white box but not truly inverse” behavior on light terminals.
- No test for mixed `->` / `--` graph-expression input.

### Adapter coverage is still light relative to the backend split

- `biscuit-visualized` now has strong backend coverage, including cache separation and actual SVG/PNG effects.
- `biscuit-terminal` adapter coverage is much thinner:
  - `biscuit-terminal/lib/src/components/mermaid.rs` mostly tests builder/fallback basics.
  - `biscuit-terminal/lib/src/components/graph_expression.rs` mostly tests parsing and fallback basics.
- The terminal package should still have focused tests for adapter-only behavior:
  - metadata emission
  - width/layout application
  - terminal-aware defaults
  - fallback behavior
  - flag-to-render-option wiring

## Tests Better Owned by `biscuit-visualized`

- `biscuit-terminal/cli/tests/integration_test.rs:1315-1459` contains a large block of render smoke tests that only assert command success for `bar-chart`, `line-chart`, `timeline`, `state-diagram`, `erd`, `flowchart`, and `git-graph`.
- Those tests are now mostly exercising visualization backend ownership, not terminal-specific ownership.

Recommendation:

- Keep `biscuit-terminal` tests focused on CLI contract and terminal presentation:
  - `--json`
  - `--meta`
  - fallback output on non-image terminals
  - width/layout/margin behavior
  - terminal-aware default theme selection
- Move backend artifact/fidelity/cache assertions into `biscuit-visualized`, where the rendering responsibility now lives.

## Documentation Drift

### `biscuit-terminal` docs are already out of sync with the shipped API

- `biscuit-terminal/docs/components/mermaid_diagram.md:20-24` refers to `result.render_time_ms`, but `MermaidRenderResult` only has `output`, `png_path`, and `cache_hit`.
- `biscuit-terminal/docs/components/graph_expression.md:49-52` documents `--orientation lr`, but the CLI only accepts spelled-out values such as `left-to-right`.

Recommendation:

- Update the component docs in the same change as any follow-up fixes.
- Add doc-tested examples where feasible, or keep examples close to already-tested CLI forms.

### Downstream docs still describe the old world

- `darkmatter/lib/src/mermaid/render_terminal.rs:1-33` and the associated README text still talk about `mmdc`-era behavior even though terminal Mermaid rendering is now pure Rust.

Recommendation:

- Treat downstream doc cleanup as unfinished migration work from the split.

## Ergonomics and Performance Suggestions

### Ergonomics

- Add a graph-side equivalent of `MermaidDiagram::try_render()` that returns rendered output plus metadata.
  This would make CLI wiring simpler and bring the two adapter APIs back into alignment.
- Consider renaming or aliasing the terminal graph wrapper to something explicitly adapter-oriented.
  The design expected a `GraphExpressionRenderer`-style wrapper; the current `GraphExpression` type is usable, but the name blurs the line between parsed graph data and terminal display behavior.

### Performance

- Cache the font database used for rasterization.
  `biscuit-visualized/src/src/raster/png.rs:18-25` calls `load_system_fonts()` on every render. Since both Mermaid PNG and graph PNG flows go through this path, memoizing the font DB should reduce repeated system font scanning.
- After the graph display path is unified, avoid redundant terminal-side work.
  Right now the graph command does not have the same clear render-result handoff Mermaid does, which makes future optimization harder than it needs to be.

## Recommended Follow-Up Order

1. Fix `bt graph-expression --meta`.
2. Make `--title` real for `state-diagram` and `erd`, or remove it.
3. Fix graph inverse mode so it is truly inverse on both light and dark terminals.
4. Decide and enforce policy for mixed `->` and `--` expressions.
5. Move backend-oriented smoke coverage into `biscuit-visualized` and tighten terminal-only coverage in `biscuit-terminal`.
6. Clean up the drifted docs in `biscuit-terminal` and downstream consumers such as `darkmatter`.
