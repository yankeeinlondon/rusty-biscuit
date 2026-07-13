//! Shared headless-browser test harness for the rusty-biscuit monorepo.
//!
//! Provides a [`BrowserHarness`] trait and a default [`ChromeHarness`]
//! implementation that wraps [`chromiumoxide`] so multiple crates can
//! drive a real headless Chrome/Chromium from `cargo test` and assert
//! on the **computed** result of the CSS/HTML they emit.
//!
//! Mirrors the role `biscuit-test-harness` plays for real-terminal
//! tests: a central crate that hides the Chrome-discovery and lifecycle
//! plumbing, exposes a small trait surface, and skip-cleans when no
//! browser is available.
//!
//! ## Skip-clean contract
//!
//! Every test using this crate must check [`ChromeHarness::available`]
//! (or call [`require_browser`]) first and early-return cleanly when no
//! Chrome/Chromium is installed. Set the `BISCUIT_BROWSER_REQUIRED=1`
//! environment variable to convert a missing browser into a hard
//! failure — CI jobs that provision Chrome should set this so browser
//! coverage is actually enforced.
//!
//! Point the `CHROME` environment variable at a specific executable to
//! override discovery.

use std::path::PathBuf;

use async_trait::async_trait;
use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::cdp::browser_protocol::emulation::MediaFeature;
use chromiumoxide::cdp::browser_protocol::input::{
    DispatchKeyEventParams, DispatchKeyEventType, DispatchMouseEventParams, DispatchMouseEventType,
    MouseButton,
};
use futures_util::StreamExt;

/// Environment variable that, when set to `1`, converts a missing
/// browser from a skip into a panic.
pub const BISCUIT_BROWSER_REQUIRED: &str = "BISCUIT_BROWSER_REQUIRED";

/// Environment variable that points at a specific Chrome/Chromium
/// executable. When unset, [`find_chrome`] searches the platform's
/// usual install locations and Playwright's cache.
pub const CHROME_ENV: &str = "CHROME";

/// Errors returned by [`BrowserHarness`] operations.
#[derive(Debug, thiserror::Error)]
pub enum BrowserError {
    /// No Chrome/Chromium executable could be located.
    #[error("no Chrome/Chromium found (set CHROME=/path or install one)")]
    NoBrowser,
    /// The browser launched but a subsequent command failed.
    #[error("browser command failed: {0}")]
    Command(String),
    /// I/O failure (writing a temp file, etc.).
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
}

/// Common contract every headless-browser harness implements.
///
/// The contract is `async` because both `chromiumoxide` and the
/// underlying CDP protocol are async. Implementations are responsible
/// for launching the browser, navigating to a page, querying computed
/// styles, and tearing the session down via [`shutdown`](BrowserHarness::shutdown)
/// (with `Drop` as a best-effort fallback).
#[async_trait]
pub trait BrowserHarness: Send {
    /// Launch a headless browser instance.
    ///
    /// ## Errors
    ///
    /// Returns [`BrowserError::NoBrowser`] when no executable is found,
    /// or [`BrowserError::Command`] when the CDP launch fails.
    async fn spawn(&mut self) -> Result<(), BrowserError>;

    /// Render `html` as a standalone page and return the page handle.
    ///
    /// Writes the HTML to a temp file and navigates the browser to it.
    /// The returned object allows further interaction via the other
    /// trait methods.
    ///
    /// ## Errors
    ///
    /// Returns [`BrowserError::Command`] on CDP failure.
    async fn render_html(&mut self, html: &str) -> Result<(), BrowserError>;

    /// Return the computed value of `property` for the first element
    /// matching `selector`, as the browser reports it.
    ///
    /// ## Errors
    ///
    /// Returns [`BrowserError::Command`] on CDP failure. Returns
    /// `Ok("<no-match>")` when the selector does not match.
    async fn computed_style(
        &mut self,
        selector: &str,
        property: &str,
    ) -> Result<String, BrowserError>;

    /// Capture a screenshot of the element matching `selector` (or the
    /// full viewport if `selector` is `None`) as PNG bytes.
    ///
    /// ## Errors
    ///
    /// Returns [`BrowserError::Command`] on CDP failure.
    async fn screenshot(&mut self, selector: Option<&str>) -> Result<Vec<u8>, BrowserError>;

