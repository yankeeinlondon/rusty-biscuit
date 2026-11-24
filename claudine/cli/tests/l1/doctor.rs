//! `claudine doctor` through the shipped binary: help and usage, the exit
//! status of each report shape, the JSON contract against the library's
//! rows, the human report's plain rendering, redaction, and that an
//! inspection of a missing or broken home changes nothing. Each acceptance
//! state (located YAML errors, a bundle Darkmatter refuses, unresolved,
//! repaired, and failed repairs) is asserted in both formats.
//!
//! Each row's own outcomes are the library's (`lib/src/doctor/*_tests.rs`);
//! these tests prove the command reaches them before configuration loads.
//! The `tts` and `path` rows read this host, so only their presence, order,
//! and (for `path`) the deliberate miss or hit are asserted here. On native
//! Windows every run but the healthy-home case skips the real shell probes
//! (`SKIP_SHELL_PROBES`): each probe adds up to a second of shell startup,
//! and their outcomes are the library's `path` tests.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use claudine::composition::registration::{
    MANIFEST_REFERENCE, PublishedFile, RegistrationInventory, legacy_manifest_path, legacy_revisions_dir, publish, registrations_dir,
};
use claudine::doctor::DoctorRow;
use darkmatter::markdown::compose::RequestSnapshot;
use serde_json::Value;

use crate::common;
use common::CliProcessFixture;

const IDS: [&str; 7] = ["config", "registration", "darkmatter-permissions", "repo-config", "stray-config-keys", "tts", "path"];

/// A Claudine configuration without `tts` (hook speech off, so the `tts`
/// row is never above `info`) and with its own `permissions` block.
const CLAUDINE_CONFIG: &str = "# mine\nlogging: false   # quiet\n\npermissions:\n  network: deny\n";

fn darkmatter_config() -> String {
    format!("# mine\npermissions: ~/.claudine/config.yaml\nregister:\n  - {MANIFEST_REFERENCE}\n")
}

/// A home with nothing for doctor to report: both files, the published
/// registration, and a `claudine` on the fixture `PATH`.
fn healthy(prefix: &str) -> CliProcessFixture {
    let fixture = CliProcessFixture::named(prefix);
    common::write(&fixture.home().join(".claudine/config.yaml"), CLAUDINE_CONFIG);
    common::write(&fixture.home().join(".darkmatter.yaml"), &darkmatter_config());
    publish(fixture.home(), RegistrationInventory::claudine()).expect("publish the registration");
    install_claudine_on_path(&fixture);
    fixture
}

/// The `path` row only looks the name up; it never runs what it finds.
fn install_claudine_on_path(fixture: &CliProcessFixture) {
    common::write_executable(&fixture.bin_dir().join(format!("claudine{}", std::env::consts::EXE_SUFFIX)), "#!/bin/sh\n");
}

/// The `test-fixtures` seam that keeps a child from probing the host's
/// Windows shells; it changes nothing on other OSes.
const SKIP_SHELL_PROBES: &str = "CLAUDINE_TEST_DOCTOR_SKIP_SHELL_PROBES";

struct Run {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

fn doctor(fixture: &CliProcessFixture, args: &[&str]) -> Run {
    let mut command = fixture.command();
    run(command.env(SKIP_SHELL_PROBES, "1").env("COLUMNS", "300").arg("doctor").args(args))
}

fn run(command: &mut assert_cmd::Command) -> Run {
    let output = command.write_stdin("").output().expect("claudine runs");
    Run {
        code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

fn rows(run: &Run) -> Vec<Value> {
    let parsed: Value = serde_json::from_str(&run.stdout).unwrap_or_else(|error| panic!("{error}: {}", run.stdout));
    let rows = parsed.as_array().expect("one JSON array").clone();
    let ids: Vec<&str> = rows.iter().map(|row| row["id"].as_str().unwrap()).collect();
    assert_eq!(ids, IDS, "one row per check, in order");
    for row in &rows {
        let keys: Vec<&str> = row.as_object().unwrap().keys().map(String::as_str).collect();
        assert_eq!(keys.len(), 5, "{row}");
        for key in ["id", "severity", "summary", "detail", "fixable"] {
            assert!(keys.contains(&key), "{key} missing from {row}");
        }
        assert!(row["fixable"].is_boolean() && row["detail"].is_string(), "{row}");
        assert!(!row["summary"].as_str().unwrap().contains('\n'), "{row}");
    }
    rows
}

fn row<'a>(rows: &'a [Value], id: &str) -> &'a Value {
    rows.iter().find(|row| row["id"] == id).unwrap()
}

fn severity(rows: &[Value], id: &str) -> String {
    row(rows, id)["severity"].as_str().unwrap().to_string()
}

/// Every file and directory under `root`, with each file's bytes. A writer
/// lock sidecar is listed without its (empty) bytes: Windows refuses to read
/// a file another handle holds locked.
fn tree(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    let mut entries = BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for child in std::fs::read_dir(&dir).unwrap() {
            let path = child.unwrap().path();
            if std::fs::symlink_metadata(&path).unwrap().is_dir() {
                pending.push(path.clone());
                entries.insert(path, None);
            } else if path.to_string_lossy().ends_with(darkmatter::config::write::LOCK_SUFFIX) {
                entries.insert(path, Some(Vec::new()));
            } else {
                entries.insert(path.clone(), Some(std::fs::read(&path).unwrap()));
            }
        }
    }
    entries
}

/// The human report's line for row `id` up to its summary.
fn head(id: &str, severity: &str) -> String {
    format!("[{severity}] {} ({id}): ", claudine::doctor::title_of(id).unwrap())
}

