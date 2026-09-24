//! End-to-end contracts for the kache step of `just init` (`_ensure-kache`,
//! fixes/2026-09-23-ensuring-kache-support spec §3–§4): the order of the
//! host-mutating steps, the two daemon-restart triggers, idempotence, the
//! failure contract after a failed restart or one that leaves the old daemon
//! version running, the incomplete-activation report
//! when a higher-precedence wrapper outranks the host entry, and the
//! non-qualifying host's
//! below-floor branches — without a TTY and answered through a PTY.
//!
//! Each test runs the real recipe on an isolated fixture host
//! (`common::kache`) whose `kache`, `cargo`, `codesign`, clone probe, and
//! config writes all append to one ordered command log, so order is read from
//! what ran rather than from the justfile's text. Neighbors: the report-only
//! and failure-path cases are in `kache_ensure_contracts.rs`, each status
//! drift in `kache_status_contracts.rs`. The real macOS re-sign and a real
//! daemon stay a host smoke check (spec Verification 2, 5, 7), not L1.
#![cfg(unix)]

mod common;

use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use common::kache::{FakeDaemon, InvalidWorktreeBase, KacheHostFixture, RepoInputs};

fn repo_root() -> PathBuf {
    let manifest_dir = biscuit_test_harness::manifest_dir!();
    manifest_dir
        .parent()
        .and_then(|path| path.parent())
        .expect("test-toolkit must live under <repo>/tools/test-toolkit")
        .to_path_buf()
}

fn repo_inputs() -> RepoInputs {
    let root = repo_root();
    RepoInputs {
        justfile: root.join("justfile"),
        just_dir: root.join("just"),
        scripts: vec![
            root.join("scripts/cargo-path.sh"),
            root.join("scripts/kache-config-merge.py"),
            root.join("scripts/kache-host.sh"),
        ],
        floor_file: root.join(".github/kache-min-version"),
    }
}

fn floor() -> String {
    fs::read_to_string(repo_root().join(".github/kache-min-version"))
        .expect("floor file")
        .trim()
        .to_owned()
}

fn run(fixture: &KacheHostFixture, recipe: &str) -> (Option<i32>, String) {
    let output = fixture.just(recipe);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr);
    (
        output.status.code(),
        format!("{stdout}\n--- stderr ---\n{stderr}"),
    )
}

/// A qualifying host that has never seen kache: no binary, no user config,
/// no daemon service.
fn fresh_host() -> KacheHostFixture {
    let fixture = KacheHostFixture::new(&repo_inputs());
    fixture.uninstall_kache();
    fs::remove_file(fixture.kache_config()).expect("remove kache config");
    fixture.set_daemon(&FakeDaemon {
        service_installed: false,
        running: false,
        version: None,
    });
    fixture
}

fn running_daemon(version: &str) -> FakeDaemon {
    FakeDaemon {
        service_installed: true,
        running: true,
        version: Some(version.to_owned()),
    }
}

/// The spec §4 step a logged command belongs to, or `None` for bookkeeping
/// reads (`kache --version`, `daemon --json`, the passthrough probe's runs).
fn step(command: &str, fixture: &KacheHostFixture) -> Option<&'static str> {
    let kache_config = fixture.kache_config();
    let kache_config = kache_config.display();
    let cargo_config = fixture.cargo_home().join("config.toml");
    let cargo_config = cargo_config.display();
    if command.starts_with("clone ") {
        Some("qualify")
    } else if command.starts_with("cargo binstall ") {
        Some("install")
    } else if command.starts_with("codesign ") && command.ends_with("/kache") {
        Some("re-sign")
    } else if command == "kache doctor --json" {
        Some("doctor")
    } else if command.starts_with(&format!("config-merge {kache_config} cache ")) {
        Some("config write")
    } else if command == "kache daemon install" {
        Some("daemon install")
    } else if command == "kache daemon start" {
        Some("daemon start")
    } else if command == "kache daemon restart" {
        Some("daemon restart")
    } else if command == format!("config-merge {cargo_config} build rustc-wrapper kache") {
        Some("activation")
    } else {
        None
    }
}

