---
prompt: |-
    Research the top crates used in Rust projects for clipboard management. We're looking for one or more crates
    to provide support macOS, Windows, and Linux for at least the following features:

    - copy to clipboard
    - retrieve latest clipboard content
    - retrieve last 10 clipboard entries (if available on host platform)
    - retrieve metadata about clipboard entries:
        - date/time added
        - image/text/audio/etc.
        - other?

    Your task is to create a list of all the top crates used in Rust projects for managing the clipboard.

    For each:

    - describe the functional footprint
    - list the latest version of the crate
    - list all the features the crate exposes and what each feature adds functionally
    - provide all important URL's (repo, docs, etc.)
    - repo stars
    - first commit date
    - latest commit date
    - describe the use cases this crate is typically used for
    - give a code example or two of how this crate would be used in a Rust project

    After compiling the list, provide recommendations:

    - your recommendations can be a single crate or a combination of crates but you must explain WHY you think this combination is the best way to achieve the broad ambitions of this biscuit-clipboard's requirements.
last_updated: 2026-05-02
---

# Rust Clipboard Crate Research

Research into the top crates used in Rust projects for clipboard management, evaluated against biscuit-clipboard's requirements for cross-platform (macOS, Windows, Linux) clipboard access, history, and metadata.

## Summary of Requirements

| Requirement                 | Notes                                                                                                                                  |
|-----------------------------|----------------------------------------------------------------------------------------------------------------------------------------|
| Copy to clipboard           | Set text/image data on the system clipboard                                                                                            |
| Retrieve latest content     | Get the current clipboard contents (text, image, HTML, RTF, files)                                                                     |
| Retrieve last 10 entries    | Clipboard history -- **no Rust crate provides this natively**, but `clipboard-rs` provides change listeners that make it implementable |
| Metadata (date, type, etc.) | Timestamps, content-type detection -- only `clipboard-rs` exposes multiple typed formats (text/HTML/RTF/image/files)                   |

**Key finding:** No Rust crate ships a ready-made clipboard history store, but `clipboard-rs` is the only crate that exposes both a clipboard change listener and multiple content formats (HTML, RTF, files, images, custom). This makes building a history layer dramatically simpler than the polling-based approach required for `arboard`.

---

## 1. clipboard-rs (Primary Recommendation)

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

---

## 2. arboard (Secondary Recommendation / Fallback)

The dominant general-purpose clipboard crate for Rust. Maintained by 1Password staff. Forked from `rust-clipboard` and significantly evolved since. Best-in-class for simple text+image use cases but lacks change listeners and rich-text formats.

| Attribute        | Value                  |
|------------------|------------------------|
| Latest version   | **3.6.1** (2025-08-23) |
| Recent downloads | ~2.6M/month            |
| Repo stars       | **925**                |
| License          | MIT OR Apache-2.0      |
| MSRV             | 1.71.0                 |
| Edition          | 2021                   |
| First commit     | 2020-07-04             |
| Latest commit    | 2025-08-23             |

### URLs

- Repository: https://github.com/1Password/arboard
- Docs: https://docs.rs/arboard/3.6.1
- Crates.io: https://crates.io/crates/arboard

### Functional Footprint

