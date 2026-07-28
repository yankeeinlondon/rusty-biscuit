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

// Whitebox: wires the deprecated `TerminalCodeRenderer` adapter directly to
// exercise the production code-rendering path the public entry points use.
#![cfg(target_os = "macos")]
#![allow(deprecated)]

use std::fs;
use std::rc::Rc;
use std::time::Duration;

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

/// Magenta (`#ff00ff`) does not occur in terminal chrome, text, or theme
/// backgrounds — its presence proves the image was decoded and painted, not
/// merely that the protocol bytes were consumed.
const MAGENTA: [u8; 3] = [255, 0, 255];

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
    // Allow the terminal to decode and paint the image payload.
    std::thread::sleep(Duration::from_millis(400));

    let Some(png) = harness
        .capture_window_png()
        .expect("screen-capture invocation")
    else {
        eprintln!(
            "skipping pixel assertion: window-region capture unavailable (non-macOS or screencapture failed)"
        );
        return;
    };

    let (magenta, non_black, total) = classify_pixels(&png, MAGENTA, 60);
    // A near-black capture means screen recording is blocked (no permission);
    // we cannot tell that from a paint failure, so skip rather than hard-fail.
    if non_black * 100 < total {
        eprintln!(
            "skipping pixel assertion: capture is essentially black ({non_black}/{total} non-black) \
             — Screen Recording permission likely not granted to the parent terminal"
        );
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

/// Encodes a `size`×`size` opaque PNG filled with a single RGB color.
fn write_solid_png(path: &std::path::Path, size: u32, rgb: [u8; 3]) {
    let img = image::RgbImage::from_pixel(size, size, image::Rgb(rgb));
    img.save_with_format(path, image::ImageFormat::Png)
        .expect("encode probe PNG");
}

/// Counts pixels in `png` that are near `target` RGB (per-channel within
/// `tol`), and the total pixels that are not near-black. Returns
/// `(near_target, non_black, total)`.
fn classify_pixels(png: &[u8], target: [u8; 3], tol: i32) -> (u64, u64, u64) {
    let img = image::load_from_memory(png).expect("decode screen capture");
    let rgb = img.to_rgb8();
    let mut near_target = 0u64;
    let mut non_black = 0u64;
    let total = (rgb.width() as u64) * (rgb.height() as u64);
    for px in rgb.pixels() {
        let [r, g, b] = px.0;
        let near = (r as i32 - target[0] as i32).abs() <= tol
            && (g as i32 - target[1] as i32).abs() <= tol
            && (b as i32 - target[2] as i32).abs() <= tol;
        if near {
            near_target += 1;
        }
        if r > 30 || g > 30 || b > 30 {
            non_black += 1;
        }
    }
    (near_target, non_black, total)
}
