# Diagram Components

`biscuit_visualized::components` (feature `terminal`) holds the diagram
components. They decide what a diagram shows and render it for the terminal,
the browser, and the render tree; `biscuit-terminal` only puts the pixels on
screen (`TerminalImage`). The `terminal` feature depends on
`biscuit-terminal` (with `image`) and `renderable`; `biscuit-terminal`
never depends on this crate, so never add a re-export of these types there.

| Component | Module | Wraps | Description |
|-----------|--------|-------|-------------|
| `MermaidDiagram` | `components/mermaid.rs` | `mermaid::MermaidDiagram` | Mermaid diagram as a terminal image; defaults to `ImageWidth::Scale(1.0)` (body text one line tall), measured from the SVG's root `viewBox` (`natural_size`) before rasterizing; `with_time_budget` passes through to the library (default 2 s); falls back to the fenced source on any render failure, a time-budget overrun included (logged as a warning) |
| `GraphExpression` | `components/graph_expression.rs` | `graph::GraphDiagram` | Expression or DOT graph as a terminal image; also `BrowserRenderable` |
| `GitGraph` | `components/git_graph.rs` | `components::MermaidDiagram` | Typed git topology as a Mermaid `gitGraph`: owns the lane/tag rule, merges, the no-substitution rule, and fits the terminal by trimming commits and lanes (see below) |
| `TerminalDiagramRenderer` | `components/render_hook.rs` | `components::MermaidDiagram` | The `biscuit_terminal::render_tree::DiagramRenderer` hook that draws Mermaid fences in a render tree |

## Display size and PNG hand-off

`MermaidDiagram` and `GraphExpression` (and `GitGraph`, through
`MermaidDiagram`) rasterize once at the final display size, via
`components/display.rs`: `display_pixels(columns, cell)` gives width =
resolved columns × cell width and `row_height` = cell height (measured, else
`CellSize::FALLBACK` 8x16; never zero), and `display_image(path, columns)`
wraps the cached PNG in `EncodedPng` for
`TerminalImage::from_encoded_png(..).with_width(ImageWidth::Characters(columns))`.
Kitty and iTerm2 get the rasterizer's exact bytes, and the reserved rows
match the padded height. The displayed columns are the ones the component
resolved (margins included); never re-resolve a percent width against the
full terminal width. Size is measured on every render, so a resize gets a new
raster (cache keys differ by geometry).

Palette: `components::mermaid::MermaidDiagram::new` (and the crate-private
`MermaidRenderer::for_terminal`) style the diagram with `terminal_palette()` =
`Palette::for_terminal(&TerminalColors::detect(), terminal_is_dark())`: the
terminal's background/foreground and ANSI 1–6 accents, background-derived
accents for unanswered slots, the fixed `Dark`/`Default` palette when stdout
is not a terminal or `CI` is set (so tests are deterministic). `with_theme`
switches to that theme's fixed palette, `with_palette` to any. `GitGraph`
renders through the component (terminal palette unless `with_theme`); its
`plan` measures with `with_theme(terminal_theme())`, which is safe because
palettes set colors only, never geometry.

Background: `with_background(Background)` on both; `with_transparent_background(bool)`
remains (`false` = `Background::Theme`). Terminal defaults are
`Transparent` (Kitty/iTerm2 composite over the terminal); `inverted(..)` uses
the opaque theme background. On a Sixel route (no transparency),
`components/display.rs::display_background` swaps a `Transparent` request for
`Solid` = the terminal's measured background (`SixelBackground` `Measured`,
an OSC 11 reply). Without one (none, or a `COLORFGBG`/per-app estimate such
as `#1e1e1e` under tmux) it stays `Transparent`: the Sixel encoder leaves
alpha-0 pixels unset and blends only antialiased edges onto the estimate, so
no theme or guessed box is painted. Explicit `Theme`/`Solid` are kept. The
cache keys on background, so Sixel and Kitty rasters are separate entries;
`TerminalImage` then decodes the PNG and Sixel-encodes it (Kitty/iTerm2 get
the PNG bytes unchanged). `MermaidRenderer::render_to_cached_png_at_width`
(crate-private) takes `(target_width, row_height)`;
`GraphExpression::render_to_cached_png_at_width(target_width)` is unchanged
(no row padding).

Two `MermaidDiagram` types exist on purpose: `mermaid::MermaidDiagram` makes
SVG/PNG artifacts and knows nothing about terminals;
`components::MermaidDiagram` is the terminal component built on it. Import the
component from `components`.

