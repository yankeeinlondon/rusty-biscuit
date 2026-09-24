//! Behavioral contracts for `just kache-status`
//! (fixes/2026-09-23-ensuring-kache-support), run as a real subprocess on an
//! isolated fixture host (`common::kache`) with a fake `kache` and a clone
//! probe the test controls.
#![cfg(unix)]

mod common;

use std::path::PathBuf;

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

fn status(fixture: &KacheHostFixture) -> (Option<i32>, String) {
    let output = fixture.just("kache-status");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr);
    (
        output.status.code(),
        format!("{stdout}\n--- stderr ---\n{stderr}"),
    )
}

/// A store on the checkout's device that cannot clone into it (ext4, ZFS
/// without block cloning) restores by copy. `qualify` rejects that layout, so
/// status must too: it used to compare device ids only and certify it as
/// "active on a filesystem that clones blocks".
#[test]
fn status_fails_when_the_store_cannot_clone_into_the_checkout() {
    let fixture = KacheHostFixture::new(&repo_inputs());
    fixture.activate_wrapper();
    fixture.fail_clones_into(&["checkout"]);

    let (code, output) = status(&fixture);

    assert_eq!(
        code,
        Some(1),
        "a store that cannot clone is drift:\n{output}"
    );
    assert!(
        output.contains("VERDICT: DRIFT"),
        "the verdict is drift, not a healthy one:\n{output}"
    );
    assert!(
        output.contains("cannot clone into this checkout: clone-unsupported"),
        "the drift names the failed clone check and its reason:\n{output}"
    );
}

/// The control for the test above and for every drift test below: the same
/// host with a working clone, a running daemon at the binary's version, and
/// a pinned store with `ignore_env` is healthy.
#[test]
fn status_reports_no_clone_problem_when_the_store_clones_into_the_checkout() {
    let fixture = KacheHostFixture::new(&repo_inputs());
    fixture.activate_wrapper();

    let (code, output) = status(&fixture);

    assert!(
        output.contains("store -> checkout: clone"),
        "the clone check is reported:\n{output}"
    );
    assert!(
        !output.contains("cannot clone"),
        "a cloning store is not drift:\n{output}"
    );
    assert_eq!(code, Some(0), "a cloning host is healthy:\n{output}");
    assert!(
        output.contains("VERDICT: active on a filesystem that clones blocks"),
        "the healthy verdict is printed:\n{output}"
    );
}

/// A worktree base on the store's device that cannot be cloned into leaves
/// every worktree created there restoring by copy, even while the checkout
/// itself clones.
#[test]
fn status_fails_when_the_store_cannot_clone_into_the_worktree_base() {
    let mut fixture = KacheHostFixture::new(&repo_inputs());
    fixture.activate_wrapper();
    let base = fixture.configure_worktree_base();
    fixture.fail_clones_into(&["worktree-base"]);

    let (code, output) = status(&fixture);

    assert_eq!(
        code,
        Some(1),
        "a base that cannot clone is drift:\n{output}"
    );
    assert!(
        output.contains("store -> checkout: clone"),
        "the checkout still clones:\n{output}"
    );
    assert!(
        output.contains(&format!(
            "cannot clone into the worktree base ({}): clone-unsupported",
            base.display()
        )),
        "the drift names the base and the failed clone check:\n{output}"
    );
}

/// The `active` line of a status report.
fn active_line(output: &str) -> &str {
    output
        .lines()
        .map(str::trim_start)
        .find(|line| line.starts_with("active "))
        .unwrap_or_else(|| panic!("status prints an active line:\n{output}"))
}

fn assert_not_in_use(code: Option<i32>, output: &str) {
    assert_eq!(code, Some(0), "no kache wrapper is not drift:\n{output}");
    assert!(
        output.contains("VERDICT: not in use"),
        "the verdict is not-in-use:\n{output}"
    );
}

