//! Genuine macOS OS-input evidence for prompted-link interactions.
//!
//! The Browser-tier tests use CDP input and remain headless. These opt-in
//! Level-3 tests instead launch one headed Chrome instance with an isolated
//! profile, activate its exact PID and nonce-titled window, and inject Quartz
//! keyboard/pointer events through `cliclick`.

#![cfg(target_os = "macos")]

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use biscuit_browser_harness::find_chrome;
use biscuit_test_harness::cliclick;
use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::Page;
use darkmatter::markdown::output::HtmlOptions;
use darkmatter::markdown::Markdown;
use futures_util::StreamExt;
use serial_test::serial;
use test_toolkit::{require_level, Level};

static WINDOW_SEQUENCE: AtomicU64 = AtomicU64::new(1);

const KEY_CANARY: &str = "dmcanary";
const PROVISIONING_FAILURE: &str = "Level-3 harness provisioning failure";

struct Level3Chrome {
    browser: Browser,
    page: Page,
    handler: tokio::task::JoinHandle<()>,
    _launch_process_id: u32,
    window_process_id: u32,
    title: String,
    _profile: tempfile::TempDir,
    _workdir: tempfile::TempDir,
}

impl Level3Chrome {
    async fn launch(html: &str, title: String) -> Result<Self, String> {
        let chrome = find_chrome().ok_or_else(|| "Chrome/Chromium not found".to_string())?;
        let profile = tempfile::tempdir().map_err(|error| error.to_string())?;
        let workdir = tempfile::tempdir().map_err(|error| error.to_string())?;
        let path = workdir.path().join("level3-popover.html");
        std::fs::write(&path, html).map_err(|error| error.to_string())?;

        let config = BrowserConfig::builder()
            .chrome_executable(chrome)
            .user_data_dir(profile.path())
            .with_head()
            .window_size(900, 700)
            .arg("--window-position=120,120")
            .arg("--no-first-run")
            .arg("--no-default-browser-check")
            .arg("--disable-session-crashed-bubble")
            .build()
            .map_err(|error| error.to_string())?;
        let (mut browser, mut browser_handler) = Browser::launch(config)
            .await
            .map_err(|error| error.to_string())?;
        let launch_process_id = browser
            .get_mut_child()
            .and_then(|child| child.as_mut_inner().id())
            .ok_or_else(|| "headed Chrome launch did not expose its process ID".to_string())?;
        let handler = tokio::spawn(async move {
            while browser_handler.next().await.is_some() {}
        });
        let page = browser
            .new_page(format!("file://{}", path.display()))
            .await
            .map_err(|error| error.to_string())?;
        page.wait_for_navigation()
            .await
            .map_err(|error| error.to_string())?;

        let actual_title: String = page
            .evaluate("document.title")
            .await
            .map_err(|error| error.to_string())?
            .into_value()
            .map_err(|error| error.to_string())?;
        if actual_title != title {
            return Err(format!(
                "headed Chrome loaded title {actual_title:?}, expected {title:?}"
            ));
        }
        let window_process_id = match resolve_window_process_id(&title).await {
            Ok(process_id) => process_id,
            Err(error) => {
                let _ = browser.close().await;
                let _ = browser.wait().await;
                handler.abort();
                let _ = handler.await;
                return Err(error);
            }
        };

        Ok(Self {
            browser,
            page,
            handler,
            _launch_process_id: launch_process_id,
            window_process_id,
            title,
            _profile: profile,
            _workdir: workdir,
        })
    }

    async fn evaluate(&self, script: &str) -> Result<String, String> {
        self.page
            .evaluate(script)
            .await
            .map_err(|error| error.to_string())?
            .into_value()
            .map_err(|error| error.to_string())
    }

    async fn element_screen_center(&self, selector: &str) -> Result<(i32, i32), String> {
        let script = format!(
            "(() => {{\
                const el = document.querySelector({selector:?});\
                if (!el) return '';\
                const r = el.getBoundingClientRect();\
                const contentX = window.screenX + (window.outerWidth - window.innerWidth) / 2;\
                const contentY = window.screenY + (window.outerHeight - window.innerHeight);\
                return `${{Math.round(contentX + r.left + r.width / 2)}},${{Math.round(contentY + r.top + r.height / 2)}}`;\
            }})()"
        );
        let coordinates = self.evaluate(&script).await?;
        coordinates
            .split_once(',')
            .and_then(|(x, y)| Some((x.parse().ok()?, y.parse().ok()?)))
            .ok_or_else(|| format!("selector {selector:?} had no screen-space center"))
    }

