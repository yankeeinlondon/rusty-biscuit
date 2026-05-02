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
//! * `clipboard` — Attempt OSC52 clipboard write.
//! * `mode2027` — Enable/disable Mode 2027.
//! * `cursor` — Query cursor position (DSR).
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
        "clipboard" => probe_clipboard(),
        "mode2027" => probe_mode_2027(),
        "cursor" => probe_cursor(),
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

// ---------------------------------------------------------------------------
// Cursor position
// ---------------------------------------------------------------------------

fn probe_cursor() {
    use biscuit_terminal::discovery::cursor_position::cursor_position;
    println!("cursor_position={:?}", cursor_position());
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
    probe_cursor();
    probe_clipboard();
    probe_mode_2027();
}