/// Runs doctor with `args` in both formats over the same, unchanged home:
/// the JSON rows, then the human report.
fn both_formats(fixture: &CliProcessFixture, args: &[&str]) -> (Run, Run) {
    let json = doctor(fixture, &[args, &["--json"]].concat());
    let human = doctor(fixture, args);
    (json, human)
}

// ------------------------------------------------------------- help, usage

#[test]
fn help_names_both_options_and_needs_no_configuration() {
    let fixture = CliProcessFixture::named("doctor-help").without_darkmatter_config();

    let help = doctor(&fixture, &["--help"]);

    assert_eq!(help.code, Some(0), "{}", help.stderr);
    for option in ["--fix", "--json"] {
        assert!(help.stdout.contains(option), "{option}:\n{}", help.stdout);
    }
    let overview = run(fixture.command().env("COLUMNS", "300").arg("--help"));
    assert_eq!(overview.code, Some(0), "{}", overview.stderr);
    let administration = overview.stdout.find("Administration:").expect(&overview.stdout);
    let listed = overview.stdout.find("doctor").expect(&overview.stdout);
    assert!(listed > administration, "{}", overview.stdout);
    assert!(tree(fixture.home()).is_empty(), "help writes nothing");
}

#[test]
fn an_unknown_option_is_a_usage_error_with_nothing_reported() {
    let fixture = CliProcessFixture::named("doctor-usage");

    let refused = doctor(&fixture, &["--repair"]);

    assert_eq!(refused.code, Some(2), "{}", refused.stderr);
    assert!(refused.stdout.is_empty(), "{}", refused.stdout);
    assert!(refused.stderr.contains("--repair"), "{}", refused.stderr);
}

// ---------------------------------------------------------- report shapes

#[test]
fn a_healthy_home_reports_every_row_and_exits_zero() {
    let fixture = healthy("doctor-healthy");
    fixture.initialize_repository();
    let before = tree(fixture.home());

    // The one spawned case with the host's real Windows shell probes; their
    // `ok` or `warn` findings never fail the run.
    let mut command = fixture.command();
    let report = run(command.env("COLUMNS", "300").args(["doctor", "--json"]));

    assert_eq!(report.code, Some(0), "{}\n{}", report.stdout, report.stderr);
    let rows = rows(&report);
    for id in ["config", "registration", "stray-config-keys"] {
        assert_eq!(severity(&rows, id), "ok", "{}", row(&rows, id));
    }
    // Without repository files, only the user scope's handoff is assessed.
    assert_eq!(severity(&rows, "darkmatter-permissions"), "info", "{}", row(&rows, "darkmatter-permissions"));
    assert!(row(&rows, "darkmatter-permissions")["detail"].as_str().unwrap().contains("refers to Claudine's configuration"));
    assert_eq!(severity(&rows, "repo-config"), "info", "an absent repository file is informational");
    assert!(["ok", "info"].contains(&severity(&rows, "tts").as_str()), "hook speech is off: {}", row(&rows, "tts"));
    assert!(["ok", "warn"].contains(&severity(&rows, "path").as_str()), "{}", row(&rows, "path"));
    assert!(rows.iter().all(|row| row["fixable"] == false), "{rows:#?}");
    // The fixture home is every probed shell's user profile, so this also
    // covers PowerShell's startup cache, which Sniff keeps out of it.
    assert_eq!(tree(fixture.home()), before, "an inspection writes nothing");
}

#[test]
fn warnings_alone_exit_zero_and_both_formats_show_the_library_rows() {
    let fixture = healthy("doctor-warnings");
    common::write(&legacy_manifest_path(fixture.home()), "kind: darkmatter-host\nformat_version: 1\n");

    let report = doctor(&fixture, &["--json"]);

    assert_eq!(report.code, Some(0), "{}\n{}", report.stdout, report.stderr);
    let rows = rows(&report);
    assert_eq!(severity(&rows, "registration"), "warn", "{}", row(&rows, "registration"));
    assert_eq!(row(&rows, "registration")["fixable"], true, "the leftover can be cleaned up");
    assert!(rows.iter().all(|row| row["severity"] != "error"), "{rows:#?}");

    // The file rows are this host's library result exactly; `tts` and
    // `path` read the process environment, which differs in the child.
    let snapshot = RequestSnapshot::new(fixture.cwd()).with_home(Some(fixture.home().to_path_buf()));
    let library: Vec<DoctorRow> = claudine::doctor::run(&snapshot, false);
    let library: Value = serde_json::to_value(&library).unwrap();
    assert_eq!(&rows[..5], &library.as_array().unwrap()[..5]);

    // The human report: the same rows, plain when redirected even with
    // color forced, and plain under `--plain`.
    for args in [&[][..], &["--plain"][..]] {
        let mut command = fixture.command();
        let human = run(command.env(SKIP_SHELL_PROBES, "1").env("COLUMNS", "300").env_remove("NO_COLOR").env("FORCE_COLOR", "1").arg("doctor").args(args));
        assert_eq!(human.code, Some(0), "{args:?}: {}", human.stderr);
        assert!(!human.stdout.contains('\u{1b}'), "{args:?} has ANSI:\n{}", human.stdout);
        let mut at = 0;
        for row in &rows {
            let head = format!("[{}] {} ({}): ", row["severity"].as_str().unwrap(), claudine::doctor::title_of(row["id"].as_str().unwrap()).unwrap(), row["id"].as_str().unwrap());
            let found = human.stdout[at..].find(&head).unwrap_or_else(|| panic!("{head} after byte {at}:\n{}", human.stdout));
            at += found + head.len();
        }
        let registration = row(&rows, "registration");
        for line in registration["detail"].as_str().unwrap().lines() {
            assert!(human.stdout.contains(line), "{line}:\n{}", human.stdout);
        }
        assert!(human.stdout.trim_end().ends_with("0 errors, 1 warning."), "{}", human.stdout);
    }
}