## Render-tree hook

Install `TerminalDiagramRenderer` with
`TerminalRenderOptions::with_diagram_renderer(Rc::new(TerminalDiagramRenderer))`.
At `GraphicsMode::Rich` it answers `DiagramOutcome::Rendered` with exactly
`MermaidDiagram::new(source).try_render(terminal)`, with the terminal's
`fixed_width` set to `DiagramRenderRequest::available_width` (the fence's
columns: a list item's or quote's inner width). Below `Rich` (an `Automatic`
render at `Vector`) it returns `DiagramOutcome::Declined`, so the fence stays
code and never carries an image. A fragment it returns (`DiagramOutcome::Nodes`)
is rendered by biscuit-terminal with promotion off, so it cannot re-promote its
own fence. When and how
biscuit-terminal consults it (and the strictness rules on failure) is in the
`biscuit-terminal` skill's `diagrams.md`. Darkmatter installs it in
`markdown/render_tree/entrypoints.rs`; consumers: `bt` (diagram commands),
darkmatter, `sniff-cli` (`repo deps --ui`), `worktree-cli` (`wt list` graph).

## Tests

- Unit tests live beside the components (`components/git_graph/tests.rs`, and
  the `#[cfg(test)]` modules).
- The `l1` integration binary (`tests/l1/`, `required-features =
  ["terminal"]`, `autotests = false`) holds the parity tests, the diagram rows
  of the layout matrix (and their insta snapshots), and the real-hook tests.
  It includes biscuit-terminal's `tests/l1/parity_helpers.rs` and
  `tests/layout_matrix_support/mod.rs` through `#[path]`; both are declared in
  `[package.metadata.ci.tests] source-inputs` and spelled in a test with
  `include_str!` so CI's test-input index sees them. Keep the diagram rows'
  checks in `layout_matrix.rs` identical to biscuit-terminal's.
- Run with `just test` (all features) from `biscuit-visualized/`.

## GitGraph

Full contract: `biscuit-visualized/docs/components/git_graph.md`. The caller runs git; the component never does.

- `GraphLine::with_tip(sha)` is the branch's own tip. A line without commits is labeled there. `tip()` falls back to the newest drawn commit, **never** `fork_sha`. `forked_at` unset (`fork_sha == None`) means an unknown connection, not "tip here".
- `GraphLine::with_merge(source, destination)` appends a `LaneMerge` (oldest first; `merges` field). Every merge is drawn from its own source: it emits `merge <lane> id: "…" tag: "…"` at the destination commit on whatever lane draws it, only when the source was already emitted and the destination lane has a head (Merman, like Mermaid.js, rejects a merge into a lane with no commit). A lane pauses after a mid-lane source (and the lanes forked at it) until the destination is emitted, then resumes after the `merge` with `checkout <source>`; paused lanes left over (a cycle) have their blocking merge dropped and resume. Merman keeps both parents of the labeled merge (no repair). Siblings hanging from one commit are reordered so a lane whose merges land on a sibling's subtree is emitted first (`emit_merged_lanes_first`). A second merge into one commit is not drawn; every undrawn merge sets `incomplete`.
- **No substitution:** a lane whose fork is undrawn or unknown is drawn unconnected (declared `branch` before the root lane's first commit), never from another lane's start. An undrawn tag, label, or merge destination is left out. Each sets `GitGraphPlan::incomplete`, as does the caller's `with_incomplete_history()`, and renders the dim `INCOMPLETE_HISTORY_NOTE` ("Some history is not shown") after the hidden-lanes note. Tags of lanes the height cap hid are not counted twice.
- Trimming (`trim_one_commit`) never folds lane tips, forks, merge sources and destinations, or tagged commits. The height cap (`fit_lanes`) keeps a lane's ancestors: the lanes holding its fork commit and every merge destination, and its parent's lane.
- `biscuit-visualized` writes a wider gitGraph `commitSpacing` (default 50) only for colliding tags (different commits, overlapping rows) and only to a one-em gap, so a single long label (e.g. `main`/`origin/main` stacked on one tip) keeps the default step and trimming works normally; a graph with colliding tags is widened everywhere and may still need to shrink. Assert the step against `biscuit_visualized::mermaid::default_gitgraph_commit_step()`. Prove tag placement with `biscuit_visualized::mermaid::MermaidDiagram::gitgraph_geometry()` (`tag_overlaps()`), never with Mermaid text.
