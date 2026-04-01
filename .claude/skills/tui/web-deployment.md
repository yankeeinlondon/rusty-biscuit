# Web Deployment with Ratzilla

Ratzilla brings Ratatui applications to the web using WebAssembly, replacing terminal backends with web-based renderers.

## Backend Comparison

| Backend | Technology | Strengths | Weaknesses |
|---------|-----------|-----------|------------|
| **WebGl2Backend** | WebGL2 | Highest performance (60+ FPS) | Limited Unicode/emoji |
| **CanvasBackend** | Canvas 2D | Good balance, better Unicode | No text selection |
| **DomBackend** | HTML/CSS | Accessibility, text selection | Slowest performance |

## Setup

```toml
[dependencies]
ratatui = { version = "0.29", default-features = false }
ratzilla = "0.2"
console_error_panic_hook = "0.1"  # For debugging

[profile.release]
opt-level = 'z'      # Optimize for size
lto = true           # Link time optimization
codegen-units = 1
panic = 'abort'
```

## Basic Application

```rust
use std::{cell::RefCell, io, rc::Rc};
use ratzilla::{DomBackend, WebRenderer};

fn main() -> io::Result<()> {
    // Panic hook for debugging
    #[cfg(target_arch = "wasm32")]
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    // Create backend
    let backend = DomBackend::new()?;
    let terminal = Terminal::new(backend)?;

    // Shared state (must use Rc<RefCell> for closures)
    let counter = Rc::new(RefCell::new(0));

    // Handle events
    terminal.on_key_event({
        let counter_clone = counter.clone();
        move |key_event| {
            if key_event.code == KeyCode::Char(' ') {
                *counter_clone.borrow_mut() += 1;
            }
        }
    });

    // Render loop
    terminal.draw_web(move |f| {
        let count = counter.borrow();
        let paragraph = Paragraph::new(format!("Count: {}", count))
            .block(Block::bordered().title("Ratzilla"));
        f.render_widget(paragraph, f.area());
    });

    Ok(())
}
```

## Preventing Browser Scroll

```rust
use web_sys::KeyboardEvent;
use wasm_bindgen::prelude::*;

fn setup_input_handling() {
    let window = web_sys::window().expect("no window");

    let closure = Closure::wrap(Box::new(move |event: KeyboardEvent| {
        let keys_to_prevent = ["ArrowUp", "ArrowDown", " ", "Tab"];

        if keys_to_prevent.contains(&event.key().as_str()) {
            event.prevent_default();
        }
    }) as Box<dyn FnMut(KeyboardEvent)>);

    window
        .add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())
        .expect("failed to add listener");

    closure.forget();
}

fn main() -> io::Result<()> {
    #[cfg(target_arch = "wasm32")]
    setup_input_handling();

    // ... rest of app
}
```

## HTML Template

```html
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>Ratzilla TUI</title>
    <style>
        body, html {
            margin: 0;
            padding: 0;
            width: 100%;
            height: 100%;
            overflow: hidden;
            background-color: #000;
        }
        #ratzilla-container {
            width: 100vw;
            height: 100vh;
            font-family: 'Courier New', monospace;
        }
    </style>
</head>
<body>
    <div id="ratzilla-container"></div>
</body>
</html>
```

## Trunk Configuration

```toml
# Trunk.toml
[build]
target = "index.html"
dist = "dist"

[serve]
port = 8080
open = true

[clean]
dist = true
```

## Building and Deploying

### Local Development

```bash
# Install trunk
cargo install trunk

# Add WASM target
rustup target add wasm32-unknown-unknown

# Serve locally
trunk serve
```

### Production Build

```bash
trunk build --release --public-url "/your-repo-name/"
```

### GitHub Pages Deployment

```yaml
# .github/workflows/deploy.yml
name: Deploy to GitHub Pages

on:
  push:
    branches: [main]

permissions:
  contents: write

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-unknown-unknown

      - name: Install Trunk
        run: |
          wget -qO- https://github.com/trunk-rs/trunk/releases/download/v0.20.0/trunk-x86_64-unknown-linux-gnu.tar.gz | tar -xzf-
          sudo mv trunk /usr/local/bin/

      - name: Build
        run: trunk build --release --public-url "/${{ github.event.repository.name }}/"

      - name: Deploy
        uses: JamesIves/github-pages-deploy-action@v4
        with:
          folder: dist
          branch: gh-pages
```

## CRT Terminal Effect (CSS)

```css
:root {
    --terminal-green: #33ff33;
}

#ratzilla-container {
    text-shadow: 0 0 5px var(--terminal-green);
    filter: brightness(1.1) contrast(1.2);
}

/* Scanlines */
#ratzilla-container::before {
    content: " ";
    display: block;
    position: absolute;
    top: 0; left: 0; bottom: 0; right: 0;
    background: linear-gradient(
        rgba(18, 16, 16, 0.1) 50%,
        rgba(0, 0, 0, 0) 50%
    );
    background-size: 100% 4px;
    z-index: 10;
    pointer-events: none;
}

/* Flicker */
@keyframes flicker {
    0% { opacity: 0.98; }
    5% { opacity: 0.95; }
    10% { opacity: 0.99; }
    100% { opacity: 1; }
}

#ratzilla-container::after {
    content: " ";
    position: absolute;
    top: 0; left: 0; bottom: 0; right: 0;
    opacity: 0;
    animation: flicker 0.15s infinite;
    pointer-events: none;
}
```

## Common Gotchas

### Blocking Loop

**Problem**: `while loop` freezes browser

**Solution**: Use `terminal.draw_web()` instead of manual loop

### State Sharing

**Problem**: Can't share mutable state across closures

**Solution**: Use `Rc<RefCell<T>>` for shared state

### Public URL

**Problem**: WASM files not found on GitHub Pages

**Solution**: Set `--public-url` to repo name:
```bash
trunk build --release --public-url "/repo-name/"
```

### std::time::Instant

**Problem**: `Instant` doesn't work in WASM

**Solution**: Use `web-time` crate or `chrono`

## Best Practices

1. **Use panic hooks** - Enable `console_error_panic_hook` for debugging
2. **Prevent default events** - Stop browser from handling TUI keys
3. **Optimize for size** - Use `opt-level = 'z'` and LTO
4. **Test locally first** - Use `trunk serve` before deploying
5. **Provide loading indicator** - Show message while WASM loads
