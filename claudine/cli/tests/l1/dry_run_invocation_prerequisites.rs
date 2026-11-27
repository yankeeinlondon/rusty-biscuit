//! An invocation-wide prerequisite is one finding for the whole dry run, never
//! dropped and never repeated by each task that depends on it (L1).
//!
//! The prerequisites are the configurations (Darkmatter's `~/.darkmatter.yaml`
//! and Claudine's `~/.claudine/config.yaml`, whose load and captured permission
//! verdict every preparation shares), the skipped schema triggers, the
//! repository's inline guardrails file, the launch-repository discovery, and
//! the launch directory itself. Each row spawns `compose`, `inline-compose`,
//! or `sequence --dry-run`, mostly with `--json`, in its own fixture. The
//! Darkmatter loader's own handling of each `network` shape and each broken
//! YAML form is pinned once at its public loader
//! (`darkmatter/lib/tests/l1/config_fields.rs`, `config_load.rs`,
//! `network_approval.rs`); these rows prove only the wiring.

use std::fs;

use serde_json::Value;

use crate::common::CliProcessFixture;
use crate::dry_run_network_approval::Responder;

const PROMPT: &str = "---\nagent: claude\n---\nHello.\n";
const INLINE: &str = "---\nagent: claude\nprompt: Say hello.\n---\n";
/// A named body task (composes the root's body) and a prompt task.
const SEQUENCE: &str = "---\nagent: claude\nsequence:\n  - name: body\n  - name: prompt\n    prompt: ./p.md\n---\nBody here.\n";

/// A fixture whose home's `.darkmatter.yaml` is `darkmatter`, with the three
/// documents and a provider stub the check only looks up.
fn fixture(darkmatter: &str) -> CliProcessFixture {
    let fixture = CliProcessFixture::named("dry-run-prerequisites");
    fs::write(fixture.home().join(".darkmatter.yaml"), darkmatter).unwrap();
    fs::write(fixture.cwd().join("p.md"), PROMPT).unwrap();
    fs::write(fixture.cwd().join("i.md"), INLINE).unwrap();
    fs::write(fixture.cwd().join("s.md"), SEQUENCE).unwrap();
    let claude = fixture.bin_dir().join(format!("claude{}", std::env::consts::EXE_SUFFIX));
    fs::write(&claude, "#!/bin/sh\nexit 0\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&claude, fs::Permissions::from_mode(0o755)).unwrap();
    }
    fixture
}

