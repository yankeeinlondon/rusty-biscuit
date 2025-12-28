---
name: resvg
description: Comprehensive guide to the resvg Rust crate for high-performance, cross-platform SVG rendering
created: 2025-12-26
last_updated: 2025-12-26T00:00:00Z
hash: 57f6dc3adc25bc82
tags:
  - rust
  - svg
  - rendering
  - graphics
  - linebender
---

# resvg: High-Performance SVG Rendering in Rust

## Table of Contents

- [Introduction](#introduction)
- [Key Features and Design Philosophy](#key-features-and-design-philosophy)
- [Core Architecture](#core-architecture)
- [Getting Started](#getting-started)
- [Advanced Usage](#advanced-usage)
- [Common Use Cases](#common-use-cases)
- [Text and Font Handling](#text-and-font-handling)
- [Performance Optimization](#performance-optimization)
- [Gotchas and Workarounds](#gotchas-and-workarounds)
- [Comparison with Alternatives](#comparison-with-alternatives)
- [Ecosystem Integration](#ecosystem-integration)
- [Quick Reference](#quick-reference)

## Introduction

**resvg** is a high-performance, pure-Rust SVG rendering library that provides efficient and accurate rendering of SVG files. It is designed as a fast, small, and portable solution for processing Scalable Vector Graphics with the ambitious goal of supporting the entire SVG specification.

The library is part of the [Linebender](https://linebender.org/) ecosystem of Rust graphics tools and is actively maintained with regular releases (currently at version 0.45.1 as of April 2025). resvg is distributed under the MIT license and has garnered nearly 8 million downloads on crates.io, demonstrating its widespread adoption in the Rust community.

### What Makes resvg Special

resvg is particularly notable for its focus on:

- **Memory Safety**: Pure-Rust implementation with minimal `unsafe` code
- **Reproducibility**: Identical pixel-perfect results across different architectures (e.g., x86 Windows vs. ARM macOS)
- **Cross-Platform Consistency**: No dependencies on system libraries ensures consistent rendering
- **Rendering Correctness**: Adheres strictly to the SVG static specification with approximately 1,600 regression tests

Unlike many other libraries, resvg doesn't just draw shapes; it handles complex filters, masks, and paths with high fidelity.

## Key Features and Design Philosophy

resvg distinguishes itself from other SVG rendering libraries through several key design principles:

### Pure Rust Implementation

Unlike many alternatives that wrap C/C++ libraries, resvg is implemented entirely in Rust with minimal `unsafe` code. This eliminates entire classes of memory safety vulnerabilities while maintaining high performance.

### Separation of Concerns

The library cleanly separates SVG parsing (handled by the `usvg` crate) from rendering (handled by `resvg`). This modular approach allows developers to use just the parsing components with alternative rendering backends.

Key distinction:
- **`usvg`**: The heavy lifter that "cleans" the SVG, converting complex CSS, relative units, and nested groups into a simplified tree
- **`resvg`**: The renderer that draws the simplified tree onto a pixel buffer

### Extensive Test Coverage

resvg maintains approximately 1,600 SVG-to-PNG regression tests to ensure rendering correctness. These tests are publicly available and serve as a valuable resource for validating other SVG implementations.

### Static SVG Subset Focus

The library specifically targets the **static** SVG subset, intentionally excluding:
- Animations (SMIL, CSS animations)
- Scripting (JavaScript)
- Interactivity (event handlers, cursor elements)
- Audio and video elements

This focus allows for a smaller, more maintainable codebase while covering the most common use cases.

### Cross-Platform Reproducibility

By avoiding system libraries and dependencies, resvg ensures consistent rendering results across different platforms and architectures, making it ideal for reproducible build pipelines and cross-platform applications.

## Core Architecture

### Rendering Pipeline

The resvg rendering pipeline follows a two-stage process that separates SVG parsing from rasterization:

```
SVG Input → usvg Parser → SVG Tree → resvg Renderer → Output (Pixmap/PNG)
```

This architecture enables several important capabilities:

- **Pre-parsing Optimization**: SVG documents can be parsed once and rendered multiple times with different parameters
- **Multiple Rendering Backends**: Developers can implement custom renderers using the `usvg` tree representation
- **Efficient Caching**: Parsed SVG trees can be cached in memory for repeated rendering operations

### SVG Parsing with usvg

The **usvg** (Universal SVG) crate handles the initial parsing and normalization of SVG documents. It performs several critical operations:

- **XML Parsing**: Converts raw SVG XML into a structured tree representation
- **CSS Resolution**: Processes stylesheets and resolves CSS properties
- **Path Flattening**: Converts complex path data into simplified forms
- **Font Resolution**: Maps font families to available fonts in the font database
- **Unit Conversion**: Normalizes various units (pixels, percentages, points) to a consistent format

The parsing process produces a `usvg::Tree` that serves as the input to the rendering stage.

### Rendering Capabilities

resvg supports comprehensive rendering of the static SVG subset, including:

- **Path Rendering**: Complex shapes, bezier curves, and geometric primitives
- **Text Rendering**: Including text layout, font selection, and text metrics
- **Filter Effects**: Blurring, lighting effects, and compositing operations
- **Gradient and Pattern Fills**: Linear and radial gradients, repeating patterns
- **Clipping and Masking**: Complex clipping paths and opacity masks
- **Image Embedding**: Raster images embedded within SVG documents
- **Transformations**: Translation, rotation, scaling, and skewing

The library uses **tiny-skia** as its rendering backend, which provides a software rasterizer that is both fast and precise. This approach ensures that resvg doesn't depend on platform-specific graphics libraries like Cairo or Skia.

## Getting Started

### Dependencies

Add these three crates to your `Cargo.toml`:

```toml
[dependencies]
resvg = "0.43.0"
usvg = "0.43.0"
tiny-skia = "0.11.4"
```

### Basic Rendering: SVG to PNG

The following example demonstrates the fundamental workflow of loading an SVG file and rendering it to a PNG image:

```rust
use resvg::{tiny_skia, usvg};

fn render_svg_to_png() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Load and parse the SVG file
    let svg_data = std::fs::read("example.svg")?;
    let tree = usvg::Tree::from_data(&svg_data, &usvg::Options::default())?;

    // 2. Get the SVG dimensions
    let size = tree.size();

    // 3. Create a pixmap (target surface) for rendering
    let mut pixmap = tiny_skia::Pixmap::new(size.width() as u32, size.height() as u32)
        .ok_or("Failed to create pixmap")?;

    // 4. Render the SVG onto the pixmap
    resvg::render(&tree, usvg::FitTo::Original, tiny_skia::Transform::identity(),
                  &mut pixmap.as_mut());

    // 5. Save the rendered pixmap as PNG
    pixmap.save_png("output.png")?;

    println!("Successfully rendered SVG to PNG");
    Ok(())
}
```

### Working with SVG Strings

For in-memory SVG data:

```rust
use usvg::{TreeParsing, TreeTextToPath};

fn main() {
    let svg_data = r#"
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
            <circle cx="50" cy="50" r="40" stroke="black" stroke-width="3" fill="red" />
            <text x="50" y="55" font-size="10" text-anchor="middle" fill="white">Rust</text>
        </svg>
    "#;

    // Parse the SVG string into a Tree
    let mut opt = usvg::Options::default();

    // Required if your SVG contains text
    let mut fontdb = usvg::fontdb::Database::new();
    fontdb.load_system_fonts();

    let tree = usvg::Tree::from_str(svg_data, &opt, &fontdb).unwrap();

    // Determine the output size
    let rtree = resvg::Tree::from_usvg(&tree);
    let size = rtree.size.to_int_size();

    // Create a pixmap and render
    let mut pixmap = tiny_skia::Pixmap::new(size.width(), size.height()).unwrap();
    resvg::render(&rtree, tiny_skia::Transform::default(), &mut pixmap.as_mut());

    // Save the result
    pixmap.save_png("output.png").expect("Failed to save PNG");
}
```

## Advanced Usage

### Custom Rendering Options

For more advanced use cases, customize the rendering process with various options:

```rust
use resvg::{tiny_skia, usvg};
use std::path::Path;

fn render_with_custom_options() -> Result<(), Box<dyn std::error::Error>> {
    // Configure parsing options
    let mut opts = usvg::Options::default();
    opts.fontdb.load_system_fonts(); // Load system fonts
    opts.fontdb.set_generic_families(); // Set up generic font families

    // Load and parse the SVG
    let svg_data = std::fs::read("complex.svg")?;
    let tree = usvg::Tree::from_data(&svg_data, &opts)?;

    // Get the SVG size
    let size = tree.size();

    // Create a pixmap with specific dimensions (e.g., for scaling)
    let scale_factor = 2.0;
    let scaled_width = (size.width() * scale_factor) as u32;
    let scaled_height = (size.height() * scale_factor) as u32;

    let mut pixmap = tiny_skia::Pixmap::new(scaled_width, scaled_height)
        .ok_or("Failed to create pixmap")?;

    // Apply a transform for scaling
    let transform = tiny_skia::Transform::from_scale(scale_factor, scale_factor);

    // Render with FitTo::Size to control output dimensions
    resvg::render(&tree,
                  usvg::FitTo::Size(scaled_width, scaled_height),
                  transform,
                  &mut pixmap.as_mut());

    // Save the result
    pixmap.save_png("scaled_output.png")?;

    Ok(())
}
```

### Dynamic Scaling

Scaling an SVG with resvg is one of its most powerful features. Because the source is vector-based, you can render a tiny icon at 4K resolution without any pixelation.

#### Scaling to a Specific Width

```rust
// Get the original SVG size
let rtree = resvg::Tree::from_usvg(&tree);
let svg_width = rtree.size.width();
let svg_height = rtree.size.height();

// Define your target width (e.g., 3840 for 4K)
let target_width = 3840.0;
let scale = target_width / svg_width;

// Create a pixmap based on the NEW scaled dimensions
let mut pixmap = tiny_skia::Pixmap::new(
    target_width as u32,
    (svg_height * scale) as u32
).unwrap();

// Apply the scale transform during rendering
let transform = tiny_skia::Transform::from_scale(scale, scale);

resvg::render(
    &rtree,
    transform,
    &mut pixmap.as_mut()
);

pixmap.save_png("output_4k.png").unwrap();
```

When you apply a scale transform, resvg doesn't just "stretch" the resulting pixels. Instead, it recalculates the math for every path, curve, and gradient at the new resolution.

#### Common Scaling Strategies

| Strategy | Implementation Tip | Use Case |
|----------|-------------------|----------|
| **Uniform Scale** | Use `Transform::from_scale(s, s)` | Standard high-res exports |
| **Fit to Box** | Calculate `min(target_w / w, target_h / h)` | Creating thumbnails that must fit a specific UI slot |
| **DPI Scaling** | Multiply scale by `1.25` or `2.0` | Rendering for Windows/macOS high-density displays |

### Modifying SVG Content Before Rendering

There are two main approaches for modifying an SVG before rendering:

#### String Manipulation (Simple Changes)

For simple color swaps or basic modifications:

```rust
let mut svg_string = std::fs::read_to_string("icon.svg").unwrap();

// Simple color swap
let customized_svg = svg_string.replace("#0000FF", "#FFD700");

// Hide an element if you know its unique ID
let customized_svg = customized_svg.replace("id=\"secret-layer\"", "display=\"none\"");

let tree = usvg::Tree::from_str(&customized_svg, &opt, &fontdb).unwrap();
```

#### Tree Manipulation (Complex Logic)

For more complex logic, traverse and modify the `usvg` tree:

```rust
use usvg::{NodeExt, Fill, Paint, Color};

// Parse the tree as usual
let tree = usvg::Tree::from_str(svg_data, &opt, &fontdb).unwrap();

// Traverse the tree and modify nodes
for node in tree.root.descendants() {
    if let usvg::NodeKind::Path(ref mut path) = *node.borrow_mut() {
        // Change every red shape to green
        if let Some(ref mut fill) = path.fill {
            if let Paint::Color(c) = fill.paint {
                if c == Color::new_rgb(255, 0, 0) { // Red
                    fill.paint = Paint::Color(Color::new_rgb(0, 255, 0)); // Green
                }
            }
        }

        // Hide elements with a specific ID
        if node.id() == "background-layer" {
            path.visibility = usvg::Visibility::Hidden;
        }
    }
}
```

| Task | Recommended Method | Why? |
|------|-------------------|------|
| **Simple Color Swap** | `.replace()` on String | Extremely fast and low code overhead |
| **Conditional Hiding** | Tree Traversal (`node.id()`) | Precision; allows logic like "hide all circles" |
| **Dark Mode** | CSS Injection or String Replace | SVGs often use a single "theme" color you can swap globally |
| **Data Injection** | String Templates (`format!()`) | Best for changing text labels or chart values |

### Robust Error Handling

```rust
use resvg::{tiny_skia, usvg};
use std::io;

enum RenderingError {
    Io(io::Error),
    SvgParsing(String),
    Rendering(String),
    InvalidInput(String),
}

impl std::fmt::Display for RenderingError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            RenderingError::Io(e) => write!(f, "IO error: {}", e),
            RenderingError::SvgParsing(msg) => write!(f, "SVG parsing error: {}", msg),
            RenderingError::Rendering(msg) => write!(f, "Rendering error: {}", msg),
            RenderingError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
        }
    }
}

impl From<io::Error> for RenderingError {
    fn from(e: io::Error) -> Self { RenderingError::Io(e) }
}

fn render_svg_with_validation(svg_path: &str) -> Result<(), RenderingError> {
    // Validate input file exists
    let path = Path::new(svg_path);
    if !path.exists() {
        return Err(RenderingError::InvalidInput(format!("File not found: {}", svg_path)));
    }

    // Read SVG data
    let svg_data = std::fs::read(path)?;

    // Parse SVG with comprehensive error handling
    let tree = usvg::Tree::from_data(&svg_data, &usvg::Options::default())
        .map_err(|e| RenderingError::SvgParsing(e.to_string()))?;

    // Validate SVG has content
    let size = tree.size();
    if size.width() == 0.0 || size.height() == 0.0 {
        return Err(RenderingError::InvalidInput(
            "SVG has zero width or height".to_string()
        ));
    }

    // Create pixmap and render
    let mut pixmap = tiny_skia::Pixmap::new(size.width() as u32, size.height() as u32)
        .ok_or(RenderingError::Rendering("Failed to create pixmap".to_string()))?;

    resvg::render(&tree,
                  usvg::FitTo::Original,
                  tiny_skia::Transform::identity(),
                  &mut pixmap.as_mut());

    // Save with error handling
    pixmap.save_png("validated_output.png")
        .map_err(|e| RenderingError::Rendering(e.to_string()))?;

    Ok(())
}
```

## Common Use Cases

### 1. Generating Static Assets (CLI & Build Tools)

Many developers use resvg to build command-line tools that convert SVG files into raster formats like PNG:

- **Icon Generation**: Automating the creation of various icon sizes (@1x, @2x, etc.) for mobile and web apps from a single source SVG
- **Asset Pipelines**: Converting vector art into optimized PNGs during a project's build process to save on runtime CPU usage

### 2. High-Performance Web Services

Because resvg is written in Rust, it is exceptionally fast and memory-safe, making it ideal for backend services:

- **Dynamic Social Share Images**: Generating "Open Graph" images on the fly where text or data is injected into an SVG template and then flattened to a PNG
- **On-the-fly Thumbnailing**: Creating raster previews of user-uploaded SVG files for galleries or file explorers

Example web service integration with Actix-web:

```rust
use actix_web::{web, App, HttpResponse, HttpServer};
use resvg::{tiny_skia, usvg};

async fn render_svg(
    query: web::Query<RenderParams>,
) -> Result<HttpResponse, actix_web::Error> {
    // Parse query parameters
    let width = query.width.unwrap_or(800);
    let height = query.height.unwrap_or(600);

    // Load SVG
    let svg_data = std::fs::read(&query.path)
        .map_err(|e| actix_web::error::ErrorBadRequest(e))?;

    // Parse and render
    let tree = usvg::Tree::from_data(&svg_data, &usvg::Options::default())
        .map_err(|e| actix_web::error::ErrorBadRequest(e))?;

    let mut pixmap = tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| actix_web::error::ErrorInternalServerError("Failed to create pixmap"))?;

    resvg::render(&tree,
                  usvg::FitTo::Size(width, height),
                  tiny_skia::Transform::identity(),
                  &mut pixmap.as_mut());

    // Return PNG response
    let png_data = pixmap.encode_png()
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    Ok(HttpResponse::Ok()
        .content_type("image/png")
        .body(png_data))
}

#[derive(serde::Deserialize)]
struct RenderParams {
    path: String,
    width: Option<u32>,
    height: Option<u32>,
}
```

### 3. Desktop Application UI

For GUI frameworks (like **iced** or **egui**) or custom game engines, resvg acts as the rendering backend for icons and illustrations:

- **Resolution Independence**: Instead of shipping hundreds of PNGs, an app can ship small SVG files and use resvg to render them perfectly at whatever scale the user's monitor requires (High DPI/Retina support)

### 4. Report Generation and Data Visualization

While libraries like `plotters` handle the logic of charts, resvg is often the final step in the chain:

- **PDF/Export Features**: Taking a complex data visualization generated as an SVG and converting it into a high-resolution image for inclusion in a PDF report or presentation slide

### 5. Game Development

In game engines, resvg can be used to:

- **Texture Baking**: Rendering vector UI elements or character skins into textures at runtime
- **Map Rendering**: Converting SVG-based map data into tiles that the game engine can display

## Text and Font Handling

### The Font Challenge

One of the most common issues when using resvg is font rendering, especially when SVG documents reference fonts that aren't available on the target system.

**Key Issue**: resvg uses its own font database rather than system font services, which can lead to different font fallback behavior compared to browsers. Custom fonts must be explicitly loaded into the font database.

### Setting Up Font Database

```rust
use resvg::usvg;
use std::path::Path;

fn setup_font_database() -> usvg::FontDB {
    let mut fontdb = usvg::FontDB::new();

    // Load system fonts
    fontdb.load_system_fonts();

    // Load custom fonts from specific directories
    let custom_fonts_dir = Path::new("/path/to/custom/fonts");
    if custom_fonts_dir.exists() {
        fontdb.load_fonts_dir(custom_fonts_dir);
    }

    // Load specific font files
    fontdb.load_font_file("/path/to/CustomFont.ttf").ok();

    // Set up generic font families
    fontdb.set_generic_families();

    fontdb
}

// Use in rendering options
let opts = usvg::Options {
    fontdb: setup_font_database(),
    ..Default::default()
};
```

### Embedding Fonts in Your Application

For serverless environments or containers without fonts, embed fonts directly:

```rust
use std::fs;

// Load font from memory (useful for embedded fonts)
fn load_font_from_memory() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let font_data = fs::read("/path/to/CustomFont.ttf")?;
    Ok(font_data)
}

// Add to font database
let font_data = load_font_from_memory()?;
fontdb.load_font_data(font_data);

// Or embed at compile time
fontdb.load_font_data(include_bytes!("../assets/Roboto-Regular.ttf").to_vec());
```

### Text-to-Path Conversion

To ensure text renders identically on every machine regardless of installed fonts, use **text-to-path conversion**. This turns text characters into literal vector shapes (paths).

```rust
use usvg::{TreeParsing, TreeTextToPath};

fn main() {
    let svg_data = r#"<svg><text x="10" y="50" font-family="MyCustomFont">Hello</text></svg>"#;

    let mut opt = usvg::Options::default();
    let mut fontdb = usvg::fontdb::Database::new();

    // Load fonts so usvg can find the glyphs to convert
    fontdb.load_system_fonts();
    fontdb.load_font_file("assets/MyCustomFont.ttf").expect("Font not found");

    // usvg automatically converts <text> to <path> during parsing
    // based on the fonts found in fontdb
    let tree = usvg::Tree::from_str(svg_data, &opt, &fontdb).unwrap();

    // Now 'tree' contains no text elements, only paths!
}
```

#### Why Text-to-Path is Essential

resvg is a **static** renderer without a dynamic layout engine. It relies on usvg to:

1. **Font Matching**: Finding the closest match in the `fontdb`
2. **Shaping**: Using `rustybuzz` internally to decide exactly where each glyph goes
3. **Outlining**: Converting glyphs into vector paths

#### Pros and Cons of Text-to-Path

| Feature | Text-to-Path (Converted) | Native Text (Raw SVG) |
|---------|-------------------------|----------------------|
| **Portability** | Perfect. Looks the same everywhere | Risky. Depends on system fonts |
| **File Size** | Larger (every letter is a complex path) | Smaller (just a string of characters) |
| **Editability** | None. You can't change the spelling | Easy to edit in a text editor |
| **Performance** | Faster to render (it's just a path) | Slower (requires font lookup/shaping) |

### Text Rendering Limitations

**Limitation 1**: No native text shaping - resvg doesn't use platform text shaping engines, which can affect complex scripts (Arabic, Indic) and advanced typography features (ligatures, kerning).

**Workaround**: Convert text to paths during preprocessing:

```rust
let mut opts = usvg::Options::default();
opts.fontdb.load_system_fonts();
opts.text_rendering_mode = usvg::TextRenderingMode::OptimizeLegibility;

let tree = usvg::Tree::from_data(&svg_data, &opts)?;
```

**Limitation 2**: Text selection and measurement - Unlike browsers, resvg doesn't provide APIs for text selection or precise text measurement after rendering.

**Workaround**: Use usvg's tree parsing to access text metrics before rendering:

```rust
for node in tree.root().descendants() {
    if let usvg::Node::Text(text) = node {
        let bbox = text.calculate_bbox();
        let font = text.font;
        let font_size = text.font_size;
        println!("Text found: {:?} at {:?}", text.chunks, bbox);
    }
}
```

## Performance Optimization

### Caching Parsed SVGs

For repeated rendering of the same SVG, parse once and render multiple times:

```rust
use std::collections::HashMap;

struct SvgCache {
    cache: HashMap<String, usvg::Tree>,
}

impl SvgCache {
    fn new() -> Self { Self { cache: HashMap::new() } }

    fn get_or_parse(&mut self, svg_path: &str) -> Result<&usvg::Tree, Box<dyn std::error::Error>> {
        if !self.cache.contains_key(svg_path) {
            let svg_data = std::fs::read(svg_path)?;
            let tree = usvg::Tree::from_data(&svg_data, &usvg::Options::default())?;
            self.cache.insert(svg_path.to_string(), tree);
        }
        Ok(self.cache.get(svg_path).unwrap())
    }
}
```

### Memory Optimization

For memory-constrained environments, optimize rendering by:

- **Reusing pixmaps**: Instead of creating new pixmaps for each render, reuse existing ones
- **Appropriate dimensions**: Use `FitTo::Width` or `FitTo::Height` to render at specific dimensions rather than full resolution
- **Font database optimization**: Only load necessary fonts instead of all system fonts

```rust
fn render_memory_optimized(svg_data: &[u8], width: u32) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let tree = usvg::Tree::from_data(svg_data, &usvg::Options::default())?;

    // Calculate appropriate height maintaining aspect ratio
    let aspect_ratio = tree.size().height() / tree.size().width();
    let height = (width as f32 * aspect_ratio) as u32;

    // Reuse pixmap if possible (in real scenario, keep in struct)
    let mut pixmap = tiny_skia::Pixmap::new(width, height)
        .ok_or("Failed to create pixmap")?;

    // Render with specific width
    resvg::render(&tree,
                  usvg::FitTo::Width(width),
                  tiny_skia::Transform::identity(),
                  &mut pixmap.as_mut());

    Ok(pixmap.encode_png()?)
}
```

### Performance Optimization Strategies

```
Rendering Performance
├── Parsing Optimization
│   ├── Caching parsed trees
│   ├── Preprocessing SVGs
│   └── Using usvg directly
├── Rendering Optimization
│   ├── Appropriate FitTo modes
│   ├── Transform optimization
│   └── Target surface configuration
└── Memory Optimization
    ├── Reusing pixmaps
    ├── Memory pool allocation
    └── Font database optimization
```

## Gotchas and Workarounds

### SVG Compatibility Issues

**Gotcha 1**: Unsupported SVG features - resvg explicitly excludes dynamic SVG features:
- Animations (SMIL, CSS animations)
- Scripting (JavaScript)
- Interactivity (event handlers, cursor elements)
- Audio and video elements

**Workaround**: Preprocess SVGs to remove or simplify unsupported features:

```rust
fn preprocess_svg(svg_data: &[u8]) -> Result<String, Box<dyn std::error::Error>> {
    let svg_string = String::from_utf8(svg_data.to_vec())?;

    // Remove script tags
    let no_scripts = regex::Regex::new(r#"<script.*?>.*?</script>"#)?;
    let cleaned = no_scripts.replace_all(&svg_string, "");

    // Remove animation elements
    let no_anims = regex::Regex::new(r#"<(animate|animateTransform|set).*?/>"#)?;
    let cleaned = no_anims.replace_all(&cleaned, "");

    Ok(cleaned.to_string())
}
```

**Gotcha 2**: CSS complexity - resvg has limited support for advanced CSS features (flexbox, grid, custom properties).

**Workaround**: Preprocess CSS to simplify complex selectors and properties.

### Resource Loading and Security

**Gotcha**: By default, resvg only loads external resources that are in the same directory as the base SVG file or subdirectories. This prevents malicious SVGs from accessing files elsewhere on the system.

**Workaround**: For trusted content, you can relax these restrictions:

```rust
let mut opts = usvg::Options::default();
opts.fontdb.load_system_fonts();

// Allow loading resources from parent directories (use with caution!)
opts.resources_dir = std::path::Path::new("/trusted/path").to_path_buf();
```

For environments requiring strict security (e.g., processing user-uploaded SVGs), maintain the default restrictions and preprocess SVGs to embed external resources:

```rust
fn embed_external_images(svg: &str, base_path: &Path) -> Result<String, Box<dyn std::error::Error>> {
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

## Comparison with Alternatives

### Feature Comparison

| Feature | resvg | librsvg | Cairo-SVG | Lunasvg |
|---------|-------|---------|-----------|---------|
| **Language** | Pure Rust | C | C++ | C++ |
| **Memory Safety** | High | Medium | Low | Low |
| **SVG 1.1 Support** | Extensive | Extensive | Moderate | Basic |
| **SVG 2 Support** | In progress | In progress | No | No |
| **Animation Support** | No | Limited | No | No |
| **Text Rendering** | Custom | Native | Native | Custom |
| **Font Handling** | Custom DB | Native | Native | Basic |
| **Cross-Platform** | Excellent | Good | Good | Moderate |
| **Performance** | Very Good | Good | Good | Excellent |
| **Binary Size** | Small | Large | Medium | Small |
| **WASM Support** | Excellent | No | No | Limited |

### When to Choose resvg

resvg is particularly well-suited for:

- **Rust-native applications**: Where avoiding C dependencies and maintaining memory safety are priorities
- **Cross-platform consistency**: When identical rendering results across different platforms is required
- **Serverless environments**: Due to its small binary size and minimal dependencies
- **Security-sensitive contexts**: Processing untrusted SVGs with reduced risk of memory vulnerabilities
- **Custom rendering pipelines**: Where the modular usvg/resvg separation can be leveraged

### Consider Alternatives When

- **Animation support is required**: resvg explicitly doesn't support SMIL or CSS animations
- **Native text rendering is needed**: For applications requiring exact text matching with system browsers
- **Existing C/C++ infrastructure**: When integrating with existing graphics stacks based on Cairo or Skia
- **Maximum rendering speed**: While fast, resvg may not be the absolute fastest for all SVG subsets

## Ecosystem Integration

### Linebender Ecosystem

resvg is a member of the [Linebender](https://linebender.org/) ecosystem of crates which address graphics and text needs. The strength of this ecosystem lies in its **modular design** and **comprehensive coverage** of graphics-related functionality.

### Typical Workflow

To use resvg, you typically follow this flow:

1. **Parse**: Use the `usvg` crate to simplify the SVG into a "flat" tree
2. **Render**: Use `resvg` to draw that tree onto a pixel buffer (provided by `tiny-skia`)
3. **Encode**: Save that buffer as a PNG or display it in a window

## Quick Reference

### Coordinate Systems

`tiny_skia::Transform` allows you to scale or rotate the entire SVG during the rendering process without losing quality.

### Essential Reminders

- **Fonts**: SVG text requires a font database. If text isn't showing up, ensure you've called `fontdb.load_system_fonts()`
- **Never rely on system fonts**: For production services, always bundle `.ttf` or `.otf` files in your project's `assets/` folder
- **Security**: Don't process untrusted SVGs without proper validation and resource restrictions

### Common Patterns

```rust
// Load from file
let svg_data = std::fs::read("file.svg")?;
let tree = usvg::Tree::from_data(&svg_data, &usvg::Options::default())?;

// Load from string
let tree = usvg::Tree::from_str(svg_string, &opt, &fontdb)?;

// Render at original size
resvg::render(&tree, usvg::FitTo::Original,
              tiny_skia::Transform::identity(), &mut pixmap.as_mut());

// Render scaled
let transform = tiny_skia::Transform::from_scale(2.0, 2.0);
resvg::render(&tree, usvg::FitTo::Size(width, height),
              transform, &mut pixmap.as_mut());

// Save as PNG
pixmap.save_png("output.png")?;
```

## Conclusion

The **resvg** crate provides a powerful, safe, and efficient solution for SVG rendering in Rust. Its pure-Rust implementation, comprehensive test coverage, and focus on reproducibility make it an excellent choice for a wide range of applications.

The library's modular design and separation between parsing (usvg) and rendering (resvg) offer flexibility for advanced use cases, while the straightforward API makes it accessible for simple rendering tasks. As the library continues to evolve with improving SVG 2 support and performance optimizations, it is likely to become even more capable.

For projects requiring cross-platform consistency, memory safety, and comprehensive SVG support, resvg represents one of the best options in the Rust ecosystem. By understanding its architecture, potential gotchas, and optimization strategies, developers can effectively leverage this library to add robust SVG rendering capabilities to their applications.
