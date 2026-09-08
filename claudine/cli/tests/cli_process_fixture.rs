//! Self-tests for the L1 spawn contract in `common::CliProcessFixture`.
//!
//! Every other L1 binary trusts the builder to hand it a `claudine` process
//! that cannot see the checkout it was compiled from. These tests prove that
//! from the outside: a provider stub written into the fixture `bin` records
//! the environment it actually received, and the assertions are made against
//! that recording rather than against the builder's internals.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

mod common;
use common::CliProcessFixture;
use common::wrap::create_claudine_monorepo;

/// Name of the provider stub. `claude` is deliberately a provider a developer
/// machine is likely to have installed, so "the fixture stub ran" is also the
/// proof that a host install did not win selection.
const PROVIDER: &str = "claude";

const STUB_SENTINEL: &str = "fixture-probe";

/// Write the recording provider stub into the fixture `bin` directory.
///
/// The stub dumps the environment claudine handed it to the file named by
/// `CLAUDINE_PROBE_CAPTURE`. Bracketed values distinguish "empty" from the
/// shell's own rendering of an unset variable.
///
/// `FIXTURE_PROBE_CONTROL` deliberately carries no `CLAUDINE_` prefix: it is
/// the live control proving the parent's environment reaches the child at all,
/// and the builder scrubs the whole `CLAUDINE_*` namespace at build time.
/// `CLAUDINE_PROBE_CAPTURE` is scrubbed too, but [`run_probe`] sets it *after*
/// `build()`, which is exactly the per-key ordering the contract promises.
fn write_probe_stub(bin_dir: &Path) {
    #[cfg(windows)]
    {
        common::write(
            &bin_dir.join(format!("{PROVIDER}.cmd")),
            concat!(
                "@echo off\r\n",
                ">\"%CLAUDINE_PROBE_CAPTURE%\" echo PATH=%PATH%\r\n",
                ">>\"%CLAUDINE_PROBE_CAPTURE%\" echo HOME=%HOME%\r\n",
                ">>\"%CLAUDINE_PROBE_CAPTURE%\" echo CWD=%CD%\r\n",
                ">>\"%CLAUDINE_PROBE_CAPTURE%\" echo XDG_CONFIG_HOME=[%XDG_CONFIG_HOME%]\r\n",
                ">>\"%CLAUDINE_PROBE_CAPTURE%\" echo HOMEDRIVE=[%HOMEDRIVE%]\r\n",
                ">>\"%CLAUDINE_PROBE_CAPTURE%\" echo HOMEPATH=[%HOMEPATH%]\r\n",
                ">>\"%CLAUDINE_PROBE_CAPTURE%\" echo CONTROL=[%FIXTURE_PROBE_CONTROL%]\r\n",
                ">>\"%CLAUDINE_PROBE_CAPTURE%\" echo CLAUDINE_STEP_TIMEOUT=[%CLAUDINE_STEP_TIMEOUT%]\r\n",
                ">>\"%CLAUDINE_PROBE_CAPTURE%\" echo CLAUDINE_RENDEZVOUS_REPORT=[%CLAUDINE_RENDEZVOUS_REPORT%]\r\n",
                ">>\"%CLAUDINE_PROBE_CAPTURE%\" echo PLAYA_DRY_RUN=[%PLAYA_DRY_RUN%]\r\n",
                ">>\"%CLAUDINE_PROBE_CAPTURE%\" echo PLAYA_SPOOL_DIR=[%PLAYA_SPOOL_DIR%]\r\n",
                ">>\"%CLAUDINE_PROBE_CAPTURE%\" echo GIT_DIR=[%GIT_DIR%]\r\n",
                ">>\"%CLAUDINE_PROBE_CAPTURE%\" echo GIT_WORK_TREE=[%GIT_WORK_TREE%]\r\n",
                ">>\"%CLAUDINE_PROBE_CAPTURE%\" echo TERM_WIDTH=[%TERM_WIDTH%]\r\n",
                ">>\"%CLAUDINE_PROBE_CAPTURE%\" echo COLUMNS=[%COLUMNS%]\r\n",
                ">>\"%CLAUDINE_PROBE_CAPTURE%\" echo FORCE_COLOR=[%FORCE_COLOR%]\r\n",
                ">>\"%CLAUDINE_PROBE_CAPTURE%\" echo PATHEXT=[%PATHEXT%]\r\n",
                ">>\"%CLAUDINE_PROBE_CAPTURE%\" echo COMSPEC=[%COMSPEC%]\r\n",
                ">>\"%CLAUDINE_PROBE_CAPTURE%\" echo SYSTEMROOT=[%SystemRoot%]\r\n",
                ">>\"%CLAUDINE_PROBE_CAPTURE%\" echo STUB=fixture-probe\r\n",
                "exit /b 0\r\n",
            ),
        );
    }
    #[cfg(not(windows))]
    {
        common::write_executable(
            &bin_dir.join(PROVIDER),
            r#"#!/bin/sh
{
  printf 'PATH=%s\n' "$PATH"
  printf 'HOME=%s\n' "$HOME"
  printf 'CWD=%s\n' "$(pwd)"
  printf 'XDG_CONFIG_HOME=[%s]\n' "$XDG_CONFIG_HOME"
  printf 'HOMEDRIVE=[%s]\n' "$HOMEDRIVE"
  printf 'HOMEPATH=[%s]\n' "$HOMEPATH"
  printf 'CONTROL=[%s]\n' "$FIXTURE_PROBE_CONTROL"
  printf 'CLAUDINE_STEP_TIMEOUT=[%s]\n' "$CLAUDINE_STEP_TIMEOUT"
  printf 'CLAUDINE_RENDEZVOUS_REPORT=[%s]\n' "$CLAUDINE_RENDEZVOUS_REPORT"
  printf 'PLAYA_DRY_RUN=[%s]\n' "$PLAYA_DRY_RUN"
  printf 'PLAYA_SPOOL_DIR=[%s]\n' "$PLAYA_SPOOL_DIR"
  printf 'GIT_DIR=[%s]\n' "$GIT_DIR"
  printf 'GIT_WORK_TREE=[%s]\n' "$GIT_WORK_TREE"
  printf 'TERM_WIDTH=[%s]\n' "$TERM_WIDTH"
  printf 'COLUMNS=[%s]\n' "$COLUMNS"
  printf 'FORCE_COLOR=[%s]\n' "$FORCE_COLOR"
  printf 'PATHEXT=[%s]\n' "$PATHEXT"
  printf 'COMSPEC=[%s]\n' "$COMSPEC"
  printf 'SYSTEMROOT=[%s]\n' "$SystemRoot"
  printf 'STUB=fixture-probe\n'
} > "$CLAUDINE_PROBE_CAPTURE"
exit 0
"#,
        );
    }
}

