//! Durable CI/CD workflow contract tests.
//!
//! These guard the invariants of the package-keyed CI grid
//! (`fixes/2026-08-06-cicd`): the package is the unit of selection, execution,
//! and result identity; the repository tracks no rustc wrapper while the
//! pinned compiler cache stays a host opt-in with a single authority; the
//! primary workflow runs a bootstrap preflight that gates the package fan-out;
//! declared test tiers are non-vacuous; and release automation follows
//! successful CI instead of racing it. They inspect workflow/action and
//! manifest source so a regression fails locally without a live GitHub Actions
//! run.

use std::{fs, path::PathBuf, process::Command};

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
    // Working trees on Windows check out CRLF (core.autocrlf=true) while the
    // index holds LF; contract scanning must not depend on the host's checkout
    // convention.
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
        .replace("\r\n", "\n")
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

fn job_block(file: &str, header: &str) -> String {
    let source = workflow(file);
    jobs(&source)
        .into_iter()
        .find(|job| job.starts_with(header))
        .unwrap_or_else(|| panic!("{file} must define the `{}` job", header.trim()))
}

/// Every workspace package's resolved `[package.metadata.ci]` policy plus its
/// manifest path, which source-based contracts use to find the owning crate.
///
/// EVERY member is enumerated: a package with no metadata block yields an
/// empty record (the default policy), because the default-policy packages are
/// precisely what the undeclared-tier guards exist for. Dropping them here is
/// how `claudine-gen` owned real L2 tests while every contract stayed green.
fn package_policies() -> Vec<serde_json::Value> {
    let output = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(repo_root())
        .output()
        .expect("cargo metadata runs");
    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("metadata is JSON");
    let members: Vec<&str> = metadata["workspace_members"]
        .as_array()
        .expect("workspace_members is an array")
        .iter()
        .map(|value| value.as_str().expect("member id is a string"))
        .collect();
    metadata["packages"]
        .as_array()
        .expect("packages is an array")
        .iter()
        .filter(|package| members.contains(&package["id"].as_str().unwrap_or("")))
        .map(|package| {
            let mut record = serde_json::Map::new();
            record.insert("name".to_owned(), package["name"].clone());
            record.insert("manifest-path".to_owned(), package["manifest_path"].clone());
            if let Some(ci) = package["metadata"]["ci"].as_object() {
                for (key, value) in ci {
                    record.insert(key.clone(), value.clone());
                }
            }
            serde_json::Value::Object(record)
        })
        .collect()
}

fn gating_packages() -> Vec<serde_json::Value> {
    package_policies()
        .into_iter()
        .filter(|policy| policy["gates"].as_bool().unwrap_or(true))
        .collect()
}

// --- D1/D2: no tracked rustc wrapper; kache is a host opt-in -----------------