- **Text**: get and set UTF-8 text on the system clipboard
- **Images**: get and set RGBA image data (via `ImageData` struct or the `image` crate's `RgbaImage`)
- **Clear**: clear the clipboard contents
- **Linux clipboard selection**: choose between `Clipboard`, `Primary`, and `Secondary` selections
- **Wayland support**: optional native Wayland data-control protocol support (not enabled by default)
- **Builder API**: `Clipboard::new()` returns a `Clipboard` handle; operations use `get_text()`, `set_text()`, `get_image()`, `set_image()`, with chainable `.wait()` and `.clipboard()` on Linux
- **No change detection / no HTML / no RTF / no file lists** -- callers must poll and diff

### Features (v3.6.1)

| Feature                | Default | Description                                                                                   |
|------------------------|---------|-----------------------------------------------------------------------------------------------|
| `default`              | Yes     | Enables `image-data`                                                                          |
| `image-data`           | Yes     | Enables image clipboard support; pulls in `image` crate, plus platform-specific graphics deps |
| `image`                | No      | Re-exports the `image` crate as a dependency for `RgbaImage` interop                          |
| `wayland-data-control` | No      | Enables native Wayland clipboard via `wl-clipboard-rs`; uses data-control protocol extensions |
| `wl-clipboard-rs`      | No      | Alias/inner dep gate for the Wayland backend                                                  |
| `core-graphics`        | No      | macOS: pulls in `objc2-core-graphics` for image data conversion                               |
| `windows-sys`          | No      | Windows: pulls in `windows-sys/Win32_Graphics_Gdi` for image data conversion                  |

### Platform Backends

- **macOS**: `objc2-app-kit`, `objc2-foundation` (AppKit NSPasteboard)
- **Windows**: `clipboard-win` 5.3.1 + `windows-sys` (Win32 clipboard API)
- **Linux/X11**: `x11rb` 0.13 (X11 clipboard via X selections)
- **Linux/Wayland**: optional via `wayland-data-control`, uses `wl-clipboard-rs`

### Typical Use Cases

- GUI and TUI applications that need clipboard interop
- CLI tools that pipe stdin to clipboard or clipboard to stdout
- Password managers (1Password's own use)
- Image editors and screenshot tools that need to copy/paste image data
- Any Rust application that needs reliable cross-platform text+image clipboard access without extra format complexity

### Code Examples

```rust
use arboard::Clipboard;

fn main() -> Result<(), arboard::Error> {
    let mut clipboard = Clipboard::new()?;

    let text = clipboard.get_text()?;
    println!("Current clipboard: {}", text);

    clipboard.set_text("Hello from biscuit-clipboard!")?;

    if let Ok(image) = clipboard.get_image() {
        println!("Image: {}x{} pixels", image.width, image.height);
    }
    Ok(())
}
```

---

## 3. copypasta

Alacritty's clipboard library. A fork of `rust-clipboard` that added Wayland support. Text-only; does not support image data or change detection.

| Attribute        | Value                   |
|------------------|-------------------------|
| Latest version   | **0.10.2** (2025-04-25) |
| Recent downloads | ~200K/month             |
| Repo stars       | **348**                 |
| License          | MIT / Apache-2.0        |
| MSRV             | 1.71.0                  |
| Edition          | 2021                    |
| Latest commit    | 2025-05-09              |

### URLs

- Repository: https://github.com/alacritty/copypasta
- Docs: https://docs.rs/copypasta
- Crates.io: https://crates.io/crates/copypasta

### Functional Footprint

- **Text only**: get and set `String` contents via `ClipboardProvider` trait
- **Trait-based API**: `ClipboardProvider` with `get_contents()` and `set_contents()`
- **Platform dispatch**: `ClipboardContext` type alias selects platform backend at compile time
- **NopClipboardContext**: fallback no-op clipboard for platforms without support

### Features (v0.10.2)

| Feature          | Default | Description                                                          |
|------------------|---------|----------------------------------------------------------------------|
| `x11`            | Yes     | X11 clipboard via `x11-clipboard`                                    |
| `wayland`        | Yes     | Wayland clipboard via `smithay-clipboard` (requires Wayland surface) |
| `wayland-dlopen` | Yes     | Runtime Wayland lib loading via `smithay-clipboard/dlopen`           |

### Code Example

```rust
use copypasta::{ClipboardContext, ClipboardProvider};

fn main() {
    let mut ctx = ClipboardContext::new().unwrap();
    ctx.set_contents("Hello, world!".to_owned()).unwrap();
    println!("{}", ctx.get_contents().unwrap());
}
```

---

## 4. clipboard-win

Windows-specific low-level Win32 clipboard crate. Used internally by `arboard` and `clipboard-rs` as their Windows backend. Worth knowing about because of its **`monitor` feature**, which provides clipboard change notifications on Windows -- the building block for change detection on Windows.

| Attribute        | Value                  |
|------------------|------------------------|
| Latest version   | **5.4.1** (2025-07-17) |
| Recent downloads | ~2.85M/month           |
| Repo stars       | **74**                 |
| License          | BSL-1.0                |
| First commit     | 2015-07-12             |
| Latest commit    | 2025-07-17             |

### URLs

- Repository: https://github.com/DoumanAsh/clipboard-win
- Docs: https://docs.rs/clipboard-win
- Crates.io: https://crates.io/crates/clipboard-win

### Functional Footprint

- **Raw Win32 clipboard access**: open, close, empty, get/set data in various formats
- **Format management**: enumerate and inspect clipboard formats
- **Unicode text, bitmap images, file lists, custom formats**
- **Clipboard sequence numbers**: `get_sequence_number()` to detect changes without reading data
- **Change monitoring**: `monitor` feature wraps the Win32 clipboard listener API

### Features (v5.4.1)

| Feature   | Default | Description                                          |
|-----------|---------|------------------------------------------------------|
| `std`     | Yes     | Standard library support                             |
| `monitor` | No      | Clipboard change monitoring (adds `windows-win` dep) |

### Code Example

```rust
use clipboard_win::{get_clipboard_string, set_clipboard_string};

fn main() {
    set_clipboard_string("Hello from Windows!").unwrap();
    println!("{}", get_clipboard_string().unwrap());
}
```

---

## 5. wl-clipboard-rs

Wayland clipboard library for **terminal/headless apps that don't own a Wayland surface**. Used by `arboard` (via the `wayland-data-control` feature) and `clipboard-rs` (via the `wayland` feature).

| Attribute        | Value                  |
|------------------|------------------------|
| Latest version   | **0.9.3** (2025-12-13) |
| Recent downloads | ~858K/month            |
| Repo stars       | **475**                |
| License          | MIT/Apache-2.0         |
| First commit     | 2019-02-12             |
| Latest commit    | 2025-12-13             |

### URLs

- Repository: https://github.com/YaLTeR/wl-clipboard-rs
- Docs: https://docs.rs/wl-clipboard-rs
- Crates.io: https://crates.io/crates/wl-clipboard-rs

### Features (v0.9.3)

| Feature      | Default | Description                                                          |
|--------------|---------|----------------------------------------------------------------------|
| `dlopen`     | No      | Runtime loading of `libwayland-client.so` (requires `native_lib`)    |
| `native_lib` | No      | Link `libwayland-client.so` directly instead of using pure-Rust impl |

### Status: **Infrastructure dependency**

Not a direct user-facing crate; pulled in transitively. Worth knowing about because `clipboard-rs`'s `wayland` feature and `arboard`'s `wayland-data-control` feature both route through it.

---

## 6. smithay-clipboard

Wayland clipboard for **GUI apps that already own a Wayland surface** (winit/iced/egui style). Explicitly NOT for CLI/headless apps or clipboard managers. Used by `copypasta`. Listed for disambiguation -- biscuit-clipboard should NOT depend on this directly; it must use `wl-clipboard-rs` instead because clipboard managers are headless by nature.

| Attribute        | Value                  |
|------------------|------------------------|
| Latest version   | **0.7.3** (2025-11-02) |
| Recent downloads | ~1.18M/month           |
| Repo stars       | **35**                 |
| License          | MIT                    |

---

## 7. x11-clipboard

Low-level X11 clipboard primitive. Used by `arboard`, `clipboard-rs`, `copypasta`, and `terminal-clipboard`. **Maintenance mode** as of late 2024 per the upstream author.

| Attribute        | Value                  |
|------------------|------------------------|
| Latest version   | **0.9.3** (2024-10-17) |
| Recent downloads | ~416K/month            |
| Repo stars       | **52**                 |
| License          | MIT/Apache-2.0         |

Not a direct dependency for biscuit-clipboard.

---

## 8. terminal-clipboard

Minimal text-only facade by the `broot`/`dysk` author. Adds Termux/Android support that other crates don't.

| Attribute        | Value                  |
|------------------|------------------------|
| Latest version   | **0.4.1** (2023-10-31) |
| Recent downloads | ~1K/month              |
| Repo stars       | **11**                 |
| License          | MIT                    |

Not recommended for biscuit-clipboard -- text-only, no image support, no change detection.

---

## 9. clipboard (rust-clipboard)

The original Rust clipboard crate. **Unmaintained** -- last publish was 2018. The crate that `copypasta`, `arboard`, `cli-clipboard`, and `clipboard-rs` all eventually trace back to.

| Attribute        | Value                  |
|------------------|------------------------|
| Latest version   | **0.5.0** (2018-09-22) |
| Recent downloads | ~25K/month (legacy)    |
| Repo stars       | **378**                |
| License          | MIT / Apache-2.0       |

### Status: **DEPRECATED** -- do not use.

---

## 10. cli-clipboard

A fork of `rust-clipboard` focused on CLI usage. **Not actively maintained** -- last publish was 2022.

| Attribute        | Value                  |
|------------------|------------------------|
| Latest version   | **0.4.0** (2022-12-13) |
| Recent downloads | ~21K/month             |
| Repo stars       | **23**                 |
| License          | MIT / Apache-2.0       |

### Status: **NOT RECOMMENDED** -- superseded by `arboard` and `clipboard-rs`.

---

## Comparison Matrix

| Feature              | clipboard-rs             | arboard    | copypasta     | clipboard-win        | wl-clipboard-rs |
|----------------------|--------------------------|------------|---------------|----------------------|-----------------|
| **Text get/set**     | Yes                      | Yes        | Yes           | Yes (Win)            | Yes (Wayland)   |
| **Image get/set**    | Yes (PNG/JPEG/BMP/TIFF)  | Yes (RGBA) | No            | Yes (DIB)            | MIME-typed      |
| **HTML**             | **Yes**                  | No         | No            | format-only          | No              |
| **RTF**              | **Yes**                  | No         | No            | format-only          | No              |
| **File lists**       | **Yes**                  | No         | No            | Yes (CF_HDROP)       | MIME-typed      |
| **Custom formats**   | **Yes**                  | No         | No            | Yes                  | MIME-typed      |
| **Change listener**  | **Yes (cross-platform)** | No         | No            | Yes (`monitor` feat) | No              |
| **macOS**            | Yes                      | Yes        | Yes           | No                   | No              |
| **Windows**          | Yes                      | Yes        | Yes           | Yes                  | No              |
| **Linux X11**        | Yes                      | Yes        | Yes           | No                   | No              |
| **Linux Wayland**    | Yes (opt)                | Yes (opt)  | Yes (surface) | No                   | Yes             |
| **iOS / Android**    | Beta / WIP               | No         | No            | No                   | No              |
| **Maintained**       | **Active**               | **Active** | Active        | Active               | Active          |
| **Stars**            | 160                      | 925        | 348           | 74                   | 475             |
| **Recent downloads** | 10K/mo                   | 2.6M/mo    | 200K/mo       | 2.85M/mo             | 858K/mo         |

---

## Recommendations

### Primary recommendation: **`clipboard-rs`** as the sole backend

For a clipboard *manager* like biscuit-clipboard (history, metadata, multi-format), `clipboard-rs` is the right primitive:

1. **Only crate with a cross-platform clipboard change listener** -- this is the foundation for clipboard history. Polling-based history (the only option with `arboard`) wastes CPU and can miss rapid changes.
2. **Multi-format support out of the box** -- text, HTML, RTF, files, images, custom formats. This directly satisfies the "metadata about content type" requirement: each format is its own typed accessor (`get_text`, `get_html`, `get_rich_text`, `get_files`, `get_image`).
3. **All three target platforms** -- Windows, macOS, Linux X11 + optional Wayland.
4. **Actively maintained** (latest release 2026-04-02, MSRV 1.67, MIT license).
5. **Inspired by Electron's clipboard API** -- well-trodden design that maps cleanly to a clipboard-manager use case.

```toml
[dependencies]
clipboard-rs = { version = "0.3", features = ["image", "wayland"] }
```

### Secondary: keep `arboard` as a fallback option

Despite the recommendation above, `arboard` is far more widely deployed (925 stars, 2.6M monthly downloads vs. clipboard-rs's 160/10K). If `clipboard-rs` proves unstable or its Linux backends don't meet biscuit-clipboard's reliability bar, `arboard` is the obvious fallback for the get/set primitives. A trait-based abstraction inside biscuit-clipboard could make swapping backends cheap.

### What `clipboard-rs` does NOT provide (and biscuit-clipboard must build)

Even with change listeners and typed formats, no Rust crate provides clipboard *history*. biscuit-clipboard must layer:

| Requirement                            | Strategy                                                                                                                                                                                                                                      |
|----------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| **Clipboard history** (last N entries) | On each `on_clipboard_change` callback, snapshot every available format and push into a ring buffer / SQLite store                                                                                                                            |
| **Entry timestamps**                   | Record `SystemTime::now()` inside the change handler. No OS surfaces a real "clipboard entry timestamp"                                                                                                                                       |
| **Entry type**                         | Probe `ContentFormat::{Text, Html, Rtf, Image, Files}` -- store the highest-fidelity type plus any alternates                                                                                                                                 |
| **Deduplication**                      | Hash payload contents (e.g. `biscuit-hash` xxHash) to skip identical re-copies                                                                                                                                                                |
| **Audio clipboard**                    | Not supported on any major OS clipboard at the format level. macOS's `NSPasteboard` and Win32 don't have an audio clipboard format. If an audio file is copied, it surfaces as a file-list entry -- biscuit-clipboard can sniff the MIME type |

### Why NOT the other crates

| Crate                | Reason to skip                                                                         |
|----------------------|----------------------------------------------------------------------------------------|
| `copypasta`          | Text-only, no image, no change detection, no rich formats                              |
| `clipboard`          | Unmaintained since 2018, no Wayland                                                    |
| `cli-clipboard`      | Unmaintained since 2022, shells out to `xclip`/`xsel` on Linux                         |
| `clipboard-win`      | Windows-only; already a transitive dep of `clipboard-rs` -- no need to depend directly |
| `wl-clipboard-rs`    | Already pulled in via `clipboard-rs`'s `wayland` feature                               |
| `smithay-clipboard`  | Requires a Wayland surface -- wrong abstraction for a headless clipboard manager       |
| `terminal-clipboard` | Text-only, no image, no change detection                                               |
| `x11-clipboard`      | Low-level primitive; already used by `clipboard-rs`                                    |

### Final recommendation

**Use `clipboard-rs` as the sole clipboard backend crate** with the `image` and `wayland` features enabled. Build biscuit-clipboard's history and metadata layers on top of its `ClipboardWatcherContext` change listener, capturing typed formats and timestamps in a ring-buffer-backed history store with content-hash deduplication. Keep the call sites isolated behind a small internal trait so a future swap to `arboard` (or both, gated by feature) remains cheap.