/// Everything a probe run needs: a fixture with a seeded user config and the
/// recording stub in place.
fn probe_fixture(name: &str) -> (CliProcessFixture, PathBuf) {
    let fixture = CliProcessFixture::named(name);
    fixture.seed_user_config();
    write_probe_stub(fixture.bin_dir());
    let capture = fixture.workspace_path().join("probe-capture.txt");
    (fixture, capture)
}

/// Run a prepared command and read back what the stub recorded.
fn run_probe(mut command: assert_cmd::Command, capture: &Path) -> BTreeMap<String, String> {
    command
        .env("CLAUDINE_PROBE_CAPTURE", capture)
        .args([PROVIDER, "record the environment"])
        .assert()
        .success();

    read_probe(capture)
}

/// The same, through the raw `std::process::Command` surface.
///
/// Spawned rather than run to completion: holding a live [`std::process::Child`]
/// is the whole reason the raw path exists, so every raw-path assertion below
/// also proves the command it was handed can be spawned at all.
fn run_probe_std(mut command: std::process::Command, capture: &Path) -> BTreeMap<String, String> {
    let output = command
        .env("CLAUDINE_PROBE_CAPTURE", capture)
        .args([PROVIDER, "record the environment"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("the raw fixture command must spawn")
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

#[test]
fn default_command_pins_cwd_home_and_the_minimal_system_path() {
    let (fixture, capture) = probe_fixture("fixture-default-shape");

    let recorded = run_probe(fixture.command(), &capture);

    // This assertion used to be the whole of the suite's defence against a
    // temp directory inside the checkout: the child landed on the checkout root
    // and this line reported it, naming the symptom and not the cause.
    // `CliProcessFixture::named` now rejects such a workspace at construction
    // (`common::checkout_containment_error`), so what remains here is the
    // direct proof that `build()` pins `current_dir` — keep it.
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

    let mut expected = vec![fixture.bin_dir().to_path_buf()];
    expected.extend(common::minimal_system_path());
    assert_eq!(
        path_entries(&recorded["PATH"]),
        expected,
        "the default PATH must be the fixture bin followed by the minimal system set"
    );
}

/// Acceptance criterion 8's guard: a fake provider named like a host-installed
/// one resolves to the fake, and no host tool prefix reaches the child.
#[test]
fn default_path_hides_host_tool_prefixes_and_resolves_the_fixture_stub() {
    let (fixture, capture) = probe_fixture("fixture-default-path-isolation");

    // `run_probe` asserts the recording carries the stub sentinel, so reaching
    // this line already proves the fixture stub — not a host `claude` — ran.
    let recorded = run_probe(fixture.command(), &capture);

    let child: Vec<PathBuf> = path_entries(&recorded["PATH"]);
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
            !child.iter().any(|entry| entry.to_string_lossy().contains(prefix)),
            "child PATH must not carry the host tool prefix {prefix:?}: {child:?}"
        );
    }

    let allowed = common::minimal_system_path();
    let host = std::env::var("PATH").unwrap_or_default();
    for entry in path_entries(&host) {
        if allowed.contains(&entry) {
            continue;
        }
        assert!(
            !child.contains(&entry),
            "host PATH entry {} leaked into the child's PATH: {child:?}",
            entry.display()
        );
    }
}

#[test]
fn default_command_drops_inherited_home_and_config_overrides() {
    let (fixture, capture) = probe_fixture("fixture-inherited-overrides");

    // The removals only mean something when the parent process carries the
    // variables in the first place. Under nextest each test runs in its own
    // process, so mutating this process' environment before the first spawn
    // cannot affect another test.
    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", "/sentinel-xdg");
        std::env::set_var("HOMEDRIVE", "Z:");
        std::env::set_var("HOMEPATH", "\\sentinel-homepath");
        std::env::set_var("FIXTURE_PROBE_CONTROL", "sentinel-control");
    }

    let recorded = run_probe(fixture.command(), &capture);

    // The control proves the parent's environment does reach the child, so the
    // absences below are removals rather than a passthrough that never happens.
    assert_eq!(
        recorded["CONTROL"], "[sentinel-control]",
        "the parent environment must reach the child for this test to mean anything"
    );
    for key in ["XDG_CONFIG_HOME", "HOMEDRIVE", "HOMEPATH"] {
        assert!(
            !recorded[key].contains("sentinel"),
            "{key} was inherited from the parent process: {}",
            recorded[key]
        );
    }
    assert_same_dir(
        &recorded["HOME"],
        fixture.home(),
        "HOME must still resolve to the fixture home",
    );
}

