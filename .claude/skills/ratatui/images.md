# Images in Terminals

The `ratatui-image` crate enables image rendering in terminals using various protocols.

## Image Protocols

| Protocol | Description | Terminal Support | Best For |
|----------|-------------|-----------------|----------|
| **Kitty** | Most advanced; supports Z-levels and alpha | Kitty, WezTerm | Modern terminals, animations |
| **Sixel** | Classic standard; palette-based (256 colors) | Foot, Alacritty, iTerm2 | Wide compatibility |
| **iTerm2** | Base64-encoded; very stable | iTerm2, WezTerm | macOS terminals |
| **Halfblocks** | Unicode characters ▀ and ▄ | Everywhere | Universal fallback |

## Basic Usage

```toml
[dependencies]
ratatui-image = "1.0"
```

```rust
use ratatui_image::{picker::Picker, protocol::StaticImage};

struct App {
    image: Box<dyn ratatui_image::protocol::Protocol>,
}

impl App {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Query terminal for best protocol
        let mut picker = Picker::from_query_stdio()?;

        // Load image
        let dyn_img = image::open("logo.png")?;
        let image = picker.new_protocol(dyn_img, area, resize)?;

        Ok(Self { image })
    }

    fn render(&self, f: &mut Frame, area: Rect) {
        let image_widget = ratatui_image::Image::new(self.image.as_ref());
        f.render_widget(image_widget, area);
    }
}
```

## Fallback for Font Size Detection

```rust
let picker = Picker::from_query_stdio()
    .unwrap_or_else(|_| {
        Picker::new((8, 16))  // Fallback to 8x16 font
    });
```

## Stateful Images (for zooming/panning)

```rust
use ratatui_image::protocol::StatefulImage;

struct App {
    image_state: ImageState,
    image_protocol: Box<dyn Protocol>,
}

fn render(f: &mut Frame, app: &mut App, area: Rect) {
    let image = StatefulImage::new(None);
    f.render_stateful_widget(
        image,
        area,
        &mut app.image_state
    );
}
```

## Common Issues

### Ghosting/Artifacting

**Problem**: Image pixels stay stuck when resizing or scrolling

**Solution**: Use `StatefulImage` or ensure Ratatui marks cells as "skipped" in the buffer

### Large Images

**Problem**: 4K images cause memory spikes and lag

**Solution**: Pre-resize images to terminal-friendly sizes (max 800px width):
```rust
use image::imageops::FilterType;

let resized = dyn_img.resize(800, 600, FilterType::Lanczos3);
```

### Tmux/Screen Incompatibility

**Problem**: Images don't show in tmux

**Solution**: Enable passthrough in `.tmux.conf`:
```tmux
set -g allow-passthrough on
```

### Font Size Detection Fails

**Problem**: `Picker::from_query_stdio` fails on Windows or in multiplexers

**Solution**: Provide fallback font size manually:
```rust
let picker = match Picker::from_query_stdio() {
    Ok(p) => p,
    Err(_) => Picker::new((8, 16)),
};
```

## Best Practices

1. **Let Picker choose protocol** - Don't hardcode Sixel or Kitty
2. **Pre-resize large images** - Avoid loading 4K PNGs directly
3. **Test across terminals** - Verify rendering in different environments
4. **Provide text fallbacks** - Not all terminals support graphics
5. **Cache protocol instances** - Don't recreate on every frame
