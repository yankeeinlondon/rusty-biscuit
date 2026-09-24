//! Behavioral contracts for the root justfile's `_ensure-kache` (the kache
//! step of `just init`, fixes/2026-09-23-ensuring-kache-support), run as a
//! real subprocess on an isolated fixture host (`common::kache`).
#![cfg(unix)]

mod common;

use std::path::PathBuf;

use common::kache::{KacheHostFixture, RepoInputs};

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

/// A host whose checkout cannot be cloned into, so qualification fails and
/// `_ensure-kache` takes its report-only path.
fn non_qualifying_host() -> KacheHostFixture {
    let fixture = KacheHostFixture::new(&repo_inputs());
    fixture.fail_clones_into(&["checkout"]);
    fixture
}

fn ensure(fixture: &KacheHostFixture) -> (Option<i32>, String) {
    let output = fixture.just("_ensure-kache");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr);
    (
        output.status.code(),
        format!("{stdout}\n--- stderr ---\n{stderr}"),
    )
}

const ACTIVE_WARNING: &str = "kache is ACTIVE on a filesystem that does not earn it";

/// The neutralized `rustc-wrapper = ""` that `_ensure-kache`'s own failure
/// path writes disables wrapping; re-running init on a non-qualifying host
/// used to match the key's text and report it as active kache.
#[test]
fn ensure_does_not_report_a_neutralized_wrapper_as_active_on_a_non_qualifying_host() {
    let fixture = non_qualifying_host();
    fixture.write_config_wrapper("");

    let (code, output) = ensure(&fixture);

    assert_eq!(
        code,
        Some(0),
        "a non-qualifying host is not an error:\n{output}"
    );
    assert!(
        output.contains("this filesystem does not earn kache"),
        "the fixture takes the non-qualifying path:\n{output}"
    );
    assert!(
        !output.contains(ACTIVE_WARNING),
        "an empty wrapper is not active kache:\n{output}"
    );
}

/// The control: a real kache activation on the same host is reported.
#[test]
fn ensure_reports_a_kache_wrapper_as_active_on_a_non_qualifying_host() {
    let fixture = non_qualifying_host();
    fixture.activate_wrapper();

    let (code, output) = ensure(&fixture);

    assert_eq!(
        code,
        Some(0),
        "a non-qualifying host is not an error:\n{output}"
    );
    assert!(
        output.contains(ACTIVE_WARNING),
        "an active kache on a non-qualifying host is reported:\n{output}"
    );
}

/// A Windows host whose checkout is on a ReFS Dev Drive (`B:`) while
/// `%LOCALAPPDATA%` and the store kache resolves by default sit on the NTFS
/// system drive: the layout a fresh Dev Drive host has.
fn fresh_refs_dev_drive_host() -> KacheHostFixture {
    let mut fixture = KacheHostFixture::new(&repo_inputs());
    fixture.emulate_windows();
    fixture.place_off_device(&[&fixture.local_app_data(), &fixture.store()]);
    fixture
}

/// The qualifying Windows layout must get a store on the Dev Drive and be
/// activated. The ReFS branch used to publish `candidate=-`, so a default
/// store off the Dev Drive left placement undecidable and init turned kache
/// off on exactly the host the spec qualifies.
#[test]
fn ensure_places_the_store_on_the_refs_drive_when_the_default_is_off_device() {
    let fixture = fresh_refs_dev_drive_host();
    let drive_root_store = fixture.windows_volume_root().join("kache");

    let (code, output) = ensure(&fixture);

    assert_eq!(code, Some(0), "{output}");
    assert!(
        output.contains(&format!(
            "kache-host: verdict=qualify candidate={}\n",
            drive_root_store.display()
        )),
        "the probe names kache\\ at the ReFS drive root as the candidate:\n{output}"
    );
    assert!(
        output.contains(&format!(
            "store          {} — moved: yes",
            drive_root_store.display()
        )),
        "placement uses the candidate instead of turning kache off:\n{output}"
    );
    let config = std::fs::read_to_string(fixture.kache_config()).expect("kache config");
    assert!(
        config.contains(&format!("local_store = \"{}\"", drive_root_store.display())),
        "the ReFS store is pinned in the user config:\n{config}"
    );
    let cargo_config = std::fs::read_to_string(fixture.cargo_home().join("config.toml"))
        .expect("activation written");
    assert!(
        cargo_config.contains("rustc-wrapper = \"kache\""),
        "kache is activated:\n{cargo_config}"
    );
}

