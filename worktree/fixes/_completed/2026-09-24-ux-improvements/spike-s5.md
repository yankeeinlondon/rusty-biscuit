---
spike: S5
date: 2026-09-24
---

# Spike S5: SVG natural size, terminal cell size, and `ImageWidth::Scale`

Sources: `mermaid-rs-renderer-0.3.1`, `0.2.1`, `usvg-0.45.1`, and `terminal_size-0.4.4` from
`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/` (abbreviated `mrr-0.3.1/` etc.). Repo paths
are relative to the worktree root. The lockfile pins `mermaid-rs-renderer` 0.2.1 (`Cargo.lock:7922-7923`).

## 1. `mermaid-rs-renderer` 0.3.1

**Signature** (`mrr-0.3.1/src/render.rs:206-211`):

```rust
pub fn render_svg_with_dimensions(layout: &Layout, theme: &Theme, config: &LayoutConfig,
    dimensions: Option<(f32, f32)>) -> String
```

- **It returns the SVG `String`, not dimensions.** `dimensions` is an optional output-canvas override
  (the `mmdr -w/-H` path, `mrr-0.3.1/src/cli.rs:220`). `render_svg` equals this function with `None` (`render.rs:85-87`).
- It is **not re-exported at the crate root.** Call it as `mermaid_rs_renderer::render::render_svg_with_dimensions`
  (the module is `pub mod render`, `lib.rs:100`). The root re-exports only `SvgDimensions`, `measure_svg_dimensions`, `render_svg`, and `write_output_svg` (`lib.rs:121`).
- **The function we want for sizing is `measure_svg_dimensions(layout, config, dimensions) -> SvgDimensions`**
  (`render.rs:89-93`). The crate-level wrappers are `measure(input, RenderOptions)` and `measure_with_dimensions` (`lib.rs:232-252`). They run parse and layout but skip SVG serialization.
- `SvgDimensions { width, height, viewbox_x, viewbox_y, viewbox_width, viewbox_height }`, all `f32`
  (`render.rs:20-27`).
- **Units:** SVG user units. The root is written as `width="{w}" height="{h}" viewBox="…"` with no unit suffix (`render.rs:291-292`), so 1 unit = 1 px at scale 1.0.
- **Do they equal the root attributes?** Yes, `render_svg_with_dimensions` destructures the result of `measure_svg_dimensions`
  (`render.rs:218-225`), and that result becomes the attributes. Caveats:
  - When `dimensions` is `None` and `use_max_width` is set, the root gets `width="100%"`, no `height`, and a `max-width` style. This applies to C4, gitGraph, mindmap, and pie (`render.rs:251-285`). `GitGraphConfig::default()` sets `use_max_width: true` (`config.rs:247,252`). usvg resolves a percentage root width against the viewBox (`usvg-0.45.1/src/parser/converter.rs:466-470`), so `tree.size().width()` still equals `viewbox_width`.
  - For gitGraph, `width = layout.width` and `viewbox_width = gitgraph.width` (`render.rs:123-136`). Both come from the same `max_x - min_x + 2·padding` (`layout/gitgraph.rs:468`), so they match.
  - A `preferred_aspect_ratio` can pad `width` or `height` beyond the viewBox (`render.rs:180-185`). The default `LayoutConfig` has none. A `Some(dimensions)` override also replaces `width`/`height` while leaving the viewBox unchanged (`render.rs:186-194`).
  - **Recommendation:** treat `viewbox_width` as the natural width, because that is what usvg rasterizes.
- **Theme and config inputs:** `&Theme` and `&LayoutConfig`. There is no separate theme argument for measuring. `measure_svg_dimensions` takes no theme, but the layout it measures was computed with the theme (`compute_layout(graph, theme, config)`, `layout/mod.rs:158`). The measurement must therefore reuse biscuit's theme (`build_theme` plus the init overrides, `biscuit-visualized/src/src/mermaid/render.rs:216-224`).
- **gitGraph font sizes in 0.3.1:**
  - Commit label **10**: confirmed. `commit_label_font_size: 10.0` (`config.rs:280`), emitted at `render.rs:4306`.
  - Branch label **14**: *only for some themes*. `branch_label_font_size` defaults to `0.0` (`config.rs:277`), which falls back to `theme.font_size` (`render.rs:4139-4143`, `layout/gitgraph.rs:35-39`).
  - Body text: `Theme::mermaid_default()` is **16** (`theme.rs:81,89`) and `Theme::modern()` is **14** (`theme.rs:131,139`).
  - Biscuit's Default and Forest themes use `mermaid_default`, so body and branch labels are 16. Dark and Neutral use `modern`, so they are 14 (`biscuit-visualized/src/src/mermaid/render.rs:232`, `:590`, `:618`, `:641`). `point_label_font_size` overrides `theme.font_size` (`render.rs:238-239`).
  - These defaults are unchanged from 0.2.1 (`mrr-0.2.1/src/config.rs:273,276`, `theme.rs:88,138`).

