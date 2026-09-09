//! Self-tests for the L1 spawn contract in `common::CliProcessFixture`.
//!
//! Every other L1 binary trusts the builder to hand it an `md` process that
//! cannot see the checkout it was compiled from. These tests prove that from
//! the outside: a stub written into the fixture `bin` is executed by
//! `md compose` (a `::shell` directive approved by a fixture-home whitelist)
//! and records the environment it actually received, so assertions are made
//! against that recording rather than against the builder's internals.

mod common;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use common::fixture::{checkout_root, minimal_system_path};
#[cfg(unix)]
use common::write_executable;
use common::{CliProcessFixture, checkout_containment_error, git, write};

/// Name of the recording stub. It is executed through `md compose`'s shell
/// expansion, so "the fixture stub ran" also proves the child resolved it
/// through the fixture-composed `PATH`.
const PROBE: &str = "dm-fixture-probe";

const STUB_SENTINEL: &str = "fixture-probe";

/// The compose input whose `::shell` directive launches the stub.
const PROBE_DOCUMENT: &str = "# Probe\n\n::shell dm-fixture-probe record\n";

/// Write the recording stub into the fixture `bin` directory.
///
/// The stub dumps the environment md handed it to the file named by
/// `MD_PROBE_CAPTURE` (set after `build()`, which is exactly the per-key
/// ordering the contract promises). Bracketed values distinguish "empty"
/// from a shell's own rendering of an unset variable. `MD_DRY_RUN` lives in
/// the `MD_` namespace the builder sweeps, so the capture key would be
/// scrubbed if it were inherited rather than call-site-chosen.
fn write_probe_stub(bin_dir: &Path) {
    #[cfg(windows)]
    {
        write(
            &bin_dir.join(format!("{PROBE}.cmd")),
            concat!(
                "@echo off\r\n",
                ">\"%MD_PROBE_CAPTURE%\" echo PATH=%PATH%\r\n",
                ">>\"%MD_PROBE_CAPTURE%\" echo HOME=%HOME%\r\n",
                ">>\"%MD_PROBE_CAPTURE%\" echo CWD=%CD%\r\n",
                ">>\"%MD_PROBE_CAPTURE%\" echo USERPROFILE=[%USERPROFILE%]\r\n",
                ">>\"%MD_PROBE_CAPTURE%\" echo HOMEDRIVE=[%HOMEDRIVE%]\r\n",
                ">>\"%MD_PROBE_CAPTURE%\" echo HOMEPATH=[%HOMEPATH%]\r\n",
                ">>\"%MD_PROBE_CAPTURE%\" echo APPDATA=[%APPDATA%]\r\n",
                ">>\"%MD_PROBE_CAPTURE%\" echo LOCALAPPDATA=[%LOCALAPPDATA%]\r\n",
                ">>\"%MD_PROBE_CAPTURE%\" echo XDG_CONFIG_HOME=[%XDG_CONFIG_HOME%]\r\n",
                ">>\"%MD_PROBE_CAPTURE%\" echo XDG_CACHE_HOME=[%XDG_CACHE_HOME%]\r\n",
                ">>\"%MD_PROBE_CAPTURE%\" echo TMPDIR=[%TMPDIR%]\r\n",
                ">>\"%MD_PROBE_CAPTURE%\" echo TEMP=[%TEMP%]\r\n",
                ">>\"%MD_PROBE_CAPTURE%\" echo TMP=[%TMP%]\r\n",
                ">>\"%MD_PROBE_CAPTURE%\" echo GIT_DIR=[%GIT_DIR%]\r\n",
                ">>\"%MD_PROBE_CAPTURE%\" echo GIT_WORK_TREE=[%GIT_WORK_TREE%]\r\n",
                ">>\"%MD_PROBE_CAPTURE%\" echo GIT_INDEX_FILE=[%GIT_INDEX_FILE%]\r\n",
                ">>\"%MD_PROBE_CAPTURE%\" echo GIT_CONFIG_NOSYSTEM=[%GIT_CONFIG_NOSYSTEM%]\r\n",
                ">>\"%MD_PROBE_CAPTURE%\" echo GIT_CONFIG_COUNT=[%GIT_CONFIG_COUNT%]\r\n",
                ">>\"%MD_PROBE_CAPTURE%\" echo COLUMNS=[%COLUMNS%]\r\n",
                ">>\"%MD_PROBE_CAPTURE%\" echo LINES=[%LINES%]\r\n",
                ">>\"%MD_PROBE_CAPTURE%\" echo TERM=[%TERM%]\r\n",
                ">>\"%MD_PROBE_CAPTURE%\" echo COLORTERM=[%COLORTERM%]\r\n",
                ">>\"%MD_PROBE_CAPTURE%\" echo FORCE_COLOR=[%FORCE_COLOR%]\r\n",
                ">>\"%MD_PROBE_CAPTURE%\" echo COLORFGBG=[%COLORFGBG%]\r\n",
                ">>\"%MD_PROBE_CAPTURE%\" echo CLICOLOR_FORCE=[%CLICOLOR_FORCE%]\r\n",
                ">>\"%MD_PROBE_CAPTURE%\" echo THEME=[%THEME%]\r\n",
                ">>\"%MD_PROBE_CAPTURE%\" echo DARKMATTER_TEST_VAR=[%DARKMATTER_TEST_VAR%]\r\n",
                ">>\"%MD_PROBE_CAPTURE%\" echo DM_TEST_VAR=[%DM_TEST_VAR%]\r\n",
                ">>\"%MD_PROBE_CAPTURE%\" echo MD_DRY_RUN=[%MD_DRY_RUN%]\r\n",
                ">>\"%MD_PROBE_CAPTURE%\" echo AGENT=[%AGENT%]\r\n",
                ">>\"%MD_PROBE_CAPTURE%\" echo MODEL=[%MODEL%]\r\n",
                ">>\"%MD_PROBE_CAPTURE%\" echo RUST_LOG=[%RUST_LOG%]\r\n",
                ">>\"%MD_PROBE_CAPTURE%\" echo CONTROL=[%FIXTURE_PROBE_CONTROL%]\r\n",
                ">>\"%MD_PROBE_CAPTURE%\" echo PATHEXT=[%PATHEXT%]\r\n",
                ">>\"%MD_PROBE_CAPTURE%\" echo COMSPEC=[%COMSPEC%]\r\n",
                ">>\"%MD_PROBE_CAPTURE%\" echo SYSTEMROOT=[%SystemRoot%]\r\n",
                ">>\"%MD_PROBE_CAPTURE%\" echo STUB=fixture-probe\r\n",
                "exit /b 0\r\n",
            ),
        );
    }
    #[cfg(not(windows))]
    {
        write_executable(
            &bin_dir.join(PROBE),
            r#"#!/bin/sh
{
  printf 'PATH=%s\n' "$PATH"
  printf 'HOME=%s\n' "$HOME"
  printf 'CWD=%s\n' "$(pwd)"
  printf 'USERPROFILE=[%s]\n' "$USERPROFILE"
  printf 'HOMEDRIVE=[%s]\n' "$HOMEDRIVE"
  printf 'HOMEPATH=[%s]\n' "$HOMEPATH"
  printf 'APPDATA=[%s]\n' "$APPDATA"
  printf 'LOCALAPPDATA=[%s]\n' "$LOCALAPPDATA"
  printf 'XDG_CONFIG_HOME=[%s]\n' "$XDG_CONFIG_HOME"
  printf 'XDG_CACHE_HOME=[%s]\n' "$XDG_CACHE_HOME"
  printf 'TMPDIR=[%s]\n' "$TMPDIR"
  printf 'TEMP=[%s]\n' "$TEMP"
  printf 'TMP=[%s]\n' "$TMP"
  printf 'GIT_DIR=[%s]\n' "$GIT_DIR"
  printf 'GIT_WORK_TREE=[%s]\n' "$GIT_WORK_TREE"
  printf 'GIT_INDEX_FILE=[%s]\n' "$GIT_INDEX_FILE"
  printf 'GIT_CONFIG_NOSYSTEM=[%s]\n' "$GIT_CONFIG_NOSYSTEM"
  printf 'GIT_CONFIG_COUNT=[%s]\n' "$GIT_CONFIG_COUNT"
  printf 'COLUMNS=[%s]\n' "$COLUMNS"
  printf 'LINES=[%s]\n' "$LINES"
  printf 'TERM=[%s]\n' "$TERM"
  printf 'COLORTERM=[%s]\n' "$COLORTERM"
  printf 'FORCE_COLOR=[%s]\n' "$FORCE_COLOR"
  printf 'COLORFGBG=[%s]\n' "$COLORFGBG"
  printf 'CLICOLOR_FORCE=[%s]\n' "$CLICOLOR_FORCE"
  printf 'THEME=[%s]\n' "$THEME"
  printf 'DARKMATTER_TEST_VAR=[%s]\n' "$DARKMATTER_TEST_VAR"
  printf 'DM_TEST_VAR=[%s]\n' "$DM_TEST_VAR"
  printf 'MD_DRY_RUN=[%s]\n' "$MD_DRY_RUN"
  printf 'AGENT=[%s]\n' "$AGENT"
  printf 'MODEL=[%s]\n' "$MODEL"
  printf 'RUST_LOG=[%s]\n' "$RUST_LOG"
  printf 'CONTROL=[%s]\n' "$FIXTURE_PROBE_CONTROL"
  printf 'PATHEXT=[%s]\n' "$PATHEXT"
  printf 'COMSPEC=[%s]\n' "$COMSPEC"
  printf 'SYSTEMROOT=[%s]\n' "$SystemRoot"
  printf 'STUB=fixture-probe\n'
} > "$MD_PROBE_CAPTURE"
exit 0
"#,
        );
    }
}