/// Assert the live control, so every absence this file reports is a removal
/// rather than a passthrough that never happened.
fn assert_parent_environment_reached_the_child(recorded: &BTreeMap<String, String>) {
    assert_eq!(
        recorded["CONTROL"], "[sentinel-control]",
        "the parent environment must reach the child for this test to mean anything"
    );
}

/// An exported `CLAUDINE_STEP_TIMEOUT` re-parameterizes the timeout tests this
/// fix tightened, so the developer's symptom is "your machine says the new
/// budgets are wrong".
#[test]
fn default_command_drops_the_inherited_claudine_namespace() {
    let (fixture, capture) = probe_fixture("fixture-inherited-claudine-namespace");

    // Under nextest each test runs in its own process, so mutating this
    // process' environment before the first spawn cannot affect another test.
    unsafe {
        std::env::set_var("CLAUDINE_STEP_TIMEOUT", "sentinel-step-timeout");
        std::env::set_var("FIXTURE_PROBE_CONTROL", "sentinel-control");
    }

    let recorded = run_probe(fixture.command(), &capture);

    assert_parent_environment_reached_the_child(&recorded);
    assert_eq!(
        recorded["CLAUDINE_STEP_TIMEOUT"], "[]",
        "an inherited CLAUDINE_* variable reached the child"
    );
}