#[test]
fn inspecting_a_missing_home_reports_errors_and_creates_nothing() {
    let fixture = CliProcessFixture::named("doctor-missing").without_darkmatter_config();
    let missing = |args: &[&str]| {
        let mut command = fixture.command_builder()
            // The deliberate miss: nothing but the empty fixture `bin` is searched.
            .fake_only_path()
            .build();
        run(command.env(SKIP_SHELL_PROBES, "1").env("COLUMNS", "300").arg("doctor").args(args))
    };

    let (report, human) = (missing(&["--json"]), missing(&[]));

    assert_eq!(report.code, Some(1), "{}\n{}", report.stdout, report.stderr);
    let rows = rows(&report);
    for id in ["config", "registration", "path"] {
        assert_eq!(severity(&rows, id), "error", "{}", row(&rows, id));
    }
    assert!(row(&rows, "config")["detail"].as_str().unwrap().contains("claudine setup"), "{rows:#?}");
    assert!(tree(fixture.home()).is_empty(), "no configuration, backup, or registration is created");
    assert!(!fixture.audio_spool().exists(), "no worker or audio job is started");
    assert!(!report.stderr.contains("setup"), "no setup guidance or prompt:\n{}", report.stderr);
    assert_eq!(human.code, Some(1), "{}\n{}", human.stdout, human.stderr);
    for id in ["config", "registration", "path"] {
        assert!(human.stdout.contains(&head(id, "error")), "{id}:\n{}", human.stdout);
    }
    assert!(human.stdout.contains("claudine setup"), "{}", human.stdout);
    assert!(!human.stderr.contains("setup"), "no setup guidance or prompt:\n{}", human.stderr);
    assert!(tree(fixture.home()).is_empty(), "the human report creates nothing either");
}

#[test]
fn inspecting_a_broken_home_leaves_every_file_as_found() {
    let fixture = CliProcessFixture::named("doctor-broken");
    install_claudine_on_path(&fixture);
    // A legacy JSON configuration with no YAML beside it, and an invalid
    // Darkmatter file: neither is converted, rewritten, or refreshed.
    common::write(&fixture.home().join(".claudine/config.json"), "{ \"logging\": true }\n");
    common::write(&fixture.home().join(".darkmatter.yaml"), "register: [\n");
    let before = tree(fixture.home());

    let (report, human) = both_formats(&fixture, &[]);

    assert_eq!(report.code, Some(1), "{}\n{}", report.stdout, report.stderr);
    let rows = rows(&report);
    assert_eq!(severity(&rows, "config"), "error", "{}", row(&rows, "config"));
    assert!(row(&rows, "config")["detail"].as_str().unwrap().contains("config.json"), "{}", row(&rows, "config"));
    assert_eq!(human.code, Some(1), "{}\n{}", human.stdout, human.stderr);
    assert!(human.stdout.contains(&head("config", "error")) && human.stdout.contains("config.json"), "{}", human.stdout);
    assert_eq!(tree(fixture.home()), before, "nothing is converted, created, or published");
    assert!(!registrations_dir(fixture.home()).exists());
    assert!(!fixture.audio_spool().exists());
}

// ------------------------------------------------- remedies keep their word
//
// A `config` remedy names one command that reads the file; each runs here
// against the home doctor diagnosed and does what the remedy says.

/// The `config` row's detail of a JSON report.
fn config_detail(run: &Run) -> String {
    row(&rows(run), "config")["detail"].as_str().unwrap().to_string()
}

#[test]
fn the_legacy_remedy_is_claudine_setup_which_converts_the_file() {
    let fixture = CliProcessFixture::named("doctor-legacy-remedy");
    common::write(&fixture.home().join(".claudine/config.json"), "{ \"logging\": true }\n");
    let detail = config_detail(&doctor(&fixture, &["--json"]));
    assert!(detail.contains("Remedy: run `claudine setup`, which converts it"), "{detail}");

    let setup = run(fixture.command().arg("setup"));

    assert_eq!(setup.code, Some(0), "{}\n{}", setup.stdout, setup.stderr);
    let converted = std::fs::read_to_string(fixture.home().join(".claudine/config.yaml")).expect("setup converted it");
    assert!(converted.contains("logging: true"), "{converted}");
    assert!(!fixture.home().join(".claudine/config.json").exists());
}

#[test]
fn the_user_file_remedy_names_claudine_actions_which_prints_the_located_error() {
    let fixture = CliProcessFixture::named("doctor-user-full-message");
    common::write(&fixture.home().join(".claudine/config.yaml"), "logging: [\n");
    let detail = config_detail(&doctor(&fixture, &["--json"]));
    assert!(detail.contains("Remedy: correct it; `claudine actions` prints the full message"), "{detail}");

    let actions = run(fixture.command().env("COLUMNS", "300").arg("actions"));

    assert_eq!(actions.code, Some(1), "{}\n{}", actions.stdout, actions.stderr);
    assert!(actions.stderr.contains("config.yaml:2:1") && actions.stderr.contains("not valid YAML"), "{}", actions.stderr);
}