/// The steps that ran, in order, each listed at its first occurrence.
fn steps(fixture: &KacheHostFixture) -> Vec<&'static str> {
    let mut seen = Vec::new();
    for command in fixture.commands() {
        if let Some(step) = step(&command, fixture)
            && !seen.contains(&step)
        {
            seen.push(step);
        }
    }
    seen
}

fn ran(fixture: &KacheHostFixture, wanted: &str) -> bool {
    fixture
        .commands()
        .iter()
        .any(|command| step(command, fixture) == Some(wanted))
}

fn cargo_wrapper(fixture: &KacheHostFixture) -> Option<String> {
    let text = fs::read_to_string(fixture.cargo_home().join("config.toml")).ok()?;
    let config: toml::Table = text.parse().expect("Cargo config parses");
    config
        .get("build")?
        .get("rustc-wrapper")?
        .as_str()
        .map(str::to_owned)
}

fn kache_cache_table(fixture: &KacheHostFixture) -> toml::Table {
    let text = fs::read_to_string(fixture.kache_config()).expect("kache config exists");
    let config: toml::Table = text.parse().expect("kache config parses");
    config
        .get("cache")
        .and_then(toml::Value::as_table)
        .cloned()
        .expect("kache config has a [cache] table")
}

/// Spec §4 on a fresh qualifying host: qualify → install → (macOS re-sign) →
/// doctor → config write → daemon install → daemon start → activation, with
/// activation the last host mutation; the result pins the store with
/// `ignore_env = true`, activates through `$CARGO_HOME`, and `kache-status`
/// then calls the host healthy.
#[test]
fn ensure_sets_up_a_fresh_qualifying_host_in_spec_order() {
    let fixture = fresh_host();

    let (code, output) = run(&fixture, "_ensure-kache");

    assert_eq!(code, Some(0), "init succeeds:\n{output}");
    let mut expected = vec!["qualify", "install"];
    if cfg!(target_os = "macos") {
        // The binstall release is hardened; install-kache re-signs it.
        expected.push("re-sign");
    }
    expected.extend([
        "doctor",
        "config write",
        "daemon install",
        "daemon start",
        "activation",
    ]);
    assert_eq!(
        steps(&fixture),
        expected,
        "commands: {:#?}\n{output}",
        fixture.commands()
    );
    let commands = fixture.commands();
    let last_mutation = commands
        .iter()
        .rposition(|command| {
            step(command, &fixture).is_some_and(|step| step != "doctor" && step != "qualify")
        })
        .expect("mutations ran");
    assert_eq!(
        step(&commands[last_mutation], &fixture),
        Some("activation"),
        "activation is the last host mutation: {commands:#?}"
    );
    assert!(
        !ran(&fixture, "daemon restart"),
        "a daemon started after the config write needs no restart: {commands:#?}"
    );

    let cache = kache_cache_table(&fixture);
    assert_eq!(
        cache.get("local_store").and_then(toml::Value::as_str),
        Some(fixture.store().to_str().expect("utf-8 store")),
        "the store kache resolves is pinned"
    );
    assert_eq!(
        cache.get("ignore_env").and_then(toml::Value::as_bool),
        Some(true)
    );
    assert_eq!(cargo_wrapper(&fixture).as_deref(), Some("kache"));
    assert_eq!(fixture.daemon_version(), Some(floor()));
    assert!(output.contains(SET_UP), "activation is reported:\n{output}");
    assert!(!output.contains(NOT_ACTIVE), "{output}");

    let (code, status) = run(&fixture, "kache-status");
    assert_eq!(code, Some(0), "the state init leaves is healthy:\n{status}");
    assert!(
        status.contains("VERDICT: active on a filesystem that clones blocks"),
        "{status}"
    );
}

/// Restart trigger 1: the running daemon reports another version than the
/// installed binary while the config is already ratified.
#[test]
fn ensure_restarts_a_daemon_running_another_version_before_activation() {
    let fixture = KacheHostFixture::new(&repo_inputs());
    fixture.set_daemon(&running_daemon("0.0.1"));
    let config_before = fs::read(fixture.kache_config()).expect("kache config");

    let (code, output) = run(&fixture, "_ensure-kache");

    assert_eq!(code, Some(0), "{output}");
    assert_eq!(
        fs::read(fixture.kache_config()).expect("kache config"),
        config_before,
        "the config was already ratified, so the version is the only trigger"
    );
    assert!(
        output.contains(&format!(
            "restarting the daemon — binary changed (daemon reports 0.0.1, installed {})",
            floor()
        )),
        "{output}"
    );
    assert!(!output.contains("config changed"), "{output}");
    let order = steps(&fixture);
    assert_eq!(
        order[order.len() - 2..],
        ["daemon restart", "activation"],
        "{output}"
    );
    assert_eq!(fixture.daemon_version(), Some(floor()));
}