    /// Evaluate `script` in a freshly-navigated page and return its result
    /// as a `String`. The script must evaluate to a string value; wrap object
    /// results in `JSON.stringify` (or join the fields yourself).
    ///
    /// Unlike [`computed_style`](BrowserHarness::computed_style) — which opens a
    /// fresh page per call and so cannot observe state mutated by an earlier
    /// call — `evaluate` performs all interaction (querying, clicking,
    /// re-reading) inside a single page, so DOM toggles such as
    /// `<summary>.click()` are observable in the same evaluation.
    ///
    /// ## Errors
    ///
    /// Returns [`BrowserError::Command`] on CDP failure or when the result is
    /// not a string.
    async fn evaluate(&mut self, script: &str) -> Result<String, BrowserError>;

    /// Gracefully tear the browser session down, awaiting full process exit.
    ///
    /// [`Drop`] is best-effort and cannot `await`, so it leaves the spawned
    /// browser (and the OS handles it inherited from the test process) alive
    /// past the harness scope — which trips nextest's browser-leak timeout.
    /// Every browser test must call this before its Tokio runtime exits so the
    /// child process is closed, waited on, and its handler task reaped.
    ///
    /// Idempotent: a second call after teardown is a no-op.
    async fn shutdown(&mut self);
}

/// Locate a usable Chrome/Chromium executable, or `None` to skip.
///
/// Checks, in order:
///
/// 1. The `CHROME` environment variable, if set and pointing at an
///    existing file.
/// 2. Conventional install paths on macOS and Linux.
/// 3. Playwright's cache directory under `~/Library/Caches/ms-playwright`.
#[must_use]
pub fn find_chrome() -> Option<PathBuf> {
    if let Ok(p) = std::env::var(CHROME_ENV) {
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

/// Returns `true` when `BISCUIT_BROWSER_REQUIRED=1` is set in the
/// environment (also accepts `true`/`TRUE`).
#[must_use]
pub fn browser_required() -> bool {
    matches!(
        std::env::var(BISCUIT_BROWSER_REQUIRED).as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE")
    )
}

/// Decision returned by [`browser_decision`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrowserDecision {
    /// The test should run; a browser was located.
    Run,
    /// The test should skip cleanly. Carries a human-readable reason.
    Skip(String),
    /// The test should panic. Carries the panic message.
    Panic(String),
}

/// Compute the test gating decision given an availability flag.
///
/// Separated from [`require_browser`] so unit tests can exercise the
/// panic branch without uninstalling Chrome.
#[must_use]
pub fn browser_decision(available: bool) -> BrowserDecision {
    if available {
        return BrowserDecision::Run;
    }
    if browser_required() {
        return BrowserDecision::Panic(format!(
            "{BISCUIT_BROWSER_REQUIRED}=1 but no Chrome/Chromium found"
        ));
    }
    BrowserDecision::Skip(
        "skipping: no Chrome/Chromium (set CHROME=/path or install one)".to_string(),
    )
}

/// Gate a test on the availability of a browser.
///
/// Returns `true` when the test should continue (a browser was located),
/// and `false` after emitting a `skipping: ...` line when no browser
/// was found and `BISCUIT_BROWSER_REQUIRED` is not set.
///
/// ## Panics
///
/// Panics when no browser is found and `BISCUIT_BROWSER_REQUIRED=1`.
///
/// ## Examples
///
/// ```ignore
/// #[tokio::test]
/// async fn browser_renders_red_box() {
///     if !biscuit_browser_harness::require_browser() {
///         return;
///     }
///     let mut h = ChromeHarness::new();
///     h.spawn().await.unwrap();
///     // ...
/// }
/// ```
#[must_use]
pub fn require_browser() -> bool {
    match browser_decision(ChromeHarness::available()) {
        BrowserDecision::Run => true,
        BrowserDecision::Skip(msg) => {
            eprintln!("{msg}");
            false
        }
        BrowserDecision::Panic(msg) => panic!("{msg}"),
    }
}

/// Wrap an HTML *fragment* (no `<html>`/`<body>`) into a standalone
/// document with a configurable body background.
///
/// Useful when the system under test emits an HTML fragment but the
/// browser needs a full document context for layout/contrast checks.
#[must_use]
pub fn wrap_fragment(fragment: &str, body_background: &str) -> String {
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"></head>\
         <body style=\"margin:0;background:{body_background};\">{fragment}</body></html>"
    )
}