/// `GIT_DIR`/`GIT_WORK_TREE` override cwd-based repository discovery, so an
/// inherited pair defeats the pinned `current_dir` entirely.
#[test]
fn default_command_drops_the_inherited_git_plumbing_family() {
    let (fixture, capture) = probe_fixture("fixture-inherited-git-plumbing");

    unsafe {
        std::env::set_var("GIT_DIR", "/sentinel-git/.git");
        std::env::set_var("GIT_WORK_TREE", "/sentinel-git");
        std::env::set_var("FIXTURE_PROBE_CONTROL", "sentinel-control");
    }

    let recorded = run_probe(fixture.command(), &capture);

    assert_parent_environment_reached_the_child(&recorded);
    for key in ["GIT_DIR", "GIT_WORK_TREE"] {
        assert_eq!(
            recorded[key], "[]",
            "an inherited {key} reached the child and would relocate repository discovery"
        );
    }
}

/// `cli/src/log.rs` reads `TERM_WIDTH` then `COLUMNS` before falling back to 80
/// columns, and `FORCE_COLOR` would out-vote the builder's `NO_COLOR=1`. An
/// interactive shell exports `COLUMNS` routinely.
#[test]
fn default_command_drops_the_inherited_render_inputs() {
    let (fixture, capture) = probe_fixture("fixture-inherited-render-inputs");

    unsafe {
        std::env::set_var("TERM_WIDTH", "44");
        std::env::set_var("COLUMNS", "44");
        std::env::set_var("FORCE_COLOR", "1");
        std::env::set_var("FIXTURE_PROBE_CONTROL", "sentinel-control");
    }

    let recorded = run_probe(fixture.command(), &capture);

    assert_parent_environment_reached_the_child(&recorded);
    for key in ["TERM_WIDTH", "COLUMNS", "FORCE_COLOR"] {
        assert_eq!(
            recorded[key], "[]",
            "an inherited {key} reached the child and would reshape its rendering"
        );
    }
}

/// The scrub is per key at build time, so it removes what was *inherited* and
/// never what a test chose. Without this, a builder that scrubbed at spawn time
/// would silently drop the timeout every migrated watchdog test sets.
#[test]
fn a_scrubbed_key_set_after_build_still_reaches_the_child() {
    let (fixture, capture) = probe_fixture("fixture-scrubbed-key-set-after-build");

    unsafe {
        std::env::set_var("CLAUDINE_STEP_TIMEOUT", "sentinel-step-timeout");
        std::env::set_var("TERM_WIDTH", "44");
        std::env::set_var("FIXTURE_PROBE_CONTROL", "sentinel-control");
    }

    let mut command = fixture.command();
    command
        .env("CLAUDINE_STEP_TIMEOUT", "0.5s")
        .env("TERM_WIDTH", "200");
    let recorded = run_probe(command, &capture);

    assert_parent_environment_reached_the_child(&recorded);
    assert_eq!(
        recorded["CLAUDINE_STEP_TIMEOUT"], "[0.5s]",
        "a CLAUDINE_* value chosen after `build()` must out-rank the scrub"
    );
    assert_eq!(
        recorded["TERM_WIDTH"], "[200]",
        "a render width chosen after `build()` must out-rank the scrub"
    );
}

