//! Contracts for the kache recipes in the root justfile
//! (`install-kache`, `_ensure-kache`, `kache-status`), as reworked by
//! fixes/2026-09-23-ensuring-kache-support (2026-09-23).
//!
//! These pin the structural contract of the recipes the way
//! `ci_workflow_contracts.rs` always has — by asserting the justfile text —
//! because the behaviors are host-mutating (install, sign, daemon, activation)
//! and cannot run inside a test: the installer always targets latest with a
//! binary-only mode, the macOS re-sign gate lives in the installer, init
//! decides through the shared probe and runs the spec §4 order under the
//! failure contract, and the status recipe reads the store from the same
//! probe instead of reconstructing kache's resolution rules. The full D1/D2
//! rework (CI-side assertions) stays in `ci_workflow_contracts.rs`.

use std::{fs, path::PathBuf};

fn repo_root() -> PathBuf {
    let manifest_dir = biscuit_test_harness::manifest_dir!();
    manifest_dir
        .parent()
        .and_then(|path| path.parent())
        .expect("test-toolkit must live under <repo>/tools/test-toolkit")
        .to_path_buf()
}

fn read(relative: &str) -> String {
    let path = repo_root().join(relative);
    // Working trees on Windows check out CRLF while the index holds LF;
    // contract scanning must not depend on the host's checkout convention.
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
        .replace("\r\n", "\n")
}

/// One recipe's full text: the header line and every body line up to (not
/// including) the next recipe definition. A recipe definition starts at
/// column 0, mentions `:` but not `:=` (assignments), and is not a comment.
fn recipe(name_prefix: &str) -> String {
    let justfile = read("justfile");
    let mut block = String::new();
    let mut inside = false;
    for line in justfile.lines() {
        let is_header = !line.is_empty()
            && !line.starts_with(' ')
            && !line.starts_with('\t')
            && !line.starts_with('#')
            && !line.starts_with("import")
            && line.contains(':')
            && !line.contains(":=");
        if is_header {
            if inside {
                return block;
            }
            inside = line.starts_with(name_prefix);
        }
        if inside {
            block.push_str(line);
            block.push('\n');
        }
    }
    assert!(
        !block.is_empty(),
        "the justfile must define a recipe starting with `{name_prefix}`"
    );
    block
}

/// The installer always targets the latest release (a same-version host is a
/// binstall no-op, so a second `just init` changes nothing) and exposes the
/// binary-only mode as a positional boolean. The meets-floor
/// "not reinstalling" skip is gone: the floor is a check the other recipes
/// apply, never a pin.
#[test]
fn install_kache_targets_latest_in_a_binary_only_mode() {
    let block = recipe("install-kache");
    assert!(
        block.starts_with("install-kache binary_only=\"false\":"),
        "the recipe must declare the binary_only parameter with a full-mode default:\n{block}"
    );
    assert!(
        block.contains("cargo binstall --no-confirm kache"),
        "the install path is cargo-binstall against the latest release"
    );
    assert!(
        !block.contains("binstall --no-confirm --force"),
        "binstall --force would reinstall an already-latest binary on every init, breaking \
         idempotence (--force belongs to the source-install fallback alone)"
    );
    assert!(
        !block.contains("not reinstalling"),
        "the meets-floor skip is gone: install means install-or-upgrade"
    );
    assert!(
        block.contains("must be exactly 'true' or 'false', passed positionally"),
        "a mistyped mode value is a usage error, not a silent full run"
    );
}

/// On macOS every install or upgrade re-signs the binary ad hoc and is gated
/// on the shared env-passthrough probe, with a source install as the
/// fallback — so no upgrade path can skip the re-sign (spec §3).
#[test]
fn install_kache_resigns_and_gates_on_macos_with_source_fallback() {
    let block = recipe("install-kache");
    assert!(
        block.contains("codesign --force -s - \"$(command -v kache)\""),
        "the ad hoc re-sign runs against the installed binary immediately after install"
    );
    assert!(
        block.contains("./scripts/kache-host.sh probe-passthrough"),
        "the gate is the shared probe's verification, not the re-sign itself"
    );
    assert!(
        block.contains("cargo install --locked --force kache"),
        "a re-signed binary that still strips DYLD_* falls back to a source install; --force \
         replaces the binary cargo would otherwise skip as already installed"
    );
    // The probe runs first: an already re-signed binary is left untouched, so
    // a same-version init rewrites nothing and restarts no daemon (spec
    // verification check 5). A fresh binstall release is hardened, fails the
    // probe, and is re-signed — the re-sign cannot be skipped by any upgrade.
    let probe = block
        .find("./scripts/kache-host.sh probe-passthrough")
        .expect("the shared probe gates the install");
    let resign = block
        .find("codesign --force -s -")
        .expect("the ad hoc re-sign exists");
    assert!(
        probe < resign,
        "the passthrough probe must gate BEFORE the re-sign fires"
    );
}

