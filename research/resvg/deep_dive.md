# resvg: High-Performance SVG Rendering in Rust

## Table of Contents

1. [Introduction](#introduction)
1. [Ecosystem Architecture](#ecosystem-architecture)
1. [Core Integration: The "Three Musketeers"](#core-integration)
1. [Getting Started: Basic Rendering](#getting-started)
1. [Advanced Usage](#advanced-usage)
   * [Handling Fonts and Text](#handling-fonts-and-text)
   * [Scaling and Fitting Logic](#scaling-and-fitting-logic)
   * [High-DPI Rendering for Print](#high-dpi-rendering)
1. [Real-World Use Cases](#real-world-use-cases)
1. [Common Gotchas and Solutions](#common-gotchas-and-solutions)
1. [Performance and Limitations](#performance-and-limitations)
1. [The Landscape: Alternatives and Comparisons](#the-landscape)
1. [Licensing and Compliance](#licensing-and-compliance)

---

## 1. Introduction <a name="introduction"></a>

`resvg` is a high-performance, stand-alone SVG rendering library written in Rust. Unlike browser-based engines that must handle the massive overhead of the DOM, JavaScript, and complex CSS, `resvg` is purpose-built to convert SVG vector data into raster bitmaps (pixels).

It is widely regarded as the gold standard for SVG rendering in the Rust ecosystem due to its extreme specification compliance, security-first design, and lack of heavy system dependencies.

---

## 2. Ecosystem Architecture <a name="ecosystem-architecture"></a>

To use `resvg` effectively, you must understand that it is part of a modular ecosystem. It is not a monolithic engine; rather, it coordinates several specialized sub-crates:

1. **`usvg` (Micro SVG):** The parser and optimizer. It converts raw, messy SVG XML into a simplified, "flattened" tree (`usvg::Tree`). It resolves CSS, converts shapes (rects, circles) into paths, and handles inherited attributes.
1. **`resvg` (The Renderer):** The logic engine. It traverses the `usvg::Tree` and generates drawing commands.
1. **`tiny-skia` (The Backend):** A pure-Rust software rasterizer (a subset of Google’s Skia). It handles the actual math of drawing pixels, gradients, and anti-aliasing.

---

## 3. Core Integration: The "Three Musketeers" <a name="core-integration"></a>

Most `resvg` projects involve three primary partners to handle the full lifecycle of an image:

|Library|Role|Why use it?|
|:------|:---|:----------|
|**`usvg`**|Parsing|Converts SVG XML strings into a renderable data tree.|
|**`tiny-skia`**|Surface|Provides the memory buffer (`Pixmap`) and 2D drawing backend.|
|**`image`**|Export|Encodes the raw pixel output into standard files (PNG, JPEG, WebP).|

---

## 4. Getting Started: Basic Rendering <a name="getting-started"></a>

The "Happy Path" involves loading a file, parsing it via `usvg`, and rendering it onto a `tiny-skia` Pixmap.

````rust
use resvg::{usvg, tiny_skia};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Load and parse the SVG into an optimized Tree
    let opt = usvg::Options::default();
    let svg_data = std::fs::read("input.svg")?;
    let tree = usvg::Tree::from_data(&svg_data, &opt)?;

    // 2. Determine target size from the SVG's internal dimensions
    let size = tree.size().to_int_size();
    
    // 3. Create a pixel buffer (Pixmap)
    let mut pixmap = tiny_skia::Pixmap::new(size.width(), size.height())
        .ok_or("Failed to create pixmap")?;

    // 4. Render the tree onto the pixmap
    resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());

    // 5. Save to PNG
    pixmap.save_png("output.png")?;

    Ok(())
}
````

---

## 5. Advanced Usage <a name="advanced-usage"></a>

### Handling Fonts and Text <a name="handling-fonts-and-text"></a>

`resvg` does not bundle fonts. To render text, you must provide a `fontdb` (Font Database). In production (especially Docker or WASM), you should load specific font files to ensure consistency.

````rust
use resvg::{usvg, tiny_skia};
use fontdb::Database;

fn render_with_custom_font() -> Result<(), Box<dyn std::error::Error>> {
    let mut fontdb = Database::new();
    fontdb.load_font_file("assets/Roboto.ttf")?;
    
    let opt = usvg::Options {
        fontdb: std::sync::Arc::new(fontdb),
        ..Default::default()
    };

    let svg_data = r#"<svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">
        <text x="10" y="50" font-family="Roboto" font-size="20">Hello Rust</text>
    </svg>"#.as_bytes();

    let tree = usvg::Tree::from_data(svg_data, &opt)?;
    // ... render onto pixmap ...
    Ok(())
}
````

### Scaling and Fitting Logic <a name="scaling-and-fitting-logic"></a>

Because SVGs are vector-based, you often need to scale them to fit specific UI containers or icons.

````rust
fn render_icon_at_scale(rtree: &usvg::Tree, scale: f32) -> tiny_skia::Pixmap {
    let size = rtree.size().to_int_size().scale_by(scale).unwrap();
    let mut pixmap = tiny_skia::Pixmap::new(size.width(), size.height()).unwrap();
    
    // Apply a scale transform to the renderer
    let transform = tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(rtree, transform, &mut pixmap.as_mut());
    
    pixmap
}
````

### High-DPI Rendering for Print <a name="high-dpi-rendering"></a>

When generating images for PDFs or print, standard 72 DPI is insufficient. You can apply a "zoom" factor to generate high-resolution assets.

````rust
fn svg_to_high_dpi_png(svg_path: &str) -> Vec<u8> {
    let tree = usvg::Tree::from_data(&std::fs::read(svg_path).unwrap(), &usvg::Options::default()).unwrap();

    // 300 DPI is ~4.16x the standard 72 DPI
    let zoom = 4.166; 
    let size = tree.size().to_int_size().scale_by(zoom).unwrap();
    
    let mut pixmap = tiny_skia::Pixmap::new(size.width(), size.height()).unwrap();
    let transform = tiny_skia::Transform::from_scale(zoom, zoom);
    
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    pixmap.encode_png().unwrap()
}
````

---

## 6. Real-World Use Cases <a name="real-world-use-cases"></a>

1. **Dynamic Social Media Images (OG Images):** Generating preview cards for blog posts by swapping text in an SVG template and rendering to PNG.
1. **Game Assets:** Storing UI elements as SVGs and rasterizing them to textures at runtime to support both 1080p and 4K without pixelation.
1. **Desktop GUI Toolkits:** Powering icon rendering in frameworks like `egui`, `Slint`, and `Iced`.
1. **Headless Visual Regression Testing:** Using `resvg` as a deterministic "source of truth" to compare UI snapshots and catch layout bugs.
1. **Embedded Systems:** High-quality vector graphics on devices without GPUs.

---

## 7. Common Gotchas and Solutions <a name="common-gotchas-and-solutions"></a>

* **The "Missing Text" Problem:** If text doesn't appear, `usvg` couldn't find the font. **Solution:** Explicitly load fonts into `fontdb::Database`.
* **Coordinate Confusion:** By default, `resvg` renders 1:1. If you render a 10px SVG into a 100px buffer without a transform, it will be tiny in the corner. **Solution:** Use `tiny_skia::Transform::from_scale()`.
* **CSS Limitations:** `resvg` does not support external CSS or complex selectors like `:hover`. **Solution:** Inline your styles or pre-process SVGs with `svgo` using the `convertStyleToAttrs` plugin.
* **Allocation Overhead:** Parsing an SVG and creating a `fontdb` is expensive. **Solution:** Parse static icons once at startup and reuse the `usvg::Tree`.

---

## 8. Performance and Limitations <a name="performance-and-limitations"></a>

### ✅ Good Fit

* Server-side image generation.
* Static vector icons for apps/games.
* Deterministic rendering for tests.

### ❌ Bad Fit

* **Animations:** Does not support SMIL or CSS animations (renders frame 0).
* **Interactive Content:** No JavaScript support or DOM manipulation.
* **Real-time 60fps Full-screen Scenes:** CPU-bound software rasterization may be too slow for complex, full-screen animation compared to GPU solutions.

---

## 9. The Landscape: Alternatives and Comparisons <a name="the-landscape"></a>

|Library|Best For|Pros|Cons|
|:------|:-------|:---|:---|
|**`resvg`**|**General Rust Dev**|Pure Rust, high compliance.|CPU-only.|
|**`librsvg`**|**Legacy/Linux**|Battle-tested, supports Cairo.|Heavy C dependencies.|
|**`Vello`**|**High Performance**|GPU-accelerated (WGPU).|Experimental API, needs GPU.|
|**`ThorVG`**|**Embedded**|Tiny binary, Lottie support.|Non-native Rust (FFI).|
|**`tiny-skia`**|**Custom Logic**|Zero-dependency drawing.|No SVG parsing logic.|

---

## 10. Licensing and Compliance <a name="licensing-and-compliance"></a>

`resvg` uses the **MPL-2.0 (Mozilla Public License 2.0)**.

* **Commercial Use:** You can use `resvg` in closed-source proprietary applications.
* **Modifications:** If you modify `resvg`'s own source code and distribute it, you must make those specific modifications public.
* **Static Linking:** Statically linking `resvg` into your binary does **not** require you to open-source your entire application.

Sub-components like `tiny-skia` often use the **BSD-3-Clause**, which is even more permissive. Always verify the `Cargo.toml` of sub-crates for specific compliance requirements.