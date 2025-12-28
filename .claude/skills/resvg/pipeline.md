# Rendering Pipeline Architecture

The resvg rendering pipeline follows a two-stage process that separates SVG parsing from rasterization.

## Pipeline Flow

```
SVG Input (File/Bytes/String)
    ↓
usvg Parser
    ↓
SVG Tree (Parsed representation)
    ↓
resvg Renderer
    ↓
Output (Pixmap/PNG)
```

## Stage 1: SVG Parsing with usvg

The **usvg** crate (Universal SVG) handles initial parsing and normalization:

- **XML Parsing**: Converts raw SVG XML into structured tree representation
- **CSS Resolution**: Processes stylesheets and resolves CSS properties
- **Path Flattening**: Converts complex path data into simplified forms
- **Font Resolution**: Maps font families to available system fonts
- **Unit Conversion**: Normalizes various units (pixels, percentages, points) to consistent format

The parsing process produces a `usvg::Tree` that serves as input to rendering stage.

## Stage 2: Rendering with resvg

resvg uses **tiny-skia** as its rendering backend, providing a software rasterizer that is both fast and precise. This approach ensures resvg doesn't depend on platform-specific graphics libraries like Cairo or Skia.

### Supported Features

- **Path Rendering**: Complex shapes, bezier curves, geometric primitives
- **Text Rendering**: Text layout, font selection, text metrics
- **Filter Effects**: Blurring, lighting effects, compositing operations
- **Gradient and Pattern Fills**: Linear and radial gradients, repeating patterns
- **Clipping and Masking**: Complex clipping paths and opacity masks
- **Image Embedding**: Raster images embedded within SVG documents
- **Transformations**: Translation, rotation, scaling, skewing

## Architecture Benefits

### Pre-parsing Optimization

SVG documents can be parsed once and rendered multiple times with different parameters or to different surfaces without reparsing.

```rust
// Parse once
let tree = usvg::Tree::from_data(&svg_data, &usvg::Options::default())?;

// Render multiple times with different scales
for scale in [1.0, 2.0, 3.0] {
    let mut pixmap = tiny_skia::Pixmap::new(
        (tree.size().width() * scale) as u32,
        (tree.size().height() * scale) as u32
    )?;

    let transform = tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, usvg::FitTo::Original, transform, &mut pixmap.as_mut());

    pixmap.save_png(&format!("output_{}x.png", scale))?;
}
```

### Multiple Rendering Backends

Developers can implement custom renderers using the `usvg` tree representation.

### Efficient Caching

Parsed SVG trees can be cached in memory for repeated rendering operations (see [Performance Optimization](./performance.md)).

## FitTo Options

Control how the SVG scales to fit the pixmap:

- `FitTo::Original` - Render at original SVG dimensions
- `FitTo::Width(u32)` - Fit to specific width, maintain aspect ratio
- `FitTo::Height(u32)` - Fit to specific height, maintain aspect ratio
- `FitTo::Size(u32, u32)` - Fit to specific width and height

## Transforms

Apply transformations during rendering:

```rust
// Scale
let transform = tiny_skia::Transform::from_scale(2.0, 2.0);

// Rotate (angle in radians)
let transform = tiny_skia::Transform::from_rotate(std::f32::consts::PI / 4.0);

// Translate
let transform = tiny_skia::Transform::from_translate(10.0, 20.0);

// Combine transforms
let transform = tiny_skia::Transform::from_scale(2.0, 2.0)
    .post_translate(10.0, 20.0);
```
