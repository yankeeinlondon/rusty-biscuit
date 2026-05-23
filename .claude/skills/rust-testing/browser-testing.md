# Browser Render Testing (chromiumoxide)

Test HTML/CSS render output by driving a **real headless Chrome/Chromium** over
the Chrome DevTools Protocol with [`chromiumoxide`](https://crates.io/crates/chromiumoxide).
Use this when a crate emits HTML/CSS (Markdown→HTML, component libraries,
report generators) and you need to verify the browser actually *applies* the
styles — not merely that a substring appears in the source.

## Principles

- **Assert computed styles, not pixels.** `getComputedStyle(el).<prop>` is
  deterministic and stable across machines/OSes. Pixel screenshots vary with
  font rendering and platform; reserve them for opt-in visual baselines pinned
  to a fixed OS/container.
- **Assert what the browser computed, not what you emitted.** Reading back the
  resolved color/padding/width proves the CSS is valid and applied. A source
  substring (`html.contains("background-color")`) proves neither.
- **Wrap fragments into a standalone document.** A render *fragment* often has
  no `<html>`/`<body>` and no page background. Contrast/layout only exist once
  there is a page context, so wrap it: `<!doctype html><html><body
  style="background:…">{fragment}</body></html>`.
- **Skip cleanly when no browser is present.** Locate Chrome/Chromium and
  early-return when absent; gate hard-failure behind an env flag for CI that
  provisions a browser (mirrors the Level-2 terminal-harness convention).
- **Poll the handler.** `Browser::launch` returns `(browser, handler)`; the
  handler future MUST be driven (spawn a task that polls it) or no CDP messages
  flow and every call hangs.
- **Serialize browser tests** with `#[serial_test::serial(browser)]` — launching
  many Chrome instances in parallel is slow and flaky.

## Dependencies

```toml
[dev-dependencies]
chromiumoxide = { version = "0.7", default-features = false, features = ["tokio-runtime"] }
futures-util = "0.3"
tokio = { version = "1", features = ["full"] }
tempfile = "3"
serial_test = "3"
```

`chromiumoxide` does **not** bundle a browser; it drives a system one. Set
`default-features = false` + `tokio-runtime` so it doesn't pull in async-std.

## Browser discovery (skip-clean)

```rust
use std::path::PathBuf;

/// Locate a usable Chrome/Chromium executable, or `None` to skip.
fn find_chrome() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("CHROME") {
        let p = PathBuf::from(p);
        if p.exists() { return Some(p); }
    }
    for c in [
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        "/Applications/Chromium.app/Contents/MacOS/Chromium",
        "/usr/bin/google-chrome",
        "/usr/bin/chromium",
        "/usr/bin/chromium-browser",
    ] {
        let p = PathBuf::from(c);
        if p.exists() { return Some(p); }
    }
    // Fall back to a Playwright-managed Chromium if one is installed.
    if let Some(home) = std::env::var_os("HOME") {
        let base = PathBuf::from(home).join("Library/Caches/ms-playwright");
        if let Ok(entries) = std::fs::read_dir(base) {
            for e in entries.flatten() {
                if e.file_name().to_string_lossy().starts_with("chromium-") {
                    let exe = e.path().join("chrome-mac/Chromium.app/Contents/MacOS/Chromium");
                    if exe.exists() { return Some(exe); }
                }
            }
        }
    }
    None
}
```

## Computed-style assertion

```rust
use chromiumoxide::browser::{Browser, BrowserConfig};
use futures_util::StreamExt;

/// Render a standalone HTML document and return the browser-computed value of
/// `property` for the first element matching `selector`.
async fn computed_style(html_doc: &str, selector: &str, property: &str) -> Option<String> {
    let chrome = find_chrome()?;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("page.html");
    std::fs::write(&path, html_doc).unwrap();

    let config = BrowserConfig::builder()
        .chrome_executable(chrome)
        .arg("--no-sandbox")
        .build()
        .expect("BrowserConfig");
    let (browser, mut handler) = Browser::launch(config).await.expect("launch");
    let pump = tokio::spawn(async move { while handler.next().await.is_some() {} });

    let page = browser
        .new_page(format!("file://{}", path.display()))
        .await
        .unwrap();
    page.wait_for_navigation().await.unwrap();

    let expr = format!(
        "(() => {{ const el = document.querySelector('{selector}'); \
          return el ? getComputedStyle(el).getPropertyValue('{property}') : '<no-match>'; }})()"
    );
    let value: String = page.evaluate(expr).await.unwrap().into_value().unwrap();

    let mut browser = browser;
    browser.close().await.ok();
    pump.abort();

    Some(value.trim().to_string())
}

#[tokio::test]
#[serial_test::serial(browser)]
async fn code_block_background_computes() {
    if find_chrome().is_none() { return; } // skip-clean

    let fragment = render_my_html();             // crate under test
    let doc = format!(
        "<!doctype html><html><body style=\"background:#202020\">{fragment}</body></html>"
    );
    let bg = computed_style(&doc, ".code-block", "background-color").await.unwrap();

    assert_eq!(bg, "rgb(17, 27, 39)"); // resolved by the browser, not matched in source
}
```

## Screenshot inspection (dev/agent visual check)

The same stack produces a PNG you can open or view — no MCP/Node required. Keep
it as an `examples/` binary so it's runnable on demand without polluting the
test suite:

```rust
use chromiumoxide::page::ScreenshotParams;

page.save_screenshot(
    ScreenshotParams::builder().full_page(true).build(),
    "out.png",
).await?;
```

```bash
cargo run -p my_crate --example html_to_png -- input.md out.png
```

Prefer this screenshot-to-file path over a Playwright MCP server for one-shot
"render this and let me look" inspection: it adds no persistent tool surface and
no verbose accessibility-tree dumps to the context window. Reach for an
interactive browser MCP (or a Playwright agent skill) only when you need
multi-step navigation of a *live* app.

## Reuse / extraction

`find_chrome` and the fragment-wrapper are dependency-free on purpose. When a
second crate needs them, extract them into a shared `*-browser-harness` crate
(mirroring a terminal `*-test-harness`) with an `available()` probe, a
`skip_with_reason()` helper, a document wrapper, and `computed_style` /
`screenshot` helpers — then have each crate's tests depend on it.

## Gotchas

- **Hang on every call** → the handler future isn't being polled. Spawn the
  pump task immediately after `Browser::launch`.
- **`wait_for_navigation` never resolves** for `data:` URLs in some versions;
  prefer a temp `file://` URL.
- **`HtmlOptions`/config structs may be `#[non_exhaustive]`** in the crate under
  test — construct via `Default::default()` then mutate fields, not a struct
  literal.
- **CI**: provision a browser and flip skip-clean into hard-fail via an env flag
  (e.g. `MYCRATE_BROWSER_REQUIRED=1`) so coverage is enforced, not silently
  skipped.
