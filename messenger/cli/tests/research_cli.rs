//! The real `messenger research …` binary against isolated fixture trees:
//! help, exit codes, JSON output, filters, drift, stale reporting, invalid
//! inputs, interrupted generation, and byte-identical double generation. The
//! shipped repository is exercised read-only through `validate`.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use biscuit_terminal::terminal::Terminal;
use messenger::research::generate::generate;
use messenger::research::model::{Date, PlatformId};
use messenger::research::publish::{Options, Point};
use messenger::research::report::{Filter, build};
use messenger::research::{Loader, Workspace};
use serde_json::Value;
use tempfile::TempDir;

fn cli_dir() -> PathBuf {
    std::env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")))
}

fn repo_root() -> PathBuf {
    cli_dir().parent().and_then(Path::parent).expect("repository root").to_path_buf()
}

const SHIPPED: &[&str] = &[
    "messenger/docs/platforms.yaml",
    "messenger/docs/platforms.schema.yaml",
    "messenger/docs/research/platforms/_schema.yaml",
    "messenger/docs/research/platforms/_types.yaml",
    "messenger/docs/research/platforms/_overrides.schema.yaml",
    "messenger/docs/research/platforms/_fleet.md",
    "messenger/docs/research/implementation/_schema.yaml",
];
const CATALOG: &str = "messenger/docs/research/platforms/catalog.json";
const SUMMARY: &str = "messenger/docs/research/summary/platforms.md";
const MANIFEST: &str = "messenger/docs/research/publication.json";

/// A repository holding the shipped contract and the accepted-fleet fixture.
struct Fleet {
    dir: TempDir,
}

impl Fleet {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        for path in SHIPPED {
            let target = dir.path().join(path);
            fs::create_dir_all(target.parent().expect("parent")).expect("mkdir");
            fs::copy(repo_root().join(path), target).expect("copy");
        }
        let fleet = Self { dir };
        for platform in PlatformId::ALL {
            let source = repo_root().join(format!("messenger/lib/tests/fixtures/research/lifecycle/fleet/{platform}.md"));
            let text = fs::read_to_string(source)
                .expect("fleet fixture")
                .replace("$schema: ../../../../../../docs/research/platforms/_schema.yaml", "$schema: ./_schema.yaml");
            fleet.write(&format!("messenger/docs/research/platforms/{platform}.md"), &text);
        }
        fleet
    }

    fn root(&self) -> &Path {
        self.dir.path()
    }

    fn path(&self, relative: &str) -> PathBuf {
        relative.split('/').fold(self.root().to_path_buf(), |acc, segment| acc.join(segment))
    }

    fn write(&self, relative: &str, content: &str) {
        let path = self.path(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        fs::write(path, content).expect("write");
    }

    fn read(&self, relative: &str) -> Vec<u8> {
        fs::read(self.path(relative)).expect("read")
    }

    fn text(&self, relative: &str) -> String {
        String::from_utf8(self.read(relative)).expect("utf-8")
    }

    fn research(&self, args: &[&str]) -> Output {
        research(Some(self.root()), args)
    }

    /// Every file under `messenger/docs/`.
    fn docs(&self) -> BTreeMap<PathBuf, Vec<u8>> {
        fn walk(dir: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
            for entry in fs::read_dir(dir).expect("dir").flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, out);
                } else {
                    out.insert(path.clone(), fs::read(&path).expect("read"));
                }
            }
        }
        let mut out = BTreeMap::new();
        walk(&self.root().join("messenger/docs"), &mut out);
        out
    }
}

fn research(root: Option<&Path>, args: &[&str]) -> Output {
    // `bin_exe!` prefers nextest's run-time binary path, which the WSL2
    // archive leg needs (the compile-time path names the builder's target dir).
    let mut command = Command::new(biscuit_test_harness::bin_exe!("messenger"));
    command.arg("research");
    if let Some(root) = root {
        command.arg("--root").arg(root);
    }
    if !args.contains(&"--today") {
        command.args(["--today", "2026-09-17"]);
    }
    command.args(args);
    command.env_remove("COMPLETE").env("NO_COLOR", "1");
    command.output().expect("run messenger")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("utf-8 stdout")
}

