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
}
