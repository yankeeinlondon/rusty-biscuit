//! Discovery probe — example binary for Level-1 PTY tests.
//!
//! Called inside a pseudoterminal by the test suite.  Accepts environment
//! variables that control which discovery routines are exercised and prints
//! machine-readable `key=value` lines.
//!
//! ## Environment variables
//!
//! | Variable | Purpose |
//! |----------|---------|
//! | `PROBE` | Selects the probe mode (see below). Defaults to `all`. |
//! | `PROBE_TERM_PROGRAM` | Overrides `TERM_PROGRAM` for the duration of the probe. |
//! | `PROBE_TERM` | Overrides `TERM` for the duration of the probe. |
//!
//! ## Probe modes
//!
//! * `all` — Run every discovery routine and print all results.
//! * `osc10` — Query foreground colour (OSC 10).
//! * `osc11` — Query background colour (OSC 11).
//! * `osc12` — Query cursor colour (OSC 12).
//! * `osc10_timeout` — Query foreground colour with custom timeout.
//! * `osc11_timeout` — Query background colour with custom timeout.
//! * `osc12_timeout` — Query cursor colour with custom timeout.
//! * `clipboard` — Attempt OSC52 clipboard write.
//! * `clipboard_support` — Check OSC52 support.
//! * `clipboard_target` — Attempt OSC52 clipboard write with target.
//! * `clipboard_clear` — Attempt OSC52 clipboard clear.
//! * `clipboard_get` — Attempt OSC52 clipboard read.
//! * `mode2027` — Enable/disable Mode 2027.
//! * `mode_2027_support` — Check Mode 2027 support.
//! * `cursor` — Query cursor position (DSR).
//! * `cursor_timeout` — Query cursor position with custom timeout.
//! * `terminal` — Build a `Terminal` instance and print its fields.
//!
//! ## Output format
//!
//! Every line is `key=value` where `value` is the `Debug` representation of
//! the result.  `Option::None` is printed as `None` so tests can assert with
//! simple string containment.
//!
//! ```text
//! bg_color=Some(RgbValue { r: 128, g: 128, b: 128 })
//! bg_color=None
//! ```

fn main() {
    // Apply env overrides so the library sees the terminal we want.
    if let Ok(v) = std::env::var("PROBE_TERM_PROGRAM") {
        // SAFETY: this is a single-threaded example binary; no other
        // threads are reading environment variables concurrently.
        unsafe { std::env::set_var("TERM_PROGRAM", v) };
    }
    if let Ok(v) = std::env::var("PROBE_TERM") {
        // SAFETY: single-threaded example binary.
        unsafe { std::env::set_var("TERM", v) };
    }

    let mode = std::env::var("PROBE").unwrap_or_else(|_| "all".to_string());

    match mode.as_str() {
        "osc10" => probe_osc10(),
        "osc11" => probe_osc11(),
        "osc12" => probe_osc12(),
        "osc10_timeout" => probe_osc10_timeout(),
        "osc11_timeout" => probe_osc11_timeout(),
        "osc12_timeout" => probe_osc12_timeout(),
        "clipboard" => probe_clipboard(),
        "clipboard_support" => probe_clipboard_support(),
        "clipboard_target" => probe_clipboard_target(),
        "clipboard_clear" => probe_clipboard_clear(),
        "clipboard_get" => probe_clipboard_get(),
        "mode2027" => probe_mode_2027(),
        "mode_2027_support" => probe_mode_2027_support(),
        "cursor" => probe_cursor(),
        "cursor_timeout" => probe_cursor_timeout(),
        "terminal" => probe_terminal(),
        _ => probe_all(),
    }
}

// ---------------------------------------------------------------------------
// OSC colour queries
// ---------------------------------------------------------------------------

fn probe_osc10() {
    use biscuit_terminal::discovery::osc_queries::text_color;
    println!("text_color={:?}", text_color());
}

fn probe_osc11() {
    use biscuit_terminal::discovery::osc_queries::bg_color;
    println!("bg_color={:?}", bg_color());
}

fn probe_osc12() {
    use biscuit_terminal::discovery::osc_queries::cursor_color;
    println!("cursor_color={:?}", cursor_color());
}

fn probe_osc10_timeout() {
    use biscuit_terminal::discovery::osc_queries::text_color_with_timeout;
    use std::time::Duration;
    println!(
        "osc10_timeout={:?}",
        text_color_with_timeout(Duration::from_millis(250))
    );
}

