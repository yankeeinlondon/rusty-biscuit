# Image Rendering

The `TerminalImage` component renders images inline using the Kitty graphics protocol or iTerm2 protocol, with automatic fallback to alt text for unsupported terminals.

## Basic Usage

```rust
use biscuit_terminal::components::terminal_image::TerminalImage;
use biscuit_terminal::terminal::Terminal;
use std::path::Path;

let term = Terminal::new();
let img = TerminalImage::new(Path::new("photo.jpg"))?;

let output = img.render_to_terminal(&term)?;
print!("{}", output);
```

## Width Specifications

### ImageWidth Enum

```rust
pub enum ImageWidth {
    Fill,               // Fill available width
    Percent(f32),       // Percentage (0.0-1.0)
    Characters(u32),    // Fixed column count
}

impl Default for ImageWidth {
    fn default() -> Self {
        ImageWidth::Percent(0.5)  // 50% default
    }
}
```

### Builder Pattern

```rust
let img = TerminalImage::new(path)?
    .with_width(ImageWidth::Percent(0.75))  // 75% of terminal
    .with_margins(2, 2)                      // Left/right margins
    .with_alt_text("Screenshot of UI");
```

### Parsing Width Specs

For CLI or user input:

```rust
use biscuit_terminal::components::terminal_image::{
    parse_filepath_and_width, parse_width_spec, TerminalImage
};

// Parse "image.png|50%" format
let (filepath, width_spec) = parse_filepath_and_width("image.png|50%")?;

// Parse width string directly
let width = parse_width_spec("75%")?;   // Percent(0.75)
let width = parse_width_spec("80")?;    // Characters(80)
let width = parse_width_spec("fill")?;  // Fill

// Full spec parsing
let img = TerminalImage::from_spec("photo.jpg|fill")?;
```

### Width Examples

```
image.png           → Default 50% width
image.png|25%       → 25% of terminal width
image.png|80        → Fixed 80 columns
image.png|fill      → Fill available width
```

## Protocol Selection

The library automatically selects the best protocol:

```rust
match term.image_support {
    // Kitty protocol for these terminals
    ImageSupport::Kitty => {
        // Kitty, WezTerm, Ghostty, Konsole, Warp
        img.render_as_kitty(term_width)
    }
    // iTerm2 protocol (forced for iTerm2 even if Kitty advertised)
    ImageSupport::ITerm => {
        img.render_as_iterm2(term_width)
    }
    // Fallback to alt text
    ImageSupport::None => {
        Ok(img.generate_alt_text())
    }
}
```

### iTerm2 Special Handling

iTerm2 advertises Kitty protocol support but can fail with it. The library forces the native iTerm2 protocol when `TERM_PROGRAM=iTerm.app`:

```rust
// In render_to_terminal():
match term.image_support {
    ImageSupport::Kitty if matches!(term.app, TerminalApp::ITerm2) => {
        self.render_as_iterm2(width)  // Force iTerm2 protocol
    }
    ImageSupport::Kitty => self.render_as_kitty(width),
    // ...
}
```

## Direct Protocol Rendering

For lower-level control:

```rust
// Kitty protocol with cell dimensions
let output = img.render_as_kitty(80)?;

// iTerm2 protocol
let output = img.render_as_iterm2(80)?;

// Raw protocol (with PNG data already loaded)
let png_data = img.encode_as_png(&loaded_image)?;
let kitty_escape = img.render_kitty_cells(&png_data, width_cells, height_cells);
let iterm_escape = img.render_iterm2(&png_data, "40", "filename.png");
```

## Cell Size and Aspect Ratio

The library uses measured cell size for correct aspect ratio:

```rust
use biscuit_terminal::discovery::fonts::cell_size;

// Returns pixel dimensions of terminal cells
if let Some(cs) = cell_size() {
    println!("Cell: {}x{} pixels", cs.width, cs.height);
}
// Falls back to 8×16 if detection fails
```

This prevents "squished" images in terminals with non-2:1 cell aspect ratios (like WezTerm).

## Error Handling

```rust
pub enum TerminalImageError {
    FileNotFound { path: String },
    InvalidPath { path: String, reason: String },
    InvalidWidthSpec { spec: String },
    IoError(std::io::Error),
    ImageLoadError(image::ImageError),
    EncodingError { message: String },
    UnsupportedTerminal,
}
```

## Alt Text Generation

```rust
// Default: generates from filename
let alt = img.generate_alt_text();  // "[Image: photo.jpg]"

// Custom alt text
let img = img.with_alt_text("Product screenshot");
```

## Kitty Protocol Details

The Kitty graphics protocol transmits images as base64-encoded PNG:

```
ESC_G f=100,a=T,t=d,c=<cols>,r=<rows>,m=1;<base64_chunk> ESC\
ESC_G m=0;<final_chunk> ESC\
```

- `f=100`: PNG format
- `a=T`: Transmit and display
- `t=d`: Direct (inline) data
- `c=`/`r=`: Cell dimensions
- `m=0|1`: More chunks flag
- Data chunked at 4096 bytes

## iTerm2 Protocol Details

```
ESC]1337;File=name=<base64_name>;inline=1;preserveAspectRatio=1;width=<spec>;size=auto:<base64_data>BEL
```

## Complete Example

```rust
use biscuit_terminal::components::terminal_image::{TerminalImage, ImageWidth};
use biscuit_terminal::terminal::Terminal;
use std::path::Path;

fn display_image(path: &str, width_pct: f32) -> Result<(), Box<dyn std::error::Error>> {
    let term = Terminal::new();

    let img = TerminalImage::new(Path::new(path))?
        .with_width(ImageWidth::Percent(width_pct))
        .with_alt_text(format!("Image: {}", path));

    match img.render_to_terminal(&term) {
        Ok(output) => print!("{}", output),
        Err(e) => eprintln!("Failed to render image: {}", e),
    }

    Ok(())
}
```

## Related

- [Terminal Struct](./terminal-struct.md) - ImageSupport detection
- [Discovery Functions](./discovery.md) - image_support() function
