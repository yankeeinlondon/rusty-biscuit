//! Level-3 verification that inline image protocol bytes actually paint.
//!
//! Pixel-readback is an OS-level operation: `capture_window_png` raises the
//! WezTerm window to the foreground and calls `screencapture` to sample the
//! rendered pixels. That focus-stealing behavior is reserved for L3 by the
//! `biscuit-test-harness` contract, so this test lives outside the L2 suite
//! and is gated behind `RUN_LEVEL3=1`. The L2 suite never steals focus.
//!
//! ## Companion coverage
//!
//! L2 (`level2_render_tree_terminal/images.rs::
//! level2_tree_rich_image_node_emits_protocol_and_renders_in_real_terminal`)
//! verifies the iTerm2 image protocol bytes are emitted and consumed without
//! surfacing the alt-text fallback. This L3 test goes one step further by
//! sampling the painted pixels themselves — a dropped or malformed payload
//! would pass the L2 byte assertion but fail here.
//!
//! ## Skips cleanly
//!
//! Skips when WezTerm is unavailable, when window-region capture is
//! unavailable (off macOS, or `screencapture` failed), or when the capture
//! comes back essentially black — the signature of missing Screen Recording
//! permission, which cannot be distinguished from a genuine paint failure.
//! Every one of those exits is a `return`, which libtest reports as a pass, so
//! all three honor `BISCUIT_TEST_LEVEL_REQUIRED=3` and panic instead when an
//! operator has demanded genuine Level-3 evidence.
//!
//! ## Why the fixed sleep became a poll
//!
//! The capture used to be taken once, 400 ms after `cat`. That number was a
//! guess at how long WezTerm needs to decode and paint an iTerm2 payload, and
//! it failed in three ways. Too short on a loaded host, so a correct build
//! failed spuriously. Silent about *when* the pixels appeared, so it could
//! never distinguish "painted in response to the payload" from "already
//! there". And a single transient `screencapture` failure — one `None` — was
//! enough to downgrade the whole pixel assertion into a skip, which reports as
//! a pass.
//!
//! The loop's exit test is the conjunction of the two conditions below it: the
//! `magenta > 1000` assertion, and the non-black guard that decides whether the
//! assertion is meaningful at all. It therefore cannot stop on a
//! partially-decoded frame, and the counts it carries out are the counts that
//! satisfied it. Only an all-`None` five seconds now yields the
//! capture-unavailable skip. Spec §4 of
//! `fixes/2026-09-07-faster-darkmatter-tests` mandates this shape: "Poll the
//! final asserted condition, retain real-boundary coverage".
//!
//! ## What the 5-second bound costs
//!
//! `capture_window_png` raises the window and lets the compositor settle before
//! sampling, so an iteration costs roughly 350 ms whatever the 50 ms sleep
//! says; the bound buys about a dozen attempts, each re-raising the window.
//! And it is a real ceiling: a paint regression slower than 400 ms but faster
//! than five seconds now passes where the old sleep would have failed. That is
//! accepted — the claim here is that the decoded image is painted at all, and
//! no paint-latency floor is asserted at any tier.

// Whitebox: wires the deprecated `TerminalCodeRenderer` adapter directly to
// exercise the production code-rendering path the public entry points use.
#![cfg(target_os = "macos")]
#![allow(deprecated)]

#[path = "image_test_support/mod.rs"]
mod image_test_support;

use std::fs;
use std::rc::Rc;
use std::time::{Duration, Instant};

use biscuit_terminal::discovery::detection::ImageSupport;
use biscuit_terminal::render_tree::{
    TerminalRenderContext, TerminalRenderOptions, render_terminal_document,
};
use biscuit_terminal::terminal::Terminal;
use biscuit_test_harness::SpawnVisibility;
use biscuit_test_harness::TerminalHarness;
use biscuit_test_harness::wezterm::WezTermHarness;
use darkmatter::markdown::render_tree::{TerminalCodeRenderer, fold_markdown_to_document};
use renderable::tree::{GraphicsMode, RenderStrictness, SourceDescriptor};
use serial_test::serial;
use tempfile::tempdir;
use test_toolkit::{Backend, Level, require_level};

use image_test_support::{classify_pixels, write_solid_png};

/// Magenta (`#ff00ff`) does not occur in terminal chrome, text, or theme
/// backgrounds — its presence proves the image was decoded and painted, not
/// merely that the protocol bytes were consumed.
const MAGENTA: [u8; 3] = [255, 0, 255];