/// When `%LOCALAPPDATA%` is itself on the ReFS volume, its `kache` directory
/// (kache's Windows default) is the candidate, ahead of the drive root.
#[test]
fn ensure_prefers_localappdata_when_it_is_on_the_refs_drive() {
    let mut fixture = KacheHostFixture::new(&repo_inputs());
    fixture.emulate_windows();
    fixture.place_off_device(&[&fixture.store()]);
    let user_cache_store = fixture.local_app_data().join("kache");

    let (code, output) = ensure(&fixture);

    assert_eq!(code, Some(0), "{output}");
    assert!(
        output.contains(&format!(
            "store          {} — moved: yes",
            user_cache_store.display()
        )),
        "placement uses %LOCALAPPDATA%\\kache on the serving drive:\n{output}"
    );
    assert!(
        !fixture.windows_volume_root().join("kache").exists(),
        "nothing is created at the drive root:\n{output}"
    );
}

/// A worktree base on the ReFS serving drive is reported with the same
/// filesystem-type decision that qualified the checkout. The base line used
/// to call the Unix `cp --reflink=always` probe, which fails under Git Bash,
/// so init printed `FAILED` for a base `kache-status` reports as covered.
#[test]
fn ensure_reports_a_refs_worktree_base_by_filesystem_type() {
    let mut fixture = fresh_refs_dev_drive_host();
    let base = fixture.configure_worktree_base();
    let base_dir_name = base.file_name().and_then(|name| name.to_str()).expect("base name");
    fixture.fail_clones_into(&[base_dir_name]);

    let (code, output) = ensure(&fixture);

    assert_eq!(code, Some(0), "{output}");
    assert!(
        output.contains("kache-host: base=covered"),
        "the base is on the serving drive:\n{output}"
    );
    assert!(
        output.contains(&format!(
            "worktree base {} is on the serving device; clone check store -> base: clone\n",
            base.display()
        )),
        "a ReFS base clones:\n{output}"
    );
    let reflink_attempts: Vec<String> = fixture
        .commands()
        .into_iter()
        .filter(|command| command.starts_with("clone "))
        .collect();
    assert!(
        reflink_attempts.is_empty(),
        "the Windows path never runs a userspace reflink probe: {reflink_attempts:?}\n{output}"
    );
}

/// The control: an NTFS checkout does not qualify, and the cascade never
/// runs, so nothing is created on the drive.
#[test]
fn ensure_leaves_an_ntfs_windows_checkout_without_a_store() {
    let fixture = fresh_refs_dev_drive_host();
    fixture.set_windows_checkout_fstype("NTFS");

    let (code, output) = ensure(&fixture);

    assert_eq!(code, Some(0), "{output}");
    assert!(
        output.contains("kache-host: verdict=no-qualify reason=filesystem=NTFS-is-not-ReFS"),
        "{output}"
    );
    assert!(
        !fixture.windows_volume_root().join("kache").exists(),
        "no store directory is created on a non-qualifying drive:\n{output}"
    );
}

/// A qualifying host whose `just install-kache` fails, so `_ensure-kache`
/// ends at its first pre-activation step through the failure contract.
fn failing_install_host() -> KacheHostFixture {
    let fixture = KacheHostFixture::new(&repo_inputs());
    fixture.fail_kache_install();
    fixture
}

const LEFT_OFF: &str = "kache left OFF";
const STILL_ACTIVE: &str = "kache STILL ACTIVE — manual action required";

/// The control: an earlier run's activation in a writable
/// `$CARGO_HOME/config.toml` is neutralized, and only then is kache called
/// off. Init continues, so the recipe exits 0.
#[test]
fn ensure_neutralizes_a_writable_activation_when_a_step_fails() {
    let fixture = failing_install_host();
    fixture.activate_wrapper();

    let (code, output) = ensure(&fixture);

    assert_eq!(
        code,
        Some(0),
        "init continues past a kache failure:\n{output}"
    );
    assert!(output.contains("install-kache failed"), "{output}");
    let config =
        std::fs::read_to_string(fixture.cargo_home().join("config.toml")).expect("Cargo config");
    assert!(
        config.contains("rustc-wrapper = \"\""),
        "the activation is neutralized:\n{config}"
    );
    assert!(output.contains(LEFT_OFF), "{output}");
    assert!(!output.contains(STILL_ACTIVE), "{output}");
}

/// An activation in a legacy `$CARGO_HOME/config` is the one Cargo runs even
/// with a `config.toml` beside it, so that is the file the failure path
/// neutralizes.
#[test]
fn ensure_neutralizes_an_activation_in_a_legacy_cargo_home_config_when_a_step_fails() {
    let fixture = failing_install_host();
    fixture.write_legacy_cargo_config("[build]\nrustc-wrapper = \"kache\"\n");
    fixture.write_config_wrapper("sccache");

    let (code, output) = ensure(&fixture);

    assert_eq!(code, Some(0), "init continues past a kache failure:\n{output}");
    let legacy =
        std::fs::read_to_string(fixture.cargo_home().join("config")).expect("legacy config");
    assert!(
        legacy.contains("rustc-wrapper = \"\""),
        "the activation Cargo reads is neutralized:\n{legacy}"
    );
    assert!(output.contains(LEFT_OFF), "{output}");
    assert!(!output.contains(STILL_ACTIVE), "{output}");
}