    fn activate_exact_window(&self) -> Result<(), String> {
        cliclick::activate_process_window(self.window_process_id, &self.title)
            .map_err(|error| format!("{PROVISIONING_FAILURE}: {error}"))
    }

    async fn verify_keyboard_canary(&self) -> Result<(i32, i32), String> {
        self.activate_exact_window()?;
        let coordinates = self.element_screen_center("#os-key-canary").await?;
        cliclick::click_then_text(coordinates.0, coordinates.1, KEY_CANARY)
            .map_err(|error| format!("{PROVISIONING_FAILURE}: keyboard injection failed: {error}"))?;
        tokio::time::sleep(Duration::from_millis(150)).await;
        let observed = self
            .evaluate(
                "(() => { const input = document.querySelector('#os-key-canary'); \
                 return `${input.value}|${window.__osKeyCanaryEvents || 0}`; })()",
            )
            .await?;
        let observed_canary = observed
            .split_once('|')
            .is_some_and(|(value, events)| {
                value == KEY_CANARY && events.parse::<u32>().unwrap_or(0) > 0
            });
        if !observed_canary {
            return Err(format!(
                "{PROVISIONING_FAILURE}: the live page did not observe the OS keyboard canary; \
                 pid={} title={:?} observed={observed:?}",
                self.window_process_id, self.title,
            ));
        }
        Ok(coordinates)
    }

    async fn verify_pointer_canary(&self, click_from: (i32, i32)) -> Result<(), String> {
        self.activate_exact_window()?;
        let target = self.element_screen_center("#os-pointer-canary").await?;
        cliclick::click_then_move(click_from.0, click_from.1, target.0, target.1)
            .map_err(|error| format!("{PROVISIONING_FAILURE}: pointer injection failed: {error}"))?;
        tokio::time::sleep(Duration::from_millis(150)).await;
        let observed = self
            .evaluate("String(window.__osPointerCanaryEvents || 0)")
            .await?;
        if observed.parse::<u32>().unwrap_or(0) == 0 {
            return Err(format!(
                "{PROVISIONING_FAILURE}: the live page did not observe the OS pointer canary; \
                 pid={} title={:?} observed={observed:?}",
                self.window_process_id, self.title,
            ));
        }
        Ok(())
    }

    async fn shutdown(mut self) {
        let _ = self.browser.close().await;
        let _ = self.browser.wait().await;
        self.handler.abort();
        let _ = self.handler.await;
    }
}