fn probe_osc11_timeout() {
    use biscuit_terminal::discovery::osc_queries::bg_color_with_timeout;
    use std::time::Duration;
    println!(
        "osc11_timeout={:?}",
        bg_color_with_timeout(Duration::from_millis(250))
    );
}

fn probe_osc12_timeout() {
    use biscuit_terminal::discovery::osc_queries::cursor_color_with_timeout;
    use std::time::Duration;
    println!(
        "osc12_timeout={:?}",
        cursor_color_with_timeout(Duration::from_millis(250))
    );
}

// ---------------------------------------------------------------------------
// Clipboard
// ---------------------------------------------------------------------------

fn probe_clipboard() {
    use biscuit_terminal::discovery::clipboard::set_clipboard;
    let result = set_clipboard("hello-pty");
    println!(
        "clipboard_result={:?}",
        result.map(|_| "ok").map_err(|e| e.to_string())
    );
}

fn probe_clipboard_support() {
    use biscuit_terminal::discovery::clipboard::osc52_support;
    println!("osc52_support={}", osc52_support());
}

fn probe_clipboard_target() {
    use biscuit_terminal::discovery::clipboard::{ClipboardTarget, set_clipboard_with_target};
    let result = set_clipboard_with_target("primary-pty", ClipboardTarget::Primary);
    println!(
        "clipboard_target_result={:?}",
        result.map(|_| "ok").map_err(|e| e.to_string())
    );
}

fn probe_clipboard_clear() {
    use biscuit_terminal::discovery::clipboard::clear_clipboard;
    let result = clear_clipboard();
    println!(
        "clipboard_clear_result={:?}",
        result.map(|_| "ok").map_err(|e| e.to_string())
    );
}

fn probe_clipboard_get() {
    use biscuit_terminal::discovery::clipboard::get_clipboard;
    println!("clipboard_get={:?}", get_clipboard());
}

// ---------------------------------------------------------------------------
// Mode 2027
// ---------------------------------------------------------------------------

fn probe_mode_2027() {
    use biscuit_terminal::discovery::mode_2027::{disable_mode_2027, enable_mode_2027};
    let en = enable_mode_2027();
    let dis = disable_mode_2027();
    println!(
        "enable_mode_2027={:?}",
        en.map(|_| "ok").map_err(|e| e.to_string())
    );
    println!(
        "disable_mode_2027={:?}",
        dis.map(|_| "ok").map_err(|e| e.to_string())
    );
}

fn probe_mode_2027_support() {
    use biscuit_terminal::discovery::mode_2027::supports_mode_2027;
    println!("supports_mode_2027={}", supports_mode_2027());
}

// ---------------------------------------------------------------------------
// Cursor position
// ---------------------------------------------------------------------------

fn probe_cursor() {
    use biscuit_terminal::discovery::cursor_position::cursor_position;
    println!("cursor_position={:?}", cursor_position());
}

fn probe_cursor_timeout() {
    use biscuit_terminal::discovery::cursor_position::cursor_position_with_timeout;
    use std::time::Duration;
    println!(
        "cursor_timeout={:?}",
        cursor_position_with_timeout(Duration::from_millis(250))
    );
}

// ---------------------------------------------------------------------------
// Terminal init
// ---------------------------------------------------------------------------

fn probe_terminal() {
    use biscuit_terminal::terminal::Terminal;
    let term = Terminal::new();
    println!("is_tty={}", term.is_tty);
    println!("app={:?}", term.app);
    println!("color_depth={:?}", term.color_depth);
    println!("supports_italic={}", term.supports_italic);
    println!("image_support={:?}", term.image_support);
    println!("underline_support={:?}", term.underline_support);
    println!("osc_link_support={}", term.osc_link_support);
    println!("is_ci={}", term.is_ci);
}

// ---------------------------------------------------------------------------
// "all" — everything
// ---------------------------------------------------------------------------

fn probe_all() {
    probe_terminal();
    probe_osc10();
    probe_osc11();
    probe_osc12();
    probe_osc10_timeout();
    probe_osc11_timeout();
    probe_osc12_timeout();
    probe_cursor();
    probe_cursor_timeout();
    probe_clipboard();
    probe_clipboard_support();
    probe_clipboard_target();
    probe_clipboard_clear();
    probe_clipboard_get();
    probe_mode_2027();
    probe_mode_2027_support();
}