/// The whole of stdout is one JSON document without presentation escapes.
fn json(output: &Output) -> Value {
    let text = stdout(output);
    assert!(!text.contains('\u{1b}'), "escape sequence in JSON stdout:\n{text}");
    serde_json::from_str(&text).unwrap_or_else(|error| panic!("stdout is not one JSON document ({error}):\n{text}"))
}

fn code(output: &Output) -> i32 {
    output.status.code().expect("exit code")
}

#[test]
fn help_lists_the_research_commands_and_exit_statuses() {
    let output = research(None, &["--help"]);
    assert_eq!(code(&output), 0);
    let help = stdout(&output);
    for command in ["validate", "generate", "report", "recover", "--root", "--today", "EXIT STATUS"] {
        assert!(help.contains(command), "{command} missing from:\n{help}");
    }
    let generate = stdout(&research(None, &["generate", "--help"]));
    assert!(generate.contains("--check"));
    let report = stdout(&research(None, &["report", "--help"]));
    for filter in ["--platform", "--interface", "--operation", "--json"] {
        assert!(report.contains(filter), "{filter}");
    }
    assert_eq!(code(&research(None, &["report", "--platform", "email"])), 2, "an unknown platform is a usage error");
    assert_eq!(code(&research(None, &["--today", "17/09/2026", "report"])), 2);
}

/// Review-1 regression: `--today 2026-02-31` was accepted.
#[test]
fn today_accepts_only_real_calendar_days() {
    let fleet = Fleet::new();
    for valid in ["2024-02-29", "2000-02-29"] {
        let output = fleet.research(&["--today", valid, "validate", "--json"]);
        assert_eq!(code(&output), 0, "{valid}: {}", String::from_utf8_lossy(&output.stderr));
    }
    let invalid = [
        "2026-02-29", "1900-02-29", "2026-02-30", "2026-02-31", "2026-04-31", "2026-06-31", "2026-09-31", "2026-11-31",
        "2026-01-00", "2026-00-10", "2026-13-01",
    ];
    for text in invalid {
        let output = research(None, &["--today", text, "report"]);
        assert_eq!(code(&output), 2, "{text} is a usage error");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains(&format!("`{text}` is not a real YYYY-MM-DD calendar date")), "{text}: {stderr}");
    }
}

#[test]
fn validate_accepts_the_fleet_fixture() {
    let fleet = Fleet::new();
    let output = fleet.research(&["validate", "--json"]);
    assert_eq!(code(&output), 0, "{}", stdout(&output));
    let value = json(&output);
    assert_eq!(value["valid"], true);
    assert_eq!(value["diagnostics"], Value::Array(Vec::new()));

    let human = fleet.research(&["validate"]);
    assert_eq!(code(&human), 0);
    assert!(stdout(&human).contains("Research inputs are valid."));

    // A single candidate can be judged as accepted research or as a fragment.
    let candidate = fleet.path("messenger/docs/research/platforms/discord.md");
    let output = fleet.research(&["validate", "--json", candidate.to_str().unwrap()]);
    assert_eq!(code(&output), 0, "{}", stdout(&output));
}

