//! Render a Markdown file's darkmatter HTML to a PNG via headless Chrome.
//!
//! This is the agent/dev visual-inspection path: render once, screenshot to a
//! file, then look at the PNG — no MCP server or Node tooling required, reusing
//! the same `chromiumoxide` stack as the browser tests.
//!
//! ```text
//! cargo run -p darkmatter --example html_to_png -- input.md out.png [dark|light]
//! ```
//!
//! Requires a Chrome/Chromium binary (set `CHROME=/path`, or have Google
//! Chrome / Playwright Chromium installed).

use std::path::PathBuf;

use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::page::ScreenshotParams;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::output::HtmlOptions;
use futures_util::StreamExt;

fn find_chrome() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("CHROME") {
        let p = PathBuf::from(p);
        if p.exists() {
            return Some(p);
        }
    }
    for c in [
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        "/Applications/Chromium.app/Contents/MacOS/Chromium",
        "/usr/bin/google-chrome",
        "/usr/bin/chromium",
    ] {
        let p = PathBuf::from(c);
        if p.exists() {
            return Some(p);
        }
    }
    None
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let input = args
        .get(1)
        .expect("usage: html_to_png <input.md> <out.png> [dark|light]");
    let out = args
        .get(2)
        .expect("usage: html_to_png <input.md> <out.png> [dark|light]");
    let mode = args.get(3).map(String::as_str).unwrap_or("dark");
    let (page_bg, color_mode) = match mode {
        "light" => (
            "#ffffff",
            darkmatter::markdown::highlighting::ColorMode::Light,
        ),
        _ => (
            "#202020",
            darkmatter::markdown::highlighting::ColorMode::Dark,
        ),
    };

    let chrome = find_chrome().expect("no Chrome/Chromium found (set CHROME=/path)");

    let md: Markdown = std::fs::read_to_string(input)?.into();
    let mut options = HtmlOptions::default();
    options.color_mode = color_mode;
    let fragment = md.as_html(options)?;
    let doc = format!(
        "<!doctype html><html><head><meta charset=\"utf-8\">\
         <style>body{{margin:0;padding:24px;background:{page_bg};\
         font-family:ui-monospace,SFMono-Regular,Menlo,monospace;}}</style></head>\
         <body>{fragment}</body></html>"
    );

    let dir = tempfile::tempdir()?;
    let html_path = dir.path().join("page.html");
    std::fs::write(&html_path, &doc)?;

    let config = BrowserConfig::builder()
        .chrome_executable(chrome)
        .arg("--no-sandbox")
        .window_size(900, 700)
        .build()?;
    let (browser, mut handler) = Browser::launch(config).await?;
    let handle = tokio::spawn(async move { while handler.next().await.is_some() {} });

    let page = browser
        .new_page(format!("file://{}", html_path.display()))
        .await?;
    page.wait_for_navigation().await?;
    page.save_screenshot(ScreenshotParams::builder().full_page(true).build(), out)
        .await?;

    let mut browser = browser;
    browser.close().await?;
    handle.abort();

    println!("wrote {out}");
    Ok(())
}
