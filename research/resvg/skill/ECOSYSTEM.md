# Ecosystem Integration

`resvg` is rarely used in isolation. It relies on these "Partner Crates":

|Library|Role|Usage|
|:------|:---|:----|
|**`usvg`**|Parser|Converts SVG XML into a simplified data tree. Required for input.|
|**`tiny-skia`**|Backend|Provides the `Pixmap` (pixel buffer) and math types (`Transform`, `Rect`).|
|**`fontdb`**|Font Mgr|Stores and queries fonts. Required for text rendering.|
|**`image`**|Export|Used to convert raw `Pixmap` bytes into WebP, JPEG, or BMP.|

### Saving as WebP (via `image` crate)

`tiny-skia` only saves to PNG. For other formats:

````rust
use image::{RgbaImage, ImageFormat};

fn save_as_webp(pixmap: tiny_skia::Pixmap, path: &str) {
    let img = RgbaImage::from_raw(pixmap.width(), pixmap.height(), pixmap.data().to_vec())
        .expect("Buffer mismatch");
    img.save_with_format(path, ImageFormat::WebP).unwrap();
}
````