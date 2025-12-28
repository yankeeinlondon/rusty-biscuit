# Font Handling

Font rendering in resvg requires explicit configuration and understanding of the text-to-path conversion process.

## Core Concept: Font Database

resvg uses its own font database rather than system font services. This provides:

- **Cross-platform consistency**: Same rendering on all systems
- **Portability**: Works in environments without system fonts (Docker, Lambda)
- **Security**: No dependency on system font configuration

## Loading Fonts

### System Fonts

```rust
use usvg::fontdb::Database;

let mut fontdb = Database::new();
fontdb.load_system_fonts();
fontdb.set_generic_families(); // Set up serif, sans-serif, monospace
```

### Custom Font Files

```rust
// Load specific font file
fontdb.load_font_file("/path/to/CustomFont.ttf")
    .expect("Font not found");

// Load all fonts from directory
fontdb.load_fonts_dir("/path/to/fonts/directory");
```

### Embedded Fonts (Best for Serverless)

```rust
// Embed font at compile time
let font_data = include_bytes!("../assets/Roboto-Regular.ttf");
fontdb.load_font_data(font_data.to_vec());
```

This ensures your Rust binary is completely self-contained and produces pixel-perfect results in any environment.

## Text-to-Path Conversion

resvg automatically converts text to vector paths during parsing. This:

- **Eliminates font dependencies** at render time
- **Ensures portability** - looks identical everywhere
- **Increases file size** - every letter becomes a path
- **Removes editability** - can't change text after conversion

### How It Works

```rust
use usvg::{TreeParsing, TreeTextToPath};

let svg_data = r#"<svg><text x="10" y="50" font-family="Arial">Hello</text></svg>"#;

let mut opt = usvg::Options::default();
let mut fontdb = usvg::fontdb::Database::new();
fontdb.load_system_fonts();

// usvg automatically converts <text> to <path> during parsing
// based on fonts found in fontdb
let tree = usvg::Tree::from_str(svg_data, &opt, &fontdb)?;

// Now 'tree' contains no text elements, only paths!
```

The conversion process:

1. **Font Matching**: Finding closest match in fontdb
2. **Shaping**: Using rustybuzz internally to position glyphs
3. **Outlining**: Converting glyphs into vector paths

## Font Database Setup Pattern

```rust
fn setup_font_database() -> usvg::fontdb::Database {
    let mut fontdb = usvg::fontdb::Database::new();

    // Load system fonts
    fontdb.load_system_fonts();

    // Load custom fonts from specific directories
    let custom_fonts_dir = std::path::Path::new("/path/to/custom/fonts");
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

## Text-to-Path Trade-offs

| Feature | Text-to-Path (Converted) | Native Text (Raw SVG) |
|---------|-------------------------|----------------------|
| **Portability** | Perfect - looks same everywhere | Risky - depends on system fonts |
| **File Size** | Larger (every letter is complex path) | Smaller (just character strings) |
| **Editability** | None - can't change spelling | Easy to edit in text editor |
| **Performance** | Faster to render (just paths) | Slower (requires font lookup/shaping) |

## Common Font Issues

### Missing Custom Fonts

**Problem**: SVG references font not available on system

**Solution**: Load font explicitly into fontdb before parsing:

```rust
// For Docker/serverless: embed font
let font_data = include_bytes!("../assets/CustomFont.ttf");
fontdb.load_font_data(font_data.to_vec());

// For local development: load from file
fontdb.load_font_file("assets/CustomFont.ttf")?;
```

### Text Not Appearing

**Problem**: Text renders as blank or missing

**Cause**: Font database not initialized

**Solution**: Always call `fontdb.load_system_fonts()` when SVG contains text:

```rust
let mut fontdb = usvg::fontdb::Database::new();
fontdb.load_system_fonts(); // Required for text rendering!
```

### Font Fallback Behavior

**Problem**: Different fallback than browsers

**Cause**: resvg uses its own font matching algorithm

**Solution**: Explicitly specify font families in SVG with good fallback chain:

```xml
<text font-family="Roboto, Arial, Helvetica, sans-serif">
```

## Text Rendering Modes

Configure text rendering quality:

```rust
let mut opts = usvg::Options::default();
opts.text_rendering_mode = usvg::TextRenderingMode::OptimizeLegibility;
```

Options:
- `OptimizeSpeed` - Faster, lower quality
- `OptimizeLegibility` - Better quality, slower (default)
- `GeometricPrecision` - Highest quality, slowest