async fn resolve_window_process_id(title: &str) -> Result<u32, String> {
    let mut last_error = None;
    for _ in 0..20 {
        match cliclick::process_id_for_window(title) {
            Ok(process_id) => return Ok(process_id),
            Err(error) => last_error = Some(error),
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    Err(format!(
        "{PROVISIONING_FAILURE}: the headed Chrome page has no unique AX window: {}",
        last_error
            .map(|error| error.to_string())
            .unwrap_or_else(|| "window lookup produced no diagnostic".to_string()),
    ))
}

fn unique_window_title() -> String {
    format!(
        "Darkmatter Level 3 {} {}",
        std::process::id(),
        WINDOW_SEQUENCE.fetch_add(1, Ordering::Relaxed),
    )
}

fn popover_page(title: &str) -> String {
    let markdown: Markdown =
        "[Click](https://example.com \"prompt='Extra detail'\")\n".into();
    let fragment = markdown
        .as_html(HtmlOptions::default())
        .expect("render prompted link");
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>{title}</title></head>\
         <body style=\"margin:24px\">\
         <input id=\"os-key-canary\" aria-label=\"OS keyboard canary\" \
         style=\"position:absolute;left:24px;top:24px;width:180px;height:28px\">\
         <div id=\"os-pointer-canary\" style=\"position:absolute;left:240px;top:24px;\
         width:80px;height:28px;background:#ddd\"></div>\
         <main style=\"margin-top:96px\">{fragment}</main>\
         <script>\
         window.__osKeyCanaryEvents=0;window.__osPointerCanaryEvents=0;window.__nav='';\
         document.querySelector('#os-key-canary').addEventListener('input',()=>window.__osKeyCanaryEvents++);\
         document.querySelector('#os-pointer-canary').addEventListener('mousemove',()=>window.__osPointerCanaryEvents++);\
         document.querySelector('.dm-popover-wrapper a').addEventListener('click',(event)=>{{\
             event.preventDefault();window.__nav=event.currentTarget.href;\
         }});\
         </script></body></html>"
    )
}

fn level3_available() -> bool {
    find_chrome().is_some() && cliclick::available() && cliclick::accessibility_trusted()
}

async fn launch_level3_page() -> Result<Level3Chrome, String> {
    let title = unique_window_title();
    Level3Chrome::launch(&popover_page(&title), title).await
}

#[tokio::test]
#[serial(level3_browser)]
async fn level3_popover_tab_focuses_anchor_and_reveals_prompt() {
    require_level!(
        Level::L3,
        level3_available(),
        "headed Chrome + cliclick + macOS Accessibility",
    );

    let harness = launch_level3_page().await.expect("launch headed Chrome");
    let result = async {
        let canary = harness.verify_keyboard_canary().await?;
        harness.activate_exact_window()?;
        cliclick::click_then_keys(canary.0, canary.1, &["tab"])
            .map_err(|error| format!("OS Tab injection failed: {error}"))?;
        tokio::time::sleep(Duration::from_millis(150)).await;
        harness
            .evaluate(
                "(() => { const a=document.querySelector('.dm-popover-wrapper a'); \
                 const p=document.querySelector('.dm-popover-prompt'); \
                 return `active=${document.activeElement===a};vis=${getComputedStyle(p).visibility}`; })()",
            )
            .await
    }
    .await;
    harness.shutdown().await;
    assert_eq!(
        result.expect("Level-3 Tab interaction"),
        "active=true;vis=visible",
        "OS Tab must focus the prompted link and reveal its prompt",
    );
}

#[tokio::test]
#[serial(level3_browser)]
async fn level3_popover_enter_activates_link() {
    require_level!(
        Level::L3,
        level3_available(),
        "headed Chrome + cliclick + macOS Accessibility",
    );

    let harness = launch_level3_page().await.expect("launch headed Chrome");
    let result = async {
        let canary = harness.verify_keyboard_canary().await?;
        harness.activate_exact_window()?;
        cliclick::click_then_keys(canary.0, canary.1, &["tab", "return"])
            .map_err(|error| format!("OS Tab/Enter injection failed: {error}"))?;
        tokio::time::sleep(Duration::from_millis(150)).await;
        harness.evaluate("String(window.__nav || '')").await
    }
    .await;
    harness.shutdown().await;
    assert_eq!(
        result.expect("Level-3 Enter interaction"),
        "https://example.com/",
        "OS Enter must activate the prompted link's ordinary href",
    );
}

#[tokio::test]
#[serial(level3_browser)]
async fn level3_popover_pointer_hover_reveals_prompt() {
    require_level!(
        Level::L3,
        level3_available(),
        "headed Chrome + cliclick + macOS Accessibility",
    );

    let harness = launch_level3_page().await.expect("launch headed Chrome");
    let result = async {
        let keyboard_canary = harness.verify_keyboard_canary().await?;
        harness.verify_pointer_canary(keyboard_canary).await?;
        harness.activate_exact_window()?;
        let anchor = harness
            .element_screen_center(".dm-popover-wrapper a")
            .await?;
        cliclick::click_then_move(
            keyboard_canary.0,
            keyboard_canary.1,
            anchor.0,
            anchor.1,
        )
        .map_err(|error| format!("OS pointer hover injection failed: {error}"))?;
        tokio::time::sleep(Duration::from_millis(150)).await;
        harness
            .evaluate(
                "getComputedStyle(document.querySelector('.dm-popover-prompt')).visibility",
            )
            .await
    }
    .await;
    harness.shutdown().await;
    assert_eq!(
        result.expect("Level-3 pointer interaction"),
        "visible",
        "OS pointer hover must reveal the prompted link's prompt",
    );
}
