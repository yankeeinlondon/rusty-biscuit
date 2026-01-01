# Integration patterns (image, crossterm, ratatui-image)

## With `image` (decode + optional processing)

`viuer` expects a decoded `DynamicImage`. Do transformations with `image` first (resize/crop/rotate), then print.

````rust
use viuer::{print, Config};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let img = image::open("example.jpg")?;
    // do image ops here if needed...
    print(&img, &Config::default())?;
    Ok(())
}
````

## With `crossterm` (size + cursor coordination)

Use terminal size to scale or to place images within a layout.

````rust
use crossterm::terminal;
use viuer::{print, Config};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (cols, _rows) = terminal::size()?;
    let img = image::open("logo.png")?;

    let conf = Config {
        width: Some(cols as u32 / 2),
        x: 5,
        y: 2,
        ..Default::default()
    };

    print(&img, &conf)?;
    Ok(())
}
````

If you have placement issues in a mixed text UI:

* consider `absolute_offset: true`
* consider managing cursor explicitly via `crossterm` and enabling `restore_cursor`

## With Ratatui TUIs

`viuer` is “print-and-forget”, which can be awkward in a redraw loop. Prefer `ratatui-image`:

* It’s a widget approach that handles redraw/erase/resize semantics within a frame.

When to still use viuer directly in a TUI:

* One-off splash/logo at startup
* Dedicated pane you fully control, where you can manage cursor position and clearing

## Alternative selection quick notes

* If your app is fundamentally Ratatui-based: choose `ratatui-image`.
* If you need maximum quality in non-graphics terminals: `chafa`.
* If you need streaming frames/video: `rasteroid`.
* If you want ASCII-art aesthetics (not fidelity): `artem`.