/// A single key press dispatched into the browser via CDP input.
///
/// Unlike a JS `element.focus()` / `element.click()` shortcut, a [`KeyStroke`]
/// is delivered via CDP `Input.dispatchKeyEvent`, so it exercises the browser's
/// own focus traversal and default-action handling — Browser-tier evidence.
/// This is NOT OS-level input injection: CDP injects synthetic events straight
/// into the renderer and never traverses the OS input path. Genuine OS
/// injection (Level 3, e.g. `cliclick` synthesizing real Quartz events) is a
/// separate mechanism. The three constants cover the interactions the
/// Browser-tier keyboard tests need.
#[derive(Debug, Clone, Copy)]
pub struct KeyStroke {
    /// The DOM `key` value (e.g. `"Tab"`, `"Enter"`).
    pub key: &'static str,
    /// The physical `code` value (e.g. `"Tab"`, `"Enter"`).
    pub code: &'static str,
    /// The Windows virtual key code Chromium expects for focus/default-action
    /// handling (Tab = 9, Enter = 13).
    pub windows_virtual_key_code: i64,
    /// Whether the Shift modifier is held (drives Shift+Tab reverse traversal).
    pub shift: bool,
}

impl KeyStroke {
    /// Tab — advances focus to the next focusable element.
    pub const TAB: KeyStroke = KeyStroke {
        key: "Tab",
        code: "Tab",
        windows_virtual_key_code: 9,
        shift: false,
    };
    /// Shift+Tab — moves focus to the previous focusable element.
    pub const SHIFT_TAB: KeyStroke = KeyStroke {
        key: "Tab",
        code: "Tab",
        windows_virtual_key_code: 9,
        shift: true,
    };
    /// Enter — activates the focused element (a link's default navigation).
    pub const ENTER: KeyStroke = KeyStroke {
        key: "Enter",
        code: "Enter",
        windows_virtual_key_code: 13,
        shift: false,
    };
}

