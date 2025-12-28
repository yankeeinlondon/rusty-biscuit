# Common Use Cases

resvg excels in several application scenarios where static SVG rendering is required.

## 1. Icon and Asset Generation

Converting SVG icons to raster formats at various resolutions.

**Use case**: Build tools that generate @1x, @2x, @3x icons from single source SVG

```rust
fn generate_icon_set(svg_path: &str, sizes: &[u32])
    -> Result<(), Box<dyn std::error::Error>> {
    let svg_data = std::fs::read(svg_path)?;
    let tree = usvg::Tree::from_data(&svg_data, &usvg::Options::default())?;

    for size in sizes {
        let mut pixmap = tiny_skia::Pixmap::new(*size, *size)?;

        resvg::render(&tree, usvg::FitTo::Size(*size, *size),
                      tiny_skia::Transform::identity(), &mut pixmap.as_mut());

        pixmap.save_png(&format!("icon_{}x{}.png", size, size))?;
    }

    Ok(())
}

// Generate 1x, 2x, 3x icons
generate_icon_set("icon.svg", &[32, 64, 96])?;
```

## 2. Web Service Backends

Rendering SVGs server-side for API responses or thumbnail generation.

**Use case**: Dynamic social share images (Open Graph)

```rust
async fn generate_og_image(
    title: &str,
    author: &str
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // Load SVG template
    let mut svg_template = std::fs::read_to_string("og-template.svg")?;

    // Inject dynamic data
    svg_template = svg_template
        .replace("{{TITLE}}", title)
        .replace("{{AUTHOR}}", author);

    // Render to PNG
    let tree = usvg::Tree::from_data(
        svg_template.as_bytes(),
        &usvg::Options::default()
    )?;

    let mut pixmap = tiny_skia::Pixmap::new(1200, 630)?; // OG image size

    resvg::render(&tree, usvg::FitTo::Size(1200, 630),
                  tiny_skia::Transform::identity(), &mut pixmap.as_mut());

    Ok(pixmap.encode_png()?)
}
```

## 3. Desktop Application UI

Resolution-independent rendering for high-DPI displays.

**Use case**: GUI frameworks (iced, egui) rendering SVG icons

```rust
struct IconCache {
    cache: HashMap<String, tiny_skia::Pixmap>,
}

impl IconCache {
    fn get_icon(&mut self, icon_name: &str, dpi_scale: f32)
        -> Result<&tiny_skia::Pixmap, Box<dyn std::error::Error>> {
        let cache_key = format!("{}_{}", icon_name, dpi_scale);

        if !self.cache.contains_key(&cache_key) {
            let svg_data = std::fs::read(&format!("icons/{}.svg", icon_name))?;
            let tree = usvg::Tree::from_data(&svg_data, &usvg::Options::default())?;

            let base_size = 24.0; // Base icon size
            let scaled_size = (base_size * dpi_scale) as u32;

            let mut pixmap = tiny_skia::Pixmap::new(scaled_size, scaled_size)?;

            let transform = tiny_skia::Transform::from_scale(dpi_scale, dpi_scale);
            resvg::render(&tree, usvg::FitTo::Original, transform, &mut pixmap.as_mut());

            self.cache.insert(cache_key.clone(), pixmap);
        }

        Ok(self.cache.get(&cache_key).unwrap())
    }
}
```

## 4. Document Generation

Embedding SVG graphics in generated PDFs or documents.

**Use case**: Report generation with data visualizations

```rust
fn embed_chart_in_pdf(svg_path: &str, pdf_builder: &mut PdfBuilder)
    -> Result<(), Box<dyn std::error::Error>> {
    let svg_data = std::fs::read(svg_path)?;
    let tree = usvg::Tree::from_data(&svg_data, &usvg::Options::default())?;

    // Render at print quality (300 DPI)
    let scale = 300.0 / 96.0; // 96 DPI is typical screen DPI
    let width = (tree.size().width() * scale) as u32;
    let height = (tree.size().height() * scale) as u32;

    let mut pixmap = tiny_skia::Pixmap::new(width, height)?;

    let transform = tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, usvg::FitTo::Original, transform, &mut pixmap.as_mut());

    // Add to PDF
    pdf_builder.add_image(pixmap.encode_png()?)?;

    Ok(())
}
```

