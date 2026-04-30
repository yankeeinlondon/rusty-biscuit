//! Stub binary that imitates `snoretoast` for helper integration tests.
//!
//! Real SnoreToast prints the chosen action label on stdout (or, on exit
//! code 5, the user's reply text) and returns one of `0`–`5` / `-1`. The
//! stub mirrors that contract under env-var control.
//!
//! ## Environment Variables
//!
//! - `STUB_SNORETOAST_STDOUT` — exact stdout payload (no trailing newline).
//! - `STUB_SNORETOAST_EXIT` — process exit code. Defaults to `0`.
//! - `STUB_SNORETOAST_SLEEP_MS` — sleep before printing (timeout path).

fn main() {
    if let Ok(ms) = std::env::var("STUB_SNORETOAST_SLEEP_MS")
        && let Ok(ms) = ms.parse::<u64>()
    {
        std::thread::sleep(std::time::Duration::from_millis(ms));
    }

    if let Ok(payload) = std::env::var("STUB_SNORETOAST_STDOUT") {
        print!("{payload}");
    }

    let exit = std::env::var("STUB_SNORETOAST_EXIT")
        .ok()
        .and_then(|raw| raw.parse::<i32>().ok())
        .unwrap_or(0);
    std::process::exit(exit);
}
