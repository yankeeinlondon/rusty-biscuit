# Sizing, terminal cells, and layout

## Terminal cells vs pixels (the #1 confusion)

* `Config.width` and `Config.height` are **terminal cell dimensions**, not pixels.
* With half-block rendering, viuer effectively uses **two vertical pixels per cell** (upper/lower halves), so images can look “half-height” if you were thinking in pixels.

Example: `height: Some(25)` may correspond to ~50 vertical pixels in half-block mode.

## Fit image to terminal

If you want to scale the image to your current terminal:

````rust
use viuer::{terminal_size, resize, print, Config};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let img = image::open("large_image.jpg")?;

    let (cols, rows) = terminal_size();
    // rows * 2 because of 2-pixels-per-cell vertical mapping in fallback rendering
    let resized = resize(&img, cols as u32, (rows * 2) as u32);

    print(&resized, &Config::default())?;
    Ok(())
}
````

## Place images at coordinates

`x` is a column offset; `y` is a row offset. For TUIs/panes, set width/height plus offsets.

````rust
let conf = viuer::Config {
    x: 40,
    y: 2,
    width: Some(30),
    height: Some(15),
    ..Default::default()
};
````

### Cursor positioning: relative vs absolute

If you see content overwritten or placement drift, investigate:

* `absolute_offset: true` for absolute cursor positioning (helpful when you need deterministic placement)
* `restore_cursor: true` if you want the cursor returned after drawing (useful in mixed text/image output)

Exact behavior depends on your terminal and how you manage cursor state (often via `crossterm`).

## Performance tip: resize before printing

Large images can be slow due to encoding overhead (Kitty/iTerm base64; half-block color mapping). Pre-resize to what you’ll actually display.