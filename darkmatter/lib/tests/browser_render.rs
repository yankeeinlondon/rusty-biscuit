//! Browser-level (real headless Chrome) tests for darkmatter HTML output.
//!
//! Where the Level-2 terminal tests drive a real WezTerm pane, these drive a
//! real headless Chromium via the shared `biscuit-browser-harness` so we
//! assert on the **computed** result of the CSS/HTML darkmatter emits — not on
//! HTML source substrings. Computed-style assertions are deterministic and
//! robust (unlike pixel diffs), and prove the browser actually applies our
//! styles.
//!
//! ## Skips cleanly
//!
//! These tests skip silently when no Chrome/Chromium binary is found. Set
//! `BISCUIT_BROWSER_REQUIRED=1` to turn a missing browser into a hard
//! failure (for CI that provisions one). Point `CHROME` at an explicit
//! executable to override discovery.

use biscuit_browser_harness::{BrowserHarness, ChromeHarness, require_browser, wrap_fragment};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::highlighting::{ColorMode, ThemePair};
use darkmatter::markdown::output::HtmlOptions;
use serial_test::serial;

/// The `.code-block` background darkmatter emits for a `github` + dark page must
/// compute, in a real browser, to the github-*light* panel color (`#ffffff`).
/// Code blocks invert their theme variant for page contrast (Defect D), so a
/// dark page gets a light code panel — matching the terminal renderer. This
/// proves the emitted CSS is valid and applied — not just present in the source.
#[tokio::test]
#[serial(browser)]
async fn browser_code_block_background_computes_in_browser() {
    if !require_browser() {
        return;
    }

    let md: Markdown = "```rust\nfn main() {}\n```\n".into();
    let mut options = HtmlOptions::default();
    options.code_theme = ThemePair::Github;
    options.prose_theme = ThemePair::Github;
    options.color_mode = ColorMode::Dark;
    let fragment = md.as_html(options).expect("as_html");
    let doc = wrap_fragment(&fragment, "#202020");

    let mut harness = ChromeHarness::new();
    harness.spawn().await.expect("spawn chrome");
    harness.render_html(&doc).await.expect("render html");

    let bg = harness
        .computed_style(".code-block", "background-color")
        .await
        .expect("computed style query");

    // Inverted for a dark page: github light panel = #ffffff = rgb(255, 255, 255).
    assert_eq!(
        bg, "rgb(255, 255, 255)",
        "browser-computed .code-block background-color should be the inverted (github-light) panel color",
    );
}