/// The ownership split (2026-09-23 ruling): install-kache owns the binary and
/// nothing else — no daemon work in either mode, and config writes only in
/// full mode.
#[test]
fn install_kache_owns_the_binary_and_nothing_else() {
    let block = recipe("install-kache");
    assert!(
        !block.contains("kache daemon"),
        "daemon lifecycle belongs to _ensure-kache's post-config-write step, never here"
    );
    let mode_gate = block
        .find("if [[ \"{{ binary_only }}\" == \"false\" ]]; then")
        .expect("the config seed must sit behind the full-mode gate");
    let seed = block
        .find("local_max_size")
        .expect("the default store-cap seed stays in full mode");
    assert!(
        mode_gate < seed,
        "the store-cap seed is a config write and therefore full-mode only"
    );
}

/// init decides through the shared filesystem probe, never the OS name, and
/// a non-qualifying verdict is terminal for the sequence.
#[test]
fn ensure_kache_decides_through_the_shared_probe() {
    let block = recipe("_ensure-kache");
    assert!(
        block.contains("./scripts/kache-host.sh qualify"),
        "qualification runs the shared probe script"
    );
    assert!(
        block.contains("kache-host: verdict=no-qualify reason="),
        "the non-qualifying branch parses the probe's verdict line"
    );
    for stale in ["skipped on WSL", "skipped on Windows"] {
        assert!(
            !block.contains(stale),
            "the OS-name policy text is gone (`{stale}`)"
        );
    }
}

/// CI never installs or upgrades kache; if a workflow ever runs `just init`,
/// the step steps aside before probing (2026-09-23 ruling).
#[test]
fn ensure_kache_steps_aside_in_ci_environments() {
    let block = recipe("_ensure-kache");
    let guard = block
        .find("${CI:-}")
        .or_else(|| block.find("${GITHUB_ACTIONS:-}"))
        .expect("the CI guard reads CI/GITHUB_ACTIONS");
    let probe = block
        .find("./scripts/kache-host.sh qualify")
        .expect("the probe runs");
    assert!(
        guard < probe,
        "the CI guard must run before anything that could install or upgrade"
    );
}

/// The qualifying path runs the spec §4 order, textually: install, placement
/// from doctor, the user config write, the passthrough re-assert, the daemon
/// lifecycle strictly after the config write, and activation last. The
/// config write always sets the ratified pair — the pin is deliberate even
/// when it equals kache's default.
#[test]
fn ensure_kache_runs_the_spec_section_4_order() {
    let block = recipe("_ensure-kache");
    let install = block
        .find("just install-kache")
        .expect("step 2 installs or upgrades via the explicit recipe");
    let placement = block
        .find("./scripts/kache-host.sh report")
        .expect("step 3 reads the store through the shared report");
    let config_write = block
        .find("kache-config-merge.py \"$kache_config\" cache local_store \"$store\" ignore_env true")
        .expect("step 4 writes the ratified local_store + ignore_env");
    let passthrough = block
        .find("./scripts/kache-host.sh probe-passthrough")
        .expect("step 5 re-asserts the passthrough gate");
    let daemon = block
        .find("kache daemon --json")
        .expect("step 6 reads the daemon state");
    let activation = block
        .find("kache-config-merge.py \"$cargo_home/config.toml\" build rustc-wrapper kache")
        .expect("step 7 activates through the Cargo home config");
    for (earlier, later, why) in [
        (install, placement, "placement follows install"),
        (placement, config_write, "the config write follows placement"),
        (config_write, passthrough, "the passthrough gate follows the config write"),
        (passthrough, daemon, "the daemon lifecycle follows the config write"),
        (daemon, activation, "activation is last"),
    ] {
        assert!(
            earlier < later,
            "spec §4 ordering violated ({why}): {earlier} vs {later}"
        );
    }
}