/// Restart trigger 2: the config write changed the file while the daemon
/// already runs the installed version.
#[test]
fn ensure_restarts_the_daemon_when_the_config_write_changed_the_file() {
    let fixture = KacheHostFixture::new(&repo_inputs());
    fixture.write_kache_config(&format!(
        "[cache]\nlocal_store = \"{}\"\n",
        fixture.store().display()
    ));

    let (code, output) = run(&fixture, "_ensure-kache");

    assert_eq!(code, Some(0), "{output}");
    assert!(
        output.contains("restarting the daemon — config changed."),
        "the version matches, so the config change is the only trigger:\n{output}"
    );
    let order = steps(&fixture);
    assert_eq!(
        order[order.len() - 3..],
        ["config write", "daemon restart", "activation"],
        "{output}"
    );
}

/// Every file under `dir` with its bytes and modification time.
fn snapshot(dir: &Path) -> Vec<(PathBuf, Vec<u8>, SystemTime)> {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .expect("snapshot dir")
        .map(|entry| {
            let path = entry.expect("dir entry").path();
            let bytes = fs::read(&path).expect("read snapshot file");
            let modified = fs::metadata(&path)
                .and_then(|metadata| metadata.modified())
                .expect("mtime");
            (path, bytes, modified)
        })
        .collect();
    entries.sort();
    entries
}

fn verdict_lines(output: &str) -> Vec<&str> {
    output
        .lines()
        .filter(|line| line.starts_with("kache-host: verdict=") || line.contains("  verdict "))
        .collect()
}

/// Spec Verification 5: a second init changes nothing — no reinstall or
/// re-sign, no daemon work, no config rewrite or backup — and reports the
/// same verdict.
#[test]
fn ensure_is_idempotent_on_a_second_run() {
    let fixture = fresh_host();
    let (code, first) = run(&fixture, "_ensure-kache");
    assert_eq!(code, Some(0), "{first}");
    let kache_config_dir = fixture.kache_config().parent().expect("dir").to_path_buf();
    let before = (snapshot(&kache_config_dir), snapshot(&fixture.cargo_home()));
    fixture.clear_command_logs();

    let (code, second) = run(&fixture, "_ensure-kache");

    assert_eq!(code, Some(0), "{second}");
    assert_eq!(
        (snapshot(&kache_config_dir), snapshot(&fixture.cargo_home())),
        before,
        "no config file was rewritten or backed up:\n{second}"
    );
    for mutation in [
        "re-sign",
        "daemon install",
        "daemon start",
        "daemon restart",
    ] {
        assert!(
            !ran(&fixture, mutation),
            "a second run repeats no {mutation}: {:#?}",
            fixture.commands()
        );
    }
    assert!(
        second.contains("is already the latest"),
        "binstall ran as a no-op:\n{second}"
    );
    assert!(!second.contains("restarting the daemon"), "{second}");
    assert_eq!(verdict_lines(&second), verdict_lines(&first));
    assert!(!verdict_lines(&second).is_empty(), "{second}");
}

/// The failure contract after a failed restart: no activation, a WARNING,
/// kache left off, and init continues (exit 0).
#[test]
fn ensure_leaves_kache_off_when_the_daemon_restart_fails() {
    let fixture = KacheHostFixture::new(&repo_inputs());
    fixture.set_daemon(&running_daemon("0.0.1"));
    fixture.fail_daemon_restart();

    let (code, output) = run(&fixture, "_ensure-kache");

    assert_eq!(code, Some(0), "init continues past the failure:\n{output}");
    assert!(ran(&fixture, "daemon restart"), "{output}");
    assert!(
        !ran(&fixture, "activation"),
        "a failed restart never reaches activation: {:#?}",
        fixture.commands()
    );
    assert_eq!(cargo_wrapper(&fixture), None, "no wrapper was written");
    assert!(
        output.contains("WARNING") && output.contains("'kache daemon restart' failed."),
        "{output}"
    );
    assert!(output.contains("kache left OFF"), "{output}");
    assert!(!output.contains("qualified and set up"), "{output}");
}

