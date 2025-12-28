# Gotchas and Workarounds

Common issues when using resvg and how to resolve them.

## Font Handling Challenges

### Gotcha: Missing Custom Fonts

**Problem**: SVG references font not available on target system

**Impact**: Text renders in wrong font or disappears

**Workaround**: Load fonts explicitly into font database

```rust
use std::path::Path;

fn setup_font_database() -> usvg::fontdb::Database {
    let mut fontdb = usvg::fontdb::Database::new();

    // Load system fonts
    fontdb.load_system_fonts();

    // Load custom fonts
    let custom_fonts_dir = Path::new("/path/to/custom/fonts");
    if custom_fonts_dir.exists() {
        fontdb.load_fonts_dir(custom_fonts_dir);
    }

    // Load specific font files
    fontdb.load_font_file("/path/to/CustomFont.ttf").ok();

    fontdb.set_generic_families();
    fontdb
}
```

For serverless environments without fonts, **embed fonts** directly:

```rust
// Load font from memory (useful for embedded fonts)
let font_data = include_bytes!("../assets/CustomFont.ttf");
fontdb.load_font_data(font_data.to_vec());
```

### Gotcha: Different Font Fallback Behavior

**Problem**: resvg uses its own font matching algorithm, different from browsers

**Impact**: Different font selected than expected

**Workaround**: Provide explicit font fallback chain in SVG:

```xml
<text font-family="Roboto, Arial, Helvetica, sans-serif">
```

## Text Rendering Limitations

### Gotcha: No Native Text Shaping

**Problem**: resvg doesn't use platform text shaping engines

**Impact**: Complex scripts (Arabic, Indic) and advanced typography (ligatures, kerning) may not render correctly

**Workaround**: For complex scripts, use text-to-path conversion with proper fonts:

```rust
let mut opts = usvg::Options::default();
opts.fontdb.load_system_fonts();
opts.text_rendering_mode = usvg::TextRenderingMode::OptimizeLegibility;

let tree = usvg::Tree::from_data(&svg_data, &opts)?;
```

### Gotcha: No Text Selection or Measurement APIs

**Problem**: Unlike browsers, resvg doesn't provide APIs for text selection or precise measurement after rendering

**Workaround**: Use usvg's tree parsing to access text metrics before rendering:

```rust
// Access text elements and their metrics
for node in tree.root().descendants() {
    if let usvg::Node::Text(text) = node {
        let bbox = text.calculate_bbox();
        let font = text.font;
        let font_size = text.font_size;
        println!("Text found: {:?} at {:?}", text.chunks, bbox);
    }
}
```

## SVG Compatibility Issues

### Gotcha: Unsupported SVG Features

**Problem**: resvg explicitly excludes dynamic SVG features

**Unsupported features**:
- Animations (SMIL, CSS animations)
- Scripting (JavaScript)
- Interactivity (event handlers, cursor elements)
- Audio and video elements

**Workaround**: Preprocess SVGs to remove unsupported features:

```rust
fn preprocess_svg(svg_data: &[u8]) -> Result<String, Box<dyn std::error::Error>> {
    let svg_string = String::from_utf8(svg_data.to_vec())?;

    // Remove script tags
    let re = regex::Regex::new(r#"<script.*?>.*?</script>"#)?;
    let cleaned = re.replace_all(&svg_string, "");

    // Remove animation elements
    let re = regex::Regex::new(r#"<(animate|animateTransform|set).*?/>"#)?;
    let cleaned = re.replace_all(&cleaned, "");

    Ok(cleaned.to_string())
}
```

### Gotcha: Limited CSS Support

**Problem**: resvg has limited support for advanced CSS features (flexbox, grid, custom properties)

**Impact**: Complex CSS layouts may not render correctly

**Workaround**: Simplify CSS during preprocessing:

```rust
fn preprocess_css(css: &str) -> String {
    // Remove comments
    let cleaned = regex::Regex::new(r"<!--.*?-->").unwrap()
        .replace_all(css, "")
        .to_string();

    // Remove unsupported CSS features
    let no_grid = regex::Regex::new(r"display:\s*grid;?").unwrap()
        .replace_all(&cleaned, "")
        .to_string();

    no_grid
}
```