/// The tightening knob has to survive its own `env_clear()`. On Windows that
/// clear also takes `PATHEXT`, `COMSPEC`, and `SystemRoot` — without which this
/// file's own `.cmd` stub cannot be resolved or launched — so `build()` puts
/// exactly those three back. The `cfg!` arms below are both compiled on every
/// platform: the Unix arm proves the restore stays Windows-only, which is the
/// half of the contract a macOS run can verify.
#[test]
fn inherit_no_env_keeps_the_defaults_and_drops_everything_else() {
    let (fixture, capture) = probe_fixture("fixture-inherit-no-env");

    unsafe {
        std::env::set_var("FIXTURE_PROBE_CONTROL", "sentinel-control");
    }

    let recorded = run_probe(fixture.command_builder().inherit_no_env().build(), &capture);

    assert_eq!(
        recorded["CONTROL"], "[]",
        "a cleared environment must not carry the parent's own variables"
    );
    assert_same_dir(
        &recorded["HOME"],
        fixture.home(),
        "clearing the environment must still leave the fixture defaults in place",
    );
    let mut expected = vec![fixture.bin_dir().to_path_buf()];
    expected.extend(common::minimal_system_path());
    assert_eq!(
        path_entries(&recorded["PATH"]),
        expected,
        "clearing the environment must not clear the fixture PATH"
    );

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

/// Export the families the builder scrubs, so a comparison between the two
/// command surfaces covers the removals and not only the defaults.
///
/// Under nextest each test runs in its own process, so mutating this process'
/// environment before the first spawn cannot affect another test.
fn export_scrubbed_sentinels() {
    unsafe {
        std::env::set_var("CLAUDINE_STEP_TIMEOUT", "sentinel-step-timeout");
        std::env::set_var("GIT_DIR", "/sentinel-git/.git");
        std::env::set_var("GIT_WORK_TREE", "/sentinel-git");
        std::env::set_var("XDG_CONFIG_HOME", "/sentinel-xdg");
        std::env::set_var("TERM_WIDTH", "44");
        std::env::set_var("COLUMNS", "44");
        std::env::set_var("FORCE_COLOR", "1");
        std::env::set_var("FIXTURE_PROBE_CONTROL", "sentinel-control");
    }
}

/// The drift test for the two command surfaces.
///
/// `build()` and `build_std()` apply one computed `ChildEnvironment`, but
/// nothing in the type system says they must: each takes its own program and
/// its own adapter. A policy change reaching only the `assert_cmd` half would
/// otherwise surface as a live-child test that inherited the developer's
/// `$HOME` — the exact failure the builder exists to prevent — so the two are
/// compared here against what the child actually received.
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
    // Non-vacuity: two identically broken surfaces would also compare equal,
    // so the policy itself is asserted on the raw half.
    assert_parent_environment_reached_the_child(&through_std);
    for key in [
        "CLAUDINE_STEP_TIMEOUT",
        "GIT_DIR",
        "GIT_WORK_TREE",
        "XDG_CONFIG_HOME",
        "TERM_WIDTH",
        "COLUMNS",
        "FORCE_COLOR",
    ] {
        assert_eq!(
            through_std[key], "[]",
            "the raw command surface let an inherited {key} through"
        );
    }
    assert_same_dir(
        &through_std["CWD"],
        fixture.cwd(),
        "the raw command surface must launch from the fixture cwd, not the checkout",
    );
    assert_same_dir(
        &through_std["HOME"],
        fixture.home(),
        "the raw command surface must point HOME at the fixture home",
    );
}

/// Reporting stays off on **both** surfaces, even against a parent that turned
/// it on.
///
/// `CLAUDINE_RENDEZVOUS_REPORT` is the one default that lives inside a
/// namespace the builder also sweeps, so "the child gets `false`" is not the
/// default alone — it is the default *plus* the ordering that applies the
/// sweep first. Neither half is expressed in a type, and a live-child test on
/// the raw surface is exactly where a session report would start reaching a
/// developer's daemon, so both are asserted here rather than left to the
/// set-equality comparison above (two identically enabled surfaces compare
/// equal).
#[test]
fn both_command_surfaces_disable_rendezvous_reporting_over_an_enabled_parent() {
    let (fixture, assert_cmd_capture) = probe_fixture("fixture-rendezvous-report");
    let std_capture = fixture.workspace_path().join("probe-capture-std.txt");
    unsafe {
        std::env::set_var("CLAUDINE_RENDEZVOUS_REPORT", "true");
    }

    let through_assert_cmd = run_probe(fixture.command(), &assert_cmd_capture);
    let through_std = run_probe_std(fixture.command_std(), &std_capture);

    for (surface, recorded) in [
        ("assert_cmd", &through_assert_cmd),
        ("raw", &through_std),
    ] {
        assert_eq!(
            recorded["CLAUDINE_RENDEZVOUS_REPORT"], "[false]",
            "the {surface} surface handed the child {:?} instead of the disabled \
             default, so an unrelated test can reach a live rendezvous daemon",
            recorded["CLAUDINE_RENDEZVOUS_REPORT"],
        );
    }
}

