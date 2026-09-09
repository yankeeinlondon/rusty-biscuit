mod common;

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Command;

use common::{SniffCliFixture, checkout_containment_error};

fn environment_recorder() -> Command {
    if cfg!(windows) {
        let command = std::env::var_os("COMSPEC").unwrap_or_else(|| OsString::from("cmd.exe"));
        let mut recorder = Command::new(command);
        recorder.args(["/D", "/C", "set"]);
        recorder
    } else {
        Command::new("/usr/bin/env")
    }
}

fn parse_environment(stdout: &[u8]) -> BTreeMap<OsString, OsString> {
    String::from_utf8_lossy(stdout)
        .lines()
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| {
            (
                OsString::from(key),
                OsString::from(value.trim_end_matches('\r')),
            )
        })
        .collect()
}

#[test]
fn assert_and_raw_surfaces_produce_the_same_effective_environment() {
    let fixture = SniffCliFixture::new();
    let builder = fixture.command_builder().inherit_no_env();

    let mut assert_surface = assert_cmd::Command::from_std(environment_recorder());
    builder.apply_policy_to(&mut assert_surface);
    let assert_output = assert_surface
        .output()
        .expect("run assert-surface recorder");
    assert!(assert_output.status.success());

    let mut raw_surface = environment_recorder();
    builder.apply_policy_to(&mut raw_surface);
    let raw_output = raw_surface.output().expect("run raw-surface recorder");
    assert!(raw_output.status.success());

    let assert_environment = parse_environment(&assert_output.stdout);
    let raw_environment = parse_environment(&raw_output.stdout);
    assert_eq!(assert_environment, raw_environment);
    assert_eq!(
        assert_environment.get(&OsString::from("HOME")),
        Some(&fixture.home().as_os_str().to_os_string())
    );
    assert_eq!(
        assert_environment.get(&OsString::from("PATH")),
        raw_environment.get(&OsString::from("PATH"))
    );
    assert!(!assert_environment.contains_key(&OsString::from("GIT_DIR")));
    assert!(!assert_environment.contains_key(&OsString::from("FORCE_COLOR")));
}

#[test]
fn intentional_override_after_policy_wins_over_the_scrub() {
    let fixture = SniffCliFixture::new();
    let mut recorder = environment_recorder();
    fixture.command_builder().apply_policy_to(&mut recorder);
    recorder.env("SNIFF_WAN_IP_ENDPOINTS", "http://fixture.invalid/ip");
    let output = recorder.output().expect("run override recorder");
    let environment = parse_environment(&output.stdout);
    assert_eq!(
        environment.get(&OsString::from("SNIFF_WAN_IP_ENDPOINTS")),
        Some(&OsString::from("http://fixture.invalid/ip"))
    );
}

#[test]
fn inherited_git_sniff_and_rendering_inputs_are_scrubbed() {
    let fixture = SniffCliFixture::new();
    let hostile = tempfile::tempdir().unwrap();
    let hostile_git_dir = hostile.path().join("repo.git");
    let hostile_work_tree = hostile.path().join("worktree");
    let hostile_home = hostile.path().join("home");
    let hostile_cache = hostile.path().join("cache");
    let hostile_bin = hostile.path().join("bin");
    let hostile_tmp = hostile.path().join("checkout-ancestor/tmp");
    for dir in [
        &hostile_git_dir,
        &hostile_work_tree,
        &hostile_home,
        &hostile_cache,
        &hostile_bin,
        &hostile_tmp,
    ] {
        std::fs::create_dir_all(dir).unwrap();
    }
    let mut recorder = environment_recorder();
    recorder
        .env("GIT_DIR", &hostile_git_dir)
        .env("GIT_WORK_TREE", &hostile_work_tree)
        .env("HOME", &hostile_home)
        .env("XDG_CACHE_HOME", &hostile_cache)
        .env("PATH", &hostile_bin)
        .env("COLUMNS", "44")
        .env("SNIFF_WAN_IP_ENDPOINTS", "http://host.invalid/ip")
        .env("FORCE_COLOR", "1")
        .env("TMPDIR", &hostile_tmp);
    fixture.command_builder().apply_policy_to(&mut recorder);

    let output = recorder.output().expect("run hostile-environment recorder");
    let environment = parse_environment(&output.stdout);
    assert!(!environment.contains_key(&OsString::from("GIT_DIR")));
    assert!(!environment.contains_key(&OsString::from("GIT_WORK_TREE")));
    assert!(!environment.contains_key(&OsString::from("SNIFF_WAN_IP_ENDPOINTS")));
    assert!(!environment.contains_key(&OsString::from("FORCE_COLOR")));
    assert!(!environment.contains_key(&OsString::from("COLUMNS")));
    assert_eq!(
        environment.get(&OsString::from("HOME")),
        Some(&fixture.home().as_os_str().to_os_string())
    );
    assert_eq!(
        environment.get(&OsString::from("XDG_CONFIG_HOME")),
        Some(&fixture.config_dir().as_os_str().to_os_string())
    );
    assert_eq!(
        environment.get(&OsString::from("XDG_CACHE_HOME")),
        Some(&fixture.cache_dir().as_os_str().to_os_string())
    );
    assert_eq!(
        environment.get(&OsString::from("TMPDIR")),
        Some(&fixture.tmp_dir().as_os_str().to_os_string())
    );
    let path = environment.get(&OsString::from("PATH")).unwrap();
    assert!(!std::env::split_paths(path).any(|entry| entry == hostile_bin));
    assert_eq!(
        environment.get(&OsString::from("NO_COLOR")),
        Some(&OsString::from("1"))
    );
}