**Symbols biscuit-visualized uses** (`biscuit-visualized/src/src/mermaid/render.rs:213-225`, `:232`, `:399`, `:590`, `:618`, `:641`):
`parse_mermaid`, `LayoutConfig::default`, `compute_layout`, `render_svg`, `Theme::{mermaid_default, modern}`, and
`Theme` fields. Changes from 0.2.1 to 0.3.1:

- Signatures: none changed. `compute_layout` is the same (`mrr-0.2.1/src/layout/mod.rs:150` vs `mrr-0.3.1/src/layout/mod.rs:158`), and `ParseOutput { graph, init_config }` is the same. The `Theme` struct is field-identical (the diff is empty).
- `LayoutConfig` gains `pub timeline: TimelineConfig`. This only breaks struct literals; biscuit uses `::default()`.
- **Behavior:** `parse_mermaid` now errors on input with no diagram header (`mrr-0.3.1/src/parser.rs:60-62`). 0.2.1 silently defaulted to a flowchart (`mrr-0.2.1/src/parser.rs:170-173`). It also validates every `%%{init}%%` line up front and rejects malformed ones (`parser.rs:59,90-107`). Existing fixtures without a header will start failing.
- Dependencies: the default features still include `png`, which pulls resvg/usvg **0.47** (`mrr-0.3.1/Cargo.toml:163,180`; 0.2.1 pulled 0.46). The repo uses resvg 0.45 (`biscuit-visualized/src/Cargo.toml:20`, `biscuit-terminal/lib/Cargo.toml:61`). The duplicate already exists today; consider `default-features = false`. The crate is edition 2024.

## 2. Cell pixel size in `biscuit-terminal`

- **API:** `biscuit_terminal::discovery::fonts::cell_size() -> Option<CellSize>` (`discovery/fonts/window_size.rs:254-263`, re-exported at `discovery/fonts/mod.rs:65`). `CellSize { width: u32, height: u32 }` (`discovery/fonts/types.rs:260-265`). It is wrapped by `Terminal::cell_size()`, which returns the cached value or queries live (`terminal.rs:503-505`); the builder has an override at `terminal.rs:716`. All paths below are under `biscuit-terminal/lib/src/`.
- **Method, in order:**
  1. The `TIOCGWINSZ` ioctl on `/dev/tty`, using `ws_xpixel/ws_col` and `ws_ypixel/ws_row` (`window_size.rs:195-226`). This is atomic and has no timeout.
  2. `CSI 14 t` on `/dev/tty` in raw mode (`window_size.rs:34-80`), divided by the `terminal_width()`/`terminal_height()` grid (`window_size.rs:259-262`). This step is skipped in CI (`:48`).
  - There is no `CSI 16 t` query.
- **Doc drift:** the comment at `terminal.rs:502` says the fallback is "a live terminal query via CSI 14t". The actual order is ioctl first, then CSI 14t.
- **Platforms:** Unix only. Both steps return `None` on non-Unix platforms (`window_size.rs:125-129`, `:228-231`), so **native Windows never has a cell size**. Both are gated by `is_tty()`, which is true when stdout **or** stderr is a terminal (`discovery/detection/dimensions.rs:54-61`).
- **The 8x16 fallback is not centralized.** It is repeated as literals:
  - `unwrap_or((8u32, 16u32))` at `components/terminal_image/protocol.rs:122`, `kitty.rs:24,81`, and `iterm.rs:24,84`
  - width only (`unwrap_or(8u32)`) at `components/mermaid.rs:591` and `components/graph_expression.rs:348`
  - a hard-coded `8` at `components/terminal_image/width.rs:80`
- **Rows and columns:** `terminal_width()`, `terminal_height()`, and `dimensions()` (`dimensions.rs:75-111`) call `terminal_size::terminal_size()`. That checks **stdout, then stderr, then stdin** on Unix (`terminal_size-0.4.4/src/unix.rs:9-19`) and on Windows (`windows.rs:12-26`), and falls back to 80x24. It does **not** use `/dev/tty`. `Terminal::width()` and `height()` add a fixed override (`terminal.rs:476-497`).
- **`ImageWidth` today** (`components/terminal_image/width.rs:44-52`): `Fill | Percent(f32) | Characters(u32)`, default `Percent(0.5)`.
  - The column resolver is `TerminalImage::resolve_dimensions_for(width, layout, term_width)` (`components/terminal_image/mod.rs:423-458`). It computes `available = term_width - left_margin - right_margin`. `Percent` is taken of **term_width**, not `available` (`:439`). Every variant is clamped to `1..=available` (`:442`).
  - The legacy pixel helper `calculate_display_dimensions` uses a fixed 8 px/col and never upscales (`width.rs:72-94`).
  - The iTerm width parameter also matches on the variant (`iterm.rs:31-34`, `:103-106`).
