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

/// Splits a workflow's `jobs:` section into one block per job. Everything above
/// `jobs:` is dropped so an input's `description:` prose cannot be mistaken for a
/// step that runs the command it describes.
fn jobs(source: &str) -> Vec<String> {
    let jobs_section = source
        .split_once("\njobs:\n")
        .expect("every workflow declares a jobs: section")
        .1;

    let mut blocks: Vec<String> = Vec::new();
    for line in jobs_section.lines() {
        let opens_block = line.starts_with("  ")
            && !line.starts_with("   ")
            && line.ends_with(':')
            && !line.trim_start().starts_with(['#', '-']);
        if opens_block {
            blocks.push(String::new());
        }
        if let Some(block) = blocks.last_mut() {
            block.push_str(line);
            block.push('\n');
        }
    }
    blocks
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
fn expensive_tiers_stage_behind_l1_but_lint_never_gates_it() {
    let shared = workflow("_area-ci.yml");
    // The expensive L2 and browser tiers run only after L1 (D4).
    assert!(
        shared.matches("needs: test").count() >= 2,
        "the L2 and browser tiers must each depend on L1 (D4)"
    );
    // L1 does NOT stage behind lint. A clippy hint in one package must not
    // delete every L1 leg's evidence for the whole area.
    assert!(
        !shared.contains("needs: lint"),
        "the L1 test job must not depend on lint — a lint failure must not suppress test evidence"
    );
}

#[test]
fn a_failing_leg_can_never_be_dropped_from_the_area_verdict() {
    // `soft_os` drove `continue-on-error` on the L1 legs, which did not merely
    // make them non-blocking — it removed them from the run's verdict, so 14 red
    // Windows areas read as normal. A known failure belongs in the results
    // baseline, which keeps the signal.
    let shared = workflow("_area-ci.yml");
    assert!(
        !shared.contains("continue-on-error"),
        "no area gate may be continue-on-error — that erases the leg from the run's verdict"
    );
    for retired in ["soft-os", "soft_os"] {
        assert!(
            !shared.contains(retired),
            "_area-ci.yml must carry no trace of the retired `{retired}` policy"
        );
        assert!(
            !workflow("ci.yml").contains(retired),
            "ci.yml must not pass the retired `{retired}` policy to the reusable workflow"
        );
        assert!(
            !read(".github/ci/areas.json").contains(retired),
            "areas.json must not declare the retired `{retired}` policy"
        );
    }
}

#[test]
fn area_ci_selects_the_ci_nextest_profile_explicitly() {
    let shared = workflow("_area-ci.yml");
    assert!(
        shared.contains("NEXTEST_PROFILE: ci"),
        "area CI must explicitly select the `ci` nextest profile, not rely on detection (D6)"
    );
}

#[test]
fn global_changes_run_canaries_before_full_fan_out() {
    let ci = workflow("ci.yml");
    assert!(
        ci.contains("canary:") && ci.contains("fromJSON(needs.scope.outputs.canary_matrix)"),
        "ci.yml must define a canary stage driven by the scope-derived canary matrix (D11)"
    );
    assert!(
        ci.contains("has_canaries"),
        "the canary stage must gate on a full-scope canary selection"
    );
    assert!(
        ci.contains("needs: [scope, preflight, canary]")
            && ci.contains("needs.canary.result == 'success' || needs.canary.result == 'skipped'"),
        "the area fan-out must not start after a canary failure (D11)"
    );
    let areas = read(".github/ci/areas.json");
    assert!(
        areas.contains(r#""canary": true"#),
        "areas.json must declare initial global-change canaries (D11)"
    );
}

#[test]
fn scope_job_emits_an_actionable_summary() {
    let ci = workflow("ci.yml");
    assert!(
        ci.contains("$GITHUB_STEP_SUMMARY") && ci.contains("## CI scope"),
        "the scope job must write an actionable scope summary to the step summary (D15)"
    );
}

#[test]
fn area_ci_treats_rust_warnings_as_failures_and_runs_lint() {
    let shared = workflow("_area-ci.yml");
    // `-D warnings` belongs to the gates whose JOB is to reject warnings. At
    // workflow level it also applied to the test job's compilation, so a plain
    // rustc warning failed the build and no test ran — re-coupling lint and test
    // through the back door after `needs: lint` was removed.
    assert!(
        !shared.contains("\n  RUSTFLAGS:"),
        "RUSTFLAGS must not be set at workflow level — it would deny warnings in the test jobs too"
    );
    assert_eq!(
        shared.matches(r#"RUSTFLAGS: "-D warnings""#).count(),
        1,
        "lint is the only gate that may reject Rust warnings"
    );
    for job in jobs(&shared) {
        let denies_warnings = job.contains(r#"RUSTFLAGS: "-D warnings""#);
        let is_warning_gate = job.starts_with("  lint:");
        assert_eq!(
            denies_warnings,
            is_warning_gate,
            "only the lint gate may deny warnings; offending job: {}",
            job.lines().next().unwrap_or_default()
        );
    }
    assert!(
        shared.contains(r#"run: cd "${{ inputs.area }}" && just lint"#),
        "shared area coverage must execute each area's lint and documentation guards"
    );
    // The lint gate's authority is the recipe itself, not a CI-only variable, so
    // a local `just lint` enforces the same bar.
    let devops = read("just/devops.just");
    assert!(
        devops.contains("cargo clippy") && devops.contains("-- -D warnings"),
        "`_lint` must pass -D warnings to clippy directly so lint denies warnings off CI too"
    );

    // The compile gate must stay a COMPILE gate. Promoting warnings there made a
    // dead-code hint report as `error: could not compile <crate>`, attributed a
    // dependency's warning to whichever area happened to build it, and failed in
    // a way `just check` — which sets no RUSTFLAGS — could not reproduce.
    let check = jobs(&shared)
        .into_iter()
        .find(|job| job.starts_with("  check:"))
        .expect("_area-ci.yml must define the compile-check job");
    assert!(
        !check.contains("RUSTFLAGS"),
        "the compile check must not promote warnings to errors — a warning is not \
         a build failure, and cross-platform dead code is normal"
    );
}

// --- area policy source of truth (areas.json) --------------------------------

#[test]
fn area_policy_retains_native_and_heavy_area_coverage() {
    let areas = read(".github/ci/areas.json");
    // Native macOS/Linux/Windows L1 evidence is now the DEFAULT for every area,
    // not a per-area override `sniff` happened to carry. Asserting the default
    // covers all 21 gating areas instead of one.
    let policy = read("scripts/ci/affected_scope.py");
    let default_environments = policy
        .lines()
        .find(|line| line.trim_start().starts_with(r#""environments":"#))
        .expect("affected_scope.py must declare a default `environments` list");
    // All four supported environments, not three: WSL2 is hosted by a Windows
    // runner but is a distinct Linux environment, and a green ubuntu-latest leg
    // says nothing about the 9p boundary, WSLg probes, or NAT'd networking.
    for environment in ["ubuntu-latest", "windows-latest", "macos-latest", "wsl2-ubuntu"] {
        assert!(
            default_environments.contains(environment),
            "every area must run {environment} L1 evidence by default; default list is: \
             {default_environments}"
        );
    }
    assert!(
        !areas.contains("full_os"),
        "`full_os` is retired: WSL2 is an environment a Windows runner hosts, not a runner label"
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

#[test]
fn l2_provisions_and_verifies_only_the_ci_capable_backend() {
    let shared = workflow("_area-ci.yml");
    // tmux/PTY is the only backend a headless runner can host; it is installed
    // AND its runtime reachability is verified (D8).
    assert!(
        shared.contains("apt-get install -y tmux") && shared.contains("tmux -V"),
        "the L2 job must provision AND verify the tmux backend"
    );
    // No global L2 hard-require: WezTerm/Kitty/Apple-backed tests must skip
    // cleanly here, not panic on a runner that cannot host their GUI.
    assert!(
        !shared.contains("BISCUIT_TEST_LEVEL_REQUIRED:"),
        "the L2 job must not SET a global L2 hard-require (would panic GUI-only tests)"
    );
    // Focus safety: CI must never run L3 or take foreground focus.
    assert!(
        !shared.contains("BISCUIT_L3_TAKE_FOCUS") && !shared.contains("test-l3"),
        "CI must not run L3 or enable focus-taking"
    );
}

#[test]
fn areas_declaring_native_deps_are_provisioned() {
    // Playa's Linux ALSA/PulseAudio headers must be declared as area policy and
    // installed before build/test — a missing lib is a provisioning failure, not
    // a product-test failure (D9).
    let areas = read(".github/ci/areas.json");
    assert!(
        areas.contains("libasound2-dev"),
        "playa must declare its Linux native audio prerequisites in areas.json"
    );

    // One installer, shared by developer hosts and CI: the root justfile recipe
    // reads areas.json directly. A second CI-only implementation would drift
    // from `just init`, so the retired composite must not come back.
    assert!(
        !repo_root().join(".github/actions/install-native").exists(),
        "the install-native composite must stay retired; `just _ensure-native-libs` is the one installer"
    );
    let justfile = read("justfile");
    assert!(
        justfile.contains("_ensure-native-libs area=\"\":")
            && justfile.contains(".github/ci/areas.json"),
        "the native-libs recipe must be area-scopable and read areas.json as the source of truth"
    );
    assert!(
        justfile.contains("init: ") && justfile.contains("_ensure-native-libs"),
        "`just init` must depend on the same recipe CI runs"
    );
}

#[test]
fn native_prerequisites_are_installed_before_anything_is_built() {
    // A `-sys` crate must never fail to compile for a missing system library, so
    // every job that builds runs the install step first (D9 / 2026-07-25 spec
    // decision "Native libraries: one isolated install step").
    const BUILD_COMMANDS: [&str; 6] = [
        "cargo check --all-targets",
        "cargo llvm-cov",
        "just test ",
        "just lint",
        "just test-l2",
        "just test-browser",
    ];
    const PROVISION: &str = "just _ensure-native-libs";

    let mut provisioning_jobs = 0;
    for name in ["_area-ci.yml", "ci.yml"] {
        let source = workflow(name);
        for job in jobs(&source) {
            let Some(provisioned_at) = job.find(PROVISION) else {
                for command in BUILD_COMMANDS {
                    assert!(
                        !job.contains(command),
                        "{name}: a job running `{command}` must first install native prerequisites"
                    );
                }
                continue;
            };
            provisioning_jobs += 1;
            for command in BUILD_COMMANDS {
                if let Some(built_at) = job.find(command) {
                    assert!(
                        provisioned_at < built_at,
                        "{name}: `{command}` must run after native prerequisites are installed"
                    );
                }
            }
        }
    }
    // check, test, lint, l2, browser in the reusable area workflow, plus the
    // workspace-wide affected-coverage job in ci.yml.
    assert_eq!(
        provisioning_jobs, 6,
        "every building CI job must provision native prerequisites"
    );
}

#[test]
fn heavy_areas_shard_l1_and_surface_all_failures() {
    // Both heavy areas (claudine ~3964 tests, darkmatter) declare 4-shard L1
    // policy sized from measured cold-run durations (~7-8 min/shard).
    let areas = read(".github/ci/areas.json");
    assert!(
        areas.matches(r#""shards": ["1/4", "2/4", "3/4", "4/4"]"#).count() >= 2,
        "claudine and darkmatter must both declare measured 4-shard L1 policy"
    );

    let shared = workflow("_area-ci.yml");
    assert!(
        shared.contains("--no-fail-fast"),
        "L1 shards must run --no-fail-fast so one failure cannot hide the shard's evidence (D7)"
    );
    assert!(
        shared.contains("actions/upload-artifact") && shared.contains("junit-"),
        "each test tier must publish collision-free per-shard JUnit artifacts (D7)"
    );
}

// --- D12: specialized runtime contracts are reusable and orchestrated ---------

/// Each specialized runtime workflow the primary orchestrator calls, paired with
/// the unique runtime evidence that must survive the move to `workflow_call`,
/// and the scope expression that selects it.
const ORCHESTRATED: [(&str, &str, &str); 4] = [
    (
        "rendezvous-tests.yml",
        "os: [macos-latest, ubuntu-latest, windows-latest]",
        "contains(fromJSON(needs.scope.outputs.area_names), 'sniff')",
    ),
    (
        "biscuit-tui-windows-captured-stdout.yml",
        "captured_stdout_receives_only_value_no_tui_bytes",
        "contains(fromJSON(needs.scope.outputs.area_names), 'biscuit-tui')",
    ),
    (
        "playa-windows.yml",
        "--features audio-ducking-windows",
        "contains(fromJSON(needs.scope.outputs.area_names), 'playa')",
    ),
    (
        // messenger is EXEMPT from area ownership, so it is selected from the
        // affected PACKAGE list rather than from `area_names`. Its unique
        // evidence is the desktop-feature build; the bespoke WSL1 job it used to
        // carry was deleted in favour of the shared `_wsl-ci.yml` mechanism.
        "messenger-desktop-tests.yml",
        "--features desktop",
        "contains(fromJSON(needs.scope.outputs.packages), 'messenger')",
    ),
];

#[test]
fn specialized_contracts_are_reusable_and_orchestrated_by_primary_ci() {
    let ci = workflow("ci.yml");
    for (name, evidence, selector) in ORCHESTRATED {
        let source = workflow(name);
        assert!(
            source.contains("workflow_call") && source.contains("workflow_dispatch"),
            "{name} must be reusable and still manually dispatchable (D12)"
        );
        assert!(
            !source.contains("\n  push:\n") && !source.contains("\n  pull_request:\n"),
            "{name} must not self-trigger once ci.yml orchestrates it — one CI run per commit"
        );
        assert!(
            !source.contains("\nconcurrency:\n"),
            "{name} must not carry its own concurrency group; ci.yml owns cancellation"
        );
        assert!(
            !source.contains("dtolnay/rust-toolchain"),
            "{name} must honor the pinned rust-toolchain.toml, not a floating @stable override"
        );
        assert!(
            source.contains("rustup show"),
            "{name} must materialize the pinned toolchain"
        );
        assert!(
            source.contains(evidence),
            "{name} must preserve its unique runtime evidence ({evidence})"
        );
        assert!(
            ci.contains(&format!("uses: ./.github/workflows/{name}")),
            "ci.yml must orchestrate {name} (D12)"
        );
        assert!(
            ci.contains(selector),
            "ci.yml must select {name} from affected scope, not run it unconditionally"
        );
    }
}

#[test]
fn release_artifact_builds_stay_out_of_per_commit_validation() {
    // build-integrations packages aarch64 binaries for a published release. It
    // has a different lifecycle from per-commit validation, so D12's "one primary
    // run per commit" does NOT absorb it.
    let integrations = workflow("build-integrations.yml");
    assert!(
        integrations.contains("release:") && integrations.contains("types: [published]"),
        "build-integrations must stay release-triggered"
    );
    assert!(
        !workflow("ci.yml").contains("build-integrations.yml"),
        "ci.yml must not call the release artifact build on every commit"
    );
}

#[test]
fn ci_summarizes_the_first_actionable_failure_class() {
    // D15: the advisory summary must name a failure CLASS, derived from job
    // results rather than log text, so a Node warning or cache collision inside a
    // passing job can never be promoted over the real root cause.
    //
    // Its SCOPE narrowed when `ci-verdict` landed. Every `areas.json` area is now
    // a rollup cell, so this job covers only what the rollup cannot see: the
    // bootstrap stages and the specialized workflows, which are not areas, emit
    // no `manifest.jsonl`, and can never appear in an affected scope.
    let ci = workflow("ci.yml");
    assert!(
        ci.contains("## Jobs outside the rollup") && ci.contains("First actionable failure class"),
        "ci.yml must write a failure-class summary for the un-rolled-up jobs (D15)"
    );
    for stage in [
        "bootstrap (scope calculation)",
        "bootstrap (preflight)",
        "canary (shared-change regression)",
        "coverage",
    ] {
        assert!(
            ci.contains(stage),
            "the failure-class summary must be able to report `{stage}`"
        );
    }
    for (name, _, _) in ORCHESTRATED {
        let job = name.trim_end_matches(".yml");
        assert!(
            ci.contains(&format!("uses: ./.github/workflows/{name}")),
            "{job} must be part of the orchestrated graph the summary classifies"
        );
    }
}

// --- D5: controlled required toolchain + latest-stable advisory ---------------

#[test]
fn required_ci_pins_an_exact_rust_toolchain() {
    let toolchain = read("rust-toolchain.toml");
    assert!(
        !toolchain.contains(r#"channel = "stable""#),
        "rust-toolchain.toml must not float on `stable` (fmt/clippy drift hazard)"
    );
    assert!(
        toolchain.contains(r#"channel = "1."#),
        "rust-toolchain.toml must pin an exact 1.x version"
    );
    assert!(
        toolchain.contains("clippy") && toolchain.contains("rustfmt"),
        "the pinned toolchain must ship clippy (lint) and rustfmt (read-only fmt check)"
    );
}

#[test]
fn required_ci_honors_the_toolchain_file_without_stable_override() {
    for name in ["ci.yml", "_area-ci.yml"] {
        let source = workflow(name);
        assert!(
            !source.contains("dtolnay/rust-toolchain@stable"),
            "{name} must honor rust-toolchain.toml, not override it with floating @stable"
        );
        assert!(
            source.contains("rustup show"),
            "{name} must materialize the pinned toolchain from rust-toolchain.toml"
        );
    }
}

#[test]
fn latest_stable_advisory_is_separate_and_non_required() {
    let advisory = workflow("rust-latest-stable.yml");
    assert!(
        advisory.contains("schedule:") && advisory.contains("workflow_dispatch"),
        "latest-stable must be a scheduled/manual advisory, not part of required CI"
    );
    assert!(
        advisory.contains("RUSTUP_TOOLCHAIN: stable"),
        "advisory must override the pin with floating latest stable"
    );
    assert!(
        advisory.contains("cargo fmt --all --check"),
        "advisory must run a read-only formatting check, never write-mode"
    );
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

// --- D14: scheduled automation is separate and bounded ------------------------

#[test]
fn benchmarks_are_scheduled_only_and_carry_a_measured_budget() {
    let bench = workflow("bench-nightly.yml");
    assert!(
        bench.contains("schedule:") && bench.contains("workflow_dispatch"),
        "bench-nightly must be a scheduled/manual performance workflow"
    );
    assert!(
        !bench.contains("\n  push:\n") && !bench.contains("\n  pull_request:\n"),
        "bench-nightly must not run on push — a slow benchmark is not a test regression (D14)"
    );
    // The budget is a reviewed number backed by recorded durations, not a
    // default. The comment block carries the measurement it was derived from.
    assert!(
        bench.contains("timeout-minutes: 90") && bench.contains("Timeout budget"),
        "the benchmark job must document the measurement its timeout budget came from (AC30)"
    );
    assert!(
        bench.contains("runner image") && bench.contains("toolchain"),
        "benchmark results must record runner class and toolchain so comparisons stay valid"
    );
}

#[test]
fn benchmark_upload_failure_cannot_erase_a_successful_measurement() {
    // AC31: execution and upload are separate steps. Execution gates the job;
    // the Bencher upload is best-effort and reads the captured output rather
    // than re-running the suite.
    let bench = workflow("bench-nightly.yml");
    let run_at = bench
        .find("- name: Run benchmarks")
        .expect("bench-nightly must have a benchmark execution step");
    let upload_at = bench
        .find("- name: Upload results to Bencher.dev")
        .expect("bench-nightly must have a separate upload step");
    assert!(run_at < upload_at, "execution must precede upload");

    // Bound to the upload step alone, so a `continue-on-error` belonging to a
    // later step cannot satisfy this.
    let rest = &bench[upload_at + 1..];
    let upload = &rest[..rest.find("- name: ").unwrap_or(rest.len())];
    assert!(
        upload.contains("continue-on-error: true"),
        "the upload step must be best-effort so it cannot fail an otherwise good measurement"
    );
    assert!(
        !bench[..upload_at].contains("continue-on-error: true"),
        "benchmark EXECUTION must not be continue-on-error — a broken bench must be visible"
    );
    assert!(
        upload.contains("steps.bench.outcome == 'success'"),
        "the upload must be tied to a successful measurement, not run unconditionally"
    );
}

#[test]
fn scheduled_workflows_are_operationally_distinct() {
    // AC32: coverage, fuzz, benchmark, and maintenance results must not be
    // mistakable for one another — distinct workflow names and distinct
    // schedule slots so they neither collide for runners nor blur together.
    const SCHEDULED: [&str; 5] = [
        "bench-nightly.yml",
        "fuzz-nightly.yml",
        "coverage.yml",
        "sniff-performance.yml",
        "maintenance-audit.yml",
    ];

    let mut names: Vec<String> = Vec::new();
    let mut crons: Vec<String> = Vec::new();
    for file in SCHEDULED {
        let source = workflow(file);
        let name = source
            .lines()
            .find_map(|line| line.strip_prefix("name: "))
            .unwrap_or_else(|| panic!("{file} must declare a workflow name"))
            .to_string();
        assert!(
            !names.contains(&name),
            "{file}: workflow name `{name}` is not unique"
        );
        names.push(name);

        for line in source.lines() {
            if let Some(cron) = line.trim().strip_prefix("- cron: ") {
                let cron = cron.split('#').next().unwrap_or(cron).trim().to_string();
                assert!(
                    !crons.contains(&cron),
                    "{file}: schedule slot {cron} collides with another scheduled workflow"
                );
                crons.push(cron);
            }
        }
    }
    assert_eq!(
        crons.len(),
        SCHEDULED.len(),
        "each scheduled workflow must own exactly one schedule slot"
    );
}

#[test]
fn maintenance_audit_reports_without_changing_anything() {
    // Task 5.4: findings stay advisory until a reviewed change advances a
    // pinned value. An audit that can red the Actions tab, or that bumps pins
    // on its own, is worse than none.
    let audit = workflow("maintenance-audit.yml");
    assert!(
        audit.contains("schedule:") && audit.contains("workflow_dispatch"),
        "the maintenance audit must be scheduled and manually dispatchable"
    );
    assert!(
        audit.contains("permissions:") && audit.contains("contents: read"),
        "the audit must hold read-only permissions — it never writes to the repository"
    );
    for forbidden in ["git commit", "git push", "create-pull-request", "peter-evans"] {
        assert!(
            !audit.contains(forbidden),
            "the maintenance audit must not change the repository (found `{forbidden}`)"
        );
    }
    for authority in ["rust-toolchain.toml", ".github/kache-version", "nextest"] {
        assert!(
            audit.contains(authority),
            "the audit must cover the pinned authority `{authority}`"
        );
    }
}

// --- environment is not os ----------------------------------------------------

#[test]
fn every_test_tier_stamps_its_result_identity() {
    // A rollup cell is keyed by {area, environment, tier, shard}, never by a
    // GitHub display name — for a skipped matrix leg the name expression is not
    // merely unstable, it is reported raw and unresolvable. The shared `just`
    // recipes read these three variables when staging each nextest report.
    for (file, expected_jobs) in [("_area-ci.yml", 3), ("_wsl-ci.yml", 1)] {
        let source = workflow(file);
        let mut stamped = 0;
        for job in jobs(&source) {
            if !job.contains("BISCUIT_CI_AREA:") {
                continue;
            }
            stamped += 1;
            for key in ["BISCUIT_CI_ENVIRONMENT:", "BISCUIT_CI_SHARD:"] {
                assert!(
                    job.contains(key),
                    "{file}: a job stamping BISCUIT_CI_AREA must also stamp {key}"
                );
            }
        }
        assert_eq!(
            stamped, expected_jobs,
            "{file}: every job that runs tests must stamp its result identity"
        );
    }
}

#[test]
fn wsl_is_an_environment_and_never_a_runner_label() {
    // A WSL job runs on `windows-latest` and executes through `wsl-bash`. If it
    // shared the native matrix, every `runner.os == 'Windows'` branch — kache
    // gating, native packages, paths, shells, cache keys, artifact names — would
    // apply to a Linux guest. Isolation is structural, not by review.
    let area = workflow("_area-ci.yml");
    assert!(
        !area.contains("wsl2-ubuntu") || area.contains("_wsl-ci.yml"),
        "_area-ci.yml must delegate wsl2-ubuntu rather than host it"
    );
    for job in jobs(&area) {
        if !(job.starts_with("  test:") || job.starts_with("  l2:")) {
            continue;
        }
        // Comments explain the separation; only executable YAML can violate it.
        let executable: String = job
            .lines()
            .filter(|line| !line.trim_start().starts_with('#'))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            !executable.contains("wsl"),
            "_area-ci.yml: wsl2-ubuntu must never enter a runs-on matrix"
        );
    }

    let wsl = workflow("_wsl-ci.yml");
    assert!(
        wsl.contains("BISCUIT_CI_ENVIRONMENT: wsl2-ubuntu") && wsl.contains("runs-on: windows-latest"),
        "the WSL job runs on a Windows runner but MUST report the wsl2-ubuntu environment"
    );
    // WSLg (GUI, audio, clipboard, D-Bus) is WSL2-only. Under WSL1 the
    // capability-probing packages take their fallback branches, so a green WSL1
    // leg would misreport them for every real user.
    let executable: String = wsl
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !executable.contains("wsl-version:"),
        "no wsl-version pin: the action defaults to 2 and WSL1 is not an acceptable fallback"
    );
    assert!(
        wsl.contains("microsoft-standard-WSL2"),
        "the WSL job must verify it actually got WSL2 before reporting any result"
    );
    // The 9p boundary penalty, and where real WSL developers keep repositories.
    assert!(
        wsl.contains("git clone") && wsl.contains("/home/runner/rusty-biscuit"),
        "the WSL job must check out onto ext4, not build over /mnt/c"
    );
    // Build once on Linux, run in the guest: no toolchain install, no compile,
    // binaries byte-identical to the native Linux leg's.
    assert!(
        wsl.contains("cargo nextest archive") && wsl.contains("--workspace-remap"),
        "the WSL leg must run from a nextest archive rather than compiling in the guest"
    );
    assert!(
        !wsl.contains("sh.rustup.rs"),
        "the WSL guest must not install a Rust toolchain"
    );
}

#[test]
fn artifact_names_carry_the_environment_not_the_runner_label() {
    // `junit-<area>-L1-windows-latest-*` and `junit-<area>-L1-wsl2-ubuntu-*` are
    // two different cells produced on the same runner label. Keying the artifact
    // to the label would merge them.
    let area = workflow("_area-ci.yml");
    assert!(
        area.contains("name: junit-${{ inputs.area }}-L1-${{ matrix.environment }}-"),
        "L1 artifacts must be keyed by environment"
    );
    assert!(
        area.contains("name: junit-${{ inputs.area }}-L2-${{ matrix.environment }}"),
        "L2 artifacts must be keyed by environment"
    );
    assert!(
        workflow("_wsl-ci.yml").contains("junit-${{ inputs.area }}-L1-wsl2-ubuntu-"),
        "the WSL leg's artifact must name the wsl2-ubuntu environment"
    );
}

#[test]
fn l2_runs_on_every_environment_with_a_provisioned_backend() {
    // tmux is the only backend headless CI can host, and it installs on BOTH
    // Linux (apt) and macOS (brew) — so L2 is a matrix, not a hardcoded ubuntu
    // leg. `tmux -V` needs no server, display, or pane, so verification takes no
    // focus; CI must never steal focus.
    let area = workflow("_area-ci.yml");
    assert!(
        area.contains("brew install tmux") && area.contains("apt-get install -y tmux"),
        "the L2 job must provision tmux on both macOS and Linux"
    );
    assert!(
        area.contains("environment: ${{ fromJSON(inputs.l2-environments) }}"),
        "the L2 job must be a matrix over the environments with a provisioned backend"
    );
    // A backend that cannot exist on this runner skips inside require_level!.
    // That skip must be readable, not silent.
    assert!(
        area.contains("Report L2 backend coverage") && area.contains("CI=1"),
        "the L2 job must report which declared backends were reachable and which skipped"
    );

    // Windows L2 is a POLICY GAP, never a green `0 run / N skipped` cell. Every
    // area that owns L2 tests and runs on Windows must say so in areas.json;
    // `affected_scope.py` fails the scope calculation when one does not.
    let areas = read(".github/ci/areas.json");
    assert!(
        areas.contains(r#""tier": "L2""#) && areas.contains("POLICY GAP")
            || read("scripts/ci/affected_scope.py").contains("POLICY GAP"),
        "an unprovisionable L2 tier must be recorded as an owned policy gap"
    );
}

#[test]
fn exclusions_are_owned_and_time_bounded() {
    // Ten `"ci": false` records accumulated because an exclusion cost nothing to
    // leave in place. Every one now names an owner and a date, and a lapsed date
    // fails the scope calculation loudly.
    let policy = read("scripts/ci/affected_scope.py");
    assert!(
        policy.contains("def validate_expiry") && policy.contains("expired on"),
        "affected_scope.py must reject a lapsed exclusion or policy gap"
    );
    let areas = read(".github/ci/areas.json");
    let excluded = areas.matches(r#""ci": false"#).count();
    assert_eq!(excluded, 10, "the ten non-gating areas must each keep one record");
    assert_eq!(
        areas.matches(r#""exclusion_class""#).count(),
        excluded,
        "every exclusion must declare its class"
    );
    assert!(
        areas.matches(r#""owner""#).count() >= excluded,
        "every exclusion must name an accountable owner"
    );
}

// --- the single required check: ci-verdict ------------------------------------

/// Every producer job, paired with the `job:` value its status artifact carries.
///
/// `ci-rollup` parses that value as a TIER: a status naming a test tier explains
/// a `MISSING` cell downstream of it, while a status naming anything else becomes
/// a cell in its own right. Publishing `test` would therefore manufacture a
/// phantom `<area>/<environment>/test` cell beside the real L1 one and count the
/// same failure twice.
const PRODUCER_STATUS: [(&str, &str, &str); 6] = [
    ("_area-ci.yml", "  check:", "JOB: check"),
    ("_area-ci.yml", "  test:", "JOB: L1"),
    ("_area-ci.yml", "  lint:", "JOB: lint"),
    ("_area-ci.yml", "  l2:", "JOB: L2"),
    ("_area-ci.yml", "  browser:", "JOB: browser"),
    ("_wsl-ci.yml", "  wsl:", "JOB: L1"),
];

fn job_block(file: &str, header: &str) -> String {
    let source = workflow(file);
    jobs(&source)
        .into_iter()
        .find(|job| job.starts_with(header))
        .unwrap_or_else(|| panic!("{file} must define the `{}` job", header.trim()))
}

#[test]
fn every_producer_job_emits_an_explicit_status_artifact() {
    // Plan 1.3: a failed or cancelled producer must be able neither to prevent
    // the verdict from running nor to be silently read as a pass. Measured in run
    // 30323254931: when `needs:` skips a matrix job GitHub never evaluates the
    // matrix context, so the whole matrix collapses into ONE skipped job named
    // with the raw `${{ matrix.os }}` expression — no artifact, no environment,
    // no shard, nothing to key policy to. An explicit status artifact is the only
    // durable evidence that job existed.
    for (file, header, job_value) in PRODUCER_STATUS {
        let name = header.trim();
        let block = job_block(file, header);
        assert!(
            block.contains("name: Record producer status"),
            "{file}: `{name}` must record its own conclusion"
        );
        assert!(
            block.contains("RESULT: ${{ job.status }}"),
            "{file}: `{name}` must publish GitHub's own job status, not a derived one"
        );
        assert!(
            block.contains(job_value),
            "{file}: `{name}` must publish `{job_value}` — ci-rollup reads `job` as a tier"
        );
        assert!(
            block.contains("name: status-${{ inputs.area }}-"),
            "{file}: `{name}` must upload under the `status-` prefix the walker matches on"
        );

        // `always()`, not `!cancelled()` and not the default: the whole point is
        // that a FAILED job still reports itself.
        let tail = &block[block
            .find("name: Record producer status")
            .expect("checked above")..];
        assert_eq!(
            tail.matches("if: ${{ always() }}").count(),
            2,
            "{file}: `{name}` must guard both the status write and its upload with always()"
        );
    }
}

#[test]
fn junit_uploads_carry_the_whole_staging_directory_and_its_manifest() {
    // `.config/nextest.toml` writes every ci-profile invocation to the SAME
    // `target/nextest/ci/test-results.xml`, so uploading that path published only
    // the LAST package's report for a multi-package area. The staging tree holds
    // one XML per invocation plus `manifest.jsonl`, and the manifest is what
    // carries {area, environment, tier, shard, package} identity — without it
    // every record degrades to artifact-name parsing with an unknown shard.
    for file in ["_area-ci.yml", "_wsl-ci.yml"] {
        // Comments explain the retired path; only executable YAML can use it.
        let executable: String = workflow(file)
            .lines()
            .filter(|line| !line.trim_start().starts_with('#'))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            !executable.contains("target/nextest/ci/test-results.xml"),
            "{file}: a JUnit upload must not point at the single overwritten report"
        );
    }
    let area = workflow("_area-ci.yml");
    assert_eq!(
        area.matches("path: target/nextest/ci-reports").count(),
        3,
        "the L1, L2, and browser uploads must each publish the whole staging directory"
    );

    // The guest stages onto ext4, which the Windows host cannot read, so the
    // reports cross the 9p mount once. Copying the staging root's CONTENTS is
    // what puts manifest.jsonl at the artifact root, which is the only place the
    // walker reads it from.
    let wsl = workflow("_wsl-ci.yml");
    assert!(
        wsl.contains(
            "BISCUIT_JUNIT_STAGE_DIR: /home/runner/rusty-biscuit/target/nextest/ci-reports"
        ),
        "the WSL job must pin the guest staging root; two wsl-bash invocations must agree on it"
    );
    assert!(
        wsl.contains(r#"cp -R '${{ env.BISCUIT_JUNIT_STAGE_DIR }}/.' "$workspace/wsl-junit/""#),
        "the WSL job must copy the staging root's contents so manifest.jsonl lands at the artifact root"
    );
}

#[test]
fn ci_verdict_is_the_single_required_check() {
    let ci = workflow("ci.yml");
    let verdict = job_block("ci.yml", "  ci-verdict:");

    // `always()` is the only condition that survives a failed, cancelled, AND
    // skipped `needs`. A required check a failing producer can skip is not one.
    assert!(
        verdict.contains("if: always()"),
        "ci-verdict must run even when every producer failed"
    );
    for producer in [
        "scope",
        "preflight",
        "canary",
        "area-ci",
        "affected-coverage",
        "claudine-generator-signals",
        "darkmatter-no-color",
        "rendezvous",
        "biscuit-tui-captured-stdout",
        "playa-windows",
        "messenger-desktop",
    ] {
        assert!(
            verdict.contains(&format!("      - {producer}\n")),
            "ci-verdict must wait on `{producer}` so its artifacts are downloadable"
        );
    }

    // `--scope` is the ONLY way the rollup learns an area was scheduled and
    // produced nothing. Inferred scope reads the artifacts on disk, which by
    // construction cannot see an area that produced no artifact at all — exactly
    // the case MISSING exists to catch. It is also what makes an out-of-scope
    // baseline entry ignored rather than counted as a pass.
    assert!(
        verdict.contains(r#"--scope "$SCOPE""#)
            && verdict.contains("needs.scope.outputs.area_names"),
        "ci-verdict must pass the affected scope to the rollup"
    );
    assert!(
        verdict.contains("needs.scope.result") && verdict.contains("exit 1"),
        "a broken scope job must fail the verdict, not render a vacuous CLEAR over an empty grid"
    );

    // repo-deps and drift link biscuit-terminal, sniff, and cargo_metadata;
    // ci-rollup links none of them. The always-runs required check must not pay
    // for the other two bins' dependency graph.
    assert!(
        verdict.contains("--no-default-features --bin ci-rollup"),
        "ci-verdict must build only the ci-rollup bin"
    );
    assert!(
        verdict.contains("ci-rollup rollup") && verdict.contains("ci-rollup verdict"),
        "ci-verdict must roll the artifacts up and then judge them against the baseline"
    );
    assert!(
        verdict.contains(".github/ci/ci-baseline.toml"),
        "the verdict must be taken against the machine-readable baseline"
    );
    assert!(
        !ci.contains("baseline-failures.txt"),
        "the retired display-name baseline must have no consumers"
    );

    // Downloading with no `name:` yields one directory per artifact, which is
    // the layout `--artifacts` walks.
    assert!(
        verdict.contains("uses: actions/download-artifact@v4") && !verdict.contains("name: junit-"),
        "ci-verdict must download every artifact in the run, not a named subset"
    );
}

#[test]
fn only_ci_verdict_makes_a_run_level_claim() {
    // Two "what failed" reporters that can disagree is worse than one. The area
    // `classify` job printed a per-area failure class from job RESULTS while the
    // rollup judges by EVIDENCE, so a baselined known-red gate would print a
    // failure line under a correctly CLEAR verdict. Its information now lives in
    // the status artifacts, keyed by {area, environment, tier} — a resolution
    // classify never had, since one area-level line cannot tell a Windows-only
    // failure from a Linux-only one.
    let area = workflow("_area-ci.yml");
    assert!(
        !area.contains("  classify:") && !area.contains("first actionable failure class"),
        "_area-ci.yml must not carry a second, environment-blind failure reporter"
    );

    let summary = job_block("ci.yml", "  summary:");
    assert!(
        !summary.contains("No gate reported a failure"),
        "the advisory summary must make no run-level green claim: reading job results alone \
         cannot see a MISSING cell, a cancelled producer, or a baselined entry that started passing"
    );
    assert!(
        !summary.contains("needs.area-ci.result"),
        "every area gate is a rollup cell; the advisory summary must not report them a second time"
    );
    assert!(
        summary.contains("ci-verdict"),
        "the advisory summary must name ci-verdict as the run's verdict"
    );
}

// --- shared recipes must stay usable from a runner ---------------------------

#[test]
fn just_commit_is_deterministic_under_ci() {
    // The local flow writes its message with an LLM, speaks a completion sound,
    // and needs credentials — none of which exist on a runner. Under CI the
    // recipe must be a plain `git commit` with a caller-supplied message.
    let justfile = read("justfile");
    // Recipe bodies are indented, so the recipe ends at the next line with
    // column-zero content. A blank line does NOT end it — the body has several.
    let recipe: String = justfile
        .lines()
        .skip_while(|line| !line.starts_with("commit *args="))
        .take_while(|line| {
            line.starts_with("commit *args=") || line.is_empty() || line.starts_with(' ')
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        recipe.starts_with("commit *args="),
        "root justfile must define a `commit` recipe"
    );

    let ci_branch = recipe
        .find(r#"if [[ -n "${CI:-}" ]]; then"#)
        .expect("`just commit` must branch on CI");
    let ci_exit = recipe[ci_branch..]
        .find("exit 0")
        .expect("the CI branch must return before the interactive flow");
    let ci_path = &recipe[ci_branch..ci_branch + ci_exit];

    assert!(
        ci_path.contains("git commit -m"),
        "the CI branch must perform a plain git commit"
    );
    for interactive in ["claudine", "_speak", "COMMIT_MODEL"] {
        assert!(
            !ci_path.contains(interactive),
            "the CI branch must not reach `{interactive}` — no LLM, audio, or network on a runner"
        );
    }
    // `quote()` shell-escapes the message, so a commit subject containing quotes
    // or apostrophes cannot break out of the generated script.
    assert!(
        recipe.contains("{{ quote(args) }}"),
        "the CI message must be shell-quoted rather than interpolated raw"
    );
}


// --- a PR that schedules no run at all ----------------------------------------

/// A conflicted PR must be reported, not merely fail to schedule anything.
///
/// `pull_request` workflows run against `refs/pull/N/merge`. GitHub cannot
/// create that ref while the PR conflicts, so it creates no workflow run --
/// silently, with no check suite and no annotation. `ci-verdict` therefore never
/// reports, and the PR is indistinguishable from one whose CI has not started.
/// Measured on PR #19, where the entire matrix was suppressed undetected.
#[test]
fn a_conflicted_pr_is_reported_rather_than_silently_unscheduled() {
    let health = read(".github/workflows/pr-health.yml");

    // `pull_request` is exactly the trigger that cannot fire here, so relying on
    // it would reintroduce the blind spot this guard exists to close.
    assert!(
        health.contains("pull_request_target:"),
        "the guard must use `pull_request_target`, which runs against the base \
         branch and needs no merge commit"
    );
    assert!(
        health.contains("branches-ignore: [main]"),
        "a push to a branch with an existing PR must also be covered, for the \
         case where a later push introduces the conflict"
    );

    // `pull_request_target` runs with a privileged token against the base ref.
    // Checking out head-branch code under it is the well-known escalation path.
    // Comments are stripped first: the workflow's own security note names the
    // action it forbids, and matching that would assert on prose, not on steps.
    let steps: String = health
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !steps.contains("actions/checkout"),
        "pr-health must never check out pull-request code under \
         `pull_request_target`"
    );

    assert!(
        health.contains("exit 1"),
        "a conflicted PR must fail the check, not emit a warning -- a warning \
         is as invisible as the missing run it is reporting"
    );
}