/// A load checks the configuration rules but never compiles a mapper:
/// `claudine actions` stops on an unknown sound and lists a broken mapper's
/// action as if it were fine, so only the sound's remedy names it.
#[test]
fn claudine_actions_reports_a_broken_rule_but_not_a_broken_mapper() {
    let fixture = CliProcessFixture::named("doctor-sound-remedy");
    common::write(&fixture.home().join(".claudine/config.yaml"), "default_sounds:\n  success: SECRET-SOUND\n");
    let detail = config_detail(&doctor(&fixture, &["--json"]));
    assert!(detail.contains("Remedy: correct the setting; `claudine actions` prints the full message"), "{detail}");
    let actions = run(fixture.command().env("COLUMNS", "300").arg("actions"));
    assert_eq!(actions.code, Some(1), "{}\n{}", actions.stdout, actions.stderr);
    assert!(actions.stderr.contains("default_sounds.success"), "{}", actions.stderr);

    let fixture = CliProcessFixture::named("doctor-mapper-remedy");
    common::write(
        &fixture.home().join(".claudine/config.yaml"),
        "actions:\n  before_tool:\n    - call:\n        command: check\n        mapper: { type: regex, pattern: \"(SECRET\" }\n",
    );
    let detail = config_detail(&doctor(&fixture, &["--json"]));
    assert!(
        detail.contains("Remedy: correct `actions.before_tool[0].call.mapper.pattern` so it is a valid regular expression"),
        "{detail}"
    );
    assert!(!detail.contains("claudine actions"), "{detail}");
    let actions = run(fixture.command().env("COLUMNS", "300").arg("actions"));
    assert_eq!(actions.code, Some(0), "{}\n{}", actions.stdout, actions.stderr);
    assert!(!actions.stderr.contains("regex") && !actions.stderr.contains("mapper"), "{}", actions.stderr);
}

/// `claudine setup` performs only the user handoff, so the repository's
/// remedy is doctor's own repair, which performs it.
#[test]
fn a_repository_handoff_is_left_by_setup_and_repaired_by_doctor_fix() {
    let fixture = healthy("doctor-repository-handoff");
    assert!(common::init_git_repo(fixture.cwd()), "git init");
    common::write(&fixture.cwd().join(".claudine/config.yaml"), "# repo\nactive_messenger: null\n");
    let darkmatter = fixture.cwd().join(".darkmatter.yaml");
    common::write(&darkmatter, "permissions:\n  network: deny\n");
    let rows_before = rows(&doctor(&fixture, &["--json"]));
    let permissions = row(&rows_before, "darkmatter-permissions");
    let detail = permissions["detail"].as_str().unwrap();
    assert!(detail.contains("the repository `permissions` is an inline block"), "{permissions}");
    assert!(detail.contains("Remedy: run `claudine doctor --fix`") && !detail.contains("claudine setup"), "{permissions}");

    let setup = run(fixture.command().arg("setup"));
    assert_eq!(setup.code, Some(0), "{}\n{}", setup.stdout, setup.stderr);
    assert_eq!(std::fs::read_to_string(&darkmatter).unwrap(), "permissions:\n  network: deny\n", "setup leaves it");

    let fixed = doctor(&fixture, &["--json", "--fix"]);
    assert_eq!(fixed.code, Some(0), "{}\n{}", fixed.stdout, fixed.stderr);
    assert_eq!(std::fs::read_to_string(&darkmatter).unwrap(), "permissions: ./.claudine/config.yaml\n");
}

/// A repository `.darkmatter.yaml` whose reference names a missing file is
/// a located `config` error even without a repository Claudine file, where
/// the handoff is skipped; `--fix` repairs nothing.
#[test]
fn a_broken_repository_reference_without_a_claudine_file_is_a_located_error_in_both_formats() {
    let fixture = healthy("doctor-broken-repository-reference");
    assert!(common::init_git_repo(fixture.cwd()), "git init");
    let darkmatter = fixture.cwd().join(".darkmatter.yaml");
    common::write(&darkmatter, "permissions: ./policy.yaml\n");
    let located = "has a `permissions` reference to a file that does not exist (line 1, column 14)";
    let before = tree(fixture.cwd());

    for args in [&[][..], &["--fix"][..]] {
        let (json, human) = both_formats(&fixture, args);

        assert_eq!(json.code, Some(1), "{args:?}: {}\n{}", json.stdout, json.stderr);
        let rows = rows(&json);
        let config = row(&rows, "config");
        assert_eq!(config["severity"], "error", "{config}");
        assert!(config["detail"].as_str().unwrap().contains(&format!("{}: {located}", darkmatter.display())), "{config}");
        let handoff = row(&rows, "darkmatter-permissions")["detail"].as_str().unwrap().to_string();
        assert!(handoff.contains("repository scope skipped: this Claudine configuration does not exist"), "{handoff}");

        assert_eq!(human.code, Some(1), "{args:?}: {}", human.stderr);
        assert!(human.stdout.contains(&format!("{}: {located}", darkmatter.display())), "{}", human.stdout);
        assert!(!human.stdout.contains("policy.yaml") && !json.stdout.contains("policy.yaml"), "{}", human.stdout);
        assert_eq!(tree(fixture.cwd()), before, "{args:?}: nothing in the repository is written");
    }
}

