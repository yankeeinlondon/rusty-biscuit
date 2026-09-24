//! Durable CI/CD workflow contract tests.
//!
//! These guard the invariants of the package-keyed CI grid
//! (`fixes/2026-08-06-cicd`): the package is the unit of selection, execution,
//! and result identity; the repository tracks no rustc wrapper while `just
//! init` owns the compiler-cache host setup through one ordered sequence
//! (2026-09-23) that CI never enters; the
//! primary workflow runs a bootstrap preflight that gates the package fan-out;
//! declared test tiers are non-vacuous; and release automation follows
//! successful CI instead of racing it. They inspect workflow/action and
//! manifest source so a regression fails locally without a live GitHub Actions
//! run.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

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

// --- D1/D2: no tracked rustc wrapper; init owns the kache host setup ---------

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

/// The Phase 1 compiler-work wrapper is bound to one command, never to a job or
/// a workflow.
///
/// Mixing wrapped and unwrapped units in one `target/` is a documented
/// repository hazard, and a workflow- or job-level `RUSTC_WRAPPER` is exactly
/// how that happens: the runner-tool builds, the native-prerequisite recipes,
/// and the counter's own build would all be wrapped too. The only permitted
/// non-step spelling is the empty value each workflow already sets to clear a
/// stray host wrapper.
#[test]
fn the_compiler_work_wrapper_is_never_global() {
    for file in ["_package-ci.yml", "_wsl-ci.yml", "_area-ci.yml", "ci.yml"] {
        let source = read(&format!(".github/workflows/{file}"));
        for line in source.lines() {
            let executable = line.split('#').next().unwrap_or("");
            if !executable.contains("RUSTC_WRAPPER:") {
                continue;
            }
            let value = executable
                .split_once("RUSTC_WRAPPER:")
                .map(|(_, rest)| rest.trim())
                .unwrap_or("");
            if value == "\"\"" {
                // The clearing value is allowed at any level; it can only ever
                // remove a wrapper.
                continue;
            }
            assert!(
                value.contains("steps.counter.outputs.wrapper"),
                "{file}: the only non-empty `RUSTC_WRAPPER` permitted is the measurement \
                 step's output, so an unmeasured run inherits the empty value; found \
                 `{value}`"
            );
            let indent = line.len() - line.trim_start().len();
            // A step's `env:` entry indents to ten in these files; a job-level
            // `env:` reaches six and a workflow-level one two.
            assert!(
                indent >= 8,
                "{file}: `RUSTC_WRAPPER` naming a binary must sit in a STEP's `env:`, not a \
                 job or workflow one — a job-wide wrapper would also wrap the runner-tool \
                 builds and mix wrapped and unwrapped units in one target/. Found indent \
                 {indent}"
            );
        }
        assert!(
            source.contains("RUSTC_WRAPPER: \"\""),
            "{file}: must still clear a stray host wrapper at workflow level"
        );
    }
}

/// Measuring compiler work changes no cell, artifact, or gate.
///
/// The switch defaults off on every path, and its steps publish a measurement
/// artifact of their own rather than editing the status document the rollup
/// reads to decide a cell's outcome. A baseline that could change a result is
/// not a baseline.
#[test]
fn the_compiler_work_measurement_defaults_off_and_produces_its_own_artifact() {
    // `_wsl-ci.yml` is absent on purpose: the guest owns no compile to measure
    // any more. Its archive came from a package-local producer that no longer
    // exists, and the Linux owner's measurement is published under the
    // wsl2-ubuntu cell exactly as before.
    for file in ["_package-ci.yml", "_area-ci.yml"] {
        let source = read(&format!(".github/workflows/{file}"));
        let input = source
            .split("measure-compiler-work:")
            .nth(1)
            .unwrap_or_else(|| panic!("{file} must declare the measure-compiler-work input"));
        assert!(
            input.contains("type: boolean") && input.contains("default: false"),
            "{file}: measure-compiler-work must be a boolean defaulting to false"
        );
    }

    // `ci.yml` only exposes it on `workflow_dispatch`, so an ordinary pull
    // request or push cannot turn it on.
    let ci = read(".github/workflows/ci.yml");
    let dispatch = ci
        .split("workflow_dispatch:")
        .nth(1)
        .expect("ci.yml declares workflow_dispatch");
    assert!(
        dispatch.contains("measure-compiler-work:"),
        "ci.yml must expose the measurement switch on workflow_dispatch alone"
    );
    assert!(
        ci.contains("measure-compiler-work: ${{ inputs.measure-compiler-work == true }}"),
        "ci.yml must pass the switch as a strict boolean, so a null `inputs` on a \
         pull_request or push reads as false"
    );

    let package = read(".github/workflows/_package-ci.yml");
    let wsl = read(".github/workflows/_wsl-ci.yml");
    assert!(
        !wsl.contains("measure-compiler-work") && !wsl.contains("RUSTC_WRAPPER: ${{"),
        "_wsl-ci.yml compiles nothing and must carry no measurement switch or wrapper"
    );
    assert!(
        package.contains("name: measurement-${{ matrix.package }}-"),
        "_package-ci.yml: the measurement must be its own {{package, gate, environment}}-keyed \
         artifact"
    );
    assert!(
        !package.contains("BISCUIT_CI_STAGE_SECONDS") || !package.contains("compiler_work\":%s"),
        "_package-ci.yml: the measurement must not be written into status.json — the rollup \
         reads that document to decide cell outcomes"
    );
    // The status writers are unchanged: none of them reads a measurement output.
    for needle in ["steps.counter.outputs", "steps.compiler_work.outputs"] {
        for status_block in package.split("- name: Record producer status").skip(1) {
            let block = status_block.split("- name: Upload producer status").next().unwrap_or("");
            assert!(
                !block.contains(needle),
                "_package-ci.yml: a producer status step must not read `{needle}`"
            );
        }
    }
}