/// Abandons the pixel claim because the resource it needs is missing, panicking
/// instead when `BISCUIT_TEST_LEVEL_REQUIRED=3` demands Level 3.
///
/// ## Notes
///
/// `require_level!` applies that enforcement only to the gate at the top of the
/// test. Both of this test's other exits are decided mid-body from the capture
/// itself, and a bare `return` is reported green by libtest — so without the
/// same check here, an operator's authorized Level-3 run on a host that never
/// granted Screen Recording would file a pass for a claim it never evaluated.
fn skip_pixel_assertion(reason: &str) {
    let required = std::env::var(test_toolkit::BISCUIT_TEST_LEVEL_REQUIRED)
        .ok()
        .and_then(|value| value.trim().parse::<u8>().ok());
    assert!(
        required != Some(Level::L3.as_u8()),
        "{}=3 but the pixel assertion cannot be evaluated: {reason}",
        test_toolkit::BISCUIT_TEST_LEVEL_REQUIRED,
    );
    eprintln!("skipping pixel assertion: {reason}");
}

#[test]
#[serial(level3_terminal)]
fn level3_rich_image_node_paints_distinctive_pixels() {
    require_level!(Level::L3, WezTermHarness::available(), Backend::WezTerm);

    let dir = tempdir().unwrap();
    let png_path = dir.path().join("probe.png");
    write_solid_png(&png_path, 240, MAGENTA);

    let source = SourceDescriptor::Virtual {
        name: "rich_image_pixels".into(),
    };
    let (doc, diags) = fold_markdown_to_document(source, "![probe](probe.png)\n");
    assert!(
        diags.is_empty(),
        "image fixture must fold cleanly: {diags:?}"
    );

    // Rich tier on an iTerm2-capable TTY (WezTerm renders iTerm2 graphics).
    let mut term = Terminal::new_optimistic(120);
    term.image_support = ImageSupport::ITerm;
    term.is_tty = true;
    let mut context = TerminalRenderContext::from_terminal(&term);
    context.graphics_mode = GraphicsMode::Rich;
    context.image_base_path = Some(dir.path().to_path_buf());
    let opts = TerminalRenderOptions {
        context,
        strictness: RenderStrictness::Warn,
        code_renderer: Some(Rc::new(TerminalCodeRenderer::new())),
    };
    let rendered = render_terminal_document(&doc, &opts).expect("tree terminal render");
    assert!(
        rendered.output.contains("\u{1b}]1337;File="),
        "Rich image node must emit the iTerm2 image protocol",
    );

    let ansi_path = dir.path().join("rich_image_pixels.ansi");
    fs::write(&ansi_path, rendered.output).unwrap();

    // L3 owns its own Foreground WezTerm pane. The harness contract reserves
    // Foreground spawn for tests that call `capture_window_png` /
    // `focus_spawned_pane`: AXRaise can only reach a window that is already on
    // the active workspace, and a Background (`biscuit-bg`) spawn would leave
    // the pane on a different workspace where the raise step cannot find it.
    let mut harness = WezTermHarness::new().with_spawn_visibility(SpawnVisibility::Foreground);
    harness.spawn_shell().expect("spawn WezTerm shell");

    // Drive the pane: clear, then `cat` the ANSI bytes so the inline image is
    // painted into the real terminal grid.
    harness.send_text(b"clear\n").expect("clear failed");
    harness.settle();
    harness
        .send_text(format!("cat {}\n", ansi_path.display()).as_bytes())
        .expect("send_text failed");
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut last_counts = None;
    while Instant::now() < deadline {
        let Some(png) = harness
            .capture_window_png()
            .expect("screen-capture invocation")
        else {
            std::thread::sleep(Duration::from_millis(50));
            continue;
        };
        let counts = classify_pixels(&png, MAGENTA, 60);
        last_counts = Some(counts);
        if counts.0 > 1000 && counts.1 * 100 >= counts.2 {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    let Some((magenta, non_black, total)) = last_counts else {
        skip_pixel_assertion(
            "window-region capture unavailable (non-macOS or screencapture failed)",
        );
        return;
    };
    // A near-black capture means screen recording is blocked (no permission);
    // we cannot tell that from a paint failure, so skip rather than hard-fail.
    if non_black * 100 < total {
        skip_pixel_assertion(&format!(
            "capture is essentially black ({non_black}/{total} non-black) — Screen Recording \
             permission likely not granted to the parent terminal"
        ));
        return;
    }

    // A real, non-black capture with no magenta means the image did not paint.
    assert!(
        magenta > 1000,
        "Rich image node did not paint: only {magenta} magenta pixels in a {total}-pixel \
         capture ({non_black} non-black). The image protocol bytes were emitted but the \
         terminal did not render the decoded image.",
    );
}
