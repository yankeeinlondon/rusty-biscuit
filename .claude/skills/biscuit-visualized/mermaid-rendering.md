# Mermaid Rendering

## MermaidDiagram API

The main entry point for rendering Mermaid diagrams to SVG or PNG. Rendering is done by Merman (pinned fork, see [SKILL.md](./SKILL.md#dependencies)) with this crate's text measurer and font chain. User-facing doc: `biscuit-visualized/docs/merman.md`.

### Construction

```rust
use biscuit_visualized::mermaid::{MermaidDiagram, MermaidTheme, MermaidConfig};

let diagram = MermaidDiagram::new("flowchart LR\n    A --> B")
    .with_theme(MermaidTheme::Dark)
    .with_title("My Diagram")
    .with_config(config);
```

### Rendering

```rust
use biscuit_visualized::artifact::{RenderRequest, OutputFormat};

let req = RenderRequest { format: OutputFormat::Svg, ..RenderRequest::default() };
let artifact = diagram.render(&req)?;

// artifact.path: PathBuf to the rendered file (may be in cache)
// artifact.format: OutputFormat::Svg
// artifact.cache_hit: bool
// artifact.alt_text: String
```

`RenderRequest::background` decides what is painted behind the diagram (Merman leaves the root unpainted): `Theme` (default) paints a `viewBox`-sized rect of the built-in theme's background (`Dark` `#333`, `Default`/`Forest` `white`, `Neutral` `#ffffff`), `Solid { r, g, b }` paints that color, and `Transparent` paints nothing. PNG-only fields (`target_width`, `row_height`) are covered in [Rasterization](./rasterization.md).

`natural_size()` renders (or reads from the cache) the SVG artifact and returns its root `viewBox` size; the terminal component scales from it.

### Pipeline (`mermaid/render.rs`)

1. `default_chain()` and the site config: `{"theme": MermaidTheme::as_str()}` merged over `MermaidConfig::to_json()`.
2. `prepare` (source rewrites): ZenUML header → `RenderFailed`; `rewrite_label_newlines`; if the header is `gitGraph` and the source contains `tag:`, one Merman `layout_json` request, then `gitgraph_layout::commit_spacing_for` and `with_commit_spacing` (appends `%%{init: {"gitGraph": {"commitSpacing": N}}}%%`).
3. `cache_key_for` on the **rewritten** source (see [Artifacts & Caching](./artifacts-caching.md#mermaid-cache-key)); a hit returns before any SVG request.
4. `render_svg`: Merman `RenderRequest::svg` with `SvgPipeline::resvg_safe()`, a uniform `TextMeasurementPolicy` around `LabelMeasurer` (profile `biscuit-visualized`), `SvgEnvironment::deterministic()`, diagram id `bv-<xxhash of the cache key>`.
5. `postprocess` (`mermaid/postprocess.rs`); for SVG with `MermaidTarget::HtmlStatic`, `html_static` (see [HTML static target](#html-static-target)); then `with_background`, then `rasterize_svg_for_request` for PNG, then `FileCache::store`.

### Time budget

`with_time_budget(Duration)` (default `MermaidDiagram::DEFAULT_TIME_BUDGET` = 2 s; `time_budget()` reads it) on `mermaid::MermaidDiagram`; the component's `with_time_budget` passes through. `control()` builds `OperationControl::new().with_deadline(budget)` **per Merman request**, so the gitGraph `layout_json` pass and the SVG render each get the full budget. It bounds Merman's parse/layout/SVG only (Merman's checkpoints), not post-processing or rasterization. `RenderError::Cancelled` → `tracing::warn!` (budget, elapsed) and `MermaidError::RenderFailed`; the component shows the code block. Tests read per-request elapsed times through the `cfg(test)` `take_merman_calls()`.

### Fallback

When rendering fails or the consumer doesn't support images:

```rust
let code_block = diagram.fallback_code_block();
// Returns: ```mermaid\n<source>\n```
```

Render failures (all `MermaidError::RenderFailed`): parse error, unknown diagram type, ZenUML (detected by `header_keyword` == `zenuml`; Merman would draw it, Mermaid.js core does not support it), a math label (`$$…$$`: Merman refuses the diagram because `math` is not compiled in), and an exceeded time budget.

## MermaidTheme

| Variant | Description | Best For |
|---------|-------------|----------|
| `Dark` | Dark background, light elements | Dark terminal themes |
| `Default` | Standard Mermaid light theme | Light terminals |
| `Forest` | Green tones | Nature-themed output |
| `Neutral` | Muted, low-contrast | Formal/documentation |

**Interim:** each variant maps to Merman's built-in theme of the same name (`as_str()`: `dark`, `default`, `forest`, `neutral`). **Planned:** compiled palettes (terminal-derived in a terminal, accents for unstyled nodes, contrast-aware text; the four variants become fixed palettes) replace this mapping. `MermaidTheme::default()` is `Dark`.

### Adaptive Theme Selection

```rust
// Pick theme based on detected color mode
let theme = MermaidTheme::for_color_mode(is_dark);
// is_dark=true  → Dark
// is_dark=false → Default

// Get the opposite theme
let inverted = theme.inverse();
```

### Theme Variables via Init Directives

Mermaid supports inline theme customization through `%%{init: {...}}%%` directives. These are passed through as-is:

```
%%{init: {'theme': 'dark', 'themeVariables': {'primaryColor': '#ff0000'}}}%%
flowchart LR
    A --> B
```

Merman reads them (and frontmatter) as Mermaid.js does, over the site config this crate passes. Nothing in the source is rewritten for the theme; the only directive this crate adds is gitGraph `commitSpacing`.

## MermaidConfig (Quadrant Charts)

For fine-grained quadrant chart customization. `to_json()` becomes Merman site config: `quadrantChart.pointLabelFontSize` / `pointRadius` and `themeVariables.quadrant{1..4}Fill` (no SVG string patches). `point_label_font_size` sizes quadrant point labels only.

```rust
use biscuit_visualized::mermaid::MermaidConfig;

let config = MermaidConfig::default()
    .with_point_label_font_size(14)
    .with_point_radius(8)
    .with_quadrant_fill(1, "#ff0000")   // Top-right
    .with_quadrant_fill(2, "#00ff00")   // Top-left
    .with_quadrant_fill(3, "#0000ff")   // Bottom-left
    .with_quadrant_fill(4, "#ffff00");  // Bottom-right
```

### QuadrantTheme Presets

| Preset | Description |
|--------|-------------|
| `Default` | No customization, uses Mermaid defaults |
| `MagicQuadrangle` | Gartner Magic Quadrant-inspired colors and sizing |

```rust
use biscuit_visualized::mermaid::{QuadrantTheme, MermaidConfig};

// Apply a preset theme to a config, adapting to dark/light mode
let config = QuadrantTheme::MagicQuadrangle.apply(MermaidConfig::default(), true);

// Parse from string (for CLI args)
let theme = QuadrantTheme::parse("magic-quadrangle");
```

## Supported Diagram Types

Every family Merman's `all-diagrams` feature renders, among them flowchart (`flowchart`/`graph`), sequence, class, state, ER, pie, XY chart, quadrant, gantt, timeline, journey, mindmap, gitGraph, requirement, C4, block, kanban, architecture, packet, radar, sankey, and treemap. Not rendered (code-block fallback): ZenUML and math labels.

## gitGraph: `commitSpacing` and geometry

Full contract: `biscuit-visualized/docs/mermaid-gitgraph.md`. Merman keeps both parents of a labeled `merge` and renders a merge of a branch with no commit of its own (second parent = the branch's head), as Mermaid.js does, so there is **no merge repair and no "empty branch" error** anymore. A merge *into* a lane with no commit is still a Merman/Mermaid.js error; the `GitGraph` component never emits one.

- **Tag spacing** (`gitgraph_layout::commit_spacing_for`, run in `prepare`): only when the source contains `tag:`. One `layout_json` pass at the current spacing (Merman default 50 = `COMMIT_STEP` 40 + `LAYOUT_OFFSET` 10; `DEFAULT_COMMIT_SPACING`). Returns `None` when the effective config already sets `gitGraph.commitSpacing` (author wins, directive or frontmatter), when the direction is not LR (TB/BT tags are rotated and not spaced), or when no pair collides. Colliding pair = tags on different commits, different columns (`parallelCommits` same column is skipped), whose boxes share rows (strict). Gap = `TAG_GAP_EM` (1.0) × `themeVariables.fontSize` (16 default; read from the layout document's `meta.effective_config`). Each pair needs `step + shortfall / column distance`; the max wins, rounded up to 0.01 px (`SPACING_RESOLUTION`); `SPACING_EPSILON` 0.01. Single pass: boxes move linearly with spacing, no verify loop. Tag boxes: widths from our measurer at `tagLabelFontSize` (10 default) and Merman's `drawCommitTags` formula (`TAG_PX` 4, `TAG_PY` 2, `TAG_LIFT` 19.2, `TAG_STACK_STEP` 20; rotated 45° for TB/BT).
- **Directive:** `with_commit_spacing` appends `%%{init: {"gitGraph": {"commitSpacing": N}}}%%` as the last line; Merman deep-merges directives in order, so nothing else changes. `commitSpacing` exists only on the fork (`yankeeinlondon/merman` branch `gitgraph-commit-spacing`).
- **Geometry** (`#[doc(hidden)] MermaidDiagram::gitgraph_geometry()`): `gitgraph_layout::geometry_from_layout_json` reads the layout document of the rewritten source **strictly** (missing/mistyped fields, duplicate keys anywhere, trailing content → `LayoutDataError` → `RenderFailed`, each naming the JSON path) and builds `GitGraphGeometry` (commits with id, lane, parents, `index`, `x`, tags with bounds; `commit_step` from the commits' `pos`, `tag_gap`, size) with `tag_boxes()` / `tag_overlaps()`. `Ok(None)` for non-gitGraphs. `#[doc(hidden)] default_gitgraph_commit_step()` is 50; compare against it, never a literal. Prove tag placement with it, never with Mermaid text. Tests: `src/src/tests/gitgraph_tests.rs`, `src/src/tests/gitgraph_layout_tests.rs`.

## SVG Post-Processing (`mermaid/postprocess.rs`)

String-level edits keyed on Merman's output at the pinned rev (guard tests in `src/src/tests/postprocess_tests.rs` fail when either becomes dead; re-check on every Merman bump). Idempotent; an SVG with neither defect is returned borrowed, unchanged.

1. **Gradient strokes → solid:** every non-default theme's neo look strokes shapes with `stroke:url(#…)` to an `objectBoundingBox` `<linearGradient>`, which paints nothing on zero-height dividers (class/ER). Every such reference becomes the stylesheet's node stroke color (else the gradient's first stop).
2. **`sans-serif` default:** a root `<svg>` with no font (no `#<id>{font-family:…}` rule, e.g. wardley) gets `font-family="sans-serif"`, so usvg never falls back to serif.

Gone with mermaid-rs-renderer: the legend baseline fix, the pie label-contrast fix, quadrant fill/point-radius string patches (now site config), and forcing gitGraph HIGHLIGHT commits to the branch color. **Planned:** an accent-color rewrite keyed on our palette colors.

## HTML static target

`MermaidDiagram::with_target(MermaidTarget::HtmlStatic)` (`target()` reads it; `#[non_exhaustive]` enum, default `RasterSafe`) selects SVG for a browser. The component `components::mermaid::MermaidDiagram::with_target` (and the crate-private `MermaidRenderer::with_target`) pass it through, so `render_to_svg` (Darkmatter's `render_browser_mermaid` path) honors it. Darkmatter only selects it and sanitizes.

`postprocess::html_static(svg, &LabelMeasurer)` runs after `postprocess`, SVG only:

- **Font:** every `font-family` value (`<style>` rules, inline `style` declarations, `font-family` attributes) becomes `'<Family>',<original stack>`. `<Family>` = `LabelMeasurer::measured_family(style)`: the measurer's own cached `face_for` resolution (`FontChain::resolve` of `LabelMeasurer::query`), then the face's first fontdb family name. `render_svg` builds one `Arc<LabelMeasurer>` for Merman and reuses it here. Weight/style come from the same declaration block or the element's attributes (family choice is weight-independent in the chain anyway). Attribute values and stylesheet text are entity-decoded before splitting on `;` (Merman writes `&quot;`), and re-encoded only when edited. The name is single-quoted with CSS hex escapes for quotes, `\`, `<`, `>`, `&`, and controls. Prepended even when the stack already starts with that family in another spelling (`"trebuchet ms"` on macOS); only the exact `'<Family>',` this pass writes counts as done (idempotence).
- **Overflow:** each non-root `<svg>` whose content holds a `<text>` gets `overflow` → `visible` in its `style`/`overflow` attribute, or ` overflow="visible"` added when it has neither. Merman clips class/ER/mindmap/state labels with `overflow:hidden` there. Root `<svg>` and viewports without text untouched; text content never edited.
- **PNG ignores the target** (`output_target(format)`): rasterized from the raster-safe SVG, identical key. D33's "no overflow patch" still holds for raster output (`postprocess_is_idempotent_and_adds_no_overflow_patch`).
- **Cache key:** `"target": "html-static"` is added to the options JSON only for SVG + `HtmlStatic`; raster-safe keys are unchanged, so no `+bv<n>` bump.
- Tests: `src/src/tests/html_static_tests.rs` (injected Inter chain for the post-process; host chain end to end; `take_rasterized_svgs()` is the `cfg(test)` record of SVGs handed to the rasterizer), plus `the_html_static_target_has_its_own_svg_key_only` in `mermaid_cache_key_tests.rs`.

## MermaidError

```rust
pub enum MermaidError {
    /// Mermaid rendering failed (parse error, unknown type, ZenUML, math label,
    /// time budget exceeded, unreadable gitGraph layout data)
    RenderFailed(String),
    /// SVG-to-PNG rasterization failed
    RasterizationFailed(RasterError),
    /// File I/O error during caching or temp file operations
    Io(#[from] std::io::Error),
}
```

## Source Files

| File (under `biscuit-visualized/src/src/`) | Contents |
|------|----------|
| `mermaid/mod.rs` | Module re-exports |
| `mermaid/render.rs` | `MermaidDiagram`: pipeline, Merman requests, time budget, cache key, background, `natural_size`, ZenUML detection |
| `mermaid/newline.rs` | `\n` → `<br/>` label rewrite (class, ER, mindmap, requirement) |
| `mermaid/gitgraph_layout.rs` | Strict layout-data reader, `GitGraphGeometry` builder, `commitSpacing` computation and directive |
| `mermaid/gitgraph.rs` | `GitGraphGeometry` types, `TAG_GAP_EM`, `default_gitgraph_commit_step()` |
| `mermaid/postprocess.rs` | Gradient-stroke fix, `sans-serif` default, HTML static edits (`html_static`) |
| `mermaid/config.rs` | `MermaidTheme`, `MermaidConfig`, `QuadrantTheme` |
| `mermaid/error.rs` | `MermaidError` |
