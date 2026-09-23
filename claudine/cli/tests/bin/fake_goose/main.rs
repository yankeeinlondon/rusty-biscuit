//! Fake `goose` provider for `tests/sequence_budget.rs`.
//!
//! A built target rather than a script or a test-time compile: a `.cmd` shim
//! cannot carry Claudine's prompt argument on Windows, and an archived CI run
//! has no compiler.
//!
//! Behaviors, one per launch (1-based), separated by commas; launches past
//! the end of the plan succeed:
//!
//! - `ok` exits 0;
//! - `fail` exits 7;
//! - `sleep:<ms>` sleeps, then exits 0;
//! - `hang` sleeps for ten minutes.
//!
//! Each launch writes `launch-<n>.txt` (its prompt) and `pid-<n>.txt` into
//! `FAKE_DIR`, and `done-<n>.txt` only when it exits on its own.

use std::{env, fs, thread, time::Duration};

fn main() {
    // Marker mode, for callers that only need to count launches
    // (`wrap_compose_validation`). It shares this binary because the reason a
    // compiled fixture exists at all is the same one: Rust refuses to pass
    // arguments it cannot safely escape to a `.bat`/`.cmd` file, so a batch
    // shim carrying Claudine's prompt fails with "batch file arguments are
    // invalid" before the provider ever runs.
    if let Some(marker) = env::var_os("PROVIDER_MARKER") {
        use std::io::Write as _;
        let mut log = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(marker)
            .expect("open PROVIDER_MARKER");
        writeln!(log, "run").expect("record the launch");
        println!("done");
        return;
    }

    let dir = std::path::PathBuf::from(env::var_os("FAKE_DIR").expect("FAKE_DIR"));
    fs::create_dir_all(&dir).unwrap();
    let mut n = 1;
    while fs::OpenOptions::new().write(true).create_new(true).open(dir.join(format!("pid-{n}.txt"))).is_err() {
        n += 1;
    }
    fs::write(dir.join(format!("pid-{n}.txt")), std::process::id().to_string()).unwrap();
    let mut prompt = String::new();
    let mut previous = None;
    for argument in env::args().skip(1) {
        if previous.as_deref() == Some("-t") {
            prompt = argument.clone();
        }
        previous = Some(argument);
    }
    fs::write(dir.join(format!("launch-{n}.txt")), &prompt).unwrap();
    let plan = env::var("FAKE_PLAN").unwrap_or_default();
    let step = plan.split(',').nth(n - 1).unwrap_or("ok").trim().to_string();
    let code = if step == "fail" {
        7
    } else if step == "hang" {
        thread::sleep(Duration::from_secs(600));
        0
    } else if let Some(ms) = step.strip_prefix("sleep:") {
        thread::sleep(Duration::from_millis(ms.parse().unwrap()));
        0
    } else {
        0
    };
    fs::write(dir.join(format!("done-{n}.txt")), code.to_string()).unwrap();
    std::process::exit(code);
}
