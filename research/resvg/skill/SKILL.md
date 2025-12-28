
````yaml
name: resvg
description: Expert guidance on using the `resvg` crate for high-performance, spec-compliant SVG rendering in Rust. This skill covers parsing with `usvg`, rasterizing with `tiny-skia`, font management, and scaling strategies.
````

# resvg: SVG Rendering in Rust

`resvg` is the gold standard for SVG rendering in the Rust ecosystem. It focuses on converting SVG vector data into raster bitmaps (pixels) with high specification compliance, outperforming many browsers in rendering accuracy.

## Core Architecture

`resvg` is part of a modular ecosystem. To use it effectively, you must coordinate three layers:

1. **`usvg`**: The parser and optimizer. Simplifies complex SVGs into a `usvg::Tree`.
1. **`resvg`**: The renderer. Orchestrates the drawing logic.
1. **`tiny-skia`**: The software rasterizer backend (CPU-based).

## Quick Start: Basic Rendering

The most common pattern is converting an SVG file to a PNG.

````rust
use resvg::{usvg, tiny_skia};

fn render_svg(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Parse SVG into optimized Tree
    let opt = usvg::Options::default();
    let svg_data = std::fs::read(path)?;
    let tree = usvg::Tree::from_data(&svg_data, &opt)?;

    // 2. Prepare Pixmap (buffer)
    let size = tree.size().to_int_size();
    let mut pixmap = tiny_skia::Pixmap::new(size.width(), size.height()).unwrap();

    // 3. Render
    resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());

    // 4. Output
    pixmap.save_png("output.png")?;
    Ok(())
}
````

## When to use resvg

* ✅ **Server-side generation**: OG images, dynamic badges, or PDF assets.
* ✅ **Game UI**: Rasterizing vector icons to textures at runtime.
* ✅ **Desktop Apps**: Rendering icons in frameworks like `egui` or `iced`.
* ✅ **Deterministic Testing**: Visual regression testing with a stable "source of truth."

## Detailed Guides

* [Common Gotchas & Troubleshooting](TROUBLESHOOTING.md): Fix missing text and CSS issues.
* [Advanced Scaling & Fonts](ADVANCED.md): Handling DPI, `fontdb`, and coordinate systems.
* [Ecosystem & Integration](ECOSYSTEM.md): How `usvg`, `tiny-skia`, and `image` work together.
* [Alternatives & Comparisons](ALTERNATIVES.md): When to use `vello` or `librsvg` instead.