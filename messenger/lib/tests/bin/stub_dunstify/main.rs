//! Stub binary that imitates `dunstify` for desktop helper integration tests.
//!
//! Behaviour is controlled exclusively through environment variables so the
//! same binary can drive every test path. Real dunstify writes the assigned
//! notification id on stdout (and, with `--wait`, an action key on a second
//! line) and returns an exit code that encodes the close reason.
//!
//! ## Environment Variables
//!
//! - `STUB_DUNSTIFY_ID` — id printed on stdout. Defaults to `42`.
//! - `STUB_DUNSTIFY_ACTION` — second-line action key (only printed when the
//!   value is non-empty).
//! - `STUB_DUNSTIFY_EXIT` — process exit code. Defaults to `0`.
//! - `STUB_DUNSTIFY_SLEEP_MS` — sleep before printing (used to exercise the
//!   helper timeout path).
//! - `STUB_DUNSTIFY_STDOUT_OVERRIDE` — replaces the entire stdout payload
//!   (used to exercise the parse-error path).

fn main() {
    if let Ok(ms) = std::env::var("STUB_DUNSTIFY_SLEEP_MS")
        && let Ok(ms) = ms.parse::<u64>()
    {
        std::thread::sleep(std::time::Duration::from_millis(ms));
    }

    if let Ok(payload) = std::env::var("STUB_DUNSTIFY_STDOUT_OVERRIDE") {
        print!("{payload}");
    } else {
        let id = std::env::var("STUB_DUNSTIFY_ID").unwrap_or_else(|_| "42".to_string());
        println!("{id}");
        if let Ok(action) = std::env::var("STUB_DUNSTIFY_ACTION")
            && !action.is_empty()
        {
            println!("{action}");
        }
    }

    let exit = std::env::var("STUB_DUNSTIFY_EXIT")
        .ok()
        .and_then(|raw| raw.parse::<i32>().ok())
        .unwrap_or(0);
    std::process::exit(exit);
}
