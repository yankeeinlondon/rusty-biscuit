# usvg Usage Patterns

### 1. Handling Text and Fonts

`usvg` will not render text if fonts are not explicitly loaded into the `fontdb`.

````rust
let mut fontdb = usvg::fontdb::Database::new();
fontdb.load_system_fonts(); // Load OS fonts
fontdb.load_font_file("assets/custom.ttf")?; // Load specific assets

let opts = usvg::Options {
    fontdb: std::sync::Arc::new(fontdb),
    ..Default::default()
};
````

### 2. Geometry Extraction (CNC / Plotters)

Because `usvg` converts all shapes to paths and applies transforms, extracting coordinates is straightforward.

````rust
for node in tree.root().descendants() {
    if let usvg::Node::Path(path) = node {
        // 'path' contains segments: MoveTo, LineTo, CurveTo
        for segment in path.data().segments() {
            match segment {
                usvg::PathSegment::MoveTo { x, y } => /* ... */,
                usvg::PathSegment::LineTo { x, y } => /* ... */,
                usvg::PathSegment::CurveTo { x1, y1, x2, y2, x, y } => /* ... */,
                usvg::PathSegment::ClosePath => /* ... */,
            }
        }
    }
}
````

### 3. Modifying Options

* **DPI**: Use `opts.dpi` to control how absolute units (cm, pt) convert to pixels (default is 96.0).
* **Resources**: Use `opts.resources_dir` to resolve relative paths for `<image>` tags.