/// A restart the service manager reports as done while the old daemon keeps
/// answering is a pre-activation failure: the daemon must run the installed
/// version before the wrapper is written. The synchronous-restart control is
/// `ensure_restarts_a_daemon_running_another_version_before_activation`.
#[test]
fn ensure_leaves_kache_off_when_the_restarted_daemon_keeps_the_old_version() {
    let fixture = KacheHostFixture::new(&repo_inputs());
    fixture.set_daemon(&running_daemon("0.0.1"));
    fixture.keep_daemon_on_restart();

    let (code, output) = run(&fixture, "_ensure-kache");

    assert_eq!(code, Some(0), "init continues past the failure:\n{output}");
    assert!(ran(&fixture, "daemon restart"), "{output}");
    assert_eq!(fixture.daemon_version().as_deref(), Some("0.0.1"));
    assert!(
        !ran(&fixture, "activation"),
        "a daemon at the old version never reaches activation: {:#?}",
        fixture.commands()
    );
    assert_eq!(cargo_wrapper(&fixture), None, "no wrapper was written");
    assert!(
        output.contains("WARNING")
            && output.contains(&format!(
                "the kache daemon still runs version 0.0.1, not the installed {}, after restart",
                floor()
            )),
        "{output}"
    );
    assert!(output.contains("kache left OFF"), "{output}");
    assert!(!output.contains("qualified and set up"), "{output}");
}

const SET_UP: &str = "kache: qualified and set up —";
const INCOMPLETE: &str = "kache: qualified, but activation INCOMPLETE";
const NOT_ACTIVE: &str = "kache NOT ACTIVE";

/// Runs init on an otherwise healthy qualifying host whose `setup` adds a
/// wrapper that outranks `$CARGO_HOME/config.toml`, and asserts the shared
/// outcome: the host entry is still written and kept (it applies once the
/// override goes), init still exits 0, and the report says activation is
/// incomplete instead of claiming success. Returns the output for the
/// source-specific undo line.
fn ensure_with_overriding_wrapper(setup: impl FnOnce(&mut KacheHostFixture)) -> String {
    let mut fixture = KacheHostFixture::new(&repo_inputs());
    setup(&mut fixture);

    let (code, output) = run(&fixture, "_ensure-kache");

    assert_eq!(code, Some(0), "init continues past the override:\n{output}");
    assert!(ran(&fixture, "activation"), "{output}");
    assert_eq!(
        cargo_wrapper(&fixture).as_deref(),
        Some("kache"),
        "the host entry is kept for when the override goes:\n{output}"
    );
    assert!(
        !output.contains(SET_UP),
        "an overridden activation is not reported as set up:\n{output}"
    );
    assert!(output.contains(INCOMPLETE), "{output}");
    assert!(
        output.contains("written, but NOT in effect"),
        "the activation line says it is not in effect:\n{output}"
    );
    assert!(
        output.contains("WARNING") && output.contains(NOT_ACTIVE),
        "{output}"
    );
    output
}

/// `RUSTC_WRAPPER=""` means "no wrapper" over every config file.
#[test]
fn ensure_reports_activation_incomplete_under_an_empty_rustc_wrapper() {
    let output = ensure_with_overriding_wrapper(|fixture| fixture.set_rustc_wrapper_env(""));

    assert!(
        output.contains("environment RUSTC_WRAPPER=\"\" — undo: 'unset RUSTC_WRAPPER'"),
        "{output}"
    );
}

#[test]
fn ensure_reports_activation_incomplete_under_a_foreign_rustc_wrapper() {
    let output = ensure_with_overriding_wrapper(|fixture| fixture.set_rustc_wrapper_env("sccache"));

    assert!(
        output.contains("environment RUSTC_WRAPPER=\"sccache\" — undo: 'unset RUSTC_WRAPPER'"),
        "{output}"
    );
}