/// Audio stays a decision the test can read, not a process it has to own.
///
/// A lifecycle audio effect makes claudine re-exec *itself* as playa's
/// detached spool worker, which deliberately outlives the command that
/// enqueued the job. `just test-leaks` caught two orphaned `claudine`
/// processes from one L1 test that composed a shipped prompt with such an
/// effect, and each run also played a sound on the developer's machine. So
/// both keys are asserted here rather than left to a whole-suite sweep that
/// only reports the symptom: `PLAYA_DRY_RUN` stops the worker existing, and
/// `PLAYA_SPOOL_DIR` keeps a job that *is* under test out of the shared
/// per-user spool root.
///
/// The parent exports the values that would defeat both, because they live in
/// the `PLAYA_*` namespace the builder sweeps — so this is the same
/// scrub-then-default ordering the reporting test above pins.
#[test]
fn both_command_surfaces_keep_audio_out_of_the_developers_machine() {
    let (fixture, assert_cmd_capture) = probe_fixture("fixture-playa-defaults");
    let std_capture = fixture.workspace_path().join("probe-capture-std.txt");
    unsafe {
        std::env::set_var("PLAYA_DRY_RUN", "0");
        std::env::set_var("PLAYA_SPOOL_DIR", "/sentinel-shared-spool");
    }

    let through_assert_cmd = run_probe(fixture.command(), &assert_cmd_capture);
    let through_std = run_probe_std(fixture.command_std(), &std_capture);

    for (surface, recorded) in [
        ("assert_cmd", &through_assert_cmd),
        ("raw", &through_std),
    ] {
        assert_eq!(
            recorded["PLAYA_DRY_RUN"], "[1]",
            "the {surface} surface let the parent re-enable real playback, so a \
             lifecycle audio effect would spawn a detached worker that outlives \
             the test",
        );
        assert_same_dir(
            recorded["PLAYA_SPOOL_DIR"].trim_matches(['[', ']']),
            &fixture.workspace_path().join("playa-spool"),
            "the {surface} surface must point the spool at the fixture, not at \
             the shared per-user root",
        );
    }
}

/// The cleared-environment arm of the same drift contract.
///
/// `inherit_no_env()` is where the two surfaces are most likely to diverge: the
/// Windows console restore is the only conditional the policy carries, and it
/// is unreachable on the hosts most runs happen on. Comparing the surfaces
/// checks it on `windows-latest` without a Windows-only test, and the `cfg!`
/// arm below pins the platform half a Unix run can see.
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
    for key in ["PATHEXT", "COMSPEC", "SYSTEMROOT"] {
        if cfg!(windows) {
            assert_ne!(
                through_std[key], "[]",
                "{key} must come back after env_clear on the raw surface too, or the \
                 console host cannot launch the fixture's .cmd stubs"
            );
        } else {
            assert_eq!(
                through_std[key], "[]",
                "{key} is Windows console plumbing; a Unix run must not gain it"
            );
        }
    }
}

