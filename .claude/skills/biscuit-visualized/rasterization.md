# Rasterization

SVG-to-PNG conversion using `resvg`. Two complementary entry points:

| Goal | Function | Sizing input |
|------|----------|--------------|
| Render at a known display target | `rasterize_svg_to_png(svg, width)` | Pixel width |
| Render at an HiDPI multiplier | `rasterize_svg_to_png_bytes(svg, scale)` | Integer scale factor |

Both preserve aspect ratio and run through the same `resvg` + `tiny_skia` pipeline; they just differ in how the output dimensions are computed.

## PNG encoding

Every PNG (public functions and `RenderRequest` renders alike) is encoded by the private `encode_png` with the `png` crate (0.18, the version tiny-skia 0.12 already uses) at zlib's default compression level (`Compression::Balanced`) with adaptive filtering, after converting tiny-skia's premultiplied alpha to straight alpha. tiny-skia 0.12's own `encode_png` now uses the same defaults (0.11 used fast compression whose output could exceed the raw pixel size), but the settings stay explicit: they are part of every PNG artifact's cache key (`PNG_ENCODING` in `artifact.rs`), so a change in tiny-skia's defaults or ours never serves bytes encoded the old way. `Compression::High` was measured larger on a gradient test image, so it is not used.

## Width-based API (preferred for display)

```rust
use biscuit_visualized::raster::rasterize_svg_to_png;

let svg_data = std::fs::read_to_string("diagram.svg")?;
let png_bytes = rasterize_svg_to_png(&svg_data, 1600)?;
// PNG is exactly 1600 px wide; height derived from SVG aspect ratio.
std::fs::write("diagram.png", &png_bytes)?;
```

**Use this when the caller knows the display target.** Examples:

- Terminal cell area: `cells × cell_pixel_width` → exact terminal pixel width.
- Layout container: parent width in pixels.
- Thumbnail bucket: e.g. "always render at 800 px wide".

Because `resvg` is fed the exact target dimensions, glyphs and shapes are rasterised once at the display resolution — there's no intermediate bitmap to resample, so text stays crisp.

## Scale-based API (HiDPI multiplier)

```rust
use biscuit_visualized::raster::rasterize_svg_to_png_bytes;

let png_bytes = rasterize_svg_to_png_bytes(&svg_data, 2)?;
// PNG is svg_native_width × 2 wide.
```

**Use this when the caller wants a fixed multiplier of the SVG's native size**, independent of any particular display target:

- 2× / 3× artwork for print or web export.
- Caching at multiple DPIs ahead of display.
- Anywhere "render N× sharper" is the actual requirement.

| Scale | Effective DPI | Use case |
|-------|--------------|----------|
| 1 | 96 | Static SVG export, predictable file size |
| 2 | 192 | Retina/HiDPI export — historic library default |
| 3 | 288 | High-quality print |

## File-to-file convenience

```rust
use std::path::Path;
use biscuit_visualized::raster::rasterize_svg;

rasterize_svg(Path::new("input.svg"), Path::new("output.png"), 2)?;
```

This wrapper is scale-based today. For a width-based file-to-file path, read the SVG, call `rasterize_svg_to_png(&svg, width)`, write the bytes — the building blocks are all public.

## `RenderRequest` plumbing

Higher-level renderers (`GraphDiagram::render`, `MermaidDiagram::render`) carry the choice through `RenderRequest`:

```rust
use biscuit_visualized::artifact::{OutputFormat, RenderRequest};

// Width-based — preferred for display
let req = RenderRequest::default()
    .with_target_width(1600);

// Scale-based — preferred for fixed multipliers
let req = RenderRequest {
    format: OutputFormat::Png,
    scale: 2,
    target_width: None,           // None → scale path
    ..RenderRequest::default()
};

// Terminal display: exact width, height padded to whole 18 px rows
let req = RenderRequest::default()
    .with_target_width(540)
    .with_row_height(18);
```

When `target_width` is `Some`, the rasterizer renders at that exact pixel width and `scale` is ignored. When `target_width` is `None`, the rasterizer falls back to `scale × svg_native`.

