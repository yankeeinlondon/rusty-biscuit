//! Stub binary that imitates `alerter` for helper integration tests.
//!
//! Real alerter writes a single JSON object to stdout describing the user's
//! activation choice. The stub assembles the same JSON payload from env
//! vars so the test can drive every parse branch in `AlerterHelper::parse_output`.
//!
//! ## Environment Variables
//!
//! - `STUB_ALERTER_TYPE` — `activationType` field (e.g. `actionClicked`,
//!   `replied`, `closed`, `timeout`). Defaults to `closed`.
//! - `STUB_ALERTER_VALUE` — `activationValue` field (action id or reply text).
//! - `STUB_ALERTER_STDOUT_OVERRIDE` — replace the entire stdout payload
//!   (parse-error path).
//! - `STUB_ALERTER_EXIT` — process exit code. Defaults to `0`.
//! - `STUB_ALERTER_SLEEP_MS` — sleep before printing (timeout path).

fn main() {
    if let Ok(ms) = std::env::var("STUB_ALERTER_SLEEP_MS")
        && let Ok(ms) = ms.parse::<u64>()
    {
        std::thread::sleep(std::time::Duration::from_millis(ms));
    }

    if let Ok(payload) = std::env::var("STUB_ALERTER_STDOUT_OVERRIDE") {
        print!("{payload}");
    } else {
        let activation_type = std::env::var("STUB_ALERTER_TYPE")
            .unwrap_or_else(|_| "closed".to_string());
        let mut payload = format!("{{\"activationType\":\"{}\"", escape_json(&activation_type));
        if let Ok(value) = std::env::var("STUB_ALERTER_VALUE")
            && !value.is_empty()
        {
            payload.push_str(&format!(",\"activationValue\":\"{}\"", escape_json(&value)));
        }
        payload.push('}');
        println!("{payload}");
    }

    let exit = std::env::var("STUB_ALERTER_EXIT")
        .ok()
        .and_then(|raw| raw.parse::<i32>().ok())
        .unwrap_or(0);
    std::process::exit(exit);
}

fn escape_json(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}