/// An inherited `RUSTC_WRAPPER=kache` outranks every config file, so no
/// neutralization turns kache off; the failure path used to claim it had.
#[test]
fn ensure_reports_an_inherited_kache_wrapper_as_still_active_when_a_step_fails() {
    let mut fixture = failing_install_host();
    fixture.activate_wrapper();
    fixture.set_rustc_wrapper_env("kache");

    let (code, output) = ensure(&fixture);

    assert_eq!(
        code,
        Some(0),
        "init continues past a kache failure:\n{output}"
    );
    assert!(
        !output.contains(LEFT_OFF),
        "kache is not off while RUSTC_WRAPPER names it:\n{output}"
    );
    assert!(output.contains(STILL_ACTIVE), "{output}");
    assert!(
        output.contains("environment RUSTC_WRAPPER — undo: 'unset RUSTC_WRAPPER'"),
        "the environment source and its undo are named:\n{output}"
    );
    assert!(
        !output.contains(&format!(
            "{}/config.toml — undo",
            fixture.cargo_home().display()
        )),
        "the neutralized config no longer needs a manual undo:\n{output}"
    );
}

/// A kache activation in a parent directory's `.cargo/config.toml` is one
/// Cargo reads from the checkout and the failure path does not edit, so kache
/// stays on; the helper used to miss ancestor configs and claim it was off.
#[test]
fn ensure_reports_a_parent_directory_kache_wrapper_as_still_active_when_a_step_fails() {
    let fixture = failing_install_host();
    let parent_config = fixture.write_parent_config_wrapper("kache");

    let (code, output) = ensure(&fixture);

    assert_eq!(
        code,
        Some(0),
        "init continues past a kache failure:\n{output}"
    );
    assert!(
        !output.contains(LEFT_OFF),
        "kache is not off while a parent config names it:\n{output}"
    );
    assert!(output.contains(STILL_ACTIVE), "{output}");
    assert!(
        output.contains(&format!(
            "{} — undo: set [build] rustc-wrapper = \"\" there",
            parent_config.display()
        )),
        "the parent config and its undo are named:\n{output}"
    );
    let config = std::fs::read_to_string(&parent_config).expect("parent Cargo config");
    assert!(
        config.contains("rustc-wrapper = \"kache\""),
        "a config outside $CARGO_HOME is left for the human:\n{config}"
    );
}

/// An activation the helper cannot rewrite stays in force; the failure path
/// used to say so and then claim kache was off anyway.
#[test]
fn ensure_reports_an_unwritable_activation_as_still_active_when_a_step_fails() {
    let fixture = failing_install_host();
    fixture.activate_wrapper();
    if !fixture.make_cargo_home_read_only() {
        eprintln!("SKIP: a read-only directory is still writable here (running as root)");
        return;
    }
    let config_path = fixture.cargo_home().join("config.toml");

    let (code, output) = ensure(&fixture);

    assert_eq!(
        code,
        Some(0),
        "init continues past a kache failure:\n{output}"
    );
    assert!(
        output.contains(&format!(
            "could not neutralize the existing activation in {}",
            config_path.display()
        )),
        "{output}"
    );
    assert!(!output.contains(LEFT_OFF), "{output}");
    assert!(output.contains(STILL_ACTIVE), "{output}");
    assert!(
        output.contains(&format!(
            "{} — undo: set [build] rustc-wrapper = \"\" there",
            config_path.display()
        )),
        "the file that still activates kache is named:\n{output}"
    );
    let config = std::fs::read_to_string(&config_path).expect("Cargo config");
    assert!(config.contains("rustc-wrapper = \"kache\""), "{config}");
}

/// When the effective wrapper cannot be decided (a Cargo config that does not
/// parse), the failure path must not claim kache is off.
#[test]
fn ensure_does_not_claim_kache_off_when_the_wrapper_is_undecidable() {
    let fixture = failing_install_host();
    std::fs::write(fixture.cargo_home().join("config.toml"), "[build\n")
        .expect("write Cargo config");

    let (code, output) = ensure(&fixture);

    assert_eq!(
        code,
        Some(0),
        "init continues past a kache failure:\n{output}"
    );
    assert!(!output.contains(LEFT_OFF), "{output}");
    assert!(
        output.contains("kache MAY STILL BE ACTIVE — manual action required"),
        "{output}"
    );
    assert!(output.contains("unparseable-cargo-config"), "{output}");
}
