# Performance Optimization

Strategies for optimizing resvg rendering performance and memory usage.

## Caching Parsed SVGs

For repeated rendering of the same SVG, parse once and render multiple times:

```rust
use std::collections::HashMap;

struct SvgCache {
    cache: HashMap<String, usvg::Tree>,
}

impl SvgCache {
    fn new() -> Self {
        Self { cache: HashMap::new() }
    }

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

// Usage
let mut cache = SvgCache::new();
let tree = cache.get_or_parse("icon.svg")?;

// Render multiple times without reparsing
for i in 0..100 {
    let mut pixmap = /* create pixmap */;
    resvg::render(tree, /* options */);
}
```

## Memory Optimization

### Reusing Pixmaps

Instead of creating new pixmaps for each render, reuse existing ones:

```rust
// Bad: creates new pixmap each time
for svg in svgs {
    let mut pixmap = tiny_skia::Pixmap::new(width, height)?;
    render_to_pixmap(&svg, &mut pixmap);
}

// Good: reuse pixmap
let mut pixmap = tiny_skia::Pixmap::new(width, height)?;
for svg in svgs {
    pixmap.fill(tiny_skia::Color::TRANSPARENT); // Clear previous
    render_to_pixmap(&svg, &mut pixmap);
    save_pixmap(&pixmap);
}
```

### Appropriate Dimensions

Use `FitTo::Width` or `FitTo::Height` to render at specific dimensions rather than full resolution:

```rust
// Render at lower resolution for thumbnails
fn render_thumbnail(svg_data: &[u8], width: u32)
    -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let tree = usvg::Tree::from_data(svg_data, &usvg::Options::default())?;

    // Calculate height maintaining aspect ratio
    let aspect_ratio = tree.size().height() / tree.size().width();
    let height = (width as f32 * aspect_ratio) as u32;

    let mut pixmap = tiny_skia::Pixmap::new(width, height)?;

    resvg::render(&tree, usvg::FitTo::Width(width),
                  tiny_skia::Transform::identity(), &mut pixmap.as_mut());

    Ok(pixmap.encode_png()?)
}
```

### Font Database Optimization

Only load necessary fonts instead of all system fonts:

```rust
// Bad: loads hundreds of fonts
let mut fontdb = usvg::fontdb::Database::new();
fontdb.load_system_fonts(); // Can be slow and memory-heavy

// Good: load only needed fonts
let mut fontdb = usvg::fontdb::Database::new();
fontdb.load_font_file("assets/Roboto-Regular.ttf")?;
fontdb.load_font_file("assets/Roboto-Bold.ttf")?;
fontdb.set_generic_families();
```

## Preprocessing SVGs

Remove unnecessary elements before parsing:

```rust
fn preprocess_svg(svg_data: &[u8]) -> Result<String, Box<dyn std::error::Error>> {
    let svg_string = String::from_utf8(svg_data.to_vec())?;

    // Remove comments
    let re = regex::Regex::new(r"<!--.*?-->")?;
    let cleaned = re.replace_all(&svg_string, "");

    // Remove metadata
    let re = regex::Regex::new(r"<metadata>.*?</metadata>")?;
    let cleaned = re.replace_all(&cleaned, "");

    // Remove unsupported elements
    let re = regex::Regex::new(r"<(script|animate|animateTransform).*?>")?;
    let cleaned = re.replace_all(&cleaned, "");

    Ok(cleaned.to_string())
}
```

## Parallel Rendering

For batch processing, render multiple SVGs in parallel:

```rust
use rayon::prelude::*;

fn batch_render(svg_paths: &[String]) -> Vec<Result<(), Box<dyn std::error::Error>>> {
    svg_paths.par_iter().map(|path| {
        let svg_data = std::fs::read(path)?;
        let tree = usvg::Tree::from_data(&svg_data, &usvg::Options::default())?;

        let size = tree.size();
        let mut pixmap = tiny_skia::Pixmap::new(
            size.width() as u32,
            size.height() as u32
        )?;

        resvg::render(&tree, usvg::FitTo::Original,
                      tiny_skia::Transform::identity(), &mut pixmap.as_mut());

        let output_path = path.replace(".svg", ".png");
        pixmap.save_png(&output_path)?;

        Ok(())
    }).collect()
}
```

## Performance Benchmarks

Typical performance characteristics:

| SVG Type | Size | Parse Time | Render Time (1x) | Render Time (4x) |
|----------|------|------------|------------------|------------------|
| Simple icon | 5KB | ~1ms | ~5ms | ~20ms |
| Complex illustration | 100KB | ~10ms | ~50ms | ~200ms |
| Text-heavy document | 50KB | ~15ms | ~30ms | ~120ms |

Notes:
- Parse time is one-time cost (cache the tree!)
- Render time scales roughly linearly with pixel count
- Text-to-path conversion adds ~10-20% to parse time

## Optimization Checklist

- [ ] Cache parsed `usvg::Tree` for repeated renders
- [ ] Reuse pixmaps instead of allocating new ones
- [ ] Load only necessary fonts, not all system fonts
- [ ] Use appropriate `FitTo` modes for target dimensions
- [ ] Preprocess SVGs to remove unnecessary elements
- [ ] Use parallel rendering for batch operations
- [ ] Profile with `cargo flamegraph` to identify bottlenecks
- [ ] Consider memory pool allocation for high-throughput scenarios
