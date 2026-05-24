# `clipboard.rs` crate

A modern, multi-format clipboard crate inspired by the Electron clipboard API. The only Rust crate that provides both clipboard change listeners and multi-format read/write across all three desktop platforms.

| Attribute        | Value                  |
|------------------|------------------------|
| Latest version   | **0.3.4** (2026-04-02) |
| Recent downloads | ~10K/month             |
| Repo stars       | **160**                |
| License          | MIT                    |
| MSRV             | 1.67                   |
| Edition          | 2021                   |
| Latest commit    | 2026-04-02             |

### URLs

- Repository: https://github.com/ChurchTao/clipboard-rs
- Docs: https://docs.rs/clipboard-rs
- Crates.io: https://crates.io/crates/clipboard-rs

### Functional Footprint

- **Multi-format read/write**: plain text, HTML, RTF (rich text), PNG/JPEG/BMP/TIFF images, file lists (URI), arbitrary custom formats
- **Clipboard change listener**: handler-based real-time monitoring -- standout feature, enables history without polling
- **Thumbnail generation** for images
- **Custom format access** via format identifiers (e.g. macOS UTI types, Windows custom CF formats)
- **Configurable X11 read timeout** via `ClipboardContext::new_with_options` (default 500ms)
- **Platform coverage**: Windows, macOS, Linux X11, Linux Wayland (opt-in), iOS (beta), Android (in progress)

### Features (v0.3.4)

| Feature   | Default | Description                                                      |
|-----------|---------|------------------------------------------------------------------|
| `default` | Yes     | Enables `image`                                                  |
| `image`   | Yes     | PNG/JPEG/BMP/TIFF image clipboard support; pulls in `image` 0.25 |
| `wayland` | No      | Wayland support via `wl-clipboard-rs` 0.9 (Unix only)            |

### Platform Backends

- **Windows**: `windows` 0.59 + `clipboard-win` 5.4.1 (with `monitor` feature for change detection)
- **macOS**: `objc2` 0.6.3 (NSPasteboard with `changeCount` polling)
- **Linux/X11**: `x11rb` 0.13.2 (impl borrows from `x11-clipboard`)
- **Linux/Wayland**: optional via `wayland` feature, uses `wl-clipboard-rs`

### Typical Use Cases

- Clipboard managers and history tools (the exact use case for biscuit-clipboard)
- Apps that need to round-trip rich content (HTML/RTF, e.g. snippet managers)
- Cross-platform automation tools that need to detect clipboard changes
- File-aware clipboard utilities (drag/drop file lists)

### Code Examples

#### Multi-format read

```rust
use clipboard_rs::{Clipboard, ClipboardContext, ContentFormat};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = ClipboardContext::new()?;

    if ctx.has(ContentFormat::Text) {
        println!("Text: {}", ctx.get_text()?);
    }
    if ctx.has(ContentFormat::Html) {
        println!("HTML: {}", ctx.get_html()?);
    }
    if ctx.has(ContentFormat::Rtf) {
        println!("RTF: {}", ctx.get_rich_text()?);
    }
    if ctx.has(ContentFormat::Image) {
        let img = ctx.get_image()?;
        println!("Image: {}x{}", img.get_width(), img.get_height());
    }
    if ctx.has(ContentFormat::Files) {
        for f in ctx.get_files()? {
            println!("File: {}", f);
        }
    }
    Ok(())
}
```

#### Clipboard change listener (the killer feature for history)

```rust
use clipboard_rs::{
    Clipboard, ClipboardContext, ClipboardHandler, ClipboardWatcher,
    ClipboardWatcherContext,
};

struct Manager {
    ctx: ClipboardContext,
}

impl ClipboardHandler for Manager {
    fn on_clipboard_change(&mut self) {
        // record timestamp + content type + payload here
        if let Ok(text) = self.ctx.get_text() {
            println!("clipboard changed: {}", text);
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manager = Manager { ctx: ClipboardContext::new()? };
    let mut watcher = ClipboardWatcherContext::new()?;
    watcher.add_handler(manager);
    watcher.start_watch();
    Ok(())
}
```