/// One step in a [`ChromeHarness::drive`] input sequence.
///
/// Steps run in order against a single page so keyboard/pointer input and the
/// DOM reads that observe their effect share one live document — `evaluate`
/// cannot express this because it navigates a fresh page per call.
pub enum InputStep<'a> {
    /// Evaluate JS against the page. The result of the **last** `Eval` step is
    /// what [`ChromeHarness::drive`] returns.
    Eval(&'a str),
    /// Dispatch a real key press (keydown then keyup) via CDP input.
    Key(KeyStroke),
    /// Move the pointer over the center of the first element matching the
    /// selector (real CDP `mouseMoved`), so `:hover` engages.
    Hover(&'a str),
}

/// Default [`BrowserHarness`] backed by [`chromiumoxide`].
///
/// Owns a temporary directory that holds the rendered HTML file and a
/// running browser process. Drop runs `browser.close()` best-effort.
pub struct ChromeHarness {
    chrome_path: Option<PathBuf>,
    browser: Option<Browser>,
    handler_handle: Option<tokio::task::JoinHandle<()>>,
    workdir: Option<tempfile::TempDir>,
    /// Per-harness Chrome profile (`--user-data-dir`). Each spawn gets its
    /// own dir so concurrent test processes never collide on Chrome's
    /// `SingletonLock`. chromiumoxide otherwise defaults every instance to a
    /// single shared `$TMPDIR/chromiumoxide-runner`, which fails the second
    /// concurrent launch — and `#[serial]` cannot prevent that under
    /// nextest's process-per-test model. Held only so the dir is removed
    /// when the harness drops.
    #[allow(dead_code)]
    profile_dir: Option<tempfile::TempDir>,
    extra_args: Vec<String>,
}

impl ChromeHarness {
    /// Construct a new harness. The browser is not launched until
    /// [`spawn`](BrowserHarness::spawn) is called.
    #[must_use]
    pub fn new() -> Self {
        Self {
            chrome_path: None,
            browser: None,
            handler_handle: None,
            workdir: None,
            profile_dir: None,
            extra_args: vec!["--no-sandbox".to_string()],
        }
    }

    /// Override the Chrome/Chromium executable used for the next spawn.
    #[must_use]
    pub fn with_chrome_path(mut self, path: PathBuf) -> Self {
        self.chrome_path = Some(path);
        self
    }

    /// Append an extra command-line argument to the browser launch.
    #[must_use]
    pub fn with_arg(mut self, arg: impl Into<String>) -> Self {
        self.extra_args.push(arg.into());
        self
    }

    /// Returns `true` when a Chrome/Chromium executable can be located.
    #[must_use]
    pub fn available() -> bool {
        find_chrome().is_some()
    }

    async fn page(&self) -> Result<chromiumoxide::Page, BrowserError> {
        let browser = self
            .browser
            .as_ref()
            .ok_or_else(|| BrowserError::Command("browser not spawned".to_string()))?;
        let workdir = self
            .workdir
            .as_ref()
            .ok_or_else(|| BrowserError::Command("render_html not called yet".to_string()))?;
        let path = workdir.path().join("page.html");
        let page = browser
            .new_page(format!("file://{}", path.display()))
            .await
            .map_err(|e| BrowserError::Command(e.to_string()))?;
        page.wait_for_navigation()
            .await
            .map_err(|e| BrowserError::Command(e.to_string()))?;
        Ok(page)
    }

    /// Evaluate `script` in a freshly-navigated page after emulating the given
    /// CSS media features via CDP `Emulation.setEmulatedMedia`.
    ///
    /// Each `(name, value)` pair is a media feature the page's `@media` rules
    /// see — for example `("prefers-color-scheme", "dark")` or
    /// `("prefers-reduced-motion", "reduce")`. The emulation and the script run
    /// on the **same** page, which [`evaluate`](BrowserHarness::evaluate) cannot
    /// express because it navigates a fresh page per call. Passing an empty
    /// `features` slice is equivalent to `evaluate`.
    ///
    /// ## Errors
    ///
    /// Returns [`BrowserError::Command`] on CDP failure or when the result is
    /// not a string.
    pub async fn evaluate_with_media(
        &mut self,
        features: &[(&str, &str)],
        script: &str,
    ) -> Result<String, BrowserError> {
        let page = self.page().await?;
        if !features.is_empty() {
            let emulated = features
                .iter()
                .map(|(name, value)| MediaFeature::new(*name, *value))
                .collect();
            page.emulate_media_features(emulated)
                .await
                .map_err(|e| BrowserError::Command(e.to_string()))?;
        }
        let value: String = page
            .evaluate(script)
            .await
            .map_err(|e| BrowserError::Command(e.to_string()))?
            .into_value()
            .map_err(|e| BrowserError::Command(e.to_string()))?;
        Ok(value)
    }

    /// Run an ordered [`InputStep`] sequence against a single fresh page and
    /// return the last `Eval` step's string result (empty if none).
    ///
    /// This is the Browser-tier input driver: `Key`/`Hover` steps deliver key
    /// and pointer events via CDP input, so a test proves Tab focus traversal,
    /// pointer hover, and Enter default-action navigation through the browser's
    /// input pipeline exercised over CDP rather than JS `.focus()`/`.click()`
    /// shortcuts. CDP injects synthetic events into the renderer, so this is
    /// NOT OS-level input injection (Level 3); genuine OS injection is a
    /// separate mechanism (e.g. `cliclick`).
    ///
    /// ## Errors
    ///
    /// Returns [`BrowserError::Command`] on CDP failure or when an `Eval`
    /// result is not a string.
    pub async fn drive(&mut self, steps: &[InputStep<'_>]) -> Result<String, BrowserError> {
        let page = self.page().await?;
        let mut last = String::new();
        for step in steps {
            match step {
                InputStep::Eval(script) => {
                    last = page
                        .evaluate(*script)
                        .await
                        .map_err(|e| BrowserError::Command(e.to_string()))?
                        .into_value()
                        .map_err(|e| BrowserError::Command(e.to_string()))?;
                }
                InputStep::Key(stroke) => {
                    let modifiers = if stroke.shift { 8 } else { 0 };
                    for event_type in [DispatchKeyEventType::KeyDown, DispatchKeyEventType::KeyUp] {
                        let params = DispatchKeyEventParams::builder()
                            .r#type(event_type)
                            .key(stroke.key)
                            .code(stroke.code)
                            .windows_virtual_key_code(stroke.windows_virtual_key_code)
                            .modifiers(modifiers)
                            .build()
                            .map_err(BrowserError::Command)?;
                        page.execute(params)
                            .await
                            .map_err(|e| BrowserError::Command(e.to_string()))?;
                    }
                }
                InputStep::Hover(selector) => {
                    // Read the element's viewport-space center, then move the
                    // real pointer there so `:hover` matches.
                    let coords_js = format!(
                        "(() => {{ const el = document.querySelector('{selector}'); \
                         if (!el) return ''; const r = el.getBoundingClientRect(); \
                         return `${{r.left + r.width / 2}},${{r.top + r.height / 2}}`; }})()"
                    );
                    let coords: String = page
                        .evaluate(coords_js)
                        .await
                        .map_err(|e| BrowserError::Command(e.to_string()))?
                        .into_value()
                        .map_err(|e| BrowserError::Command(e.to_string()))?;
                    let (x, y) = coords
                        .split_once(',')
                        .and_then(|(x, y)| Some((x.parse::<f64>().ok()?, y.parse::<f64>().ok()?)))
                        .ok_or_else(|| {
                            BrowserError::Command(format!("hover selector had no box: {selector}"))
                        })?;
                    let params = DispatchMouseEventParams::builder()
                        .r#type(DispatchMouseEventType::MouseMoved)
                        .x(x)
                        .y(y)
                        .button(MouseButton::None)
                        .build()
                        .map_err(BrowserError::Command)?;
                    page.execute(params)
                        .await
                        .map_err(|e| BrowserError::Command(e.to_string()))?;
                }
            }
        }
        Ok(last)
    }
}

impl Default for ChromeHarness {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BrowserHarness for ChromeHarness {
    async fn spawn(&mut self) -> Result<(), BrowserError> {
        let chrome = match self.chrome_path.clone().or_else(find_chrome) {
            Some(p) => p,
            None => return Err(BrowserError::NoBrowser),
        };
        // A fresh per-harness profile dir keeps concurrent test processes off
        // chromiumoxide's shared default profile, whose `SingletonLock` fails
        // the second simultaneous launch.
        let profile = tempfile::tempdir()?;
        let mut builder = BrowserConfig::builder()
            .chrome_executable(chrome)
            .user_data_dir(profile.path());
        for arg in &self.extra_args {
            builder = builder.arg(arg);
        }
        let config = builder
            .build()
            .map_err(|e| BrowserError::Command(e.to_string()))?;
        self.profile_dir = Some(profile);
        let (browser, mut handler) = Browser::launch(config)
            .await
            .map_err(|e| BrowserError::Command(e.to_string()))?;
        let handle = tokio::spawn(async move { while handler.next().await.is_some() {} });
        self.browser = Some(browser);
        self.handler_handle = Some(handle);
        Ok(())
    }

    async fn render_html(&mut self, html: &str) -> Result<(), BrowserError> {
        if self.browser.is_none() {
            self.spawn().await?;
        }
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("page.html");
        std::fs::write(&path, html)?;
        self.workdir = Some(dir);
        Ok(())
    }

    async fn computed_style(
        &mut self,
        selector: &str,
        property: &str,
    ) -> Result<String, BrowserError> {
        let page = self.page().await?;
        let expr = format!(
            "(() => {{ const el = document.querySelector('{selector}'); \
              return el ? getComputedStyle(el).getPropertyValue('{property}') : '<no-match>'; }})()"
        );
        let value: String = page
            .evaluate(expr)
            .await
            .map_err(|e| BrowserError::Command(e.to_string()))?
            .into_value()
            .map_err(|e| BrowserError::Command(e.to_string()))?;
        Ok(value.trim().to_string())
    }

    async fn evaluate(&mut self, script: &str) -> Result<String, BrowserError> {
        let page = self.page().await?;
        let value: String = page
            .evaluate(script)
            .await
            .map_err(|e| BrowserError::Command(e.to_string()))?
            .into_value()
            .map_err(|e| BrowserError::Command(e.to_string()))?;
        Ok(value)
    }

    async fn screenshot(&mut self, selector: Option<&str>) -> Result<Vec<u8>, BrowserError> {
        use chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotFormat;
        use chromiumoxide::page::ScreenshotParams;

        let page = self.page().await?;
        let bytes = match selector {
            Some(sel) => {
                let element = page
                    .find_element(sel)
                    .await
                    .map_err(|e| BrowserError::Command(e.to_string()))?;
                element
                    .screenshot(CaptureScreenshotFormat::Png)
                    .await
                    .map_err(|e| BrowserError::Command(e.to_string()))?
            }
            None => page
                .screenshot(ScreenshotParams::builder().build())
                .await
                .map_err(|e| BrowserError::Command(e.to_string()))?,
        };
        Ok(bytes)
    }

    async fn shutdown(&mut self) {
        // Close the browser while its handler task is still draining CDP
        // messages, then `wait` for the chromium process to exit completely so
        // the OS handles it inherited from this test process are released. Drop
        // can do neither (it cannot await), which is what leaks handles past
        // the browser-leak timeout.
        if let Some(mut browser) = self.browser.take() {
            let _ = browser.close().await;
            let _ = browser.wait().await;
        }
        // With the connection closed the handler loop ends on its own; `abort`
        // guards a stuck stream, and awaiting reaps the task — and the handles
        // it held — before the runtime exits.
        if let Some(handle) = self.handler_handle.take() {
            handle.abort();
            let _ = handle.await;
        }
    }
}

impl Drop for ChromeHarness {
    fn drop(&mut self) {
        // Tests should call [`shutdown`](BrowserHarness::shutdown) for a clean,
        // awaited teardown; this is the best-effort fallback for the panic path
        // (and any caller that forgot). Drop cannot await, so it can only abort
        // the handler and fire-and-forget the close.
        if let Some(handle) = self.handler_handle.take() {
            handle.abort();
        }
        if let Some(mut browser) = self.browser.take()
            && let Ok(handle) = tokio::runtime::Handle::try_current()
        {
            handle.spawn(async move {
                let _ = browser.close().await;
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_toolkit::EnvGuard;

    #[test]
    fn keystroke_constants_carry_expected_codes() {
        // Bind through locals so clippy does not fold the const reads into
        // `assertions_on_constants`.
        let (tab, shift_tab, enter) = (KeyStroke::TAB, KeyStroke::SHIFT_TAB, KeyStroke::ENTER);
        assert_eq!(tab.windows_virtual_key_code, 9);
        assert!(!tab.shift, "Tab holds no modifier");
        assert!(shift_tab.shift, "Shift+Tab holds the Shift modifier");
        assert_eq!(shift_tab.key, "Tab");
        assert_eq!(enter.windows_virtual_key_code, 13);
        assert_eq!(enter.key, "Enter");
    }

    #[test]
    fn wrap_fragment_adds_body_background() {
        let out = wrap_fragment("<p>x</p>", "#202020");
        assert!(out.contains("background:#202020"));
        assert!(out.contains("<p>x</p>"));
        assert!(out.starts_with("<!doctype html>"));
    }

    #[test]
    #[serial_test::serial]
    fn browser_decision_runs_when_available() {
        // Even with BISCUIT_BROWSER_REQUIRED set, available=true → Run.
        let _g = EnvGuard::set_safe(BISCUIT_BROWSER_REQUIRED, "1");
        assert_eq!(browser_decision(true), BrowserDecision::Run);
    }

    #[test]
    #[serial_test::serial]
    fn browser_decision_panics_when_unavailable_and_required() {
        let _g = EnvGuard::set_safe(BISCUIT_BROWSER_REQUIRED, "1");
        match browser_decision(false) {
            BrowserDecision::Panic(msg) => {
                assert!(msg.contains(BISCUIT_BROWSER_REQUIRED));
                assert!(msg.contains("Chrome"));
            }
            other => panic!("expected Panic, got {other:?}"),
        }
    }

    #[test]
    #[serial_test::serial]
    fn browser_decision_skips_when_unavailable_and_not_required() {
        let _g = EnvGuard::remove_safe(BISCUIT_BROWSER_REQUIRED);
        match browser_decision(false) {
            BrowserDecision::Skip(msg) => assert!(msg.contains("Chrome")),
            other => panic!("expected Skip, got {other:?}"),
        }
    }

    #[test]
    #[serial_test::serial]
    fn browser_required_reads_env() {
        let g_on = EnvGuard::set_safe(BISCUIT_BROWSER_REQUIRED, "1");
        assert!(browser_required());
        drop(g_on);

        let _g_off = EnvGuard::set_safe(BISCUIT_BROWSER_REQUIRED, "0");
        assert!(!browser_required());
    }
}