## 5. Game Development

Converting vector assets to game-ready textures.

**Use case**: Texture baking for UI elements

```rust
fn bake_ui_textures(asset_dir: &str, output_dir: &str)
    -> Result<(), Box<dyn std::error::Error>> {
    use std::path::Path;

    for entry in std::fs::read_dir(asset_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension() == Some(std::ffi::OsStr::new("svg")) {
            let svg_data = std::fs::read(&path)?;
            let tree = usvg::Tree::from_data(&svg_data, &usvg::Options::default())?;

            // Render at power-of-2 dimensions for GPU
            let mut pixmap = tiny_skia::Pixmap::new(512, 512)?;

            resvg::render(&tree, usvg::FitTo::Size(512, 512),
                          tiny_skia::Transform::identity(), &mut pixmap.as_mut());

            let output_path = Path::new(output_dir)
                .join(path.file_stem().unwrap())
                .with_extension("png");

            pixmap.save_png(&output_path)?;
        }
    }

    Ok(())
}
```

## 6. Thumbnail Generation

Creating raster previews of user-uploaded SVG files.

**Use case**: File explorers, galleries

```rust
fn create_thumbnail(svg_path: &str, max_dimension: u32)
    -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let svg_data = std::fs::read(svg_path)?;
    let tree = usvg::Tree::from_data(&svg_data, &usvg::Options::default())?;

    let size = tree.size();
    let aspect_ratio = size.height() / size.width();

    // Calculate dimensions maintaining aspect ratio
    let (width, height) = if size.width() > size.height() {
        (max_dimension, (max_dimension as f32 * aspect_ratio) as u32)
    } else {
        ((max_dimension as f32 / aspect_ratio) as u32, max_dimension)
    };

    let mut pixmap = tiny_skia::Pixmap::new(width, height)?;

    resvg::render(&tree, usvg::FitTo::Size(width, height),
                  tiny_skia::Transform::identity(), &mut pixmap.as_mut());

    Ok(pixmap.encode_png()?)
}
```

## 7. CLI Tools

Command-line SVG converters.

**Use case**: Batch SVG-to-PNG conversion

```rust
use clap::Parser;

#[derive(Parser)]
struct Args {
    #[arg(short, long)]
    input: String,

    #[arg(short, long)]
    output: String,

    #[arg(short, long, default_value = "1.0")]
    scale: f32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let svg_data = std::fs::read(&args.input)?;
    let tree = usvg::Tree::from_data(&svg_data, &usvg::Options::default())?;

    let size = tree.size();
    let width = (size.width() * args.scale) as u32;
    let height = (size.height() * args.scale) as u32;

    let mut pixmap = tiny_skia::Pixmap::new(width, height)?;

    let transform = tiny_skia::Transform::from_scale(args.scale, args.scale);
    resvg::render(&tree, usvg::FitTo::Original, transform, &mut pixmap.as_mut());

    pixmap.save_png(&args.output)?;

    println!("Converted {} to {}", args.input, args.output);
    Ok(())
}
```

## Use Case Summary

| Use Case | Key Benefit | Common Pattern |
|----------|-------------|----------------|
| Icon Generation | Resolution independence | Multi-size batch rendering |
| Web Services | Cross-platform consistency | Dynamic data injection |
| Desktop Apps | High-DPI support | Icon caching with DPI scaling |
| Document Generation | Print quality | High DPI scaling (300+) |
| Game Development | GPU-friendly formats | Power-of-2 dimensions |
| Thumbnails | Fast previews | Aspect ratio preservation |
| CLI Tools | Automation | Batch processing |