`row_height` pads the canvas height up to the next multiple of that many pixels; the drawing sits at the top and the padding below it is transparent, or filled with the color of `Background::Solid`. A terminal reserves whole cell rows for an image, so a padded PNG fills them 1:1 instead of being stretched. It is ignored for SVG and has no public-function equivalent: it is reachable only through `RenderRequest` (the crate-private `rasterize_svg_for_request` is the renderers' single SVG-to-PNG path).

The cache key incorporates `scale`, `target_width`, `row_height`, `background`, and the PNG encoding, so renders differing in any of them produce distinct cache entries.

## Query available fonts

```rust
use biscuit_visualized::raster::available_font_families;

let fonts = available_font_families();
// Installed family names plus the bundled "Inter", sorted, from the same
// database the rasterizer draws with, e.g. ["Arial", "Helvetica", "Inter", ...]
```

## Font Database and Resolution

Text is drawn through the crate's one font-resolution chain
(`fonts::FontChain`, crate-private, `image` feature), the same chain the
Mermaid text measurer (`fonts::measure::LabelMeasurer`) resolves with:

- The process chain (`fonts::default_chain()`) is built lazily on first use
  behind a `OnceLock`: `load_system_fonts()`, then the bundled Inter
  Regular/Bold (family `Inter`) and, on Linux builds, the bundled Noto Emoji.
  `MERMAID_FONT` is read once, there. fontique (the OS generic family) and the
  OS/2 scan of step 4 run only when a query first needs them.
- `raster::usvg_options(chain)` hands usvg the chain's own
  `Arc<fontdb::Database>` and a `usvg::FontResolver` whose `select_font`
  calls `FontChain::resolve(&FontQuery::from_usvg(font))` and whose
  `select_fallback` calls `FontChain::fallback`.
- Resolution order: `MERMAID_FONT` (installed or bundled Inter, any casing)
  → the CSS stack (names case-insensitive; generics by the OS answer) → the
  OS `sans-serif` answer (fontique) → an installed OS/2 sans-serif face by a
  fixed rule. Fallback: the resolved face, then installed faces by case-folded
  family name, then Noto Emoji (Linux) last; bundled Inter is never a fallback face.
- `MERMAID_FONT` therefore overrides every rasterized SVG's fonts, graphs
  included.
- Tests build a chain with `FontChain::injected(installed_bytes,
  generic_answers, mermaid_font)` (cfg(test)) and rasterize with
  `raster::png::rasterize_svg_to_png_bytes_with`; never `load_system_fonts`
  in a test that is not about the default chain.

User-facing rules: `biscuit-visualized/docs/merman.md`.

## `RasterError`

```rust
pub enum RasterError {
    /// SVG data could not be parsed
    SvgParseFailed(String),
    /// Rendering to pixmap failed (includes "SVG reports zero or negative width"
    /// from the width-based API)
    RenderFailed(String),
    /// File I/O error
    IoError(#[from] std::io::Error),
}
```

Common causes:

- `SvgParseFailed`: Malformed SVG, unsupported SVG features.
- `RenderFailed`: Zero-size viewport, extremely large dimensions, or an SVG that reports a non-positive native width (width-based API only — the rasterizer needs the SVG's native width to compute the uniform scale).

## Source Files

| File | Contents |
|------|----------|
| `biscuit-visualized/src/raster/mod.rs` | `usvg_options()` (chain database + `FontResolver`), `available_font_families()`, public re-exports |
| `biscuit-visualized/src/fonts/` | The font-resolution chain: `query.rs` (`FontQuery`, CSS family parser), `chain.rs` (steps, fallback, face identities), `metadata.rs` (OS/2 sans-serif rule), `matching.rs` (fontdb's CSS matching), `bundled.rs` |
| `biscuit-visualized/src/raster/png.rs` | `rasterize_svg_to_png()` (width-based), `rasterize_svg_to_png_bytes()` (scale-based), `rasterize_svg()` file wrapper, `rasterize_svg_for_request()` (crate-private), `encode_png()`, `RasterError` |
| `biscuit-visualized/src/artifact.rs` | `RenderRequest` (`scale`, `target_width`, `row_height`, `background`; `with_target_width`/`with_row_height`/`with_background` builders), `Background`, `PNG_ENCODING` |
