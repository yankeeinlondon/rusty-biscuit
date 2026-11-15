//! The trusted CI checker for permission widening.
//!
//! ```text
//! cargo run --locked -p darkmatter --example permission_widening -- \
//!     --repository <dir> --inputs <permission-inputs.json> --base <commit> --head <commit>
//! ```
//!
//! Reads both commits as object data only (`config::widening`) and prints
//! one JSON object on stdout: the report, or `{"error": "…"}`. Exit status 0
//! means the comparison ran (whatever it found), 1 that it failed, 2 a usage
//! error. CI builds this from the base branch's checkout, never from the
//! candidate.

use std::path::PathBuf;
use std::process::ExitCode;

use darkmatter::config::widening::{WideningInputs, compare_revisions};

fn main() -> ExitCode {
    let mut repository = None;
    let mut inputs = None;
    let mut base = None;
    let mut head = None;
    let mut arguments = std::env::args().skip(1);
    while let Some(flag) = arguments.next() {
        let slot = match flag.as_str() {
            "--repository" => &mut repository,
            "--inputs" => &mut inputs,
            "--base" => &mut base,
            "--head" => &mut head,
            _ => return usage(&format!("unknown argument `{flag}`")),
        };
        match arguments.next() {
            Some(value) => *slot = Some(value),
            None => return usage(&format!("`{flag}` needs a value")),
        }
    }
    let (Some(repository), Some(inputs), Some(base), Some(head)) = (repository, inputs, base, head) else {
        return usage("`--repository`, `--inputs`, `--base`, and `--head` are all required");
    };
    let result = std::fs::read_to_string(&inputs)
        .map_err(|error| format!("`{inputs}` cannot be read: {error}"))
        .and_then(|text| WideningInputs::from_json(&text).map_err(|error| error.to_string()))
        .and_then(|inputs| {
            compare_revisions(&PathBuf::from(&repository), &base, &head, &inputs).map_err(|error| error.to_string())
        });
    match result {
        Ok(report) => {
            println!("{}", serde_json::to_string_pretty(&report).expect("a report serializes"));
            ExitCode::SUCCESS
        }
        Err(error) => {
            println!("{}", serde_json::json!({ "error": error }));
            ExitCode::FAILURE
        }
    }
}

fn usage(message: &str) -> ExitCode {
    println!("{}", serde_json::json!({ "error": message }));
    ExitCode::from(2)
}
