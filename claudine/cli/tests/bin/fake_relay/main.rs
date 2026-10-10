//! Fake provider that relays each launch to another program, for
//! `tests/l1/pr_flow_rehearsal.rs` on Windows.
//!
//! A built target rather than a `.cmd` shim: Rust refuses to pass an argument
//! containing a newline to a batch file ("batch file arguments are invalid"),
//! and an interactive launch carries Claudine's prompt in argv.
//!
//! Copied into place as `<name>.exe`, it reads `<name>.relay.json` beside
//! itself, runs `program` with `args` and its own stdin, discards that
//! program's stdout, and appends its stderr to `stderr`. It exits 1 when the
//! program fails, and otherwise prints each of `stdout` on its own line.
//! Its own arguments are ignored.
//!
//! With `"forward": true` it is a transparent stand-in for `program`
//! instead: its own arguments follow `args`, all three standard streams are
//! inherited, and it exits with the program's code. That skips the `cmd.exe`
//! a `.cmd` shim costs on every launch.

use std::fs;
use std::io::Write as _;
use std::process::{Command, Stdio};

#[derive(serde::Deserialize)]
struct Relay {
    program: String,
    args: Vec<String>,
    #[serde(default)]
    stderr: String,
    #[serde(default)]
    stdout: Vec<String>,
    #[serde(default)]
    forward: bool,
}

fn main() {
    let config = std::env::current_exe().expect("own path").with_extension("relay.json");
    let relay: Relay = serde_json::from_str(&fs::read_to_string(&config).expect("read relay config"))
        .expect("parse relay config");
    if relay.forward {
        let status = Command::new(&relay.program)
            .args(&relay.args)
            .args(std::env::args_os().skip(1))
            .status()
            .expect("run forwarded program");
        std::process::exit(status.code().unwrap_or(1));
    }
    let log = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&relay.stderr)
        .expect("open relay stderr log");
    let status = Command::new(&relay.program)
        .args(&relay.args)
        .stdout(Stdio::null())
        .stderr(log)
        .status()
        .expect("run relayed program");
    if !status.success() {
        std::process::exit(1);
    }
    let mut out = std::io::stdout().lock();
    for line in &relay.stdout {
        writeln!(out, "{line}").expect("write relay stdout");
    }
}