- **`MermaidDiagram` column-to-pixel conversion** (`components/mermaid.rs:578-610`): it takes the columns from `resolve_dimensions_for(&self.width, &self.layout, term.width())` (`:586-590`). Then `target_width_px = image_width_cols × cell_size().width` (default 8) (`:591-592`) is passed to `render_to_cached_png_at_width` (`:333-354`) as `RenderRequest.target_width`.
  - biscuit-visualized builds the SVG first (`biscuit-visualized/src/src/mermaid/render.rs:164`) and then calls `rasterize_svg_to_png(svg, width)` (`:174`).
  - That applies a uniform `scale = target_width / tree.size().width()` (`biscuit-visualized/src/src/raster/png.rs:53-69`).
  - The PNG is shown with `.with_width(self.width.clone())` (`mermaid.rs:598-600`), so the protocol re-resolves the same columns (`protocol.rs:114-126`).

## 3. Inputs for `ImageWidth::Scale(f32)`

| Formula input | Existing source |
|---|---|
| `cell_height_px`, `cell_width_px` | `term.cell_size()` (`terminal.rs:503`), falling back to `(8, 16)` as in `protocol.rs:119-122` |
| `svg_width` (natural width, user units) | **Not available to biscuit-terminal today** (see the gap below) |
| available columns (after margins) | `resolve_dimensions_for(...).available_width` (`terminal_image/mod.rs:433-435`) |
| terminal columns | `term.width()` (`terminal.rs:476-478`), backed by `terminal_size` on stdout, then stderr, then stdin |

- Compute `ppu = scale × cell_h / 16`, then `cols = ceil(svg_w × ppu / cell_w)`, then clamp to `1..=available_width`.
- **Plumbing:**
  - `resolve_dimensions_for` (`mod.rs:437-441`) has no SVG width and receives only `ImageWidth`. Either give the variant a pre-resolved form, or add a `natural_width_px: Option<f32>` parameter.
  - The other match sites need a new arm: `width.rs:82-86`, `iterm.rs:31-34`, `iterm.rs:103-106`, and `parse_width_spec` (`width.rs:120-187`) if a text spec such as `1.0x` is wanted.
- **Where to get `svg_width`:** `MermaidDiagram::render_to_image` (`mermaid.rs:578`) needs it **before** it picks `target_width_px`. Today it only gets it after rasterizing. Options:
  1. Add a `measure()` beside `render_svg` in biscuit-visualized (`mermaid/render.rs:212-227`). It would reuse the same parse, `build_theme`, init overrides, and `compute_layout`, then call `measure_svg_dimensions(&layout, &layout_config, None).viewbox_width`. This needs 0.3.1, because 0.2.1 has no measure API (`mrr-0.2.1/src/lib.rs:116`).
  2. Call `MermaidRenderer::render_to_svg()` (`mermaid.rs:287-297`), which produces a cached SVG, and parse the root `viewBox` with usvg or a regex. This works on 0.2 but costs an extra SVG render or cache file.
  - Either way, set `target_width_px = cols × cell_w` to match the existing `mermaid.rs:592`, so the raster fills the cells exactly. The effective scale is then within one cell of the request.
  - `GraphExpression` (`graph_expression.rs:340-349`) has the same shape if it should support `Scale` too.
- **Gaps:**
  - **Windows has no cell size.** `ppu` becomes exactly `scale` via the 8x16 fallback, so sizing is approximate.
  - **The CSI 14t fallback is skipped in CI** (`window_size.rs:48`), while the ioctl is not. CI therefore also resolves to 8x16 unless a pty populates pixels.
  - **Captured stdout is covered.** Rows, columns (`terminal_size` falls through to stderr) and cell size (via `/dev/tty` when `is_tty()` sees stderr) all satisfy R10 on Unix.
  - **All three standard streams redirected:** `is_tty()` is false, so cell size is `None` even though `/dev/tty` exists, and the grid falls back to 80x24. No `/dev/tty` grid query exists for rows or columns.
  - **The 8x16 fallback is duplicated** in 6 places plus `width.rs:80`. The new code should share one constant rather than add another literal.