/// `claudine <command> <file> --dry-run --json`: the exit code and the one
/// envelope, which is all stdout holds; stderr stays empty.
fn dry_run(fixture: &CliProcessFixture, command: &str, file: &str) -> (Option<i32>, Value) {
    let output = fixture.command().args([command, file, "--dry-run", "--json"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let envelope = serde_json::from_str(&stdout).unwrap_or_else(|error| panic!("{error}: {stdout}"));
    assert!(output.stderr.is_empty(), "{}", String::from_utf8_lossy(&output.stderr));
    (output.status.code(), envelope)
}

fn rows(envelope: &Value) -> Vec<Value> {
    envelope["diagnostics"].as_array().unwrap().clone()
}

fn code(row: &Value) -> &str {
    row["code"].as_str().unwrap_or_default()
}

/// The configuration rows: Darkmatter's `dm.config.*` and Claudine's
/// `config.*`.
fn config_rows(envelope: &Value) -> Vec<Value> {
    rows(envelope).into_iter().filter(|row| code(row).starts_with("dm.config.") || code(row).starts_with("config.")).collect()
}

/// `(file name, line, column)` of a row's source.
fn located(row: &Value) -> (String, Option<u64>, Option<u64>) {
    let source = &row["source"];
    let file = source["file"].as_str().unwrap_or_default();
    let name = file.rsplit('/').next().unwrap_or_default().to_string();
    (name, source["line"].as_u64(), source["column"].as_u64())
}

/// A row under the root subject carries no task frame.
fn under_root(row: &Value) -> bool {
    row.get("trail").is_none_or(|trail| trail.as_array().is_none_or(Vec::is_empty))
}

fn network(value: &str) -> String {
    format!("permissions:\n  network: {value}\n")
}

/// The one located configuration row of `envelope`, under the root.
fn one_config_row(exit: Option<i32>, envelope: &Value, file: &str, line: u64, column: u64) -> Value {
    let found = config_rows(envelope);
    assert_eq!(exit, Some(2), "{envelope:#}");
    assert_eq!(found.len(), 1, "{envelope:#}");
    let row = found[0].clone();
    assert_eq!(located(&row), (file.to_string(), Some(line), Some(column)), "{row:#}");
    assert!(under_root(&row), "{row:#}");
    row
}

// ---- R2-F4.2 / R2-F4.6: the captured `network` verdict, compose and inline ----

/// A valid or absent setting is clean, exit 0; an invalid one is one located
/// finding at its value (`.darkmatter.yaml:2:12`), exit 2, though the prompt
/// reads nothing remote.
fn network_cell(command: &str, file: &str, darkmatter: impl AsRef<str>, valid: bool) {
    let fixture = fixture(darkmatter.as_ref());
    let (exit, envelope) = dry_run(&fixture, command, file);
    if valid {
        assert_eq!((exit, config_rows(&envelope).len()), (Some(0), 0), "{envelope:#}");
    } else {
        let row = one_config_row(exit, &envelope, ".darkmatter.yaml", 2, 12);
        assert_eq!(code(&row), "dm.config.invalid", "{row:#}");
        assert!(row["hint"].is_string(), "{row:#}");
    }
}

macro_rules! network_cells {
    ($($name:ident: $darkmatter:expr, $valid:expr;)*) => {
        mod compose_network {
            $(#[test] fn $name() { super::network_cell("compose", "p.md", $darkmatter, $valid); })*
        }
        mod inline_compose_network {
            $(#[test] fn $name() { super::network_cell("inline-compose", "i.md", $darkmatter, $valid); })*
        }
        mod sequence_network {
            $(#[test] fn $name() { super::network_cell("sequence", "s.md", $darkmatter, $valid); })*
        }
    };
}

network_cells! {
    allow: super::network("allow"), true;
    ask: super::network("ask"), true;
    deny: super::network("deny"), true;
    absent_network: "permissions: {}\n", true;
    null_network: super::network("null"), true;
    null_permissions: "permissions: null\n", true;
    invalid_token: super::network("aswk"), false;
    empty_string: super::network("\"\""), false;
    list: super::network("[allow]"), false;
}

#[test]
fn an_invalid_network_setting_is_one_row_in_the_human_report_too() {
    let fixture = fixture(&network("aswk"));
    let output = fixture.command().args(["compose", "p.md", "--dry-run"]).output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2), "{stderr}");
    assert_eq!(stderr.matches("/permissions/network").count(), 1, "{stderr}");
    assert!(stderr.contains("Dry run found 1 error and 0 warnings"), "{stderr}");
    assert!(!stderr.contains("Error:"), "never the standard error report: {stderr}");
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "Hello.");
}

// ---- R2-F4.3 / R2-F4.4: a configuration that does not load ----

/// Malformed YAML, a duplicate key, and a second document: one located
/// `dm.config.invalid` under the root for the invocation. In a sequence the
/// named body task and the prompt task each have one `not checked` row
/// blocked by it, never their own copy.
fn load_failure_cell(command: &str, file: &str, darkmatter: &str, line: u64, column: u64) {
    let fixture = fixture(darkmatter);
    let (exit, envelope) = dry_run(&fixture, command, file);
    let row = one_config_row(exit, &envelope, ".darkmatter.yaml", line, column);
    assert_eq!(code(&row), "dm.config.invalid", "{row:#}");
    if command == "sequence" {
        let blocked: Vec<_> = rows(&envelope)
            .into_iter()
            .filter(|row| row["message"].as_str().is_some_and(|message| message.contains("(the configuration does not load)")))
            .collect();
        assert_eq!(blocked.len(), 2, "{envelope:#}");
        for blocked in &blocked {
            assert_eq!(code(blocked), "dm.check.not_checked", "{blocked:#}");
            assert_eq!(located(&blocked["related"][0]), (".darkmatter.yaml".to_string(), Some(line), Some(column)));
        }
    }
}

macro_rules! load_failure_cells {
    ($($name:ident: $darkmatter:expr, $line:expr, $column:expr;)*) => {
        mod compose_load_failure {
            $(#[test] fn $name() { super::load_failure_cell("compose", "p.md", $darkmatter, $line, $column); })*
        }
        mod inline_compose_load_failure {
            $(#[test] fn $name() { super::load_failure_cell("inline-compose", "i.md", $darkmatter, $line, $column); })*
        }
        mod sequence_load_failure {
            $(#[test] fn $name() { super::load_failure_cell("sequence", "s.md", $darkmatter, $line, $column); })*
        }
    };
}

load_failure_cells! {
    malformed: "permissions: [\n", 2, 1;
    duplicate_key: "permissions:\n  network: allow\n  network: deny\n", 3, 3;
    second_document: "permissions:\n  network: allow\n---\nregister: []\n", 3, 1;
}

// ---- Claudine's configuration, and a Darkmatter reference to it ----

/// `~/.darkmatter.yaml` points its `permissions` at Claudine's file, which
/// holds the broken block: Claudine's load reports it once, located, under
/// the root (never a "command line" subject), and the Darkmatter verdict that
/// follows from the same block is not a second row.
fn claudine_reference_cell(command: &str, file: &str, claudine: &str, line: u64, column: u64) {
    let fixture = fixture("permissions: ~/.claudine/config.yaml\n");
    fs::create_dir_all(fixture.home().join(".claudine")).unwrap();
    fs::write(fixture.home().join(".claudine/config.yaml"), claudine).unwrap();
    let (exit, envelope) = dry_run(&fixture, command, file);
    let row = one_config_row(exit, &envelope, "config.yaml", line, column);
    assert_eq!(code(&row), "config.invalid", "{row:#}");
    let output = fixture.command().args([command, file, "--dry-run"]).output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("command line"), "the root names the document: {stderr}");
}

macro_rules! claudine_reference_cells {
    ($($name:ident: $claudine:expr, $line:expr, $column:expr;)*) => {
        mod compose_claudine_reference {
            $(#[test] fn $name() { super::claudine_reference_cell("compose", "p.md", $claudine, $line, $column); })*
        }
        mod sequence_claudine_reference {
            $(#[test] fn $name() { super::claudine_reference_cell("sequence", "s.md", $claudine, $line, $column); })*
        }
    };
}

claudine_reference_cells! {
    invalid_token: "permissions:\n  network: aswk\n", 2, 12;
    duplicate_key: "permissions:\n  network: allow\n  network: deny\n", 3, 3;
}

// ---- R2-F4.5: a remote read under an invalid setting ----

/// The read is refused with no request sent, and its refusal relates to the
/// one configuration finding instead of standing as a second cause.
#[test]
fn a_remote_read_under_an_invalid_setting_is_refused_and_relates_to_the_configuration() {
    let server = Responder::start();
    let fixture = fixture(&network("aswk"));
    fs::write(fixture.cwd().join("r.md"), format!("---\nagent: claude\n---\nLocal.\n\n::file {}\n", server.url("/child.md")))
        .unwrap();
    let (exit, envelope) = dry_run(&fixture, "compose", "r.md");
    one_config_row(exit, &envelope, ".darkmatter.yaml", 2, 12);
    assert_eq!(server.requests(), 0, "{envelope:#}");
    let refusal: Vec<_> = rows(&envelope).into_iter().filter(|row| code(row) == "dm.transclusion.failure").collect();
    assert_eq!(refusal.len(), 1, "{envelope:#}");
    assert_eq!(located(&refusal[0]), ("r.md".to_string(), Some(6), Some(1)));
    assert_eq!(located(&refusal[0]["related"][0]), (".darkmatter.yaml".to_string(), Some(2), Some(12)), "{envelope:#}");
}

// ---- R2-F4.7: skipped schema triggers ----

/// A broken trigger in `~/schemas` is one warning for the invocation, as the
/// real run warns once, however many prompts discover it.
#[test]
fn a_skipped_schema_trigger_is_one_warning_for_the_invocation() {
    let fixture = fixture("{}\n");
    fs::create_dir_all(fixture.home().join("schemas")).unwrap();
    fs::write(fixture.home().join("schemas/memory.yaml"), "kind: schema-trigger\ntitle: Memory Trigger\nmatch:\n").unwrap();
    for (command, file) in [("compose", "p.md"), ("sequence", "s.md")] {
        let (exit, envelope) = dry_run(&fixture, command, file);
        let triggers: Vec<_> = rows(&envelope).into_iter().filter(|row| code(row) == "dm.schema.invalid_trigger").collect();
        assert_eq!((exit, triggers.len()), (Some(2), 1), "{command}: {envelope:#}");
        assert_eq!(located(&triggers[0]).0, "memory.yaml");
        assert!(under_root(&triggers[0]), "{command}: {envelope:#}");
    }
}

// ---- R2-F4.8: the repository's inline guardrails ----

/// A guardrails file with retired instructions is one warning at that file,
/// not one per inline prompt task that reads it.
#[test]
fn retired_guardrails_are_one_warning_at_the_guardrails_file() {
    let fixture = fixture("{}\n");
    fixture.initialize_repository();
    fs::create_dir_all(fixture.cwd().join(".claudine")).unwrap();
    fs::write(fixture.cwd().join(".claudine/inline-compose.md"), "> - Do not edit the source file directly\n").unwrap();
    fs::write(fixture.cwd().join("i2.md"), INLINE).unwrap();
    let sequence = "---\nagent: claude\nsequence:\n  - name: a\n    prompt: ./i.md\n  - name: b\n    prompt: ./i2.md\n  - name: c\n    prompt: ./i.md\n---\n";
    fs::write(fixture.cwd().join("three.md"), sequence).unwrap();
    let (exit, envelope) = dry_run(&fixture, "sequence", "three.md");
    let retired: Vec<_> = rows(&envelope)
        .into_iter()
        .filter(|row| row["message"].as_str().is_some_and(|message| message.contains("retired response-harvesting")))
        .collect();
    assert_eq!((exit, retired.len()), (Some(2), 1), "{envelope:#}");
    assert_eq!(located(&retired[0]).0, "inline-compose.md", "{envelope:#}");
    assert!(under_root(&retired[0]), "{envelope:#}");
}

// ---- R2-F4.9: launch-repository discovery ----

/// A launch directory whose `.git` names no repository: a real run refuses
/// before any lifecycle runs, so the dry run reports it once, exit 2.
#[test]
fn a_failed_launch_repository_discovery_is_one_root_error() {
    let fixture = fixture("{}\n");
    fs::write(fixture.cwd().join(".git"), "gitdir: ./missing-gitdir\n").unwrap();
    for (command, file) in [("compose", "p.md"), ("sequence", "s.md")] {
        let (exit, envelope) = dry_run(&fixture, command, file);
        let found: Vec<_> = rows(&envelope)
            .into_iter()
            .filter(|row| row["message"].as_str().is_some_and(|message| message.contains("repository discovery")))
            .collect();
        assert_eq!((exit, found.len()), (Some(2), 1), "{command}: {envelope:#}");
        assert_eq!(found[0]["severity"], "error");
        assert!(under_root(&found[0]), "{command}: {envelope:#}");
    }
}

// ---- R2-F4.10: a launch directory that cannot be read ----

/// The launch directory is removed before `claudine` starts (only Unix lets a
/// process run in a removed directory): the dry run still answers with its
/// one envelope and exit 2, never the standard error report and exit 1.
#[cfg(unix)]
#[test]
fn an_unreadable_launch_directory_is_the_dry_runs_one_finding() {
    let fixture = fixture("{}\n");
    let gone = fixture.workspace_path().join("gone");
    fs::create_dir(&gone).unwrap();
    let document = fixture.cwd().join("s.md");
    let program = fixture.command_std().get_program().to_owned();
    let mut command = std::process::Command::new("sh");
    command.args(["-c", "cd \"$1\" && rmdir \"$PWD\" && shift && exec \"$@\"", "sh"]).arg(&gone).arg(program);
    command.args(["sequence", &document.to_string_lossy(), "--dry-run", "--json"]);
    // Claudine is a grandchild of the shell; apply the same hermetic policy.
    fixture.command_builder().apply_policy_to(&mut command);
    let output = command.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let envelope: Value = serde_json::from_str(&stdout).unwrap_or_else(|error| panic!("{error}: {stdout}"));
    assert_eq!(output.status.code(), Some(2), "{envelope:#}");
    assert!(output.stderr.is_empty(), "{}", String::from_utf8_lossy(&output.stderr));
    let found = rows(&envelope);
    assert_eq!(found.len(), 1, "{envelope:#}");
    assert_eq!(code(&found[0]), "io.read_failed", "{envelope:#}");
}
