# Mermaid Diagrams to Raster Image

Excellent question. You're correct that Mermaid officially requires JavaScript, but there are several robust ways to generate raster images from Mermaid diagrams for terminal rendering. Here are your best options, ordered by practicality:

## 1. **Mermaid CLI (Most Practical)**
The official [`@mermaid-js/mermaid-cli`](https://github.com/mermaid-js/mermaid-cli) package is your simplest path. It's a Node.js tool that uses headless Chrome to render diagrams to PNG/SVG.

**Implementation:**

```rust
use std::process::Command;
use std::fs;
use std::io::Write;

fn render_mermaid_to_png(source: &str, output_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Create unique temp file to avoid race conditions
    let mut temp_file = tempfile::NamedTempFile::with_suffix(".mmd")?;
    temp_file.write_all(source.as_bytes())?;

    // Call mermaid CLI
    let status = Command::new("mmdc")
        .args([
            "-i", temp_file.path().to_str().unwrap(),
            "-o", output_path,
            "--scale", "2",  // High DPI for better terminal rendering
            "--backgroundColor", "transparent"
        ])
        .status()?;

    if !status.success() {
        return Err(format!("mmdc exited with status: {}", status).into());
    }

    Ok(())
    // temp_file is automatically deleted when dropped
}
```

**Pros:** ✅ Most accurate rendering ✅ Maintains feature parity ✅ SVG/PNG output
**Cons:** ❌ Requires Node.js/Puppeteer ❌ Heavy dependency (~300MB)
**Installation:** `npm install -g @mermaid-js/mermaid-cli`

## 2. **Headless Browser Directly (More Control)**
Control a browser directly from Rust for better error handling and performance.

**Using `headless_chrome` crate:**

```rust
use headless_chrome::{Browser, LaunchOptions};
use std::fs;

fn render_mermaid(source: &str, output_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let browser = Browser::new(
        LaunchOptions::default_builder()
            .headless(true)
            .build()?
    )?;

    let tab = browser.new_tab()?;

    // Escape source for safe HTML embedding
    let escaped_source = source
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");

    let html = format!(r#"<!DOCTYPE html>
<html>
<head>
    <script src="https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.min.js"></script>
</head>
<body>
    <pre class="mermaid">{escaped_source}</pre>
    <script>mermaid.initialize({{startOnLoad:true}});</script>
</body>
</html>"#);

    tab.navigate_to(&format!("data:text/html,{}", urlencoding::encode(&html)))?;

    // Wait for mermaid to render (check for SVG element)
    tab.wait_for_element("svg")?;

    // Capture screenshot of the diagram element
    let element = tab.find_element(".mermaid svg")?;
    let png_data = element.capture_screenshot(
        headless_chrome::protocol::cdp::Page::CaptureScreenshotFormatOption::Png
    )?;

    fs::write(output_path, png_data)?;
    Ok(())
}
```

**Pros:** ✅ Pure Rust control ✅ No temp files needed
**Cons:** ❌ Complex ❌ Still requires Chrome/Chromium

## 3. **External Rendering Service (Lightweight)**
Use a hosted service like [mermaid.ink](https://mermaid.ink) (free, rate-limited) or self-host it.

```rust
use base64::{Engine, engine::general_purpose::STANDARD};
use std::fs;

async fn render_mermaid_via_service(source: &str, output_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // mermaid.ink uses base64 encoding (NOT URL encoding)
    let encoded = STANDARD.encode(source);
    let url = format!("https://mermaid.ink/img/base64:{}", encoded);

    let response = reqwest::get(&url).await?;

    if !response.status().is_success() {
        return Err(format!("mermaid.ink returned {}", response.status()).into());
    }

    let bytes = response.bytes().await?;
    fs::write(output_path, &bytes)?;

    Ok(())
}
```

**Pros:** ✅ No local dependencies ✅ Fast
**Cons:** ❌ Requires internet ❌ Privacy concerns ❌ Rate limits ❌ Max ~4KB diagram size (URL length limit)

## 4. **Alternative: SVG → Raster Conversion**
If you can get SVG output (Mermaid CLI can generate it), convert to raster with native Rust:

**Using `resvg` crate:**

```rust
use resvg::{tiny_skia, usvg};

fn svg_to_png(svg_data: &str, output_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Parse SVG (use from_data for &[u8], from_str doesn't exist)
    let tree = usvg::Tree::from_data(svg_data.as_bytes(), &usvg::Options::default())?;

    // Create pixmap matching SVG dimensions
    let size = tree.size();
    let mut pixmap = tiny_skia::Pixmap::new(size.width() as u32, size.height() as u32)
        .ok_or("Failed to create pixmap (invalid dimensions)")?;

    // Render at original size
    resvg::render(&tree, tiny_skia::Transform::identity(), &mut pixmap.as_mut());

    // Save to PNG
    pixmap.save_png(output_path)?;
    Ok(())
}
```

**Pros:** ✅ Pure Rust ✅ Fast ✅ Good for SVG pipeline ✅ Memory-safe
**Cons:** ❌ Still need SVG source first (use `mmdc -o diagram.svg` to get it)

## 5. **Native Alternatives (For Specific Diagram Types)**
For *some* diagrams, consider native tools that don't need JS:

| Diagram Type | Tool | Rust Crate | Command |
|--------------|------|------------|---------|
| Flowcharts/Graphs | Graphviz | `dot` rendering | `dot -Tpng input.dot -o output.png` |
| Sequence | PlantUML | N/A (Java) | `plantuml -tpng input.puml` |
| ASCII Art | `ditaa` | N/A (Java) | Built-in rendering |

**Example integration:**

```rust
use std::process::Command;

fn render_graphviz(dot_source: &str, output_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut child = Command::new("dot")
        .args(["-Tpng", "-o", output_path])
        .stdin(std::process::Stdio::piped())
        .spawn()?;

    // Write DOT source to stdin
    use std::io::Write;
    child.stdin.take().unwrap().write_all(dot_source.as_bytes())?;

    let status = child.wait()?;
    if !status.success() {
        return Err("Graphviz failed".into());
    }
    Ok(())
}
```

**Pros:** ✅ No JS/browser ✅ Fast ✅ Widely available
**Cons:** ❌ Limited feature set ❌ Different syntax than Mermaid

---

## **Recommended Architecture**

Since you're already using `pulldown-cmark`, here's a clean integration:

```rust
use pulldown_cmark::{Event, Tag, TagEnd, CodeBlockKind, CowStr};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Replace mermaid code blocks with rendered images
fn process_mermaid_blocks<'a>(events: impl Iterator<Item = Event<'a>>) -> Vec<Event<'a>> {
    let mut result = Vec::new();
    let mut events = events.peekable();

    while let Some(event) = events.next() {
        match &event {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(lang)))
                if lang.as_ref() == "mermaid" =>
            {
                // Collect diagram source
                let mut source = String::new();
                for inner in events.by_ref() {
                    match inner {
                        Event::Text(text) => source.push_str(&text),
                        Event::End(TagEnd::CodeBlock) => break,
                        _ => {}
                    }
                }

                // Generate deterministic filename from content hash
                let mut hasher = DefaultHasher::new();
                source.hash(&mut hasher);
                let image_path = format!("/tmp/mermaid_{:x}.png", hasher.finish());

                // Render diagram (implement with Option 1, 2, or 3)
                match render_mermaid_to_png(&source, &image_path) {
                    Ok(()) => {
                        // Replace code block with image
                        let html = format!(
                            r#"<img src="{}" alt="Mermaid diagram" />"#,
                            image_path
                        );
                        result.push(Event::Html(CowStr::from(html)));
                    }
                    Err(e) => {
                        // Fallback: keep code block and add error comment
                        eprintln!("Mermaid render failed: {e}");
                        result.push(event);
                        result.push(Event::Text(CowStr::from(source)));
                        result.push(Event::End(TagEnd::CodeBlock));
                    }
                }
            }
            _ => result.push(event),
        }
    }
    result
}
```

## **My Recommendation**

1. **For production:** Use **Mermaid CLI (Option 1)** - it's the most reliable and maintainable
2. **For minimal dependencies:** Use **mermaid.ink service (Option 3)** with a fallback
3. **For maximum performance:** Self-host the Mermaid renderer (docker container) and use Option 2

Add these to your `Cargo.toml` based on which option(s) you choose:

```toml
[dependencies]
# For Option 1 (Mermaid CLI) - temp file handling
tempfile = "3"

# For Option 2 (Headless Chrome)
headless_chrome = "1.0.14"
urlencoding = "2"

# For Option 3 (mermaid.ink service)
reqwest = { version = "0.12", features = ["json"] }
base64 = "0.22"

# For Option 4 (SVG to PNG) - resvg re-exports usvg and tiny-skia
resvg = "0.44"
```

The key insight: **you don't need to reimplement Mermaid** - just automate the browser execution that Mermaid already needs, then pipe the raster output into your terminal renderer.