/// `rustc-wrapper = ""` is the neutralized state `_ensure-kache` writes when
/// it fails; Cargo reads it as no wrapper. Status used to match the key's
/// text alone and certify it as an active, healthy kache.
#[test]
fn status_reports_an_empty_config_wrapper_as_inactive() {
    let fixture = KacheHostFixture::new(&repo_inputs());
    fixture.write_config_wrapper("");

    let (code, output) = status(&fixture);

    assert_not_in_use(code, &output);
    let active = active_line(&output);
    assert!(
        active.starts_with("active       no") && active.contains("empty wrapper"),
        "the empty wrapper is reported as disabling wrapping: {active}"
    );
}

/// Each spelling resolves, by Cargo's rules, to the `kache` on `PATH`: the
/// bare name through `PATH`, the absolute path as is, and `bin/kache`
/// relative to the directory holding `$CARGO_HOME` (the fixture root).
#[test]
fn status_reports_a_kache_config_wrapper_as_active_by_name_or_path() {
    let spellings = |fixture: &KacheHostFixture| {
        [
            "kache".to_owned(),
            fixture.kache_bin().display().to_string(),
            "bin/kache".to_owned(),
        ]
    };
    for index in 0..3 {
        let fixture = KacheHostFixture::new(&repo_inputs());
        let value = spellings(&fixture)[index].clone();
        fixture.write_config_wrapper(&value);

        let (code, output) = status(&fixture);

        let active = active_line(&output);
        assert!(
            active.starts_with("active       YES") && active.contains("config.toml"),
            "a `{value}` wrapper is active kache: {active}"
        );
        assert!(
            !output.contains("VERDICT: not in use"),
            "an active kache is judged, not dismissed:\n{output}"
        );
        assert_eq!(code, Some(0), "a cloning host is healthy:\n{output}");
    }
}

/// A kache wrapper in a `config.toml` that a legacy `$CARGO_HOME/config`
/// hides from Cargo is not active. Status used to read both files and call
/// kache active from the one Cargo ignores; real Cargo does not run it.
#[test]
fn status_reports_a_kache_wrapper_hidden_by_a_legacy_config_as_inactive() {
    let fixture = KacheHostFixture::new(&repo_inputs());
    fixture.write_legacy_cargo_config("[term]\ncolor = \"never\"\n");
    fixture.activate_wrapper();

    let (code, output) = status(&fixture);

    assert_not_in_use(code, &output);
    assert_eq!(
        active_line(&output),
        "active       no — nothing sets a rustc wrapper",
        "{output}"
    );

    match fixture.cargo_check_runs_kache() {
        Some(ran) => assert!(!ran, "real Cargo ignores the hidden config.toml"),
        None => eprintln!("skipping the real-Cargo half: cargo could not be started"),
    }
}

/// A kache wrapper in the legacy `$CARGO_HOME/config` is active even when
/// the `config.toml` beside it disables wrapping, because Cargo reads only
/// the legacy file.
#[test]
fn status_reports_a_kache_wrapper_in_a_legacy_config_as_active() {
    let fixture = KacheHostFixture::new(&repo_inputs());
    fixture.write_legacy_cargo_config("[build]\nrustc-wrapper = \"kache\"\n");
    fixture.write_config_wrapper("");

    let (code, output) = status(&fixture);

    assert_eq!(code, Some(0), "{output}");
    assert_eq!(
        active_line(&output),
        format!(
            "active       YES — {}/config (host-wide: every repo on this machine)",
            fixture.cargo_home().display()
        ),
        "{output}"
    );

    match fixture.cargo_check_runs_kache() {
        Some(ran) => assert!(ran, "real Cargo runs the legacy config's wrapper"),
        None => eprintln!("skipping the real-Cargo half: cargo could not be started"),
    }
}

/// The problem lines of a drift verdict.
fn problems(output: &str) -> Vec<&str> {
    output
        .lines()
        .map(str::trim_start)
        .filter(|line| line.starts_with("- "))
        .collect()
}