/// A below-floor install on a non-qualifying host upgrades only after an
/// interactive confirmation, in binary-only mode; a non-interactive init
/// stops with an error instead of upgrading or skipping silently.
#[test]
fn ensure_kache_upgrades_below_floor_only_when_confirmed() {
    let block = recipe("_ensure-kache");
    assert!(
        block.contains("[ -t 0 ]"),
        "interactivity is decided on stdin being a TTY"
    );
    assert!(
        block.contains("just install-kache true"),
        "the confirmed upgrade is the binary-only mode"
    );
    assert!(
        block.contains("non-interactive"),
        "the non-interactive path names what it is"
    );
    assert!(
        block.contains("refusing to upgrade or skip silently") && block.contains("exit 1"),
        "a non-interactive below-floor host stops with an error, not a warning"
    );
}

/// The failure contract (spec §4): every pre-activation step routes through
/// one helper that prints WARNING lines, leaves kache off, and undoes an
/// activation an earlier run wrote — the empty rustc-wrapper value is the
/// neutralized form, applied through the same validated merge helper.
#[test]
fn ensure_kache_failure_contract_leaves_kache_off() {
    let block = recipe("_ensure-kache");
    assert!(
        block.contains("just install-kache || kache_off"),
        "an install failure ends the sequence through the failure contract"
    );
    assert!(
        block.contains("build rustc-wrapper \"\""),
        "the undo neutralizes an existing activation through the merge helper"
    );
    assert!(
        block.contains("kache left OFF; 'just init' continues"),
        "a pre-activation failure lets init complete its other steps"
    );
    assert!(
        block.contains("warn "),
        "failures are loud (WARNING lines)"
    );
}

/// The report block (spec §6): verdict with devices, the worktree-base line,
/// version against the floor, passthrough, the store and whether it moved,
/// the old/abandoned store size (reported, never deleted), daemon state, and
/// the activation state.
#[test]
fn ensure_kache_ends_with_the_one_report_block() {
    let block = recipe("_ensure-kache");
    for key in [
        "say \"verdict\"",
        "say \"worktree base\"",
        "say \"version\"",
        "say \"passthrough\"",
        "say \"store\"",
        "say \"old store\"",
        "say \"daemon\"",
        "say \"activation\"",
    ] {
        assert!(
            block.contains(key),
            "the spec §6 report block must carry `{key}`"
        );
    }
    assert!(
        block.contains("du -sh"),
        "an abandoned store's size is reported; its deletion stays with the human"
    );
}

/// kache-status reads the store, devices, base, and passthrough from the
/// shared probe (`kache doctor` behind it) — never a reconstruction of
/// kache's resolution rules, which produced false verdicts — keeps the
/// activation-precedence reporting, and fails loudly on drift while active.
#[test]
fn kache_status_shares_the_probe_and_fails_loudly_on_drift() {
    let block = recipe("kache-status");
    assert!(
        block.contains("./scripts/kache-host.sh report"),
        "status facts come from the same probe the init verdict used"
    );
    assert!(
        !block.contains("KACHE_DIR"),
        "the KACHE_DIR reconstruction behind the false verdicts is gone"
    );
    assert!(
        block.contains("repo .cargo/config.toml") && block.contains("cargo_home/config.toml"),
        "the activation-precedence reporting (env / repo / Cargo home) stays"
    );
    assert!(
        block.contains(r#"export RUSTC_WRAPPER=\"\""#),
        "the empty-value neutralization guidance stays, restated for the config-file era"
    );
    assert!(
        block.contains("exit 1"),
        "drift while active fails loudly (non-zero exit)"
    );
}

/// No recipe text may still assert the superseded policy anywhere in the
/// justfile — comment drift is checked in the same change as the behavior
/// (repo rule).
#[test]
fn justfile_carries_no_stale_kache_policy_text() {
    let justfile = read("justfile");
    for stale in [
        "Installed, NOT activated",
        "never reinstall",
        "never reinstalling",
        "Activation stays a host decision",
        "Exists because activation is HOST policy",
        "does NOT activate it",
    ] {
        assert!(
            !justfile.contains(stale),
            "stale policy text `{stale}` remains in the justfile"
        );
    }
}
