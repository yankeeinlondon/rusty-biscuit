---

## name: usvg
description: Expert knowledge for parsing, simplifying, and processing SVG files using the Rust 'usvg' crate. Use this for SVG-to-raster conversion, CNC/plotter toolpath generation, or SVG asset optimization.

# usvg (Micro SVG) Skill

`usvg` is a strict, lossy, and lightweight SVG parsing/processing library in Rust. It simplifies the complex SVG spec into a predictable `usvg::Tree` where all shapes are converted to paths and CSS is resolved.

## Core Capabilities

* **Simplification**: Converts `<rect>`, `<circle>`, etc., into `<path>`.
* **CSS Resolution**: Cascades and resolves all `<style>` and `class` attributes into concrete properties.
* **Normalization**: Handles `viewBox` and coordinate transformations, preparing the tree for rendering.
* **Text Processing**: Maps text to specific glyph positions (requires a font database).

## Quick Start: Parsing an SVG

````rust
use usvg::{Tree, Options, fontdb};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let svg_data = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
        <circle cx="50" cy="50" r="40" fill="red" />
    </svg>"#;

    // 1. Setup Options & Fonts (Critical for text)
    let mut fontdb = fontdb::Database::new();
    fontdb.load_system_fonts();
    
    let opts = Options {
        fontdb: std::sync::Arc::new(fontdb),
        ..Default::default()
    };

    // 2. Parse into a simplified tree
    let tree = Tree::from_str(svg_data, &opts)?;

    // 3. Inspect the tree (The circle is now a Path)
    for node in tree.root().descendants() {
        if let usvg::Node::Path(path) = node {
            println!("Found path with fill: {:?}", path.fill());
        }
    }
    Ok(())
}
````

## Detailed Documentation

* [Usage Patterns & Examples](PATTERNS.md)
* [Integration (resvg, tiny-skia, vello)](INTEGRATIONS.md)
* [Troubleshooting & Gotchas](TROUBLESHOOTING.md)

## When to use usvg

* **Use when**: You need to render SVGs to pixels, generate CNC paths, or analyze SVG geometry programmatically.
* **Avoid when**: You need an interactive SVG editor (usvg is lossy) or need SMIL animations/dynamic CSS (calc(), variables).