/// The repository file is read only by a provider launch. The compiled
/// marker provider stands in for Claude Code, so a launch that got past the
/// file would leave its marker.
#[cfg(feature = "test-fixtures")]
#[test]
fn the_repository_file_remedy_names_claudine_claude_which_stops_before_the_provider() {
    let fixture = CliProcessFixture::named("doctor-repository-full-message");
    std::fs::copy(
        biscuit_test_harness::bin_exe!("claudine-fake-goose"),
        fixture.bin_dir().join(format!("claude{}", std::env::consts::EXE_SUFFIX)),
    )
    .expect("the fake provider is built with `test-fixtures`");
    let marker = fixture.home().join("provider-launched.txt");
    common::write(&fixture.home().join(".claudine/config.yaml"), CLAUDINE_CONFIG);
    // A real `git init`: the launch finds its repository through `git`.
    assert!(common::init_git_repo(fixture.cwd()), "git init");
    common::write(&fixture.cwd().join(".claudine/config.yaml"), "logging: [\n");
    let detail = config_detail(&doctor(&fixture, &["--json"]));
    assert!(
        detail.contains("Remedy: correct it; `claudine claude` run inside the repository stops with the full message"),
        "{detail}"
    );

    let launch = run(fixture.command().env("COLUMNS", "300").env("PROVIDER_MARKER", &marker).args(["claude", "hi"]));

    assert_eq!(launch.code, Some(1), "{}\n{}", launch.stdout, launch.stderr);
    assert!(launch.stderr.contains("config.yaml:2:1") && launch.stderr.contains("not valid YAML"), "{}", launch.stderr);
    assert!(!marker.exists(), "the provider never starts");
}

/// Doctor leaves a repository's legacy `config.json` in place, and the
/// launch its remedy names converts it.
#[cfg(feature = "test-fixtures")]
#[test]
fn a_repository_legacy_file_is_left_in_place_and_claudine_claude_converts_it() {
    let fixture = CliProcessFixture::named("doctor-repository-legacy");
    std::fs::copy(
        biscuit_test_harness::bin_exe!("claudine-fake-goose"),
        fixture.bin_dir().join(format!("claude{}", std::env::consts::EXE_SUFFIX)),
    )
    .expect("the fake provider is built with `test-fixtures`");
    common::write(&fixture.home().join(".claudine/config.yaml"), CLAUDINE_CONFIG);
    assert!(common::init_git_repo(fixture.cwd()), "git init");
    let legacy = fixture.cwd().join(".claudine/config.json");
    common::write(&legacy, "{ \"canonical_provider\": \"claude\" }");
    let yaml = fixture.cwd().join(".claudine/config.yaml");

    let detail = config_detail(&doctor(&fixture, &["--json", "--fix"]));
    assert!(
        detail.contains("Remedy: `claudine claude` with a prompt, run inside the repository, converts it before the provider starts"),
        "{detail}"
    );
    assert!(legacy.exists() && !yaml.exists(), "doctor converts nothing");

    let marker = fixture.home().join("provider-launched.txt");
    let launch = run(fixture.command().env("COLUMNS", "300").env("PROVIDER_MARKER", &marker).args(["claude", "hi"]));

    assert_eq!(launch.code, Some(0), "{}\n{}", launch.stdout, launch.stderr);
    assert!(!legacy.exists() && yaml.exists(), "{}", launch.stderr);
    assert!(marker.exists(), "the provider starts after the conversion");
}

// ------------------------------------------------------ located YAML errors

#[test]
fn a_duplicate_key_is_located_in_both_formats_and_changes_nothing() {
    let fixture = healthy("doctor-duplicate-key");
    common::write(&fixture.home().join(".claudine/config.yaml"), "logging: false\nlogging: true\npermissions:\n  network: deny\n");
    let before = tree(fixture.home());

    let (json, human) = both_formats(&fixture, &[]);

    assert_eq!(json.code, Some(1), "{}\n{}", json.stdout, json.stderr);
    let rows = rows(&json);
    let config = row(&rows, "config");
    assert_eq!(config["severity"], "error", "{config}");
    let location = "is not one valid YAML mapping (line 2, column 1)";
    // `~/.darkmatter.yaml` names the broken file, so its reference is broken too.
    assert_eq!(config["summary"], format!("{location} (+1 more)"), "{config}");
    let detail = config["detail"].as_str().unwrap();
    assert!(detail.contains("config.yaml: ") && detail.contains("(line 2, column 1)"), "{config}");
    assert!(detail.contains(".darkmatter.yaml: has a `permissions` reference to Claudine's configuration, which does not supply"), "{config}");
    assert_eq!(severity(&rows, "stray-config-keys"), "info", "an unreadable file is not inspected for keys");

    assert_eq!(human.code, Some(1), "{}", human.stderr);
    assert!(human.stdout.contains(&format!("{}{location}", head("config", "error"))), "{}", human.stdout);
    assert!(human.stdout.contains("config.yaml: ") && human.stdout.contains("(line 2, column 1)"), "{}", human.stdout);
    assert_eq!(tree(fixture.home()), before, "nothing is rewritten or created");
    assert!(!fixture.audio_spool().exists(), "no worker or audio job is started");
}

#[test]
fn a_nested_stray_key_is_located_in_both_formats_and_changes_nothing() {
    let fixture = healthy("doctor-nested-key");
    common::write(&fixture.home().join(".claudine/config.yaml"), "logging: false\npermissions:\n  network: deny\n  sockets: []\n");
    let before = tree(fixture.home());

    let (json, human) = both_formats(&fixture, &[]);

    assert_eq!(json.code, Some(1), "{}\n{}", json.stdout, json.stderr);
    let rows = rows(&json);
    assert_eq!(severity(&rows, "config"), "error", "{}", row(&rows, "config"));
    // Darkmatter refuses the block too, so the reference to it is broken.
    assert_eq!(row(&rows, "config")["summary"], "has 1 unknown key (+1 more)", "{}", row(&rows, "config"));
    let stray = row(&rows, "stray-config-keys");
    assert_eq!(stray["severity"], "error", "{stray}");
    let location = "unknown key `sockets` in `/permissions` (line 4, column 3)";
    assert_eq!(stray["summary"], location, "{stray}");
    assert!(stray["detail"].as_str().unwrap().contains(&format!("config.yaml: {location}")), "{stray}");

    assert_eq!(human.code, Some(1), "{}", human.stderr);
    assert!(human.stdout.contains(&format!("{}{location}", head("stray-config-keys", "error"))), "{}", human.stdout);
    assert!(human.stdout.contains(&format!("config.yaml: {location}")), "{}", human.stdout);
    assert_eq!(tree(fixture.home()), before, "nothing is rewritten or created");
    assert!(!fixture.audio_spool().exists(), "no worker or audio job is started");
}