## Resource Loading and Security

### Gotcha: Restricted External Resource Loading

**Problem**: By default, resvg only loads external resources in same directory as base SVG or subdirectories

**Reason**: Security - prevents malicious SVGs from accessing arbitrary files

**Workaround for trusted content**:

```rust
// Configure resource loading options
let mut opts = usvg::Options::default();
opts.fontdb.load_system_fonts();

// Allow loading resources from parent directories (use with caution!)
opts.resources_dir = std::path::Path::new("/trusted/path").to_path_buf();
```

**Workaround for untrusted content**: Embed external resources as data URLs:

```rust
fn embed_external_images(svg: &str, base_path: &Path)
    -> Result<String, Box<dyn std::error::Error>> {
    let re = regex::Regex::new(r#"<image\s+[^>]*href="([^"]+)"[^>]*/>"#)?;

    let processed = re.replace_all(svg, |caps: &regex::Captures| {
        let href = &caps[1];
        if !href.starts_with("data:") {
            let image_path = base_path.join(href);
            if let Ok(image_data) = std::fs::read(&image_path) {
                let mime_type = mime_guess::from_path(&image_path)
                    .first_or_octet_stream()
                    .to_string();
                let encoded = base64::encode(&image_data);
                return format!(r#"<image href="data:{};base64,{}"/>"#, mime_type, encoded);
            }
        }
        caps[0].to_string()
    });

    Ok(processed.to_string())
}
```

## Memory and Performance Issues

### Gotcha: Large SVG Memory Usage

**Problem**: Very large or complex SVGs consume significant memory during parsing

**Impact**: Out-of-memory errors in constrained environments

**Workaround**: Stream processing for very large files:

```rust
fn render_large_svg_chunked(svg_path: &str, chunk_size: u32)
    -> Result<(), Box<dyn std::error::Error>> {
    let svg_data = std::fs::read(svg_path)?;
    let tree = usvg::Tree::from_data(&svg_data, &usvg::Options::default())?;

    let size = tree.size();

    // Render in tiles
    for y in (0..size.height() as u32).step_by(chunk_size as usize) {
        for x in (0..size.width() as u32).step_by(chunk_size as usize) {
            let mut pixmap = tiny_skia::Pixmap::new(chunk_size, chunk_size)?;

            // Translate to render specific tile
            let transform = tiny_skia::Transform::from_translate(
                -(x as f32),
                -(y as f32)
            );

            resvg::render(&tree, usvg::FitTo::Original, transform, &mut pixmap.as_mut());

            // Process or save tile
            pixmap.save_png(&format!("tile_{}_{}.png", x, y))?;
        }
    }

    Ok(())
}
```

### Gotcha: Zero-Size SVG

**Problem**: SVG has width="0" or height="0"

**Impact**: Pixmap creation fails

**Workaround**: Validate dimensions before creating pixmap:

```rust
let size = tree.size();
if size.width() == 0.0 || size.height() == 0.0 {
    return Err("SVG has zero width or height".into());
}

// Ensure minimum size
let width = size.width().max(1.0) as u32;
let height = size.height().max(1.0) as u32;
```

## Common Error Patterns

### Parsing Errors

**Invalid XML**: Ensure SVG is valid XML before parsing

```rust
match usvg::Tree::from_data(&svg_data, &usvg::Options::default()) {
    Ok(tree) => { /* render */ },
    Err(e) => {
        eprintln!("SVG parsing failed: {}", e);
        // Log the SVG content for debugging
        eprintln!("SVG content: {}", String::from_utf8_lossy(&svg_data));
    }
}
```

### Pixmap Creation Failures

**Dimensions too large**: Check pixmap dimensions before creation

```rust
const MAX_DIMENSION: u32 = 16384; // 16K

if width > MAX_DIMENSION || height > MAX_DIMENSION {
    return Err(format!(
        "Dimensions too large: {}x{} (max {})",
        width, height, MAX_DIMENSION
    ).into());
}
```

### Font Loading Failures

**Missing font file**: Handle font loading errors gracefully

```rust
if let Err(e) = fontdb.load_font_file("CustomFont.ttf") {
    eprintln!("Warning: Failed to load custom font: {}", e);
    // Continue with fallback fonts
}
```