/// The `PATH` escapes have to reach the raw surface, or the live-child cohort
/// migrating onto it in Phase 5D loses them and rebuilds `PATH` by hand — which
/// is the per-call-site environment chain the builder replaced.
#[test]
fn the_raw_command_surface_keeps_the_named_path_escapes() {
    let (fixture, fake_only_capture) = probe_fixture("fixture-raw-path-escapes");
    let host_capture = fixture.workspace_path().join("probe-capture-host.txt");

    // Escape: fake-only PATH. The proof is the absence of the minimal system
    // set, exactly as on the `assert_cmd` surface.
    let fake_only = run_probe_std(
        fixture.command_builder().fake_only_path().build_std(),
        &fake_only_capture,
    );
    // Escape: full host PATH. The subject is the escape itself, so the tool it
    // needs is "whatever the host has".
    let host = run_probe_std(
        fixture.command_builder().host_path().build_std(),
        &host_capture,
    );

    assert_eq!(
        path_entries(&fake_only["PATH"]),
        vec![fixture.bin_dir().to_path_buf()],
        "the fake-only escape must expose nothing but the fixture bin on the raw surface"
    );
    let child = path_entries(&host["PATH"]);
    assert_eq!(
        child.first(),
        Some(&fixture.bin_dir().to_path_buf()),
        "the fixture bin must still win resolution under the host escape: {child:?}"
    );
    for entry in path_entries(&std::env::var("PATH").unwrap_or_default()) {
        assert!(
            child.contains(&entry),
            "the host escape must keep host PATH entry {} on the raw surface: {child:?}",
            entry.display()
        );
    }
}

#[test]
fn the_raw_command_surface_keeps_the_ambient_context_escape() {
    let (fixture, capture) = probe_fixture("fixture-raw-ambient-context");
    let Some((repo_root, launch_dir, _bin)) = create_claudine_monorepo(fixture.workspace_path())
    else {
        eprintln!("skipping: `git init` unavailable on this host");
        return;
    };

    // Escape: ambient context. As on the `assert_cmd` surface, the subject is
    // launch-context discovery from a nested package directory.
    let recorded = run_probe_std(
        fixture
            .command_builder()
            .ambient_context(&launch_dir)
            .build_std(),
        &capture,
    );

    assert_same_dir(
        &recorded["CWD"],
        &repo_root,
        "the raw surface's ambient-context escape must anchor on the test-built repository",
    );
}

/// The containment check is the builder's, not the surface's: it fires at
/// `command_builder()` time and so protects `build_std()` identically.
#[test]
#[should_panic(expected = "is not inside the fixture workspace")]
fn the_raw_command_surface_rejects_an_ambient_context_outside_the_workspace() {
    let fixture = CliProcessFixture::named("fixture-raw-ambient-context-rejected");
    let _ = fixture
        .command_builder()
        .ambient_context(Path::new(env!("CARGO_MANIFEST_DIR")))
        .build_std();
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

    // Escape: full host PATH. This test's subject is the escape itself, so the
    // tool it needs is "whatever the host has" — see the assertions below.
    let recorded = run_probe(fixture.command_builder().host_path().build(), &capture);

    let child = path_entries(&recorded["PATH"]);
    assert_eq!(
        child.first(),
        Some(&fixture.bin_dir().to_path_buf()),
        "the fixture bin must still win resolution under the host escape: {child:?}"
    );
    let host = std::env::var("PATH").unwrap_or_default();
    for entry in path_entries(&host) {
        assert!(
            child.contains(&entry),
            "the host escape must keep host PATH entry {}: {child:?}",
            entry.display()
        );
    }
}