/// A kache-named wrapper Cargo cannot start. Real Cargo fails every build
/// under it; status used to match the name alone and print `active YES` with
/// the healthy verdict. The review-3 reproduction, verbatim.
#[test]
fn status_fails_when_the_kache_wrapper_path_does_not_exist() {
    const MISSING: &str = "/definitely/missing/kache";
    let mut fixture = KacheHostFixture::new(&repo_inputs());
    fixture.activate_wrapper();
    fixture.set_rustc_wrapper_env(MISSING);

    let (code, output) = status(&fixture);

    assert_eq!(code, Some(1), "a missing wrapper is drift:\n{output}");
    assert!(
        active_line(&output).starts_with("active       BROKEN — environment RUSTC_WRAPPER="),
        "the wrapper is not reported as working kache:\n{output}"
    );
    assert!(!output.contains("VERDICT: active"), "{output}");
    assert_eq!(
        problems(&output),
        [format!(
            "- Cargo would run the rustc wrapper {MISSING}, which does not exist — every build fails"
        )],
        "exactly the missing wrapper is the problem:\n{output}"
    );

    // Ground truth: Cargo cannot run that wrapper either.
    let Some(cargo) = cargo_check_with_wrapper(MISSING) else {
        eprintln!("skipping the real-Cargo half: cargo is not on PATH");
        return;
    };
    assert!(
        !cargo.status.success(),
        "Cargo fails under a missing wrapper:\n{}",
        String::from_utf8_lossy(&cargo.stderr)
    );
    assert!(
        String::from_utf8_lossy(&cargo.stderr).contains(MISSING),
        "Cargo names the wrapper it could not run:\n{}",
        String::from_utf8_lossy(&cargo.stderr)
    );
}

/// `cargo check` of an empty scratch crate with `RUSTC_WRAPPER=wrapper`, a
/// scratch `CARGO_HOME` and target directory; `None` when cargo is absent.
fn cargo_check_with_wrapper(wrapper: &str) -> Option<std::process::Output> {
    let scratch = tempfile::tempdir().expect("tempdir");
    let project = scratch.path().join("project");
    std::fs::create_dir_all(project.join("src")).expect("project directory");
    std::fs::write(
        project.join("Cargo.toml"),
        "[package]\nname = \"wrapper-probe\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[workspace]\n",
    )
    .expect("write manifest");
    std::fs::write(project.join("src/lib.rs"), "").expect("write lib");
    std::fs::create_dir_all(scratch.path().join("cargo-home")).expect("cargo home");
    std::process::Command::new("cargo")
        .args(["check", "--offline", "--quiet"])
        .current_dir(&project)
        .env("CARGO_HOME", scratch.path().join("cargo-home"))
        .env("CARGO_TARGET_DIR", scratch.path().join("target"))
        .env("RUSTC_WRAPPER", wrapper)
        .env_remove("CARGO_BUILD_RUSTC_WRAPPER")
        .env_remove("RUSTC_WORKSPACE_WRAPPER")
        .env_remove("CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER")
        .env_remove("RUSTC")
        .env_remove("CARGO_BUILD_RUSTC")
        .output()
        .ok()
}

/// An executable named kache that is not the `kache` on `PATH` — the binary
/// whose version, daemon, and passthrough status checks. Its name certifies
/// nothing about it.
#[test]
fn status_fails_when_the_kache_wrapper_is_another_executable() {
    let fixture = KacheHostFixture::new(&repo_inputs());
    let other = fixture.install_other_kache();
    fixture.write_config_wrapper(&other.display().to_string());

    let (code, output) = status(&fixture);

    assert_eq!(code, Some(1), "an uncertified wrapper is drift:\n{output}");
    assert!(
        active_line(&output).starts_with("active       BROKEN"),
        "{output}"
    );
    assert_eq!(
        problems(&output),
        [format!(
            "- Cargo would run the rustc wrapper {}, not the kache on PATH ({}) whose version and passthrough are checked here",
            other.display(),
            fixture.kache_bin().display()
        )],
        "exactly the foreign binary is the problem:\n{output}"
    );
}