#[test]
fn no_authored_value_reaches_either_report_or_stderr() {
    // A schema error quoting its value, and a broken permissions reference.
    let cases = [
        ("config", "logging: SECRET-SENTINEL-value\npermissions:\n  network: deny\n".to_string(), darkmatter_config()),
        (
            "darkmatter-permissions",
            CLAUDINE_CONFIG.to_string(),
            format!("permissions: ~/.SECRET-SENTINEL-reference.yaml\nregister:\n  - {MANIFEST_REFERENCE}\n"),
        ),
    ];
    for (id, claudine_config, darkmatter_text) in cases {
        let fixture = healthy("doctor-redaction");
        common::write(&fixture.home().join(".claudine/config.yaml"), &claudine_config);
        common::write(&fixture.home().join(".darkmatter.yaml"), &darkmatter_text);
        for args in [&["--json"][..], &[][..]] {
            let report = doctor(&fixture, args);
            assert_eq!(report.code, Some(1), "{id} {args:?}: {}\n{}", report.stdout, report.stderr);
            assert!(!report.stdout.contains("SECRET"), "{id} {args:?}:\n{}", report.stdout);
            assert!(!report.stderr.contains("SECRET"), "{id} {args:?}:\n{}", report.stderr);
            if args == ["--json"] {
                let rows = rows(&report);
                assert_eq!(severity(&rows, id), "error", "{}", row(&rows, id));
            }
        }
    }
}

// ------------------------------------------------- registration refusal

/// Publishes the shipped registration under `home`, then rewrites `member`
/// with `edit` and the manifest with that member's new digest, so every
/// digest agrees and only Darkmatter's loader can refuse the bundle. Shared
/// in shape with the library's `doctor/registration_tests.rs`; public API only.
fn publish_with_edited_member(home: &Path, member: &str, edit: impl FnOnce(&str) -> String) -> PathBuf {
    publish(home, RegistrationInventory::claudine()).unwrap();
    let root = registrations_dir(home).join("darkmatter");
    let path = member.split('/').fold(root.clone(), |path, part| path.join(part));
    let published = std::fs::read_to_string(&path).unwrap();
    let original = RegistrationInventory::claudine().file(member).unwrap_or_else(|| panic!("`{member}` is published"));
    let edited = RegistrationInventory::claudine().clone().with_member(PublishedFile::new(
        member.to_string(),
        original.kind(),
        original.name().map(str::to_string),
        edit(&published).into_bytes(),
    ));
    std::fs::write(&path, edited.file(member).unwrap().bytes()).unwrap();
    std::fs::write(root.join("manifest.yaml"), edited.manifest().bytes()).unwrap();
    path
}

#[test]
fn a_changed_bundle_darkmatter_refuses_is_an_error_in_both_formats_and_is_left_without_fix() {
    let fixture = healthy("doctor-refused-bundle");
    publish_with_edited_member(fixture.home(), "schemas/action.yaml", |text| format!("{text}\nbroken: [\n"));
    let before = tree(fixture.home());

    let (json, human) = both_formats(&fixture, &[]);

    assert_eq!(json.code, Some(1), "{}\n{}", json.stdout, json.stderr);
    let rows = rows(&json);
    let registration = row(&rows, "registration");
    assert_eq!(registration["severity"], "error", "{registration}");
    let refused = "Darkmatter refuses the bundle it lists as Claudine's registration";
    assert!(registration["summary"].as_str().unwrap().contains(refused), "{registration}");
    assert_eq!(registration["fixable"], true, "republishing repairs it: {registration}");
    assert_eq!(human.code, Some(1), "{}", human.stderr);
    assert!(human.stdout.contains(&head("registration", "error")) && human.stdout.contains(refused), "{}", human.stdout);
    assert_eq!(tree(fixture.home()), before, "an inspection repairs nothing");
}

// ----------------------------------------------------------------- repairs

/// A home both repairs apply to: the registration is unpublished and
/// unregistered, and the permissions block still sits in Darkmatter's file.
fn needs_both_repairs(prefix: &str) -> CliProcessFixture {
    let fixture = healthy(prefix);
    std::fs::remove_dir_all(registrations_dir(fixture.home())).unwrap();
    common::write(&fixture.home().join(".claudine/config.yaml"), "# mine\nlogging: false   # quiet\n");
    common::write(&fixture.home().join(".darkmatter.yaml"), "# theirs\npermissions:\n  network: deny\nregister: []\n");
    fixture
}

