# Rasterization

SVG-to-PNG conversion using `resvg`. Two complementary entry points:

| Goal | Function | Sizing input |
|------|----------|--------------|
| Render at a known display target | `rasterize_svg_to_png(svg, width)` | Pixel width |
| Render at an HiDPI multiplier | `rasterize_svg_to_png_bytes(svg, scale)` | Integer scale factor |

Both preserve aspect ratio and run through the same `resvg` + `tiny_skia` pipeline; they just differ in how the output dimensions are computed.

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
    transparent_background: false,
};
```

When `target_width` is `Some`, the rasterizer renders at that exact pixel width and `scale` is ignored. When `target_width` is `None`, the rasterizer falls back to `scale × svg_native`.

The cache key incorporates both fields, so width-based and scale-based renders of the same SVG produce distinct cache entries.

## Query available fonts

```rust
use biscuit_visualized::raster::available_font_families;

let fonts = available_font_families();
// Returns Vec<String> of system font family names available to resvg
// e.g., ["Arial", "Helvetica", "Courier New", ...]
```

## Font Database

The system font database is loaded lazily on first use and cached via `OnceLock` for thread safety. This means:

- First rasterization call may be slightly slower (font enumeration).
- Subsequent calls reuse the cached font database.
- No need to manually initialize or manage fonts.

`resvg` uses the system font database to render text in SVGs. If a font referenced in the SVG is not installed, `resvg` falls back to a default font.

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
| `biscuit-visualized/src/raster/mod.rs` | Font database initialization, `available_font_families()`, public re-exports |
| `biscuit-visualized/src/raster/png.rs` | `rasterize_svg_to_png()` (width-based), `rasterize_svg_to_png_bytes()` (scale-based), `rasterize_svg()` file wrapper, `RasterError` |
| `biscuit-visualized/src/artifact.rs` | `RenderRequest` with both `scale: u32` and `target_width: Option<u32>` fields, plus `with_target_width()` builder |
