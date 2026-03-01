use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::{Value, json};

fn run_stdout(args: &[&str]) -> String {
    let output = cargo_bin_cmd!("sniff")
        .args(args)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    String::from_utf8(output).expect("stdout should be valid UTF-8")
}

fn strip_ansi(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            if matches!(chars.peek(), Some('[')) {
                let _ = chars.next();
                while let Some(next) = chars.next() {
                    if ('@'..='~').contains(&next) {
                        break;
                    }
                }
            }
            continue;
        }
        output.push(ch);
    }

    output
}

fn normalize_text(output: &str) -> String {
    strip_ansi(output).trim().to_string()
}

fn normalize_topics_table(output: &str) -> String {
    let cleaned = normalize_text(output);

    // Already in markdown table form.
    if cleaned
        .lines()
        .next()
        .is_some_and(|line| line.starts_with('|'))
    {
        return cleaned;
    }

    // Convert box-drawn table output back to stable markdown.
    let rows: Vec<Vec<String>> = cleaned
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if !trimmed.starts_with('│') {
                return None;
            }

            let cells: Vec<String> = trimmed
                .split('│')
                .skip(1)
                .take(3)
                .map(|cell| cell.trim().to_string())
                .collect();

            if cells.is_empty() { None } else { Some(cells) }
        })
        .collect();

    if rows.is_empty() {
        return cleaned;
    }

    let mut lines = Vec::new();
    lines.push(format!("| {} |", rows[0].join(" | ")));
    lines.push(format!(
        "| {} |",
        rows[0]
            .iter()
            .map(|_| "---")
            .collect::<Vec<_>>()
            .join(" | ")
    ));

    for row in rows.iter().skip(1) {
        lines.push(format!("| {} |", row.join(" | ")));
    }

    lines.join("\n")
}

fn normalized_os_summary(output: &str) -> Value {
    let json: Value = serde_json::from_str(&normalize_text(output)).expect("valid JSON");
    let manager_count = json
        .get("system_package_managers")
        .and_then(|v| v.get("managers"))
        .and_then(Value::as_array)
        .map_or(0, Vec::len);

    json!({
        "distribution": json.get("distribution"),
        "kernel": json.get("kernel"),
        "long_version": json.get("long_version"),
        "name": json.get("name"),
        "os_type": json.get("os_type"),
        "version": json.get("version"),
        "manager_count": manager_count
    })
}

#[test]
fn help_output_snapshot() {
    insta::assert_snapshot!("help_output", normalize_text(&run_stdout(&["--help"])));
}

#[test]
fn completions_help_output_snapshot() {
    insta::assert_snapshot!(
        "completions_help_output",
        normalize_text(&run_stdout(&["--completions", "--help"]))
    );
}

#[test]
fn topics_table_snapshot() {
    insta::assert_snapshot!(
        "topics_table",
        normalize_topics_table(&run_stdout(&["topics"]))
    );
}

#[test]
fn structure_output_snapshot() {
    insta::assert_snapshot!(
        "structure_output",
        normalize_text(&run_stdout(&["structure"]))
    );
}

#[test]
fn os_json_snapshot() {
    let stdout = run_stdout(&["os", "--json"]);
    insta::assert_json_snapshot!("os_json_summary", normalized_os_summary(&stdout));
}