/// End to end over the real shipped artifacts: the accepted documents are
/// still legacy prose without a bound schema, so the fleet is not yet valid.
#[test]
fn validate_reports_the_shipped_legacy_documents_as_unbound() {
    let output = research(Some(&repo_root()), &["validate", "--json"]);
    assert_eq!(code(&output), 1, "{}", stdout(&output));
    let value = json(&output);
    assert_eq!(value["valid"], false);
    let unbound: Vec<&str> = value["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|d| d["rule"] == "SR-SCHEMA-BINDING")
        .map(|d| d["path"].as_str().unwrap())
        .collect();
    assert_eq!(unbound.len(), 5, "{value}");
    assert!(unbound.iter().all(|path| path.starts_with("messenger/docs/research/platforms/")));
    let text = stdout(&output);
    assert!(!text.contains(&repo_root().display().to_string()), "host path in output");
}

#[test]
fn generate_twice_is_byte_identical_and_check_detects_drift() {
    let fleet = Fleet::new();
    let first = fleet.research(&["generate", "--json"]);
    assert_eq!(code(&first), 0, "{}", stdout(&first));
    let first = json(&first);
    assert_eq!(first["unchanged"], false);
    let snapshot = fleet.docs();

    let second = fleet.research(&["generate", "--json"]);
    assert_eq!(code(&second), 0);
    let second = json(&second);
    assert_eq!(second["unchanged"], true);
    assert_eq!(first["snapshot_id"], second["snapshot_id"]);
    assert_eq!(fleet.docs(), snapshot, "the second generation changed bytes");

    let check = fleet.research(&["generate", "--check", "--json"]);
    assert_eq!(code(&check), 0);
    assert_eq!(json(&check)["drift"], Value::Null);
    assert!(stdout(&fleet.research(&["generate", "--check"])).contains("No drift"));

    // Changing an input without regenerating is drift, and --check writes nothing.
    let roster = fleet.text("messenger/docs/platforms.yaml");
    fleet.write("messenger/docs/platforms.yaml", &roster.replacen("refresh_interval_days: 30", "refresh_interval_days: 7", 1));
    let before = fleet.docs();
    let check = fleet.research(&["generate", "--check", "--json"]);
    assert_eq!(code(&check), 1);
    let drift = json(&check);
    assert_eq!(drift["drift"]["kind"], "changed");
    assert_eq!(drift["drift"]["paths"], serde_json::json!([CATALOG, MANIFEST]));
    assert_eq!(fleet.docs(), before);
    let human = stdout(&fleet.research(&["generate", "--check"]));
    assert!(human.contains("Drift") && human.contains(CATALOG), "{human}");

    // A hand edit of generated output is refused as unavailable, not drift.
    fleet.write("messenger/docs/platforms.yaml", &roster);
    let catalog = fleet.text(CATALOG);
    fleet.write(CATALOG, &catalog.replacen("\"known\"", "\"unknown\"", 1));
    let check = fleet.research(&["generate", "--check", "--json"]);
    assert_eq!(code(&check), 3);
    assert!(json(&check)["error"].as_str().unwrap().contains(CATALOG));
}

#[test]
fn invalid_inputs_are_refused_and_leave_the_snapshot_unchanged() {
    let fleet = Fleet::new();
    assert_eq!(code(&fleet.research(&["generate"])), 0);
    let discord = fleet.text("messenger/docs/research/platforms/discord.md");
    fleet.write(
        "messenger/docs/research/platforms/discord.md",
        &discord.replacen("  value: 2000\n", "  value: \"2000\"\n", 1),
    );
    let before = fleet.docs();
    let output = fleet.research(&["generate", "--json"]);
    assert_eq!(code(&output), 3, "a hand-edited accepted document fails verification: {}", stdout(&output));
    assert_eq!(fleet.docs(), before);

    // Without a published snapshot the same input is judged and refused.
    let fresh = Fleet::new();
    fresh.write(
        "messenger/docs/research/platforms/discord.md",
        &fresh.text("messenger/docs/research/platforms/discord.md").replacen("  value: 2000\n", "  value: \"2000\"\n", 1),
    );
    let before = fresh.docs();
    let output = fresh.research(&["generate", "--json"]);
    assert_eq!(code(&output), 1, "{}", stdout(&output));
    let value = json(&output);
    assert_eq!(value["refused"], true);
    assert_eq!(value["diagnostics"][0]["rule"], "SR-STRICT-SCALARS");
    assert_eq!(fresh.docs(), before, "a refused generation writes nothing");
    assert!(!fresh.path(MANIFEST).exists());
    let human = fresh.research(&["generate"]);
    assert_eq!(code(&human), 1);
    assert!(stdout(&human).contains("Generation refused"));
}

#[test]
fn an_interrupted_generation_requires_explicit_recovery() {
    let fleet = Fleet::new();
    assert_eq!(code(&fleet.research(&["generate"])), 0);
    let published = fleet.docs();

    let loader = Loader::new(Workspace::new(fleet.root()).expect("absolute"));
    let discord = fleet.text("messenger/docs/research/platforms/discord.md").replacen("  value: 2000\n", "  value: 4000\n", 1);
    let updates = BTreeMap::from([(PlatformId::Discord, discord)]);
    let today = Date::parse("2026-09-17").unwrap();
    generate(&loader, &updates, &today, Options { interrupt_at: Some(Point::MidReplacement(2)), ..Options::default() })
        .expect_err("interrupted");

    for args in [&["generate", "--json"][..], &["generate", "--check", "--json"][..]] {
        let output = fleet.research(args);
        assert_eq!(code(&output), 3, "{args:?}");
        assert!(json(&output)["error"].as_str().unwrap().contains("messenger research recover"), "{args:?}");
    }
    let report = fleet.research(&["report", "--json"]);
    assert_eq!(code(&report), 3, "a mixture is never reported");

    let recovered = fleet.research(&["recover", "--json"]);
    assert_eq!(code(&recovered), 0);
    assert_eq!(json(&recovered)["recovery"], "rolled_back");
    assert_eq!(fleet.docs(), published);
    assert_eq!(code(&fleet.research(&["generate", "--check"])), 0);
    let again = fleet.research(&["recover"]);
    assert_eq!(code(&again), 0);
    assert!(stdout(&again).contains("Nothing to recover"));
}

#[test]
fn report_json_is_filterable_and_marks_stale_research() {
    let fleet = Fleet::new();
    assert_eq!(code(&fleet.research(&["report", "--json"])), 3, "no snapshot yet");
    assert_eq!(code(&fleet.research(&["generate"])), 0);

    let report = json(&fleet.research(&["report", "--json"]));
    assert_eq!(report["truncation"].as_array().unwrap().len(), 7);
    assert_eq!(report["diagnostics"].as_array().unwrap().len(), 7);
    assert!(report["freshness"].as_array().unwrap().iter().all(|f| f["stale"] == false));
    let discord = &report["truncation"][0];
    assert_eq!(discord["adapter"], "discord");
    assert_eq!(discord["surfaces"][0]["surface"], "body");
    assert_eq!(discord["surfaces"][0]["enforceability"]["status"], "enforceable");

    let slack = json(&fleet.research(&["report", "--json", "--platform", "slack"]));
    let adapters: Vec<&str> = slack["truncation"].as_array().unwrap().iter().map(|h| h["adapter"].as_str().unwrap()).collect();
    assert_eq!(adapters, ["slack", "slack-webhook"]);
    assert_eq!(slack["truncation"][0]["service_side_loss"][0]["id"], "c.slack.text.truncation");

    let operation = json(&fleet.research(&["report", "--json", "--operation", "create_message"]));
    let constraints = operation["sections"].as_array().unwrap().iter().find(|s| s["kind"] == "constraints").unwrap();
    assert_eq!(constraints["rows"].as_array().unwrap().len(), 3);
    let interface = json(&fleet.research(&["report", "--json", "--interface", "slack_incoming_webhook"]));
    assert_eq!(interface["truncation"].as_array().unwrap().len(), 1);

    // Past the refresh date the snapshot is stale but still reported.
    let stale = research(Some(fleet.root()), &["--today", "2026-10-10", "report", "--json"]);
    let stale = json(&stale);
    assert!(stale["freshness"].as_array().unwrap().iter().all(|f| f["stale"] == true));

    let human = fleet.research(&["report"]);
    assert_eq!(code(&human), 0);
    let text = stdout(&human);
    for heading in ["Freshness", "Coverage", "Constraints", "Truncation handoff", "Diagnostic handoff", "Unresolved questions"] {
        assert!(text.contains(heading), "{heading} missing from the human report");
    }
    assert!(text.contains("c.discord.content.max"));
}

#[test]
fn the_human_report_renders_at_narrow_and_normal_widths() {
    let fleet = Fleet::new();
    assert_eq!(code(&fleet.research(&["generate"])), 0);
    let catalog = messenger::research::generate::published_catalog(&Workspace::new(fleet.root()).unwrap()).expect("catalog");
    let report = build(&catalog, &Filter::default(), &Date::parse("2026-09-17").unwrap());
    for width in [60, 120] {
        let term = Terminal::builder().is_tty(false).width(width).build();
        let text = messenger_cli::research::render_report(&report, &term);
        assert!(text.contains("Truncation handoff"), "width {width}");
        assert!(text.contains("discord-webhook"), "width {width}");
        if std::env::var_os("SHOW_RESEARCH_REPORT").is_some() {
            println!("---- width {width} ----\n{text}");
        }
    }
    let _ = fleet.read(SUMMARY);
}