/// Cargo reads `.cargo/config.toml` in every ancestor of the checkout, so a
/// parent directory's kache activation is in use. Status used to read only
/// the checkout's own `.cargo/` and `$CARGO_HOME` and call it "not in use".
#[test]
fn status_reports_a_kache_wrapper_in_a_parent_directory_config_as_active() {
    let fixture = KacheHostFixture::new(&repo_inputs());
    let parent_config = fixture.write_parent_config_wrapper("kache");

    let (code, output) = status(&fixture);

    let active = active_line(&output);
    assert!(
        active.starts_with("active       YES")
            && active.contains(&format!(
                "{} (a parent directory of this checkout)",
                parent_config.display()
            )),
        "the parent directory's activation is active kache: {active}"
    );
    assert!(
        !output.contains("VERDICT: not in use"),
        "an active kache is judged, not dismissed:\n{output}"
    );
    assert_eq!(code, Some(0), "a cloning host is healthy:\n{output}");
}

/// Another wrapper is not kache: nothing of kache's is in use, so there is
/// nothing for status to police.
#[test]
fn status_reports_an_unrelated_config_wrapper_as_not_kache() {
    let fixture = KacheHostFixture::new(&repo_inputs());
    fixture.write_config_wrapper("sccache");

    let (code, output) = status(&fixture);

    assert_not_in_use(code, &output);
    let active = active_line(&output);
    assert!(
        active.starts_with("active       no") && active.contains("'sccache', not kache"),
        "the foreign wrapper is named: {active}"
    );
}

/// A set-but-empty `RUSTC_WRAPPER` overrides every config file, so a kache
/// activation in `$CARGO_HOME` is shadowed rather than in use — and still
/// reported, because undoing the variable brings it back.
#[test]
fn status_honors_an_empty_rustc_wrapper_env_over_a_kache_config() {
    let mut fixture = KacheHostFixture::new(&repo_inputs());
    fixture.activate_wrapper();
    fixture.set_rustc_wrapper_env("");

    let (code, output) = status(&fixture);

    assert_not_in_use(code, &output);
    let active = active_line(&output);
    assert!(
        active.contains("environment RUSTC_WRAPPER= sets an empty wrapper"),
        "the empty environment value wins: {active}"
    );
    assert!(
        output.contains("shadowed     kache in")
            && output.contains("takes effect if RUSTC_WRAPPER is undone"),
        "the shadowed config activation is still reported:\n{output}"
    );
}

#[test]
fn status_reports_a_kache_rustc_wrapper_env_without_config_as_active() {
    let mut fixture = KacheHostFixture::new(&repo_inputs());
    fixture.set_rustc_wrapper_env("kache");

    let (code, output) = status(&fixture);

    let active = active_line(&output);
    assert!(
        active.starts_with("active       YES — environment RUSTC_WRAPPER=kache"),
        "the environment activation is active kache: {active}"
    );
    assert_eq!(code, Some(0), "a cloning host is healthy:\n{output}");
}

/// An active, otherwise healthy host with one fact drifted: status must fail
/// and name that fact. A drift that also breaks another invariant would pass
/// on the wrong reason, so each case below changes exactly one fact.
fn assert_drift(fixture: &KacheHostFixture, reason: &str) {
    fixture.activate_wrapper();

    let (code, output) = status(fixture);

    assert_eq!(code, Some(1), "drift fails status:\n{output}");
    assert!(
        output.contains("VERDICT: DRIFT"),
        "the verdict is drift, not a healthy one:\n{output}"
    );
    let problems: Vec<&str> = output
        .lines()
        .map(str::trim_start)
        .filter(|line| line.starts_with("- "))
        .collect();
    assert_eq!(
        problems.len(),
        1,
        "exactly the drifted fact is a problem:\n{output}"
    );
    assert!(
        problems[0].contains(reason),
        "the problem names `{reason}`:\n{output}"
    );
}