#[test]
fn ensure_reports_activation_incomplete_under_cargo_build_rustc_wrapper() {
    let output = ensure_with_overriding_wrapper(|fixture| {
        fixture.set_cargo_build_rustc_wrapper_env("sccache");
    });

    assert!(
        output.contains(
            "environment CARGO_BUILD_RUSTC_WRAPPER=\"sccache\" — undo: 'unset CARGO_BUILD_RUSTC_WRAPPER'"
        ),
        "{output}"
    );
}

#[test]
fn ensure_reports_activation_incomplete_under_a_repository_config_wrapper() {
    let mut repo_config = PathBuf::new();
    let output = ensure_with_overriding_wrapper(|fixture| {
        repo_config = fixture.write_checkout_config_wrapper("sccache");
    });

    assert!(
        output.contains(&format!(
            "{} sets \"sccache\" — undo: delete its [build] rustc-wrapper line",
            repo_config.display()
        )),
        "the repository config is named by absolute path:\n{output}"
    );
}

#[test]
fn ensure_reports_activation_incomplete_under_a_parent_directory_config_wrapper() {
    let mut parent_config = PathBuf::new();
    let output = ensure_with_overriding_wrapper(|fixture| {
        parent_config = fixture.write_parent_config_wrapper("sccache");
    });

    assert!(
        output.contains(&format!(
            "{} sets \"sccache\" — undo: delete its [build] rustc-wrapper line",
            parent_config.display()
        )),
        "{output}"
    );
}

/// A kache-named wrapper outranking the host entry that Cargo cannot start:
/// init used to classify it by name and report "qualified and set up".
#[test]
fn ensure_reports_activation_incomplete_under_a_missing_kache_wrapper_path() {
    let output = ensure_with_overriding_wrapper(|fixture| {
        fixture.set_rustc_wrapper_env("/definitely/missing/kache");
    });

    assert!(
        output.contains(
            "the rustc wrapper Cargo would run, /definitely/missing/kache (set by RUSTC_WRAPPER), does not exist"
        ),
        "{output}"
    );
    assert!(output.contains("undo: 'unset RUSTC_WRAPPER'"), "{output}");
}

/// Another executable named kache is not the binary init installed,
/// re-signed, and version- and passthrough-checked.
#[test]
fn ensure_reports_activation_incomplete_under_another_kache_executable() {
    let mut expected = String::new();
    let output = ensure_with_overriding_wrapper(|fixture| {
        let other = fixture.install_other_kache();
        let config = fixture.write_parent_config_wrapper(&other.display().to_string());
        expected = format!(
            "the rustc wrapper Cargo would run, {} (set by {}), is not the kache init verified ({})",
            other.display(),
            config.display(),
            fixture.kache_bin().display()
        );
    });

    assert!(output.contains(&expected), "{expected}\n{output}");
}

/// A legacy `$CARGO_HOME/config` hides a `config.toml` beside it from Cargo.
/// Init used to write the activation into `config.toml` and report success
/// while Cargo built without kache; it now writes the file Cargo reads, and
/// real Cargo runs the wrapper.
#[test]
fn ensure_activates_through_a_legacy_cargo_home_config() {
    let fixture = KacheHostFixture::new(&repo_inputs());
    fixture.write_legacy_cargo_config("[term]\ncolor = \"never\"\n");
    let ignored = fixture.cargo_home().join("config.toml");
    fs::write(&ignored, "[net]\nretry = 2\n").expect("write ignored config.toml");

    let (code, output) = run(&fixture, "_ensure-kache");

    assert_eq!(code, Some(0), "init succeeds:\n{output}");
    assert!(output.contains(SET_UP), "{output}");
    let legacy = fixture.cargo_home().join("config");
    assert!(
        output.contains(&format!(
            "[build] rustc-wrapper = \"kache\" in {}",
            legacy.display()
        )),
        "the activation names the file Cargo reads:\n{output}"
    );
    let config: toml::Table = fs::read_to_string(&legacy)
        .expect("legacy config")
        .parse()
        .expect("legacy config parses");
    assert_eq!(
        config["build"]["rustc-wrapper"].as_str(),
        Some("kache"),
        "{config}"
    );
    assert_eq!(config["term"]["color"].as_str(), Some("never"), "{config}");
    assert_eq!(
        fs::read_to_string(&ignored).expect("config.toml"),
        "[net]\nretry = 2\n",
        "the file Cargo ignores is left alone"
    );

    let (code, status) = run(&fixture, "kache-status");
    assert_eq!(code, Some(0), "{status}");
    assert!(
        status.contains(&format!("active       YES — {} (host-wide", legacy.display())),
        "{status}"
    );

    match fixture.cargo_check_runs_kache() {
        Some(ran) => assert!(ran, "real Cargo runs the wrapper init activated"),
        None => eprintln!("skipping the real-Cargo half: cargo could not be started"),
    }
}

