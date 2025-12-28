# Integration with Other Libraries

`usvg` creates the data structure; other libraries draw it.

### 1. resvg (Standard Rasterizer)

The most common pairing for converting SVG to PNG.

````rust
// Requires: usvg, resvg, tiny-skia
let tree = usvg::Tree::from_data(&data, &opts)?;
let mut pixmap = tiny_skia::Pixmap::new(width, height).unwrap();

resvg::render(
    &tree,
    usvg::Transform::default(),
    &mut pixmap.as_mut()
);

pixmap.save_png("output.png")?;
````

### 2. tiny-skia (2D Backend)

Used for the underlying pixel buffer. You can use `tiny-skia` to draw backgrounds before rendering the SVG on top.

### 3. Vello (GPU Rendering)

For high-performance GPU-bound rendering.

````rust
// Use vello_svg to bridge usvg and vello
let mut scene = vello::Scene::new();
let mut builder = vello::SceneBuilder::for_scene(&mut scene);
vello_svg::append_path_tree(&mut builder, &tree);
````

### Summary Table

|Library|Role|Why use with usvg?|
|:------|:---|:-----------------|
|**resvg**|Renderer|Standard CPU-based SVG to Pixels.|
|**tiny-skia**|2D Engine|Low-level drawing primitives and pixel buffers.|
|**vello**|GPU Renderer|Hardware-accelerated rendering via wgpu.|
|**fontdb**|Font Manager|Required for text rendering.|