#[test]
fn ambient_context_escape_pins_the_cwd_to_a_test_built_repository() {
    let (fixture, capture) = probe_fixture("fixture-ambient-context");
    let Some((repo_root, launch_dir, _bin)) = create_claudine_monorepo(fixture.workspace_path())
    else {
        eprintln!("skipping: `git init` unavailable on this host");
        return;
    };

    // Escape: ambient context. The subject is launch-context discovery from a
    // nested package directory, so the CWD moves to a repository this test
    // built inside its own workspace.
    let recorded = run_probe(
        fixture.command_builder().ambient_context(&launch_dir).build(),
        &capture,
    );

    // Claudine deliberately starts the agent at the repository root rather
    // than at the launch directory, so the recording is `repo_root` even
    // though the escape pinned the launch to `<repo>/claudine/cli`. That the
    // child landed on a repository root at all is the proof: without the
    // escape the launch CWD is the fixture cwd, which is no repository, and
    // the child would have recorded that directory instead.
    assert_same_dir(
        &recorded["CWD"],
        &repo_root,
        "the ambient-context escape must anchor the run on the repository the test built",
    );
    assert!(
        launch_dir.starts_with(&repo_root),
        "fixture invariant: the pinned launch directory lives inside the built repository"
    );
    let recorded_cwd = PathBuf::from(&recorded["CWD"])
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(&recorded["CWD"]));
    let checkout = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    assert!(
        !recorded_cwd.starts_with(&checkout),
        "the rusty-biscuit checkout must never become the launch context: {recorded_cwd:?}"
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

/// The parent-side half of the isolation contract.
///
/// The builder protects the `claudine` child; the fixture's own `git init` runs
/// from the test process with its whole environment intact. With `GIT_DIR`
/// exported, `git init` builds the repository *there* and leaves the fixture
/// directory bare — which is how, on 2026-08-31, a pre-push hook run committed
/// fixture files onto a feature branch. The hijack destination here is inside
/// the fixture workspace, so the failing case would create a throwaway
/// directory rather than touch anything real.
#[test]
fn a_parent_side_git_cannot_be_relocated_by_an_inherited_gitdir() {
    let fixture = CliProcessFixture::named("fixture-parent-side-git");
    let hijacked = fixture.workspace_path().join("hijacked-repository");
    let work = fixture.workspace_path().join("intended-repository");
    std::fs::create_dir_all(&work).expect("fixture work directory");

    // Under nextest each test runs in its own process, so this cannot affect
    // another test.
    unsafe {
        std::env::set_var("GIT_DIR", &hijacked);
    }

    if !common::init_git_repo(&work) {
        eprintln!("skipping: `git init` unavailable on this host");
        return;
    }

    assert!(
        work.join(".git").exists(),
        "git init must build the repository in the directory the fixture named"
    );
    assert!(
        !hijacked.exists(),
        "an inherited GIT_DIR relocated the fixture's own repository to {}",
        hijacked.display()
    );
}

/// The fixture workspace is built under `std::env::temp_dir()`, so a developer
/// whose temp dir sits inside the checkout gets a workspace claudine's
/// repository discovery walks straight back out of.
///
/// The check is exercised directly rather than through a `#[should_panic]`
/// fixture construction: inducing the real condition means mutating `TMPDIR`
/// in this process, which cannot be spelled portably (Windows reads
/// `TMP`/`TEMP`) and would not run on the platforms most likely to hit it.
/// Comparing two canonical paths is the whole of the rule, so calling it with
/// stand-in paths tests exactly what `named()` calls.
#[test]
fn a_workspace_inside_the_checkout_is_rejected_by_naming_the_temp_dir_variable() {
    let checkout = PathBuf::from("rusty-biscuit-stand-in");
    let inside = checkout.join("target").join("tmpdir-probe").join("fixture");

    let message = common::checkout_containment_error(&inside, &checkout)
        .expect("a workspace under the checkout must be rejected");
    // `std::env::temp_dir()` reads `TMPDIR` on Unix and `TMP`/`TEMP` on
    // Windows, so the actionable name differs by platform.
    let temp_dir_variable = if cfg!(windows) { "TMP (or TEMP)" } else { "TMPDIR" };
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
        common::checkout_containment_error(&outside, &checkout).is_none(),
        "a workspace outside the checkout must be accepted"
    );
    // A textual-prefix sibling is not containment; `starts_with` compares
    // components, and this pins that it keeps doing so.
    let sibling = PathBuf::from("rusty-biscuit-stand-in-2").join("fixture");
    assert!(
        common::checkout_containment_error(&sibling, &checkout).is_none(),
        "a sibling sharing a textual prefix must not read as containment"
    );

    // Why `named()` canonicalizes before calling this: a spelling that *resolves*
    // inside the checkout but does not lexically begin with it slips through a
    // component comparison. Both arguments arriving canonical is the
    // precondition, not an optimization — and it is what makes the raw path
    // `build_std()` opens inherit the same protection, since the check is on the
    // fixture rather than on either command surface.
    let uncanonical = PathBuf::from("elsewhere")
        .join("..")
        .join("rusty-biscuit-stand-in")
        .join("fixture");
    assert!(
        common::checkout_containment_error(&uncanonical, &checkout).is_none(),
        "the check compares path components; an uncanonicalized argument is a caller bug"
    );
}
