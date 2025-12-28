---
name: resvg
description: Expert knowledge for rendering static SVG files to raster formats using the resvg Rust crate, a high-performance pure-Rust library from the Linebender ecosystem that ensures cross-platform consistency and memory safety for SVG-to-PNG conversion, icon generation, thumbnails, and server-side rendering
last_updated: 2025-12-26T00:00:00Z
hash: 3fb03d866a5b8238
---

# resvg - Pure Rust SVG Rendering

High-performance SVG rendering library for converting static SVG to raster formats (PNG). Part of the Linebender ecosystem. Prioritizes memory safety, cross-platform reproducibility, and rendering correctness.

## Core Principles

- **Two-stage pipeline**: Parse with `usvg` → Render with `resvg` → Output with `tiny-skia`
- **Static SVG subset only**: No animations, scripting, or dynamic features
- **Text-to-path conversion**: Convert text to vector paths for font-independent rendering
- **Explicit font loading**: Use `fontdb.load_system_fonts()` or embed fonts for portability
- **Cache parsed trees**: Parse once, render multiple times for performance
- **Cross-platform consistency**: Identical pixel-perfect output on all architectures
- **Memory-safe by design**: Pure Rust with minimal unsafe code
- **Separation of concerns**: Use `usvg` directly for custom rendering backends

## Quick Reference

### Basic SVG to PNG

```rust
use resvg::{tiny_skia, usvg};

fn render_svg_to_png() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Parse SVG
    let svg_data = std::fs::read("example.svg")?;
    let tree = usvg::Tree::from_data(&svg_data, &usvg::Options::default())?;

    // 2. Create pixmap
    let size = tree.size();
    let mut pixmap = tiny_skia::Pixmap::new(size.width() as u32, size.height() as u32)
        .ok_or("Failed to create pixmap")?;

    // 3. Render
    resvg::render(&tree, usvg::FitTo::Original,
                  tiny_skia::Transform::identity(), &mut pixmap.as_mut());

    // 4. Save
    pixmap.save_png("output.png")?;
    Ok(())
}
```

### With Font Support

```rust
use usvg::{TreeParsing, TreeTextToPath};

let mut opt = usvg::Options::default();
let mut fontdb = usvg::fontdb::Database::new();
fontdb.load_system_fonts(); // or load_font_file() for custom fonts

let tree = usvg::Tree::from_str(svg_data, &opt, &fontdb)?;
```

### Scaling to Specific Dimensions

```rust
let rtree = resvg::Tree::from_usvg(&tree);
let target_width = 3840.0;
let scale = target_width / rtree.size.width();

let mut pixmap = tiny_skia::Pixmap::new(
    target_width as u32,
    (rtree.size.height() * scale) as u32
)?;

let transform = tiny_skia::Transform::from_scale(scale, scale);
resvg::render(&rtree, transform, &mut pixmap.as_mut());
```

## Related Skills

### Part Of

- **linebender** - An ecosystem of crates which address graphics and text needs; the strength of this ecosystem lies in its modular design and comprehensive coverage. Use this skill when you need broader context or complementary tools for working with resvg.

## Topics

### Architecture & Features

- [Rendering Pipeline](./pipeline.md) - Two-stage architecture, parsing vs rendering
- [Font Handling](./fonts.md) - Font databases, text-to-path conversion, embedding fonts
- [Performance Optimization](./performance.md) - Caching, memory management, scaling strategies

### Practical Patterns

- [Common Use Cases](./use-cases.md) - Icon generation, web services, desktop apps, report generation
- [Gotchas & Workarounds](./gotchas.md) - Font issues, text limitations, SVG compatibility, security

### Integration Examples

- [Web Service Integration](./web-service.md) - Actix-web example, dynamic rendering
- [Dynamic SVG Modification](./modification.md) - Changing colors, hiding elements, tree manipulation

## Common Patterns

### Error Handling

```rust
enum RenderingError {
    Io(io::Error),
    SvgParsing(String),
    Rendering(String),
    InvalidInput(String),
}

fn render_with_validation(svg_path: &str) -> Result<(), RenderingError> {
    let svg_data = std::fs::read(svg_path)?;

    let tree = usvg::Tree::from_data(&svg_data, &usvg::Options::default())
        .map_err(|e| RenderingError::SvgParsing(e.to_string()))?;

    let size = tree.size();
    if size.width() == 0.0 || size.height() == 0.0 {
        return Err(RenderingError::InvalidInput("Zero size SVG".to_string()));
    }

    // ... render
    Ok(())
}
```

### SVG Caching for Repeated Renders

```rust
use std::collections::HashMap;

struct SvgCache {
    cache: HashMap<String, usvg::Tree>,
}

impl SvgCache {
    fn get_or_parse(&mut self, svg_path: &str)
        -> Result<&usvg::Tree, Box<dyn std::error::Error>> {
        if !self.cache.contains_key(svg_path) {
            let svg_data = std::fs::read(svg_path)?;
            let tree = usvg::Tree::from_data(&svg_data, &usvg::Options::default())?;
            self.cache.insert(svg_path.to_string(), tree);
        }
        Ok(self.cache.get(svg_path).unwrap())
    }
}
```

## When to Use resvg

**Best for:**
- Rust-native applications requiring memory safety
- Cross-platform consistency (identical rendering everywhere)
- Serverless environments (small binary, minimal dependencies)
- Security-sensitive contexts (processing untrusted SVGs)
- Custom rendering pipelines (modular usvg/resvg separation)

**Consider alternatives when:**
- Animation support is required (SMIL, CSS animations)
- Native text rendering needed (exact browser matching)
- Existing C/C++ graphics infrastructure (Cairo, Skia)

## Resources

- [Official Docs](https://docs.rs/resvg)
- [GitHub](https://github.com/RazrFalcon/resvg)
- [Linebender Ecosystem](https://linebender.org/)
- [usvg Crate](https://docs.rs/usvg)
- [tiny-skia Crate](https://docs.rs/tiny-skia)
