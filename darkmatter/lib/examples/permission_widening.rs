//! The trusted CI checker for permission widening, and the renderer of its
//! pull-request report.
//!
//! ```text
//! cargo run --locked -p darkmatter --example permission_widening -- \
//!     --repository <dir> --inputs <permission-inputs.json> --base <commit> --head <commit>
//! cargo run --locked -p darkmatter --example permission_widening -- \
//!     --render <template.md> --data <report-data.json>
//! ```
//!
//! The first form reads both commits as object data only
//! (`config::widening`) and prints one JSON object on stdout: the report, or
//! `{"error": "…"}`. The second composes the report template with the data
//! `scripts/ci/permission_scope.py decide` wrote, as the template's `report`
//! frontmatter, and prints the composed Markdown; any composition warning
//! fails it, so a template and its data never drift apart silently. Exit
//! status 0 means the command ran (whatever the comparison found), 1 that it
//! failed, 2 a usage error. CI builds this from the base branch's checkout,
//! never from the candidate.

use std::path::PathBuf;
use std::process::ExitCode;

use darkmatter::config::widening::{WideningInputs, compare_revisions};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{ComposeOptions, ComposeRequest, RequestSnapshot};
use darkmatter::markdown::MergeStrategy;

fn main() -> ExitCode {
    let mut repository = None;
    let mut inputs = None;
    let mut base = None;
    let mut head = None;
    let mut template = None;
    let mut data = None;
    let mut arguments = std::env::args().skip(1);
    while let Some(flag) = arguments.next() {
        let slot = match flag.as_str() {
            "--repository" => &mut repository,
            "--inputs" => &mut inputs,
            "--base" => &mut base,
            "--head" => &mut head,
            "--render" => &mut template,
            "--data" => &mut data,
            _ => return usage(&format!("unknown argument `{flag}`")),
        };
        match arguments.next() {
            Some(value) => *slot = Some(value),
            None => return usage(&format!("`{flag}` needs a value")),
        }
    }
    if template.is_some() || data.is_some() {
        let (Some(template), Some(data), None, None, None, None) = (template, data, &repository, &inputs, &base, &head)
        else {
            return usage("`--render` and `--data` go together, without the comparison's arguments");
        };
        return match render(&template, &data) {
            Ok(markdown) => {
                print!("{markdown}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("permission report: {error}");
                ExitCode::FAILURE
            }
        };
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

/// Composes `template` with the JSON object in `data` as its `report`.
fn render(template: &str, data: &str) -> Result<String, String> {
    let content = std::fs::read_to_string(template).map_err(|error| format!("`{template}` cannot be read: {error}"))?;
    let data: serde_json::Value = std::fs::read_to_string(data)
        .map_err(|error| format!("`{data}` cannot be read: {error}"))
        .and_then(|text| serde_json::from_str(&text).map_err(|error| format!("`{data}` is not JSON: {error}")))?;
    let mut document: Markdown = content.into();
    document
        .fm_merge_with(serde_json::json!({ "report": data }), MergeStrategy::PreferExternal)
        .map_err(|error| error.to_string())?;
    let snapshot = RequestSnapshot::from_process().map_err(|error| error.to_string())?;
    let request = ComposeRequest::prepare(ComposeOptions::new().with_source_file(template), &snapshot)
        .map_err(|error| error.to_string())?;
    let (composed, report) = document.compose_with(&request).map_err(|error| error.to_string())?;
    if let Some(warning) = report.warnings.first() {
        return Err(format!("composing `{template}` warned: {}", warning.message));
    }
    let mut markdown = composed.content().to_string();
    if !markdown.ends_with('\n') {
        markdown.push('\n');
    }
    Ok(markdown)
}

fn usage(message: &str) -> ExitCode {
    println!("{}", serde_json::json!({ "error": message }));
    ExitCode::from(2)
}
