# biscuit-browser-harness

Shared headless-browser test harness for the rusty-biscuit workspace. Wraps
[`chromiumoxide`] so multiple crates can drive a real headless Chrome or
Chromium from `cargo test` and assert on the **computed** result of the
HTML / CSS they emit.

Mirrors the role `biscuit-test-harness` plays for real-terminal tests: a
central crate that hides browser discovery and lifecycle plumbing,
exposes a small trait surface, and skip-cleans when no browser is
available.

## Trait Surface

```rust
#[async_trait]
pub trait BrowserHarness: Send {
    async fn spawn(&mut self) -> Result<(), BrowserError>;
    async fn render_html(&mut self, html: &str) -> Result<(), BrowserError>;
    async fn computed_style(&mut self, selector: &str, property: &str)
        -> Result<String, BrowserError>;
    async fn screenshot(&mut self, selector: Option<&str>)
        -> Result<Vec<u8>, BrowserError>;
}
```

The default [`ChromeHarness`] implementation:

- launches Chrome / Chromium in headless mode
- writes each `render_html` payload to a `tempfile::NamedTempFile` and
  navigates to `file://`
- queries computed styles via CDP `Runtime.evaluate` so results match
  what a real browser would paint
- captures element-bounded or full-viewport screenshots as PNG bytes
- tears the browser down on drop

## Skip-Clean Contract

Every test using this crate **must** check
[`ChromeHarness::available`] — or call the convenience helper
[`require_browser`] — before driving the harness, and early-return
cleanly when no browser is installed:

```rust
#[tokio::test]
#[serial_test::serial(browser)]
async fn browser_computed_style_matches() {
    if !biscuit_browser_harness::require_browser() { return; }

    let mut h = ChromeHarness::new();
    h.spawn().await.expect("spawn chrome");
    h.render_html(&wrap_fragment("<div class='x'>hi</div>", "#fff"))
        .await
        .expect("render");
    let bg = h.computed_style(".x", "background-color").await.expect("style");
    assert_eq!(bg, "rgb(17, 27, 39)");
}
```

`require_browser()` prints `skipping: requires Chrome/Chromium` to
stderr when no browser is found and returns `false`. Tests then return
`Ok` without exercising the harness.

## Environment

| Variable | Purpose |
|----------|---------|
| `CHROME` | Path to a specific Chrome / Chromium executable. Overrides discovery. |
| `BISCUIT_BROWSER_REQUIRED=1` | Convert a missing browser from a skip into a hard panic. CI jobs that provision Chrome should set this. |

`find_chrome()` searches the platform's usual install locations and the
Playwright cache (`~/Library/Caches/ms-playwright`, `~/.cache/ms-playwright`)
when `CHROME` is unset.

## Test Tier

Browser tests are part of the **browser** tier in the workspace's
testing taxonomy — they require an external resource (Chrome) and are
selected by the canonical `just test-browser` recipe. See
[`docs/testing-strategy.md`](../docs/testing-strategy.md) and the
[`rust-testing` skill](../.claude/skills/rust-testing/SKILL.md) for the
full tier rubric.

Name browser test functions with the `browser_` prefix so the nextest
filter `test(/browser_/)` selects them. The `_test_browser` shared
recipe in `just/devops.just` wires this into every consuming crate's
`test-browser` recipe.
