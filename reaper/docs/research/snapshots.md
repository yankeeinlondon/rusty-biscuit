---
prompt: |-
    When scraping websites we often want to create image "snapshots". This can be done quite easily using [`chromiumoxide`](https://crates.io/crates/chromiumoxide) and we do plan on using that in the Reaper package area but it might be able to be done via a lower cost HTTP based approach too.

    Your task is to do research and answer the following questions:

    - if you were to avoid not only chromiumoxide, puppeteer, playwrite, or one of the other headless browser solutions ... what options do you have in terms of creating a snapshot of a page? Is it possible? What tradeoffs are there?
    - Using [`chromiumoxide`](https://crates.io/crates/chromiumoxide):
        - how do you capture a web page's screen shot?
        - how can you capture at multiple viewport resolutions?
            - can this be done in parallel?
        - what besides viewport sizes might be worth creating variants from?
        - give 2-3 Rust code examples of using `chromiumoxide` to take screenshots of the page
    - What would be the best solution for capturing screenshots using **non** Chromium browsers? Playwright? Something else?
        - imagine that we use chomiumoxide for virtually everything else, what are the downsides of adding non-chromium browser support via these other solutions?
last_updated: 2026-06-03
---
## Summary

Accurate image snapshots of arbitrary websites require a rendering engine. You can avoid `chromiumoxide`, Puppeteer, Playwright, and other browser automation layers, but you cannot avoid the core problem: HTML, CSS, fonts, images, layout, media queries, JavaScript, canvas, SVG, shadow DOM, lazy loading, cookies, storage, and network timing all affect the final pixels.

There are lower-cost approaches, but they either render a narrower subset of the web platform, outsource browser rendering to an HTTP screenshot service, or create a synthetic approximation rather than a real screenshot.

## Non-Browser Snapshot Options

### 1. HTTP Fetch + Content Extraction + Template Rendering

Fetch the page with `reqwest`, parse the HTML, extract main content, then render a Reaper-owned template to PNG/SVG/PDF.

This is viable when the goal is a visual summary rather than a faithful browser screenshot.

**Good fit:**

- article previews
- documentation pages
- search/index cards
- "what changed" visual summaries
- pages where text and metadata matter more than exact layout

**Tradeoffs:**

- not a screenshot of the site
- misses client-rendered content unless the initial HTML contains it
- ignores site CSS, responsive layout, modals, ads, cookie banners, and personalization
- much cheaper and more deterministic than browser rendering

This approach pairs well with Reaper's extraction pipeline: canonical URL, title, metadata, main content, hero image, favicon, and structured data become the rendered snapshot inputs.

### 2. HTML/CSS Print Renderers

Tools such as [WeasyPrint](https://doc.courtbouillon.org/weasyprint/stable/) render HTML/CSS to PDF without using a full browser engine. WeasyPrint describes itself as a visual rendering engine for HTML and CSS that exports PDF. It is optimized for paged media, reports, invoices, and printable documents.

**Good fit:**

- controlled templates
- static HTML
- reports
- invoices
- documentation exports

**Tradeoffs:**

- not a general web screenshot tool
- no browser-grade JavaScript execution
- CSS support targets print-oriented layout, not every browser feature
- usually produces PDF first; PNG output requires an additional rasterization step

For scraped public websites, this is best treated as "render extracted content" rather than "capture the page."

### 3. `wkhtmltoimage` / Qt WebKit

[`wkhtmltoimage`](https://wkhtmltopdf.org/) renders HTML to image formats using Qt WebKit.

**Good fit:**

- simple legacy pages
- mostly static HTML
- environments where an old WebKit renderer is acceptable

**Tradeoffs:**

- still a browser-engine approach, just not Chromium
- WebKit version is old compared with modern Safari/WebKit
- poor fidelity for modern JavaScript-heavy sites
- can differ substantially from Chrome, Firefox, and Safari

This can be lower operational cost than full browser automation, but it is not a reliable proxy for the modern web.

### 4. SVG/Canvas/Image Generation From Extracted Data

Instead of rendering the page, generate a visual artifact directly with Rust libraries such as `resvg`, `image`, `tiny-skia`, `raqote`, or a higher-level internal rendering layer.

**Good fit:**

- Reaper-owned visual cards
- deterministic previews
- metadata thumbnails
- change summaries
- search-result snapshots

**Tradeoffs:**

- no page fidelity
- requires a designed snapshot format
- excellent for stable archives, weak for visual regression

This is likely the best "cheap snapshot" strategy if the product requirement is human-scannable archival evidence rather than exact page pixels.

### 5. Third-Party HTTP Screenshot APIs

Some services expose an HTTP API: send a URL, receive a PNG/JPEG/PDF.

**Good fit:**

- fast integration
- bursty workloads
- avoiding browser process management in Reaper

**Tradeoffs:**

- most services use headless Chrome, Playwright, or Puppeteer behind the API
- cost moves from CPU/RAM to vendor spend
- weaker control over cookies, auth, storage, proxying, blocking, request interception, and reproducibility
- privacy/compliance concerns if pages are private or authenticated
- vendor-specific failure modes and rate limits

This is HTTP-based from Reaper's point of view, but it is not browser-free.

### 6. Servo / Embeddable Engines

Servo is becoming interesting as an embeddable Rust browser engine, but it should be treated as experimental for production web screenshots unless Reaper can tolerate incomplete web-platform compatibility.

**Good fit:**

- Rust-native research
- controlled pages
- future-looking experimentation

**Tradeoffs:**

- not yet a drop-in replacement for Chromium/WebKit/Firefox fidelity
- compatibility risk is much higher than Playwright or Chromium CDP
- likely significant build and dependency complexity

## Practical Conclusion On Browser-Free Screenshots

For arbitrary websites, a true screenshot is only possible with a real rendering engine. A lower-cost HTTP-only pipeline can produce useful visual snapshots, but those snapshots should be named and modeled differently, for example:

- `BrowserScreenshot`: actual rendered page pixels
- `ContentSnapshot`: Reaper-rendered visual summary from extracted content
- `PrintSnapshot`: HTML/CSS paged rendering
- `ApiScreenshot`: vendor-rendered browser screenshot

That distinction matters because these artifacts answer different questions.

## Capturing Screenshots With `chromiumoxide`

`chromiumoxide` controls Chrome/Chromium through the Chrome DevTools Protocol. Its `Page` API exposes both `screenshot` and `save_screenshot`. The current docs show `Page::screenshot` returning `Vec<u8>` and `Page::save_screenshot` writing the image to disk while also returning the bytes. Screenshot options are configured with `ScreenshotParams`, including format, JPEG quality, clipping, full-page capture, and transparent background for PNG. See [`Page::screenshot` / `Page::save_screenshot`](https://docs.rs/chromiumoxide/latest/chromiumoxide/page/struct.Page.html) and [`CaptureScreenshotParams`](https://docs.rs/chromiumoxide/latest/chromiumoxide/cdp/browser_protocol/page/struct.CaptureScreenshotParams.html).

The browser handler stream must be driven while commands are running. The upstream README demonstrates launching the browser, spawning a task that polls the handler, then creating pages and navigating them.

## Example 1: Save A Full-Page PNG

```rust
use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotFormat;
use chromiumoxide::page::ScreenshotParams;
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (mut browser, mut handler) = Browser::launch(
        BrowserConfig::builder().build()?,
    )
    .await?;

    let handler_task = tokio::spawn(async move {
        while let Some(event) = handler.next().await {
            if event.is_err() {
                break;
            }
        }
    });

    let page = browser.new_page("https://example.com").await?;

    page.save_screenshot(
        ScreenshotParams::builder()
            .format(CaptureScreenshotFormat::Png)
            .full_page(true)
            .omit_background(false)
            .build(),
        "example-full-page.png",
    )
    .await?;

    browser.close().await?;
    handler_task.await?;

    Ok(())
}
```

## Example 2: Capture Bytes For Storage

```rust
use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotFormat;
use chromiumoxide::page::ScreenshotParams;
use futures::StreamExt;

async fn screenshot_bytes(url: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let (mut browser, mut handler) = Browser::launch(
        BrowserConfig::builder().build()?,
    )
    .await?;

    let handler_task = tokio::spawn(async move {
        while let Some(event) = handler.next().await {
            if event.is_err() {
                break;
            }
        }
    });

    let page = browser.new_page(url).await?;

    let png = page
        .screenshot(
            ScreenshotParams::builder()
                .format(CaptureScreenshotFormat::Png)
                .full_page(false)
                .build(),
        )
        .await?;

    browser.close().await?;
    handler_task.await?;

    Ok(png)
}
```

## Example 3: Capture Multiple Viewports

Viewport emulation is done through Chrome DevTools Protocol, typically with `Emulation.setDeviceMetricsOverride`. In chromiumoxide this is available as `SetDeviceMetricsOverrideParams`. The command takes width, height, device scale factor, and whether the viewport should be treated as mobile. The CDP docs exposed by chromiumoxide also support related fields such as screen size, orientation, and an optional visible-area viewport. See [`SetDeviceMetricsOverrideParams`](https://docs.rs/chromiumoxide_cdp/latest/chromiumoxide_cdp/cdp/browser_protocol/emulation/struct.SetDeviceMetricsOverrideParams.html).

```rust
use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::cdp::browser_protocol::emulation::SetDeviceMetricsOverrideParams;
use chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotFormat;
use chromiumoxide::page::{Page, ScreenshotParams};
use futures::StreamExt;

#[derive(Clone, Copy)]
struct Viewport {
    name: &'static str,
    width: i64,
    height: i64,
    device_scale_factor: f64,
    mobile: bool,
}

async fn capture_variant(
    page: Page,
    url: &str,
    viewport: Viewport,
) -> Result<(), Box<dyn std::error::Error>> {
    page.execute(SetDeviceMetricsOverrideParams::new(
        viewport.width,
        viewport.height,
        viewport.device_scale_factor,
        viewport.mobile,
    ))
    .await?;

    page.goto(url).await?;

    let output = format!("snapshot-{}.png", viewport.name);

    page.save_screenshot(
        ScreenshotParams::builder()
            .format(CaptureScreenshotFormat::Png)
            .full_page(true)
            .build(),
        output,
    )
    .await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = "https://example.com";

    let viewports = [
        Viewport {
            name: "desktop",
            width: 1440,
            height: 900,
            device_scale_factor: 1.0,
            mobile: false,
        },
        Viewport {
            name: "tablet",
            width: 834,
            height: 1112,
            device_scale_factor: 2.0,
            mobile: true,
        },
        Viewport {
            name: "mobile",
            width: 390,
            height: 844,
            device_scale_factor: 3.0,
            mobile: true,
        },
    ];

    let (mut browser, mut handler) = Browser::launch(
        BrowserConfig::builder().build()?,
    )
    .await?;

    let handler_task = tokio::spawn(async move {
        while let Some(event) = handler.next().await {
            if event.is_err() {
                break;
            }
        }
    });

    let mut pages = Vec::new();

    for _ in viewports {
        pages.push(browser.new_page("about:blank").await?);
    }

    let captures = pages
        .into_iter()
        .zip(viewports)
        .map(|(page, viewport)| capture_variant(page, url, viewport));

    futures::future::try_join_all(captures).await?;

    browser.close().await?;
    handler_task.await?;

    Ok(())
}
```

## Can Multiple Viewports Be Captured In Parallel?

Yes, but prefer parallel pages or browser contexts, not concurrent mutation of one page.

Safe patterns:

- one browser process, multiple pages/tabs, one viewport per page
- a bounded queue of screenshot jobs
- one browser process per worker when isolation matters
- one browser context per session profile if cookies/storage must differ

Avoid sharing one `Page` while changing viewport settings concurrently. Viewport, media emulation, cookies, local storage, and user agent are page/context state. If two jobs mutate the same page, snapshots will race.

A practical Reaper design would use bounded concurrency:

- small number of browser processes
- several pages per process
- per-domain rate limits
- timeout per navigation and per screenshot
- memory pressure monitoring
- retry with a fresh page before retrying with a fresh browser

## Screenshot Variants Beyond Viewport Size

Viewport is only one dimension. Useful variants include:

- device pixel ratio: `1.0`, `2.0`, `3.0`
- mobile emulation: desktop versus mobile
- orientation: portrait versus landscape
- full page versus viewport-only
- clipped element or main-content region
- light versus dark color scheme
- reduced motion versus normal motion
- print media versus screen media
- JavaScript enabled versus disabled, if supported by the capture path
- images enabled versus blocked
- web fonts enabled versus blocked
- locale and `Accept-Language`
- timezone
- geolocation
- authenticated versus anonymous session
- fresh visitor versus returning visitor cookies
- consent accepted versus rejected
- ad/content blocking enabled versus disabled
- wait strategy: load event, network idle, fixed settle delay, selector visible
- scroll position, especially top, middle, and after lazy-load triggering
- output format: PNG, JPEG, WebP if supported by the underlying protocol/version
- JPEG quality
- transparent background for PNG

For Reaper, the most valuable initial set is probably:

1. desktop full-page PNG
2. mobile full-page PNG
3. desktop viewport-only PNG
4. main-content or article-region clip, when a reliable content box exists

That keeps cost controlled while preserving the highest-signal visual differences.

## Non-Chromium Browser Screenshots

The best general solution for non-Chromium screenshots is Playwright.

Playwright officially supports Chromium, Firefox, and WebKit, and its screenshot API supports page screenshots, full-page screenshots, buffers, clips, and element screenshots. The browser documentation also notes that Playwright manages browser binaries and can install Chromium, Firefox, and WebKit builds. Sources: [Playwright browsers](https://playwright.dev/docs/browsers), [Playwright screenshots](https://playwright.dev/docs/screenshots).

For Rust, the options are less clean than for Node:

- use Playwright via a Node sidecar CLI/service
- use a Rust Playwright binding such as [`playwright`](https://docs.rs/playwright/latest/playwright/)
- shell out to a purpose-built Playwright screenshot tool
- use Selenium/WebDriver for Firefox/Safari/Edge coverage

Selenium/WebDriver is more universal across real installed browsers, but it is usually a worse fit for high-throughput screenshot capture than Playwright. It has more driver/version coordination, weaker ergonomics, and less consistent full-page screenshot behavior across browser families. It is useful when "real installed browser controlled through standard WebDriver" matters more than operational simplicity.

## Downsides Of Adding Playwright For Non-Chromium Support

If Reaper uses `chromiumoxide` for most browser work, adding Playwright only for non-Chromium support creates a second browser automation stack.

Main downsides:

- additional runtime dependency on Node.js or a Rust binding that wraps Playwright
- separate browser binary management
- larger install footprint; Playwright browser caches are hundreds of MB
- another update cadence tied to Playwright's pinned browser versions
- duplicated concepts: browser, context, page, viewport, cookies, permissions, route interception, timeouts
- different error types and failure behavior
- harder observability because CDP events and Playwright events do not map perfectly
- CI setup becomes heavier, especially on Linux where browser dependencies may need installation
- screenshots may differ because Playwright uses patched browser builds for Firefox/WebKit
- auth/session reuse between chromiumoxide and Playwright is not automatic
- proxy, TLS, locale, permissions, downloads, and storage configuration must be implemented twice
- more security surface from extra browser binaries and a Node toolchain, if Node is used
- more complicated support matrix for macOS/Linux/CI

The benefit is real cross-browser coverage. The cost is operational complexity.

## Recommendation

Use three tiers:

1. **Primary:** `chromiumoxide` for real browser screenshots in the normal Reaper pipeline.
2. **Cheap fallback / archive preview:** Reaper-rendered `ContentSnapshot` generated from fetched HTML, metadata, extracted main content, and selected images.
3. **Cross-browser validation:** Playwright for targeted Firefox/WebKit/Safari-like captures, not for every page by default.

That gives Reaper a low-cost visual artifact for broad crawling, a high-fidelity Chromium screenshot when exact pixels matter, and non-Chromium coverage when browser variance is specifically under investigation.