/// A host whose checkout cannot be cloned into, with kache installed below
/// the floor and a newer release available.
fn non_qualifying_below_floor_host() -> KacheHostFixture {
    let fixture = KacheHostFixture::new(&repo_inputs());
    fixture.fail_clones_into(&["checkout"]);
    fixture.set_kache_version("0.1.0");
    fixture.set_daemon(&running_daemon("0.1.0"));
    fixture
}

fn assert_no_daemon_or_activation_work(fixture: &KacheHostFixture) {
    let commands = fixture.commands();
    assert!(
        !commands
            .iter()
            .any(|command| command.starts_with("kache daemon ")),
        "a non-qualifying host gets no daemon work: {commands:#?}"
    );
    assert!(
        !commands
            .iter()
            .any(|command| command.starts_with("config-merge ")),
        "a non-qualifying host gets no config write or activation: {commands:#?}"
    );
}

/// Spec §3: below the floor with no terminal, init refuses — an error exit,
/// and nothing installed.
#[test]
fn ensure_refuses_a_below_floor_upgrade_without_a_terminal() {
    let fixture = non_qualifying_below_floor_host();

    let (code, output) = run(&fixture, "_ensure-kache");

    assert_ne!(code, Some(0), "a non-interactive init errors:\n{output}");
    assert!(
        output.contains("refusing to upgrade or skip silently"),
        "{output}"
    );
    assert!(
        !fixture
            .commands()
            .iter()
            .any(|command| command.starts_with("cargo")),
        "nothing is installed: {:#?}",
        fixture.commands()
    );
    assert_no_daemon_or_activation_work(&fixture);
}

/// Answers the below-floor prompt through a PTY (stdin is a terminal, so the
/// recipe asks) and returns the recipe's exit code with its transcript.
fn answer_below_floor_prompt(fixture: &KacheHostFixture, answer: &str) -> (i32, String) {
    use expectrl::{Eof, Expect, Session, process::unix::WaitStatus};

    let mut session = Session::spawn(fixture.just_command("_ensure-kache")).expect("spawn in PTY");
    session.set_expect_timeout(Some(Duration::from_secs(60)));
    let prompt = session
        .expect("[y/N] ")
        .expect("the below-floor prompt appears on a terminal");
    let mut transcript = String::from_utf8_lossy(prompt.before()).into_owned();
    session.send_line(answer).expect("answer the prompt");
    let rest = session.expect(Eof).expect("the recipe finishes");
    // `Eof` matches everything left, so the tail is the match itself.
    transcript.push_str(&String::from_utf8_lossy(rest.get(0).unwrap_or_default()));
    let code = match session.get_process().wait().expect("wait for just") {
        WaitStatus::Exited(_, code) => code,
        other => panic!("just did not exit normally: {other:?}\n{transcript}"),
    };
    (code, transcript)
}

/// Spec §3: a confirmed upgrade is binary-only — binstall runs, and no daemon
/// work, config write, or activation follows.
#[test]
fn ensure_upgrades_binary_only_when_the_below_floor_prompt_is_confirmed() {
    let fixture = non_qualifying_below_floor_host();

    let (code, transcript) = answer_below_floor_prompt(&fixture, "y");

    assert_eq!(code, 0, "{transcript}");
    assert!(ran(&fixture, "install"), "{:#?}", fixture.commands());
    assert!(
        transcript.contains("binary-only mode: no config written, no daemon touched."),
        "{transcript}"
    );
    let version = run_kache_version(&fixture);
    assert_eq!(
        version,
        format!("kache {}", floor()),
        "upgraded to the release"
    );
    assert_no_daemon_or_activation_work(&fixture);
    assert_eq!(cargo_wrapper(&fixture), None);
}