#[test]
fn repository_tracks_no_rustc_wrapper() {
    let config = repo_root().join(".cargo/config.toml");
    if config.exists() {
        let source = fs::read_to_string(&config).expect("read .cargo/config.toml");
        assert!(
            !source.contains("rustc-wrapper"),
            ".cargo/config.toml must not set rustc-wrapper — that is host policy, \
             not repository policy (docs/kache-strategy.md)"
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

    let justfile = read("justfile");
    assert!(
        justfile.contains(".github/kache-version"),
        "root justfile must read KACHE_VERSION from the single authority file"
    );
    assert!(
        !justfile.contains(r#"KACHE_VERSION := """#),
        "root justfile must not hard-code a second kache version literal"
    );
}

#[test]
fn installing_the_compiler_cache_is_not_a_dependency_of_init() {
    let justfile = read("justfile");
    let init_line = justfile
        .lines()
        .find(|line| line.starts_with("init:"))
        .expect("root justfile must declare an `init` recipe");
    assert!(
        !init_line.contains("kache"),
        "`just init` must not depend on a kache recipe; it is an explicit opt-in \
         (`just install-kache`)"
    );
    assert!(
        justfile.contains("install-kache:"),
        "the pinned installer must remain available as an explicit recipe"
    );
}

#[test]
fn ci_does_not_wire_the_kache_wrapper() {
    for stale in [".github/actions/enable-kache", ".github/actions/report-kache"] {
        assert!(
            !repo_root().join(stale).exists(),
            "{stale} must not exist — CI no longer uses the kache wrapper"
        );
    }

    let workflows = repo_root().join(".github/workflows");
    for entry in fs::read_dir(&workflows).expect("read .github/workflows") {
        let path = entry.expect("workflow entry").path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("yml") {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let source = read(&format!(".github/workflows/{name}"));
        for marker in ["enable-kache", "report-kache", "kunobi-ninja/kache-action"] {
            assert!(
                !source.contains(marker),
                "{name} references `{marker}`; CI must not wire the kache wrapper"
            );
        }
    }
}

#[test]
fn every_cargo_workflow_neutralizes_a_stray_rustc_wrapper() {
    let workflows = repo_root().join(".github/workflows");
    for entry in fs::read_dir(&workflows).expect("read .github/workflows") {
        let path = entry.expect("workflow entry").path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("yml") {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let source = read(&format!(".github/workflows/{name}"));
        let touches_cargo = source.contains("cargo ") || source.contains("just ");
        if !touches_cargo {
            continue;
        }
        assert!(
            source.contains("RUSTC_WRAPPER"),
            "{name} can invoke Cargo but does not clear RUSTC_WRAPPER"
        );
    }
}

// --- D3: bootstrap preflight gates package fan-out ---------------------------

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
        "the package fan-out must depend on a successful preflight"
    );
}

#[test]
fn expensive_tiers_stage_behind_l1_but_lint_never_gates_it() {
    let shared = workflow("_package-ci.yml");
    // The expensive L2 and browser tiers are ORDERED after L1 (D4) — but only
    // ordered. Requiring L1 to have passed meant one red L1 deleted the whole
    // tier's evidence and the rollup recorded MISSING.
    assert!(
        shared.matches("needs: test").count() >= 2,
        "the L2 and browser tiers must each be ordered after L1 (D4)"
    );
    assert!(
        shared.matches("!cancelled() && inputs.").count() >= 2,
        "the L2 and browser tiers must run on `!cancelled()`, not on L1 success — \
         absence of evidence must never be a side effect of a failure elsewhere"
    );
    // L1 does NOT stage behind lint. A clippy hint in one package must not
    // delete another package's L1 leg's evidence.
    assert!(
        !shared.contains("needs: lint"),
        "the L1 test job must not depend on lint — a lint failure must not suppress test evidence"
    );
}

#[test]
fn a_failing_leg_can_never_be_dropped_from_the_run_verdict() {
    let shared = workflow("_package-ci.yml");
    assert!(
        !shared.contains("continue-on-error"),
        "no gate may be continue-on-error — that erases the leg from the run's verdict"
    );
}

#[test]
fn package_ci_selects_the_ci_nextest_profile_explicitly() {
    let shared = workflow("_package-ci.yml");
    assert!(
        shared.contains("NEXTEST_PROFILE: ci"),
        "package CI must explicitly select the `ci` nextest profile, not rely on detection (D6)"
    );
}

/// AC10: the matrix is package-derived, never static or directory-derived.
#[test]
fn the_package_matrix_is_scope_derived_not_static() {
    let ci = workflow("ci.yml");
    assert!(
        ci.contains("fromJSON(needs.scope.outputs.package_matrix)"),
        "ci.yml must fan out from the scope-derived package matrix"
    );
    assert!(
        ci.contains("package: ${{ matrix.package }}"),
        "the fan-out must name the package from the matrix entry"
    );
    // Every matrix field the reusable workflow consumes is produced by the
    // scope job, so a static or hand-written matrix cannot satisfy it.
    for field in [
        "check_args",
        "test_args",
        "l1_include_slow",
        "native_environments",
        "l2_environments",
        "browser_environments",
        "node_environments",
        "l2_backends",
        "runner_tools",
        "companion_suites",
        "native",
    ] {
        assert!(
            ci.contains(&format!("matrix.{field}")),
            "the fan-out must forward the scope-derived matrix.{field}"
        );
    }
}

/// The fan-out has no stage in front of it, and nothing named `canary` remains.
#[test]
fn the_fan_out_has_no_stage_in_front_of_it() {
    let ci = workflow("ci.yml");
    assert!(
        !ci.contains("\n  canary:"),
        "ci.yml must not define a canary stage"
    );
    assert!(
        !ci.contains("canary_matrix") && !ci.contains("has_canaries"),
        "the scope job must not emit a canary matrix or selection flag"
    );
    assert!(
        ci.contains("needs: [scope, preflight]"),
        "the package fan-out must depend on scope and preflight only"
    );
}

/// R9: CI runs the canonical per-package recipes. L1, L2, browser, and lint
/// invoke `_test`, `_test_l2`, `_test_browser`, and `_lint`.
#[test]
fn the_reusable_workflow_invokes_the_canonical_recipes() {
    let shared = workflow("_package-ci.yml");
    assert!(shared.contains("just _test \"${{ inputs.package }}\""), "L1 invokes _test");
    assert!(
        shared.contains("just _test_l2 \"${{ inputs.package }}\""),
        "L2 invokes _test_l2"
    );
    assert!(
        shared.contains("just _test_browser \"${{ inputs.package }}\""),
        "browser invokes _test_browser"
    );
    assert!(shared.contains("just _lint \"${{ inputs.package }}\""), "lint invokes _lint");
    // Compile-check stays `cargo check --all-targets -p <package>` (R9): there
    // is no per-package canonical check recipe.
    assert!(
        shared.contains("cargo check --all-targets ${{ inputs.check-args }}"),
        "the check job runs cargo check over the passed package selector"
    );
    let ci = workflow("ci.yml");
    assert!(
        ci.contains("check-args: ${{ matrix.check_args }}"),
        "the fan-out must pass the scope-derived check_args"
    );
}

#[test]
fn scope_job_emits_an_actionable_summary() {
    let ci = workflow("ci.yml");
    assert!(
        ci.contains("$GITHUB_STEP_SUMMARY") && ci.contains("## CI scope"),
        "the scope job must write an actionable scope summary to the step summary (D15)"
    );
    assert!(
        ci.contains("expanded job estimate"),
        "the scope summary must record the expanded job count (Phase 3)"
    );
    assert!(
        ci.contains("ci-scope"),
        "the scope job must upload its resolved policy for the rollup"
    );
}

#[test]
fn package_ci_treats_rust_warnings_as_failures_and_runs_lint() {
    let shared = workflow("_package-ci.yml");
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
        shared.contains(r#"run: just _lint "${{ inputs.package }}""#),
        "package CI must lint through the canonical _lint recipe"
    );
    let devops = read("just/devops.just");
    assert!(
        devops.contains("cargo clippy") && devops.contains("-- -D warnings"),
        "`_lint` must pass -D warnings to clippy directly so lint denies warnings off CI too"
    );

    let check = job_block("_package-ci.yml", "  check:");
    assert!(
        !check.contains("RUSTFLAGS"),
        "the compile check must not promote warnings to errors — a warning is not \
         a build failure, and cross-platform dead code is normal"
    );
}

// --- package policy source of truth (manifests + environments.json) ---------

#[test]
fn package_metadata_carries_the_native_and_tier_policy() {
    // Playa's Linux ALSA/PulseAudio headers are a property of the package,
    // declared in its own manifest and installed before build/test (D9/R5).
    let playa = fs::read_to_string(repo_root().join("playa/lib/Cargo.toml")).unwrap();
    assert!(
        playa.contains("libasound2-dev") && playa.contains("libpulse-dev"),
        "playa must declare its Linux native audio prerequisites in its manifest"
    );

    // Tier ownership is package-scoped: sniff-cli owns the area's L2 tier.
    let sniff_cli = fs::read_to_string(repo_root().join("sniff/cli/Cargo.toml")).unwrap();
    assert!(
        sniff_cli.contains("\"L2\"") && sniff_cli.contains("tmux"),
        "sniff-cli must declare the L2 tier and its tmux backend"
    );

    // Feature contracts are declared per package and forwarded consistently
    // (R-feature-set-drift): biscuit-hash runs with every feature.
    let biscuit_hash = fs::read_to_string(repo_root().join("biscuit-hash/lib/Cargo.toml")).unwrap();
    assert!(
        biscuit_hash.contains("all-features = true"),
        "biscuit-hash must declare its all-features L1 contract"
    );
}

#[test]
fn the_environment_capability_table_covers_every_environment() {
    let environments = read(".github/ci/environments.json");
    for name in ["ubuntu-latest", "windows-latest", "macos-latest", "wsl2-ubuntu"] {
        assert!(
            environments.contains(&format!("\"name\": \"{name}\"")),
            "environments.json must declare the {name} environment"
        );
    }
    // The two governed unavailabilities the eight per-area policy_gaps records
    // used to restate, now declared once.
    assert!(
        environments.contains("\"tmux\": {") && environments.contains("\"available\": false"),
        "an unhostable L2 capability must carry governed unavailability metadata"
    );
    assert!(
        environments.contains("\"archive_only\": true"),
        "the wsl2-ubuntu environment must be recorded as archive-only"
    );
}

#[test]
fn l2_provisions_and_verifies_only_the_ci_capable_backend() {
    let shared = workflow("_package-ci.yml");
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
    // Execution proof, not mere availability: the leg requires exactly the
    // declared backends it provisioned (today only tmux is CI-hostable), so an
    // installed-but-never-exercised backend fails `_test_l2`'s backend-proof
    // bracket instead of rendering a green cell with zero executed L2 tests.
    assert!(
        shared.contains("BISCUIT_TEST_REQUIRED_BACKENDS"),
        "the L2 job must require the provisioned backends through BISCUIT_TEST_REQUIRED_BACKENDS"
    );
    // Focus safety: CI must never run L3 or take foreground focus.
    assert!(
        !shared.contains("BISCUIT_L3_TAKE_FOCUS") && !shared.contains("test-l3"),
        "CI must not run L3 or enable focus-taking"
    );
}

#[test]
fn packages_declaring_native_deps_are_provisioned_from_the_closure() {
    // One installer, shared by developer hosts and CI: the root justfile recipe
    // reads package manifests via `cargo metadata`. A second CI-only
    // implementation would drift from `just init`, so the retired composite
    // must not come back.
    assert!(
        !repo_root().join(".github/actions/install-native").exists(),
        "the install-native composite must stay retired; `just _ensure-native-libs` is the one installer"
    );
    let justfile = read("justfile");
    assert!(
        justfile.contains("_ensure-native-libs"),
        "the native-libs recipe must exist"
    );
    assert!(
        justfile.contains("cargo metadata") && justfile.contains("metadata.ci.native"),
        "the recipe must read native declarations from package manifests"
    );
    assert!(
        justfile.contains("init: ") && justfile.contains("_ensure-native-libs"),
        "`just init` must depend on the same recipe CI runs"
    );

    // The reusable workflow passes the scope-computed closure union and the
    // recipe never runs its whole-workspace form inside a per-package job.
    let shared = workflow("_package-ci.yml");
    assert!(
        shared.contains("inputs.native") && shared.contains("just _ensure-native-libs"),
        "the reusable workflow must provision the scope-derived native closure"
    );
}

#[test]
fn native_prerequisites_are_installed_before_anything_is_built() {
    const BUILD_COMMANDS: [&str; 6] = [
        "cargo check --all-targets",
        "cargo llvm-cov",
        "just _test ",
        "just _lint",
        "just _test_l2",
        "just _test_browser",
    ];
    const PROVISION: &str = "just _ensure-native-libs";

    let mut provisioning_jobs = 0;
    for name in ["_package-ci.yml", "ci.yml"] {
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
    // check, test, lint, l2, browser in the reusable package workflow.
    // (Coverage left CI entirely — decided 2026-08-12; `just coverage` is the
    // local tool.)
    assert_eq!(
        provisioning_jobs, 5,
        "every building CI job must provision native prerequisites"
    );
}

#[test]
fn the_l1_suite_runs_no_fail_fast() {
    let shared = workflow("_package-ci.yml");
    assert!(
        shared.contains("--no-fail-fast"),
        "L1 suites must run --no-fail-fast so one failure cannot hide the suite's evidence (D7)"
    );
    assert!(
        shared.contains("actions/upload-artifact") && shared.contains("junit-"),
        "each test tier must publish per-package JUnit artifacts (D7)"
    );
}

// --- D12: specialized runtime contracts are reusable and orchestrated ---------

/// Each surviving specialized runtime workflow the primary orchestrator calls,
/// paired with its unique runtime evidence and scope selector.
const ORCHESTRATED: [(&str, &str, &str); 1] = [(
    "biscuit-tui-windows-captured-stdout.yml",
    "captured_stdout_receives_only_value_no_tui_bytes",
    "needs.scope.outputs.biscuit_tui == 'true'",
)];

#[test]
fn retired_specialized_workflows_and_jobs_are_absent() {
    let mut live = Vec::new();
    for name in [
        "messenger-desktop-tests.yml",
        "rendezvous-tests.yml",
        "playa-windows.yml",
    ] {
        if repo_root().join(".github/workflows").join(name).exists() {
            live.push(format!("workflow file {name}"));
        }
    }

    let ci = workflow("ci.yml");
    for header in [
        "  messenger-desktop:",
        "  rendezvous:",
        "  playa-windows:",
        "  claudine-generator-signals:",
        "  darkmatter-no-color:",
    ] {
        if jobs(&ci).iter().any(|job| job.starts_with(header)) {
            live.push(format!("ci.yml job {}", header.trim_end_matches(':').trim()));
        }
    }

    assert!(
        live.is_empty(),
        "retired specialized graph entries remain: {}",
        live.join(", ")
    );
}

#[test]
fn specialized_inventory_contains_only_surviving_workflows() {
    let names: Vec<&str> = ORCHESTRATED.iter().map(|(name, _, _)| *name).collect();
    assert_eq!(
        names,
        ["biscuit-tui-windows-captured-stdout.yml"],
        "the specialized inventory must contain only workflows that remain specialized"
    );
}

#[test]
fn active_ci_authority_matches_the_retirement_contract() {
    let active_docs = [
        ".github/ci/README.md",
        "docs/topics/ci-cd.md",
        "docs/testing-strategy.md",
        "claudine/docs/rendezvous/local-ipc.md",
        "claudine/features/2026-07-12-rendezvous-dashboard/windows-support-followup.md",
        ".claude/skills/claudine/architecture.md",
    ];
    let corpus = active_docs
        .iter()
        .map(|path| read(path))
        .collect::<Vec<_>>()
        .join("\n");

    for retired in [
        "messenger-desktop-tests.yml",
        "rendezvous-tests.yml",
        "playa-windows.yml",
    ] {
        assert!(
            !corpus.contains(retired),
            "active CI authority must not assign coverage to retired {retired}"
        );
    }
    assert!(
        corpus.contains("`messenger-desktop-stubs`")
            && corpus.contains("MESSENGER_STUB_BIN_DIR")
            && corpus.contains("toolchain-free")
            && corpus.contains("package-keyed")
            && corpus.contains("wsl2-ubuntu"),
        "active CI authority must document the closed runner tool, explicit fixture path, toolchain-free WSL2 sidecar, and package-keyed result identity"
    );

    let ci_topic = read("docs/topics/ci-cd.md");
    let survivor = "biscuit-tui-windows-captured-stdout.yml";
    assert!(
        ci_topic.contains(survivor),
        "the active specialized inventory must retain {survivor}"
    );
    assert!(
        !ci_topic.contains("claudine-windows-ctrl-c.yml"),
        "the active specialized inventory must contain only executable survivors"
    );
}

#[test]
fn messenger_stub_runner_tool_reaches_native_and_wsl2_execution() {
    let wsl = workflow("_wsl-ci.yml");
    let native_test = job_block("_package-ci.yml", "  test:");
    let wsl_archive = job_block("_wsl-ci.yml", "  archive:");
    let wsl_test = job_block("_wsl-ci.yml", "  wsl:");
    let wsl_delegation = job_block("_package-ci.yml", "  wsl:");
    let mut missing = Vec::new();

    if native_test.matches("name: Build messenger desktop stubs").count() != 1
        || !native_test.contains("build_args=(--all-features -p messenger)")
        || !native_test.contains("cargo build \"${build_args[@]}\"")
        || !native_test.contains("stub_dunstify stub_notify_send stub_snoretoast stub_burnttoast stub_terminal_notifier stub_alerter")
        || !native_test.contains("MESSENGER_STUB_BIN_DIR")
        || !native_test.contains("GITHUB_ENV")
        || !native_test.contains("pwd -W")
    {
        missing.push("one native six-binary prebuild exported through GITHUB_ENV");
    }
    let native_l1 = native_test
        .split("- name: L1 tests")
        .nth(1)
        .and_then(|tail| tail.split("- name: Companion suite").next())
        .unwrap_or_default();
    if native_l1.contains("cargo build") {
        missing.push("native L1 step free of nested fixture builds");
    }
    if !wsl.contains("runner-tools:")
        || !wsl_archive.contains(
            "build_args=(--all-features -p messenger --target x86_64-unknown-linux-gnu)",
        )
        || !wsl_archive.contains("cargo build \"${build_args[@]}\"")
        || !wsl_archive.contains("messenger-desktop-stubs-${{ inputs.package }}-wsl2-ubuntu")
        || !wsl_test.contains("Download the messenger desktop stub sidecar")
        || !wsl_test.contains("chmod 0755")
        || !wsl_test.contains("chown -R biscuit:biscuit")
        || !wsl_test.contains("MESSENGER_STUB_BIN_DIR")
    {
        missing.push("WSL2 six-binary sidecar delivery to the unprivileged guest");
    }
    if !wsl_delegation.contains("runner-tools: ${{ inputs.runner-tools }}") {
        missing.push("_package-ci.yml runner-tool forwarding to _wsl-ci.yml");
    }
    if !wsl_test.contains("command -v cargo") || !wsl_test.contains("command -v rustc") {
        missing.push("guest Cargo and rustc absence proof before L1");
    }

    assert!(
        missing.is_empty(),
        "messenger runner tooling is not end-to-end: {}",
        missing.join(", ")
    );
}

#[test]
fn claudine_provider_fixture_runner_tool_reaches_native_l1_execution() {
    let native_test = job_block("_package-ci.yml", "  test:");
    let manifest = read("claudine/cli/Cargo.toml");
    let docs = read(".github/ci/README.md");
    let mut missing = Vec::new();

    if !manifest.contains("claudine-provider-fixture") {
        missing.push("claudine-cli runner-tool declaration");
    }
    if native_test
        .matches("name: Build the Claudine provider fixture")
        .count()
        != 1
        || !native_test.contains(
            "cargo build -q -p claudine-cli --example ctx_launch_anchor_provider_fixture",
        )
        || !native_test.contains("CLAUDINE_PROVIDER_FIXTURE_EXE")
        || !native_test.contains("GITHUB_ENV")
        || !native_test.contains("pwd -W")
    {
        missing.push("one native example prebuild exported through GITHUB_ENV");
    }
    let native_l1 = native_test
        .split("- name: L1 tests")
        .nth(1)
        .and_then(|tail| tail.split("- name: Companion suite").next())
        .unwrap_or_default();
    if native_l1.contains("cargo build") || native_l1.contains("rustc") {
        missing.push("native L1 step free of nested fixture compilation");
    }
    if !docs.contains("`claudine-provider-fixture`")
        || !docs.contains("CLAUDINE_PROVIDER_FIXTURE_EXE")
    {
        missing.push("closed-vocabulary documentation and exported path");
    }

    assert!(
        missing.is_empty(),
        "Claudine provider runner tooling is not end-to-end: {}",
        missing.join(", ")
    );
}

#[test]
fn retired_packages_reach_verdict_through_package_evidence() {
    let mut missing = Vec::new();
    for manifest in [
        "messenger/lib/Cargo.toml",
        "messenger/cli/Cargo.toml",
        "claudine/rendezvous/core/Cargo.toml",
        "claudine/rendezvous/client/Cargo.toml",
        "claudine/rendezvous/daemon/Cargo.toml",
    ] {
        if read(manifest).contains("gates = false") {
            missing.push(format!("{manifest} is not gating"));
        }
    }

    let package_ci = workflow("_package-ci.yml");
    let wsl = workflow("_wsl-ci.yml");
    if !package_ci.contains("junit-${{ inputs.package }}-L1-${{ matrix.environment }}")
        || !package_ci.contains("status-${{ inputs.package }}-L1-${{ matrix.environment }}")
        || !wsl.contains("junit-${{ inputs.package }}-L1-wsl2-ubuntu")
        || !wsl.contains("status-${{ inputs.package }}-L1-wsl2-ubuntu")
    {
        missing.push("package-keyed native/WSL2 L1 evidence".to_owned());
    }

    let verdict = job_block("ci.yml", "  ci-verdict:");
    for retired_job in ["rendezvous", "messenger-desktop"] {
        if verdict.contains(&format!("      - {retired_job}\n")) {
            missing.push(format!("ci-verdict still depends on {retired_job}"));
        }
    }
    if !verdict.contains("uses: actions/download-artifact@v4")
        || !verdict.contains("ci-rollup rollup")
        || !verdict.contains("ci-rollup verdict")
    {
        missing.push("artifact-driven ci-verdict consumption".to_owned());
    }

    assert!(
        missing.is_empty(),
        "retired package failures are not exclusively package-keyed evidence: {}",
        missing.join(", ")
    );
}

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
            "ci.yml must select {name} from the scope-derived flags"
        );
    }
}

#[test]
fn release_artifact_builds_stay_out_of_per_commit_validation() {
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
    let ci = workflow("ci.yml");
    assert!(
        ci.contains("## Jobs outside the rollup") && ci.contains("First actionable failure class"),
        "ci.yml must write a failure-class summary for the un-rolled-up jobs (D15)"
    );
    for stage in [
        "bootstrap (scope calculation)",
        "bootstrap (preflight)",
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
    for name in ["ci.yml", "_package-ci.yml"] {
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
fn lockfiles_are_tracked_and_the_release_premise_says_so() {
    let gitignore = read(".gitignore");
    assert!(
        !gitignore.contains("\n**/Cargo.lock"),
        ".gitignore must not re-ignore Cargo.lock; lockfiles are tracked on purpose"
    );

    let release = workflow("release-plz.yml");
    assert!(
        !release.contains("every `Cargo.lock` is gitignored"),
        "release-plz's lockfile note still claims lockfiles are gitignored"
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
fn scheduled_workflows_are_operationally_distinct() {
    const SCHEDULED: [&str; 3] = [
        "fuzz-nightly.yml",
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

/// Every test job stamps its result identity. The package half comes from the
/// recipe argument; the environment comes from `BISCUIT_CI_ENVIRONMENT`.
#[test]
fn every_test_tier_stamps_its_environment_identity() {
    for (file, expected_jobs) in [("_package-ci.yml", 3), ("_wsl-ci.yml", 1)] {
        let source = workflow(file);
        let mut stamped = 0;
        for job in jobs(&source) {
            if !job.contains("BISCUIT_CI_ENVIRONMENT") {
                continue;
            }
            stamped += 1;
        }
        assert!(
            stamped >= expected_jobs,
            "{file}: every job that runs tests must stamp BISCUIT_CI_ENVIRONMENT (found {stamped})"
        );
    }
}

#[test]
fn wsl_is_an_environment_and_never_a_runner_label() {
    // A WSL job runs on `windows-latest` and executes through `wsl-bash`. If it
    // shared the native matrix, every `runner.os == 'Windows'` branch — native
    // packages, paths, shells, cache keys, artifact names — would apply to a
    // Linux guest. Isolation is structural, not by review.
    let package_ci = workflow("_package-ci.yml");
    assert!(
        package_ci.contains("_wsl-ci.yml"),
        "_package-ci.yml must delegate wsl2-ubuntu rather than host it"
    );
    for job in jobs(&package_ci) {
        if !(job.starts_with("  test:") || job.starts_with("  l2:") || job.starts_with("  browser:"))
        {
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
            "_package-ci.yml: wsl2-ubuntu must never enter a runs-on matrix"
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
        wsl.contains("git clone") && wsl.contains("/home/runner/work/"),
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
fn wsl_provisioning_retries_with_a_delay_and_the_test_step_never_does() {
    // Measured: `Vampire/setup-wsl@v7` already retries internally, and a retry
    // with no delay re-asks a throttle that has not moved, so the delay is the
    // load-bearing part of this guard, not the second attempt.
    let wsl = workflow("_wsl-ci.yml");
    assert_eq!(
        wsl.matches("uses: Vampire/setup-wsl@v7").count(),
        2,
        "provisioning must get exactly one bounded retry — unbounded retries turn a \
         dead runner into a long timeout instead of a fast red cell"
    );
    // Without `continue-on-error` on the first attempt the job dies there and the
    // second is unreachable, which is a retry that never runs.
    assert!(
        wsl.contains("id: provision\n        continue-on-error: true"),
        "the first provisioning attempt must be non-fatal, or the retry is dead code"
    );
    assert!(
        wsl.contains("if: ${{ steps.provision.outcome == 'failure' }}\n        shell: bash")
            && wsl.contains("run: sleep 90"),
        "a delay must separate the two attempts; the action's own retry has none"
    );
    // The second attempt is NOT continue-on-error: a guest that cannot be
    // provisioned twice has to turn the cell red.
    let retry = wsl
        .split("- name: Provision the WSL2 guest (second attempt)")
        .nth(1)
        .expect("_wsl-ci.yml must define a second provisioning attempt");
    let retry = &retry[..retry.find("\n      - name:").unwrap_or(retry.len())];
    assert!(
        !retry.contains("continue-on-error"),
        "a guest that fails to provision twice must still fail the job"
    );

    // The whole point of bounding the retry to provisioning: a retried TEST step
    // turns a timeout or a killed guest into something that eventually looks
    // healthy, which is the exact failure this pipeline exists to surface.
    let executable: String = wsl
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        executable.matches("just _test ").count() == 1,
        "the WSL test step must be invoked exactly once — a retried test hides a real failure"
    );
    let l1 = executable
        .split("id: l1")
        .nth(1)
        .expect("_wsl-ci.yml must define the L1 test step");
    let l1 = &l1[..l1.find("\n      - name:").unwrap_or(l1.len())];
    assert!(
        !l1.contains("continue-on-error"),
        "the L1 test step must never be made non-fatal"
    );
}

#[test]
fn a_failed_wsl_guest_leaves_evidence_of_which_backing_store_ran_out() {
    // Confirmed: extraction exhausts the WINDOWS RUNNER's disk. The host census
    // is the load-bearing measurement AND has to precede extraction — a
    // post-mortem cannot run on a dead runner.
    let wsl = workflow("_wsl-ci.yml");
    let host_census = wsl
        .find("- name: Census the Windows host before extraction")
        .expect("_wsl-ci.yml must measure host headroom before extraction");
    let extraction = wsl
        .find("- name: L1 tests from the archive")
        .expect("_wsl-ci.yml must define the L1 test step");
    assert!(
        host_census < extraction,
        "the host disk census must run BEFORE extraction"
    );
    let executable: String = wsl
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        executable.contains("findmnt -T"),
        "the census must name the FILESYSTEM behind /tmp"
    );
    assert!(
        executable.contains("dmesg -T"),
        "an OOM kill and an ext4 I/O error are recorded in the kernel log and nowhere else"
    );
    assert!(
        executable.contains("ext4.vhdx"),
        "the host post-mortem must size the VHDX and the volume it grows into"
    );
    assert!(
        executable.contains(r#""$workspace/wsl-diag/guest-resources.log""#),
        "the resource sampler must write across the 9p mount, not onto guest ext4"
    );
    for step in ["Guest post-mortem", "Host post-mortem"] {
        let block = wsl
            .split(&format!("- name: {step}"))
            .nth(1)
            .unwrap_or_else(|| panic!("_wsl-ci.yml must define `{step}`"));
        let block = &block[..block.find("\n      - name:").unwrap_or(block.len())];
        assert!(
            block.contains("continue-on-error: true"),
            "`{step}` must not be able to overwrite the tier's real outcome"
        );
    }
    assert!(
        !wsl.contains("name: status-${{ inputs.package }}-wsl-diag")
            && wsl.contains("name: wsl-diag-${{ inputs.package }}-wsl2-ubuntu"),
        "the diagnostic artifact must use a prefix ci-rollup does not walk"
    );
}

#[test]
fn artifact_names_carry_the_environment_not_the_runner_label() {
    // `junit-<package>-L1-windows-latest-*` and
    // `junit-<package>-L1-wsl2-ubuntu-*` are two different cells produced on the
    // same runner label. Keying the artifact to the label would merge them.
    let package_ci = workflow("_package-ci.yml");
    assert!(
        package_ci.contains("name: junit-${{ inputs.package }}-L1-${{ matrix.environment }}"),
        "L1 artifacts must be keyed by package and environment"
    );
    assert!(
        package_ci.contains("name: junit-${{ inputs.package }}-L2-${{ matrix.environment }}"),
        "L2 artifacts must be keyed by package and environment"
    );
    assert!(
        workflow("_wsl-ci.yml").contains("junit-${{ inputs.package }}-L1-wsl2-ubuntu"),
        "the WSL leg's artifact must name the package and the wsl2-ubuntu environment"
    );
}

#[test]
fn l2_runs_on_every_environment_with_a_provisioned_backend() {
    // tmux is the only backend headless CI can host, and it installs on BOTH
    // Linux (apt) and macOS (brew) — so L2 is a matrix, not a hardcoded ubuntu
    // leg. `tmux -V` needs no server, display, or pane, so verification takes no
    // focus; CI must never steal focus.
    let package_ci = workflow("_package-ci.yml");
    assert!(
        package_ci.contains("brew install tmux") && package_ci.contains("apt-get install -y tmux"),
        "the L2 job must provision tmux on both macOS and Linux"
    );
    assert!(
        package_ci.contains("environment: ${{ fromJSON(inputs.l2-environments) }}"),
        "the L2 job must be a matrix over the environments with a provisioned backend"
    );
    assert!(
        package_ci.contains("Report L2 backend coverage") && package_ci.contains("CI=1"),
        "the L2 job must report which declared backends were reachable and which skipped"
    );

    // An unhostable L2 capability is a governed POLICY GAP in environments.json,
    // never a green `0 run / N skipped` cell.
    let environments = read(".github/ci/environments.json");
    assert!(
        environments.contains("\"tmux\": {") && environments.contains("POLICY GAP")
            || read("scripts/ci/affected_scope.py").contains("POLICY GAP"),
        "an unhostable L2 tier must be governed as a policy gap"
    );
}

#[test]
fn a_declared_node_capability_is_provisioned_verified_and_hard_required() {
    // `homelab-server`'s canonical L1 runs a Vue + Vitest + jsdom companion
    // suite after its Rust tests. pnpm exists on no runner image, so the suite
    // must be provisioned on the declared leg and hard-required there.
    let environments = read(".github/ci/environments.json");
    assert!(
        environments.contains("\"node_pnpm\": true"),
        "environments.json must record a node-pnpm-capable environment"
    );

    let policy = read("scripts/ci/affected_scope.py");
    assert!(
        policy.contains("node_pnpm"),
        "affected_scope.py must derive node environments from the capability table"
    );

    let ci = workflow("ci.yml");
    assert_eq!(
        ci.matches("node-environments: ${{ toJSON(matrix.node_environments) }}")
            .count(),
        1,
        "the package fan-out must forward the derived node environments"
    );

    let test_job = job_block("_package-ci.yml", "  test:");
    // Gated on the DERIVED list, never on a runner label written into the YAML.
    assert_eq!(
        test_job
            .matches("if: ${{ contains(fromJSON(inputs.node-environments), matrix.environment) }}")
            .count(),
        3,
        "pnpm setup, Node setup, and the verification step must each gate on the declared \
         capability"
    );
    assert!(
        test_job.contains("- name: Verify the pnpm toolchain") && test_job.contains("pnpm --version"),
        "a declared node capability must be verified reachable in its own named step"
    );
    assert!(
        test_job.contains("BISCUIT_FRONTEND_REQUIRED: ${{ contains(fromJSON(inputs.node-environments), matrix.environment) && '1' || '' }}"),
        "the leg that declared the capability must hard-require it through the recipe too"
    );

    // AC13: the companion suite itself is invoked, not dropped.
    assert!(
        test_job.contains("Companion suite homelab-frontend"),
        "the declared companion suite must execute, not be dropped by the conversion"
    );
    // Its producer status downgrades a green Rust JUnit report on failure (R12).
    // Comment lines are excluded: a YAML comment mentioning the mechanism must
    // not satisfy the guard.
    let status_step = job_block("_package-ci.yml", "  test:")
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .any(|line| line.contains("COMPANION") && line.contains("failure"));
    assert!(
        status_step,
        "a failed companion suite must reach the producer status and downgrade the cell"
    );

    // The recipe self-gates and is identical on every environment — the skip is
    // LOUD and non-fatal off the declared leg, fatal on it.
    let recipe = read("homelab/justfile");
    assert!(
        recipe.contains("FRONTEND TEST SUITE DID NOT RUN"),
        "a missing pnpm must announce that the suite did not run; a silent skip is what produced \
         the PASS-over-a-failed-job cell"
    );
    assert!(
        recipe.contains(r#"if [ -n "${BISCUIT_FRONTEND_REQUIRED:-}" ]; then"#)
            && recipe.contains("PROVISIONING FAILURE"),
        "the recipe must fail when the capability was declared and pnpm is still unreachable"
    );
}

#[test]
fn exclusions_are_owned_and_time_bounded() {
    // A non-gating package is never inferred from zero observed tests; it is
    // always an explicit, owned, dated record in its manifest (AC15).
    let policy = read("scripts/ci/affected_scope.py");
    assert!(
        policy.contains("def validate_expiry") && policy.contains("expired on"),
        "affected_scope.py must reject a lapsed exclusion or policy gap"
    );
    let policies = package_policies();
    let excluded: Vec<&serde_json::Value> = policies
        .iter()
        .filter(|policy| policy["gates"].as_bool() == Some(false))
        .collect();
    assert!(
        !excluded.is_empty(),
        "at least one package must declare a non-gating exclusion"
    );
    for policy in &excluded {
        assert!(
            policy["owner"].as_str().is_some_and(|owner| !owner.is_empty()),
            "an exclusion must name an owner: {}",
            policy["name"]
        );
        assert!(
            policy["reason"].as_str().is_some_and(|reason| !reason.is_empty()),
            "an exclusion must give a reason: {}",
            policy["name"]
        );
        assert!(
            policy["exclusion-class"].as_str().is_some(),
            "an exclusion must declare its class: {}",
            policy["name"]
        );
    }
    // The time-bounded zero-or-few-tests areas are now gating: they record
    // NOTHING TO RUN rather than holding an exclusion.
    for name in ["visualizer", "reaper", "agent-sandbox-cli", "tabby"] {
        let excluded = policies.iter().any(|policy| {
            policy["name"].as_str() == Some(name) && policy["gates"].as_bool() == Some(false)
        });
        assert!(
            !excluded,
            "{name} must be gating (records NOTHING TO RUN), not excluded merely for having zero tests"
        );
    }
}

// --- the single required check: ci-verdict ------------------------------------

/// Every producer job, paired with the `job:` value its status artifact carries.
const PRODUCER_STATUS: [(&str, &str, &str); 6] = [
    ("_package-ci.yml", "  check:", "JOB: check"),
    ("_package-ci.yml", "  test:", "JOB: L1"),
    ("_package-ci.yml", "  lint:", "JOB: lint"),
    ("_package-ci.yml", "  l2:", "JOB: L2"),
    ("_package-ci.yml", "  browser:", "JOB: browser"),
    ("_wsl-ci.yml", "  wsl:", "JOB: L1"),
];

#[test]
fn every_producer_job_emits_an_explicit_status_artifact() {
    // A failed or cancelled producer must be able neither to prevent the
    // verdict from running nor to be silently read as a pass. An explicit
    // status artifact carrying {package, job, environment, result} is the only
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
            block.contains("name: status-${{ inputs.package }}-"),
            "{file}: `{name}` must upload under the package-keyed `status-` prefix"
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
    // `target/nextest/ci/test-results.xml`, so uploading that path published
    // only the LAST package's report. The staging tree holds one XML per
    // invocation plus `manifest.jsonl`, and the manifest is what carries
    // {package, environment, tier} identity.
    for file in ["_package-ci.yml", "_wsl-ci.yml"] {
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
    let package_ci = workflow("_package-ci.yml");
    assert_eq!(
        package_ci.matches("path: target/nextest/ci-reports").count(),
        3,
        "the L1, L2, and browser uploads must each publish the whole staging directory"
    );

    // The guest stages onto ext4, which the Windows host cannot read, so the
    // reports cross the 9p mount once. Copying the staging root's CONTENTS is
    // what puts manifest.jsonl at the artifact root.
    let wsl = workflow("_wsl-ci.yml");
    assert!(
        wsl.contains("BISCUIT_JUNIT_STAGE_DIR: /home/runner/work/")
            && wsl.contains("/target/nextest/ci-reports"),
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

    assert!(
        verdict.contains("if: always()"),
        "ci-verdict must run even when every producer failed"
    );
    for producer in [
        "scope",
        "preflight",
        "package-ci",
        "biscuit-tui-captured-stdout",
        "ci-tooling",
    ] {
        assert!(
            verdict.contains(&format!("      - {producer}\n")),
            "ci-verdict must wait on `{producer}` so its artifacts are downloadable"
        );
    }

    // `--scope` is the ONLY way the rollup learns a package was scheduled and
    // produced nothing. `--policy` is the scope job's resolved package policy;
    // `--environments` is the capability table.
    assert!(
        verdict.contains(r#"--scope "$SCOPE""#)
            && verdict.contains("needs.scope.outputs.package_names"),
        "ci-verdict must pass the affected package scope to the rollup"
    );
    assert!(
        verdict.contains("--policy ci-artifacts/ci-scope/scope.json"),
        "ci-verdict must read the resolved package policy the scope job uploaded"
    );
    assert!(
        verdict.contains("--environments .github/ci/environments.json"),
        "ci-verdict must read the environment capability table"
    );
    assert!(
        !verdict.contains("--areas")
            && !verdict.contains("--provisioned-backends")
            && !verdict.contains("--browser-environments"),
        "ci-verdict must not pass the retired area/provisioned flags"
    );
    assert!(
        verdict.contains("needs.scope.result") && verdict.contains("exit 1"),
        "a broken scope job must fail the verdict, not render a vacuous CLEAR over an empty grid"
    );

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
        verdict.contains("uses: actions/download-artifact@v4") && !verdict.contains("name: junit-"),
        "ci-verdict must download every artifact in the run, not a named subset"
    );
    assert!(
        !ci.contains("baseline-failures.txt"),
        "the retired display-name baseline must have no consumers"
    );
}

#[test]
fn only_ci_verdict_makes_a_run_level_claim() {
    let package_ci = workflow("_package-ci.yml");
    assert!(
        !package_ci.contains("  classify:") && !package_ci.contains("first actionable failure class"),
        "_package-ci.yml must not carry a second, environment-blind failure reporter"
    );

    let summary = job_block("ci.yml", "  summary:");
    assert!(
        !summary.contains("No gate reported a failure"),
        "the advisory summary must make no run-level green claim"
    );
    assert!(
        !summary.contains("needs.package-ci.result"),
        "every package gate is a rollup cell; the advisory summary must not report them a second time"
    );
    assert!(
        summary.contains("ci-verdict"),
        "the advisory summary must name ci-verdict as the run's verdict"
    );
}

// --- shared recipes must stay usable from a runner ---------------------------

#[test]
fn just_commit_is_deterministic_under_ci() {
    let justfile = read("justfile");
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
    assert!(
        recipe.contains("{{ quote(args) }}"),
        "the CI message must be shell-quoted rather than interpolated raw"
    );
}

// --- a PR that schedules no run at all ----------------------------------------

/// A conflicted PR must be reported, not merely fail to schedule anything.
#[test]
fn a_conflicted_pr_is_reported_rather_than_silently_unscheduled() {
    let health = read(".github/workflows/pr-health.yml");
    assert!(
        health.contains("pull_request_target:"),
        "the guard must use `pull_request_target`, which runs against the base \
         branch and needs no merge commit"
    );
    assert!(
        health.contains("branches-ignore: [main]"),
        "a push to a branch with an existing PR must also be covered"
    );
    let steps: String = health
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !steps.contains("actions/checkout"),
        "pr-health must never check out pull-request code under `pull_request_target`"
    );
    assert!(
        health.contains("exit 1"),
        "a conflicted PR must fail the check, not emit a warning"
    );
}

/// The guard and the workflow it guards must agree on which pull requests are in
/// scope.
#[test]
fn pr_health_and_ci_agree_on_which_pull_requests_are_in_scope() {
    fn pull_request_base_filter(source: &str, trigger: &str) -> Option<String> {
        let after = source.split_once(&format!("\n  {trigger}:\n"))?.1;
        after
            .lines()
            .take_while(|line| line.starts_with("    ") || line.trim().is_empty())
            .find(|line| line.trim_start().starts_with("branches:"))
            .map(|line| line.trim().to_string())
    }

    let ci = workflow("ci.yml");
    let health = workflow("pr-health.yml");

    assert_eq!(
        pull_request_base_filter(&ci, "pull_request"),
        None,
        "ci.yml must not filter `pull_request` by base branch"
    );
    assert_eq!(
        pull_request_base_filter(&health, "pull_request_target"),
        None,
        "pr-health must watch the same pull requests as ci.yml"
    );
}

// --- AC11/AC4: sharding is gone; areas.json has no readers -------------------

/// No job passes `--partition`, and no shard identity appears in any result
/// (R7a/AC11).
#[test]
fn no_workflow_passes_partition() {
    let workflows = repo_root().join(".github/workflows");
    for entry in fs::read_dir(&workflows).expect("read .github/workflows") {
        let path = entry.expect("workflow entry").path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("yml") {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let source = read(&format!(".github/workflows/{name}"));
        assert!(
            !source.contains("--partition"),
            "{name} must not pass nextest --partition; sharding is removed (R7a)"
        );
        assert!(
            !source.contains("BISCUIT_CI_SHARD") && !source.contains("BISCUIT_CI_AREA"),
            "{name} must not stamp the retired area/shard identity"
        );
    }
    let devops = read("just/devops.just");
    assert!(
        !devops.contains("BISCUIT_CI_AREA") && !devops.contains("BISCUIT_CI_SHARD"),
        "the staging recipes must not reference the retired area/shard identity"
    );
}

/// AC4: `areas.json` is deleted, and no workflow, recipe, or script reads it.
/// The guard GLOBS — a hardcoded file list cannot keep the repo clean (the
/// homelab justfile carried an areas.json reference while off the old list).
#[test]
fn areas_json_is_gone_and_has_no_readers() {
    assert!(
        !repo_root().join(".github/ci/areas.json").exists(),
        ".github/ci/areas.json must be deleted (Phase 5)"
    );

    let mut candidates: Vec<PathBuf> = Vec::new();
    let mut collect = |dir: &str, extensions: &[&str]| {
        let mut pending = vec![repo_root().join(dir)];
        while let Some(dir) = pending.pop() {
            for entry in fs::read_dir(&dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display())) {
                let path = entry.expect("dir entry").path();
                if path.is_dir() {
                    pending.push(path);
                } else {
                    let name = path.file_name().unwrap().to_string_lossy().to_string();
                    let matches = name == "justfile"
                        || extensions
                            .iter()
                            .any(|ext| name.ends_with(&format!(".{ext}")));
                    if matches {
                        candidates.push(path);
                    }
                }
            }
        }
    };
    collect(".github/workflows", &["yml", "yaml"]);
    collect("just", &["just"]);
    collect("scripts", &["rs", "py", "sh"]);
    candidates.push(repo_root().join("justfile"));
    // One justfile per package area directory.
    for entry in fs::read_dir(repo_root()).expect("read repo root") {
        let path = entry.expect("root entry").path().join("justfile");
        if path.is_file() {
            candidates.push(path);
        }
    }

    assert!(
        candidates.len() > 30,
        "the glob itself must be non-vacuous (found {} files)",
        candidates.len()
    );
    for path in candidates {
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
            .replace("\r\n", "\n");
        assert!(
            !source.contains("areas.json"),
            "{} must not read the deleted areas.json",
            path.display()
        );
    }
}

/// AC12: a declared tier is non-vacuous, and a package that owns its tests
/// declares the tier. The contract scans test definitions rather than nesting
/// workspace builds inside the test suite. Nested `cargo nextest list` calls
/// made this check depend on every unrelated package compiling on the host and
/// multiplied the same cold build across nextest's process-per-test workers.
fn nextest_filter(tier: &str, package: &str) -> String {
    let mut args = vec!["_tier_filter", tier];
    if !package.is_empty() {
        args.push(package);
    }
    let output = Command::new("just")
        .args(&args)
        .current_dir(repo_root())
        .output()
        .unwrap_or_else(|error| panic!("just _tier_filter {tier} {package} failed: {error}"));
    assert!(
        output.status.success(),
        "just _tier_filter {tier} {package} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("a filter expression")
        .trim()
        .to_owned()
}

/// Per-package test counts for the two non-default tiers.
#[derive(Debug, Clone, Copy, Default)]
struct TierCounts {
    l2: usize,
    browser: usize,
}

fn tier_counts(policy: &serde_json::Value) -> TierCounts {
    // Tier markers belong on the test function itself. Keeping that convention
    // makes ownership visible in source and prevents a helper module name from
    // silently moving ordinary tests out of L1.
    fn visit(dir: &std::path::Path, counts: &mut TierCounts) {
        for entry in
            fs::read_dir(dir).unwrap_or_else(|error| panic!("read {}: {error}", dir.display()))
        {
            let path = entry.expect("source entry").path();
            if path.is_dir() {
                if path.file_name().and_then(|name| name.to_str()) != Some("target") {
                    visit(&path, counts);
                }
                continue;
            }
            if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
                continue;
            }

            let source = fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
            let mut test_attribute = false;
            for line in source.lines() {
                let line = line.trim();
                if let Some(attribute) = line.strip_prefix("#[") {
                    let attribute = attribute
                        .split(['(', ']'])
                        .next()
                        .unwrap_or_default()
                        .trim();
                    test_attribute |= attribute.rsplit("::").next().is_some_and(|name| {
                        matches!(name, "test" | "rstest" | "test_case" | "traced_test")
                    });
                    continue;
                }
                if line.is_empty() || line.starts_with("//") {
                    continue;
                }
                if test_attribute {
                    let mut words = line.split_whitespace();
                    while let Some(word) = words.next() {
                        if word != "fn" {
                            continue;
                        }
                        let name = words
                            .next()
                            .unwrap_or_default()
                            .split(['(', '<'])
                            .next()
                            .unwrap_or_default();
                        if name.starts_with("level2_") {
                            counts.l2 += 1;
                        } else if name.starts_with("browser_") {
                            counts.browser += 1;
                        }
                        break;
                    }
                }
                test_attribute = false;
            }
        }
    }

    let manifest = PathBuf::from(
        policy["manifest-path"]
            .as_str()
            .expect("a package manifest path"),
    );
    let mut counts = TierCounts::default();
    visit(
        manifest.parent().expect("manifest has a parent directory"),
        &mut counts,
    );
    counts
}

/// AC12 covers both non-default tiers in one process: declarations must own at
/// least one test, and owned tests must have a declaration so CI schedules them.
#[test]
fn declared_test_tiers_match_owned_tests() {
    assert_eq!(nextest_filter("L2", ""), "test(/(^|::)level2_/)");
    assert_eq!(nextest_filter("browser", ""), "test(/(^|::)browser_/)");

    for policy in gating_packages() {
        let package = policy["name"].as_str().expect("a package name");
        let tiers = policy["tests"]["tiers"]
            .as_array()
            .map(|tiers| {
                tiers
                    .iter()
                    .filter_map(|tier| tier.as_str())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let counts = tier_counts(&policy);

        assert_eq!(
            tiers.contains(&"L2"),
            counts.l2 > 0,
            "{package} must declare the L2 tier exactly when it owns level2_ tests (found {})",
            counts.l2
        );
        assert_eq!(
            tiers.contains(&"browser"),
            counts.browser > 0,
            "{package} must declare the browser tier exactly when it owns browser_ tests (found {})",
            counts.browser
        );
    }
}

/// B5: a change touching only `gates = false` packages impacts packages but
/// schedules none. The fan-out gate must derive from the MATRIX, or that
/// change fails the package-ci job at strategy evaluation.
#[test]
fn the_fan_out_gate_derives_from_the_matrix_not_the_impacted_list() {
    let ci = workflow("ci.yml");
    assert!(
        ci.contains("has_packages=$(jq -r '.matrix | length > 0'"),
        "has_packages must be computed from the matrix; the impacted package list \
         includes non-gating packages"
    );
    assert!(
        !ci.contains("has_packages=$(jq -r '.packages | length > 0'"),
        "deriving has_packages from the impacted list crashes a `gates = false`-only change"
    );
}

/// M4: CI's own tooling — the merge-gate binary, the scope calculator, and
/// the policy store — is not a Cargo package, so a change to it must schedule
/// the leg that runs their own test suites.
#[test]
fn ci_tooling_changes_schedule_the_tooling_leg() {
    let policy = read("scripts/ci/affected_scope.py");
    assert!(
        policy.contains(r#"CI_TOOLING_PREFIXES = ("scripts/", ".github/ci/")"#),
        "affected_scope.py must map scripts/ and .github/ci/ to the ci_tooling flag"
    );

    let ci = workflow("ci.yml");
    assert!(
        ci.contains("ci_tooling=$(jq -r '.flags.ci_tooling'"),
        "the scope job must emit the derived ci_tooling flag"
    );
    let leg = job_block("ci.yml", "  ci-tooling:");
    assert!(
        leg.contains("needs.scope.outputs.ci_tooling == 'true'"),
        "the ci-tooling leg must be gated on the scope-derived flag"
    );
    assert!(
        leg.contains("python3 scripts/ci/test_affected_scope.py"),
        "the ci-tooling leg must run the scope tests"
    );
    assert!(
        leg.contains("cargo nextest run") && leg.contains("--bin ci-rollup"),
        "the ci-tooling leg must run the rollup's test suite"
    );
    // A red tooling leg must not be invisible under a CLEAR verdict.
    let summary = job_block("ci.yml", "  summary:");
    assert!(
        summary.contains("needs.ci-tooling.result"),
        "the advisory summary must classify a ci-tooling failure"
    );
}

/// B3/B4: the WSL guest must have `jq` before its native-prerequisites step
/// parses with it, and the package's declared L1 slow-test contract must reach
/// the guest or darkmatter's L1 suite silently narrows on wsl2-ubuntu only.
#[test]
fn the_wsl_leg_provisions_jq_and_forwards_the_slow_test_contract() {
    let wsl = workflow("_wsl-ci.yml");
    assert!(
        wsl.contains("xz-utils jq"),
        "the WSL guest must be provisioned with jq — the native-prerequisites step \
         parses the package list with it before anything could install it"
    );
    assert!(
        wsl.contains("l1-include-slow:") && wsl.contains("BISCUIT_L1_INCLUDE_SLOW"),
        "_wsl-ci.yml must accept and export the package's l1-include-slow contract"
    );

    let wsl_job = job_block("_package-ci.yml", "  wsl:");
    assert!(
        wsl_job.contains("l1-include-slow: ${{ inputs.l1-include-slow }}"),
        "the WSL leg must receive the package's declared L1 slow-test policy"
    );
}
