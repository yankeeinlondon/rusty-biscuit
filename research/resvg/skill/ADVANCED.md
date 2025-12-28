# Advanced Usage

### Scaling and Fitting

By default, `resvg` renders 1:1. To fit an SVG into a specific box (e.g., 512x512) while maintaining aspect ratio:

````rust
let target_size = tiny_skia::IntSize::new(512, 512).unwrap();
let fit_to = usvg::FitTo::Size(512, 512);

// resvg::render_node is a helper that calculates scaling and returns a Pixmap
if let Some(pixmap) = resvg::render_node(
    &tree.root(), 
    &fit_to, 
    tiny_skia::Transform::default(), 
    target_size
) {
    pixmap.save_png("scaled.png")?;
}
````

### High DPI Rendering (for PDF/Print)

Standard SVG is 72 DPI. For high-quality prints (300 DPI), apply a scale factor of `4.166` (300 / 72).

````rust
let zoom = 4.166;
let size = tree.size().to_int_size().scale_by(zoom).unwrap();
let mut pixmap = tiny_skia::Pixmap::new(size.width(), size.height()).unwrap();

let transform = tiny_skia::Transform::from_scale(zoom, zoom);
resvg::render(&tree, transform, &mut pixmap.as_mut());
````

### Coordinate Mapping

If you need to place an SVG at a specific coordinate inside a larger buffer:

````rust
let transform = tiny_skia::Transform::from_translate(100.0, 50.0);
resvg::render(&tree, transform, &mut pixmap.as_mut());
````
