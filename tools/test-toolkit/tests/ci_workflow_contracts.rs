//! Durable CI/CD workflow contract tests.
//!
//! These guard the invariants established by the DevOps plan
//! (`features/2026-07-24-devops/`): optional build acceleration is opt-in, kache
//! has a single version authority, the primary workflow runs a bootstrap
//! preflight that gates area fan-out, and release automation follows successful
//! CI instead of racing it. They inspect the workflow/action source text so a
//! regression fails locally without a live GitHub Actions run.

use std::{fs, path::PathBuf};

fn repo_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(|path| path.parent())
        .expect("test-toolkit must live under <repo>/tools/test-toolkit")
        .to_path_buf()
}

fn read(relative: &str) -> String {
    let path = repo_root().join(relative);
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
}

fn workflow(name: &str) -> String {
    read(&format!(".github/workflows/{name}"))
}

// --- D1/D2: optional acceleration is opt-in with one version authority --------

#[test]
fn repository_ships_no_global_rustc_wrapper() {
    // A fresh checkout must run Cargo without kache. The repo must not commit a
    // global `rustc-wrapper` that names the optional binary.
    let config = repo_root().join(".cargo/config.toml");
    if config.exists() {
        let source = fs::read_to_string(&config).expect("read .cargo/config.toml");
        assert!(
            !source.contains("rustc-wrapper"),
            ".cargo/config.toml must not pin a global rustc-wrapper (kache is opt-in)"
        );
    }
}