#[test]
fn path_modes_are_bounded_and_keep_fixture_stubs_first() {
    let fixture = SniffCliFixture::new();

    let capture = |builder: common::SniffCommandBuilder<'_>| {
        let mut recorder = environment_recorder();
        builder.apply_policy_to(&mut recorder);
        let output = recorder.output().expect("run PATH recorder");
        let environment = parse_environment(&output.stdout);
        std::env::split_paths(environment.get(&OsString::from("PATH")).unwrap()).collect::<Vec<_>>()
    };

    let minimal = capture(fixture.command_builder());
    assert_eq!(
        minimal.first().map(PathBuf::as_path),
        Some(fixture.bin_dir())
    );
    assert_eq!(minimal.len(), 1 + common::minimal_system_path().len());

    // The assertion is that no host executable can follow fixture stubs.
    let fake_only = capture(fixture.command_builder().fake_only_path());
    assert_eq!(fake_only, [fixture.bin_dir()]);

    // `git` is the real tool this escape intentionally exposes.
    let host = capture(fixture.command_builder().host_path());
    assert_eq!(host.first().map(PathBuf::as_path), Some(fixture.bin_dir()));
    assert!(!host.is_empty());
}

#[test]
fn ambient_context_accepts_only_existing_fixture_directories() {
    let fixture = SniffCliFixture::new();
    let nested = fixture.cwd().join("repository/nested");
    std::fs::create_dir_all(&nested).unwrap();
    let _ = fixture.command_builder().ambient_context(&nested);

    let outside = tempfile::tempdir().unwrap();
    let panic = std::panic::catch_unwind(|| {
        let _ = fixture.command_builder().ambient_context(outside.path());
    });
    assert!(panic.is_err(), "an external ambient context must panic");

    let missing = fixture.workspace_path().join("missing");
    let panic = std::panic::catch_unwind(|| {
        let _ = fixture.command_builder().ambient_context(&missing);
    });
    assert!(panic.is_err(), "a missing ambient context must panic");
}

#[test]
fn canonical_checkout_containment_rejects_nested_roots() {
    let root = tempfile::tempdir().unwrap();
    let checkout = root.path().join("checkout");
    let nested = checkout.join("target/fixture");
    let outside = root.path().join("outside");
    std::fs::create_dir_all(&nested).unwrap();
    std::fs::create_dir_all(&outside).unwrap();

    let checkout = checkout.canonicalize().unwrap();
    let nested = nested.canonicalize().unwrap();
    let outside = outside.canonicalize().unwrap();
    assert!(checkout_containment_error(&nested, &checkout).is_some());
    assert!(checkout_containment_error(&outside, &checkout).is_none());
}

#[cfg(unix)]
#[test]
fn canonical_checkout_containment_rejects_symlink_spelling() {
    use std::os::unix::fs::symlink;

    let root = tempfile::tempdir().unwrap();
    let checkout = root.path().join("checkout");
    let fixture = checkout.join("fixture");
    let alias = root.path().join("alias");
    std::fs::create_dir_all(&fixture).unwrap();
    symlink(&fixture, &alias).unwrap();

    assert!(
        checkout_containment_error(
            &alias.canonicalize().unwrap(),
            &checkout.canonicalize().unwrap()
        )
        .is_some()
    );
}

#[test]
fn real_sniff_command_observes_the_fixture_launch_context() {
    let fixture = SniffCliFixture::new();
    let initialized = fixture
        .git()
        .arg("init")
        .current_dir(fixture.cwd())
        .status()
        .expect("run fixture-side git init");
    assert!(initialized.success());
    let output = fixture
        .command()
        .args(["repo", "is-monorepo", "--json", "--no-error"])
        .output()
        .expect("run shipped sniff binary");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("valid JSON stdout");
    assert_eq!(json, serde_json::json!({ "is_monorepo": false }));
}

#[test]
fn owned_command_keeps_its_disposable_launch_directory_alive() {
    let output = common::owned_sniff_command()
        .arg("--help")
        .output()
        .expect("run shipped sniff binary after constructing owned command");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 help output");
    assert!(stdout.contains("Detect system"));
    assert!(stdout.contains("Commands:"));
}