#[test]
fn a_repaired_home_exits_zero_and_a_second_run_changes_nothing() {
    let fixture = needs_both_repairs("doctor-repair");

    let (inspected, human) = both_formats(&fixture, &[]);
    assert_eq!(inspected.code, Some(1), "{}", inspected.stdout);
    let before = rows(&inspected);
    for id in ["registration", "darkmatter-permissions"] {
        assert_eq!(severity(&before, id), "error", "{}", row(&before, id));
        assert_eq!(row(&before, id)["fixable"], true, "{}", row(&before, id));
    }
    // The unresolved errors in the human report.
    assert_eq!(human.code, Some(1), "{}", human.stderr);
    for id in ["registration", "darkmatter-permissions"] {
        assert!(human.stdout.contains(&head(id, "error")), "{id}:\n{}", human.stdout);
    }
    assert!(human.stdout.trim_end().ends_with("2 errors, 0 warnings."), "{}", human.stdout);

    let repaired = doctor(&fixture, &["--fix", "--json"]);

    assert_eq!(repaired.code, Some(0), "{}\n{}", repaired.stdout, repaired.stderr);
    let after = rows(&repaired);
    for id in ["registration", "darkmatter-permissions"] {
        assert_eq!(severity(&after, id), "ok", "{}", row(&after, id));
        assert!(row(&after, id)["detail"].as_str().unwrap().contains("Repaired: "), "{}", row(&after, id));
    }
    let claudine_text = std::fs::read_to_string(fixture.home().join(".claudine/config.yaml")).unwrap();
    assert!(claudine_text.starts_with("# mine\nlogging: false   # quiet\n") && claudine_text.contains("network: deny"), "{claudine_text}");
    let darkmatter_text = std::fs::read_to_string(fixture.home().join(".darkmatter.yaml")).unwrap();
    assert!(darkmatter_text.starts_with("# theirs\n") && darkmatter_text.contains(MANIFEST_REFERENCE), "{darkmatter_text}");
    let settled = tree(fixture.home());

    let again = doctor(&fixture, &["--fix", "--json"]);

    assert_eq!(again.code, Some(0), "{}", again.stdout);
    assert!(rows(&again).iter().all(|row| !row["detail"].as_str().unwrap().contains("Repaired: ")), "{}", again.stdout);
    assert_eq!(tree(fixture.home()), settled, "a second repair is a no-op");
}

/// R4-F2: an inline `register` list the entry edit would have to append to
/// is not offered as a repair. Both formats name the file and the block-form
/// remedy before and after `--fix`, which changes nothing.
#[test]
fn an_inline_register_list_is_a_manual_registration_finding_before_and_after_fix() {
    let fixture = healthy("doctor-inline-register");
    let config = "# mine\npermissions: ~/.claudine/config.yaml\nregister: [~/.acme/manifest.yaml]\n";
    common::write(&fixture.home().join(".darkmatter.yaml"), config);
    let before = tree(fixture.home());
    let remedy = "rewrite the `register` list of `~/.darkmatter.yaml` in block form";

    for args in [&[][..], &["--fix"][..]] {
        let (json, human) = both_formats(&fixture, args);

        assert_eq!(json.code, Some(1), "{args:?}: {}\n{}", json.stdout, json.stderr);
        let rows = rows(&json);
        let registration = row(&rows, "registration");
        assert_eq!((registration["severity"].as_str(), registration["fixable"].as_bool()), (Some("error"), Some(false)), "{args:?}: {registration}");
        assert!(registration["summary"].as_str().unwrap().contains("no `register` entry names Claudine's manifest"), "{registration}");
        assert!(registration["detail"].as_str().unwrap().contains(remedy), "{args:?}: {registration}");
        assert!(!json.stdout.contains("Repaired") && !json.stdout.contains("~/.acme/"), "{args:?}: {}", json.stdout);
        assert_eq!(human.code, Some(1), "{args:?}: {}", human.stderr);
        assert!(human.stdout.contains(&head("registration", "error")) && human.stdout.contains(remedy), "{args:?}:\n{}", human.stdout);
        assert_eq!(tree(fixture.home()), before, "{args:?}: nothing is written");
    }
}

/// What Darkmatter reports about the registrations it reads from the
/// fixture's configuration, as `md compose` does: one line per diagnostic.
fn darkmatter_diagnostics(fixture: &CliProcessFixture) -> Vec<String> {
    use darkmatter::markdown::compose::{ComposeOptions, ComposeRequest};
    let snapshot = RequestSnapshot::new(fixture.cwd()).with_home(Some(fixture.home().to_path_buf()));
    let request = ComposeRequest::prepare(ComposeOptions::new(), &snapshot).expect("a request");
    darkmatter::config::registration::RegisteredHosts::load(request.config_snapshot(), request.resolution_context())
        .diagnostics()
        .iter()
        .map(|diagnostic| format!("{}: {}", diagnostic.file.display(), diagnostic.message))
        .collect()
}

/// R5-F1: a retired entry left beside the current one after the retired
/// files are gone is a fixable warning that Darkmatter also warns about;
/// `--fix` removes it, which clears Darkmatter's warning, and a second
/// `--fix` changes nothing.
#[test]
fn a_retired_register_entry_is_fixed_and_a_second_fix_changes_nothing() {
    let fixture = healthy("doctor-retired-entry");
    let kept = format!("{}  - ~/.acme/manifest.yaml\n", darkmatter_config());
    common::write(&fixture.home().join(".darkmatter.yaml"), &format!("{kept}  - ~/.claudine/darkmatter-host.yaml\n"));
    assert!(
        darkmatter_diagnostics(&fixture).iter().any(|line| line.contains("darkmatter-host.yaml")),
        "Darkmatter warns at the retired entry: {:#?}",
        darkmatter_diagnostics(&fixture)
    );

    let inspected = doctor(&fixture, &["--json"]);

    assert_eq!(inspected.code, Some(0), "warnings alone: {}", inspected.stdout);
    let registration = row(&rows(&inspected), "registration").clone();
    assert_eq!((registration["severity"].as_str(), registration["fixable"].as_bool()), (Some("warn"), Some(true)), "{registration}");
    assert!(registration["detail"].as_str().unwrap().contains("`register` entry 3 names the retired format 1 manifest"), "{registration}");
    assert!(!inspected.stdout.contains("darkmatter-host"), "no authored value: {}", inspected.stdout);

    let fixed = doctor(&fixture, &["--fix", "--json"]);

    assert_eq!(fixed.code, Some(0), "{}\n{}", fixed.stdout, fixed.stderr);
    assert_eq!(severity(&rows(&fixed), "registration"), "ok", "{}", fixed.stdout);
    assert_eq!(std::fs::read_to_string(fixture.home().join(".darkmatter.yaml")).unwrap(), kept);
    let remaining = darkmatter_diagnostics(&fixture);
    assert!(!remaining.iter().any(|line| line.contains("darkmatter-host.yaml")), "{remaining:#?}");
    let settled = tree(fixture.home());

    let again = doctor(&fixture, &["--fix", "--json"]);

    assert_eq!(again.code, Some(0), "{}", again.stdout);
    assert!(!again.stdout.contains("Repaired: "), "{}", again.stdout);
    assert_eq!(tree(fixture.home()), settled, "a second repair is a no-op");
}