fn running_daemon(version: &str) -> FakeDaemon {
    FakeDaemon {
        service_installed: true,
        running: true,
        version: Some(version.to_owned()),
    }
}

#[test]
fn status_fails_when_kache_is_below_the_floor() {
    let fixture = KacheHostFixture::new(&repo_inputs());
    fixture.set_kache_version("0.1.0");
    fixture.set_daemon(&running_daemon("0.1.0"));

    assert_drift(&fixture, "kache 0.1.0 is below the floor");
}

#[test]
fn status_fails_when_the_daemon_is_not_running() {
    let fixture = KacheHostFixture::new(&repo_inputs());
    fixture.set_daemon(&FakeDaemon {
        service_installed: true,
        running: false,
        version: None,
    });

    assert_drift(&fixture, "the kache daemon is not running");
}

/// A daemon started by hand (`kache daemon run`) still serves builds, but
/// nothing brings it back after a logout or reboot.
#[test]
fn status_fails_when_the_daemon_service_is_not_installed() {
    let fixture = KacheHostFixture::new(&repo_inputs());
    fixture.set_daemon(&FakeDaemon {
        service_installed: false,
        ..running_daemon(&floor())
    });

    assert_drift(&fixture, "the kache daemon service is not installed");
}

/// The daemon keeps running the binary it started from, so an upgrade
/// without a restart leaves the old (on macOS, hardened) build serving.
#[test]
fn status_fails_when_the_daemon_runs_another_version_than_the_binary() {
    let fixture = KacheHostFixture::new(&repo_inputs());
    fixture.set_daemon(&running_daemon("0.22.9"));

    assert_drift(
        &fixture,
        "the running daemon is kache 0.22.9 but the installed binary is",
    );
}

#[test]
fn status_fails_when_the_store_is_not_pinned() {
    let fixture = KacheHostFixture::new(&repo_inputs());
    fixture.write_kache_config("[cache]\nignore_env = true\n");

    assert_drift(&fixture, "[cache] local_store is missing from");
}

#[test]
fn status_fails_when_the_pin_names_another_store_than_kache_resolves() {
    let fixture = KacheHostFixture::new(&repo_inputs());
    fixture.set_doctor_store(&fixture.store());
    fixture.write_kache_config("[cache]\nlocal_store = \"/elsewhere/kache\"\nignore_env = true\n");

    assert_drift(
        &fixture,
        &format!(
            "[cache] local_store (/elsewhere/kache) in {} is not the store kache resolves ({})",
            fixture.kache_config().display(),
            fixture.store().display()
        ),
    );
}

#[test]
fn status_fails_when_ignore_env_is_false() {
    let fixture = KacheHostFixture::new(&repo_inputs());
    fixture.write_kache_config(&format!(
        "[cache]\nlocal_store = \"{}\"\nignore_env = false\n",
        fixture.store().display()
    ));

    assert_drift(&fixture, "[cache] ignore_env is false in");
}

#[test]
fn status_fails_when_ignore_env_is_missing() {
    let fixture = KacheHostFixture::new(&repo_inputs());
    fixture.write_kache_config(&format!(
        "[cache]\nlocal_store = \"{}\"\n",
        fixture.store().display()
    ));

    assert_drift(&fixture, "[cache] ignore_env is missing from");
}

/// With no wrapper active, the same drift is reported but not judged: there
/// is nothing of kache's in use for it to break.
#[test]
fn status_reports_but_does_not_fail_drift_when_kache_is_not_in_use() {
    let fixture = KacheHostFixture::new(&repo_inputs());
    fixture.set_daemon(&FakeDaemon {
        service_installed: false,
        running: false,
        version: None,
    });

    let (code, output) = status(&fixture);

    assert_not_in_use(code, &output);
    assert!(
        output.contains("service installed: no, running: no"),
        "the daemon state is still printed:\n{output}"
    );
}