/// Everything a probe run needs: a fixture with the recording stub in place
/// and the shell whitelist at the anchors md's policy resolution can pick —
/// the fixture home (when the resolution context carries a home) and the
/// launch directory (the base-dir fallback for stdin compose).
fn probe_fixture(name: &str) -> (CliProcessFixture, PathBuf) {
    let fixture = CliProcessFixture::named(name);
    write_probe_stub(fixture.bin_dir());
    let whitelist = format!("prefix {PROBE}\n");
    write(
        &fixture.home().join(".darkmatter-shell-whitelist"),
        &whitelist,
    );
    write(
        &fixture.cwd().join(".darkmatter-shell-whitelist"),
        &whitelist,
    );
    let capture = fixture.workspace_path().join("probe-capture.txt");
    (fixture, capture)
}

/// Run a prepared command through the `assert_cmd` surface and read back what
/// the stub recorded.
fn run_probe(mut command: assert_cmd::Command, capture: &Path) -> BTreeMap<String, String> {
    command
        .env("MD_PROBE_CAPTURE", capture)
        .args(["compose", "-"])
        .write_stdin(PROBE_DOCUMENT)
        .assert()
        .success();
    read_probe(capture)
}

/// The same, through the raw `std::process::Command` surface, spawned rather
/// than run to completion by assert_cmd: holding a live child is the reason
/// the raw surface exists.
fn run_probe_std(mut command: std::process::Command, capture: &Path) -> BTreeMap<String, String> {
    use std::io::Write as _;
    use std::process::Stdio;
    command
        .env("MD_PROBE_CAPTURE", capture)
        .args(["compose", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().expect("the raw fixture command must spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin was piped")
        .write_all(PROBE_DOCUMENT.as_bytes())
        .expect("probe document must reach md's stdin");
    let output = child
        .wait_with_output()
        .expect("the raw fixture child must be reapable");
    assert!(
        output.status.success(),
        "the raw fixture command failed: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    read_probe(capture)
}

fn read_probe(capture: &Path) -> BTreeMap<String, String> {
    let recorded = std::fs::read_to_string(capture).unwrap_or_else(|error| {
        panic!(
            "the fixture stub did not record anything at {}: {error}",
            capture.display()
        )
    });
    let parsed: BTreeMap<String, String> = recorded
        .lines()
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| (key.trim().to_string(), value.trim().to_string()))
        .collect();
    assert_eq!(
        parsed.get("STUB").map(String::as_str),
        Some(STUB_SENTINEL),
        "the recording came from something other than the fixture stub:\n{recorded}"
    );
    parsed
}

fn path_entries(value: &str) -> Vec<PathBuf> {
    std::env::split_paths(value).collect()
}

/// Compare two paths through `canonicalize` so macOS's `/var` →
/// `/private/var` symlink does not read as a difference.
fn assert_same_dir(actual: &str, expected: &Path, what: &str) {
    let left = PathBuf::from(actual)
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(actual));
    let right = expected
        .canonicalize()
        .unwrap_or_else(|_| expected.to_path_buf());
    assert_eq!(left, right, "{what}");
}

/// Assert the live control, so every absence this file reports is a removal
/// rather than a passthrough that never happened.
fn assert_parent_environment_reached_the_child(recorded: &BTreeMap<String, String>) {
    assert_eq!(
        recorded["CONTROL"], "[sentinel-control]",
        "the parent environment must reach the child for this test to mean anything"
    );
}

#[test]
fn default_command_pins_cwd_home_and_the_minimal_path() {
    let (fixture, capture) = probe_fixture("fixture-default-shape");

    let recorded = run_probe(fixture.command(), &capture);

    assert_same_dir(
        &recorded["CWD"],
        fixture.cwd(),
        "the default command must launch from the fixture cwd, not the checkout",
    );
    assert_same_dir(
        &recorded["HOME"],
        fixture.home(),
        "the default command must point HOME at the fixture home",
    );

    // `read_probe` already proved the fixture stub — not a host tool — ran,
    // which is the PATH-isolation half of the contract; this is the other
    // half: the exact composition, and no host tool prefix on it.
    let mut expected = vec![fixture.bin_dir().to_path_buf()];
    expected.extend(minimal_system_path());
    assert_eq!(
        path_entries(&recorded["PATH"]),
        expected,
        "the default PATH must be the fixture bin followed by the minimal system set"
    );
    for prefix in [
        "/opt/homebrew",
        "/usr/local",
        "node_modules",
        ".cargo",
        ".local",
        ".npm",
        "Program Files",
    ] {
        assert!(
            !recorded["PATH"].contains(prefix),
            "child PATH must not carry the host tool prefix {prefix:?}: {}",
            recorded["PATH"]
        );
    }
}

/// The variables the scrub tests below export. Under nextest each test runs
/// in its own process, so mutating this process' environment before the
/// first spawn cannot affect another test.
fn export_scrubbed_sentinels() {
    unsafe {
        std::env::set_var("FIXTURE_PROBE_CONTROL", "sentinel-control");
        std::env::set_var("XDG_CONFIG_HOME", "/sentinel-xdg/config");
        std::env::set_var("XDG_CACHE_HOME", "/sentinel-xdg/cache");
        std::env::set_var("HOMEDRIVE", "Z:");
        std::env::set_var("HOMEPATH", "\\sentinel-homepath");
        std::env::set_var("DARKMATTER_TEST_VAR", "sentinel-darkmatter");
        std::env::set_var("DM_TEST_VAR", "sentinel-dm");
        std::env::set_var("MD_DRY_RUN", "sentinel-dry-run");
        std::env::set_var("AGENT", "sentinel-agent");
        std::env::set_var("MODEL", "sentinel-model");
        std::env::set_var("RUST_LOG", "sentinel-trace");
        std::env::set_var("THEME", "sentinel-theme");
        std::env::set_var("GIT_DIR", "/sentinel-git/.git");
        std::env::set_var("GIT_WORK_TREE", "/sentinel-git");
        std::env::set_var("GIT_INDEX_FILE", "/sentinel-git/index");
        std::env::set_var("GIT_CONFIG_COUNT", "1");
        std::env::set_var("COLUMNS", "44");
        std::env::set_var("LINES", "11");
        std::env::set_var("TERM", "sentinel-term");
        std::env::set_var("COLORTERM", "sentinel-colorterm");
        std::env::set_var("COLORFGBG", "sentinel-fgbg");
        std::env::set_var("CLICOLOR_FORCE", "1");
        std::env::set_var("FORCE_COLOR", "1");
    }
}

/// Every scrub family in one recording: home/config/cache overrides, the
/// darkmatter namespaces, Git plumbing, and rendering inputs.
#[test]
fn inherited_families_do_not_reach_the_child_and_fixture_defaults_win() {
    let (fixture, capture) = probe_fixture("fixture-inherited-families");
    export_scrubbed_sentinels();

    let recorded = run_probe(fixture.command(), &capture);

    assert_parent_environment_reached_the_child(&recorded);
    for key in [
        "HOMEDRIVE",
        "HOMEPATH",
        "DARKMATTER_TEST_VAR",
        "DM_TEST_VAR",
        "MD_DRY_RUN",
        "AGENT",
        "MODEL",
        "RUST_LOG",
        "THEME",
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_INDEX_FILE",
        "GIT_CONFIG_COUNT",
        "COLUMNS",
        "LINES",
        "COLORTERM",
        "COLORFGBG",
        "CLICOLOR_FORCE",
        "FORCE_COLOR",
    ] {
        let value = recorded
            .get(key)
            .unwrap_or_else(|| panic!("the recording carries no {key} line"));
        assert_eq!(value, "[]", "an inherited {key} reached the child");
    }
    // TERM gets its own shape of assertion: md normalizes a *missing* TERM to
    // `dumb` for the processes it spawns, so the stub cannot record `[]` —
    // but it also cannot record the sentinel unless the scrub failed and the
    // inherited value passed straight through.
    assert_eq!(
        recorded["TERM"], "[dumb]",
        "an inherited TERM reached the child (md re-adds `dumb` when it is absent)"
    );
    // The home/config/cache defaults replace what was inherited rather than
    // merely removing it, and the Git system-config opt-out is pinned on.
    assert_same_dir(
        &recorded["HOME"],
        fixture.home(),
        "HOME must resolve to the fixture home over the inherited one",
    );
    assert_eq!(
        recorded["XDG_CONFIG_HOME"],
        format!("[{}]", fixture.config_dir().display()),
        "XDG_CONFIG_HOME must point at the fixture config directory"
    );
    assert_eq!(
        recorded["XDG_CACHE_HOME"],
        format!("[{}]", fixture.cache_dir().display()),
        "XDG_CACHE_HOME must point at the fixture cache directory"
    );
    assert_eq!(
        recorded["GIT_CONFIG_NOSYSTEM"], "[1]",
        "GIT_CONFIG_NOSYSTEM must be pinned so host gitconfig never applies"
    );
    if cfg!(windows) {
        for key in ["TEMP", "TMP"] {
            assert_eq!(
                recorded[key],
                format!("[{}]", fixture.tmp_dir().display()),
                "{key} must point at the fixture temp directory"
            );
        }
    } else {
        assert_eq!(
            recorded["TMPDIR"],
            format!("[{}]", fixture.tmp_dir().display()),
            "TMPDIR must point at the fixture temp directory"
        );
    }
}

/// The scrub removes what was *inherited* and never what a test chose;
/// without this rule the builder would silently drop every intentional
/// override a migrated test sets after `build()`.
#[test]
fn a_scrubbed_key_set_after_build_still_reaches_the_child() {
    let (fixture, capture) = probe_fixture("fixture-scrubbed-key-set-after-build");
    unsafe {
        std::env::set_var("DARKMATTER_TEST_VAR", "sentinel-darkmatter");
        std::env::set_var("COLUMNS", "44");
        std::env::set_var("FIXTURE_PROBE_CONTROL", "sentinel-control");
    }

    let mut command = fixture.command();
    command
        .env("DARKMATTER_TEST_VAR", "chosen")
        .env("COLUMNS", "120");
    let recorded = run_probe(command, &capture);

    assert_parent_environment_reached_the_child(&recorded);
    assert_eq!(
        recorded["DARKMATTER_TEST_VAR"], "[chosen]",
        "a darkmatter value chosen after `build()` must out-rank the scrub"
    );
    assert_eq!(
        recorded["COLUMNS"], "[120]",
        "a render width chosen after `build()` must out-rank the scrub"
    );
}

/// The tightening knob has to survive its own `env_clear()`. On Windows that
/// clear also takes `PATHEXT`, `COMSPEC`, and `SystemRoot` — without which
/// the console host cannot resolve or launch the fixture's `.cmd` stubs — so
/// `build()` puts exactly those three back. The `cfg!` arms are both compiled
/// on every platform: the Unix arm proves the restore stays Windows-only.
#[test]
fn inherit_no_env_keeps_the_defaults_and_restores_windows_console_plumbing() {
    let (fixture, capture) = probe_fixture("fixture-inherit-no-env");
    unsafe {
        std::env::set_var("FIXTURE_PROBE_CONTROL", "sentinel-control");
    }

    let recorded = run_probe(fixture.command_builder().inherit_no_env().build(), &capture);

    assert_eq!(
        recorded["CONTROL"], "[]",
        "a cleared environment must not carry the parent's own variables"
    );
    assert_parent_defaults_survive_a_clear(&fixture, &recorded);
    for key in ["PATHEXT", "COMSPEC", "SYSTEMROOT"] {
        if cfg!(windows) {
            assert_ne!(
                recorded[key], "[]",
                "{key} must come back after env_clear, or the console host \
                 cannot launch the fixture's .cmd stubs"
            );
        } else {
            assert_eq!(
                recorded[key], "[]",
                "{key} is Windows console plumbing; a Unix run must not gain it"
            );
        }
    }
}

fn assert_parent_defaults_survive_a_clear(
    fixture: &CliProcessFixture,
    recorded: &BTreeMap<String, String>,
) {
    assert_same_dir(
        &recorded["HOME"],
        fixture.home(),
        "clearing the environment must still leave the fixture defaults in place",
    );
    let mut expected = vec![fixture.bin_dir().to_path_buf()];
    expected.extend(minimal_system_path());
    assert_eq!(
        path_entries(&recorded["PATH"]),
        expected,
        "clearing the environment must not clear the fixture PATH"
    );
}

/// `build()` and `build_std()` apply one computed `ChildEnvironment`, but
/// nothing in the type system says they must. A policy change reaching only
/// one surface would surface as a live-child test inheriting the developer's
/// environment, so the two recordings are compared here — and, because two
/// identically broken surfaces would also compare equal, the policy itself is
/// asserted on the raw half.
#[test]
fn both_command_surfaces_hand_the_child_the_same_environment() {
    let (fixture, assert_cmd_capture) = probe_fixture("fixture-surface-drift");
    let std_capture = fixture.workspace_path().join("probe-capture-std.txt");
    export_scrubbed_sentinels();

    let through_assert_cmd = run_probe(fixture.command(), &assert_cmd_capture);
    let through_std = run_probe_std(fixture.command_std(), &std_capture);

    assert_eq!(
        through_assert_cmd, through_std,
        "the assert_cmd and raw command surfaces gave the child different environments"
    );
    assert_parent_environment_reached_the_child(&through_std);
    for key in [
        "DARKMATTER_TEST_VAR",
        "MD_DRY_RUN",
        "GIT_DIR",
        "GIT_WORK_TREE",
        "COLUMNS",
        "FORCE_COLOR",
    ] {
        assert_eq!(
            through_std[key], "[]",
            "the raw command surface let an inherited {key} through"
        );
    }
    // The sentinel XDG_CONFIG_HOME did not pass — the fixture default did,
    // which on the raw surface is the pinned value rather than an absence.
    assert_eq!(
        through_std["XDG_CONFIG_HOME"],
        format!("[{}]", fixture.config_dir().display()),
        "the raw command surface must pin XDG_CONFIG_HOME at the fixture, not \
         inherit it"
    );
    // md re-adds `dumb` for children when TERM is absent (see the inherited
    // -families test), so `[dumb]` here means the sentinel never passed.
    assert_ne!(
        through_std["TERM"], "[sentinel-term]",
        "the raw command surface let an inherited TERM through"
    );
    assert_same_dir(
        &through_std["CWD"],
        fixture.cwd(),
        "the raw command surface must launch from the fixture cwd, not the checkout",
    );
}

/// The cleared-environment arm of the same drift contract: the Windows
/// console restore is the only conditional the policy carries, and comparing
/// the surfaces checks it on `windows-latest` without a Windows-only test.
#[test]
fn both_command_surfaces_clear_the_environment_the_same_way() {
    let (fixture, assert_cmd_capture) = probe_fixture("fixture-surface-drift-cleared");
    let std_capture = fixture.workspace_path().join("probe-capture-std.txt");
    export_scrubbed_sentinels();

    let through_assert_cmd = run_probe(
        fixture.command_builder().inherit_no_env().build(),
        &assert_cmd_capture,
    );
    let through_std = run_probe_std(
        fixture.command_builder().inherit_no_env().build_std(),
        &std_capture,
    );

    assert_eq!(
        through_assert_cmd, through_std,
        "the two command surfaces cleared the child's environment differently"
    );
    assert_eq!(
        through_std["CONTROL"], "[]",
        "a cleared environment must not carry the parent's own variables"
    );
    assert_parent_defaults_survive_a_clear(&fixture, &through_std);
}

#[test]
fn fake_only_path_escape_carries_the_fixture_bin_alone() {
    let (fixture, capture) = probe_fixture("fixture-fake-only-path");

    // Escape: fake-only PATH. The proof this test depends on is that the
    // minimal system set is absent, which is the escape's whole contract.
    let recorded = run_probe(fixture.command_builder().fake_only_path().build(), &capture);

    assert_eq!(
        path_entries(&recorded["PATH"]),
        vec![fixture.bin_dir().to_path_buf()],
        "the fake-only escape must expose nothing but the fixture bin"
    );
}

#[test]
fn host_path_escape_prepends_the_fixture_bin_to_the_host_path() {
    let (fixture, capture) = probe_fixture("fixture-host-path");

    // Escape: full host PATH. This test's subject is the escape itself, so
    // the tool it needs is "whatever the host has" — see the assertions below.
    let recorded = run_probe(fixture.command_builder().host_path().build(), &capture);

    let child = path_entries(&recorded["PATH"]);
    assert_eq!(
        child.first(),
        Some(&fixture.bin_dir().to_path_buf()),
        "the fixture bin must still win resolution under the host escape: {child:?}"
    );
    for entry in path_entries(&std::env::var("PATH").unwrap_or_default()) {
        assert!(
            child.contains(&entry),
            "the host escape must keep host PATH entry {}: {child:?}",
            entry.display()
        );
    }
}

#[test]
fn ambient_context_escape_pins_the_cwd_to_a_test_built_directory() {
    let (fixture, capture) = probe_fixture("fixture-ambient-context");
    let nested = fixture.workspace_path().join("project").join("docs");
    std::fs::create_dir_all(&nested).expect("nested launch directory");
    // The base-dir anchor for the shell whitelist moves with the pinned
    // launch directory, so the whitelist must exist there too.
    write(
        &nested.join(".darkmatter-shell-whitelist"),
        &format!("prefix {PROBE}\n"),
    );

    // Escape: ambient context. The subject is launch-context behavior from a
    // nested directory the test built inside its own workspace.
    let recorded = run_probe(
        fixture.command_builder().ambient_context(&nested).build(),
        &capture,
    );

    assert_same_dir(
        &recorded["CWD"],
        &nested,
        "the ambient-context escape must pin the launch to the test-built directory",
    );
}

#[test]
#[should_panic(expected = "is not inside the fixture workspace")]
fn ambient_context_escape_rejects_a_directory_outside_the_workspace() {
    let fixture = CliProcessFixture::named("fixture-ambient-context-rejected");
    let _ = fixture
        .command_builder()
        .ambient_context(Path::new(env!("CARGO_MANIFEST_DIR")));
}

#[test]
#[should_panic(expected = "must exist before it is pinned")]
fn ambient_context_escape_rejects_a_directory_that_does_not_exist() {
    let fixture = CliProcessFixture::named("fixture-ambient-context-missing");
    let missing = fixture.workspace_path().join("never-created");
    let _ = fixture.command_builder().ambient_context(&missing);
}

/// The plan's named hostile-environment proof: a `GIT_DIR` at a throwaway
/// repository, a relocated `HOME`, `COLUMNS=44`, `FORCE_COLOR=1`, a poisoned
/// `PATH`, and a checkout-ancestor `TMPDIR` all leave the fixture's observed
/// defaults unchanged. Every piece of hostile state is disposable.
struct RemoveOnDrop(PathBuf);

impl Drop for RemoveOnDrop {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn hostile_inherited_environment_leaves_the_fixture_defaults_unchanged() {
    let hostile = tempfile::TempDir::new().expect("hostile scratch space");
    let throwaway_repo = hostile.path().join("throwaway-repo");
    std::fs::create_dir_all(&throwaway_repo).expect("throwaway repository directory");
    let empty_git_config = hostile.path().join("empty-gitconfig");
    write(&empty_git_config, "");
    if !git(&empty_git_config)
        .arg("init")
        .current_dir(&throwaway_repo)
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
    {
        eprintln!("note: `git init` unavailable; GIT_DIR points at the unbuilt path");
    }

    // A poisoned PATH entry carrying a same-named stub, so a leak would
    // change which recording binary runs rather than only the PATH string.
    let poisoned_bin = hostile.path().join("poisoned-bin");
    std::fs::create_dir_all(&poisoned_bin).expect("poisoned bin directory");
    #[cfg(unix)]
    write_executable(
        &poisoned_bin.join(PROBE),
        "#!/bin/sh\necho 'STUB=hostile-probe' > \"$MD_PROBE_CAPTURE\"\n",
    );
    #[cfg(windows)]
    write(
        &poisoned_bin.join(format!("{PROBE}.cmd")),
        "@echo off\r\necho STUB=hostile-probe>\"%MD_PROBE_CAPTURE%\"\r\n",
    );

    // A checkout-ancestor TMPDIR: the fixture must accept a workspace beside
    // the checkout (containment is about *inside*, not near) while still
    // owning every default. Created under the checkout's parent — never
    // inside the checkout itself — and removed on drop.
    let Some(checkout) = checkout_root() else {
        panic!("hostile TMPDIR test requires a resolvable checkout root");
    };
    let ancestor_tmp = checkout
        .parent()
        .unwrap_or_else(|| panic!("checkout {} has no parent", checkout.display()))
        .join(format!("dm-hostile-tmp-{}", std::process::id()));
    if std::fs::create_dir_all(&ancestor_tmp).is_err() {
        eprintln!(
            "skipping: checkout parent {} is not writable",
            ancestor_tmp.display()
        );
        return;
    }
    let _ancestor_guard = RemoveOnDrop(ancestor_tmp.clone());

    let mut poisoned_entries = vec![poisoned_bin.clone()];
    poisoned_entries.extend(std::env::split_paths(
        &std::env::var_os("PATH").unwrap_or_default(),
    ));
    let poisoned_path = std::env::join_paths(poisoned_entries).expect("poisoned PATH must join");
    unsafe {
        std::env::set_var("GIT_DIR", throwaway_repo.join(".git"));
        std::env::set_var("HOME", hostile.path().join("hostile-home"));
        std::env::set_var("COLUMNS", "44");
        std::env::set_var("FORCE_COLOR", "1");
        std::env::set_var("PATH", &poisoned_path);
        if cfg!(windows) {
            std::env::set_var("TEMP", &ancestor_tmp);
            std::env::set_var("TMP", &ancestor_tmp);
        } else {
            std::env::set_var("TMPDIR", &ancestor_tmp);
        }
        std::env::set_var("FIXTURE_PROBE_CONTROL", "sentinel-control");
    }

    let (fixture, capture) = probe_fixture("fixture-hostile-environment");
    let recorded = run_probe(fixture.command(), &capture);

    assert_parent_environment_reached_the_child(&recorded);
    assert_same_dir(
        &recorded["CWD"],
        fixture.cwd(),
        "a hostile environment must not move the pinned launch directory",
    );
    assert_same_dir(
        &recorded["HOME"],
        fixture.home(),
        "a relocated HOME must not out-rank the fixture home",
    );
    let mut expected = vec![fixture.bin_dir().to_path_buf()];
    expected.extend(minimal_system_path());
    assert_eq!(
        path_entries(&recorded["PATH"]),
        expected,
        "a poisoned PATH must not reach the child"
    );
    for key in ["GIT_DIR", "COLUMNS", "FORCE_COLOR"] {
        assert_eq!(recorded[key], "[]", "hostile {key} reached the child");
    }
    // The operator's own terminal shape must not reach the child either; md
    // re-adds `dumb` when TERM is absent, so anything else here is a leak.
    assert_eq!(
        recorded["TERM"], "[dumb]",
        "the operator's TERM reached the child"
    );
}

/// The containment rule as a pure comparison, with stand-in paths: inducing
/// the real condition means mutating `TMPDIR` for `CliProcessFixture::named`
/// itself, which cannot be spelled portably. The symlink-spelled variant is
/// covered by its own test below.
#[test]
fn a_workspace_inside_the_checkout_is_rejected_by_naming_the_temp_dir_variable() {
    let checkout = PathBuf::from("rusty-biscuit-stand-in");
    let inside = checkout.join("target").join("tmpdir-probe").join("fixture");

    let message = checkout_containment_error(&inside, &checkout)
        .expect("a workspace under the checkout must be rejected");
    let temp_dir_variable = if cfg!(windows) {
        "TMP (or TEMP)"
    } else {
        "TMPDIR"
    };
    for expected in [
        inside.display().to_string(),
        checkout.display().to_string(),
        temp_dir_variable.to_string(),
    ] {
        assert!(
            message.contains(&expected),
            "the rejection must name {expected:?} so the developer can act on it: {message}"
        );
    }

    let outside = PathBuf::from("somewhere-else").join("fixture");
    assert!(
        checkout_containment_error(&outside, &checkout).is_none(),
        "a workspace outside the checkout must be accepted"
    );
    // A textual-prefix sibling is not containment; `starts_with` compares
    // components, and this pins that it keeps doing so.
    let sibling = PathBuf::from("rusty-biscuit-stand-in-2").join("fixture");
    assert!(
        checkout_containment_error(&sibling, &checkout).is_none(),
        "a sibling sharing a textual prefix must not read as containment"
    );
}

/// A workspace *spelled* outside the checkout but resolving inside it through
/// a symlink must compare as inside — which is why `named()` canonicalizes
/// before calling `checkout_containment_error`. Unix-gated: creating a
/// symlink on Windows needs developer-mode privileges the CI leg does not
/// assume; the canonicalize call itself is platform-generic and compiled on
/// every leg.
#[cfg(unix)]
#[test]
fn a_symlink_spelled_workspace_inside_the_checkout_is_rejected() {
    let scratch = tempfile::TempDir::new().expect("symlink scratch space");
    let checkout = scratch.path().join("stand-in-checkout");
    std::fs::create_dir_all(checkout.join("target")).expect("stand-in checkout");
    let link_parent = scratch.path().join("link-parent");
    std::fs::create_dir_all(&link_parent).expect("link parent");

    std::os::unix::fs::symlink(checkout.join("target"), link_parent.join("inside")).unwrap();
    let resolved = link_parent.join("inside").canonicalize().unwrap();
    assert!(
        checkout_containment_error(&resolved, &checkout.canonicalize().unwrap()).is_some(),
        "a workspace spelled {} resolves inside the checkout and must be rejected",
        link_parent.join("inside").display(),
    );

    std::os::unix::fs::symlink(scratch.path(), link_parent.join("outside")).unwrap();
    let resolved_outside = link_parent.join("outside").canonicalize().unwrap();
    assert!(
        checkout_containment_error(&resolved_outside, &checkout.canonicalize().unwrap()).is_none(),
        "a symlink resolving outside the checkout must be accepted"
    );
}

/// The fixture's own `git init` runs from the test process with its whole
/// environment intact, so a hostile `GIT_CONFIG_GLOBAL`/`GIT_CONFIG_SYSTEM`
/// is the parent-side half of the isolation contract: without the override,
/// a host config could hijack identity, signing, or hooks inside a fixture
/// repository.
#[test]
fn initialize_repository_is_repository_local_and_ignores_host_git_configuration() {
    let fixture = CliProcessFixture::named("fixture-git-topology");
    let hostile_global = fixture.workspace_path().join("hostile-gitconfig");
    write(
        &hostile_global,
        "[user]\n\tname = HOST-HIJACK\n\temail = host@example.invalid\n[core]\n\thooksPath = /hostile-hooks\n[commit]\n\tgpgsign = true\n",
    );
    unsafe {
        std::env::set_var("GIT_CONFIG_GLOBAL", &hostile_global);
        std::env::set_var("GIT_CONFIG_SYSTEM", &hostile_global);
    }

    let repo = fixture.workspace_path().join("repo");
    if !fixture.initialize_repository_at(&repo) {
        eprintln!("skipping: `git` unavailable on this host");
        return;
    }

    assert!(
        repo.join(".git").exists(),
        "git init must build the repository in the directory the fixture named"
    );
    let readback_config = fixture.workspace_path().join("git-config-readback");
    write(&readback_config, "");
    let read = |key: &str| {
        let output = git(&readback_config)
            .args(["config", "--get", key])
            .current_dir(&repo)
            .output()
            .expect("git config readback must run");
        (
            output.status.success(),
            String::from_utf8_lossy(&output.stdout).trim().to_string(),
        )
    };
    let (name_ok, name) = read("user.name");
    assert!(
        name_ok && name == "md fixture",
        "repository identity must be the fixture-local one, got {name:?}"
    );
    let (hooks_set, _) = read("core.hooksPath");
    assert!(
        !hooks_set,
        "a host hooksPath must not reach the fixture repository"
    );
    let (gpg_ok, gpgsign) = read("commit.gpgsign");
    assert!(
        gpg_ok && gpgsign == "false",
        "signing must stay disabled repository-locally, got {gpgsign:?}"
    );
}

/// The topology builders keep references *relative*: relocating a
/// document-plus-schema pair (or a transcluding document and its part)
/// preserves the relative-resolution relationship instead of rewriting it to
/// an absolute path.
#[test]
fn topology_builders_keep_references_relative() {
    let fixture = CliProcessFixture::named("fixture-topology");

    let (doc, schema) = fixture.nested_schema_layout(
        "docs/guides/post.md",
        "schemas/base.yaml",
        |schema_ref| format!("---\n$schema: {schema_ref}\ntitle: Hello\n---\nBody\n"),
        "title: string\n",
    );
    let doc_text = std::fs::read_to_string(&doc).unwrap();
    assert!(
        doc_text.contains("$schema: ../../schemas/base.yaml"),
        "the schema reference must stay relative to the document (doc sits two \
         levels below the workspace, the schema one): {doc_text}"
    );
    assert_eq!(
        std::fs::read_to_string(&schema).unwrap(),
        "title: string\n",
        "the schema content must land byte-for-byte"
    );

    let (main, part) = fixture.relative_reference_layout("site", "part.md", "Part body\n");
    let main_text = std::fs::read_to_string(&main).unwrap();
    assert!(
        main_text.contains("::file ./part.md"),
        "the transclusion reference must stay relative: {main_text}"
    );
    assert!(part.ends_with("site/part.md"));
}

/// Shipped content relocated into a fixture must arrive byte-for-byte with
/// its relative structure intact — copying `example-docs` content is how
/// Phase 6 keeps corpus coverage while isolating the launch.
#[test]
fn copy_tree_relocates_shipped_content_byte_for_byte() {
    let fixture = CliProcessFixture::named("fixture-copy-tree");
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("example-docs")
        .join("rendering");

    let destination = fixture.workspace_path().join("rendering");
    common::fixture::copy_tree(&source, &destination).expect("shipped tree must copy");

    let original = std::fs::read_to_string(source.join("style-prop.md")).unwrap();
    let copied = std::fs::read_to_string(destination.join("style-prop.md")).unwrap();
    assert_eq!(
        original, copied,
        "shipped content must be byte-for-byte identical"
    );
    assert_eq!(
        std::fs::read_dir(&destination).unwrap().count(),
        std::fs::read_dir(&source).unwrap().count(),
        "the relocated tree must preserve the file inventory"
    );
}