/// Starts doctor with `args` without waiting for it, so two runs can wait
/// out the same held lock together.
fn start_doctor(fixture: &CliProcessFixture, args: &[&str]) -> std::process::Child {
    let mut command = fixture.command_std();
    command.env(SKIP_SHELL_PROBES, "1").env("COLUMNS", "300").arg("doctor").args(args);
    command.stdin(std::process::Stdio::null()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped());
    command.spawn().expect("claudine starts")
}

fn finish(child: std::process::Child) -> Run {
    let output = child.wait_with_output().expect("claudine runs");
    Run {
        code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

/// A home whose registration repair must update `.darkmatter.yaml` while
/// the returned handle holds that file's Darkmatter writer lock, with
/// format 1 leftovers the repair would remove afterwards.
fn locked_home(prefix: &str) -> (CliProcessFixture, PathBuf, std::fs::File) {
    let fixture = healthy(prefix);
    let darkmatter = fixture.home().join(".darkmatter.yaml");
    common::write(&darkmatter, "permissions: ~/.claudine/config.yaml\nregister: []\n");
    common::write(&legacy_manifest_path(fixture.home()), "kind: darkmatter-host\nformat_version: 1\n");
    common::write(&legacy_revisions_dir(fixture.home()).join("0123abcd/globals.yaml"), "kind: schema\n");
    let mut sidecar = darkmatter.clone().into_os_string();
    sidecar.push(darkmatter::config::write::LOCK_SUFFIX);
    let lock = std::fs::OpenOptions::new().read(true).write(true).create(true).truncate(false).open(&sidecar).unwrap();
    lock.lock().expect("the parent holds Darkmatter's writer lock");
    (fixture, darkmatter, lock)
}

#[test]
fn a_repair_blocked_by_a_held_lock_fails_in_both_formats_keeps_every_file_and_a_retry_repairs_it() {
    // One home per format, so the two runs wait out Darkmatter's 2 s lock
    // deadline at the same time instead of queueing on Claudine's publisher
    // lock.
    let (json_home, _, json_lock) = locked_home("doctor-repair-locked-json");
    let (human_home, darkmatter, human_lock) = locked_home("doctor-repair-locked-human");
    let (json_before, human_before) = (tree(json_home.home()), tree(human_home.home()));

    let (json, human) = (start_doctor(&json_home, &["--fix", "--json"]), start_doctor(&human_home, &["--fix"]));
    let (json, human) = (finish(json), finish(human));
    for lock in [json_lock, human_lock] {
        lock.unlock().unwrap();
    }

    assert_eq!(json.code, Some(1), "{}\n{}", json.stdout, json.stderr);
    let rows = rows(&json);
    let registration = row(&rows, "registration");
    assert_eq!(registration["severity"], "error", "{registration}");
    assert!(registration["summary"].as_str().unwrap().starts_with("the repair failed: "), "{registration}");
    let detail = registration["detail"].as_str().unwrap();
    assert!(detail.contains("Repair failed: ") && detail.contains("held its lock"), "{registration}");
    assert_eq!(registration["fixable"], true, "a retry can still repair it: {registration}");
    assert_eq!(tree(json_home.home()), json_before, "the entry and the legacy files are kept");

    assert_eq!(human.code, Some(1), "{}\n{}", human.stdout, human.stderr);
    assert!(human.stdout.contains(&format!("{}the repair failed: ", head("registration", "error"))), "{}", human.stdout);
    assert!(human.stdout.contains("Repair failed: ") && human.stdout.contains("held its lock"), "{}", human.stdout);
    assert_eq!(tree(human_home.home()), human_before, "the entry and the legacy files are kept");

    // With the lock released, the same repair succeeds.
    let retried = doctor(&human_home, &["--fix"]);

    assert_eq!(retried.code, Some(0), "{}\n{}", retried.stdout, retried.stderr);
    assert!(retried.stdout.contains(&head("registration", "ok")), "{}", retried.stdout);
    assert!(retried.stdout.contains("Repaired: "), "{}", retried.stdout);
    assert!(!retried.stdout.contains("[error]"), "{}", retried.stdout);
    assert!(std::fs::read_to_string(&darkmatter).unwrap().contains(MANIFEST_REFERENCE));
    assert!(!legacy_manifest_path(human_home.home()).exists(), "the legacy files are cleaned up");
}

// ------------------------------------------------------------ write errors

#[cfg(target_os = "linux")]
#[test]
fn a_failed_write_to_stdout_is_an_error() {
    let fixture = healthy("doctor-stdout-full");
    let full = std::fs::OpenOptions::new().write(true).open("/dev/full").unwrap();
    let mut command = fixture.command_std();
    let output = command.args(["doctor", "--json"]).env(SKIP_SHELL_PROBES, "1").stdout(full).stdin(std::process::Stdio::null()).output().unwrap();

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Error:") && stderr.to_lowercase().contains("space"), "{stderr}");
}
