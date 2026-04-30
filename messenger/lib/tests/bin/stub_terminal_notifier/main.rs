//! Stub binary that imitates `terminal-notifier` for helper integration tests.
//!
//! Real terminal-notifier writes nothing useful to stdout; the helper builds
//! its receipt id from `group_id` / `replace_id` / a fresh UUID. We mirror
//! that by emitting an empty stdout and exiting zero by default.
//!
//! ## Environment Variables
//!
//! - `STUB_TERMINAL_NOTIFIER_EXIT` — process exit code. Defaults to `0`.
//! - `STUB_TERMINAL_NOTIFIER_SLEEP_MS` — sleep before exit (timeout path).

fn main() {
    if let Ok(ms) = std::env::var("STUB_TERMINAL_NOTIFIER_SLEEP_MS")
        && let Ok(ms) = ms.parse::<u64>()
    {
        std::thread::sleep(std::time::Duration::from_millis(ms));
    }

    let exit = std::env::var("STUB_TERMINAL_NOTIFIER_EXIT")
        .ok()
        .and_then(|raw| raw.parse::<i32>().ok())
        .unwrap_or(0);
    std::process::exit(exit);
}