#[test]
fn kache_has_a_single_version_authority() {
    let version = read(".github/kache-version");
    assert!(
        !version.trim().is_empty(),
        ".github/kache-version must hold the single pinned kache version"
    );

    // The justfile must consume that same file, not carry an independent literal.
    let justfile = read("justfile");
    assert!(
        justfile.contains(".github/kache-version"),
        "root justfile must read KACHE_VERSION from the single authority file"
    );
    assert!(
        !justfile.contains(r#"KACHE_VERSION := ""#),
        "root justfile must not hard-code a second kache version literal"
    );
}

#[test]
fn area_ci_activates_kache_through_the_verified_composite_action() {
    let shared = workflow("_area-ci.yml");
    assert!(
        shared.contains("uses: ./.github/actions/enable-kache"),
        "area CI must enable kache through the shared verifying composite action"
    );
    assert!(
        !shared.contains("kunobi-ninja/kache-action"),
        "area CI must not call the raw kache action directly (bypasses version verification)"
    );
    assert!(
        !shared.contains("version: 0.8.0"),
        "area CI must not carry a duplicate hard-coded kache version literal"
    );
    // Opt-in gating flows through the composite's `enabled` input, NOT an `if:`
    // on the `uses:` step — a step that combines `if: ${{ inputs.kache }}` with a
    // local composite `uses:` fails to load on the runner.
    assert!(
        shared.contains("enabled: ${{ inputs.kache && runner.os != 'Windows' }}"),
        "kache must be opt-in AND Linux/macOS-only (kache-action@v1 rejects win32-x64)"
    );

    // The composite action is the single point that reads and verifies the pin,
    // and gates itself on its declared `enabled` input.
    let action = read(".github/actions/enable-kache/action.yml");
    assert!(
        action.contains("enabled:") && action.contains("inputs.enabled == 'true'"),
        "enable-kache must gate on a declared `enabled` input, not a caller `if:`"
    );
    assert!(
        action.contains(".github/kache-version"),
        "enable-kache must resolve the pinned version from the single authority"
    );
    assert!(
        action.contains("kache --version"),
        "enable-kache must verify the active wrapper version before Cargo runs"
    );
    assert!(
        action.contains("kache bootstrap"),
        "a missing or mismatched kache must fail a named bootstrap step"
    );
}

// --- D3: bootstrap preflight gates area fan-out ------------------------------

#[test]
fn primary_ci_runs_a_bootstrap_preflight_before_fan_out() {
    let ci = workflow("ci.yml");
    assert!(
        ci.contains("preflight:"),
        "ci.yml must define a bootstrap preflight job"
    );
    assert!(
        ci.contains("fromJSON(needs.scope.outputs.preflight_os)"),
        "preflight breadth must come from the scope-derived OS matrix"
    );
    assert!(
        ci.contains(r#"RUSTC_WRAPPER: """#),
        "preflight must prove Cargo works on a clean checkout with no wrapper"
    );
    assert!(
        ci.contains("needs: [scope, preflight]"),
        "area fan-out must depend on a successful preflight"
    );
}

#[test]
fn area_ci_treats_rust_warnings_as_failures_and_runs_lint() {
    let shared = workflow("_area-ci.yml");
    assert!(
        shared.contains(r#"RUSTFLAGS: "-D warnings""#),
        "shared compile and test jobs must reject Rust warnings"
    );
    assert!(
        shared.contains(r#"run: cd "${{ inputs.area }}" && just lint"#),
        "shared area coverage must execute each area's lint and documentation guards"
    );
}

// --- area policy source of truth (areas.json) --------------------------------

#[test]
fn area_policy_retains_native_and_heavy_area_coverage() {
    let areas = read(".github/ci/areas.json");
    assert!(
        areas.contains(r#""full_os": ["ubuntu-latest", "windows-latest", "macos-latest"]"#),
        "sniff must retain native macOS, Linux, and Windows evidence in area policy"
    );
    assert!(
        areas.contains(r#""shards": ["1/4", "2/4", "3/4", "4/4"]"#),
        "darkmatter must retain measured L1 sharding"
    );
    for area in ["claudine", "darkmatter", "sniff", "biscuit-file"] {
        assert!(
            areas.contains(&format!(r#""area": "{area}""#)),
            "{area} must be an owned CI area"
        );
    }
}

// --- D13/OQ2/OQ3: release follows CI and stays hermetic -----------------------

#[test]
fn release_automation_follows_successful_ci() {
    let release = workflow("release-plz.yml");
    assert!(
        release.contains("workflow_run:") && release.contains(r#"workflows: ["ci"]"#),
        "release-plz must be triggered by the ci workflow, not race it"
    );
    assert!(
        release.contains("github.event.workflow_run.conclusion == 'success'"),
        "release-plz must run only after CI concludes successfully"
    );
    assert!(
        release.contains("github.event.workflow_run.head_branch == 'main'"),
        "release-plz must gate on main"
    );
    assert!(
        !release.contains("\n  push:\n"),
        "release-plz must not run release calculation on a bare push to main"
    );
}

#[test]
fn lockfiles_stay_gitignored_so_release_checkout_cannot_block() {
    // The release-plz isolation (OQ3 Option C) depends on every Cargo.lock being
    // gitignored: a regenerated schematic/Cargo.lock is then invisible to
    // `git status`/`git checkout` and cannot block release-plz returning to
    // `main`. If a lockfile is ever force-tracked, that premise breaks and the
    // original checkout-overwrite failure can recur — fail loudly here instead.
    let gitignore = read(".gitignore");
    assert!(
        gitignore.contains("**/Cargo.lock"),
        ".gitignore must keep every Cargo.lock ignored (release-plz isolation premise)"
    );
}

#[test]
fn release_calculation_asserts_a_clean_tracked_worktree() {
    let release = workflow("release-plz.yml");
    assert!(
        release.contains("git status --porcelain --untracked-files=no"),
        "release-plz must assert a clean tracked worktree around release calculation"
    );
    assert!(
        release.contains("gitignored"),
        "release-plz must document why the ignored lockfile cannot block checkout"
    );
}

// --- D12: specialized runtime evidence is preserved --------------------------

#[test]
fn claudine_preserves_native_windows_ctrl_c_evidence() {
    let windows = workflow("claudine-windows-ctrl-c.yml");
    assert!(
        windows.contains("windows_ctrl_c_verification_record"),
        "claudine must preserve the native Windows Ctrl+C runtime test"
    );
    assert!(
        windows.contains(r#"RUSTFLAGS: "-D warnings""#),
        "the specialized Windows runtime job must reject Rust warnings"
    );
}
