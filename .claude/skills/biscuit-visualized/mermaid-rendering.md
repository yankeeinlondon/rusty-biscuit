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

`RenderRequest::background` decides what is painted behind the diagram (Merman leaves the root unpainted): `Theme` (default) paints a `viewBox`-sized rect of the palette's background (`Palette::background()`: `Dark` `#333333`, the others `#ffffff`), `Solid { r, g, b }` paints that color, and `Transparent` paints nothing. PNG-only fields (`target_width`, `row_height`) are covered in [Rasterization](./rasterization.md).

`natural_size()` renders (or reads from the cache) the SVG artifact and returns its root `viewBox` size; the terminal component scales from it.

### Pipeline (`mermaid/render.rs`)

1. `default_chain()`; `source_theme` (Merman's `preprocess_diagram` + `registry().detect_type` → family id and the author's `%%{init}%%`/frontmatter config); the palette (`palette()`: `with_palette` else `Palette::fixed(theme)`) → `design::design_json(palette, family)` unless the author owns the theme; site config `{"theme": "base", "themeVariables": {"fontFamily": stack}}` merged over `MermaidConfig::to_json()`, plus palette `base` variables and `darkMode` (`wants_site_palette`), `family_site_variables` (gantt, cynefin, treeView, eventmodeling, venn, treemap, kanban, pie, xychart), and `family_site_config` (C4 `c4` text colors; packet `packet` block/label colors, author keys win). See [Themes and palettes](#themes-and-palettes).
2. `prepare` (source rewrites): ZenUML header → `RenderFailed`; `rewrite_label_newlines`; if the header is `gitGraph` and the source contains `tag:`, one Merman `layout_json` request, then `gitgraph_layout::commit_spacing_for` and `with_commit_spacing` (appends `%%{init: {"gitGraph": {"commitSpacing": N}}}%%`).
3. `cache_key_for` on the **rewritten** source (see [Artifacts & Caching](./artifacts-caching.md#mermaid-cache-key)); a hit returns before any SVG request.
4. `render_svg`: Merman `RenderRequest::svg` with `SvgPipeline::resvg_safe()`, a uniform `TextMeasurementPolicy` around `LabelMeasurer` (profile `biscuit-visualized`), `SvgEnvironment::deterministic()`, diagram id `bv-<xxhash of the cache key>`.
5. `design::compile` + `RenderRequest::with_theme` on the Merman request; then `postprocess` (`mermaid/postprocess.rs`), `accents::apply_accents`, `contrast::readable_text` (always, with or without a design; accents only with a design); for SVG with `MermaidTarget::HtmlStatic`, `html_static` (see [HTML static target](#html-static-target)); then `with_background`, then `rasterize_svg_for_request` for PNG, then `FileCache::store`.

### Time budget

`with_time_budget(Duration)` (default `MermaidDiagram::DEFAULT_TIME_BUDGET` = 2 s; `time_budget()` reads it) on `mermaid::MermaidDiagram`; the component's `with_time_budget` passes through. `control()` builds `OperationControl::new().with_deadline(budget)` **per Merman request**, so the gitGraph `layout_json` pass and the SVG render each get the full budget. It bounds Merman's parse/layout/SVG only (Merman's checkpoints), not post-processing or rasterization. `RenderError::Cancelled` → `tracing::warn!` (budget, elapsed) and `MermaidError::RenderFailed`; the component shows the code block. Tests read per-request elapsed times through the `cfg(test)` `take_merman_calls()`.

### Fallback

When rendering fails or the consumer doesn't support images:

```rust
let code_block = diagram.fallback_code_block();
// Returns: ```mermaid\n<source>\n```
```

Render failures (all `MermaidError::RenderFailed`): parse error, unknown diagram type, ZenUML (detected by `header_keyword` == `zenuml`; Merman would draw it, Mermaid.js core does not support it), a math label (`$$…$$`: Merman refuses the diagram because `math` is not compiled in), and an exceeded time budget.

## Themes and palettes

`mermaid::Palette` (`mermaid/palette.rs`, backend-neutral; `Rgb` in `mermaid/color.rs`): background, text, surface, surface_alt, border, line, 6 accents (ANSI order red, green, yellow, blue, magenta, cyan), tints (accent over background, ≤22%, weaker until text reaches 4.5:1), `chart_tints()` (≤60%, pie), `text_on(fill)`. Constructors all run the contrast passes (text 4.5:1 on background and every tint/surface; accents/border/line 3:1; no large-text exception):

- `Palette::fixed(MermaidTheme)`: from Mermaid.js 12.2.0's same-named theme (bg/text/mainBkg/nodeBorder/lineColor; accents from its `git*` colors). `Dark` bg `#333333` text `#cccccc`; `Default` `#ffffff`/`#333333`; `Forest` `#ffffff`/`#000000`; `Neutral` grays. **No code maps a `MermaidTheme` to a Merman built-in theme**; `as_str()` is only the name.
- `Palette::new(bg, fg, accents)`; `Palette::for_terminal(&TerminalColors, is_dark)`: ANSI 1–6 → accents, a missing slot → background-derived accent, no fg → black/white, **no bg → `fixed(for_color_mode(is_dark))`**.
- `TerminalColors { background, foreground, ansi: [Option<Rgb>; 16] }`; `TerminalColors::detect()` (feature `terminal`) is all-`None` when stdout is not a terminal or `CI` is set, else `biscuit_terminal::discovery::osc_queries::{bg_color, text_color, ansi_palette}` (OSC 4 runs on native Windows too).
- `MermaidDiagram::with_palette(p)` / `palette()`; `with_theme(t)` **resets** the palette to `fixed(t)`. Component: `MermaidDiagram::new` uses `terminal_palette()`; `with_theme` / `with_palette` override.

Designs (`mermaid/design.rs`): one `ThemeDesignV1` per family from a role table, every facet filtered through `describe_theme_support(family, StandaloneSvg, …)` (`Unconditional`/`Conditional`). Rules: no static fill beside a target's ordinal palette (Merman lets the static fill win); node fill = surface only where the family has no node ordinal palette (class, block, architecture, …); ordinal palettes: tints (flowchart, state, kanban, journey, timeline), chart tints (pie), accents (gitGraph, sankey, chart series), mindmap = accents moved to 4.5:1 against Merman's fixed `#333` label; no widths, no fonts; `design.mermaid` only for gitGraph label colors (`commitLabel*`, `tagLabel*`, `git0..7`, `gitBranchLabel0..7`; evidence `Residual` via `compatibility-unverified`). `REJECTED_DESPITE_SUPPORT`: kanban `task-label`.

Site config: Merman lets **explicit site `themeVariables` win over a compiled theme** (and over the source's own `%%{init}%%` variables, the reverse of Mermaid.js). So `wants_site_palette` adds palette `base` root variables only for families with no ordinal palette that are not sequence/class (treemap, quadrant, ER, gantt, C4, block, …); `family_site_variables` adds per-family surfaces behind labels (gantt alternate sections, cynefin domains, eventmodeling lanes/boxes, venn sets, treemap sections, kanban columns written ten points darker because Merman lightens `cScale`, treeView description text, pie text colors, `xyChart.*`), each measured not to erase the design; `family_site_config` sets C4 boundary/relationship text and packet block/label colors in their own config sections.

Author wins: author `theme` named → no design, no palette variables; author `themeVariables` with a **color** (`sets_a_color`) → no design, palette root variables with the author's copied on top; non-color variables (widths, `darkMode`) keep the design.

Post-design edits: `accents.rs` (flowchart tint→accent stroke pairs; class boxes in document order keyed on the surface fill; sequence actor `<rect name>` boxes left to right by `x`), all keyed on palette colors only, so author-styled shapes are never recolored. `contrast.rs`: each `<text>` vs the nearest inline-filled shape behind it (closed `<g>` siblings count, `<use href>` placements followed, else the palette background); when a text's color or that shape is painted by the stylesheet or an attribute (every surface under an author `%%{init}%%` color, the sequence `box`), usvg resolves the paint and the fill is the most frequent color under a 5×3 grid over the text box (fills and strokes, clips honored), found through a numbered stroke-only marker before each `<text>` because usvg drops ids inside `<use>`. A failing text is rewritten to `text_on(fill)` (on the `<text>` and its `<tspan>`s, fill first in the style) unless its color is one the author wrote as a text color (`author_text_colors`: CSS `color`, C4 `$fontColor`/`$textColor`, text/label/title/font theme keys); author text is kept on any fill. No CSS is injected; Merman's `<style>` is scoped `#<diagram id>` (a test guards it).

`MermaidTheme::for_color_mode(is_dark)` → `Dark`/`Default`; `inverse()`; `MermaidTheme::default()` is `Dark`.

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

Merman reads them (and frontmatter); a color or a named theme there displaces the compiled theme (see [Themes and palettes](#themes-and-palettes)). Nothing in the source is rewritten for the theme; the only directive this crate adds is gitGraph `commitSpacing`.

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

Every family in Merman's registry (`all-diagrams`): architecture, block, C4, class, ER, flowchart (`flowchart`/`graph`), gantt, gitGraph, journey, kanban, mindmap, packet, pie, quadrant chart, radar, requirement, sankey, sequence, state, timeline, treemap, XY chart, and the Merman-only agentflow, cynefin, `eventmodeling`, info, ishikawa, railroad (`railroadAbnf`/`railroadEbnf`/`railroadPeg`), swimlane, `treeView`, `usecase`, venn, wardley. One fixture per family in `src/src/tests/fixtures/merman/families/`; a test fails when a Merman bump adds a family without one. Not rendered (code-block fallback): `error`, ZenUML, and math labels.

## gitGraph: `commitSpacing` and geometry

Full contract: `biscuit-visualized/docs/mermaid-gitgraph.md`. Merman keeps both parents of a labeled `merge` and renders a merge of a branch with no commit of its own (second parent = the branch's head), as Mermaid.js does, so there is **no merge repair and no "empty branch" error** anymore. A merge *into* a lane with no commit is still a Merman/Mermaid.js error; the `GitGraph` component never emits one.

- **Tag spacing** (`gitgraph_layout::commit_spacing_for`, run in `prepare`): only when the source contains `tag:`. One `layout_json` pass at the current spacing (Merman default 50 = `COMMIT_STEP` 40 + `LAYOUT_OFFSET` 10; `DEFAULT_COMMIT_SPACING`). Returns `None` when the effective config already sets `gitGraph.commitSpacing` (author wins, directive or frontmatter), when the direction is not LR (TB/BT tags are rotated and not spaced), or when no pair collides. Colliding pair = tags on different commits, different columns (`parallelCommits` same column is skipped), whose boxes share rows (strict). Gap = `TAG_GAP_EM` (1.0) × `themeVariables.fontSize` (16 default; read from the layout document's `meta.effective_config`). Each pair needs `step + shortfall / column distance`; the max wins, rounded up to 0.01 px (`SPACING_RESOLUTION`); `SPACING_EPSILON` 0.01. Single pass: boxes move linearly with spacing, no verify loop. Tag boxes: widths from our measurer at `tagLabelFontSize` read exactly as Merman reads it (`render.rs` `tag_font_size`: only a string `px`/`em`/`%`/`inherit`/`unset`/`0`; a JSON number or anything else = 10 px; base `fontSize` clamped to ≥ 1; pinned by `gitgraph_tests::tag_font_size_*`, which compare against the drawn polygons) and Merman's `drawCommitTags` formula (`TAG_PX` 4, `TAG_PY` 2, `TAG_LIFT` 19.2, `TAG_STACK_STEP` 20; rotated 45° for TB/BT).
- **Directive:** `with_commit_spacing` appends `%%{init: {"gitGraph": {"commitSpacing": N}}}%%` as the last line; Merman deep-merges directives in order, so nothing else changes. `commitSpacing` exists only on the fork (`yankeeinlondon/merman` branch `gitgraph-commit-spacing`).
- **Geometry** (`#[doc(hidden)] MermaidDiagram::gitgraph_geometry()`): `gitgraph_layout::geometry_from_layout_json` reads the layout document of the rewritten source **strictly** (missing/mistyped fields, duplicate keys anywhere, trailing content → `LayoutDataError` → `RenderFailed`, each naming the JSON path) and builds `GitGraphGeometry` (commits with id, lane, parents, `index`, `x`, tags with bounds; `commit_step` from the commits' `pos`, `tag_gap`, size) with `tag_boxes()` / `tag_overlaps()`. `Ok(None)` for non-gitGraphs. `#[doc(hidden)] default_gitgraph_commit_step()` is 50; compare against it, never a literal. Prove tag placement with it, never with Mermaid text. Tests: `src/src/tests/gitgraph_tests.rs`, `src/src/tests/gitgraph_layout_tests.rs`.

## SVG Post-Processing (`mermaid/postprocess.rs`)

String-level edits keyed on Merman's output at the pinned rev (guard tests in `src/src/tests/postprocess_tests.rs` fail when either becomes dead; re-check on every Merman bump). Idempotent; an SVG with neither defect is returned borrowed, unchanged.

1. **Gradient strokes → solid:** every non-default theme's neo look strokes shapes with `stroke:url(#…)` to an `objectBoundingBox` `<linearGradient>`, which paints nothing on zero-height dividers (class/ER). Every such reference becomes the stylesheet's node stroke color (else the gradient's first stop).
2. **`sans-serif` default:** a root `<svg>` with no font (no `#<id>{font-family:…}` rule, e.g. wardley) gets `font-family="sans-serif"`, so usvg never falls back to serif.

There are no other SVG string patches: quadrant fills and point radius go through site config, and the accent and contrast edits live in `mermaid/accents.rs` and `mermaid/contrast.rs` (see [Themes and palettes](#themes-and-palettes)).

## HTML static target

`MermaidDiagram::with_target(MermaidTarget::HtmlStatic)` (`target()` reads it; `#[non_exhaustive]` enum, default `RasterSafe`) selects SVG for a browser. The component `components::mermaid::MermaidDiagram::with_target` (and the crate-private `MermaidRenderer::with_target`) pass it through, so `render_to_svg` (Darkmatter's `render_browser_mermaid` path) honors it. Darkmatter only selects it and sanitizes.

`postprocess::html_static(svg, &LabelMeasurer)` runs after `postprocess`, SVG only:

- **Font:** every `font-family` value (`<style>` rules, inline `style` declarations, `font-family` attributes) becomes `'<Family>',<original stack>`. `<Family>` = `LabelMeasurer::measured_family(style)`: the measurer's own cached `face_for` resolution (`FontChain::resolve` of `LabelMeasurer::query`), then the face's first fontdb family name. `render_svg` builds one `Arc<LabelMeasurer>` for Merman and reuses it here. Weight/style come from the same declaration block or the element's attributes (family choice is weight-independent in the chain anyway). Attribute values and stylesheet text are entity-decoded before splitting on `;` (Merman writes `&quot;`), and re-encoded only when edited. The name is single-quoted with CSS hex escapes for quotes, `\`, `<`, `>`, `&`, and controls. Prepended even when the stack already starts with that family in another spelling (`"trebuchet ms"` on macOS); only the exact `'<Family>',` this pass writes counts as done (idempotence).
- **Overflow:** each non-root `<svg>` whose content holds a `<text>` gets `overflow` → `visible` in its `style`/`overflow` attribute, or ` overflow="visible"` added when it has neither. Merman clips class/ER/mindmap/state labels with `overflow:hidden` there. Root `<svg>` and viewports without text untouched; text content never edited.
- **PNG ignores the target** (`output_target(format)`): rasterized from the raster-safe SVG, identical key. Raster output never gets an overflow patch (`postprocess_is_idempotent_and_adds_no_overflow_patch`).
- **Cache key:** `"target": "html-static"` is added to the options JSON only for SVG + `HtmlStatic`; raster-safe keys are unchanged, so no `+bv<n>` bump.
- Tests: `src/src/tests/html_static_tests.rs` (injected Inter chain for the post-process; host chain end to end; `take_rasterized_svgs()` is the `cfg(test)` record of SVGs handed to the rasterizer), plus `the_html_static_target_has_its_own_svg_key_only` in `mermaid_cache_key_tests.rs`.

## Embedded font (HTML static opt-in)

Full doc: `biscuit-visualized/docs/mermaid-embedded-font.md`. Code: `mermaid/embed/`.

- **Opt-in:** `MermaidDiagram::with_embedded_font(true)` (default off) acts only for SVG + `HtmlStatic` (`embeds_font(format)`); otherwise byte-identical output and key. `render_with_font` → `(RenderedArtifact, Option<DiagramFont>)`; component: `with_embedded_font` + `render_to_svg_with_font`. `page_font(&[DiagramFont])` → `PageFont` (`comments()`, `css()`, `markup()`, `is_empty()`) or `FontEmbeddingError` (one warning already logged; caller emits no font).
- **Face rule (`embed/faces.rs`, `embed/permission.rs`):** per requested `font-family` list, regular and bold, decided before layout by `FaceChooser` (`LabelMeasurer::embedding`). Embed the chain's resolved face when `fsType` permits (Installable/Editable/Preview & print; least restrictive bit wins; `0x0100` → embed whole); otherwise the whole family is bundled Inter: Restricted, `0x0200` bitmap-only, no/short `OS/2`, not `glyf` (CFF/CFF2/`fvar`), no installed face, or `MERMAID_FONT=Inter`. Fail closed.
- **Measurement:** plain advances (`ShapeParams::layout_tables = false`, face rebuilt from the subset's tables) because the `allsorts` subset has no `GPOS`/`GSUB`/`kern`.
- **Subset (`embed/glyphs.rs`, `embed/subset.rs`, `embed/embedding.rs`):** characters read from the final SVG via `roxmltree` + `simplecss` (usvg's cascade), split at weight 600, U+0020 added; one rule per face × weight drawn, union over the page. Budget `BUDGET_BYTES` = 60,000 (pre-base64): over → still embedded, `info` log. Any face failing → whole page `Err`.
- **Page markup (`embed/comment.rs`):** one `<!-- mermaid font: … -->` per family (names via `comment_safe`), then `<style>`; Inter's OFL CSS comment (`INTER_COPYRIGHT`, tested against `OFL.txt`) right before its first rule, only when Inter is embedded.
- **Cache:** options JSON `target: "html-static+embedded-font"`; `{key}.font.json` record (`FontRecord`) beside the SVG, written first; missing/unreadable record → re-render. Subset never cached.
- Tests: `src/src/tests/embed_*_tests.rs` (`embed_render_tests.rs` is the render path end to end).

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
| `mermaid/color.rs` | `Rgb`, WCAG contrast, tint/contrast helpers |
| `mermaid/palette.rs` | `Palette`, `TerminalColors` (`detect()` under `terminal`) |
| `mermaid/design.rs` | Per-family `ThemeDesignV1`, support filter, site-palette rule, gitGraph label variables |
| `mermaid/accents.rs` | Flowchart / class / sequence accent edits |
| `mermaid/contrast.rs` | Text contrast pass, CSS color parser |
| `mermaid/error.rs` | `MermaidError` |