fn floor() -> String {
    std::fs::read_to_string(repo_root().join(".github/kache-min-version"))
        .expect("floor file")
        .trim()
        .to_owned()
}

/// An active host whose worktree-base setting `wt` refuses is drift, not
/// healthy: status used to read every such setting as unconfigured and exit
/// 0, and certified a `WT` naming a Git repository as covered.
fn assert_status_fails_on_invalid_base(setting: InvalidWorktreeBase) {
    let mut fixture = KacheHostFixture::new(&repo_inputs());
    fixture.activate_wrapper();
    let (reason, path) = fixture.configure_invalid_worktree_base(setting);

    let (code, output) = status(&fixture);

    assert_eq!(code, Some(1), "{setting:?} is drift:\n{output}");
    assert!(output.contains("VERDICT: DRIFT"), "{output}");
    assert!(
        output.contains(&format!(
            "the worktree base setting is invalid ({reason}: {}) — wt refuses it",
            path.display()
        )),
        "the drift names the rule and the path:\n{output}"
    );
    assert!(
        output.contains(&format!(
            "worktree base INVALID setting ({reason}: {})",
            path.display()
        )),
        "the facts section names it too:\n{output}"
    );
}

#[test]
fn status_fails_on_a_missing_wt_path() {
    assert_status_fails_on_invalid_base(InvalidWorktreeBase::MissingWtPath);
}

#[test]
fn status_fails_on_a_wt_git_repository() {
    assert_status_fails_on_invalid_base(InvalidWorktreeBase::WtGitRepository);
}

#[test]
fn status_fails_on_a_malformed_worktree_config() {
    assert_status_fails_on_invalid_base(InvalidWorktreeBase::MalformedConfig);
}

#[test]
fn status_fails_on_a_missing_config_base_dir() {
    assert_status_fails_on_invalid_base(InvalidWorktreeBase::MissingConfigBaseDir);
}

/// Writes a second kache config that pins the fixture's own store with
/// `ignore_env = true`, so selecting it changes which file kache reads and
/// nothing else, and returns its path.
fn write_alternate_kache_config(fixture: &KacheHostFixture) -> PathBuf {
    let dir = fixture.home().join("alternate-kache");
    std::fs::create_dir_all(&dir).expect("alternate config directory");
    let config = dir.join("config.toml");
    std::fs::write(
        &config,
        format!(
            "[cache]\nlocal_store = \"{}\"\nignore_env = true\n",
            fixture.store().display()
        ),
    )
    .expect("write alternate kache config");
    config
}

/// `KACHE_CONFIG` selects another file whatever `ignore_env` says, so the
/// managed pin no longer governs this shell's builds — even while that file
/// happens to name the same store.
#[test]
fn status_fails_when_kache_config_selects_another_config_file() {
    let mut fixture = KacheHostFixture::new(&repo_inputs());
    let alternate = write_alternate_kache_config(&fixture);
    fixture.set_kache_config_env(alternate.clone());

    assert_drift(
        &fixture,
        &format!(
            "this shell exports KACHE_CONFIG={}, so kache reads that file, not {}",
            alternate.display(),
            fixture.kache_config().display()
        ),
    );
}

#[test]
fn status_fails_when_the_daemon_loaded_another_config_file() {
    let fixture = KacheHostFixture::new(&repo_inputs());
    let alternate = write_alternate_kache_config(&fixture);
    fixture.set_daemon_service_config(&alternate);

    assert_drift(
        &fixture,
        &format!(
            "the running daemon loaded {}, not {}",
            alternate.display(),
            fixture.kache_config().display()
        ),
    );
}