/// Declining the prompt leaves the binary as it is.
#[test]
fn ensure_leaves_a_below_floor_install_when_the_prompt_is_declined() {
    let fixture = non_qualifying_below_floor_host();

    let (code, transcript) = answer_below_floor_prompt(&fixture, "n");

    assert_eq!(code, 0, "{transcript}");
    assert!(
        transcript.contains("below-floor install left as is"),
        "{transcript}"
    );
    assert!(
        !fixture
            .commands()
            .iter()
            .any(|command| command.starts_with("cargo")),
        "nothing is installed: {:#?}",
        fixture.commands()
    );
    assert_eq!(run_kache_version(&fixture), "kache 0.1.0");
    assert_no_daemon_or_activation_work(&fixture);
}

fn run_kache_version(fixture: &KacheHostFixture) -> String {
    let output = std::process::Command::new(fixture.kache_bin())
        .arg("--version")
        .output()
        .expect("run fake kache");
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

/// Spec §3: a non-qualifying host without kache never gets it installed.
#[test]
fn ensure_never_installs_kache_on_a_non_qualifying_host() {
    let fixture = KacheHostFixture::new(&repo_inputs());
    fixture.fail_clones_into(&["checkout"]);
    fixture.uninstall_kache();

    let (code, output) = run(&fixture, "_ensure-kache");

    assert_eq!(code, Some(0), "{output}");
    assert!(
        output.contains("kache: not installed — init leaves it that way"),
        "{output}"
    );
    assert!(
        !fixture
            .commands()
            .iter()
            .any(|command| command.starts_with("cargo") || command.starts_with("kache ")),
        "{:#?}",
        fixture.commands()
    );
    assert!(!fixture.kache_bin().exists());
}

/// A worktree-base setting `wt` refuses is named in init's report with its
/// rule and path — it used to read "unconfigured", or "covered" for a `WT`
/// naming a Git repository — while the base still does not gate
/// qualification (2026-09-23 ruling): the verdict, the activation, and the
/// healthy header are what a host with no base gets.
fn assert_ensure_reports_invalid_base_without_gating(setting: InvalidWorktreeBase) {
    let mut fixture = KacheHostFixture::new(&repo_inputs());
    let (reason, path) = fixture.configure_invalid_worktree_base(setting);

    let (code, output) = run(&fixture, "_ensure-kache");

    assert_eq!(code, Some(0), "{setting:?}:\n{output}");
    assert!(
        output.contains("kache-host: verdict=qualify candidate="),
        "the base does not change the verdict:\n{output}"
    );
    assert!(
        output.contains(&format!(
            "kache-host: base=invalid reason={reason} path={}\n",
            path.display()
        )),
        "qualify names the invalid setting:\n{output}"
    );
    assert!(
        output.contains(&format!(
            "worktree base  INVALID setting ({reason}: {}) — wt refuses it",
            path.display()
        )),
        "the report block names the invalid setting:\n{output}"
    );
    assert!(output.contains(SET_UP), "{output}");
    assert_eq!(cargo_wrapper(&fixture).as_deref(), Some("kache"), "{output}");
}

#[test]
fn ensure_reports_a_missing_wt_path_without_gating_qualification() {
    assert_ensure_reports_invalid_base_without_gating(InvalidWorktreeBase::MissingWtPath);
}

#[test]
fn ensure_reports_a_wt_git_repository_without_gating_qualification() {
    assert_ensure_reports_invalid_base_without_gating(InvalidWorktreeBase::WtGitRepository);
}

#[test]
fn ensure_reports_a_malformed_worktree_config_without_gating_qualification() {
    assert_ensure_reports_invalid_base_without_gating(InvalidWorktreeBase::MalformedConfig);
}

#[test]
fn ensure_reports_a_missing_config_base_dir_without_gating_qualification() {
    assert_ensure_reports_invalid_base_without_gating(InvalidWorktreeBase::MissingConfigBaseDir);
}

/// Writes a second kache config beside the managed one, pinning `store` with
/// `ignore_env = true` as `just init` would, and returns its path.
fn write_alternate_kache_config(fixture: &KacheHostFixture, store: &Path) -> PathBuf {
    let dir = fixture.home().join("alternate-kache");
    fs::create_dir_all(&dir).expect("alternate config directory");
    let config = dir.join("config.toml");
    fs::write(
        &config,
        format!(
            "[cache]\nlocal_store = \"{}\"\nignore_env = true\n",
            store.display()
        ),
    )
    .expect("write alternate kache config");
    config
}

/// Review 6: a shell exporting `KACHE_CONFIG` makes kache read that file, and
/// `ignore_env = true` does not stop the selection. Init used to pin a
/// clone-capable store in the managed file and activate, while every build
/// from that shell kept the alternate file's off-device store. Now the
/// override is named, no daemon work runs under it, the wrapper stays off,
/// and `kache-status` reports the same override as drift.
#[test]
fn ensure_leaves_kache_off_when_kache_config_selects_an_off_device_store() {
    let mut fixture = KacheHostFixture::new(&repo_inputs());
    let alternate_store = fixture.home().join("alternate-store");
    fs::create_dir_all(&alternate_store).expect("alternate store");
    fixture.place_off_device(&[&alternate_store]);
    let alternate = write_alternate_kache_config(&fixture, &alternate_store);
    fixture.set_kache_config_env(alternate.clone());

    let (code, output) = run(&fixture, "_ensure-kache");

    assert_eq!(code, Some(0), "init continues past the override:\n{output}");
    assert!(
        output.contains("WARNING")
            && output.contains(&format!(
                "this environment exports KACHE_CONFIG={}",
                alternate.display()
            )),
        "the warning names the override:\n{output}"
    );
    assert!(output.contains("kache left OFF"), "{output}");
    assert!(!output.contains("qualified and set up"), "{output}");
    assert!(
        !ran(&fixture, "activation"),
        "an overridden config never reaches activation: {:#?}",
        fixture.commands()
    );
    assert_eq!(cargo_wrapper(&fixture), None, "no wrapper was written");
    for mutation in ["daemon install", "daemon start", "daemon restart"] {
        assert!(
            !ran(&fixture, mutation),
            "no {mutation} runs under the alternate config: {:#?}",
            fixture.commands()
        );
    }
    let pinned = kache_cache_table(&fixture)
        .get("local_store")
        .and_then(toml::Value::as_str)
        .map(PathBuf::from);
    assert!(
        pinned.as_deref().is_some_and(|pinned| pinned != alternate_store),
        "the managed file still carries init's own placement, not the override's store: {pinned:?}"
    );

    // An activation left from an earlier run, as `kache-status` would find it.
    fixture.activate_wrapper();
    let (code, status) = run(&fixture, "kache-status");
    assert_eq!(code, Some(1), "the override is drift:\n{status}");
    assert!(status.contains("VERDICT: DRIFT"), "{status}");
    assert!(
        status.contains(&format!(
            "this shell exports KACHE_CONFIG={}, so kache reads that file, not {}",
            alternate.display(),
            fixture.kache_config().display()
        )),
        "status names the override:\n{status}"
    );
}

/// The daemon half of the single-source contract: a service whose launch
/// environment selects another config file serves another store even when
/// init's own shell reads the managed one. The daemon reports the file it
/// loaded (`daemon_config_path`); a foreign one keeps the wrapper off.
#[test]
fn ensure_leaves_kache_off_when_the_daemon_loaded_another_config() {
    let fixture = KacheHostFixture::new(&repo_inputs());
    let alternate = write_alternate_kache_config(&fixture, &fixture.store());
    fixture.set_daemon_service_config(&alternate);

    let (code, output) = run(&fixture, "_ensure-kache");

    assert_eq!(code, Some(0), "init continues past the override:\n{output}");
    assert!(
        output.contains("WARNING")
            && output.contains(&format!(
                "the running kache daemon loaded {} (daemon_config_path), not {}",
                alternate.display(),
                fixture.kache_config().display()
            )),
        "the warning names the daemon's config:\n{output}"
    );
    assert!(output.contains("kache left OFF"), "{output}");
    assert!(
        !ran(&fixture, "activation"),
        "a daemon on another config never reaches activation: {:#?}",
        fixture.commands()
    );
    assert_eq!(cargo_wrapper(&fixture), None, "no wrapper was written");
}
