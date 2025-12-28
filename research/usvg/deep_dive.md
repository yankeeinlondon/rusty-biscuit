# Deep Dive: usvg (Micro/SVG)

`usvg` is a strict, lossy, and lightweight SVG parsing and processing library written in Rust. It serves as the foundational processing engine for the **resvg** rendering ecosystem. Unlike general-purpose XML parsers or DOM manipulators, `usvg` is designed to bridge the gap between the massive, complex SVG specification and the needs of modern rendering engines.

---

## Table of Contents

1. [Core Philosophy: The "u" in usvg](#core-philosophy)
1. [Architecture and Functional Footprint](#architecture)
1. [The `usvg::Tree` Structure](#the-tree-structure)
1. [Getting Started: Basic Usage](#getting-started)
1. [Advanced Configuration (Fonts & Resources)](#advanced-configuration)
1. [Integration Partners: resvg, tiny-skia, and Vello](#integration-partners)
1. [Comparison with Other Libraries](#comparison)
1. [Practical Use Cases](#practical-use-cases)
1. [Gotchas and Limitations](#gotchas)
1. [Licensing and Compliance](#licensing)

---

## 1. Core Philosophy <a name="core-philosophy"></a>

The primary value proposition of `usvg` is **simplification and normalization**. The SVG specification is notoriously large and redundant, offering multiple ways to achieve the same visual result (e.g., inline styles vs. CSS blocks vs. presentation attributes).

`usvg` acts as a **pre-processor** that converts this "chaotic" SVG data into a much smaller, predictable subset. It is "lossy" not in terms of visual quality, but in terms of document structure: it discards editor-specific metadata, flattens groups, and converts all geometric primitives (rectangles, circles, etc.) into paths.

---

## 2. Architecture and Functional Footprint <a name="architecture"></a>

The functionality of `usvg` is divided into four distinct stages:

### A. Loading & Parsing

`usvg` uses `roxmltree` for strict XML parsing. It accepts SVG data from files, memory slices, or strings. Loading is controlled via `usvg::Options`, which define parameters like DPI and resource paths.

### B. Simplification (Normalization)

This is the core stage where the library earns its name:

* **CSS & Style Resolution:** All `<style>` blocks and `class` attributes are cascaded and resolved into concrete presentation attributes.
* **Attribute Inheritance:** Inherited properties (like `fill` or `stroke-width`) are computed explicitly for every node.
* **Primitive Conversion:** Basic shapes like `<rect>`, `<circle>`, `<ellipse>`, `<line>`, `<polyline>`, and `<polygon>` are converted into `<path>` data.
* **Transform Application:** It prepares coordinate systems so the renderer doesn't have to calculate complex inheritance chains at runtime.

### C. Tree Representation

The result is a `usvg::Tree`, a simplified, mostly immutable structure containing only what is necessary for rendering.

### D. Export

While primarily used for internal rendering, the simplified tree can be exported back to a "normalized" SVG string.

---

## 3. The `usvg::Tree` Structure <a name="tree-structure"></a>

The output of the parsing process is a `usvg::Tree`. Its hierarchy is composed of:

* **`usvg::Node`**: An enum representing a Root, Group, Path, or Image.
* **`usvg::Path`**: Contains `usvg::PathSegment` sequences (MoveTo, LineTo, CurveTo, etc.).
* **`usvg::Paint`**: Represents a resolved fill or stroke, which can be a Solid Color, LinearGradient, RadialGradient, or Pattern.
* **`usvg::Image`**: Decoded raster data (PNG/JPG) ready for texture mapping.

---

## 4. Getting Started: Basic Usage <a name="getting-started"></a>

To use `usvg`, you typically parse a string and traverse the resulting tree. Note that `usvg` identifies paths even if the original SVG used shapes like `<rect>`.

````rust
use usvg::{Tree, Options, TreeParsing};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let svg_data = r##"
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
        <style> .my-rect { fill: red; } </style>
        <rect x="10" y="10" width="80" height="80" class="my-rect" stroke="blue" stroke-width="2"/>
    </svg>
    "##;

    // 1. Parse with default options
    let opts = Options::default();
    let tree = Tree::from_str(svg_data, &opts)?;

    // 2. Traverse and find the path (the rect was converted to a path)
    for node in tree.root().descendants() {
        if let usvg::Node::Path(path) = node {
            // Check the resolved fill color
            if let usvg::Paint::Color(color) = path.fill().unwrap().paint() {
                println!("Found a path with fill color: {:?}", color);
            }
            
            // Check stroke width (resolved in user units)
            if let Some(stroke) = path.stroke() {
                println!("Stroke width: {}", stroke.width().get());
            }
        }
    }

    Ok(())
}
````

---

## 5. Advanced Configuration <a name="advanced-configuration"></a>

### Font Handling

SVGs are highly dependent on fonts. If `usvg` cannot find a font, text may vanish. You must provide a `fontdb` (font database).

````rust
use usvg::{fontdb, Tree, Options, TreeParsing};

let mut fontdb = fontdb::Database::new();
fontdb.load_system_fonts(); // Crucial for text rendering

let opts = Options {
    fontdb: std::sync::Arc::new(fontdb),
    dpi: 96.0, // Standard screen DPI
    ..Default::default()
};

let tree = Tree::from_str(svg_input, &opts)?;
````

### External Resources

If your SVG references external images (`<image href="logo.png">`), you must specify the directory where `usvg` should look for them:

````rust
let opts = usvg::Options {
    resources_dir: Some(std::path::PathBuf::from("/assets/images")),
    ..Default::default()
};
````

---

## 6. Integration Partners <a name="integration-partners"></a>

`usvg` is rarely used in isolation. It is the "brain," while other libraries provide the "eyes" and "hands."

|Library|Role|Why use with `usvg`?|
|:------|:---|:-------------------|
|**resvg**|Renderer|The high-level API that turns a `usvg::Tree` into pixels.|
|**tiny-skia**|2D Engine|The default CPU-based drawing backend for `resvg`.|
|**vello**|GPU Renderer|A high-performance engine that uses `usvg` to build GPU scenes.|

---

## 7. Comparison with Other Libraries <a name="comparison"></a>

|Library|Primary Use Case|Complexity|Pure Rust?|
|:------|:---------------|:---------|:---------|
|**usvg**|**Normalizing/Simplifying SVG for rendering**|**Medium**|**Yes**|
|**librsvg**|High-fidelity rendering (standard Linux tool)|High|No (C deps)|
|**svg**|Simple XML generation/parsing|Low|Yes|
|**svgtypes**|Parsing specific attributes (Paths/Colors)|Low|Yes|
|**Vello**|Experimental GPU-accelerated rendering|High|Yes|

---

## 8. Practical Use Cases <a name="practical-use-cases"></a>

1. **High-Fidelity Rasterization:** Converting SVG to PNG/JPEG for thumbnails or web assets.
1. **CNC and Pen Plotters:** Because `usvg` flattens all shapes into paths and applies transforms, it is ideal for generating toolpaths for laser cutters or plotters.
1. **Game UI Engines:** Providing a clean, resolved tree to convert vector icons into GPU vertex buffers or Signed Distance Fields (SDFs).
1. **SVG Optimization:** Stripping editor metadata and converting complex CSS into simple attributes to create "minified" SVGs.
1. **Static Analysis:** Auditing large icon libraries for brand color compliance or performance-heavy complexity (node counting).

---

## 9. Gotchas and Limitations <a name="gotchas"></a>

* **Missing Text:** Usually caused by not loading system fonts into the `fontdb`. `usvg` does not "guess" fonts; it is strict.
* **Coordinate Space:** If an SVG appears the wrong size, check the `viewBox` vs. `width/height`. `usvg` calculates logical dimensions based on these; always check `tree.size()`.
* **CSS Limitations:** `usvg` supports a subset of CSS 2.1. It does **not** support `calc()`, CSS variables (custom properties), or interactive pseudo-classes like `:hover`.
* **No Animation:** `usvg` does not support SMIL (`<animate>`). It captures only the "initial state" of the SVG.
* **It Doesn't Render:** A common point of confusion is that `usvg` creates the data structure but does not draw pixels. You **must** use `resvg` or `tiny-skia` to actually output an image.

---

## 10. Licensing and Compliance <a name="licensing"></a>

`usvg` is distributed under the **Mozilla Public License 2.0 (MPL-2.0)**.

* **Weak Copyleft:** You can use `usvg` in proprietary, closed-source applications.
* **Modification Clause:** If you modify the source code of `usvg` itself, those specific changes must be made available under the MPL-2.0.
* **Application Impact:** You do **not** need to open-source your entire application just for linking against or using `usvg`.