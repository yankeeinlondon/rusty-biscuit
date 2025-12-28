The `resvg` crate is a high-performance SVG rendering library for Rust. Because `resvg` focuses strictly on the conversion of SVG data into pixel buffers, it is almost always paired with other crates to handle parsing, memory allocation, and file output.

Here are the three most common libraries integrated with `resvg`.

---

### 1. `usvg`

**Relationship:** The Pre-processor / Parser.

**How and Why:**
`resvg` does not actually parse raw SVG XML strings directly. Instead, it relies on `usvg` to "simplify" the SVG. `usvg` takes a complex SVG (with CSS, nested transforms, and inherited attributes) and converts it into a simplified Tree structure that `resvg` can understand. You cannot use `resvg` without `usvg` if you are starting from an SVG file or string.

**Code Example:**

````rust
use usvg::{Tree, Options};
use resvg::render;
use tiny_skia::Pixmap;

fn main() {
    let svg_data = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
                        <circle cx="50" cy="50" r="40" fill="red" />
                      </svg>"#;

    // 1. Use usvg to parse and simplify the SVG
    let tree = Tree::from_str(svg_data, &Options::default()).unwrap();

    // 2. Prepare a pixmap (buffer) to render into
    let mut pixmap = Pixmap::new(100, 100).unwrap();

    // 3. resvg uses the usvg tree to render
    resvg::render(&tree, usvg::Transform::default(), &mut pixmap.as_mut());
}
````

---

### 2. `tiny-skia`

**Relationship:** The Rendering Engine / Surface.

**How and Why:**
`resvg` is a "frontend" that knows *what* to draw, but it needs a "backend" to actually manipulate pixels. `tiny-skia` is the default 2D graphics library used for this. It provides the `Pixmap` (a pixel buffer) and the drawing primitives. When you use `resvg`, you use `tiny-skia` to allocate the memory where the image will be drawn and to handle the final pixel layout (usually RGBA).

**Code Example:**

````rust
use tiny_skia::{Pixmap, Transform};
// resvg depends on tiny-skia types for its render function targets

fn render_to_buffer(tree: &usvg::Tree) -> Vec<u8> {
    // Create a tiny-skia Pixmap
    let mut pixmap = Pixmap::new(500, 500).expect("Could not allocate memory");

    // Render the resvg tree onto the tiny-skia pixmap
    resvg::render(
        tree,
        Transform::identity(), // tiny-skia's transform type
        &mut pixmap.as_mut()
    );

    // Return the raw underlying bytes (RGBA)
    pixmap.data().to_vec()
}
````

---

### 3. `image` (or `png`)

**Relationship:** The File IO / Encoder.

**How and Why:**
Once `resvg` and `tiny-skia` have produced a buffer of raw pixels in memory, you often need to save that image to a file (like a `.png` or `.jpg`) or convert it to another format. The `image` crate is the standard Rust tool for this. While `tiny-skia` has a basic `.save_png()` method, the `image` crate is used for more complex tasks like resizing the output, adjusting color profiles, or encoding into formats like WebP or BMP.

**Code Example:**

````rust
use image::{RgbaImage, ImageFormat};
use tiny_skia::Pixmap;

fn save_svg_as_webp(pixmap: Pixmap, path: &str) {
    // 1. Convert tiny-skia Pixmap (raw bytes) into an 'image' crate RgbaImage
    let width = pixmap.width();
    let height = pixmap.height();
    let raw_bytes = pixmap.data().to_vec();

    let img = RgbaImage::from_raw(width, height, raw_bytes)
        .expect("Failed to create image container");

    // 2. Use the 'image' crate to save as WebP (something tiny-skia can't do alone)
    img.save_with_format(path, ImageFormat::WebP).unwrap();
}
````

---

### Summary Table

|Library|Role|Why use it with `resvg`?|
|:------|:---|:-----------------------|
|**`usvg`**|Parsing|To convert SVG XML strings into a renderable data tree.|
|**`tiny-skia`**|Surface|To provide the memory buffer and 2D drawing backend for the pixels.|
|**`image`**|Export|To encode the raw pixel output into standard files (PNG, JPEG, WebP).|