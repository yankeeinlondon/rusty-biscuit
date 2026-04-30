//! Stub binary that imitates `notify-send` for helper integration tests.
//!
//! Real notify-send (with `-p`) prints the assigned notification id on stdout
//! and exits zero. We mirror that contract.
//!
//! ## Environment Variables
//!
//! - `STUB_NOTIFY_SEND_ID` — id printed on stdout. Defaults to `99`.
//! - `STUB_NOTIFY_SEND_EXIT` — process exit code. Defaults to `0`.
//! - `STUB_NOTIFY_SEND_SLEEP_MS` — sleep before printing (timeout path).
//! - `STUB_NOTIFY_SEND_STDOUT_OVERRIDE` — replaces the entire stdout payload
//!   (parse-error path).

fn main() {
    if let Ok(ms) = std::env::var("STUB_NOTIFY_SEND_SLEEP_MS")
        && let Ok(ms) = ms.parse::<u64>()
    {
        std::thread::sleep(std::time::Duration::from_millis(ms));
    }

    if let Ok(payload) = std::env::var("STUB_NOTIFY_SEND_STDOUT_OVERRIDE") {
        print!("{payload}");
    } else {
        let id = std::env::var("STUB_NOTIFY_SEND_ID").unwrap_or_else(|_| "99".to_string());
        println!("{id}");
    }

    let exit = std::env::var("STUB_NOTIFY_SEND_EXIT")
        .ok()
        .and_then(|raw| raw.parse::<i32>().ok())
        .unwrap_or(0);
    std::process::exit(exit);
}
