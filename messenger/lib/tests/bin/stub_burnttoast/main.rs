//! Stub binary that imitates the `pwsh` invocation BurntToast wraps.
//!
//! The real helper pipes a PowerShell script over stdin and watches stdout
//! for a `__MESSENGER_ACTIVATION__\t<json>` marker line. The stub drains
//! stdin (so the helper's `write_all` does not stall), then emits whatever
//! activation marker the test asks for.
//!
//! ## Environment Variables
//!
//! - `STUB_BURNTTOAST_JSON` — JSON payload written after the marker. When
//!   unset the stub emits no marker (the helper falls back to `dismissed`).
//! - `STUB_BURNTTOAST_EXIT` — process exit code. Defaults to `0`.
//! - `STUB_BURNTTOAST_SLEEP_MS` — sleep before printing (timeout path).
//! - `STUB_BURNTTOAST_STDOUT_PREFIX` — extra stdout written before the
//!   activation marker (defaults to a brief preamble line).
//! - `STUB_BURNTTOAST_STDIN_LOG` — append the received script to this file.

use std::io::Read;
use std::io::Write;

fn main() {
    let mut stdin = String::new();
    let _ = std::io::stdin().read_to_string(&mut stdin);
    if let Ok(path) = std::env::var("STUB_BURNTTOAST_STDIN_LOG") {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .expect("open burnttoast stdin log");
        writeln!(file, "-----SCRIPT-----").expect("write burnttoast stdin log marker");
        write!(file, "{stdin}").expect("write burnttoast stdin log");
    }

    if let Ok(ms) = std::env::var("STUB_BURNTTOAST_SLEEP_MS")
        && let Ok(ms) = ms.parse::<u64>()
    {
        std::thread::sleep(std::time::Duration::from_millis(ms));
    }

    let prefix = std::env::var("STUB_BURNTTOAST_STDOUT_PREFIX")
        .unwrap_or_else(|_| "stub burnttoast ready\n".to_string());
    print!("{prefix}");

    if let Ok(json) = std::env::var("STUB_BURNTTOAST_JSON") {
        println!("__MESSENGER_ACTIVATION__\t{json}");
    }

    let exit = std::env::var("STUB_BURNTTOAST_EXIT")
        .ok()
        .and_then(|raw| raw.parse::<i32>().ok())
        .unwrap_or(0);
    std::process::exit(exit);
}