#[test]
fn kache_has_a_single_version_floor() {
    let version = read(".github/kache-min-version");
    assert!(
        !version.trim().is_empty(),
        ".github/kache-min-version must hold the single minimum kache version"
    );

    let justfile = read("justfile");
    assert!(
        justfile.contains(".github/kache-min-version"),
        "root justfile must read KACHE_MIN_VERSION from the single authority file"
    );
    assert!(
        !justfile.contains(r#"KACHE_MIN_VERSION := ""#),
        "root justfile must not hard-code a second kache version literal"
    );
    assert!(
        !justfile.contains("--version \"{{ KACHE_MIN_VERSION }}\""),
        "install-kache installs the latest release; the floor is a check, not a pin"
    );
}

/// The root justfile plus every file it `import`s — the full recipe surface a
/// recipe could activate from.
fn justfile_corpus() -> Vec<String> {
    let root = read("justfile");
    let mut corpus = vec![root.clone()];
    for line in root.lines() {
        if let Some(rest) = line.strip_prefix("import \"")
            && let Some(path) = rest.strip_suffix('"')
        {
            corpus.push(read(path));
        }
    }
    corpus
}

/// Ruling 2026-09-23 (`fixes/2026-09-23-ensuring-kache-support`), superseding
/// the 2026-09-09 activation-is-host-policy ruling this test used to pin:
/// `just init` owns the kache host setup end to end through `_ensure-kache`'s
/// ordered sequence — probe, install, store placement, user config write,
/// daemon lifecycle, activation LAST — under the failure contract pinned in
/// `kache_recipe_contracts.rs`. What stays structural here: the repository
/// tracks no rustc wrapper (a qualifying host's activation lives in its own
/// `$CARGO_HOME/config.toml`, written only by that sequence), the installer
/// stays an explicit recipe targeting latest (the floor is a check, not a
/// pin), and no recipe may activate outside the sequence — a bare `kache
/// init` or `export RUSTC_WRAPPER=kache` line, or an activation write from
/// any other recipe, is the ungated activation whose wrapped-but-broken
/// builds this fix exists to prevent.
#[test]
fn init_owns_the_kache_host_setup_through_the_ensure_step() {
    let justfile = read("justfile");
    assert!(
        justfile.contains("    just _ensure-kache\n"),
        "`just init` must run the `_ensure-kache` step"
    );
    assert!(
        justfile.contains("_ensure-kache:"),
        "the `_ensure-kache` step must exist"
    );
    assert!(
        justfile.contains("install-kache binary_only="),
        "the installer must remain available as an explicit recipe (binary_only parameter)"
    );

    // No recipe — in the root justfile or any file it imports — may activate
    // kache by hand.
    for source in justfile_corpus() {
        for line in source.lines() {
            let cmd = line.trim_start();
            assert!(
                !(cmd.starts_with("kache init")
                    || cmd.starts_with("export RUSTC_WRAPPER=kache")),
                "no recipe may activate kache by hand; activation is the last step of \
                 _ensure-kache's ordered sequence (docs/kache-strategy.md)"
            );
        }
    }

    // The activation write itself — the merge helper carrying a NON-EMPTY
    // rustc-wrapper value — may appear only inside `_ensure-kache`. (The
    // empty-value write is the failure contract's undo and travels with it.)
    let mut activators: Vec<String> = Vec::new();
    let mut recipe = String::new();
    for line in justfile.lines() {
        let is_header = !line.is_empty()
            && !line.starts_with(' ')
            && !line.starts_with('\t')
            && !line.starts_with('#')
            && !line.starts_with("import")
            && line.contains(':')
            && !line.contains(":=");
        if is_header {
            recipe = line.split(':').next().unwrap_or("").trim().to_owned();
        }
        if line.contains("rustc-wrapper kache") {
            activators.push(recipe.clone());
        }
    }
    assert_eq!(
        activators,
        vec!["_ensure-kache".to_owned()],
        "the activation write belongs to _ensure-kache's ordered sequence alone — \
         no other recipe may write a non-empty rustc-wrapper"
    );
}

/// The 2026-09-23 CI rule: CI never installs or upgrades kache — runners have
/// no clone-capable store of their own, and `Swatinem/rust-cache` serves the
/// legs that compile. `_ensure-kache` carries an in-recipe CI guard as a
/// backstop, but the contract is stronger than the guard: no workflow may run
/// `just init` (which owns the host setup) or reach for kache's installer at
/// all, so a workflow that starts fighting the runner's own cache setup fails
/// here before it ships.
#[test]
fn ci_never_runs_init_nor_installs_or_upgrades_kache() {
    let workflows = repo_root().join(".github/workflows");
    for entry in fs::read_dir(&workflows).expect("read .github/workflows") {
        let path = entry.expect("workflow entry").path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("yml") {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let source = read(&format!(".github/workflows/{name}"));
        for line in source.lines() {
            // Comments and prose cannot execute; a marker inside them must
            // not fail the contract the way an executable step would.
            let executable = line.split('#').next().unwrap_or("").trim();
            if executable.is_empty() {
                continue;
            }
            assert!(
                !executable.contains("just init"),
                "{name}: CI must not run `just init` — init owns the kache host setup, \
                 and a runner's filesystem is not ours to qualify"
            );
            let manages_kache = executable.contains("kache")
                && (executable.contains("install-kache")
                    || executable.contains("binstall")
                    || executable.contains("cargo install")
                    || executable.contains("kache init"));
            assert!(
                !manages_kache,
                "{name}: CI must not install or upgrade kache (`{executable}`)"
            );
        }
    }
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
    // Job-level `continue-on-error` erases the leg from the run's verdict. The
    // STEP-level normalization of the gate commands is a different thing: the
    // job stays a blocking job, and the failed command reaches the area
    // rollup as a FAILED cell (`a_failed_gate_command_is_normalized_so_its_area_owns_the_outcome`).
    for block in jobs(&workflow("_package-ci.yml")) {
        assert!(
            !executable_lines(&block).contains("\n    continue-on-error"),
            "no gate job may be continue-on-error — that erases the leg from the run's verdict: {}",
            block.lines().next().unwrap_or("").trim()
        );
    }
}

#[test]
fn package_ci_selects_the_ci_nextest_profile_explicitly() {
    let shared = workflow("_package-ci.yml");
    assert!(
        shared.contains("NEXTEST_PROFILE: ci"),
        "package CI must explicitly select the `ci` nextest profile, not rely on detection (D6)"
    );
}

/// AC2/AC10: every matrix is planner-derived, never static or
/// directory-derived — the area fan-out in `ci.yml` and the row sets the area
/// hands to its one execution call.
#[test]
fn the_package_matrix_is_scope_derived_not_static() {
    let ci = workflow("ci.yml");
    assert!(
        ci.contains("fromJSON(needs.scope.outputs.scheduled_areas)")
            && ci.contains("fromJSON(needs.scope.outputs.area_rows)[matrix.area]"),
        "ci.yml must fan out one caller identity per planner-selected area, and \
         hand that area its own planner-built dispatch rows"
    );
    let area = workflow("_area-ci.yml");
    // The four disjoint sets, forwarded verbatim. Nothing here re-derives or
    // re-partitions them: `affected_scope.row_sets` is the only producer.
    for set in ["test", "check", "lint", "wsl"] {
        assert!(
            area.contains(&format!("{set}-rows: ${{{{ toJSON(fromJSON(inputs.rows).{set}) }}}}")),
            "the area must forward the planner's {set} row set unchanged"
        );
    }
    let execution = workflow("_package-ci.yml");
    for set in ["test", "check", "lint", "wsl"] {
        assert!(
            execution.contains(&format!("include: ${{{{ fromJSON(inputs.{set}-rows) }}}}")),
            "the execution workflow must expand the {set} row set as an include matrix, \
             so each row's values become both the job's context and its label (R1)"
        );
    }
    // Every value a job needs beyond dispatch comes from the plan, through the
    // one reader — not from a second input surface.
    assert_eq!(
        3,
        execution.matches("python3 scripts/ci/cell_contract.py").count(),
        "each row-expanding job with steps resolves its contract from the plan; \
         the WSL2 delegation has none and resolves its row inside `_wsl-ci.yml`"
    );
    assert!(
        workflow("_wsl-ci.yml").contains("python3 scripts/ci/cell_contract.py"),
        "the guest resolves its own row against the same plan"
    );
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
    // One test job over three tiers, so the recipe is selected by the cell
    // contract rather than by three near-identical jobs. The mapping lives in
    // `cell_contract.py`, which is where this contract reads it from — a
    // second copy here would be the drift this feature exists to remove.
    assert!(
        shared.contains(r#"just "$RECIPE" "${{ matrix.package }}""#),
        "the test job invokes the canonical recipe the cell contract named"
    );
    let reader = read("scripts/ci/cell_contract.py");
    for (gate, recipe) in [("L1", "_test"), ("L2", "_test_l2"), ("browser", "_test_browser")] {
        assert!(
            reader.contains(&format!(r#""{gate}": "{recipe}""#)),
            "the cell reader must map the {gate} gate to {recipe}"
        );
    }
    // The guest runs exactly one gate, so it names its recipe directly.
    assert!(
        workflow("_wsl-ci.yml").contains(r#"just _test "${{ steps.cell.outputs.package }}""#),
        "the WSL2 guest invokes _test"
    );
    assert!(
        shared.contains(r#"just _lint "${{ matrix.package }}""#),
        "lint invokes _lint"
    );
    // Compile-check is `cargo check` over the planner's `check_args` (R9,
    // AC3): there is no per-package canonical check recipe, and the target
    // selection belongs to the plan, not the workflow.
    assert!(
        shared.contains("run: cargo check ${{ steps.cell.outputs.check_args }}"),
        "the check job runs cargo check over the plan's package selector"
    );
}

/// Open Question 1, ruled Option B (2026-09-12): a changed package's unchanged
/// direct reverse dependencies are compiled as a second step of its own check
/// cell, on `ubuntu-latest` only, with the planner's explicit arguments. The
/// consumer gets no job of its own; the planner's environment constant and the
/// step's condition must spell the same label, or the planner would give a
/// package a cell whose compile step never runs.
#[test]
fn unchanged_dependents_are_compiled_inside_the_changed_packages_linux_check() {
    // The seam travels on the PLAN, so the reader is what must name it — a
    // workflow input would be a second place to keep it aligned.
    let reader = read("scripts/ci/cell_contract.py");
    for field in ["dependents_check_args", "dependents", "dependents_native"] {
        assert!(
            reader.contains(&format!(r#""{field}""#)),
            "the cell reader must publish `{field}` from the plan's dependent seam"
        );
    }

    let check = job_block("_package-ci.yml", "  check:");
    let steps = steps(&check);
    let native = step_named(&steps, "Install native prerequisites").expect("check needs native setup");
    assert!(step_env(native).iter().any(|(key, value)|
        key == "DEPENDENTS_NATIVE" && value == "${{ steps.cell.outputs.dependents_native }}"
    ));
    for job in ["test", "lint"] {
        assert!(
            !job_block("_package-ci.yml", &format!("  {job}:")).contains("dependents_native"),
            "dependent provisioning belongs only to the owning check"
        );
    }
    let step = step_with_id(&steps, "dependents")
        .expect("the check job must carry the `dependents` compile step");
    assert_eq!(step_name(step), "Compile unchanged dependents");
    let executable = executable_lines(step);
    assert!(
        executable.contains("run: cargo check ${{ steps.cell.outputs.dependents_check_args }}"),
        "the dependents step runs cargo check over the planner's explicit arguments"
    );
    assert!(
        !executable.contains("--all-targets"),
        "the dependents' target selection belongs to the plan, never a blanket flag"
    );
    let condition = executable
        .lines()
        .find(|line| line.trim_start().starts_with("if:"))
        .expect("the dependents step must be conditional");
    assert!(
        condition.contains("steps.cell.outputs.dependents_check_args != ''"),
        "the step must skip when the planner attributed no dependent: {condition}"
    );
    // The ubuntu-only rule is spelled twice — once by the planner, once here —
    // and both spellings must be the same literal.
    let planner = read("scripts/ci/affected_scope.py");
    let constant = planner
        .lines()
        .find_map(|line| line.strip_prefix("DEPENDENTS_ENVIRONMENT = "))
        .expect("the planner must define DEPENDENTS_ENVIRONMENT")
        .trim()
        .trim_matches('"');
    assert_eq!(constant, "ubuntu-latest");
    assert!(
        condition.contains(&format!("matrix.environment == '{constant}'")),
        "the step condition must name the planner's dependents environment: {condition}"
    );

    // The status fold names what was compiled and which half broke.
    let status = step_named(&steps, "Record producer status").expect("checked elsewhere");
    let env = step_env(status);
    assert!(
        env.iter()
            .any(|(key, value)| key == "DEPENDENTS" && value == "${{ steps.cell.outputs.dependents }}"),
        "the status step must carry the dependent names into the artifact"
    );
    let script = step_script(status);
    assert!(
        script.contains("dependents: $dependents") && script.contains("detail: $detail"),
        "the status artifact must record `dependents` and a `detail` naming the failed half"
    );
}

/// AC3: the check command's target selection is the planner's explicit
/// `--examples`/`--benches`, so the workflow may add no blanket flag of its
/// own — and those selectors must never reach the archive the WSL2 guest runs,
/// whose target kinds are the L1 build's and nothing more.
#[test]
fn the_check_command_adds_no_blanket_target_flag() {
    let source = workflow("_package-ci.yml");
    let offenders: Vec<String> = jobs(&source)
        .iter()
        .flat_map(|job| job.lines())
        .filter(|line| !line.trim_start().starts_with('#') && line.contains("--all-targets"))
        .map(|line| line.trim().to_owned())
        .collect();
    assert!(
        offenders.is_empty(),
        "_package-ci.yml runs a blanket target flag outside comments: {offenders:?}"
    );

    // The guest is a pure consumer now: it is handed the plan's build records
    // and downloads one, so there is no archive-args input it could misuse and
    // no place a check selector could enter.
    let wsl_delegation = job_block("_package-ci.yml", "  wsl2:");
    assert!(
        wsl_delegation.contains("row: ${{ toJSON(matrix) }}"),
        "the WSL2 cell must be handed its dispatch row and resolve the rest, \
         including its build record, from the plan"
    );
    // Comments explain what was removed; only executable YAML can violate it.
    let delegation_yaml = executable_lines(&wsl_delegation);
    let wsl_yaml = executable_lines(&workflow("_wsl-ci.yml"));
    for forbidden in ["check_args", "archive-args"] {
        assert!(
            !delegation_yaml.contains(forbidden),
            "`{forbidden}` must not reach the guest: its target kinds are the L1 build's"
        );
        assert!(
            !wsl_yaml.contains(forbidden),
            "_wsl-ci.yml consumes a build record; `{forbidden}` is not an input it declares"
        );
    }
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
        shared.contains(r#"just _lint "${{ matrix.package }}""#),
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
    // backends the PLAN attached to the cell (the hostable subset of the
    // package's declaration), so an installed-but-never-exercised backend
    // fails `_test_l2`'s backend-proof bracket instead of rendering a green
    // cell with zero executed L2 tests.
    assert!(
        shared.contains("BISCUIT_TEST_REQUIRED_BACKENDS"),
        "the L2 job must require the provisioned backends through BISCUIT_TEST_REQUIRED_BACKENDS"
    );
    // The list is the cell's verbatim. Narrowing it here by name is what made
    // the expected manifest and the completion validator disagree: the job
    // recorded `["tmux"]` while the validator demanded the package's whole
    // declaration (review-1 of 2026-09-19-direct-cell-execution).
    assert!(
        !shared.contains(r#"select(. == "tmux")"#),
        "the L2 job must not hardcode which declared backend it requires; the plan decides"
    );
    assert!(
        shared.contains("REQUIRED_BACKENDS: ${{ steps.cell.outputs.backends }}"),
        "the required backends come from the cell contract's `backends` output"
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
        shared.contains("steps.cell.outputs.native_packages")
            && shared.contains("just _ensure-native-libs"),
        "the reusable workflow must provision the plan-derived native closure"
    );
}

#[test]
fn native_prerequisites_are_installed_before_anything_is_built() {
    const BUILD_COMMANDS: [&str; 4] = [
        "cargo check ",
        "cargo llvm-cov",
        r#"just "$RECIPE""#,
        "just _lint",
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
    // check, test, and lint in the reusable execution workflow — three row
    // sets, three jobs, the test one covering all three tiers — plus
    // `ci.yml`'s build owner, the one job that compiles what every archive
    // consumer then runs and therefore the one whose missing prerequisite
    // would break every consumer at once.
    // (Coverage left CI entirely — decided 2026-08-12; `just coverage` is the
    // local tool.)
    assert_eq!(
        provisioning_jobs, 4,
        "every building CI job must provision native prerequisites"
    );
}

/// The executable text of one `case` branch of the gate step's script, with
/// `#` comments removed.
///
/// Stripping is the whole point: each branch's comment *explains* the flag it
/// sets, so a substring search over the raw branch is satisfied by the prose
/// alone and still passes when the assignment itself is deleted. Both tier
/// contracts below were vacuous that way until 2026-09-21.
fn tier_branch_code(tier: &str) -> String {
    let test_job = job_block("_package-ci.yml", "  test:");
    let test_steps = steps(&test_job);
    let gate = step_with_id(&test_steps, "gate").expect("the test job carries the gate command");
    step_script(gate)
        .split(&format!("{tier})"))
        .nth(1)
        .unwrap_or_else(|| panic!("the gate command branches on the {tier} tier"))
        .split(";;")
        .next()
        .unwrap_or_default()
        .lines()
        .map(|line| line.split('#').next().unwrap_or_default())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn the_l1_suite_runs_no_fail_fast() {
    let shared = workflow("_package-ci.yml");
    // The tiers share one job, so the flag is set per tier branch.
    assert!(
        tier_branch_code("L1").contains("tier_args=(--no-fail-fast)"),
        "L1 suites must run --no-fail-fast so one failure cannot hide the suite's evidence (D7)"
    );
    assert!(
        workflow("_wsl-ci.yml").contains("--no-fail-fast"),
        "the guest runs the same L1 contract"
    );
    assert!(
        shared.contains("actions/upload-artifact")
            && shared.contains("name: ${{ steps.cell.outputs.junit_artifact }}"),
        "each test cell must publish its own per-package JUnit artifact (D7)"
    );
}

/// R1's environment policy is per *environment*, not per tier: a truncated L2
/// report costs the same full round-trip as a truncated L1 one, on the tier
/// whose backends an author's host is least likely to have. The L2 branch
/// carried no flag until 2026-09-21 — an omission preserved across the
/// row-driven refactor, never a decision.
#[test]
fn the_l2_suite_runs_no_fail_fast() {
    assert!(
        tier_branch_code("L2").contains("tier_args=(--no-fail-fast)"),
        "L2 suites must run --no-fail-fast so one failure cannot hide the rest \
         of the tier's evidence (R1: in CI, run to completion)"
    );
}

// --- D12: specialized runtime contracts are reusable and orchestrated ---------

/// Each surviving specialized runtime workflow the primary orchestrator calls,
/// paired with its unique runtime evidence and scope selector.
///
/// Empty as of `fixes/2026-09-13-cicd-redundancies`: the last entry,
/// `biscuit-tui-windows-captured-stdout.yml`, retired when its test became
/// ordinary `windows-latest` L1 evidence inside `biscuit-tui-cli`'s own cell.
/// The shape contract below still applies to any entry that returns, which is
/// why the inventory survives as a declaration rather than being deleted.
const ORCHESTRATED: [(&str, &str, &str); 0] = [];

#[test]
fn retired_specialized_workflows_and_jobs_are_absent() {
    let mut live = Vec::new();
    for name in [
        "messenger-desktop-tests.yml",
        "rendezvous-tests.yml",
        "playa-windows.yml",
        "biscuit-tui-windows-captured-stdout.yml",
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
        "  biscuit-tui-captured-stdout:",
        "  ci-tooling:",
        "  summary:",
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
        "biscuit-tui-windows-captured-stdout.yml",
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
    assert!(
        !ci_topic.contains("claudine-windows-ctrl-c.yml"),
        "the active specialized inventory must contain only executable survivors"
    );
}

/// The messenger desktop stubs are the reference case for every compile-time
/// tool a test needs: six non-test binaries no consumer can build for itself.
///
/// The producer supplies them as a verified build sidecar, on every
/// environment. Task 6.5 deleted the legacy `cargo build` prebuild that served
/// an environment still compiling in place; a consumer must not be able to
/// reach one, and this is the reference case for saying so.
#[test]
fn messenger_stub_runner_tool_reaches_native_and_wsl2_execution() {
    let native_test = job_block("_package-ci.yml", "  test:");
    let wsl_test = job_block("_wsl-ci.yml", "  wsl:");
    let wsl_delegation = job_block("_package-ci.yml", "  wsl2:");
    let mut missing = Vec::new();

    if native_test.contains("name: Build messenger desktop stubs")
        || native_test.contains("-p messenger")
    {
        missing.push("the retired native prebuild still present");
    }
    if !native_test.contains("name: Provision the verified build sidecars")
        || !native_test.contains("stub_dunstify")
        || !native_test.contains("MESSENGER_STUB_BIN_DIR")
        || !native_test.contains("GITHUB_ENV")
    {
        missing.push("archive consumers taking the stubs from the verified sidecar directory");
    }
    if executable_lines(&native_test).contains("cargo build") {
        missing.push("a test job free of every nested fixture build");
    }
    if !wsl_test.contains("-sidecars'")
        || !wsl_test.contains("chown -R biscuit:biscuit")
        || !wsl_test.contains("MESSENGER_STUB_BIN_DIR")
    {
        missing.push("WSL2 sidecar delivery to the unprivileged guest");
    }
    if !wsl_delegation.contains("row: ${{ toJSON(matrix) }}")
        || !wsl_test.contains("scripts/ci/cell_contract.py")
    {
        // The guest resolves the package's runner tools from the plan, like
        // every other value; forwarding them as a second input would be a
        // second place to keep aligned.
        missing.push("_package-ci.yml row delegation to _wsl-ci.yml");
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

    // Package-keyed evidence, on both the native and the WSL2 legs. The names
    // themselves are derived by `cell_contract.py` from the cell key, which is
    // where the package-keying now lives.
    let reader = read("scripts/ci/cell_contract.py");
    for file in ["_package-ci.yml", "_wsl-ci.yml"] {
        let source = workflow(file);
        if !source.contains("name: ${{ steps.cell.outputs.junit_artifact }}")
            || !source.contains("name: ${{ steps.cell.outputs.status_artifact }}")
        {
            missing.push(format!("{file} package-keyed L1 evidence"));
        }
    }
    if !reader.contains(r#"f"junit-{row['package']}-{gate}-{row['environment']}""#)
        || !reader.contains(r#"f"status-{row['package']}-{gate}-{row['environment']}""#)
    {
        missing.push("package-keyed native/WSL2 L1 evidence".to_owned());
    }

    let gate = job_block("ci.yml", "  ci-gate:");
    for retired_job in ["rendezvous", "messenger-desktop"] {
        if gate.contains(&format!("      - {retired_job}\n")) {
            missing.push(format!("ci-gate still depends on {retired_job}"));
        }
    }
    let rollup = job_block("_area-ci.yml", "  coverage-audit:");
    if !rollup.contains("uses: actions/download-artifact@")
        || !rollup.contains("ci-rollup rollup")
        || !rollup.contains("ci-rollup verdict")
    {
        missing.push("artifact-driven area coverage-audit consumption".to_owned());
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
    // The stages `ci-reporting` can see. It waits on `ci-gate` rather than on
    // `preflight`, so a preflight failure is reported by the gate it blocks
    // and the summary must say so instead of naming a job it cannot read.
    for stage in [
        "bootstrap (reuse check)",
        "bootstrap (scope calculation)",
    ] {
        assert!(
            ci.contains(stage),
            "the failure-class summary must be able to report `{stage}`"
        );
    }
    assert!(
        ci.contains("it folds every blocking job's result, including \\`preflight\\`"),
        "the summary must point a reader at the gate that folds the stages it \
         cannot see itself"
    );
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
    for name in ["ci.yml", "_area-ci.yml", "_package-ci.yml"] {
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

/// release-plz resolves the branch it is releasing through
/// `git rev-parse --abbrev-ref --symbolic-full-name @{upstream}`. A checkout
/// pinned to a bare SHA detaches HEAD, that expression answers `fatal: HEAD
/// does not point to a branch`, and the calculation dies before computing a
/// single version — which is how every release run between 2026-08-31 and
/// 2026-09-13 failed. The validated SHA must still be honoured, as a freshness
/// comparison against the branch tip rather than as the checkout ref.
#[test]
fn the_release_calculation_runs_on_a_branch_not_a_detached_head() {
    let release = workflow("release-plz.yml");
    assert!(
        !release.contains("ref: ${{ github.event.workflow_run.head_sha }}"),
        "release-plz must not check out a bare SHA; a detached HEAD has no `@{{upstream}}`"
    );
    assert!(
        release.contains("github.event.workflow_run.head_sha"),
        "the validated SHA must still be compared against the checked-out branch tip"
    );
}

/// The `workflow_run` definition is read from the default branch, so the
/// release calculation cannot be proven on a pull request. `workflow_dispatch`
/// honours the selected ref, so it is the one seam that can — and it must stay
/// unable to publish.
#[test]
fn the_release_calculation_is_provable_from_a_branch_without_releasing() {
    let release = workflow("release-plz.yml");
    assert!(
        release.contains("workflow_dispatch:"),
        "release-plz needs a dispatchable seam; `workflow_run` always runs the default branch's definition"
    );
    let dry_run = job_block("release-plz.yml", "  release-dry-run:");
    assert!(
        dry_run.contains("github.event_name == 'workflow_dispatch'"),
        "the dry run must be reachable only by dispatch"
    );
    assert!(
        dry_run.contains("contents: read"),
        "the dry run must drop write permissions; it may not release"
    );
    assert!(
        dry_run.contains("release-plz update"),
        "the dry run must exercise the version calculation that fails on a bad git state"
    );
    for forbidden in ["command: release-pr", "command: release\n", "GITHUB_TOKEN"] {
        assert!(
            !dry_run.contains(forbidden),
            "the dry run must not reach a publishing path (found {forbidden:?})"
        );
    }
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
    for authority in ["rust-toolchain.toml", ".github/kache-min-version", "nextest"] {
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
    for (file, expected_jobs) in [("_package-ci.yml", 1), ("_wsl-ci.yml", 1)] {
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
        if !(job.starts_with("  test:")
            || job.starts_with("  test-l2:")
            || job.starts_with("  test-browser:"))
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
        wsl.contains("git clone") && wsl.contains("env.GUEST_ROOT"),
        "the WSL job must check out onto ext4 at its own `GUEST_ROOT`"
    );
    // Relocation, not reconstruction. The guest used to recreate the producer's
    // literal checkout path because ~150 targets read the compile-time
    // `CARGO_MANIFEST_DIR`; those are gone, and reintroducing the derivation
    // would hide the next one that comes back.
    assert!(
        !wsl.contains("producer_workspace"),
        "the WSL guest path must not be derived from the producer's workspace"
    );
    // Build once on Linux, run in the guest: no toolchain install, no compile,
    // and the SAME artifact the native Linux tiers consume.
    assert!(
        wsl.contains("--archive-file") && wsl.contains("--workspace-remap"),
        "the WSL leg must run from a nextest archive rather than compiling in the guest"
    );
    assert!(
        !executable.contains("cargo nextest archive") && !executable.contains("cargo build"),
        "the WSL workflow must produce nothing: its archive comes from the run's Linux owner"
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
    // Never retried and visible on the job. The status artifact independently
    // preserves the cell-level diagnosis.
    assert!(
        !l1.contains("continue-on-error"),
        "the L1 test step must fail its producer visibly"
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
        !wsl.contains("name: status-${{ steps.cell.outputs.package }}-wsl-diag")
            && wsl.contains("name: wsl-diag-${{ steps.cell.outputs.package }}-wsl2-ubuntu"),
        "the diagnostic artifact must use a prefix ci-rollup does not walk"
    );
}

#[test]
fn artifact_names_carry_the_environment_not_the_runner_label() {
    // `junit-<package>-L1-windows-latest-*` and
    // `junit-<package>-L1-wsl2-ubuntu-*` are two different cells produced on the
    // same runner label. Keying the artifact to the label would merge them.
    // The names are derived ONCE, by the cell reader, from the cell key the
    // row names — which is what makes "the environment, never the runner
    // label" true by construction rather than by five careful spellings.
    let reader = read("scripts/ci/cell_contract.py");
    assert!(
        reader.contains(r#"f"junit-{row['package']}-{gate}-{row['environment']}""#),
        "the JUnit artifact must be keyed by package, gate, and ENVIRONMENT"
    );
    assert!(
        reader.contains(r#"f"completion-{row['package']}-{gate}-{row['environment']}""#),
        "so must the completion record"
    );
    assert!(
        !reader.contains("row['runner']}\""),
        "no result identity may carry the runner label: `windows-latest` hosts \
         both the native Windows cell and the wsl2-ubuntu guest"
    );
    for file in ["_package-ci.yml", "_wsl-ci.yml"] {
        let source = workflow(file);
        assert!(
            source.contains("name: ${{ steps.cell.outputs.junit_artifact }}")
                && source.contains("name: ${{ steps.cell.outputs.status_artifact }}")
                && source.contains("name: ${{ steps.cell.outputs.completion_artifact }}"),
            "{file}: every producer publishes the names the cell reader derived"
        );
    }
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
    // The planner decides where an L2 cell exists at all — package policy
    // crossed with the environment capability table — and emits one row for
    // each. The workflow provisions on exactly those rows and nowhere else.
    let test_job = job_block("_package-ci.yml", "  test:");
    for step in ["Provision and verify the tmux backend", "Report L2 backend coverage"] {
        let block = test_job
            .split(&format!("- name: {step}"))
            .nth(1)
            .unwrap_or_else(|| panic!("_package-ci.yml must define `{step}`"));
        let block = &block[..block.find("\n      - name:").unwrap_or(block.len())];
        assert!(
            block.contains("if: ${{ matrix.gate == 'L2' }}"),
            "`{step}` belongs to the L2 rows alone"
        );
    }
    assert!(
        package_ci.contains("Report L2 backend coverage")
            && package_ci.contains("CI is truthy"),
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

/// A `requires-toolchain` suite (`repo-deps`, `test-toolkit`) drives `cargo`
/// and `rustc` itself, but its binaries arrive in an archive that brings no
/// toolchain. On run 35326800778 the hosted consumer's rustup proxy installed
/// the pin from inside the first tests to reach it, concurrently, and the race
/// corrupted the download directory. The pin is installed once, up front, on
/// exactly the cells whose package declares it and whose environment has
/// `cargo_toolchain`.
#[test]
fn a_declared_toolchain_requirement_is_provisioned_once_before_the_suite_runs() {
    let policy = read("scripts/ci/affected_scope.py");
    assert!(
        policy.contains(r#""requires_toolchain": tests.get("requires-toolchain", False)"#),
        "the plan's package record must carry the `requires-toolchain` declaration"
    );

    // The decision stays the planner's; the reader hands it to the job as one
    // resolved flag, so the workflow carries no capability table of its own.
    let reader = read("scripts/ci/cell_contract.py");
    assert!(
        reader.contains(r#"capability(environment, "cargo_toolchain")"#),
        "the cell reader must cross `requires-toolchain` with the environment's \
         declared capability, from the plan"
    );

    let test_job = job_block("_package-ci.yml", "  test:");
    let all = steps(&test_job);
    let step = step_named(&all, "Set up the pinned Rust toolchain")
        .expect("the L1 consumer must provision the pinned toolchain in a named step");
    assert!(
        step.contains("if: ${{ steps.cell.outputs.requires_toolchain == 'true' }}"),
        "the toolchain step must gate on the plan's answer, never on a runner label"
    );
    assert!(
        step.contains("run: rustup show"),
        "the step installs the pin `rust-toolchain.toml` names, nothing else"
    );
    let index = |name: &str| {
        all.iter()
            .position(|step| step.starts_with(&format!("      - name: {name}\n")))
            .unwrap_or_else(|| panic!("the test job must define the `{name}` step"))
    };
    assert!(
        index("Set up the pinned Rust toolchain") < index("Tests"),
        "the toolchain must be provisioned before the suite starts, not repaired after"
    );
    assert!(
        index("Set up the pinned Rust toolchain") < index("List this cell's expected tests"),
        "listing RUNS each test binary, so it needs the same provisioning the gate does"
    );
    assert_eq!(
        all.iter().filter(|step| step.contains("rustup")).count(),
        1,
        "no other step in the consumer may touch rustup: a second one would be a toolchain \
         the recipe could compile a replacement with"
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

    assert!(
        policy.contains("\"requires_node\""),
        "the planner must resolve the capability per CELL, so a consumer reads \
         one answer instead of re-crossing the table"
    );

    let test_job = job_block("_package-ci.yml", "  test:");
    // Gated on the PLAN's answer, never on a runner label written into the YAML.
    assert_eq!(
        test_job
            .matches("if: ${{ steps.cell.outputs.requires_node == 'true' }}")
            .count(),
        4,
        "pnpm setup, Node setup, the verification step, and the workspace install must each \
         gate on the declared capability"
    );
    assert!(
        test_job.contains("- name: Verify the pnpm toolchain") && test_job.contains("pnpm --version"),
        "a declared node capability must be verified reachable in its own named step"
    );
    // A toolchain on PATH is not a runnable suite. `tsc` and `vitest` resolve
    // through `node_modules`, so a companion whose recipe calls `pnpm` directly
    // dies on a missing binary without this — which is how
    // `test-audit-typecheck` failed once the suites moved onto the registry,
    // away from a job whose own step paired the install with the command.
    assert!(
        test_job.contains("- name: Install workspace Node dependencies")
            && test_job.contains("pnpm install --frozen-lockfile"),
        "a declared node capability must install the workspace lockfile before any suite runs"
    );
    assert!(
        test_job.contains(
            "BISCUIT_FRONTEND_REQUIRED: ${{ steps.cell.outputs.requires_node == 'true' && '1' || '' }}"
        ),
        "the cell that declared the capability must hard-require it through the recipe too"
    );

    // AC13: the declared companion suites are invoked, not dropped. Which
    // suites those are is the registry's answer (spec section 3: validate
    // identities and owners, not shell-command strings), so what this pins is
    // that the job runs the registry-driven runner for its own cell.
    assert!(
        test_job.contains("scripts/ci/companion_suites.py")
            && test_job.contains(r#"--gate "$GATE""#)
            && test_job.contains(r#"--environment "$ENVIRONMENT""#),
        "the declared companion suites must execute, resolved against the \
         registry for THIS cell's environment"
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

// --- the single required check: ci-gate ---------------------------------------

/// Every producer job, paired with the `job:` value its status artifact
/// carries. The native test job covers L1, L2, and browser in one row set, so
/// its gate is the ROW's — which is exactly the identity `ci-rollup` keys on.
const PRODUCER_STATUS: [(&str, &str, &str); 4] = [
    ("_package-ci.yml", "  check:", "JOB: check"),
    ("_package-ci.yml", "  test:", "JOB: ${{ matrix.gate }}"),
    ("_package-ci.yml", "  lint:", "JOB: lint"),
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
            block.contains("JOB_STATUS: ${{ job.status }}"),
            "{file}: `{name}` must read GitHub's own job status — the fold that turns a \
             failed gate command into `result: failure` starts from it"
        );
        assert!(
            block.contains(job_value),
            "{file}: `{name}` must publish `{job_value}` — ci-rollup reads `job` as a tier"
        );
        assert!(
            block.contains("name: ${{ steps.cell.outputs.status_artifact }}"),
            "{file}: `{name}` must upload the status artifact the cell reader named"
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

// --- truthful producers and retained cell evidence ---------------------------

/// Every gate-command step, as `(file, job header, step id, step name)`. A
/// gate command is a test, lint, or compile command whose failure must fail the
/// producer while its status artifact retains the cell-level diagnosis.
const GATE_STEPS: [(&str, &str, &str, &str); 8] = [
    ("_package-ci.yml", "  check:", "check", "cargo check (declared example/bench kinds)"),
    ("_package-ci.yml", "  check:", "dependents", "Compile unchanged dependents"),
    ("_package-ci.yml", "  test:", "gate", "Tests"),
    ("_package-ci.yml", "  test:", "companion", "Companion suites"),
    ("_package-ci.yml", "  lint:", "clippy", "Lint"),
    ("_package-ci.yml", "  lint:", "zed", "Verify the Zed extension package"),
    ("_package-ci.yml", "  lint:", "companion", "Companion suites (lint)"),
    ("_wsl-ci.yml", "  wsl:", "l1", "L1 tests from the archive"),
];

/// The steps of a job block, each as its own text (comments between steps
/// dropped, comments inside a step kept).
fn steps(job: &str) -> Vec<String> {
    let body = match job.split_once("    steps:\n") {
        Some((_, body)) => body,
        None => return Vec::new(),
    };
    let mut steps: Vec<String> = Vec::new();
    for line in body.lines() {
        if line.starts_with("      - ") {
            steps.push(String::new());
        } else if line.starts_with("      #") || steps.is_empty() {
            continue;
        }
        if let Some(step) = steps.last_mut() {
            step.push_str(line);
            step.push('\n');
        }
    }
    steps
}

fn step_named<'a>(steps: &'a [String], name: &str) -> Option<&'a String> {
    steps
        .iter()
        .find(|step| step.starts_with(&format!("      - name: {name}\n")))
}

fn step_with_id<'a>(steps: &'a [String], id: &str) -> Option<&'a String> {
    steps
        .iter()
        .find(|step| step.contains(&format!("\n        id: {id}\n")))
}

fn step_name(step: &str) -> String {
    step.lines()
        .next()
        .unwrap_or("")
        .trim_start_matches("      - name: ")
        .to_owned()
}

/// The `KEY: value` entries of a step's `env:` block.
fn step_env(step: &str) -> Vec<(String, String)> {
    let Some((_, rest)) = step.split_once("\n        env:\n") else {
        return Vec::new();
    };
    rest.lines()
        .take_while(|line| line.starts_with("          "))
        .filter(|line| !line.trim_start().starts_with('#'))
        .filter_map(|line| line.trim().split_once(": "))
        .map(|(key, value)| (key.to_owned(), value.to_owned()))
        .collect()
}

/// A step's `run: |` script, dedented to what the runner executes.
fn step_script(step: &str) -> String {
    let (_, rest) = step
        .split_once("\n        run: |\n")
        .expect("the step must carry a block-scalar run script");
    rest.lines()
        .take_while(|line| line.is_empty() || line.starts_with("          "))
        .map(|line| line.get(10..).unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

/// A failed test, lint, or compile command must be visible as a failed producer
/// job. Its status artifact still records the same failed cell so the area
/// coverage audit can render the precise package/environment/tier diagnosis.
#[test]
fn a_failed_gate_command_fails_its_producer_and_is_still_reported() {
    for (file, header, id, name) in GATE_STEPS {
        let job = job_block(file, header);
        let steps = steps(&job);
        let gate = step_with_id(&steps, id)
            .unwrap_or_else(|| panic!("{file}: `{}` must carry the `{id}` gate step", header.trim()));
        assert_eq!(
            step_name(gate),
            name,
            "{file}: the `{id}` step must be the `{name}` gate command"
        );
        assert!(
            !executable_lines(gate).contains("\n        continue-on-error: true\n"),
            "{file}: `{name}` must fail its producer visibly; fail-fast:false preserves the \
             other unevidenced matrix cells"
        );

        let status = step_named(&steps, "Record producer status")
            .unwrap_or_else(|| panic!("{file}: `{}` must record its producer status", header.trim()));
        let env = step_env(status);
        assert!(
            env.iter()
                .any(|(_, value)| value.contains(&format!("steps.{id}.outcome"))),
            "{file}: the status step of `{}` must read `steps.{id}.outcome` — a gate outcome \
             that never reaches the status artifact is a failure the rollup cannot see",
            header.trim()
        );
        assert!(
            env.iter()
                .any(|(key, value)| key == "JOB_STATUS" && value == "${{ job.status }}"),
            "{file}: the status step must still read `job.status` as JOB_STATUS, so setup, \
             upload, and cancellation failures are reported unchanged"
        );
        assert!(
            !env.iter().any(|(key, _)| key == "RESULT"),
            "{file}: `result` is folded from JOB_STATUS and the gate outcomes, never taken \
             from `job.status` alone"
        );
        let script = step_script(status);
        assert!(
            script.contains("result=failure"),
            "{file}: the status step must retain a defensive failed-gate fold so the \
             artifact cannot claim success if job.status is observed during always()"
        );
    }
}

/// Only bounded recovery and diagnostic steps may ignore an error. Gate,
/// setup, staging, and upload failures must remain producer failures.
#[test]
fn only_recovery_and_diagnostic_steps_ignore_errors() {
    let normalized = |file: &str, header: &str| -> Vec<String> {
        steps(&job_block(file, header))
            .iter()
            .filter(|step| executable_lines(step).contains("continue-on-error: true"))
            .map(|step| step_name(step))
            .collect()
    };
    // The Phase 1 compiler-work measurement is the one advisory pair in these
    // jobs. It publishes a baseline, not a result: a counter that failed to
    // build or a measurement artifact that failed to upload must leave the
    // cell's outcome exactly as the gate command decided it.
    let measurement = vec![
        "Report compiler work".to_owned(),
        "Upload the compiler-work measurement".to_owned(),
    ];
    // `check` and `lint` are measured too, from Phase 5: each is its own
    // compile configuration, and the same advisory pair carries it.
    for header in ["  check:", "  lint:", "  test:"] {
        assert_eq!(
            normalized("_package-ci.yml", header),
            measurement,
            "_package-ci.yml: `{}` may hide the measurement steps and nothing else",
            header.trim()
        );
    }
    // `ci.yml`'s build owner hides nothing: a failed produce, a failed upload,
    // and a lost runner must all reach the fold and the dependent cells.
    assert_eq!(
        normalized("ci.yml", "  build:"),
        Vec::<String>::new(),
        "ci.yml: the build owner may not hide any step failure"
    );
    // The WSL leg ignores only its first provisioning attempt (so the bounded
    // retry is reachable) and diagnostics that must not overwrite a test result.
    assert_eq!(
        normalized("_wsl-ci.yml", "  wsl:"),
        vec![
            "Provision the WSL2 guest",
            "Census the Windows host before extraction",
            "Guest post-mortem",
            "Host post-mortem",
        ],
        "_wsl-ci.yml: `wsl` may ignore only the first provisioning attempt and \
         diagnostics — never tests, staging, or upload"
    );
    // The audit tool's own exit 2 is normalized so its enforcement step can
    // explain a coverage block; `package-ci` is a `uses:` job with no steps.
    assert!(normalized("_area-ci.yml", "  package-ci:").is_empty());
    assert_eq!(
        normalized("_area-ci.yml", "  coverage-audit:"),
        vec!["Roll up this area's cells"],
        "_area-ci.yml: `coverage-audit` may normalize only grid rendering, never enforcement"
    );
}

/// GitHub rejects `continue-on-error` on a `uses:` job, and even if it did not,
/// it would hide a called workflow's setup failures from the fold. The
/// normalization lives at the STEP level for that reason; no caller may try
/// to do it at the job level.
#[test]
fn no_reusable_workflow_call_is_advisory() {
    for file in READER_FACING_WORKFLOWS {
        for block in jobs(&workflow(file)) {
            let executable = executable_lines(&block);
            if !executable.contains("uses: ./.github/workflows/") {
                continue;
            }
            assert!(
                !executable.contains("\n    continue-on-error"),
                "{file}: `{}` calls a reusable workflow and must not be advisory",
                block.lines().next().unwrap_or("").trim()
            );
        }
    }
}

/// One real execution of the Lint step body.
#[cfg(unix)]
struct LintStepRun {
    exit_code: Option<i32>,
    /// The value the step published for `duration_s`, empty when it published
    /// the key with no measurement.
    duration_s: String,
    context: String,
}

/// The host's `bash`, resolved absolutely: one Lint-step case hands the script
/// a `PATH` stripped down to the fixture's stubs, and which shell interprets
/// the body must not depend on that.
#[cfg(unix)]
fn host_bash() -> PathBuf {
    let located = Command::new("sh")
        .arg("-c")
        .arg("command -v bash")
        .output()
        .expect("`sh` must be runnable");
    assert!(located.status.success(), "the host must provide `bash`");
    PathBuf::from(String::from_utf8_lossy(&located.stdout).trim())
}

/// Runs the real Lint step body with `just` stubbed to exit `just_exit`.
///
/// `python3_visible` chooses between a `PATH` carrying the host's interpreters
/// and one holding only the fixture's stubs, which reaches the
/// no-interpreter case without uninstalling anything.
#[cfg(unix)]
fn run_lint_step(script: &str, just_exit: i32, python3_visible: bool) -> LintStepRun {
    use std::os::unix::fs::PermissionsExt;

    let temp = tempfile::tempdir().expect("temp dir");
    let bin = temp.path().join("bin");
    fs::create_dir(&bin).expect("the stub directory is created");
    let stub = bin.join("just");
    fs::write(&stub, format!("#!/bin/sh\nexit {just_exit}\n")).expect("the `just` stub is written");
    fs::set_permissions(&stub, fs::Permissions::from_mode(0o755)).expect("the stub is executable");
    let github_output = temp.path().join("github-output");
    fs::write(&github_output, "").expect("the step output file exists");

    let path = if python3_visible {
        format!("{}:{}", bin.display(), std::env::var("PATH").unwrap_or_default())
    } else {
        bin.display().to_string()
    };
    let output = Command::new(host_bash())
        .arg("-c")
        .arg(script)
        .env_clear()
        .env("PATH", path)
        .env("GITHUB_OUTPUT", &github_output)
        .output()
        .expect("bash must be runnable");

    let written = fs::read_to_string(&github_output).expect("the step output is readable");
    let context = format!(
        "`just` stub exiting {just_exit}, python3 {}; wrote {written:?}\n{}\n{}",
        if python3_visible { "visible" } else { "hidden" },
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let duration_s = written
        .trim()
        .strip_prefix("duration_s=")
        .unwrap_or_else(|| panic!("the lint step must publish `duration_s`; {context}"))
        .to_owned();
    LintStepRun { exit_code: output.status.code(), duration_s, context }
}

/// The interpolated Lint step body, as the runner hands it to Bash.
#[cfg(unix)]
fn lint_step_script() -> String {
    let job = job_block("_package-ci.yml", "  lint:");
    let steps = steps(&job);
    let lint = step_with_id(&steps, "clippy")
        .expect("_package-ci.yml: `lint` must carry the `clippy` gate step");
    step_script(lint).replace("${{ matrix.package }}", "queue")
}

/// AC13: the lint step must MEASURE a command that finishes in under a second
/// rather than recording the `0` an absent measurement is reported as, and the
/// timing must never decide the gate.
///
/// Run for real, with `just` stubbed — the case Bash's integer `SECONDS` wrote
/// `duration_s=0` for, which `ci-rollup` could not tell apart from a step that
/// never ran. The host's own `bash` executes the step body, so the portability
/// traps are part of the assertion rather than a CI-only surprise: macOS ships
/// Bash 3.2 (no `EPOCHREALTIME`) and BSD `date` has no `%N`.
///
/// A failing stub runs too, because a step that owns the timing can swallow the
/// child's status: a failing lint must exit with the child's code AND still
/// publish its duration. The success case repeats because a clock read in two
/// processes is only *usually* ordered — review 3 of
/// `fixes/2026-09-13-cicd-redundancies` saw the two-interpreter form publish
/// `duration_s=-0.001`, so a single sample can pass by luck.
#[cfg(unix)]
#[test]
fn the_lint_step_measures_a_sub_second_command_instead_of_recording_zero() {
    let script = lint_step_script();

    for (just_exit, samples) in [(0, 8), (7, 1)] {
        for _ in 0..samples {
            let run = run_lint_step(&script, just_exit, true);
            assert_eq!(
                run.exit_code,
                Some(just_exit),
                "the lint step must exit with its gate's status, not the timing machinery's; {}",
                run.context
            );

            let recorded = &run.duration_s;
            let seconds: f64 = recorded.parse().unwrap_or_else(|error| {
                panic!("`duration_s={recorded}` must be a number: {error}; {}", run.context)
            });
            assert!(
                seconds.is_finite(),
                "the lint duration must be a finite measurement — NaN and infinity reach the \
                 artifact as values the report cannot rank; {}",
                run.context
            );
            assert!(
                seconds > 0.0,
                "a command that ran took a measurable, non-negative amount of time; two \
                 `time.monotonic` readings taken in separate interpreters subtract unrelated \
                 origins and can go backwards; {}",
                run.context
            );
            assert!(
                recorded.contains('.'),
                "the lint duration must be fractional — an integer clock reports a fast command \
                 as `0`, which the report renders as `not recorded`; {}",
                run.context
            );
            assert!(
                seconds < 60.0,
                "the step must time the COMMAND, not the epoch; {}",
                run.context
            );
        }
    }
}

/// A measurement the step cannot take is an unavailable MEASUREMENT, never a
/// gate verdict: with no `python3` on `PATH` the step still runs `just _lint`,
/// still exits with its status, and publishes an EMPTY `duration_s` rather than
/// the `0` the report renders identically to a step that never ran.
#[cfg(unix)]
#[test]
fn the_lint_step_publishes_an_empty_duration_when_it_cannot_time_the_command() {
    let script = lint_step_script();

    for just_exit in [0, 7] {
        let run = run_lint_step(&script, just_exit, false);
        assert_eq!(
            run.exit_code,
            Some(just_exit),
            "an unavailable clock must not change the gate's status; {}",
            run.context
        );
        assert_eq!(
            run.duration_s, "",
            "an unavailable measurement is spelled as an empty `duration_s`; {}",
            run.context
        );
    }
}

/// The fold, run for real: each producer's status script is executed under
/// the job/gate outcome combinations its `always()` step may observe, and the
/// artifact it writes is read back. Unix only because the
/// scripts declare `shell: bash` and the assertion is about Bash semantics,
/// not about the host; `test-toolkit`'s own suite runs on Linux too.
#[cfg(unix)]
#[test]
fn the_status_fold_preserves_every_failure_shape() {
    /// (file, job header, principal gate step id)
    const PRODUCERS: [(&str, &str, &str); 5] = [
        ("_package-ci.yml", "  check:", "check"),
        ("_package-ci.yml", "  check:", "dependents"),
        ("_package-ci.yml", "  test:", "gate"),
        ("_package-ci.yml", "  lint:", "clippy"),
        ("_wsl-ci.yml", "  wsl:", "l1"),
    ];
    /// (job.status, principal gate outcome, companion outcome, expected result)
    const CASES: [(&str, &str, &str, &str); 6] = [
        ("success", "success", "success", "success"),
        // Defensive: even if job.status is sampled as healthy during always(),
        // the gate outcome keeps the cell failed.
        ("success", "failure", "success", "failure"),
        // A companion suite is a gate command too.
        ("success", "success", "failure", "failure"),
        // Setup failed before the gate ran: the job's own failure is reported.
        ("failure", "skipped", "skipped", "failure"),
        // The gate passed and a later upload failed: still the job's failure.
        ("failure", "success", "success", "failure"),
        ("cancelled", "cancelled", "cancelled", "cancelled"),
    ];

    for (file, header, gate_id) in PRODUCERS {
        let job = job_block(file, header);
        let steps = steps(&job);
        let status = step_named(&steps, "Record producer status").expect("checked elsewhere");
        let env = step_env(status);
        let script = step_script(status);
        let has_companion = env.iter().any(|(key, _)| key == "COMPANION");

        for (job_status, gate, companion, expected) in CASES {
            if companion == "failure" && !has_companion {
                continue;
            }
            let temp = tempfile::tempdir().expect("temp dir");
            let mut command = Command::new("bash");
            command
                .arg("-c")
                .arg(&script)
                .env_clear()
                .env("PATH", std::env::var("PATH").unwrap_or_default())
                .env("RUNNER_TEMP", temp.path());
            for (key, value) in &env {
                let resolved = if value.contains("job.status") {
                    job_status
                } else if value.contains(&format!("steps.{gate_id}.outcome")) {
                    gate
                } else if value.contains("steps.companion.outcome") {
                    companion
                } else if value.contains(".outputs.duration_s") {
                    // A measured command duration, not a step outcome. The
                    // status folds it as a number, so "success" would make the
                    // script die where the real runner would not.
                    "12"
                } else if value.contains("outputs.dependents")
                    || value.contains("outputs.companion_suites")
                {
                    // Both are JSON documents from the cell reader; an empty
                    // list is the case the fold under test is exercised against.
                    "[]"
                } else if value.contains("outputs.build_key")
                    || value.contains("outputs.build_producer")
                {
                    // A cell that consumes no archive reports no build, which
                    // is the branch the fold takes when the key is empty.
                    ""
                } else if value.contains("matrix.gate") {
                    "L1"
                } else if value.contains("matrix.package") || value.contains("outputs.package") {
                    "pkg"
                } else if value.contains("matrix.") {
                    "ubuntu-latest"
                } else if !value.contains("${{") {
                    value.as_str()
                } else {
                    // Any other step outcome (a second provisioning attempt,
                    // JUnit staging, the Zed verification) is healthy here.
                    "success"
                };
                command.env(key, resolved);
            }
            let output = command.output().expect("bash must be runnable");
            let stdout = String::from_utf8_lossy(&output.stdout);
            let case = format!(
                "{file} `{}` with job.status={job_status}, gate={gate}, companion={companion}",
                header.trim()
            );
            assert!(
                output.status.success(),
                "{case}: the status script must exit 0 — it runs under always() and a \
                 failure here loses the cell's only evidence:\n{stdout}\n{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let written = fs::read_to_string(temp.path().join("status/status.json"))
                .unwrap_or_else(|error| panic!("{case}: no status.json written: {error}"));
            let json: serde_json::Value = serde_json::from_str(&written)
                .unwrap_or_else(|error| panic!("{case}: malformed status.json {written:?}: {error}"));
            assert_eq!(
                json["result"].as_str(),
                Some(expected),
                "{case}: the cell's result must be folded from the job status and the gate \
                 outcomes; got {written}"
            );
            let normalized = job_status == "success" && expected == "failure";
            assert_eq!(
                stdout.contains("::error title="),
                normalized,
                "{case}: only a failure not already represented by job.status needs a \
                 workflow-command annotation; stdout was:\n{stdout}"
            );
        }
    }
}

/// AC4/AC13/R14: the producer status carries ONE record per companion suite,
/// with that suite's own counts and command duration.
///
/// Run for real, like the fold above: the status script is executed with a
/// companions document beside it and the artifact it writes is read back. The
/// single `companion` string this replaced could not say which suite ran, how
/// many tests it had, or how long it took — so one suite's success satisfied
/// every suite the package declared.
#[cfg(unix)]
#[test]
fn the_producer_status_carries_one_record_per_companion_suite() {
    let companions = serde_json::json!({
        "homelab-frontend": {
            "outcome": "success",
            "duration_s": 0.96,
            "counts": {"total": 218, "passed": 189, "failed": 0, "skipped": 29, "errored": 0},
        },
        "test-audit-typecheck": {
            "outcome": "failure",
            "duration_s": 0.86,
            "reason": "typecheck gate reports no test counts",
        },
    });

    for header in ["  test:", "  lint:"] {
        let job = job_block("_package-ci.yml", header);
        let steps = steps(&job);
        let status = step_named(&steps, "Record producer status").expect("checked elsewhere");
        let temp = tempfile::tempdir().expect("temp dir");
        fs::write(
            temp.path().join("companions.json"),
            serde_json::to_string(&companions).expect("the fixture serializes"),
        )
        .expect("the companions document is written where the runner writes it");

        let mut command = Command::new("bash");
        command
            .arg("-c")
            .arg(step_script(status))
            .env_clear()
            .env("PATH", std::env::var("PATH").unwrap_or_default())
            .env("RUNNER_TEMP", temp.path());
        for (key, value) in step_env(status) {
            let resolved = if value.contains("runner.temp") {
                format!("{}/companions.json", temp.path().display())
            } else if value.contains("job.status") {
                "success".to_owned()
            } else if value.contains("steps.companion.outcome") {
                "failure".to_owned()
            } else if value.contains(".outputs.duration_s") {
                // Fractional, because the lint step measures a monotonic clock:
                // the fold must carry the measurement through unrounded.
                "0.42".to_owned()
            } else if value.contains("${{") {
                "success".to_owned()
            } else {
                value.clone()
            };
            command.env(key, resolved);
        }
        let output = command.output().expect("bash must be runnable");
        assert!(
            output.status.success(),
            "_package-ci.yml `{}`: the status script must exit 0:\n{}",
            header.trim(),
            String::from_utf8_lossy(&output.stderr)
        );

        let written = fs::read_to_string(temp.path().join("status/status.json"))
            .expect("the status artifact is written");
        let json: serde_json::Value =
            serde_json::from_str(&written).expect("the status artifact is JSON");
        let recorded = &json["companions"];
        assert_eq!(
            recorded["homelab-frontend"]["counts"]["total"],
            serde_json::json!(218),
            "each suite's own counts reach the artifact: {written}"
        );
        assert_eq!(
            recorded["homelab-frontend"]["duration_s"],
            serde_json::json!(0.96),
            "each suite's own command duration reaches the artifact: {written}"
        );
        assert_eq!(
            recorded["test-audit-typecheck"]["reason"],
            serde_json::json!("typecheck gate reports no test counts"),
            "an unmeasured suite carries its reason rather than a zero: {written}"
        );
        assert_eq!(json["result"], serde_json::json!("failure"));
        let detail = json["detail"].as_str().unwrap_or_default();
        assert!(
            detail.contains("test-audit-typecheck") && !detail.contains("homelab-frontend"),
            "the detail must name the suite that failed and not the one that \
             passed; got {detail:?}"
        );
        if header == "  lint:" {
            assert_eq!(
                json["duration_s"],
                serde_json::json!(0.42),
                "R14/AC13: the lint COMMAND's duration is recorded, because no JUnit \
                 report carries it — and a sub-second measurement survives the fold \
                 as a measurement rather than being rounded to the `0` that reads as \
                 an absence: {written}"
            );
        }
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
    // One native test job over all three tiers, so one upload — and it
    // publishes the whole staging directory, `manifest.jsonl` and this cell's
    // `expected-<gate>.json` included.
    assert_eq!(
        package_ci.matches("path: target/nextest/ci-reports").count(),
        1,
        "the native test upload must publish the whole staging directory"
    );
    assert!(
        package_ci.contains("--artifacts target/nextest/ci-reports"),
        "the completeness validator reads the same staging tree the upload publishes"
    );

    // The guest stages onto ext4, which the Windows host cannot read, so the
    // reports cross the 9p mount once. Copying the staging root's CONTENTS is
    // what puts manifest.jsonl at the artifact root.
    // The guest root is the job's own `GUEST_ROOT`, so two `wsl-bash`
    // invocations agree on it through one `env:` entry.
    let wsl = workflow("_wsl-ci.yml");
    assert!(
        wsl.contains(
            r#"export BISCUIT_JUNIT_STAGE_DIR="$guest_root/target/nextest/ci-reports""#
        ),
        "the WSL job must pin the guest staging root under the guest checkout"
    );
    assert!(
        wsl.contains(
            r#"cp -R '${{ env.GUEST_ROOT }}/target/nextest/ci-reports/.' \"#
        ),
        "the WSL job must copy the staging root's contents so manifest.jsonl lands at the artifact root"
    );
}

/// The jobs `ci-gate` folds: every top-level job of `ci.yml` whose failure
/// must block a merge. The advisory `ci-reporting` is the only job outside it.
const GATED_JOBS: [&str; 6] = [
    "validation",
    "scope",
    "preflight",
    // The native build owners. A failed owner leg is a real infrastructure
    // failure of the run, and folding it here is what keeps it from being
    // visible only through whichever consumer happened to notice the archive
    // was missing.
    "build",
    "area-ci",
    // AC15's planner-vs-sniff contract. It skips on every pull request that
    // cannot move area derivation, and the fold reads `skipped` as a pass.
    "area-drift",
];

/// Spec §5 / OQ3 Option B, proven in
/// `fixes/2026-09-11-cicd-cleanup/fixtures/scratch-2026-09-12.md`: the single
/// required check is a fixed-name job whose only step folds `needs.*.result`
/// and applies no policy of its own.
#[test]
fn ci_gate_is_the_single_required_check() {
    let ci = workflow("ci.yml");
    let gate = job_block("ci.yml", "  ci-gate:");
    let executable: String = gate
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        executable.contains("\n    if: always()\n"),
        "ci-gate must run even when every producer failed or was cancelled"
    );
    let needs = executable
        .split_once("    needs:\n")
        .expect("ci-gate must declare a needs list")
        .1
        .lines()
        .take_while(|line| line.starts_with("      - "))
        .map(|line| line.trim_start_matches("      - ").to_owned())
        .collect::<Vec<_>>();
    assert_eq!(
        needs, GATED_JOBS,
        "ci-gate must wait on exactly the blocking jobs, in this order"
    );
    for job in GATED_JOBS {
        assert!(
            executable.contains(&format!("{job}:${{{{ needs.{job}.result }}}}")),
            "ci-gate must fold `needs.{job}.result`"
        );
    }
    assert!(
        !executable.contains("continue-on-error"),
        "the gate itself must never be advisory"
    );
    assert!(
        !executable.contains("\n    permissions:"),
        "the fold needs nothing beyond the workflow's default token"
    );
    assert!(
        !executable.contains("uses: "),
        "the fold must check out nothing and download nothing"
    );
    // Policy-free: no rollup, no baseline, no plan, no scope. Any of these
    // would make it the global verdict specification section 5 removes.
    for policy in [
        "ci-rollup",
        "ci-baseline.toml",
        "resolved-plan",
        "download-artifact",
        "--scope",
        "cargo build",
    ] {
        assert!(
            !executable.contains(policy),
            "ci-gate must not evaluate policy: found `{policy}`"
        );
    }
    assert!(
        executable.contains("success|skipped)") && executable.contains("exit 1"),
        "the fold must accept exactly `success` and `skipped` and fail closed on anything else"
    );
    assert!(
        !ci.contains("baseline-failures.txt"),
        "the retired display-name baseline must have no consumers"
    );
}

/// A blocking job that `ci-gate` does not `need` is a job whose failure never
/// reaches the required check. Derived from the workflow, so a new top-level
/// job cannot be added without deciding whether it blocks.
#[test]
fn every_top_level_job_is_either_folded_by_ci_gate_or_the_advisory_report() {
    let ci = workflow("ci.yml");
    let mut headers = jobs(&ci)
        .iter()
        .map(|block| block.lines().next().unwrap_or("").trim().trim_end_matches(':').to_owned())
        .filter(|name| name != "ci-gate" && name != "ci-reporting")
        .collect::<Vec<_>>();
    headers.sort_unstable();
    let mut gated = GATED_JOBS.map(str::to_owned).to_vec();
    gated.sort_unstable();
    assert_eq!(headers, gated, "every other top-level job must be folded by ci-gate");
}

#[test]
fn post_merge_reuse_preserves_the_gate_and_normal_ci_fallback() {
    let validation = job_block("ci.yml", "  validation:");
    assert!(
        validation.contains("reuse_validation.py check")
            && validation.contains("steps.reuse.outcome == 'success'")
            && validation.contains("continue-on-error: true")
            && validation.contains("actions: read")
            && validation.contains("pull-requests: read"),
        "reuse must verify GitHub evidence and discard outputs from failed lookups"
    );
    let scope = job_block("ci.yml", "  scope:");
    assert!(
        scope.contains("needs: validation")
            && scope.contains("if: ${{ !cancelled() }}")
            && scope.contains("reuse_validation.py record")
            && scope.contains("name: ${{ steps.receipt.outputs.receipt }}")
            && scope.contains("if: github.event_name == 'pull_request'"),
        "scope must run on every event, including a reused validation, and publish \
         receipts for PR validation"
    );
    // A reused validation narrows the push's plan to the environments the
    // pull request event does not schedule, rather than skipping scope: the
    // proven environments are what the planner drops, and nothing else reads
    // the reuse flag to skip work (fixes/2026-09-18-ci-cadence).
    assert!(
        scope.contains("PROVEN_BY_PULL_REQUEST: ${{ needs.validation.outputs.reuse }}")
            && scope.contains("event_args+=(--proven-event pull_request)")
            && scope.contains("event_args=(--event \"$EVENT_NAME\")")
            && scope.contains("--all-environments"),
        "scope must plan by event, drop the environments a reused PR validation proved, \
         and honor the ci:all-os label"
    );
    // On reuse every downstream job is skipped, which the fold accepts, so
    // the gate passes; the reused-run link is reporting and lives in the
    // advisory report, which therefore has to run on reuse too.
    let gate = job_block("ci.yml", "  ci-gate:");
    assert!(
        gate.contains("      - validation\n") && gate.contains("if: always()"),
        "the required gate must fold the validation job and run on reuse"
    );
    let report = job_block("ci.yml", "  ci-reporting:");
    assert!(
        report.contains("      - validation\n")
            && report.contains("\n    if: always()\n")
            && report.contains("needs.validation.result == 'success'")
            && report.contains("actions/runs/$VALIDATED_RUN"),
        "the advisory report must run on reuse and link the reused evidence"
    );
    // Every other report step keys on the scope result alone: scope runs on
    // reuse too (for the environments the pull request event did not
    // schedule), so the reuse note is additive and the report is never
    // silenced by it (fixes/2026-09-18-ci-cadence).
    let steps = report.split_once("    steps:\n").unwrap().1;
    for step in steps.split("      - ").skip(2) {
        assert!(
            step.contains("needs.scope.result") && !step.contains("needs.validation.outputs.reuse"),
            "every other report step must key on the scope result, not on reuse: {step}"
        );
    }
    // The reuse decision is verified by its own suite, which `repo-deps` owns
    // and schedules as an ordinary cell. No workflow job runs it a second
    // time — that duplication is what this fix removes.
    let policy = read("scripts/ci/affected_scope.py");
    assert!(
        policy.contains(r#""test_reuse_validation.py","#),
        "the reuse decision suite must be registered to an owning package"
    );
    for job in ["  preflight:", "  ci-gate:"] {
        assert!(
            !job_block("ci.yml", job).contains("scripts/ci/test_"),
            "{job} must run no CI suite; its owner's cell does"
        );
    }
}

/// The policy/verdict words `the_report_makes_no_merge_or_policy_claim` in
/// `scripts/ci-rollup-tests.rs` bans from the Rust advisory renderer. That test
/// exercises the renderer only, so the reuse-mode shell in `ci.yml` — which
/// writes its own step summary and never reaches the renderer — is held to the
/// same list here.
const POLICY_VOCABULARY: [&str; 8] = [
    "BLOCKED",
    "CLEAR —",
    "must not merge",
    "may merge",
    "baseline-",
    "policy-gap-",
    "## CI verdict",
    "cicd",
];

#[test]
fn only_ci_gate_makes_a_run_level_claim() {
    let package_ci = workflow("_package-ci.yml");
    assert!(
        !package_ci.contains("  classify:") && !package_ci.contains("first actionable failure class"),
        "_package-ci.yml must not carry a second, environment-blind failure reporter"
    );

    let report = job_block("ci.yml", "  ci-reporting:");
    assert!(
        !report.contains("No gate reported a failure"),
        "the advisory report must make no run-level green claim"
    );
    // Emphasis, code spans, and the shell's backslash escaping are removed
    // first: `**CLEAR**` is the same verdict as `CLEAR` and must not evade a
    // literal match.
    let prose = report.replace(['*', '`', '\\'], "");
    let rollup_tests = read("scripts/ci-rollup-tests.rs");
    for policy_word in POLICY_VOCABULARY {
        assert!(
            rollup_tests.contains(&format!("{policy_word:?},")),
            "scripts/ci-rollup-tests.rs no longer bans {policy_word:?}; the two \
             vocabularies must stay identical"
        );
        assert!(
            !prose.contains(policy_word),
            "the ci-reporting job applied policy ({policy_word:?}): {report}"
        );
    }
    assert!(
        !report.contains("needs.area-ci.result"),
        "every package gate is covered by its own area's workflow; the advisory \
         report must not report them a second time"
    );
    assert!(
        report.contains("ci-gate") && !report.contains("ci-verdict"),
        "the advisory report must name ci-gate as the run's gate"
    );
}

// --- `ci-reporting`'s three modes: exclusive, exhaustive, and executed -------

/// One token of the GitHub-expression subset the `ci-reporting` guards use.
#[derive(Clone, Debug, PartialEq, Eq)]
enum GuardToken {
    Not,
    And,
    Or,
    Open,
    Close,
    Eq,
    Ne,
    Path(String),
    Literal(String),
}

fn tokenize_guard(expression: &str) -> Vec<GuardToken> {
    let chars: Vec<char> = expression.chars().collect();
    let mut tokens = Vec::new();
    let mut at = 0;
    while at < chars.len() {
        match chars[at] {
            ' ' | '\t' | '\n' => at += 1,
            '(' => {
                tokens.push(GuardToken::Open);
                at += 1;
            }
            ')' => {
                tokens.push(GuardToken::Close);
                at += 1;
            }
            '\'' => {
                at += 1;
                let start = at;
                while at < chars.len() && chars[at] != '\'' {
                    at += 1;
                }
                assert!(at < chars.len(), "unterminated literal in {expression:?}");
                tokens.push(GuardToken::Literal(chars[start..at].iter().collect()));
                at += 1;
            }
            '&' | '|' => {
                let pair = chars[at];
                assert_eq!(
                    chars.get(at + 1),
                    Some(&pair),
                    "a single {pair:?} is not a GitHub operator: {expression:?}"
                );
                tokens.push(if pair == '&' {
                    GuardToken::And
                } else {
                    GuardToken::Or
                });
                at += 2;
            }
            '=' => {
                assert_eq!(chars.get(at + 1), Some(&'='), "expected `==` in {expression:?}");
                tokens.push(GuardToken::Eq);
                at += 2;
            }
            '!' => {
                if chars.get(at + 1) == Some(&'=') {
                    tokens.push(GuardToken::Ne);
                    at += 2;
                } else {
                    tokens.push(GuardToken::Not);
                    at += 1;
                }
            }
            other => {
                let start = at;
                while at < chars.len()
                    && (chars[at].is_ascii_alphanumeric() || matches!(chars[at], '.' | '_' | '-'))
                {
                    at += 1;
                }
                assert!(
                    at > start,
                    "unsupported character {other:?} in guard {expression:?}"
                );
                tokens.push(GuardToken::Path(chars[start..at].iter().collect()));
            }
        }
    }
    tokens
}

struct GuardParser<'a> {
    tokens: &'a [GuardToken],
    at: usize,
    context: &'a BTreeMap<String, String>,
    source: &'a str,
}

impl GuardParser<'_> {
    fn peek(&self) -> Option<&GuardToken> {
        self.tokens.get(self.at)
    }

    fn any(&mut self) -> bool {
        let mut value = self.all();
        while self.peek() == Some(&GuardToken::Or) {
            self.at += 1;
            value = self.all() || value;
        }
        value
    }

    fn all(&mut self) -> bool {
        let mut value = self.unary();
        while self.peek() == Some(&GuardToken::And) {
            self.at += 1;
            value = self.unary() && value;
        }
        value
    }

    fn unary(&mut self) -> bool {
        if self.peek() == Some(&GuardToken::Not) {
            self.at += 1;
            return !self.unary();
        }
        self.comparison()
    }

    fn comparison(&mut self) -> bool {
        if self.peek() == Some(&GuardToken::Open) {
            self.at += 1;
            let value = self.any();
            assert_eq!(
                self.peek(),
                Some(&GuardToken::Close),
                "unbalanced parentheses in {}",
                self.source
            );
            self.at += 1;
            return value;
        }
        let Some(GuardToken::Path(path)) = self.peek().cloned() else {
            panic!("expected a `needs.…` path in {}", self.source);
        };
        self.at += 1;
        let operator = self.peek().cloned();
        self.at += 1;
        let Some(GuardToken::Literal(literal)) = self.peek().cloned() else {
            panic!("expected a quoted literal after `{path}` in {}", self.source);
        };
        self.at += 1;
        // A guard that reads state this test does not model is drift, not a
        // default: fail here rather than silently evaluate it as absent.
        let actual = self.context.get(&path).unwrap_or_else(|| {
            panic!(
                "the guard reads `{path}`, which this contract's context does not model: {}",
                self.source
            )
        });
        match operator {
            Some(GuardToken::Eq) => *actual == literal,
            Some(GuardToken::Ne) => *actual != literal,
            other => panic!("unsupported operator {other:?} after `{path}` in {}", self.source),
        }
    }
}

fn eval_guard(guard: &str, context: &BTreeMap<String, String>) -> bool {
    let body = guard
        .trim()
        .strip_prefix("${{")
        .and_then(|rest| rest.strip_suffix("}}"))
        .unwrap_or_else(|| {
            panic!("a ci-reporting guard must be one `${{{{ … }}}}` expression: {guard:?}")
        });
    let tokens = tokenize_guard(body);
    let mut parser = GuardParser {
        tokens: &tokens,
        at: 0,
        context,
        source: guard,
    };
    let value = parser.any();
    assert_eq!(parser.at, tokens.len(), "trailing tokens in guard {guard:?}");
    value
}

fn step_guard(step: &str) -> Option<String> {
    step.lines()
        .find_map(|line| line.strip_prefix("        if: "))
        .map(|guard| guard.trim().to_owned())
}

/// The three mode guards as `ci.yml` writes them, in step order, together with
/// the step names assigned to each. Read from the workflow so a guard that
/// drifts cannot be satisfied by a copy kept here.
fn reporting_mode_guards() -> Vec<String> {
    let job = job_block("ci.yml", "  ci-reporting:");
    let job_steps = steps(&job);
    assert!(
        job_steps.len() >= 3,
        "ci-reporting must carry all three modes"
    );

    let mut guards: Vec<String> = Vec::new();
    let mut mode_of_step: Vec<(String, usize)> = Vec::new();
    for step in &job_steps {
        let guard = step_guard(step).unwrap_or_else(|| {
            panic!("every ci-reporting step must name the mode it belongs to:\n{step}")
        });
        let mode = match guards.iter().position(|known| known == &guard) {
            Some(index) => index,
            None => {
                guards.push(guard);
                guards.len() - 1
            }
        };
        mode_of_step.push((step_name(step), mode));
    }

    assert_eq!(
        guards.len(),
        3,
        "ci-reporting has exactly three modes, so exactly three distinct guards: {guards:#?}"
    );
    assert_eq!(
        mode_of_step.first().map(|(name, mode)| (name.as_str(), *mode)),
        Some(("Reuse successful PR validation", 0)),
        "mode 1 is the first step"
    );
    assert_eq!(
        mode_of_step.last().map(|(name, mode)| (name.as_str(), *mode)),
        Some(("Classify the first actionable failure", 2)),
        "mode 3 is the last step"
    );
    for (name, mode) in &mode_of_step[1..mode_of_step.len() - 1] {
        assert_eq!(
            *mode, 1,
            "`{name}` sits between modes 1 and 3 and must carry mode 2's guard"
        );
    }
    guards
}

fn bootstrap_context(validation: &str, reuse: &str, scope: &str) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("needs.validation.result".to_owned(), validation.to_owned()),
        (
            "needs.validation.outputs.reuse".to_owned(),
            reuse.to_owned(),
        ),
        ("needs.scope.result".to_owned(), scope.to_owned()),
    ])
}

/// The 1-based modes whose guard is true for a given bootstrap state.
fn speaking_modes(guards: &[String], context: &BTreeMap<String, String>) -> Vec<usize> {
    guards
        .iter()
        .enumerate()
        .filter(|(_, guard)| eval_guard(guard, context))
        .map(|(index, _)| index + 1)
        .collect()
}

/// Specification section 6 gives `ci-reporting` one voice per run. Since
/// fixes/2026-09-18-ci-cadence, `scope` runs on a reused PR validation too —
/// for the environments the pull request event did not schedule — so the
/// report has two exhaustive, exclusive modes keyed on the scope result (the
/// full report, or the first actionable failure) plus one additional note
/// that speaks exactly when a validation was reused. Overlapping report
/// guards would render two reports for the same run, and a gap renders none.
#[test]
fn exactly_one_ci_reporting_mode_speaks_for_every_bootstrap_state() {
    let guards = reporting_mode_guards();

    /// (validation.result, validation.outputs.reuse, scope.result, modes)
    const NAMED: [(&str, &str, &str, &[usize]); 7] = [
        // A reused PR validation: the note, plus the report of what scope
        // planned beyond it.
        ("success", "true", "success", &[1, 2]),
        // The ordinary scoped run: the full report, and only the full report.
        ("success", "false", "success", &[2]),
        ("failure", "false", "skipped", &[3]),
        ("cancelled", "false", "skipped", &[3]),
        ("success", "false", "failure", &[3]),
        ("success", "false", "cancelled", &[3]),
        // Reuse recorded by a validation job that did not itself succeed: mode
        // 1 declines it, so mode 3 has to take it or the run says nothing.
        ("failure", "true", "skipped", &[3]),
    ];

    for (validation, reuse, scope, expected) in NAMED {
        let speaking = speaking_modes(&guards, &bootstrap_context(validation, reuse, scope));
        assert_eq!(
            speaking,
            expected.to_vec(),
            "validation={validation} reuse={reuse} scope={scope}: modes {expected:?} must \
             report this run, but {speaking:?} did"
        );
    }

    for validation in JOB_RESULTS {
        for reuse in ["true", "false"] {
            for scope in JOB_RESULTS {
                let speaking =
                    speaking_modes(&guards, &bootstrap_context(validation, reuse, scope));
                let reports: Vec<usize> =
                    speaking.iter().copied().filter(|mode| *mode != 1).collect();
                assert_eq!(
                    reports.len(),
                    1,
                    "validation={validation} reuse={reuse} scope={scope}: the report modes \
                     must be exclusive and exhaustive, but {speaking:?} spoke"
                );
                assert_eq!(
                    speaking.contains(&1),
                    validation == "success" && reuse == "true",
                    "validation={validation} reuse={reuse} scope={scope}: the reuse note \
                     speaks exactly for a successful, reused validation"
                );
            }
        }
    }
}

const JOB_RESULTS: [&str; 4] = ["success", "failure", "cancelled", "skipped"];

/// Mode 3's `RESULTS` document with the `needs.*` expressions resolved, exactly
/// as the runner would hand it to the script.
#[cfg(unix)]
fn bootstrap_results_document(step: &str, validation: &str, scope: &str) -> String {
    let (_, rest) = step
        .split_once("\n          RESULTS: |\n")
        .expect("mode 3 must list the bootstrap stages it can see as a `RESULTS` block");
    let document = rest
        .lines()
        .take_while(|line| line.starts_with("            "))
        .map(|line| {
            line[12..]
                .replace("${{ needs.validation.result }}", validation)
                .replace("${{ needs.scope.result }}", scope)
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !document.contains("${{"),
        "mode 3 reads a stage this contract does not resolve: {document}"
    );
    document
}

/// Mode 3 promises "a failed or cancelled bootstrap". Run for real — a source
/// scan can see that both labels exist while the loop still matches only
/// `failure`, which is how a cancelled validation came to be reported as
/// no bootstrap failure at all.
///
/// Unix only because the step declares `shell: bash` and the assertion is about
/// what that script renders; `test-toolkit`'s own suite runs on Linux too.
#[cfg(unix)]
#[test]
fn the_bootstrap_report_names_cancellation_instead_of_denying_a_failure() {
    let job = job_block("ci.yml", "  ci-reporting:");
    let job_steps = steps(&job);
    let classify = step_named(&job_steps, "Classify the first actionable failure")
        .expect("ci-reporting must carry the failed-or-cancelled bootstrap mode");
    let script = step_script(classify);

    /// (validation.result, scope.result, must appear, must not appear)
    const CASES: [(&str, &str, &str, &str); 4] = [
        (
            "cancelled",
            "skipped",
            "First actionable failure class: bootstrap (reuse check) (cancelled)",
            "No bootstrap stage failed",
        ),
        (
            "failure",
            "skipped",
            "First actionable failure class: bootstrap (reuse check) (failed)",
            "No bootstrap stage failed",
        ),
        (
            "success",
            "cancelled",
            "First actionable failure class: bootstrap (scope calculation) (cancelled)",
            "No bootstrap stage failed",
        ),
        // Nothing failed and nothing was cancelled, yet mode 3 is speaking, so
        // scope did not succeed either. The report says that rather than
        // inventing a failure.
        (
            "success",
            "skipped",
            "No bootstrap stage failed or was cancelled",
            "First actionable failure class",
        ),
    ];

    for (validation, scope, expected, forbidden) in CASES {
        let temp = tempfile::tempdir().expect("temp dir");
        let summary = temp.path().join("step-summary.md");
        fs::write(&summary, "").expect("the runner pre-creates the step summary");

        let output = Command::new("bash")
            .arg("-c")
            .arg(&script)
            .env_clear()
            .env("PATH", std::env::var("PATH").unwrap_or_default())
            .env("GITHUB_STEP_SUMMARY", &summary)
            .env(
                "RESULTS",
                bootstrap_results_document(classify, validation, scope),
            )
            .output()
            .expect("bash must be runnable");
        let case = format!("validation={validation}, scope={scope}");
        assert!(
            output.status.success(),
            "{case}: the report script must exit 0 — it is the run's only account of a \
             bootstrap that produced no result artifacts:\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        let rendered = fs::read_to_string(&summary).expect("the script writes the step summary");
        assert!(
            rendered.contains(expected),
            "{case}: the report must state {expected:?}, but rendered:\n{rendered}"
        );
        assert!(
            !rendered.contains(forbidden),
            "{case}: the report must not state {forbidden:?}, but rendered:\n{rendered}"
        );
        // The observed state of every stage is rendered, so the reader can tell
        // a cancelled stage from a skipped one without opening the run.
        assert!(
            rendered.contains(&format!("- bootstrap (reuse check): {validation}"))
                && rendered.contains(&format!("- bootstrap (scope calculation): {scope}")),
            "{case}: the report must name each stage's observed state, but rendered:\n{rendered}"
        );
    }
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
                // A nested crate's tests belong to that crate, not to the
                // package being measured. `scripts/ci/fixtures/` holds one:
                // a scratch package the archive-portability suite COMPILES,
                // whose `level2_marker` and `browser_marker` targets exist to
                // prove a filterset reaches them from an archive. Counting
                // them here would make `repo-deps` declare tiers it does not
                // own — and it only started to matter when `scripts/` became a
                // root workspace member and so a gating package.
                let nested_crate = path.join("Cargo.toml").is_file();
                if !nested_crate
                    && path.file_name().and_then(|name| name.to_str()) != Some("target")
                {
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
/// schedules none. The fan-out gate must derive from the list the area matrix
/// expands, or that change fails the area-ci job at strategy evaluation.
#[test]
fn the_fan_out_gate_derives_from_the_scheduled_areas_not_the_impacted_list() {
    let ci = workflow("ci.yml");
    assert!(
        ci.contains("has_packages=$(jq -r '.scheduled_areas | length > 0'"),
        "has_packages must be computed from the scheduled areas; the impacted \
         package list includes non-gating packages"
    );
    assert!(
        ci.contains("area: ${{ fromJSON(needs.scope.outputs.scheduled_areas) }}"),
        "the area matrix must expand the same list has_packages guards"
    );
    assert!(
        !ci.contains("has_packages=$(jq -r '.packages | length > 0'"),
        "deriving has_packages from the impacted list crashes a `gates = false`-only change"
    );
}

/// CI's own tooling is verified by two ordinary workspace members, so an input
/// one of their suites reads must select that OWNER — not a boolean flag whose
/// only consumer was a job outside the scheduling model.
#[test]
fn tooling_inputs_select_their_registered_suite_owner() {
    let policy = read("scripts/ci/affected_scope.py");
    let declared = policy
        .split_once("SUITE_OWNER_PREFIXES: tuple[tuple[str, tuple[str, ...]], ...] = (")
        .and_then(|(_, rest)| rest.split_once("\n)"))
        .map(|(block, _)| block)
        .expect("affected_scope.py must declare SUITE_OWNER_PREFIXES");
    for (prefix, owner) in [
        (".github/ci/", "repo-deps"),
        (".github/workflows/", "test-toolkit"),
        ("tools/test-audit/", "test-toolkit"),
    ] {
        assert!(
            declared.contains(&format!(r#"("{prefix}", ("{owner}",))"#)),
            "affected_scope.py must select {owner} for a change under {prefix}"
        );
    }
    // R13.4: `scripts/` is `repo-deps`'s own package directory, so ordinary
    // source ownership already selects it. A trigger here would double-select.
    assert!(
        !declared.contains(r#""scripts/""#),
        "`scripts/` must not be a trigger prefix; it is repo-deps's package directory"
    );

    for retired in ["CI_TOOLING_PREFIXES", "CI_TOOLING_PATHS"] {
        assert!(
            !policy.contains(retired),
            "{retired} is replaced by the suite-owner table; a flag nothing \
             schedules on cannot select a package"
        );
    }
    let ci = workflow("ci.yml");
    assert!(
        !ci.contains(".flags.ci_tooling"),
        "the scope job must emit no ci_tooling flag; ownership replaces it"
    );
}

/// Spec section 3: every registered suite has exactly one owner. This suite is
/// the evidence behind AC10, AC11, and AC14–AC17; `test-toolkit` owns it and
/// therefore has to gate.
#[test]
fn the_workflow_contract_suite_is_owned_by_test_toolkit() {
    let policy = read("scripts/ci/affected_scope.py");
    let registry = policy
        .split_once("SUITE_REGISTRY: dict[str, dict[str, Any]] = {")
        .and_then(|(_, rest)| rest.split_once("\n}"))
        .map(|(block, _)| block)
        .expect("affected_scope.py must declare SUITE_REGISTRY");
    assert!(
        registry.contains(r#""test-toolkit-l1": {"#) && registry.contains(r#""test-toolkit","#),
        "the registry must name this package's Cargo suite and its owner"
    );

    let manifest = read("tools/test-toolkit/Cargo.toml");
    assert!(
        !manifest.contains("gates = false"),
        "test-toolkit owns a registered suite, so it cannot opt out of gating"
    );
}

/// The archive-path guard's eligibility policy now exists twice — in
/// [`test_toolkit::archive_guard`] and in the planner, which must select the
/// guard from the same files the scanner will read. A path the planner selects
/// and the scanner then refuses is a cell that ran nothing, and a directory one
/// side skips and the other does not is a corpus nobody checks.
///
/// The two are compared as source text, the way
/// `tooling_inputs_select_their_registered_suite_owner` compares the ownership
/// table: this crate cannot import Python, and a test that re-stated either list
/// would be a third copy to keep in step.
#[test]
fn the_guards_eligibility_policy_agrees_across_the_language_boundary() {
    let policy = read("scripts/ci/affected_scope.py");

    let skipped = policy
        .split_once("ARCHIVE_GUARD_SKIPPED_DIRS = frozenset({")
        .and_then(|(_, rest)| rest.split_once("})"))
        .map(|(block, _)| block)
        .expect("affected_scope.py must declare ARCHIVE_GUARD_SKIPPED_DIRS");
    for (directory, _) in test_toolkit::archive_guard::SKIPPED_DIRS {
        assert!(
            skipped.contains(&format!("\"{directory}\"")),
            "the scanner skips `{directory}` and the planner does not, so a change \
             under it would select a scan that then refuses every file it was given; \
             add it to ARCHIVE_GUARD_SKIPPED_DIRS in scripts/ci/affected_scope.py"
        );
    }
    let declared = skipped.matches('"').count() / 2;
    assert_eq!(
        declared,
        test_toolkit::archive_guard::SKIPPED_DIRS.len(),
        "the planner skips {declared} director(ies) and the scanner skips {}; a \
         directory only the planner skips is source nothing ever scans",
        test_toolkit::archive_guard::SKIPPED_DIRS.len()
    );

    let owned = policy
        .split_once("ARCHIVE_GUARD_OWN_INPUTS = frozenset({")
        .and_then(|(_, rest)| rest.split_once("})"))
        .map(|(block, _)| block)
        .expect("affected_scope.py must declare ARCHIVE_GUARD_OWN_INPUTS");
    for (source, _) in test_toolkit::archive_guard::GUARD_OWN_SOURCES {
        assert!(
            owned.contains(&format!("\"{source}\"")),
            "`{source}` is excluded from the scan as one of the guard's own \
             sources, so editing it would select the guard nowhere; add it to \
             ARCHIVE_GUARD_OWN_INPUTS in scripts/ci/affected_scope.py"
        );
    }
    // The recipe, the selection rule, the registry binding, the workflow
    // holding the guard's execution controls, and the cell contract that
    // carries `companions_only` are owned inputs the scanner has no opinion
    // about, so they are named here rather than derived.
    for extra in [
        "tools/test-toolkit/justfile",
        "scripts/ci/affected_scope.py",
        "tools/test-toolkit/Cargo.toml",
        ".github/workflows/_package-ci.yml",
        "scripts/ci/cell_contract.py",
    ] {
        assert!(
            owned.contains(&format!("\"{extra}\"")),
            "changing `{extra}` changes what the guard runs or what selects it, \
             and must therefore select it"
        );
    }

    // Every entry names a file that is still there. An owned input is a path
    // literal on the Python side and nothing on that side resolves it, so a
    // rename turns the entry into a rule that matches no change — the guard
    // stops being selected by the very input it is supposed to watch, and no
    // gate goes red. The count is compared too, so an entry added to the
    // frozenset and not to this test's reasoning cannot pass unexamined.
    let root = repo_root();
    let entries: Vec<&str> = owned
        .split('"')
        .skip(1)
        .step_by(2)
        .filter(|entry| !entry.trim().is_empty())
        .collect();
    assert_eq!(
        8,
        entries.len(),
        "ARCHIVE_GUARD_OWN_INPUTS declares {} entr(ies); update this contract \
         and `.github/ci/README.md` alongside the policy: {entries:?}",
        entries.len()
    );
    for entry in entries {
        assert!(
            root.join(entry).exists(),
            "ARCHIVE_GUARD_OWN_INPUTS names `{entry}`, which no longer exists; a \
             renamed owned input is a selection rule that matches nothing and \
             silently stops scheduling the guard"
        );
    }
}

/// `python3` where it exists, `python` where only that name is installed, and
/// `None` on a host with neither.
///
/// Windows ships an App Execution Alias named `python3` that exits non-zero
/// into the Store, so only a successful `--version` counts as an interpreter.
fn python_interpreter() -> Option<&'static str> {
    ["python3", "python"].into_iter().find(|candidate| {
        Command::new(candidate)
            .arg("--version")
            .output()
            .is_ok_and(|probe| probe.status.success())
    })
}

/// End to end across the boundary the two halves of the deletion contract meet
/// at: the shipped planner writes the scope, and the guard's own reader
/// consumes it.
///
/// Each half passes its own fixtures. Only this one can fail when the planner
/// starts listing a deleted path again, or when the reader starts tolerating a
/// listed path that is not there — the fail-open pair review 3 found. Skipped
/// where no Python interpreter exists, the way
/// `scripts/ci-rollup-tests.rs::the_real_planners_plan_rolls_up` is: the Rust
/// suite must still run on a host without one.
#[test]
fn the_shipped_planner_omits_deletions_and_the_reader_refuses_an_unexpected_absence() {
    use test_toolkit::archive_guard::{GuardError, GuardPlan, ScanMode, scan};

    /// Eligible, present, and owned by a package the planner schedules.
    const MODIFIED: &str = "claudine/lib/src/lib.rs";
    /// Eligible and gone: the diff recorded it as removed.
    const DELETED: &str = "claudine/lib/src/removed_by_this_change.rs";

    let Some(python) = python_interpreter() else {
        eprintln!("no Python interpreter is available; skipping the planner-to-reader fixture");
        return;
    };
    let root = repo_root();
    let output = Command::new(python)
        .current_dir(&root)
        .args([
            "scripts/ci/affected_scope.py",
            "--resolved-plan",
            "--deleted",
            DELETED,
            "--",
            MODIFIED,
            DELETED,
        ])
        .output()
        .expect("the probed interpreter must still be runnable");
    assert!(
        output.status.success(),
        "the planner must resolve a plan: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let resolved = String::from_utf8(output.stdout).expect("the resolved plan is UTF-8");

    let plan = GuardPlan::from_plan_json(Path::new("resolved-plan.json"), &resolved)
        .expect("the guard must read the planner's own document");
    assert!(plan.selected, "an eligible source change selects the guard");
    let ScanMode::Changed(listed) = &plan.mode else {
        panic!(
            "a change inventory resolves a changed scan, got {:?}",
            plan.mode
        );
    };
    assert!(
        listed.iter().any(|path| path == MODIFIED),
        "the modified file is what the scan exists to read: {listed:?}"
    );
    assert!(
        !listed.iter().any(|path| path == DELETED),
        "a path the diff reported as deleted must never be listed for scanning: {listed:?}"
    );

    // An empty tree stands in for the checkout losing a listed path between
    // planning and the scan — the one way a listed path can now be absent.
    let tree = tempfile::TempDir::new().expect("temp tree");
    let err = scan(tree.path(), &plan.mode)
        .expect_err("a listed path that is not there must not be skipped");

    assert!(matches!(err, GuardError::MissingListedPath { .. }), "{err}");
    assert!(err.to_string().contains(MODIFIED), "{err}");
}

/// The guard reader's closed field set against the frozen cross-language
/// contract `scripts/ci/schema.py::ARCHIVE_GUARD_FIELDS` is dumped into.
///
/// Both sides refuse a field outside the set, so a field added to one alone
/// would have the planner emit a scope its own consumer rejects — or, worse,
/// have the consumer ignore a renamed scope field and pass under semantics the
/// producer never asked for.
#[test]
fn the_guard_scope_field_set_matches_the_frozen_contract() {
    let text = read(".github/ci/schemas/contract.json");
    let contract: serde_json::Value =
        serde_json::from_str(&text).expect("the frozen contract parses");

    let mut shipped: Vec<&str> = contract["resolved_plan"]["archive_guard"]
        .as_object()
        .expect("the frozen contract describes the guard scope's fields")
        .keys()
        .map(String::as_str)
        .collect();
    shipped.sort_unstable();

    assert_eq!(
        shipped,
        test_toolkit::archive_guard::PLAN_SCOPE_FIELDS,
        "the guard's reader and the Python validator no longer close the \
         `archive_guard` field set over the same names"
    );
}

/// The shared accept/reject corpus, read by the guard's own plan reader.
///
/// `.github/ci/schemas/archive_guard_cases.json` is the one table
/// `scripts/ci/test_schema.py::ArchiveGuardSharedCorpusTests` reads too, so a
/// shape added there fails both suites until both readers agree on it. This
/// half fixes only the verdict; the wording each side produces stays in that
/// side's own tests.
#[test]
fn the_guards_reader_agrees_with_the_shared_scope_corpus() {
    use test_toolkit::archive_guard::{GuardError, GuardPlan, PLAN_SCHEMA_VERSION};

    let text = read(".github/ci/schemas/archive_guard_cases.json");
    let corpus: serde_json::Value =
        serde_json::from_str(&text).expect("the shared guard corpus parses");
    let cases = corpus["cases"]
        .as_array()
        .expect("the corpus is a list of cases");
    assert!(!cases.is_empty(), "an empty table synchronizes nothing");

    for case in cases {
        let name = case["name"].as_str().expect("every case is named");
        let rule = case["rule"].as_str().expect("every case states its rule");
        let valid = case["valid"].as_bool().expect("every case states its verdict");
        let document = serde_json::json!({
            "schema_version": PLAN_SCHEMA_VERSION,
            "archive_guard": case["archive_guard"],
        });

        let outcome =
            GuardPlan::from_plan_json(Path::new("resolved-plan.json"), &document.to_string());

        match (valid, outcome) {
            (true, Ok(_)) => {}
            (false, Err(GuardError::MalformedPlan { .. })) => {}
            (true, Err(error)) => panic!("`{name}` must be accepted — {rule}: {error}"),
            (false, Ok(plan)) => {
                panic!("`{name}` must be refused — {rule}; the reader accepted {plan:?}")
            }
            (false, Err(error)) => {
                panic!("`{name}` must be refused as a malformed plan — {rule}: {error}")
            }
        }
    }
}

/// The guard's plan-schema constant against the frozen cross-language contract
/// the Python validator is written to.
///
/// A passive corpus test over the shipped document, and the mechanism that
/// keeps the two languages in step: the Rust reader now refuses a plan from any
/// other generation, so bumping the version on one side alone would make the
/// planner emit a document its own consumer rejects. Iteration 1 of
/// `2026-09-19-less-brittle` stranded the pre-push fixtures exactly that way.
#[test]
fn the_plan_schema_version_matches_the_frozen_contract() {
    let text = read(".github/ci/schemas/contract.json");
    let contract: serde_json::Value =
        serde_json::from_str(&text).expect("the frozen contract parses");

    assert_eq!(
        contract["resolved_plan"]["schema_version"].as_u64(),
        Some(test_toolkit::archive_guard::PLAN_SCHEMA_VERSION),
        "the archive-path guard reads a plan generation the contract no longer describes"
    );
}

/// Every `archive_guard` shape the shipped planner emits, read back by the
/// guard's own contract-enforcing reader.
///
/// The reader validates the whole resolved-plan contract, so it can now reject
/// a document the planner considers valid. Only a fixture that feeds it the
/// real emitter's output catches that divergence; hand-written fixtures agree
/// with whichever side wrote them. Skipped where no Python interpreter exists,
/// as its sibling above is.
#[test]
fn the_shipped_planner_emits_plans_the_guards_reader_accepts() {
    use test_toolkit::archive_guard::{GuardPlan, ScanMode};

    let Some(python) = python_interpreter() else {
        eprintln!("no Python interpreter is available; skipping the shipped-plan fixture");
        return;
    };
    let root = repo_root();

    let resolve = |arguments: &[&str]| -> String {
        let output = Command::new(python)
            .current_dir(&root)
            .args(["scripts/ci/affected_scope.py", "--resolved-plan"])
            .args(arguments)
            .output()
            .expect("the probed interpreter must still be runnable");
        assert!(
            output.status.success(),
            "the planner must resolve a plan for {arguments:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).expect("the resolved plan is UTF-8")
    };

    let read_back = |arguments: &[&str]| -> GuardPlan {
        GuardPlan::from_plan_json(Path::new("resolved-plan.json"), &resolve(arguments))
            .unwrap_or_else(|err| panic!("the guard must read the planner's own document: {err}"))
    };

    let documentation = read_back(&["--", "README.md"]);
    assert!(
        !documentation.selected,
        "a documentation-only change selects no scan"
    );
    assert!(
        !documentation.reason.trim().is_empty(),
        "an unselected plan still says why"
    );

    let full = read_back(&["--all"]);
    assert!(full.selected, "an explicit full-scope request selects the guard");
    assert_eq!(full.mode, ScanMode::FullTree);

    let changed = read_back(&["--", "tools/test-toolkit/src/archive_guard.rs"]);
    let ScanMode::Changed(listed) = &changed.mode else {
        panic!("a change inventory resolves a changed scan, got {:?}", changed.mode);
    };
    assert!(
        listed.windows(2).all(|pair| pair[0] < pair[1]),
        "the planner's own list must satisfy the sorted, unique rule the reader enforces: {listed:?}"
    );
}

/// The `archive-path-guard` block of `SUITE_REGISTRY`, as source text.
fn archive_guard_registry_entry() -> String {
    let policy = read("scripts/ci/affected_scope.py");
    policy
        .split_once("\n    \"archive-path-guard\": {")
        .and_then(|(_, rest)| rest.split_once("\n    },"))
        .map(|(block, _)| block.to_owned())
        .expect("affected_scope.py must register the archive-path-guard suite")
}

/// `2026-09-19-less-brittle`: a failing guard blocks a merge through the
/// existing fold and adds nothing to the graph.
///
/// The guard is a companion of an ordinary `lint` cell of an ordinary package,
/// so its failure travels `lint` -> `_package-ci.yml` -> `_area-ci.yml`'s
/// `package-ci` -> `ci.yml`'s `area-ci` -> `ci-gate`, which folds
/// `needs.*.result` and applies no policy. The only way that chain breaks is a
/// seventh top-level job, and [`TARGET_CI_JOBS`] and [`GATED_JOBS`] are exactly
/// what catches one — hence the length assertions the specification's "keep one
/// scheduler" decision requires.
#[test]
fn a_failing_archive_path_guard_blocks_the_merge_through_the_existing_fold() {
    assert_eq!(
        8,
        TARGET_CI_JOBS.len(),
        "the guard must add NO top-level job; ci.yml now declares {TARGET_CI_JOBS:?}"
    );
    assert_eq!(
        6,
        GATED_JOBS.len(),
        "the guard reaches ci-gate through `area-ci` and adds nothing to its \
         fold; ci-gate now folds {GATED_JOBS:?}"
    );
    assert!(
        GATED_JOBS.contains(&"area-ci"),
        "the whole chain hangs off `area-ci` being folded"
    );

    // The chain itself, one link per assertion.
    assert!(
        job_block("ci.yml", "  area-ci:").contains("uses: ./.github/workflows/_area-ci.yml"),
        "`area-ci` must call the area workflow"
    );
    assert!(
        job_block("_area-ci.yml", "  package-ci:")
            .contains("uses: ./.github/workflows/_package-ci.yml"),
        "the area's package fan-out must call the package workflow"
    );

    let lint = job_block("_package-ci.yml", "  lint:");
    assert!(
        !lint.contains("\n    continue-on-error:"),
        "a lint job that swallowed its own failure would hand `area-ci` a \
         success and the guard could never block"
    );
    // The companions-only fold: the companion's outcome IS the cell's, and
    // only `success` passes. Anything weaker lets a red guard merge.
    assert!(
        lint.contains(r#"if [ "$COMPANIONS_ONLY" = "true" ]; then"#)
            && lint.contains(r#"if [ "$result" = "success" ] && [ "$COMPANION" != "success" ]; then"#),
        "the lint producer must fail a companions-only cell whose companion did \
         not SUCCEED; `!= failure` would pass a companion that never ran"
    );
}

/// The other half of the specification's integration requirement: a guard-only
/// selection must neither invoke unrelated suites nor manufacture full-tree
/// evidence.
#[test]
fn a_guard_only_selection_runs_the_guard_and_nothing_else() {
    // The registry half. A `recipe` would attach the guard to the owner's L1
    // cell as well, which would run the same scan twice on Linux under two
    // different sets of evidence rules.
    let entry = archive_guard_registry_entry();
    assert!(
        entry.contains(r#""lint_recipe": "cd tools/test-toolkit && just archive-path-guard","#),
        "the guard's lint half must name the canonical recipe: {entry}"
    );
    assert!(
        !entry.contains(r#""recipe":"#),
        "the guard must declare NO test half; with one it would also run as an \
         L1 companion: {entry}"
    );

    // The workflow half. Clippy is `if`-gated rather than deleted, so an
    // ordinary lint cell — every other cell — is untouched. The flag is the
    // plan cell's own, resolved through `cell_contract.py` like every other
    // execution input; no workflow input carries it.
    let lint = job_block("_package-ci.yml", "  lint:");
    assert!(
        lint.contains(
            "      - name: Lint\n        id: clippy\n        if: ${{ steps.cell.outputs.companions_only != 'true' }}\n"
        ),
        "the clippy step must be skipped on a companions-only cell, and only there"
    );
    assert!(
        lint.contains("COMPANIONS_ONLY: ${{ steps.cell.outputs.companions_only }}"),
        "the status fold must read the same cell output the clippy gate reads"
    );
    assert!(
        read("scripts/ci/cell_contract.py")
            .contains(r#""companions_only": "true" if cell.get("companions_only") else "","#),
        "the cell contract must carry the planner's field to the lint job"
    );
    for name in ["_area-ci.yml", "_package-ci.yml"] {
        assert!(
            !read(&format!(".github/workflows/{name}")).contains("lint-companions-only"),
            "{name} must not reintroduce a workflow input for a cell field"
        );
    }

    // The evidence half. The scan scope is the plan's, so a run that lost its
    // plan must fail rather than scan the full tree and record that as this
    // cell's result.
    assert!(
        lint.contains("BISCUIT_ARCHIVE_GUARD_PLAN: ${{ github.workspace }}/ci-artifacts/ci-resolved-plan/resolved-plan.json"),
        "the lint job must hand the guard the run's ONE resolved plan, by \
         absolute path — the recipe and nextest both leave the workspace"
    );
    let download = lint
        .split_once("      - name: Download the resolved execution plan\n")
        .map(|(_, rest)| rest.split("\n      - name: ").next().unwrap_or("").to_owned())
        .expect("the lint job must download the plan it points the guard at");
    assert!(
        download.contains("name: ci-resolved-plan")
            && download.contains("path: ci-artifacts/ci-resolved-plan"),
        "the download must land the run's one plan artifact where the guard \
         variable points: {download}"
    );
    assert!(
        !download.contains("continue-on-error") && !download.contains("if:"),
        "a lost plan artifact must FAIL this job. Tolerating or skipping it \
         would leave BISCUIT_ARCHIVE_GUARD_PLAN naming a file that does not \
         exist, and that scan's result would be recorded as this cell's \
         evidence: {download}"
    );
    assert_eq!(
        1,
        lint.matches("      - name: Download the resolved execution plan\n").count(),
        "the cell contract and the guard read the same downloaded plan"
    );
}

/// The specification's "one scheduler", as a negative: no consumer on the
/// guard's execution path derives a source scope of its own.
///
/// A second `git diff` would be silently wrong rather than loudly wrong. It
/// would produce a plausible file list against a base the planner never used,
/// the scan would pass, and the cell would record that pass as its evidence —
/// so nothing downstream could tell it apart from the planned scan.
#[test]
fn no_guard_consumer_derives_its_own_source_scope() {
    // The planner's own diff is the only one. `ci.yml`'s scope job takes it
    // and every consumer reads the plan that job publishes.
    let mut deriving: Vec<String> = Vec::new();
    let workflows = repo_root().join(".github/workflows");
    let mut entries: Vec<PathBuf> = fs::read_dir(&workflows)
        .expect("the workflow directory must be readable")
        .map(|entry| entry.expect("workflow directory entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "yml"))
        .collect();
    entries.sort();
    for path in entries {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .expect("workflow file names are UTF-8")
            .to_owned();
        for job in jobs(&workflow(&name)) {
            if job.contains("git diff --name-") {
                let header = job.lines().next().unwrap_or("").trim().to_owned();
                deriving.push(format!("{name}:{}", header.trim_end_matches(':')));
            }
        }
    }
    assert_eq!(
        vec!["ci.yml:scope".to_string()],
        deriving,
        "exactly one job may take a source diff — the one that resolves the plan"
    );

    // The recipe. A `git` call here would reach the standalone path too, where
    // there is no plan at all and the full tree is the documented answer.
    let recipe = read("tools/test-toolkit/justfile")
        .split_once("\narchive-path-guard:\n")
        .map(|(_, rest)| {
            rest.split("\n\n")
                .next()
                .expect("the recipe has a body")
                .to_owned()
        })
        .expect("tools/test-toolkit/justfile must define the guard recipe");
    assert!(
        !recipe.contains("git "),
        "the canonical recipe must take its scope from the plan, not a diff: {recipe}"
    );

    // The lint job and the local runner, which are the two callers of that
    // recipe. Each hands the guard a plan document and nothing else.
    let lint = job_block("_package-ci.yml", "  lint:");
    assert!(
        !lint.contains("git diff") && !lint.contains("--name-only"),
        "the lint job must not re-derive the scan scope it was handed"
    );
    let local = read("just/ci-local.just")
        .split_once("        guard_planned=false\n")
        .map(|(_, rest)| {
            rest.split("\n        if (( run_test")
                .next()
                .expect("the guard block is followed by the test block")
                .to_owned()
        })
        .expect("just/ci-local.just must run the planned guard");
    assert!(
        local.contains(r#"BISCUIT_ARCHIVE_GUARD_PLAN="${plan_file}""#),
        "the local run must hand the guard the plan it just resolved: {local}"
    );
    assert!(
        !local.contains("git diff"),
        "the local run must not re-derive the scan scope either: {local}"
    );
}

/// Every selection boundary takes the deletion status from its one diff and
/// declares it to the planner.
///
/// `--name-only` prints a removed path and a modified path identically, so the
/// planner's `--deleted` argument existed with no caller able to reach it,
/// `change_inventory.deleted` was empty on every hosted and local run, and the
/// guard had to read an absent file as "probably a deletion". Source contracts
/// here; `scripts/ci/test_ci_local.py`, `scripts/ci/test_affected_scope.py`,
/// and `.githooks/tests/test-pre-push.sh` run the three boundaries against
/// real repositories that delete and rename.
#[test]
fn every_selection_boundary_declares_its_deletions_to_the_planner() {
    let boundaries = [
        (".github/workflows/ci.yml", job_block("ci.yml", "  scope:")),
        ("just/ci-local.just", read("just/ci-local.just")),
        (".githooks/pre-push", read(".githooks/pre-push")),
    ];
    for (name, source) in &boundaries {
        assert!(
            source.contains("git diff --name-status -z"),
            "{name} must take the deletion status from its diff, not `--name-only`"
        );
        assert!(
            !source.contains("git diff --name-only"),
            "{name} must not keep a second, status-blind diff"
        );
        assert!(
            source.contains("scripts/ci/diff_scope.py"),
            "{name} must render its diff through the one shared parser, which is \
             what emits `--deleted`"
        );
    }

    // And the parser is the only thing that emits the argument, so a fourth
    // boundary cannot hand-roll a list that drifts from these three.
    let parser = read("scripts/ci/diff_scope.py");
    assert!(
        parser.contains(r#"parts.extend((b"--deleted", path))"#),
        "scripts/ci/diff_scope.py must emit the planner's repeatable --deleted argument"
    );
}

/// The registry's `lint_recipe` and its `just` tuple are two spellings of one
/// recipe. `validate_package_ci` already proves the TUPLE names a recipe the
/// justfile defines; what it cannot see is the two drifting apart, which would
/// leave the tuple's check green while the lint cell shelled out to something
/// else.
#[test]
fn the_guards_lint_recipe_and_its_just_tuple_name_the_same_recipe() {
    let entry = archive_guard_registry_entry();
    assert!(
        entry.contains(r#""just": (("tools/test-toolkit", "archive-path-guard"),),"#),
        "the registry must declare the guard's recipe as a `just` tuple so \
         `validate_package_ci` checks it exists: {entry}"
    );
    assert!(
        entry.contains(r#""lint_recipe": "cd tools/test-toolkit && just archive-path-guard","#),
        "the lint recipe must be the same directory and recipe as that tuple: {entry}"
    );
    // And the definition itself, because a contract that only compared the two
    // strings would stay green while both named a recipe nobody defines.
    assert!(
        read("tools/test-toolkit/justfile").contains("\narchive-path-guard:\n"),
        "tools/test-toolkit/justfile must define the `archive-path-guard` recipe"
    );
}

/// AC15: the planner replicates sniff's area rule and owns no mapping of its
/// own, so the contract that asks the real binary belongs ON the merge path.
/// It is a job of its own — not a companion suite of `repo-deps`, whose cell is
/// an archive consumer and must not go red for a sniff compile error — scoped
/// by its own planner flag and folded by
/// `ci-gate` like every other blocking job. `area-drift.yml` keeps only the
/// nightly backstop, because a planner defect could skip the gate job itself.
#[test]
fn the_area_drift_contract_is_enforced_on_the_merge_path() {
    let policy = read("scripts/ci/affected_scope.py");
    for source in [r#"AREA_DRIFT_PREFIXES = ("sniff/",)"#, r#"AREA_DRIFT_MANIFEST = "Cargo.toml""#] {
        assert!(
            policy.contains(source),
            "affected_scope.py must declare `{source}` — sniff and every package \
             manifest are the inputs ordinary package selection cannot see"
        );
    }
    assert!(
        policy.contains(r#"flags["area_drift"] = any("#),
        "the resolved plan must carry the area_drift flag"
    );

    let ci = workflow("ci.yml");
    assert!(
        ci.contains("area_drift=$(jq -r '.flags.area_drift'"),
        "the scope job must emit the derived area_drift flag"
    );

    let leg = job_block("ci.yml", "  area-drift:");
    assert!(
        leg.contains("needs.scope.outputs.area_drift == 'true'"),
        "the area-drift job must be gated on the scope-derived flag"
    );
    assert!(
        leg.contains("cargo build -p sniff-cli --release"),
        "the area-drift job must build the area authority it compares against"
    );
    // The guard turns an unprovisioned binary into a failure. Without it the
    // job would report three green tests having run nothing — the exact shape
    // this contract exists to prevent.
    assert!(
        leg.contains(r#"BISCUIT_REQUIRE_SNIFF: "1""#),
        "the area-drift job must set BISCUIT_REQUIRE_SNIFF so an absent sniff fails"
    );
    assert!(
        leg.contains("python3 scripts/ci/test_resolved_plan.py AreaGroupingTests"),
        "the area-drift job must run the contract class, not the whole suite"
    );
    // Folding is asserted structurally by `ci_gate_is_the_single_required_check`
    // against `GATED_JOBS`; named here so the two cannot drift apart silently.
    assert!(
        GATED_JOBS.contains(&"area-drift"),
        "ci-gate must fold the area-drift job"
    );

    // The standalone workflow is the backstop for a planner defect that would
    // skip the gate job. Its pull-request leg would now duplicate that job on
    // the same pull request.
    let backstop = workflow("area-drift.yml");
    assert!(
        backstop.contains("schedule:") && backstop.contains("workflow_dispatch:"),
        "area-drift.yml must keep the scheduled and manual backstop legs"
    );
    assert!(
        !backstop.contains("pull_request:"),
        "area-drift.yml must not re-run the gate job's class on the same pull request"
    );
}

/// Every host-tool guard in the CI Python suites, the workflow job that
/// provisions its tools, and the tools that job therefore declares.
///
/// Rows are `(invocation, workflow, job header, tools)`, where `invocation` is
/// what the step's body actually names. Most of these suites no longer appear
/// in a workflow by filename: they are `repo-deps`' companion suites, dispatched
/// from one step by `companion_suites.py` against `SUITE_REGISTRY`, so the
/// runner is what the row can point at.
///
/// The declaration is per step rather than a blanket fail under `CI` because
/// the two surfaces provision different things: the companion step installs
/// `just` and runs where Cargo, `jq`, and a modern Bash already are, while
/// `area-drift` is the only job anywhere that builds `sniff`.
const TOOL_GUARD_DECLARATIONS: &[(&str, &str, &str, &[&str])] = &[
    (
        "companion_suites.py",
        "_package-ci.yml",
        "  test:",
        &["CARGO", "JUST", "JQ", "BASH"],
    ),
    ("test_resolved_plan.py", "ci.yml", "  area-drift:", &["SNIFF"]),
    ("test_resolved_plan.py", "area-drift.yml", "  sniff-area-drift:", &["SNIFF"]),
];

/// The one step of `job` whose body mentions `needle`, as source.
///
/// Step-level rather than job-level because a `BISCUIT_REQUIRE_*` set on some
/// other step of the same job reaches the suite's process not at all.
fn step_containing(job: &str, needle: &str) -> String {
    job.split("\n      - ")
        .find(|step| step.contains(needle))
        .unwrap_or_else(|| panic!("no step of this job runs `{needle}`"))
        .to_string()
}

/// A guard on a host tool must skip on a developer host and FAIL in the job
/// that provisioned the tool, and its skip must name where the contract is
/// enforced. `BISCUIT_REQUIRE_<TOOL>` is how a job declares the tool present;
/// a declaration no job sets is the same silent skip wearing a different hat,
/// so both directions are closed here.
#[test]
fn every_tool_guard_declaration_is_set_by_the_job_that_enforces_it() {
    let mut declared: Vec<String> = Vec::new();
    for (suite, file, header, tools) in TOOL_GUARD_DECLARATIONS {
        let job = job_block(file, header);
        let step = step_containing(&job, &format!("scripts/ci/{suite}"));
        for tool in *tools {
            let variable = format!("BISCUIT_REQUIRE_{tool}");
            assert!(
                step.contains(&format!("{variable}: \"1\"")),
                "{file}'s `{}` job runs {suite} and provisions {tool}, so that step \
                 must set {variable} — without it the guard skips and reports a green \
                 cell that verified nothing",
                header.trim().trim_end_matches(':')
            );
            declared.push(variable);
        }
    }

    // The reverse direction: a variable a workflow sets that no guard reads is
    // as dead as one no workflow sets.
    let workflows = repo_root().join(".github/workflows");
    for entry in fs::read_dir(&workflows).expect("read .github/workflows") {
        let path = entry.expect("workflow entry").path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("yml") {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let source = read(&format!(".github/workflows/{name}"));
        for line in source.lines() {
            let Some(start) = line.find("BISCUIT_REQUIRE_") else {
                continue;
            };
            let variable: String = line[start..]
                .chars()
                .take_while(|character| character.is_ascii_uppercase() || *character == '_')
                .collect();
            let variable = variable.trim_end_matches('_').to_string();
            assert!(
                declared.contains(&variable),
                "{name} mentions {variable}, which no guard in TOOL_GUARD_DECLARATIONS reads"
            );
        }
    }
}

/// One mechanism, not thirteen. `tool_guard.require_tools` is the only place a
/// CI Python suite may decide that an absent host tool means skip: it forces
/// every caller to name the enforcing job, and it fails where that job declared
/// the tool. A suite that reaches for `unittest.skipUnless(shutil.which(...))`
/// again has reintroduced the guard that can only ever skip.
#[test]
fn no_ci_python_suite_gates_a_host_tool_outside_the_shared_guard() {
    let directory = repo_root().join("scripts/ci");
    let mut suites_using_the_guard = 0;
    for entry in fs::read_dir(&directory).expect("read scripts/ci") {
        let path = entry.expect("scripts/ci entry").path();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        if !name.starts_with("test_") || !name.ends_with(".py") {
            continue;
        }
        let source = read(&format!("scripts/ci/{name}"));
        if source.contains("require_tools(") {
            suites_using_the_guard += 1;
        }
        for line in source.lines() {
            assert!(
                !(line.contains("skipUnless") && line.contains("which(")),
                "{name} gates a host tool with skipUnless: `{}` — use \
                 tool_guard.require_tools, which names its enforcing job and fails \
                 where that job provisioned the tool",
                line.trim()
            );
        }
    }
    assert!(
        suites_using_the_guard >= 3,
        "the three suites holding host-tool guards must all reach the shared \
         mechanism; found {suites_using_the_guard}"
    );

    // Prioritized action 3 of `reviews/2026-09-15-python-test-code`: a planner
    // that cannot resolve a full-scope plan is the loudest failure this corpus
    // test can see, and it used to report the whole class green.
    let scope_suite = read("scripts/ci/test_affected_scope.py");
    assert!(
        scope_suite.contains("the planner could not resolve a full-scope plan"),
        "ArchiveInventoryClosureTests must fail, not skip, when the planner cannot \
         resolve a plan"
    );
}

/// The artifact publisher writes the owner measurement AC8 is read from, and it
/// is CommonJS that no Cargo package and no pnpm workspace entry selects. With
/// the tooling leg retired, the registry is the only thing that can schedule
/// it: a suite absent from `SUITE_REGISTRY` and from its owner's declared
/// companions runs nowhere, which is the silent gap this whole family closes.
#[test]
fn the_owner_measurement_publisher_suite_is_scheduled_by_the_registry() {
    let registry = read("scripts/ci/affected_scope.py");
    assert!(
        registry.contains("scripts/ci/artifacts/*.test.cjs"),
        "SUITE_REGISTRY must schedule the artifact publisher's Node suite; \
         no Cargo package or pnpm workspace entry selects it, so without an \
         entry it runs nowhere"
    );
    // A bare directory argument is resolved as a module, so the pattern must
    // reach `node --test` quoted and unexpanded, and the suite needs the
    // pinned Node the `node` attribute provisions.
    assert!(
        registry.contains("node --test 'scripts/ci/artifacts/*.test.cjs'"),
        "the publisher suite must be invoked by quoted glob, not by directory"
    );

    let manifest = read("scripts/Cargo.toml");
    assert!(
        manifest.contains("artifact-publisher"),
        "`repo-deps` must declare the publisher suite among its companion \
         suites; a registry entry nobody owns is scheduled for no cell"
    );
}

/// B3/B4: the WSL guest must have `jq` before its native-prerequisites step
/// parses with it, and the package's declared L1 slow-test contract must reach
/// the guest or darkmatter's L1 suite silently narrows on wsl2-ubuntu only.
#[test]
fn the_wsl_leg_provisions_jq_and_forwards_the_slow_test_contract() {
    let wsl = workflow("_wsl-ci.yml");
    assert!(
        wsl.contains("xz-utils jq python3"),
        "the WSL guest must be provisioned with jq — the native-prerequisites step \
         parses with it before anything could install it — and with python3, which \
         the guest's own completeness validator needs and no compiler can supply \
         (ruling R2)"
    );
    let apt = wsl
        .split("- name: Install guest packages")
        .nth(1)
        .expect("_wsl-ci.yml must install its guest packages in a named step");
    let apt = &apt[..apt.find("\n      - name:").unwrap_or(apt.len())];
    assert!(
        apt.contains("python3 --version") && apt.contains("jq --version"),
        "reachability is proved in the SAME step that installs: a package apt \
         installed but cannot execute is a provisioning failure, and finding that \
         out after the suite reads as a validator defect"
    );
    assert!(
        wsl.contains("export BISCUIT_L1_INCLUDE_SLOW='${{ steps.cell.outputs.l1_include_slow }}'"),
        "the guest must export the package's l1-include-slow contract, resolved \
         from the plan; without it darkmatter's L1 narrows on wsl2-ubuntu alone"
    );
    // Twice: the gate and the expected-test listing must agree about which
    // tests the cell owes, or the comparison is between two different suites.
    assert_eq!(
        2,
        wsl.matches("export BISCUIT_L1_INCLUDE_SLOW=").count(),
        "the listing and the gate must select from one slow-test contract"
    );
}

// ---------------------------------------------------------------------------
// The area-owned CI presentation
//
// Phase 2 of `fixes/2026-09-11-cicd-cleanup/plan.md` froze these as pending
// contracts; Phase 6 and the review-5 accepted-gap publisher implemented every
// one, so they are all ordinary assertions now and a regression fails this
// suite. The Rust `pending_contract` wrapper survives in
// `scripts/ci-rollup-tests.rs`; the Python twin is
// `scripts/ci/pending_contracts.py`.
// ---------------------------------------------------------------------------

/// The workflows whose job names reach a reader through the Actions graph and
/// the PR Checks tab.
const READER_FACING_WORKFLOWS: [&str; 4] =
    ["ci.yml", "_area-ci.yml", "_package-ci.yml", "_wsl-ci.yml"];

/// A job's declared `name:`, when it has one.
fn job_name(block: &str) -> Option<String> {
    block
        .lines()
        .find(|line| line.starts_with("    name:"))
        .map(|line| line.trim_start_matches("    name:").trim().to_owned())
}

/// Jobs that can be skipped as a whole *and* carry an expression in their name.
///
/// GitHub never evaluates the matrix context for a job skipped through `if:` or
/// `needs:`, so the whole matrix collapses into one skipped job labelled with
/// the raw expression. Run 34638047631 produced 63 such entries.
fn skippable_jobs_with_expression_names() -> Vec<String> {
    let mut offenders = Vec::new();
    for file in READER_FACING_WORKFLOWS {
        let source = workflow(file);
        for block in jobs(&source) {
            let header = block.lines().next().unwrap_or("").trim().to_owned();
            let skippable = block.contains("\n    if:") || block.contains("\n    needs:");
            match job_name(&block) {
                Some(name) if skippable && name.contains("${{") => {
                    offenders.push(format!("{file}:{header} name: {name}"));
                }
                _ => {}
            }
        }
    }
    offenders
}

/// AC10.
#[test]
fn no_skippable_job_is_labelled_with_an_unresolved_expression() {
    let offenders = skippable_jobs_with_expression_names();
    assert!(
        offenders.is_empty(),
        "{} job(s) are skippable and labelled with an unresolved \
         expression:\n  {}",
        offenders.len(),
        offenders.join("\n  ")
    );
}

/// AC10, from the other side: a skippable job must still be *identifiable*.
///
/// Dropping `name:` is what makes the label static, because GitHub falls back
/// to the job id (and appends the matrix values when the job does expand). A
/// job that instead kept a static but uninformative name would satisfy the
/// test above and tell a reader nothing.
#[test]
fn every_skippable_job_has_a_static_human_readable_identity() {
    let mut checked = 0;
    for file in READER_FACING_WORKFLOWS {
        let source = workflow(file);
        for block in jobs(&source) {
            let id = block
                .lines()
                .next()
                .unwrap_or("")
                .trim()
                .trim_end_matches(':')
                .to_owned();
            if !(block.contains("\n    if:") || block.contains("\n    needs:")) {
                continue;
            }
            checked += 1;
            let label = job_name(&block).unwrap_or_else(|| id.clone());
            assert!(
                label.len() >= 4 && label.chars().any(|c| c.is_ascii_alphabetic()),
                "{file}: skippable job `{id}` collapses to the label {label:?}, \
                 which names nothing a reader can act on"
            );
        }
    }
    assert!(
        checked >= 8,
        "expected the reader-facing workflows to contain skippable jobs; found {checked}"
    );
}

/// AC10: every producer label identifies its cell — package, gate, and
/// environment — without a `name:` GitHub would render raw on a whole-job skip.
///
/// Under the row layout this is one rule rather than two: every producer
/// expands an `include:` matrix of rows, so GitHub builds the label from the
/// row's own values. `lint` used to need a static `name:` because it had no
/// matrix to take an environment from; it has one now.
#[test]
fn lint_and_check_labels_identify_their_environment() {
    for job_id in ["lint", "check", "test"] {
        let block = job_block("_package-ci.yml", &format!("  {job_id}:"));
        assert!(
            job_name(&block).is_none(),
            "{job_id} is skippable, so it must take its label from its matrix \
             rather than from a `name:` GitHub would render raw"
        );
        assert!(
            block.contains(&format!("include: ${{{{ fromJSON(inputs.{job_id}-rows) }}}}")),
            "{job_id} must expand its own row set, so its label carries the \
             package, the gate, and the environment"
        );
    }
}

/// AC2: the area, not the package, is the top-level identity of the fan-out.
#[test]
fn the_area_is_the_top_level_identity_of_the_package_fan_out() {
    let ci = workflow("ci.yml");
    let fan_out = job_block("ci.yml", "  area-ci:");
    assert!(
        fan_out.contains("area: ${{ fromJSON(needs.scope.outputs.scheduled_areas) }}")
            && fan_out.contains("uses: ./.github/workflows/_area-ci.yml"),
        "ci.yml's tested fan-out must be one caller identity per selected area"
    );
    assert!(
        !jobs(&ci).iter().any(|block| block.starts_with("  package-ci:")),
        "a package-level fan-out in ci.yml would put packages back at the top level"
    );
    // The package stays the stored identity underneath (Design Decision 1) —
    // carried by the ROW now, not by a caller job's name. Area is a grouping;
    // nothing below re-keys an artifact, a baseline entry, or a receipt by it.
    let area = workflow("_area-ci.yml");
    assert!(
        area.contains("uses: ./.github/workflows/_package-ci.yml"),
        "each area must delegate execution to _package-ci.yml"
    );
    assert!(
        !job_block("_area-ci.yml", "  package-ci:").contains("\n    strategy:"),
        "one call per area: a second fan-out here would be a second place \
         deciding what runs"
    );
    let reader = read("scripts/ci/cell_contract.py");
    assert!(
        reader.contains(r#""package": row["package"]"#),
        "the package remains the result identity, read from the row"
    );
}

/// AC11: each selected area owns its own outcome, and only its own.
#[test]
fn every_selected_area_owns_an_always_coverage_audit() {
    let rollup = job_block("_area-ci.yml", "  coverage-audit:");
    assert!(
        rollup.contains("needs: package-ci") && rollup.contains("if: always()"),
        "the area coverage audit must wait for its producers and render even when they failed"
    );
    assert!(
        rollup.contains(r#"--area "$AREA""#),
        "the area coverage audit must narrow the rollup tool to its own area"
    );
    assert!(
        rollup.contains("ci-rollup verdict") && rollup.contains("--baseline"),
        "the area coverage audit must apply its own baseline and fail closed"
    );
    let audit_steps = steps(&rollup);
    let enforcement = step_named(&audit_steps, "Enforce coverage completeness and exceptions")
        .expect("the coverage audit must have one explicit enforcement step");
    assert!(
        enforcement.contains(
            "if: needs.package-ci.result == 'success' || needs.package-ci.result == 'skipped'"
        ),
        "an ordinary producer failure must not create a second red coverage-audit \
         check, and an area whose producer call was skipped — all-reused, \
         gap-only, or wrongly skipped — must still be judged"
    );
    assert!(
        enforcement.contains("PRODUCERS: ${{ needs.package-ci.result }}")
            && enforcement.contains(r#"--producers "$PRODUCERS""#),
        "the verdict must be told the producer call's result, so a call skipped \
         over executing cells blocks as missing coverage instead of passing an \
         audit with nothing to read"
    );
    // Package stays the stored identity: only the artifact NAME carries the
    // area, and it carries the slug so a nested area is a legal artifact name.
    assert!(
        rollup.contains("name: ci-results-${{ inputs.slug }}"),
        "each area must publish its own result slice under an artifact-safe name"
    );
}

/// AC11/spec §5: runner-loss attribution moved into the per-area path.
#[test]
fn runner_loss_attribution_is_narrowed_to_the_owning_area() {
    let rollup = job_block("_area-ci.yml", "  coverage-audit:");
    assert!(
        rollup.contains("runner_loss.py attribute") && rollup.contains("--package"),
        "an area must synthesize statuses only for its own packages; a job name \
         carries no area, so the narrowing is expressed as a package list"
    );
    assert!(
        rollup.contains("select(.area == $area and (.gates | length) > 0)"),
        "the package list must come from the PLAN — an all-reused or gap-only \
         area dispatches no row and still owns every package the audit accounts \
         for — and never be hand-written"
    );
}

/// AC4: a reused cell is published without any runner work for it.
#[test]
fn a_reused_cell_reaches_its_area_summary_without_being_re_executed() {
    let area = workflow("_area-ci.yml");
    let rollup = job_block("_area-ci.yml", "  coverage-audit:");
    // The rollup renders the reused cells (`render_grid`'s "Reused results"
    // table) into the area's step summary, and it is the only job in the area
    // workflow that is not a package execution.
    assert!(
        rollup.contains("ci-rollup rollup") && rollup.contains("--plan "),
        "the area coverage audit must read the plan, which says a cell was reused"
    );
    for forbidden in ["just _test", "cargo nextest", "install-action@nextest"] {
        assert!(
            !rollup.contains(forbidden),
            "the area coverage audit runs `{forbidden}`; publishing a reused cell must \
             invoke no setup, build, archive, or test step"
        );
    }
    // And the scheduling half: the rows the area hands its execution call are
    // derived from the plan's EXECUTING cells alone, so no runner is scheduled
    // for a cell a receipt satisfied — and the job that would run it is not
    // called at all when every row set is empty.
    for set in ["test", "check", "lint", "wsl"] {
        assert!(
            area.contains(&format!("{set}-rows: ${{{{ toJSON(fromJSON(inputs.rows).{set}) }}}}")),
            "the area must forward the planner's {set} row set verbatim"
        );
    }
    let call = job_block("_area-ci.yml", "  package-ci:");
    assert!(
        call.contains("fromJSON(inputs.rows).has_test_rows"),
        "the call is guarded by the planner's scalar row flags"
    );
    let adapter = read("scripts/ci/affected_scope.py");
    assert!(
        adapter.contains(r#"if cell["execution"] != "execute":"#),
        "the row adapter emits a row for an executing cell and nothing else"
    );
}

#[test]
fn empty_execution_matrices_skip_before_expansion() {
    for (job, input) in [
        ("check", "check-rows"),
        ("test", "test-rows"),
        ("lint", "lint-rows"),
        ("wsl2", "wsl-rows"),
    ] {
        let block = job_block("_package-ci.yml", &format!("  {job}:"));
        let condition = block.lines()
            .find(|line| line.starts_with("    if:"))
            .expect("an empty execution matrix needs a job-level guard");
        assert!(
            condition.contains(&format!("inputs.{input} != '[]'")),
            "{job} must skip before expanding an empty {input} matrix: {condition}"
        );
    }
}

/// AC13/spec §7: the concurrency correction survives the scheduling redesign.
#[test]
fn the_worker_policy_survives_the_area_restructure() {
    let package_ci = workflow("_package-ci.yml");
    assert!(
        package_ci.contains("just _test_threads"),
        "the CI worker policy must still come from the shared `_test_threads` recipe"
    );
    let test_job = job_block("_package-ci.yml", "  test:");
    let job_steps = steps(&test_job);
    let gate = step_with_id(&job_steps, "gate").expect("the test job carries the gate command");
    let script = step_script(gate);
    let l2_branch = script
        .split("L2)")
        .nth(1)
        .expect("the gate command branches on the L2 tier")
        .split(";;")
        .next()
        .unwrap_or_default();
    assert!(
        l2_branch.contains("l2-parallel-self-spawn") && l2_branch.contains("BISCUIT_L2_THREADS"),
        "only a declared self-isolating L2 suite may use the shared worker budget"
    );
    assert!(
        l2_branch.contains("BISCUIT_TEST_REQUIRED_BACKENDS"),
        "backend execution proof must survive the restructure"
    );
    // The declared suites come from the plan now, so the vocabulary they are
    // drawn from is the registry's — which is where the name lives.
    assert!(
        read("scripts/ci/affected_scope.py").contains("homelab-frontend"),
        "declared companion suites must survive the restructure"
    );
    assert!(
        test_job.contains("companion_suites.py"),
        "the test job must still run its cell's declared companion suites"
    );
}

/// AC11 (Phase 7): a failure-classifying job must not fail the run.
#[test]
fn advisory_jobs_cannot_fail_the_run_and_gates_are_not_advisory() {
    let report = job_block("ci.yml", "  ci-reporting:");
    assert!(
        report.contains("continue-on-error: true"),
        "the advisory report must be structurally unable to fail the run: the \
         merge gate this repository is moving to folds the run's conclusion"
    );
    // The converse. Anything that is allowed to block must not silently opt
    // out of blocking.
    for (file, header) in [
        ("ci.yml", "  scope:"),
        ("ci.yml", "  preflight:"),
        ("ci.yml", "  ci-gate:"),
        ("_area-ci.yml", "  coverage-audit:"),
    ] {
        let block = job_block(file, header);
        let executable: String = block
            .lines()
            .filter(|line| !line.trim_start().starts_with('#'))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            !executable.contains("\n    continue-on-error:"),
            "{file}: `{}` is a gate and must not be job-level continue-on-error",
            header.trim()
        );
    }
}

/// AC11: no standalone global policy verdict. `ci-gate` is a fold, and
/// `ci_gate_is_the_single_required_check` pins that it stays one.
#[test]
fn no_standalone_global_verdict_job_remains() {
    let ci = workflow("ci.yml");
    assert!(
        !jobs(&ci).iter().any(|block| block.starts_with("  ci-verdict:")),
        "ci.yml still declares a standalone `ci-verdict` job"
    );
    for block in jobs(&ci) {
        let header = block.lines().next().unwrap_or("").trim().to_owned();
        if header == "ci-gate:" {
            continue;
        }
        assert!(
            !block.contains("ci-rollup verdict"),
            "`{header}` re-judges the run; only each area's coverage audit may run the verdict"
        );
    }
}

/// AC11: the job's removal was atomic with its consumers'. Nothing outside the
/// workflows may still key on `ci-verdict` by name or on the whole-run
/// `ci-results` artifact it used to upload.
#[test]
fn the_verdict_consumers_are_rewired_when_the_job_goes() {
    let ci_diff = read("just/devops.just");
    assert!(
        !ci_diff.contains("-n ci-results ") && ci_diff.contains("-p 'ci-results-*'"),
        "`just ci-diff` must download each run's per-area slices, not a whole-run artifact"
    );
    assert!(
        ci_diff.contains("compare_args+=(--base") && ci_diff.contains("compare_args+=(--head"),
        "`just ci-diff` must hand every slice to `ci-rollup compare`"
    );
    let watchdog = read(".claudine/scripts/ci-watchdog.ts");
    assert!(
        !watchdog.contains("\"ci-verdict\"") && watchdog.contains("\"ci-gate\""),
        "the CI watchdog must wait for the `ci-gate` job"
    );
    let runner_loss = read("scripts/ci/runner_loss.py");
    let non_producers = runner_loss
        .split_once("NON_PRODUCER_JOBS = {")
        .and_then(|(_, rest)| rest.split_once('}'))
        .map(|(block, _)| block)
        .expect("runner_loss.py must declare NON_PRODUCER_JOBS");
    assert!(
        non_producers.contains("\"ci-gate\"") && !runner_loss.contains("\"ci-verdict\""),
        "runner-loss attribution must exclude the gate, not the retired verdict"
    );
}

// ---------------------------------------------------------------------------
// Phase 7 — the seams the merge-authority migration moves across
//
// The ruleset edit itself is Ken's (OQ3/B3), but the properties the migration
// depends on are static and testable now: the run conclusion must already be a
// faithful conjunction, every area's results must already be published, and
// every area coverage audit must read the plan, policy, and baseline.
// ---------------------------------------------------------------------------

/// Spec §5: `reuse_validation.py`, `release-plz.yml`, and `ci-infra-retry.yml`
/// all key on the `ci` run's conclusion, which after the migration is also
/// what branch protection folds.
#[test]
fn every_downstream_consumer_reads_the_same_ci_run_conclusion() {
    for file in ["release-plz.yml", "ci-infra-retry.yml"] {
        let source = workflow(file);
        assert!(
            source.contains("workflow_run:"),
            "{file}: must key on a `ci` run, not race it"
        );
        assert!(
            source.contains("workflows: [ci]") || source.contains(r#"workflows: ["ci"]"#),
            "{file}: must watch the `ci` workflow by name"
        );
        assert!(
            source.contains("github.event.workflow_run.conclusion"),
            "{file}: must read the run conclusion, not a named check"
        );
    }
    let reuse = read("scripts/ci/reuse_validation.py");
    assert!(
        reuse.contains(r#"run.get("status") != "completed""#)
            && reuse.contains(r#"run.get("conclusion") != "success""#),
        "reuse_validation must accept only a completed, successful run; a \
         failure, a cancellation, or an in-flight run must fall back to \
         normal CI"
    );
    assert!(
        reuse.contains(r#"run.get("path") == ".github/workflows/ci.yml""#),
        "reuse_validation must pin the run to this repository's ci.yml"
    );
}

/// Spec §5: a `continue-on-error` job is invisible to the run conclusion, so
/// exactly one job — the advisory report — may carry it.
#[test]
fn only_the_advisory_report_is_excluded_from_the_run_conclusion() {
    let mut advisory = Vec::new();
    for file in READER_FACING_WORKFLOWS {
        let source = workflow(file);
        for block in jobs(&source) {
            let header = block.lines().next().unwrap_or("").trim().to_owned();
            let executable: String = block
                .lines()
                .filter(|line| !line.trim_start().starts_with('#'))
                .collect::<Vec<_>>()
                .join("\n");
            if executable.contains("\n    continue-on-error: true") {
                advisory.push(format!("{file}:{header}"));
            }
        }
    }
    assert_eq!(
        vec!["ci.yml:ci-reporting:".to_owned()],
        advisory,
        "only the advisory report may opt out of the run conclusion; every \
         other job's failure has to reach the gate that folds it"
    );
}

/// Spec §5: the per-area result slices are the only machine-readable results
/// a run produces, so a blocked area must still publish one.
#[test]
fn a_coverage_blocked_area_still_publishes_its_result_slice() {
    let rollup = job_block("_area-ci.yml", "  coverage-audit:");
    let upload = rollup
        .find("name: Upload this area's result slice")
        .expect("the area coverage audit must upload its result slice");
    let judge = rollup
        .find("name: Enforce coverage completeness and exceptions")
        .expect("the area coverage audit must enforce its policy");
    assert!(
        upload < judge,
        "the slice must be uploaded BEFORE the verdict step, so a blocked \
         area's results are still available for reporting and future reuse"
    );
    assert!(
        rollup[upload..judge].contains("if: always()"),
        "the upload must be unconditional; a cancelled producer must not take \
         the area's results with it"
    );
    // Written, uploaded, and judged under one name, so the artifact a reader
    // downloads is the document the verdict was taken against.
    for usage in [
        r#"--out "ci-results-${{ inputs.slug }}.json""#,
        r#"path: ci-results-${{ inputs.slug }}.json"#,
        r#"--results "ci-results-${{ inputs.slug }}.json""#,
    ] {
        assert!(
            rollup.contains(usage),
            "the coverage audit must write, upload, and judge one slice file: `{usage}`"
        );
    }
}

/// The coverage audit is the only reader of the plan, policy, environment
/// table, and baseline after execution.
#[test]
fn the_area_coverage_audit_reads_plan_policy_and_baseline() {
    let rollup = job_block("_area-ci.yml", "  coverage-audit:");
    for input in [
        "name: ci-scope",
        "name: ci-resolved-plan",
        "--environments .github/ci/environments.json",
        ".github/ci/ci-baseline.toml",
        "pattern: '{junit-*,status-*,build-status-*,completion-*}'",
    ] {
        assert!(
            rollup.contains(input),
            "the area coverage audit must read `{input}`"
        );
    }
    assert!(
        job_block("ci.yml", "  ci-gate:").contains("      - area-ci\n"),
        "ci-gate must wait on the area fan-out so producer and audit failures reach the fold"
    );
}

/// Every `uses: ./.github/workflows/…` edge reachable from an entry workflow.
fn reusable_workflow_depth(entry: &str, seen: &mut Vec<String>) -> usize {
    if seen.iter().any(|name| name == entry) {
        panic!("reusable-workflow cycle through {entry}: {seen:?}");
    }
    seen.push(entry.to_owned());
    let source = workflow(entry);
    let deepest = source
        .lines()
        .filter_map(|line| line.trim().strip_prefix("uses: ./.github/workflows/"))
        .map(|called| reusable_workflow_depth(called.trim(), seen))
        .max()
        .unwrap_or(0);
    seen.pop();
    1 + deepest
}

#[test]
fn the_reusable_workflow_chain_stays_within_githubs_four_levels() {
    // The tightest constraint Phase 6 had. `ci.yml -> _area-ci.yml ->
    // _package-ci.yml -> _wsl-ci.yml` is four levels including the caller,
    // GitHub's maximum, so no margin remains for a later insertion.
    let depth = reusable_workflow_depth("ci.yml", &mut Vec::new());
    assert!(
        depth <= 4,
        "the reusable-workflow chain from ci.yml is {depth} levels deep; GitHub \
         allows four including the caller"
    );
    assert_eq!(
        depth, 4,
        "the chain is expected to sit exactly at GitHub's ceiling; a different \
         depth means the area or WSL level moved and this contract needs rereading"
    );
    assert!(
        !workflow("_wsl-ci.yml").contains("uses: ./.github/workflows/"),
        "_wsl-ci.yml is the last level the chain can afford; it must call no \
         further reusable workflow"
    );
}

/// A job block's lines outside comments, so a permission or endpoint named
/// in prose is not read as one the job holds or calls.
fn executable_lines(block: &str) -> String {
    block
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The job ids of `file` whose executable lines contain `needle`.
fn jobs_containing(file: &str, needle: &str) -> Vec<String> {
    jobs(&workflow(file))
        .iter()
        .filter(|block| executable_lines(block).contains(needle))
        .map(|block| {
            block
                .lines()
                .next()
                .unwrap_or("")
                .trim()
                .trim_end_matches(':')
                .to_owned()
        })
        .collect()
}

/// AC9, OQ4 (ruled `neutral`, 2026-09-12): an accepted policy gap is published
/// the moment its area starts, by the one job allowed to write checks, and the
/// workflow itself spells no conclusion.
#[test]
fn the_gap_publisher_is_the_only_job_holding_checks_write() {
    // A called workflow's token is CAPPED by its caller's, so `area-ci` must
    // carry the grant for `_area-ci.yml` to be able to confine it — and it is
    // the only job in `ci.yml` that does.
    assert_eq!(
        vec!["area-ci".to_owned()],
        jobs_containing("ci.yml", "checks: write"),
        "ci.yml must grant `checks: write` on `area-ci` and nowhere else"
    );
    assert_eq!(
        vec!["accepted-gaps".to_owned()],
        jobs_containing("_area-ci.yml", "checks: write"),
        "exactly one job in _area-ci.yml may hold `checks: write`, and it is the publisher"
    );
    for file in ["_package-ci.yml", "_wsl-ci.yml"] {
        assert!(
            jobs_containing(file, "checks:").is_empty(),
            "{file} must declare no checks permission of its own; it inherits \
             the read-only set `package-ci` confines it to"
        );
    }

    let publisher = executable_lines(&job_block("_area-ci.yml", "  accepted-gaps:"));
    assert!(
        !publisher.contains("\n    needs:"),
        "the publisher must wait on nothing: an accepted gap appears immediately, \
         before any test or archive build"
    );
    assert!(
        publisher.contains("python3 scripts/ci/publish_gaps.py") && publisher.contains("--area"),
        "the publisher must delegate to publish_gaps.py for this area"
    );
    assert!(
        publisher.contains("HEAD_SHA: ${{ inputs.tested-revision }}"),
        "the check runs must land on the tested revision — the PR head, which is \
         also what this job checks out. `github.sha` is the merge commit on a \
         `pull_request` event, and a check there never shows in the PR checks list"
    );
    assert!(
        publisher.contains("name: ci-resolved-plan"),
        "the publisher reads the plan the scope job already uploaded"
    );
    assert!(
        job_name(&job_block("_area-ci.yml", "  accepted-gaps:")).is_none(),
        "the publisher is skippable and must keep its static `accepted-gaps` label (AC10)"
    );
    for scope in ["actions:", "pull-requests:", "contents: write"] {
        assert!(
            !publisher.contains(scope),
            "the publisher holds `contents: read` and `checks: write` only; found `{scope}`"
        );
    }
    assert!(publisher.contains("contents: read"), "checkout needs contents: read");

    // The read-only siblings.
    let rollup = executable_lines(&job_block("_area-ci.yml", "  coverage-audit:"));
    assert!(
        rollup.contains("checks: read") && !rollup.contains("checks: write"),
        "the coverage audit stays at `checks: read`"
    );
    let package_ci = executable_lines(&job_block("_area-ci.yml", "  package-ci:"));
    assert!(
        package_ci.contains("permissions:") && package_ci.contains("contents: read"),
        "`package-ci` must declare its own read-only permissions so the fan-out \
         does not inherit the caller's `checks: write`"
    );
    assert!(!package_ci.contains("checks:"), "`package-ci` holds no checks scope at all");

    // The workflow posts nothing itself and names no conclusion; the tool is
    // where `neutral` lives, and `cancelled` keeps its one meaning.
    for block in jobs(&workflow("_area-ci.yml")) {
        let executable = executable_lines(&block);
        assert!(
            !executable.contains("check-runs") && !executable.contains("conclusion"),
            "_area-ci.yml must not create check runs or spell a conclusion outside the tool"
        );
    }
    let tool = read("scripts/ci/publish_gaps.py");
    assert!(
        tool.contains("CONCLUSION = \"neutral\""),
        "publish_gaps.py publishes `neutral` (OQ4)"
    );
    assert!(
        !tool.contains("\"cancelled\""),
        "publish_gaps.py must never emit `cancelled`; that conclusion means interruption"
    );
}

#[test]
fn the_rollup_binary_is_still_built_without_the_monorepo_crates() {
    // `scripts/Cargo.toml`'s `local-tools` feature exists so the always-runs
    // required check does not pay for gix, duckdb, and the terminal renderer.
    // Phase 4's `--plan` renderer stays out of this invocation. Every per-area
    // coverage audit builds it; `ci.yml` builds nothing itself, because CI's
    // own Cargo suites now run in `repo-deps`'s ordinary cells.
    let source = workflow("_area-ci.yml");
    assert!(
        source.contains("--no-default-features") && source.contains("--bin ci-rollup"),
        "_area-ci.yml must build ci-rollup with --no-default-features"
    );
    assert!(
        !workflow("ci.yml").contains("cargo nextest run"),
        "ci.yml must run no Cargo suite of its own; its packages' cells do"
    );
    let rollup = read("scripts/ci-rollup.rs");
    assert!(
        !rollup.contains("biscuit_terminal"),
        "ci-rollup must link none of the monorepo crates"
    );
}

// ---------------------------------------------------------------------------
// Phase 4 — per-cell evidence and the pre-trigger plan
// ---------------------------------------------------------------------------

/// AC5: the scope job consumes a per-CELL verified set, not one environment.
///
/// One environment name can never combine a macOS receipt with a prior WSL
/// cross-check receipt, which is the whole of spec section 3.
#[test]
fn the_scope_job_consumes_verified_cells_not_one_environment() {
    let ci = workflow("ci.yml");
    assert!(
        ci.contains("verify --cells") && ci.contains("--accepted-cells"),
        "the scope job must verify per cell and pass the accepted set to the planner"
    );
    assert!(
        !ci.contains("--accepted-environment") && !ci.contains("--exclude-environment"),
        "no whole-environment exclusion may remain: it cannot express two hosts, \
         and the retired form omitted the cell along with its execution"
    );
}

/// AC7: a refused receipt is reported, never silently dropped.
#[test]
fn refused_evidence_is_carried_into_the_plan() {
    let ci = workflow("ci.yml");
    assert!(
        ci.contains("--rejections") && ci.contains("--evidence-rejections"),
        "the coded rejection reasons must reach the plan, or a reviewer cannot \
         tell a missing receipt from a rejected one"
    );
}

/// The canonical plan is published beside the legacy projection it produced.
#[test]
fn the_resolved_plan_is_uploaded_for_review() {
    let ci = workflow("ci.yml");
    assert!(
        ci.contains("--plan-out resolved-plan.json"),
        "one calculation must serve both documents"
    );
    assert!(
        ci.contains("name: ci-resolved-plan"),
        "the resolved plan must be uploaded for the rollup and for reviewers"
    );
}

/// AC17: the constraint store is a trigger-boundary concern only.
///
/// Design Decision 8. A constraint CI could read would let it silently skip
/// required coverage; a constraint only the hook reads can only ever stop a
/// push.
#[test]
fn ci_never_reads_the_execution_constraint_store() {
    for name in ["ci.yml", "_package-ci.yml", "_wsl-ci.yml"] {
        let text = workflow(name);
        // `scripts/ci/constraints.py`, not `test_constraints.py`: the tooling
        // leg runs the store's own suite, which is not the store being read.
        assert!(
            !text.contains("BISCUIT_CI_CONSTRAINTS_DIR")
                && !text.contains("ci/constraints.py"),
            "{name} reads the execution-constraint store; constraints bind the \
             trigger, never CI scheduling"
        );
    }
    let hook = read(".githooks/pre-push");
    assert!(
        hook.contains("ci/constraints.py"),
        "the pre-push hook is where a recorded constraint is enforced"
    );
}

/// The three feature levels of `scripts/`, each holding its own cost line.
///
/// `ci-rollup` links none of the monorepo's crates. `ci-plan` and `ci-build`
/// are `build-tools`: both link biscuit-terminal, and `ci-build` is on the
/// critical path of every scope calculation, so gating it at `local-tools`
/// would make resolving a plan compile sniff's duckdb and gix closure.
#[test]
fn the_planner_facing_binaries_are_gated_below_local_tools() {
    let manifest = read("scripts/Cargo.toml");
    let bin = |name: &str| {
        manifest
            .split("[[bin]]")
            .find(|block| block.contains(&format!("name = \"{name}\"")))
            .unwrap_or_else(|| panic!("scripts/Cargo.toml must declare the {name} bin"))
            .to_owned()
    };
    for name in ["ci-plan", "ci-build"] {
        assert!(
            bin(name).contains("required-features = [\"build-tools\"]"),
            "{name} links biscuit-terminal and must be build-tools gated"
        );
    }
    assert!(
        manifest.contains("build-tools = [\"dep:biscuit-hash\", \"dep:biscuit-terminal\", \"dep:object\", \"dep:find-msvc-tools\"]"),
        "build-tools includes the archive inspector and native linker discovery without the local-tools closure"
    );
    assert!(
        !bin("ci-rollup").contains("required-features"),
        "the always-runs merge-gate binary links none of the monorepo's crates"
    );
    let recipe = read("just/ci-local.just");
    assert!(
        !recipe.contains("cargo build") && !recipe.contains("cargo run"),
        "`--plan` is a pre-trigger review and must not start a build"
    );
}

/// AC14: the reader-facing CI documentation states the implemented behavior and
/// no longer states the behavior this fix retired.
///
/// ## Notes
///
/// This guard exists because two claims in the `os` skill went false silently:
/// "the current verifier/calculator supports only one excluded environment per
/// run" and "the proposed scope-only mode … is not implemented yet". Both were
/// true when written and both survived the changes that falsified them, because
/// prose has no compiler. The retired phrases below are matched literally, so a
/// future change that reintroduces one fails here rather than misleading a
/// reader into skipping a required environment.
///
/// Deliberately narrow: it asserts the claims this fix moved, not doc style.
#[test]
fn the_ci_documentation_states_the_implemented_behavior() {
    /// (file, phrase, why it is wrong now)
    const RETIRED: &[(&str, &str, &str)] = &[
        (
            ".claude/skills/os/SKILL.md",
            "only one excluded environment",
            "evidence is verified and omitted per cell across every environment's notes ref",
        ),
        (
            ".claude/skills/os/SKILL.md",
            "is not implemented yet",
            "scope-only is implemented; `off` is its deprecated alias",
        ),
        (
            ".claude/skills/rust-devops/ci-cd.md",
            "reverse dependency receives compile-check only",
            "an unchanged reverse dependent receives no area, job, or cell",
        ),
        (
            "docs/topics/ci-cd.md",
            "direct reverse dependencies for compile-check only",
            "an unchanged reverse dependent receives no area, job, or cell",
        ),
        (
            "docs/topics/ci-cd.md",
            "direct reverse dependencies receive only the Windows compile-check",
            "a check cell exists only where example or bench targets are declared",
        ),
        (
            ".github/ci/README.md",
            "Phase 6 measures it against a real run",
            "the package-scoped cache key has not been measured against a real run",
        ),
        // fixes/2026-09-13-cicd-redundancies, Phase 10.
        (
            ".github/ci/README.md",
            "the advisory summary links",
            "the reporting job is `ci-reporting`; `summary` no longer exists",
        ),
        (
            ".github/ci/README.md",
            "the CI-tooling job",
            "CI's own suites are owned by `repo-deps` and `test-toolkit` and run in their cells",
        ),
        (
            "docs/topics/ci-cd.md",
            "advisory summary",
            "the reporting job is `ci-reporting`; `summary` no longer exists",
        ),
        (
            "docs/topics/ci-cd.md",
            "until Ken switches that context to",
            "`protect-your-bacon` has required `ci-gate` since 2026-09-13",
        ),
        (
            ".claude/skills/rust-devops/ci-cd.md",
            "ci.yml:summary",
            "the advisory job carrying `continue-on-error` is `ci.yml:ci-reporting`",
        ),
        (
            ".claude/skills/rust-devops/ci-cd.md",
            "still names the retired",
            "`protect-your-bacon` has required `ci-gate` since 2026-09-13",
        ),
        // `build` and `area-drift` arrived with the single-OS-compile fix and
        // `ci-gate` folds both; see `TARGET_CI_JOBS` and `GATED_JOBS`.
        (
            ".github/ci/README.md",
            "exactly six top-level jobs",
            "`ci.yml` defines eight top-level jobs",
        ),
        (
            ".github/ci/README.md",
            "fold of the four above",
            "`ci-gate` folds six jobs, including `build` and `area-drift`",
        ),
        (
            ".github/ci/README.md",
            "small contract-test leg",
            "CI's own suites are owned by `repo-deps` and `test-toolkit` and run in their cells",
        ),
        (
            "docs/topics/ci-cd.md",
            "exactly six top-level jobs",
            "`ci.yml` defines eight top-level jobs",
        ),
        (
            "docs/topics/ci-cd.md",
            "fold of those four",
            "`ci-gate` folds six jobs, including `build` and `area-drift`",
        ),
        (
            ".claude/skills/rust-devops/ci-cd.md",
            "exactly six top-level jobs",
            "`ci.yml` defines eight top-level jobs",
        ),
    ];
    for (file, phrase, why) in RETIRED {
        let source = read(file);
        assert!(
            !source.contains(phrase),
            "{file} still claims {phrase:?}; {why}"
        );
    }

    // The live facts a reader must be able to find. Each is load-bearing: a
    // reader who misses it either reruns a proven environment, or trusts an
    // area-keyed identity that does not exist.
    const REQUIRED: &[(&str, &str)] = &[
        (".github/ci/README.md", "Package is the stored identity"),
        ("docs/topics/ci-cd.md", "Area groups; package identifies"),
        ("docs/topics/ci-cd.md", "BISCUIT_CI_CONSTRAINTS_DIR"),
        ("docs/topics/ci-cd.md", "ACCEPTED GAP"),
        ("docs/topics/ci-cd.md", "compile_coverage_from"),
        ("CLAUDE.md", "BISCUIT_CI_CONSTRAINTS_DIR"),
        ("CLAUDE.md", "Area groups, package identifies"),
        (".claude/skills/rust-devops/ci-cd.md", "ACCEPTED GAP"),
        (".claude/skills/os/SKILL.md", "BISCUIT_CI_CONSTRAINTS_DIR"),
        (".claude/skills/os/ci-runners.md", "gate-input identity"),
        // fixes/2026-09-13-cicd-redundancies, Phase 10.
        (".github/ci/README.md", "ci-reporting"),
        (".github/ci/README.md", "change_inventory"),
        (".github/ci/README.md", "SUITE_REGISTRY"),
        ("docs/topics/ci-cd.md", "ci-reporting"),
        ("docs/topics/ci-cd.md", "SUITE_REGISTRY"),
        ("docs/topics/ci-cd.md", "change inventory"),
        ("docs/topics/ci-cd.md", "exactly eight top-level jobs"),
        (".github/ci/README.md", "exactly eight top-level jobs"),
        (".claude/skills/rust-devops/ci-cd.md", "exactly eight top-level jobs"),
        (".claude/skills/rust-devops/ci-cd.md", "ci-reporting"),
        (".claude/skills/rust-devops/ci-cd.md", "change_inventory"),
        (".claude/skills/rust-testing/SKILL.md", "repo-deps"),
        (
            ".claude/skills/os/windows.md",
            "Attaching a console inside a nextest process",
        ),
        ("docs/dependencies.md", "root Cargo workspace member"),
    ];
    for (file, phrase) in REQUIRED {
        let source = read(file);
        assert!(
            source.contains(phrase),
            "{file} must state {phrase:?} — see fixes/2026-09-11-cicd-cleanup/spec.md AC14"
        );
    }

    // The gate is `ci-gate`, a policy-free fold. Documentation that still
    // presents `ci-verdict` as the live required check sends a reader to a job
    // that no longer exists, so both files consulted before touching branch
    // protection must name the fold and must not claim the old job gates.
    for file in [
        ".claude/skills/rust-devops/ci-cd.md",
        "docs/topics/ci-cd.md",
        ".github/ci/README.md",
        "CLAUDE.md",
    ] {
        let source = read(file);
        assert!(
            source.contains("ci-gate") && !source.contains("still the single required context"),
            "{file} must describe `ci-gate` as the required check"
        );
    }
}

// ---------------------------------------------------------------------------
// Native build owners and archive consumers
// (fixes/2026-09-12-single-os-compile, Phase 4)
// ---------------------------------------------------------------------------

/// The four workflows that make up one `ci` run.
const CI_CHAIN: [&str; 4] = ["ci.yml", "_area-ci.yml", "_package-ci.yml", "_wsl-ci.yml"];

/// Every `actions/checkout` in the CI chain, as
/// `(workflow, job header, the `ref:` line it pins)`. An unpinned checkout
/// yields an empty ref, which is the failure the contract below reports.
fn ci_chain_checkouts() -> Vec<(&'static str, String, String)> {
    let mut found = Vec::new();
    for file in CI_CHAIN {
        let source = workflow(file);
        for job in jobs(&source) {
            let header = job.lines().next().unwrap_or_default().trim().to_owned();
            let lines: Vec<&str> = job.lines().collect();
            for (index, line) in lines.iter().enumerate() {
                if !line.contains("uses: actions/checkout@") {
                    continue;
                }
                let mut pinned = String::new();
                for next in lines.iter().skip(index + 1) {
                    let trimmed = next.trim();
                    if let Some(rest) = trimmed.strip_prefix("ref:") {
                        pinned = rest.trim().to_owned();
                        break;
                    }
                    // The step's own `with:` block, and nothing past it.
                    if trimmed.starts_with("- ") || trimmed.is_empty() {
                        break;
                    }
                }
                found.push((file, header.clone(), pinned));
            }
        }
    }
    found
}

/// One immutable revision, from scope through every archive consumer.
///
/// `pull_request` checks out the MERGE branch by default while the plan is
/// labelled with the pull-request head, and `ci-build` refuses any checkout
/// that is not `plan.head`. Left unpinned, the owner could not produce and no
/// consumer could verify. So: the two jobs that run before a plan exists read
/// the event expression, everything after reads the plan's own `head`, and the
/// reusable workflows are TOLD it rather than recomputing it.
#[test]
fn every_ci_checkout_pins_the_one_revision_the_plan_names() {
    let scope_sourced = "${{ needs.scope.outputs.head }}";
    let passed_down = "${{ inputs.tested-revision }}";
    let before_the_plan = "${{ env.TESTED_REVISION }}";

    let checkouts = ci_chain_checkouts();
    assert!(
        checkouts.len() >= 12,
        "the chain's checkouts must all be found, got {}",
        checkouts.len()
    );
    for (file, header, pinned) in &checkouts {
        let allowed: &[&str] = match (*file, header.as_str()) {
            // No plan exists yet: `validation` decides whether to compute one
            // and `scope` is the job that does.
            ("ci.yml", "validation:" | "scope:") => &[before_the_plan],
            ("ci.yml", _) => &[scope_sourced],
            _ => &[passed_down],
        };
        assert!(
            allowed.contains(&pinned.as_str()),
            "{file} `{header}` checks out `{pinned}`; it must be one of {allowed:?} \
             or `ci-build` refuses the tree as build-source-mismatch"
        );
    }

    // The one definition site, and the only place the event is read.
    let ci = workflow("ci.yml");
    assert!(
        ci.contains("TESTED_REVISION: ${{ github.event.pull_request.head.sha || github.sha }}"),
        "ci.yml must define the pre-plan revision once, as the pull-request head \
         with a push/dispatch fallback"
    );
    for file in CI_CHAIN.iter().skip(1) {
        assert!(
            !workflow(file).contains("github.event.pull_request.head.sha"),
            "{file} must be told the tested revision, not recompute it"
        );
    }

    // And the scope job publishes it from the PLAN, so the document the
    // producer is bound to is the document every later checkout follows.
    let scope = job_block("ci.yml", "  scope:");
    assert!(
        scope.contains("head: ${{ steps.scope.outputs.head }}"),
        "the scope job must publish the tested revision as an output"
    );
    assert!(
        scope.contains(r#"echo "head=$(jq -r '.head' resolved-plan.json)""#),
        "the published revision must be read back from the written plan"
    );
}

/// The revision is an input every called workflow must be given, so a caller
/// that forgets it fails at parse instead of silently testing the merge ref.
#[test]
fn every_called_workflow_requires_the_tested_revision() {
    for file in CI_CHAIN.iter().skip(1) {
        let source = workflow(file);
        let inputs = source
            .split_once("\njobs:\n")
            .expect("every workflow declares a jobs: section")
            .0;
        let body = inputs
            .split_once("\n      tested-revision:\n")
            .unwrap_or_else(|| panic!("{file} must declare a `tested-revision` input"))
            .1;
        // The input's own keys are indented deeper than its name; the next
        // sibling input ends it.
        let declaration: String = body
            .lines()
            .take_while(|line| line.starts_with("        "))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            declaration.contains("required: true"),
            "{file}'s `tested-revision` must be required, got:\n{declaration}"
        );
    }
    // Every call site hands it on, unchanged.
    for (file, job) in [
        ("ci.yml", "  area-ci:"),
        ("_area-ci.yml", "  package-ci:"),
        ("_package-ci.yml", "  wsl2:"),
    ] {
        let block = job_block(file, job);
        assert!(
            block.contains("tested-revision: ${{ needs.scope.outputs.head }}")
                || block.contains("tested-revision: ${{ inputs.tested-revision }}"),
            "{file} `{job}` must pass the tested revision down the chain"
        );
    }
}

/// The owner job schedules nothing of its own: one leg per producer
/// environment, its matrix and runner taken from the resolved plan's own
/// projections, and that producer's whole record set built in one invocation.
#[test]
fn the_build_owner_expands_one_leg_per_producer() {
    let build = job_block("ci.yml", "  build:");
    assert!(
        build.contains("needs: [scope, preflight]"),
        "the owner must wait for the plan that resolved its records"
    );
    assert!(
        build.contains("needs.scope.outputs.has_builds == 'true'"),
        "an all-reused plan resolves no record and must schedule no owner — and a \
         SKIPPED scope job leaves the outputs empty, which an emptiness test \
         against `'[]'` would read as `there are builds`"
    );
    assert!(
        build.contains("fail-fast: false"),
        "one failed key must not cancel an unrelated one"
    );
    assert!(
        build.contains("build: ${{ fromJSON(needs.scope.outputs.build_producers) }}"),
        "the matrix must be the plan's own owner projection"
    );
    assert!(
        build.contains("runs-on: ${{ fromJSON(needs.scope.outputs.build_owner_runners)[matrix.build] }}"),
        "the runner must come from the projection, never from a hardcoded label"
    );
    assert!(
        build.contains("run: bash scripts/ci/produce-owner.sh")
            && !build.contains("--key"),
        "a leg must produce all of its owner's records in one invocation"
    );
    // The plan is the scheduling authority: the owner reads its projection and
    // recalculates nothing.
    let executable = executable_lines(&build);
    assert!(
        !executable.contains("affected_scope.py"),
        "the owner must not reopen affected scope"
    );
}

/// A build artifact is plumbing keyed by `{package, producer, key}`, retained
/// for this run alone, and its status exists whatever the archive did.
#[test]
fn every_build_publishes_a_keyed_artifact_and_a_status_for_any_outcome() {
    let build = job_block("ci.yml", "  build:");
    let build_steps = steps(&build);
    let publish = step_named(&build_steps, "Publish package builds and statuses")
        .expect("the owner must publish every package's archive and status");
    assert!(publish.contains("if: ${{ always() }}"));
    assert!(publish.contains("scripts/ci/artifacts/publish.cjs"));
    // A build never creates a result cell: no `status-<package>-<tier>-<env>`
    // artifact, no JUnit, no manifest record.
    let executable = executable_lines(&build);
    for forbidden in ["name: status-", "name: junit-", "manifest.jsonl"] {
        assert!(
            !executable.contains(forbidden),
            "the owner must not publish `{forbidden}`: a build is not a result cell"
        );
    }
}

/// Every archive consumer verifies before it extracts, and compiles nothing.
#[test]
fn an_archive_consumer_verifies_first_and_never_reaches_a_compiler() {
    let header = "  test:";
    let job = job_block("_package-ci.yml", header);
    let all = steps(&job);
    // The build reference is the PLAN's, resolved by the one reader. A record
    // the plan does not carry refuses the row before anything is downloaded.
    assert!(
        job.contains("name: ${{ steps.cell.outputs.build_artifact }}"),
        "{header}: the cell must download the build its contract named"
    );
    assert!(
        read("scripts/ci/cell_contract.py").contains("cell-contract-build"),
        "a cell naming a build the plan does not carry must refuse, not compile"
    );

    let index = |name: &str| {
        all.iter()
            .position(|step| step.contains(name))
            .unwrap_or_else(|| panic!("{header} must define the `{name}` step"))
    };
    assert!(
        index("Download this cell's build") < index("Verify this cell's build"),
        "{header}: nothing may be verified before it is downloaded"
    );
    assert!(
        index("Verify this cell's build") < index("List this cell's expected tests"),
        "{header}: listing runs the archived binaries, so it follows verification too"
    );
    assert!(
        index("Verify this cell's build") < index("- name: Tests"),
        "{header}: verification must precede the tier, not follow it"
    );

    // The Cargo cache belongs to a cell that compiles, and a test tier no
    // longer is one: Task 6.5 deleted it with the compile-in-place path it
    // served. A restored cache here would be the one thing that can make
    // a silent rebuild look fast.
    assert!(
        !job.contains("Swatinem/rust-cache@v2"),
        "{header}: a Cargo cache belongs to a cell that compiles; this one consumes an archive"
    );
    // So does a toolchain, with one exception: a `requires-toolchain`
    // package's cell installs the pin for its SUITE to drive, gated on the
    // plan's own answer and never on a runner label
    // (`a_declared_toolchain_requirement_is_provisioned_once_before_the_suite_runs`).
    let toolchain_steps: Vec<&String> = all
        .iter()
        .filter(|step| step.contains("rustup") || step.contains("Rust toolchain"))
        .collect();
    assert_eq!(
        toolchain_steps.len(),
        1,
        "{header}: exactly one step may install a toolchain, and only for the suite's own use"
    );
    assert!(
        toolchain_steps[0].contains("if: ${{ steps.cell.outputs.requires_toolchain == 'true' }}"),
        "{header}: the toolchain step must be gated on the plan's answer"
    );

    // The gate itself passes the verified archive and remaps the workspace;
    // nothing in that path is a Cargo build flag. So does the listing that
    // decides what the gate owed.
    for id in ["gate", "expected"] {
        let step = step_with_id(&all, id)
            .unwrap_or_else(|| panic!("{header} must carry the `{id}` step"));
        assert!(
            step.contains("--archive-file \"$ARCHIVE_FILE\"")
                && step.contains("--workspace-remap \"$ARCHIVE_WORKSPACE\""),
            "{header}: `{id}` must name the verified archive and this checkout"
        );
        for compiler in ["cargo build", "cargo check", "cargo clippy", "rustup", "cargo test"] {
            assert!(
                !step.contains(compiler),
                "{header}: an archive consumer must not invoke `{compiler}`"
            );
        }
    }
}

/// An artifact that is missing, corrupt, or refused must stop the cell — never
/// silently start a build that produces a second, unverified set of binaries.
#[test]
fn an_archive_miss_never_enters_a_fallback_build() {
    for (file, header) in [("_package-ci.yml", "  test:"), ("_wsl-ci.yml", "  wsl:")] {
        let job = job_block(file, header);
        for name in ["Download this cell's build", "Verify this cell's build", "Verify the build in the guest"] {
            if let Some(step) = step_named(&steps(&job), name) {
                assert!(
                    !step.contains("continue-on-error"),
                    "{file} {header}: `{name}` may not be ignored — a refused build has no \
                     replacement a consumer is allowed to compile"
                );
            }
        }
    }
    // The verifier the consumer runs is the one the producer shipped with the
    // archive, so a guest with no Cargo can still refuse a bad one.
    let build = job_block("ci.yml", "  build:");
    assert!(
        build.contains("verifier: path.join(root, 'target/release/ci-build'"),
        "the producer must ship the verifier inside the artifact it produced"
    );
}

/// A cell the plan names no build for refuses, rather than compiling in place.
///
/// Until Task 6.5 `_ci_build_consumer` answered `archive=` (empty) and the tier
/// fell back to a local compile. Nothing can fall back now: every executing
/// L1/L2/browser cell references exactly one build record, so an empty answer
/// is a disagreement between the plan and the workflow — and on a
/// toolchain-free consumer the fallback would not even fail honestly.
///
/// Driven through the shipped recipe, because the refusal is the recipe's
/// behavior and a string match would pass against a recipe that printed the
/// message and carried on.
#[test]
fn a_cell_with_no_build_record_refuses_instead_of_compiling_in_place() {
    let recipe = |builds: &str, environment: &str| {
        std::process::Command::new("just")
            .arg("--justfile")
            .arg(repo_root().join("justfile"))
            .arg("--working-directory")
            .arg(repo_root())
            .arg("_ci_build_consumer")
            .arg(builds)
            .arg(environment)
            .arg("L1")
            .output()
            .expect("just must be present: it is this repository's canonical runner")
    };
    let builds = r#"[{"key":"0f1e2d3c4b5a6978","package":"alpha","producer":"ubuntu-latest",
        "artifact":"build-alpha-ubuntu-latest-0f1e2d3c4b5a6978",
        "consumers":[{"environment":"ubuntu-latest","gate":"L1"}]}]"#;

    let matched = recipe(builds, "ubuntu-latest");
    assert!(
        matched.status.success(),
        "a cell the plan names a build for must resolve it: {}",
        String::from_utf8_lossy(&matched.stderr)
    );

    let unmatched = recipe(builds, "macos-latest");
    assert!(
        !unmatched.status.success(),
        "a cell with no record must refuse, not answer 'compile in place'"
    );
    let stderr = String::from_utf8_lossy(&unmatched.stderr);
    assert!(
        stderr.contains("names no build for it")
            && stderr.contains("will not compile a replacement"),
        "the refusal must say what is missing and that nothing will be rebuilt: {stderr}"
    );
    assert!(
        !String::from_utf8_lossy(&unmatched.stdout).contains("compiling in place"),
        "the retired fallback message must be gone with the fallback"
    );
}

/// The specification's central claim, as a property of the shipped workflows:
/// ONE Linux build, two environments, two result cells.
#[test]
fn the_wsl2_guest_consumes_the_same_build_as_native_linux() {
    let wsl = workflow("_wsl-ci.yml");
    assert_eq!(
        jobs(&wsl).len(),
        1,
        "the package-local WSL2 archive producer is retired: the guest consumes \
         the run's Linux build"
    );
    // Both legs resolve their build through the SAME reader, against the same
    // plan, by their own cell key — which is what makes "one build, two cells"
    // a property rather than a coincidence of two matching jq expressions.
    let guest = job_block("_wsl-ci.yml", "  wsl:");
    let native = job_block("_package-ci.yml", "  test:");
    for (name, block) in [("the guest", &guest), ("native Linux", &native)] {
        assert!(
            block.contains("python3 scripts/ci/cell_contract.py"),
            "{name} must resolve its build record from the plan"
        );
        assert!(
            block.contains("name: ${{ steps.cell.outputs.build_artifact }}"),
            "{name} must download the artifact the plan named, not one of its own"
        );
    }
    // Two cells, one build: the guest keeps its own JUnit and status identity,
    // derived from `wsl2-ubuntu` and never from its Windows runner label.
    for artifact in [
        "name: ${{ steps.cell.outputs.junit_artifact }}",
        "name: ${{ steps.cell.outputs.status_artifact }}",
    ] {
        assert!(wsl.contains(artifact), "the guest must keep `{artifact}`");
    }
    assert!(
        guest.contains("BISCUIT_CI_ENVIRONMENT: wsl2-ubuntu"),
        "the guest's results must never merge into the native Windows cell"
    );
}

/// The consumers report what they ran, so a reader can confirm the claim.
#[test]
fn every_archive_consumer_reports_its_planned_key_and_realized_digest() {
    let package = workflow("_package-ci.yml");
    assert_eq!(
        package
            .matches("BUILD_KEY: ${{ steps.cell.outputs.build_key }}")
            .count(),
        1,
        "the native test status writer must record the key it ran"
    );
    assert_eq!(
        package
            .matches("BUILD_DIGEST: ${{ steps.verified.outputs.digest }}")
            .count(),
        1,
        "and the digest its verification realized"
    );
    let wsl = workflow("_wsl-ci.yml");
    assert!(
        wsl.contains("BUILD_KEY: ${{ steps.cell.outputs.build_key }}")
            && wsl.contains("BUILD_DIGEST: ${{ steps.manifest.outputs.digest }}"),
        "the guest must report the same pair; its manifest is read on the host \
         because `wsl-bash` cannot write a Windows $GITHUB_OUTPUT"
    );
}

/// A failed owner leg must block its own consumers and nothing else.
#[test]
fn the_area_fan_out_waits_for_the_owner_without_being_cancelled_by_it() {
    let area = job_block("ci.yml", "  area-ci:");
    assert!(
        area.contains("needs: [scope, preflight, build]"),
        "a consumer must not start before its archive exists"
    );
    assert!(
        area.contains("!cancelled()"),
        "one failed owner leg must not skip every area"
    );
    let gate = job_block("ci.yml", "  ci-gate:");
    assert!(
        gate.contains("      - build\n") && gate.contains("build:${{ needs.build.result }}"),
        "an owner failure is a blocking infrastructure failure of the run"
    );
}

/// Archive mode reaches Cargo for nothing at all — not even `cargo metadata`.
///
/// `_stage_junit` asks `cargo metadata` for the workspace root and target
/// directory when the overrides are unset, and `_backend_proof` builds its
/// checker with `cargo run`. A hosted Linux consumer happens to have Cargo, so
/// both would silently succeed there and fail only in the toolchain-free guest.
/// One spelling for both: the overrides, and the sidecar the producer built.
#[test]
fn an_archive_consumer_asks_cargo_for_nothing_including_metadata() {
    let header = "  test:";
    let job = job_block("_package-ci.yml", header);
    let job_steps = steps(&job);
    let gate = step_with_id(&job_steps, "gate").expect("the gate command exists");
    for knob in [
        "export BISCUIT_NEXTEST_BIN='cargo-nextest nextest'",
        "export BISCUIT_JUNIT_WORKSPACE_ROOT=\"$ARCHIVE_WORKSPACE\"",
        "export BISCUIT_JUNIT_TARGET_DIR=\"$ARCHIVE_WORKSPACE/target\"",
        "export INSTA_WORKSPACE_ROOT=\"$ARCHIVE_WORKSPACE\"",
    ] {
        assert!(
            gate.contains(knob),
            "{header}: archive mode must set `{knob}` so no consumer path reaches Cargo"
        );
    }
    // The expected-test listing runs the same archived binaries, so it needs
    // the same knobs; `insta` is the one exception — a listing renders no
    // snapshot.
    let expected = step_with_id(&job_steps, "expected").expect("the listing step exists");
    for knob in [
        "export BISCUIT_NEXTEST_BIN='cargo-nextest nextest'",
        "export BISCUIT_JUNIT_WORKSPACE_ROOT=\"$ARCHIVE_WORKSPACE\"",
        "export BISCUIT_JUNIT_TARGET_DIR=\"$ARCHIVE_WORKSPACE/target\"",
    ] {
        assert!(
            expected.contains(knob),
            "{header}: the listing must set `{knob}` so it reaches Cargo for nothing either"
        );
    }
    // The L2 execution proof is a producer-built sidecar, not a `cargo run`.
    assert!(
        job.contains("BISCUIT_BACKEND_PROOF_BIN=$proof"),
        "the L2 consumer must take its backend-proof checker from the verified sidecars"
    );
    let devops = read("just/devops.just");
    assert!(
        devops.contains("if [[ -n \"${BISCUIT_BACKEND_PROOF_BIN:-}\" ]]; then"),
        "`_backend_proof` must prefer the sidecar over building the checker"
    );
    let sidecars = read(".github/ci/sidecars.json");
    for name in ["backend-proof", "harness-broker"] {
        assert!(
            sidecars.contains(&format!("\"{name}\"")),
            "`{name}` must be a declared build sidecar"
        );
    }
}

// ---------------------------------------------------------------------------
// macOS and native-Windows owners
// (fixes/2026-09-12-single-os-compile, Phase 5)
// ---------------------------------------------------------------------------

/// Every native producer owns the archive its consumers execute, and nothing
/// in the table can say otherwise.
///
/// Until Task 6.5 this was `archive_cutover`, a per-producer boolean that
/// narrowed the owner matrix while a producer's consumers still compiled in
/// place. The field is gone with those paths: ownership is now exactly
/// "declares an `executes` list", so a producer that somehow stopped owning its
/// archive would have to stop declaring what it executes — which
/// `load_environments` refuses outright.
#[test]
fn every_native_producer_owns_the_archive_its_consumers_execute() {
    let table: serde_json::Value =
        serde_json::from_str(&read(".github/ci/environments.json")).expect("the table is JSON");
    let environments = table["environments"]
        .as_array()
        .expect("the table lists environments");
    let mut producers = Vec::new();
    for environment in environments {
        let build = &environment["build"];
        let name = environment["name"].as_str().expect("an environment is named");
        assert!(
            build.get("archive_cutover").is_none(),
            "{name} still carries the retired `archive_cutover` migration switch"
        );
        // A producer is the environment that declares what it executes; the
        // archive-only guest declares only the predicates it is checked against.
        if build.get("executes").is_none() {
            continue;
        }
        producers.push(name.to_owned());
    }
    assert_eq!(
        producers,
        ["ubuntu-latest", "windows-latest", "macos-latest"],
        "the three native producers, in the order the shipped table declares them"
    );
}

/// The owner leg is one job for all three producers, not three OS branches.
///
/// `runs-on` comes from the plan's projection and the record's own native
/// prerequisites are installed through the shared recipe, so adding a producer
/// is a data change. The one place `runner.os` may appear is the executable
/// suffix on the staged verifier — a file NAME, never a compatibility claim.
#[test]
fn the_owner_leg_is_one_job_across_macos_linux_and_windows() {
    let build = job_block("ci.yml", "  build:");
    let executable = executable_lines(&build);
    assert!(
        executable.contains("runs-on: ${{ fromJSON(needs.scope.outputs.build_owner_runners)[matrix.build] }}"),
        "the producer's runner is the plan's, never a hardcoded or branched label"
    );
    for os in ["ubuntu-latest", "macos-latest", "windows-latest"] {
        assert!(
            !executable.contains(&format!("runs-on: {os}")),
            "the owner job must not name `{os}` directly"
        );
    }
    // Compatibility is proven by the toolchain, in `ci-build produce`, not
    // inferred from the label that selected the leg.
    let archive = read("scripts/ci-build-archive.rs");
    assert!(
        archive.contains("fn preflight_toolchain(")
            && archive.contains("BuildStatus::of(record, \"failure\", \"preflight\")"),
        "the producer must prove its own compiler host against the plan and report a \
         `preflight` refusal when it does not match"
    );
    // Every `runner.os` in the owner job is an executable suffix.
    for line in executable.lines().filter(|line| line.contains("runner.os")) {
        assert!(
            line.contains("'.exe'"),
            "the owner job may read `runner.os` only for an executable suffix: {line}"
        );
    }
    // One native-prerequisite path for all three, from the record's own closure.
    assert!(
        executable.contains("just _ensure-native-libs \"${native_args[@]}\""),
        "the owner installs the record's declared native prerequisites through the \
         shared recipe, which is what resolves the per-OS installer"
    );
}

/// An archive consumer spells every path the way its own OS does.
///
/// Git Bash answers `/d/a/repo` for `$PWD`, and `$RUNNER_TEMP` is a Windows
/// path even inside it. Both would have reached `--workspace-remap`, insta, the
/// JUnit staging root, and the verifier as spellings no native program can
/// open — and only on Windows, which is where nothing else in this plan had
/// run until now.
#[test]
fn an_archive_consumer_hands_native_programs_native_paths() {
    for (header, id) in [("  test:", "gate"), ("  test:", "expected")] {
        let job = job_block("_package-ci.yml", header);
        let job_steps = steps(&job);
        let step = step_with_id(&job_steps, id)
            .unwrap_or_else(|| panic!("{header} must carry the `{id}` step"));
        assert!(
            step.contains("ARCHIVE_WORKSPACE: ${{ steps.verified.outputs.workspace }}"),
            "{header}: the gate must take the workspace spelling from verification, \
             which computes it once for every consumer"
        );
        for msys in ["--workspace-remap \"$PWD\"", "INSTA_WORKSPACE_ROOT=\"$PWD\""] {
            assert!(
                !step.contains(msys),
                "{header}: `{msys}` is an MSYS path on a Windows runner"
            );
        }
    }
    let devops = read("just/devops.just");
    let verify = devops
        .split("_ci_build_verify dir artifact environment plan=\"\":")
        .nth(1)
        .expect("`_ci_build_verify` exists");
    for normalized in [
        "dir=\"$(just _native_path '{{ dir }}')\"",
        "workspace=\"$(just _native_path \"$PWD\")\"",
    ] {
        assert!(
            verify.contains(normalized),
            "`_ci_build_verify` must normalize its paths: expected `{normalized}`"
        );
    }
    assert!(
        devops.contains("cygpath -m --"),
        "`_native_path` answers the drive-qualified, forward-slash spelling — the one \
         form both MSYS and Win32 accept, and the one with no verbatim prefix"
    );
}

/// The Windows consumer finds its verified tools under their own file names.
///
/// A sidecar resolved from `PATH` needs nothing — `PATHEXT` finds the `.exe`.
/// A sidecar bound to a VARIABLE is an exact file name, and a consumer that
/// probed only the Unix spelling would silently leave the binding empty and
/// send the test back to the fallback the archive exists to remove.
#[test]
fn every_consumer_resolves_its_bound_sidecars_under_their_windows_names() {
    let header = "  test:";
    let job = job_block("_package-ci.yml", header);
    let job_steps = steps(&job);
    let step = step_named(&job_steps, "Provision the verified build sidecars")
        .unwrap_or_else(|| panic!("{header} must provision its verified sidecars"));
    assert!(
        step.contains("echo \"$sidecars\" >>\"$GITHUB_PATH\""),
        "{header}: the verified sidecar directory must reach PATH"
    );
    for tool in ["stub_dunstify", "biscuit-harness-broker", "backend-proof"] {
        assert!(
            step.contains(&format!("{tool}.exe")),
            "{header}: the `{tool}` binding must also be looked for under its \
             Windows name"
        );
    }
    // One provisioning step now serves all three tiers, so the bindings are
    // unconditional — a browser row simply binds nothing it does not declare.
    assert_eq!(
        1,
        job_steps
            .iter()
            .filter(|step| step.contains("Provision the verified build sidecars"))
            .count(),
        "{header}: one sidecar provisioning step serves every native test row"
    );
}

/// No compile-in-place path survives in a test tier.
///
/// Task 6.5 deleted them all — the two legacy runner-tool prebuilds, the
/// toolchain setup, and the Cargo cache — once every producer owned its
/// archive. A tier that grew one back would rebuild the very tools the
/// producer already shipped it, and on a toolchain-free consumer it would not
/// even fail honestly: the build would simply not be there.
///
/// The one toolchain a tier may still install is not a compile path: a
/// `requires-toolchain` package's L1 cell provisions the pin for its suite to
/// drive, gated on the planner's `toolchain-environments`, and the recipe
/// still compiles nothing with it.
#[test]
fn no_test_tier_carries_a_compile_in_place_path() {
    for header in ["  test:"] {
        let job = job_block("_package-ci.yml", header);
        for retired in [
            "Build messenger desktop stubs",
            "Build the darkmatter md fixture",
            "!steps.build.outputs.archive",
            "Swatinem/rust-cache@v2",
        ] {
            assert!(
                !job.contains(retired),
                "{header}: `{retired}` is a compile-in-place path and this job consumes an archive"
            );
        }
        for step in steps(&job).iter().filter(|step| step.contains("rustup")) {
            assert!(
                step.contains("if: ${{ steps.cell.outputs.requires_toolchain == 'true' }}"),
                "{header}: an ungated toolchain install is a compile-in-place path:\n{step}"
            );
        }
        // The one Cargo invocation a tier may still make, and only under the
        // opt-in measurement dispatch: a consumer's compiler-work count must be
        // ZERO, which is the cleanest proof the cutover worked, and the tool
        // that counts it has to exist to say so.
        for line in executable_lines(&job).lines() {
            let line = line.trim();
            if !line.contains("cargo ") {
                continue;
            }
            assert!(
                line.contains("just _ci_build_counter") || line.contains("cargo-nextest"),
                "{header}: `{line}` is a compiler invocation in an archive consumer"
            );
        }
    }
}

/// macOS keeps the runtime provisioning an archive cannot carry.
///
/// A build record ships compile-time outputs. tmux is a runtime facility, so
/// its provisioning is deliberately NOT conditioned on the archive — and it
/// must still fail loudly, on the environment that declared the capability,
/// rather than let a tier report zero executed tests as a pass.
#[test]
fn the_l2_consumer_still_provisions_and_proves_its_runtime_backend() {
    let l2 = job_block("_package-ci.yml", "  test:");
    let l2_steps = steps(&l2);
    let tmux = step_named(&l2_steps, "Provision and verify the tmux backend")
        .expect("the L2 rows provision the one CI-hostable backend");
    assert!(
        tmux.contains("if: ${{ matrix.gate == 'L2' }}"),
        "tmux belongs to the L2 rows and to no other tier sharing this job"
    );
    assert!(
        !tmux.contains("steps.cell.outputs.build_artifact"),
        "tmux is a RUNTIME facility: an archive consumer needs it exactly as much as \
         a cell that compiled in place"
    );
    for environment in ["ubuntu-latest", "macos-latest"] {
        assert!(
            tmux.contains(&format!("{environment})")),
            "the tmux provisioning must cover `{environment}`"
        );
    }
    assert!(
        tmux.contains("tmux -V"),
        "a declared backend that cannot start must fail its own named step"
    );
    assert!(
        l2.contains("apple-terminal") && l2.contains("its tests skip"),
        "the macOS backend coverage row must still name the GUI emulators that skip"
    );
}

/// The certify step reads the backend-proof document from the stage tree the
/// tier wrote it into — the same tree that holds the expected manifest and the
/// JUnit reports it already reads.
///
/// `backend-proof verify` (bracketed by `_test_l2`) writes
/// `backend-proofs.json` beside `backend-executions.jsonl` in
/// `$BISCUIT_JUNIT_STAGE_DIR`, which defaults to `target/nextest/ci-reports`
/// under the workspace root. Without this flag `completion.py` reads an empty
/// proof map and refuses every L2 cell as `completion-backend-unproven`.
#[test]
fn the_certify_step_reads_backend_proofs_from_the_same_stage_tree() {
    let native = job_block("_package-ci.yml", "  test:");
    let native_steps = steps(&native);
    let certify = step_named(&native_steps, "Certify this cell's completeness")
        .expect("the native test job certifies its cell");
    for flag in [
        r#"--expected-manifest "target/nextest/ci-reports/expected-${{ matrix.gate }}.json""#,
        "--artifacts target/nextest/ci-reports",
        "--backend-proofs target/nextest/ci-reports/backend-proofs.json",
    ] {
        assert!(
            certify.contains(flag),
            "the native certify step must pass `{flag}` so every input comes from one stage tree"
        );
    }

    let wsl = workflow("_wsl-ci.yml");
    let wsl_steps = steps(&wsl);
    let certify = step_named(&wsl_steps, "Certify this cell's completeness")
        .expect("the guest certifies its cell");
    for flag in [
        r#"--artifacts "$stage""#,
        r#"--backend-proofs "$stage/backend-proofs.json""#,
    ] {
        assert!(
            certify.contains(flag),
            "the guest certify step must pass `{flag}`; the two certify steps stay parallel"
        );
    }
}

/// Lint and check compile on purpose, and their work is counted as its own
/// configuration rather than read as a duplicate of the archive.
///
/// Spec section 6: Clippy is another compiler driver and check-only target
/// kinds may emit no executable, so neither consumes an archive. What Phase 5
/// adds is the accounting — `{package, environment, gate}` measurement
/// documents for `lint` and `check` — so a post-cutover measurement shows one
/// archive plus two named configurations, not an unexplained third compile.
#[test]
fn the_lint_and_check_gates_are_measured_as_their_own_configurations() {
    for (header, gate, label, environment) in [
        (
            "  check:",
            "check",
            "check ${{ matrix.environment }}",
            "${{ matrix.environment }}",
        ),
        // Lint expands a row set of its own now, so it takes its environment
        // from the row exactly as check does — one compiler driver, one cell.
        (
            "  lint:",
            "lint",
            "lint ${{ matrix.environment }}",
            "${{ matrix.environment }}",
        ),
    ] {
        let job = job_block("_package-ci.yml", header);
        let job_steps = steps(&job);
        // They still compile, with their own feature and target selection.
        assert!(
            job.contains("Set up the pinned Rust toolchain") && job.contains("rust-cache@v2"),
            "{header}: a compile gate keeps its toolchain and its cache"
        );
        assert!(
            !job.contains("steps.cell.outputs.build_artifact"),
            "{header}: a lint or check cell references no build record"
        );
        let counter = step_named(&job_steps, "Prepare the compiler-work counter")
            .unwrap_or_else(|| panic!("{header} must prepare the compiler-work counter"));
        assert!(
            counter.contains(&format!("just _ci_build_counter \"{label}\"")),
            "{header}: the counter must be labelled `{label}`"
        );
        assert!(
            job.contains(&format!("BISCUIT_CI_BUILD_CONFIGURATION: {label}")),
            "{header}: the measured command must name its own configuration"
        );
        let report = step_named(&job_steps, "Report compiler work")
            .unwrap_or_else(|| panic!("{header} must report its compiler work"));
        assert!(
            report.contains(&format!("\"{environment}\" {gate}")),
            "{header}: the measurement is keyed `{{package, environment, {gate}}}`"
        );
        assert!(
            report.contains("continue-on-error: true"),
            "{header}: a broken measurement must never turn a green gate red"
        );
    }
}

/// The owner job blocks the merge for its own infrastructure failure only.
///
/// A compile failure is already a cell: the rollup renders every dependent cell
/// MISSING and names the build record. What no cell can show is an owner that
/// never reached a compile at all — a lost runner, a failed upload, a
/// cancellation — so the owner sits in the fold as an infrastructure job. It
/// produces no cell of its own, and `continue-on-error` would hide it.
#[test]
fn the_owner_job_reaches_the_gate_as_infrastructure_and_owns_no_cell() {
    let build = job_block("ci.yml", "  build:");
    assert!(
        !build.contains("continue-on-error"),
        "an advisory owner would fold into `ci-gate` as success and hide a lost archive"
    );
    let gate = job_block("ci.yml", "  ci-gate:");
    assert!(
        gate.contains("      - build\n") && gate.contains("build:${{ needs.build.result }}"),
        "the owner's own result must reach the policy-free fold"
    );
    // The cell outcomes stay where they were: in the package jobs and their
    // area's rollup. The owner publishes neither.
    let executable = executable_lines(&build);
    for forbidden in ["name: status-", "name: junit-", "checks:"] {
        assert!(
            !executable.contains(forbidden),
            "the owner must not carry `{forbidden}`"
        );
    }
    let area = job_block("ci.yml", "  area-ci:");
    assert!(
        area.contains("needs: [scope, preflight, build]") && area.contains("!cancelled()"),
        "the area fan-out waits for the owner without one failed leg cancelling the rest"
    );
}

// --- the reporting contract: every stage measured where it happened ----------

/// Every archive-consuming tier, as `(file, job header, gate step id)`.
const ARCHIVE_CONSUMERS: [(&str, &str, &str); 2] = [
    ("_package-ci.yml", "  test:", "gate"),
    ("_wsl-ci.yml", "  wsl:", "l1"),
];

/// Transfer, verification, extraction, and execution are four separate numbers.
///
/// The specification's reporting contract requires download, extraction, and
/// test execution to be reported apart, so a measurement cannot hide setup cost
/// inside test time. An artifact download is an action rather than a command,
/// so its window can only be observed between two steps — which is what makes
/// the transfer marker part of the contract rather than an implementation
/// detail of one tier.
#[test]
fn every_archive_consumer_reports_its_stages_separately_in_its_status() {
    for (file, header, gate) in ARCHIVE_CONSUMERS {
        let block = job_block(file, header);
        let name = header.trim();
        assert!(
            block.contains("id: transfer"),
            "{file}: `{name}` must mark when its build transfer started"
        );
        for field in [
            "download_seconds",
            "verify_seconds",
            "extract_ms",
            "execute_seconds",
        ] {
            assert!(
                block.contains(field),
                "{file}: `{name}` must report `{field}` apart from the others"
            );
        }
        assert!(
            block.contains("timings: $timings"),
            "{file}: `{name}` must carry its stage timings in the status document \
             the rollup reads"
        );
        assert!(
            block.contains("BUILD_KEY:") && block.contains("BUILD_DIGEST:"),
            "{file}: `{name}` must display the planned key and realized digest it ran"
        );
        let _ = gate;
    }
}

/// The hosted tiers close the transfer window with the verifier itself.
#[test]
fn a_hosted_consumer_hands_its_transfer_clock_to_the_verifier() {
    for (file, header, _) in ARCHIVE_CONSUMERS
        .iter()
        .filter(|(file, ..)| *file == "_package-ci.yml")
    {
        let block = job_block(file, header);
        assert!(
            block.contains("BISCUIT_CI_TRANSFER_STARTED_EPOCH: ${{ steps.transfer.outputs.epoch }}"),
            "{file}: `{}` must give `_ci_build_verify` the epoch its download began",
            header.trim()
        );
    }
}

/// The guest cannot write `$GITHUB_OUTPUT`, so its measurements cross the mount.
#[test]
fn the_wsl_guest_carries_its_own_measurements_back_to_the_host() {
    let block = job_block("_wsl-ci.yml", "  wsl:");
    for artifact in [
        "wsl-timing/verify.seconds",
        "wsl-timing/l1.seconds",
        "wsl-timing/verdict.json",
    ] {
        assert!(
            block.contains(artifact),
            "the guest must leave `{artifact}` where the host's status step can read it"
        );
    }
    assert!(
        block.contains("download_seconds=$(( $(date +%s) - "),
        "the host closes the transfer window: the guest never sees the download"
    );
}

/// Queueing closes before the producer tool starts and the upload opens after
/// it exits, so neither can be measured by `ci-build` and both must be observed
/// by the owner job.
#[test]
fn the_owner_job_reports_its_queue_and_upload_windows() {
    let build = job_block("ci.yml", "  build:");
    assert!(
        build.contains("id: job_start"),
        "the owner must record when it started, or its queue time is unknowable"
    );
    assert!(
        build.contains("PLAN_EPOCH: ${{ needs.scope.outputs.plan_epoch }}"),
        "queue time is measured from the moment the plan that names this leg existed"
    );
    let publisher = std::fs::read_to_string(repo_root().join("scripts/ci/artifacts/publish.cjs")).unwrap();
    // Whole seconds: the rollup reads `ProducerStageSeconds` as integers and a
    // fractional window failed every area's audit on run 35412170320.
    assert!(publisher.contains("upload_seconds: Math.round((Date.now() - start) / 1000)"));
    assert!(publisher.contains("status.stage_seconds.queue_seconds = queueSeconds"));
    let scope = job_block("ci.yml", "  scope:");
    assert!(
        scope.contains("plan_epoch: ${{ steps.plan_published.outputs.epoch }}"),
        "the scope job publishes the moment an owner became schedulable"
    );
}

/// Every reader-facing document and recipe, swept for the identities this fix
/// retired.
///
/// ## Notes
///
/// `the_ci_documentation_states_the_implemented_behavior` matches named phrases
/// in a fixed file list, which is the right shape for a claim that became false.
/// It cannot catch the other failure mode: a *new* document, or one nobody
/// thought to list, telling a reader to look for a job, flag, recipe, or
/// workflow that does not exist. So this one globs instead — the same reason
/// `areas_json_is_gone_and_has_no_readers` globs.
///
/// Historical records under `fixes/` and `features/` are deliberately out of
/// scope: they describe what was true when they were written and are the only
/// place these names may survive.
#[test]
fn no_reader_facing_document_or_recipe_names_a_retired_ci_entity() {
    /// (retired identity, what owns that work now)
    const RETIRED: &[(&str, &str)] = &[
        (
            "ci-tooling",
            "CI's own suites are owned by `repo-deps` and `test-toolkit`",
        ),
        (
            "ci_tooling",
            "the `flags.ci_tooling` boolean is replaced by SUITE_OWNER_PREFIXES/PATHS",
        ),
        (
            "biscuit-tui-captured-stdout",
            "the test is ordinary L1 in `biscuit-tui-cli`'s own windows-latest cell",
        ),
        (
            "test-windows-captured-stdout",
            "the canonical `just test` recipe reaches the test",
        ),
        (
            "biscuit-tui-windows-captured-stdout",
            "the specialized workflow is deleted",
        ),
    ];

    let mut swept: Vec<PathBuf> = Vec::new();
    let mut collect_tree = |dir: &str| {
        let root = repo_root().join(dir);
        let mut pending = vec![root];
        while let Some(dir) = pending.pop() {
            for entry in
                fs::read_dir(&dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
            {
                let path = entry.expect("dir entry").path();
                if path.is_dir() {
                    pending.push(path);
                } else {
                    swept.push(path);
                }
            }
        }
    };
    collect_tree("docs");
    collect_tree(".github");
    collect_tree(".claude/skills");
    collect_tree("just");

    // Every justfile: the root one, each package area's, and each `tools/`
    // member's. A stale recipe comment is what this sweep found first.
    swept.push(repo_root().join("justfile"));
    for parent in [repo_root(), repo_root().join("tools")] {
        for entry in fs::read_dir(&parent).unwrap_or_else(|e| panic!("read {}: {e}", parent.display()))
        {
            let path = entry.expect("entry").path().join("justfile");
            if path.is_file() {
                swept.push(path);
            }
        }
    }
    // The one package README that documented why CI provisions Node here.
    swept.push(repo_root().join("tools/test-audit/README.md"));

    assert!(
        swept.len() > 60,
        "the sweep itself must be non-vacuous (found {} files)",
        swept.len()
    );

    let mut violations = Vec::new();
    for path in &swept {
        // Binary fixtures (images, archives) live under these trees too.
        let Ok(source) = fs::read_to_string(path) else {
            continue;
        };
        let source = source.replace("\r\n", "\n");
        for (retired, owner) in RETIRED {
            if source.contains(retired) {
                let relative = path.strip_prefix(repo_root()).unwrap_or(path);
                violations.push(format!("{} names {retired:?} — {owner}", relative.display()));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "reader-facing documentation names retired CI entities:\n  {}",
        violations.join("\n  ")
    );
}

// ---------------------------------------------------------------------------
// The post-surgery workflow graph — fixes/2026-09-13-cicd-redundancies
//
// Each fixture below entered as a `pending_contract` oracle and was promoted
// with Phase 8's surgery. They pin the SHAPE of the graph; the assertions
// above — `GATED_JOBS`, `ci_gate_is_the_single_required_check`,
// `retired_specialized_workflows_and_jobs_are_absent`, and the two
// suite-ownership tests — pin what each surviving job may do.
// ---------------------------------------------------------------------------

/// One job's block, or `None` when the workflow does not define it.
///
/// [`job_block`] panics with its own wording for a missing job; the caller
/// below needs to say which job is missing and why that matters instead.
fn optional_job_block(file: &str, header: &str) -> Option<String> {
    jobs(&workflow(file))
        .into_iter()
        .find(|job| job.starts_with(header))
}

/// Every top-level job id of `ci.yml`, in file order.
fn top_level_job_ids(file: &str) -> Vec<String> {
    jobs(&workflow(file))
        .iter()
        .map(|block| {
            block
                .lines()
                .next()
                .unwrap_or("")
                .trim()
                .trim_end_matches(':')
                .to_owned()
        })
        .collect()
}

/// `ci.yml`'s whole top-level job set. `ci-tooling` and
/// `biscuit-tui-captured-stdout` retired into their owners' package cells and
/// the advisory `summary` became `ci-reporting`; `build` and `area-drift`
/// arrived with the single-OS-compile fix.
const TARGET_CI_JOBS: [&str; 8] = [
    "validation",
    "scope",
    "preflight",
    "build",
    "area-ci",
    "area-drift",
    "ci-gate",
    "ci-reporting",
];

/// AC7: the retired jobs are gone and no new one appeared beside them.
#[test]
fn ci_defines_exactly_the_surviving_top_level_jobs() {
    let mut actual = top_level_job_ids("ci.yml");
    actual.sort_unstable();
    let mut expected = TARGET_CI_JOBS.map(str::to_owned).to_vec();
    expected.sort_unstable();
    assert_eq!(
        expected, actual,
        "ci.yml must define exactly {expected:?}, got {actual:?}"
    );
}

/// AC15/R11: the gate folds exactly [`GATED_JOBS`] — no more, no fewer.
#[test]
fn ci_gate_needs_exactly_the_surviving_blocking_jobs() {
    let gate = job_block("ci.yml", "  ci-gate:");
    let executable: String = gate
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");

    let needs: Vec<String> = executable
        .split_once("    needs:\n")
        .expect("ci-gate must declare a needs list")
        .1
        .lines()
        .take_while(|line| line.starts_with("      - "))
        .map(|line| line.trim_start_matches("      - ").trim().to_owned())
        .collect();
    let expected = GATED_JOBS;
    assert_eq!(
        expected.map(str::to_owned).to_vec(),
        needs,
        "ci-gate must fold exactly {expected:?}, got {needs:?}"
    );

    for retired in ["biscuit-tui-captured-stdout", "ci-tooling"] {
        assert!(
            !executable.contains(retired),
            "ci-gate must fold exactly the surviving jobs; its RESULTS \
             block still names {retired}"
        );
    }

    // Unchanged by this fix (spec D6): the accept clause and the
    // fixed name the `protect-your-bacon` ruleset requires.
    assert!(
        executable.contains("success|skipped)") && executable.contains("exit 1"),
        "ci-gate must still accept exactly `success` and `skipped`"
    );
    assert!(
        executable.contains("\n    name: ci-gate\n"),
        "the required context name is not this fix's to change"
    );
}

/// AC7: the dedicated Windows workflow retires with the job that called it.
#[test]
fn the_windows_captured_stdout_workflow_is_absent() {
    let path = repo_root()
        .join(".github/workflows/biscuit-tui-windows-captured-stdout.yml");
    assert!(
        !path.exists(),
        "biscuit-tui-windows-captured-stdout.yml must be absent once \
         the test is ordinary L1 evidence"
    );
    assert!(
        ORCHESTRATED.is_empty(),
        "the specialized-workflow inventory must be absent of retired \
         entries; it still names {:?}",
        ORCHESTRATED.map(|(name, _, _)| name)
    );

    // B4: retiring the caller orphans the flag that selected it. A
    // flag nothing reads is the shape this whole fix removes.
    let ci = workflow("ci.yml");
    assert!(
        !ci.contains("biscuit_tui"),
        "the `biscuit_tui` scope output must be absent once its only \
         consumer is gone"
    );
    assert!(
        !read("scripts/ci/affected_scope.py").contains("\"biscuit-tui\""),
        "the `biscuit-tui` area flag must be absent from the planner"
    );
}

/// AC12: the report is advisory, and nothing that blocks may be.
#[test]
fn ci_reporting_is_advisory_and_no_blocking_job_is() {
    let report = optional_job_block("ci.yml", "  ci-reporting:").unwrap_or_else(|| {
        panic!("ci-reporting must exist; `summary` has not been replaced")
    });
    assert!(
        report.contains("\n    if: always()\n"),
        "ci-reporting must run even when every producer failed"
    );
    assert!(
        report.contains("\n    continue-on-error: true\n"),
        "ci-reporting must be structurally advisory, not advisory by \
         virtue of its script always succeeding"
    );

    // R11: it is NOT folded. `continue-on-error` would convert its
    // failure to `success` in the fold anyway, so listing it would be
    // misleading rather than merely redundant. Comments are stripped first —
    // `ci-gate`'s `needs:` comment names the job it deliberately omits.
    let gate = job_block("ci.yml", "  ci-gate:");
    let gate_executable: String = gate
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !gate_executable.contains("ci-reporting"),
        "ci-gate must not fold the advisory report"
    );

    for id in TARGET_CI_JOBS {
        if id == "ci-reporting" {
            continue;
        }
        let block = job_block("ci.yml", &format!("  {id}:"));
        assert!(
            !block.contains("\n    continue-on-error:"),
            "ci-reporting must be the only advisory job; `{id}` carries \
             continue-on-error at job level"
        );
    }
}

/// Spec §6: the report waits for the whole run, `ci-gate` included, so the
/// account it writes describes a settled run rather than one in flight.
#[test]
fn ci_reporting_needs_exactly_the_four_documents_it_reports_on() {
    let report = job_block("ci.yml", "  ci-reporting:");
    let executable: String = report
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");

    let needs: Vec<String> = executable
        .split_once("    needs:\n")
        .expect("ci-reporting must declare a needs list")
        .1
        .lines()
        .take_while(|line| line.starts_with("      - "))
        .map(|line| line.trim_start_matches("      - ").trim().to_owned())
        .collect();
    let expected = ["validation", "scope", "area-ci", "ci-gate"];
    assert_eq!(
        expected.map(str::to_owned).to_vec(),
        needs,
        "ci-reporting must wait on exactly {expected:?}, got {needs:?}"
    );
}

/// Spec §6 / D7: the report aggregates through the shared typed model. Parsing
/// the raw JUnit artifacts here would be a second result model, free to
/// disagree with the area that produced them.
#[test]
fn ci_reporting_aggregates_through_ci_rollup_rather_than_reparsing_artifacts() {
    let report = job_block("ci.yml", "  ci-reporting:");
    assert!(
        report.contains("ci-rollup summarize"),
        "the successful-scope mode must fold the areas' own slices"
    );
    assert!(
        report.contains("--plan ci-artifacts/ci-resolved-plan/resolved-plan.json"),
        "the report's change inventory and dependency sets come from the plan"
    );
    assert!(
        report.contains("pattern: 'ci-results-*'"),
        "the report must read every area's result slice"
    );
    for reparse in ["junit-", "manifest.jsonl", "status-"] {
        assert!(
            !report.contains(reparse),
            "the advisory report must not build a second result model from \
             `{reparse}` artifacts"
        );
    }
}

/// AC16: an area whose cells were all satisfied by evidence still fans out, so
/// its slice still reaches the report.
///
/// This half is the workflow's: with every cell reused, `package-ci` runs no
/// leg, so the slice exists only because the coverage audit neither waits for a
/// successful producer nor conditions its upload on one. The planner's half —
/// that such an area is still in the fan-out at all — is
/// `test_affected_scope.py::AllReusedAreaFanOutTests`.
#[test]
fn an_all_reused_area_still_produces_its_result_slice() {
    let audit = job_block("_area-ci.yml", "  coverage-audit:");
    assert!(
        audit.contains("\n    if: always()\n"),
        "the coverage audit must run even when no package job executed"
    );
    let upload = audit
        .split_once("name: Upload this area's result slice")
        .expect("the coverage audit must upload its slice")
        .1;
    assert!(
        upload.trim_start().starts_with("if: always()"),
        "the slice upload must not be conditional on a package job having run"
    );
}

// ---------------------------------------------------------------------------
// Work-group 2.C — the Windows captured-stdout test becomes ordinary L1
// ---------------------------------------------------------------------------

/// AC8/AC9: nothing stands between the test and normal L1 discovery.
///
/// Asserted from this suite rather than from a Biscuit TUI test because the
/// subject is *discoverability* — a test that no recipe selects cannot assert
/// its own reachability, which is the failure mode
/// `features/2026-07-24-devops/ci-failure-inventory.md` records.
#[test]
fn the_windows_captured_stdout_test_is_discoverable_as_ordinary_l1() {
    let test = read("biscuit-tui/cli/tests/level2/windows_captured_stdout.rs");
    assert!(
        !test.contains("#[ignore"),
        "the test must be reachable by the canonical L1 recipe, but it \
         is still `#[ignore]`d"
    );
    assert!(
        !test.contains("sleep(Duration::from_millis("),
        "the test must be reachable by the canonical L1 recipe without \
         a fixed readiness sleep (AC9); bounded observation replaces it"
    );
    // The tier contract expresses "Windows-only" with the `cfg`, never
    // with a prefix or an attribute (spec D4).
    assert!(
        test.contains("#![cfg(windows)]"),
        "the test must be reachable by the canonical L1 recipe on \
         Windows and compiled out elsewhere; its `#![cfg(windows)]` is \
         gone"
    );

    let justfile = read("biscuit-tui/justfile");
    assert!(
        !justfile.contains("test-windows-captured-stdout"),
        "the test must be reachable by the canonical L1 recipe, so the \
         dedicated recipe that invoked it by exact name is gone"
    );
}

/// NOT pending: the starting advantage Phase 7 depends on.
///
/// `biscuit-tui-cli` already declares `features = ["terminal-tests"]` under
/// `[package.metadata.ci.tests]`, so the CI L1 cell already COMPILES this test
/// target. Removing that declaration would leave the test green-by-absence.
#[test]
fn the_biscuit_tui_cli_l1_cell_already_compiles_the_terminal_test_target() {
    let manifest = read("biscuit-tui/cli/Cargo.toml");
    let tests = manifest
        .split_once("[package.metadata.ci.tests]")
        .expect("biscuit-tui-cli must declare its CI test policy")
        .1;
    assert!(
        tests.contains(r#"features = ["terminal-tests"]"#),
        "the CI L1 cell must compile the terminal-tests target, or the Windows \
         captured-stdout test is absent rather than passing"
    );
}

// ---------------------------------------------------------------------------
// Direct cell execution (`features/2026-09-19-direct-cell-execution`)
//
// These entered as Phase 2 `pending_contract` oracles and were promoted in
// Phase 5, when the row-driven layout landed: every one of them now asserts
// the shipped shape, so a regression fails this suite rather than a decorator.
// ---------------------------------------------------------------------------

/// The input names a workflow declares that carry a LIST OF ENVIRONMENTS —
/// the interface this feature removes. `check-os` is the same interface under
/// a shorter name.
fn declared_environment_list_inputs(file: &str) -> Vec<String> {
    let source = workflow(file);
    let Some((_, inputs_section)) = source.split_once("\n    inputs:\n") else {
        return Vec::new();
    };
    let section = inputs_section
        .split_once("\n\n")
        .map(|(inputs, _)| inputs)
        .unwrap_or(inputs_section);
    section
        .lines()
        .filter_map(|line| line.strip_prefix("      "))
        .filter_map(|line| line.split(':').next())
        .filter(|name| name.ends_with("-environments") || *name == "check-os")
        .map(str::to_owned)
        .collect()
}

/// The row-expanding jobs of `_package-ci.yml`: those whose matrix comes from
/// one of the four row-set inputs the area call passes.
fn row_expanding_jobs() -> Vec<String> {
    let source = workflow("_package-ci.yml");
    let mut found = Vec::new();
    for block in jobs(&source) {
        let executable = executable_lines(&block);
        if ["test-rows", "check-rows", "lint-rows", "wsl-rows"]
            .iter()
            .any(|name| executable.contains(&format!("inputs.{name}")))
        {
            found.push(
                block
                    .lines()
                    .next()
                    .unwrap_or("")
                    .trim()
                    .trim_end_matches(':')
                    .to_owned(),
            );
        }
    }
    found
}

/// Every producer's `(file, job id, validation step, upload step)` for R10's
/// separate, validated-then-uploaded completion artifact.
///
/// The artifact NAME is derived by `cell_contract.py` from the cell key, so
/// what identifies the upload here is the step, not a literal prefix.
fn completion_upload_steps() -> Vec<(String, String, String, String)> {
    let mut found = Vec::new();
    for file in ["_package-ci.yml", "_wsl-ci.yml"] {
        for block in jobs(&workflow(file)) {
            let job_steps = steps(&block);
            let Some(upload) = step_named(&job_steps, "Upload this cell's completion record")
            else {
                continue;
            };
            let validation = step_named(&job_steps, "Certify this cell's completeness")
                .expect("a completion record is uploaded only where one was validated");
            found.push((
                file.to_owned(),
                block
                    .lines()
                    .next()
                    .unwrap_or("")
                    .trim()
                    .trim_end_matches(':')
                    .to_owned(),
                validation.clone(),
                upload.clone(),
            ));
        }
    }
    found
}

/// AC2 / migration step 2: `_area-ci.yml` calls the execution workflow ONCE
/// for its area, never once per package.
#[test]
fn the_area_workflow_calls_the_execution_workflow_once() {
        assert_eq!(
            vec!["package-ci".to_owned()],
            jobs_containing("_area-ci.yml", "uses: ./.github/workflows/_package-ci.yml"),
            "exactly one job may call the execution workflow"
        );
        let call = executable_lines(&job_block("_area-ci.yml", "  package-ci:"));
        assert!(
            !call.contains("strategy:"),
            "the per-package fan-out is gone: one call per area, no matrix"
        );
}

/// AC2: no reader-facing workflow accepts an independent environment list.
#[test]
fn no_reader_facing_workflow_declares_an_environment_list_input() {
        for file in READER_FACING_WORKFLOWS {
            let offenders = declared_environment_list_inputs(file);
            assert!(
                offenders.is_empty(),
                "{file} still declares the environment list inputs \
                 {offenders:?}; every consumer resolves its execution \
                 contract from the resolved plan by exact cell key"
            );
        }
}

/// Ruling R9 as amended in Phase 7 of `2026-09-19-direct-cell-execution`:
/// exactly one dispatch path per area. The plan contract admits only the
/// path the workflows implement, so no area can be scheduled onto a second
/// one; rollback is a revert of workflows, planner, and audit together rather
/// than a per-area allowlist that could leave them disagreeing.
#[test]
fn the_plan_admits_exactly_the_dispatch_path_the_workflows_implement() {
        let contract: serde_json::Value =
            serde_json::from_str(&read(".github/ci/schemas/contract.json"))
                .expect("contract.json is JSON");
        let admitted: Vec<&str> = contract["vocabulary"]["execution_paths"]
            .as_array()
            .expect("the contract lists its execution paths")
            .iter()
            .filter_map(serde_json::Value::as_str)
            .collect();
        assert_eq!(
            vec!["rows"],
            admitted,
            "the plan may only name a dispatch path a shipped workflow implements"
        );
        assert!(
            !row_expanding_jobs().is_empty(),
            "the admitted `rows` path has no row-expanding job to run it"
        );
        for file in READER_FACING_WORKFLOWS {
            assert!(
                declared_environment_list_inputs(file).is_empty(),
                "{file} declares an environment list beside the rows: an area \
                 would have two dispatch paths"
            );
            assert!(
                !executable_lines(&workflow(file)).contains("execution_path"),
                "{file} branches on `execution_path`, which implies a second path"
            );
        }
        assert!(
            !repo_root().join(".github/ci/direct-execution.json").exists(),
            "the retired per-area allowlist is back; nothing reads it"
        );
}

/// Phase 8 of `2026-09-19-direct-cell-execution` retired the scope document's
/// `matrix` and `area_matrix`. A reader that still asks for either gets
/// `null` from `jq`, which iterates to nothing: the gate loop would run no
/// gate and report green. So every shipped reader is swept for the lookup.
#[test]
fn no_shipped_reader_consumes_a_retired_environment_list_projection() {
    let mut readers: Vec<PathBuf> = vec![
        repo_root().join(".githooks/pre-push"),
        repo_root().join("justfile"),
    ];
    for dir in [".github/workflows", "just", "scripts/ci"] {
        for entry in fs::read_dir(repo_root().join(dir)).expect("read reader directory") {
            let path = entry.expect("dir entry").path();
            let name = path.file_name().and_then(|name| name.to_str()).unwrap_or("");
            // The suites assert the retirement, so they name the fields.
            if path.is_file() && !name.starts_with("test_") {
                readers.push(path);
            }
        }
    }
    assert!(readers.len() > 20, "the sweep must be non-vacuous ({} files)", readers.len());
    let lookups = [".area_matrix", ".matrix[", ".matrix |", r#"["matrix"]"#, r#"["area_matrix"]"#];
    let mut offenders = Vec::new();
    for path in &readers {
        let Ok(source) = fs::read_to_string(path) else {
            continue;
        };
        for lookup in lookups {
            if source.contains(lookup) {
                offenders.push(format!("{} reads `{lookup}`", path.display()));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "read the plan's package records and cells instead: {offenders:#?}"
    );
}

/// The `workflow_call` input names `file` declares, in declaration order.
fn workflow_call_inputs(file: &str) -> Vec<String> {
    let source = workflow(file);
    let mut lines = source
        .lines()
        .skip_while(|line| *line != "  workflow_call:")
        .skip_while(|line| *line != "    inputs:")
        .skip(1);
    let mut names = Vec::new();
    for line in lines.by_ref() {
        let indent = line.len() - line.trim_start().len();
        if line.trim().is_empty() || line.trim_start().starts_with('#') || indent > 6 {
            continue;
        }
        if indent < 6 {
            break;
        }
        names.push(line.trim().trim_end_matches(':').to_owned());
    }
    names
}

/// Phase 8 of `2026-09-19-direct-cell-execution`: the environment-list
/// interface is retired, so each reusable workflow's inputs are pinned
/// exactly rather than screened by name. A reintroduced `packages`, `gates`,
/// `wsl`, or `builds` input is a second description of what runs, and a
/// heuristic on `-environments` would not see it.
#[test]
fn the_reusable_workflows_accept_only_rows_and_run_scalars() {
    const EXPECTED: &[(&str, &[&str])] = &[
        (
            "_area-ci.yml",
            &["tested-revision", "area", "slug", "rows", "accepted-gaps", "measure-compiler-work"],
        ),
        (
            "_package-ci.yml",
            &[
                "tested-revision",
                "test-rows",
                "check-rows",
                "lint-rows",
                "wsl-rows",
                "measure-compiler-work",
            ],
        ),
        ("_wsl-ci.yml", &["tested-revision", "row", "distribution"]),
    ];
    for (file, expected) in EXPECTED {
        assert_eq!(
            workflow_call_inputs(file),
            *expected,
            "{file}'s inputs changed: a producer takes its rows and per-run \
             scalars only, and resolves everything else from the plan by cell key"
        );
    }
}

/// The row-expanding jobs exist, each behind its scalar guard, none labelled
/// with an expression GitHub would render raw on a whole-job skip.
#[test]
fn every_row_expanding_job_carries_a_scalar_guard_and_no_expression_name() {
        let rows = row_expanding_jobs();
        assert!(
            !rows.is_empty(),
            "no row-driven job expands a row-set input; the workflows are \
             still driven by environment lists"
        );
        for job_id in &rows {
            let block = job_block("_package-ci.yml", &format!("  {job_id}:"));
            let executable = executable_lines(&block);
            assert!(
                executable.contains("if:"),
                "{job_id} must carry the planner's scalar guard so an empty \
                 row set skips before matrix expansion"
            );
            if let Some(name) = job_name(&block) {
                assert!(
                    !name.contains("${{"),
                    "{job_id} is skippable and must carry no expression name: \
                     GitHub never evaluates the matrix context for a skipped job"
                );
            }
        }
}

/// R3's negative: expensive tiers no longer stage behind L1. One native test
/// job over L1, L2, and browser rows cannot express `needs:`-ordered staging,
/// and the specification forbids an L1 prerequisite that would suppress
/// required L2 or browser work after a test failure.
#[test]
fn no_test_row_job_stages_behind_another_test_row_job() {
        let source = workflow("_package-ci.yml");
        assert!(
            !source.contains("needs: test\n"),
            "the D4 staging (`needs: test`) is gone: no test row's job may \
             depend on another test job's success"
        );
        for block in jobs(&source) {
            let header = block.lines().next().unwrap_or("").trim().to_owned();
            let executable = executable_lines(&block);
            if !executable.contains("inputs.test-rows")
                && !executable.contains("inputs.wsl-rows")
            {
                continue;
            }
            assert!(
                !executable.contains("\n    needs:"),
                "{header}: a row-expanding test job must not wait on another \
                 test job"
            );
        }
}

/// R10 / AC5: every executing cell's producer uploads JUnit, status, AND a
/// completion artifact; the completion upload is written only after
/// validation succeeds.
#[test]
fn every_producer_uploads_a_completion_artifact() {
        let uploads = completion_upload_steps();
        assert_eq!(
            4,
            uploads.len(),
            "every executing cell's producer — check, test, lint, and the guest \
             — must certify itself; a green status without its completion \
             record must not be possible"
        );
        for (file, job_id, validation, upload) in &uploads {
            assert!(
                upload.contains("name: ${{ steps.cell.outputs.completion_artifact }}"),
                "{file}:{job_id} names its completion artifact from the cell reader"
            );
            assert!(
                validation.contains("scripts/ci/completion.py"),
                "{file}:{job_id} validates before it publishes"
            );
            // Unconditional, in both steps: a cell that did not prove itself
            // must not reach the upload at all, and `if: always()` here would
            // publish whatever an earlier attempt left behind.
            for (what, step) in [("validation", validation), ("upload", upload)] {
                assert!(
                    !step.contains("if:"),
                    "{file}:{job_id}: the completion {what} carries no status \
                     predicate — it runs when the gate ran, or not at all"
                );
            }
            let block = job_block(file, &format!("  {job_id}:"));
            let job_steps = steps(&block);
            let junit = step_named(&job_steps, "Upload JUnit")
                .or_else(|| step_named(&job_steps, "Upload wsl2-ubuntu L1 JUnit"));
            let status = step_named(&job_steps, "Upload producer status")
                .expect("every producer uploads its status");
            assert!(
                status.contains("if: ${{ always() }}"),
                "{file}:{job_id}: the status upload still publishes after a failure"
            );
            if let Some(junit) = junit {
                assert!(
                    junit.contains("!cancelled()"),
                    "{file}:{job_id}: the JUnit upload still publishes after a failure"
                );
            }
        }
}

/// AC5's upload-failure case: a REQUIRED upload failure fails the producer.
#[test]
fn a_failed_required_upload_fails_the_job() {
        let uploads = completion_upload_steps();
        assert!(
            !uploads.is_empty(),
            "the completion upload this contract judges does not exist"
        );
        for (file, job_id, validation, upload) in &uploads {
            // Scoped to the two steps that carry the proof. The measurement
            // steps in the same job stay advisory on purpose — a broken
            // instrument must never turn a green gate red — so a job-wide
            // check here would forbid the wrong thing.
            for (what, step) in [("validation", validation), ("upload", upload)] {
                assert!(
                    !step.contains("continue-on-error"),
                    "{file}:{job_id}: a failed required {what} fails the job; an \
                     unuploaded completion record would leave a green cell with \
                     no evidence behind it"
                );
            }
        }
}

/// AC5's cancellation case: failure-path publication stays best effort under
/// the existing cancellation rules — the diagnostic uploads keep their
/// non-success conditions while the completion upload stays required.
#[test]
fn cancellation_keeps_failure_path_publication_best_effort() {
        assert!(
            !completion_upload_steps().is_empty(),
            "the completion upload steps this contract judges do not exist"
        );
        for file in ["_package-ci.yml", "_wsl-ci.yml"] {
            for block in jobs(&workflow(file)) {
                let executable = executable_lines(&block);
                if executable.contains("name: ${{ steps.cell.outputs.junit_artifact }}") {
                    assert!(
                        executable.contains("!cancelled()") || executable.contains("always()"),
                        "{file}: the JUnit diagnostic upload keeps publishing \
                         after a failure or cancellation"
                    );
                }
            }
        }
}

/// AC3: an all-reused or gap-only area makes no execution call but keeps its
/// blocking audit, its slice, and its gap publisher.
#[test]
fn an_all_reused_area_skips_execution_but_keeps_its_audit_slice_and_publisher() {
        let call = executable_lines(&job_block("_area-ci.yml", "  package-ci:"));
        assert!(
            call.contains("if:"),
            "the execution call is guarded by the planner's row-set flags so \
             an all-reused or gap-only area makes no call at all"
        );
        assert!(
            call.contains("has_test_rows")
                || call.contains("has_check_rows")
                || call.contains("has_lint_rows")
                || call.contains("has_wsl_rows"),
            "the guard is the planner's scalar row-set flags"
        );

        // The area's own obligations survive the skipped call.
        let audit = executable_lines(&job_block("_area-ci.yml", "  coverage-audit:"));
        assert!(
            audit.contains("if: always()"),
            "the audit runs even when every producer was skipped"
        );
        assert!(
            audit.contains("ci-results-"),
            "the area's result slice is still uploaded"
        );
        let publisher = executable_lines(&job_block("_area-ci.yml", "  accepted-gaps:"));
        assert!(
            publisher.contains("publish_gaps.py"),
            "the neutral gap checks are still published"
        );
}

/// NOT pending: every matrix in the reader-facing workflows keeps
/// `fail-fast: false`, today and under the row-driven layout Phase 5 lands.
#[test]
fn every_matrix_in_the_reader_facing_workflows_keeps_fail_fast_false() {
    let mut checked = 0;
    for file in READER_FACING_WORKFLOWS {
        for block in jobs(&workflow(file)) {
            if !block.contains("\n    strategy:") {
                continue;
            }
            checked += 1;
            let header = block.lines().next().unwrap_or("").trim().to_owned();
            assert!(
                block.contains("fail-fast: false"),
                "{file}:{header} declares a matrix without fail-fast: false; one \
                 red cell must never cancel its unevidenced siblings"
            );
        }
    }
    assert!(
        checked > 0,
        "the reader-facing workflows carry matrices; this contract went vacuous"
    );
}
