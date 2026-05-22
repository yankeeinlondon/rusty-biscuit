//! Browser-level (real headless Chrome) tests for darkmatter HTML output.
//!
//! Where the Level-2 terminal tests drive a real WezTerm pane, these drive a
//! real headless Chromium via `chromiumoxide` (Chrome DevTools Protocol) so we
//! assert on the **computed** result of the CSS/HTML darkmatter emits — not on
//! HTML source substrings. Computed-style assertions are deterministic and
//! robust (unlike pixel diffs), and prove the browser actually applies our
//! styles.
//!
//! ## Skips cleanly
//!
//! These tests skip silently when no Chrome/Chromium binary is found. Set
//! `DARKMATTER_BROWSER_REQUIRED=1` to turn a missing browser into a hard
//! failure (for CI that provisions one). Point `CHROME` at an explicit
//! executable to override discovery.
//!
//! ## Reuse
//!
//! `find_chrome` / `wrap_fragment` are intentionally small and dependency-free
//! so they can move to a shared `biscuit-browser-harness` crate (mirroring
//! `biscuit-test-harness`) once a second consumer — e.g. biscuit-terminal's
//! `BrowserRenderable` components — needs them.

use std::path::PathBuf;

use chromiumoxide::browser::{Browser, BrowserConfig};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::highlighting::{ColorMode, ThemePair};
use darkmatter::markdown::output::HtmlOptions;
use futures_util::StreamExt;
use serial_test::serial;

/// Locate a usable Chrome/Chromium executable, or `None` to skip.
fn find_chrome() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("CHROME") {
        let p = PathBuf::from(p);
        if p.exists() {
            return Some(p);
        }
    }
    let candidates = [
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        "/Applications/Chromium.app/Contents/MacOS/Chromium",
        "/usr/bin/google-chrome",
        "/usr/bin/chromium",
        "/usr/bin/chromium-browser",
    ];
    for c in candidates {
        let p = PathBuf::from(c);
        if p.exists() {
            return Some(p);
        }
    }
    // Playwright-managed Chromium, if installed.
    if let Some(home) = std::env::var_os("HOME") {
        let base = PathBuf::from(home).join("Library/Caches/ms-playwright");
        if let Ok(entries) = std::fs::read_dir(&base) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                if name.to_string_lossy().starts_with("chromium-") {
                    let exe = entry
                        .path()
                        .join("chrome-mac/Chromium.app/Contents/MacOS/Chromium");
                    if exe.exists() {
                        return Some(exe);
                    }
                }
            }
        }
    }
    None
}

fn browser_required() -> bool {
    matches!(
        std::env::var("DARKMATTER_BROWSER_REQUIRED").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE")
    )
}

/// Wrap a darkmatter HTML *fragment* (which has no `<html>`/`<body>` and no
/// page background) into a standalone document with a body background, so the
/// page context exists for contrast/layout assertions.
fn wrap_fragment(fragment: &str, body_background: &str) -> String {
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"></head>\
         <body style=\"margin:0;background:{body_background};\">{fragment}</body></html>"
    )
}

/// Render markdown to a standalone HTML document and return the computed value
/// of `property` for the first element matching `selector`, as the browser
/// reports it. Returns `None` only when no browser is available (skip).
async fn computed_style(
    html_doc: &str,
    selector: &str,
    property: &str,
) -> Option<Result<String, String>> {
    let chrome = find_chrome()?;

    let dir = tempfile::tempdir().map_err(|e| e.to_string()).unwrap();
    let path = dir.path().join("page.html");
    std::fs::write(&path, html_doc).unwrap();

    let config = BrowserConfig::builder()
        .chrome_executable(chrome)
        .arg("--no-sandbox")
        .build()
        .expect("BrowserConfig");
    let (browser, mut handler) = Browser::launch(config)
        .await
        .map_err(|e| e.to_string())
        .expect("launch chrome");
    let handle = tokio::spawn(async move { while handler.next().await.is_some() {} });

    let result: Result<String, String> = async {
        let page = browser
            .new_page(format!("file://{}", path.display()))
            .await
            .map_err(|e| e.to_string())?;
        page.wait_for_navigation().await.map_err(|e| e.to_string())?;
        let expr = format!(
            "(() => {{ const el = document.querySelector('{selector}'); \
              return el ? getComputedStyle(el).getPropertyValue('{property}') : '<no-match>'; }})()"
        );
        let value: String = page
            .evaluate(expr)
            .await
            .map_err(|e| e.to_string())?
            .into_value()
            .map_err(|e| e.to_string())?;
        Ok(value.trim().to_string())
    }
    .await;

    let mut browser = browser;
    let _ = browser.close().await;
    handle.abort();

    Some(result)
}

/// The `.code-block` background darkmatter emits for a `github` + dark page must
/// compute, in a real browser, to the github-dark panel color (`#111b27`).
/// This proves the emitted CSS is valid and applied — not just present in the
/// source.
#[tokio::test]
#[serial(browser)]
async fn code_block_background_computes_in_browser() {
    if find_chrome().is_none() {
        if browser_required() {
            panic!("DARKMATTER_BROWSER_REQUIRED=1 but no Chrome/Chromium found");
        }
        eprintln!("skipping: no Chrome/Chromium (set CHROME=/path or install one)");
        return;
    }

    let md: Markdown = "```rust\nfn main() {}\n```\n".into();
    let mut options = HtmlOptions::default();
    options.code_theme = ThemePair::Github;
    options.prose_theme = ThemePair::Github;
    options.color_mode = ColorMode::Dark;
    let fragment = md.as_html(options).expect("as_html");
    let doc = wrap_fragment(&fragment, "#202020");

    let Some(result) = computed_style(&doc, ".code-block", "background-color").await else {
        eprintln!("skipping: no browser");
        return;
    };
    let bg = result.expect("computed style query failed");

    // github dark panel = #111b27 = rgb(17, 27, 39).
    assert_eq!(
        bg, "rgb(17, 27, 39)",
        "browser-computed .code-block background-color should be the github-dark panel color",
    );
}
