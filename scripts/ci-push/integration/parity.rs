//! End-to-end parity with the retired shell suite (`test-pre-push.sh`): each
//! test before the last section is one of its `run_test` rows, named as the
//! parity inventory in the feature's implementation log assigns, and makes
//! that row's assertions across the same boundary — the real `ci-push`
//! binary, real Git (receipts read back as notes from the checkout and the
//! bare remote), the real evidence helpers under the real Python, and, for
//! the plan-fed rows, the real `ci-local` recipe run by the real `just`.
//! The fixture is [`crate::evidence_fixture`].
//!
//! The last section adds the binary-boundary cases the plan lists beside the
//! rows: stdin wiring, interpreter fallback, cancellation, and publication
//! failure.
//!
//! `ci-push` writes every diagnostic to stderr; the shell hook split them
//! between stdout and stderr, so output assertions read both streams.

use std::path::{Path, PathBuf};

use rstest::rstest;
use serde_json::{json, Value};

use crate::evidence_fixture::{
    cell_names, receipt_view, relabel, Evidence, Run, TestGate, GATE_PLAN_FILE, GATE_SCOPE_FILE,
    GITHUB_URL, PROHIBITION_REASON, REPORTS_DIRECTORY, ZERO,
};
use crate::fixture::install_tool;

/// A publishing fixture (`stage_publishing_tools`): the stub planner emits
/// `plan`, and the gate stages one passing `alpha` report.
fn publishing(plan: &str) -> Evidence {
    let fixture = Evidence::new();
    fixture.planner_plan(plan);
    fixture.set_tool("just", fixture.staging_gate(plan, false, 0));
    fixture
}

/// The scope receipt on HEAD binds `base`, HEAD, and HEAD's tree, carries a
/// plan naming the same pair, and projects `alpha`.
#[track_caller]
fn assert_scope_receipt_base(fixture: &Evidence, base: git2::Oid) {
    let head = fixture.head();
    let receipt = fixture
        .scope_receipt(head)
        .expect("a scope receipt on HEAD");
    let tree = fixture.git().find_commit(head).unwrap().tree_id();
    assert_eq!(receipt["base"], json!(base.to_string()), "{receipt:#}");
    assert_eq!(receipt["head"], json!(head.to_string()), "{receipt:#}");
    assert_eq!(receipt["tree"], json!(tree.to_string()), "{receipt:#}");
    assert_eq!(
        receipt["plan"]["base"],
        json!(base.to_string()),
        "{receipt:#}"
    );
    assert_eq!(
        receipt["plan"]["head"],
        json!(head.to_string()),
        "{receipt:#}"
    );
    assert_eq!(
        receipt["scope"]["packages"],
        json!(["alpha"]),
        "{receipt:#}"
    );
}

/// The remote's validation note on `head` records one complete `alpha/L1`
/// failure.
#[track_caller]
fn assert_remote_complete_failure(fixture: &Evidence, head: git2::Oid) {
    let note = fixture.remote_validation(head).unwrap_or_else(|| {
        panic!(
            "no {} note for {head} reached the remote",
            fixture.validation_ref()
        )
    });
    let view = receipt_view(&note);
    assert_eq!(view["head"], json!(head.to_string()), "{view:#}");
    let cells = view["cells"].as_array().unwrap();
    assert_eq!(cells.len(), 1, "{view:#}");
    assert_eq!(cells[0]["package"], json!("alpha"), "{view:#}");
    assert_eq!(cells[0]["gate"], json!("L1"), "{view:#}");
    assert_eq!(cells[0]["outcome"], json!("fail"), "{view:#}");
    assert_eq!(cells[0]["completion"], json!("complete"), "{view:#}");
}

/// No validation note exists, in the checkout or on the remote.
#[track_caller]
fn assert_no_validation_note(fixture: &Evidence) {
    let reference = fixture.validation_ref();
    assert!(
        !fixture.local_has_ref(&reference),
        "{reference} was written locally"
    );
    assert!(
        !fixture.remote_has_ref(&reference),
        "{reference} reached the remote"
    );
}

fn planned(fixture: &Evidence, text: &str) -> bool {
    fixture
        .planner_calls()
        .iter()
        .any(|call| call.contains(text))
}

#[track_caller]
fn assert_planned(fixture: &Evidence, text: &str) {
    assert!(
        planned(fixture, text),
        "no planner call contains {text:?}:\n{}",
        fixture.planner_log()
    );
}

/// Whether both paths exist and are one directory (`/var` is `/private/var`
/// on macOS). A removed temporary worktree is never the checkout.
fn same_path(left: &Path, right: &Path) -> bool {
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

fn gate_ran(fixture: &Evidence) -> bool {
    fixture.just_ran("pre-push")
}

fn append(path: &Path, text: &str) {
    let mut content = std::fs::read_to_string(path).unwrap();
    content.push_str(text);
    std::fs::write(path, content).unwrap();
}

/// The path printed after `prefix`, to the end of its line, without the
/// sentence's final period.
fn printed_path(run: &Run, prefix: &str) -> PathBuf {
    let text = run.all();
    let start = text
        .find(prefix)
        .unwrap_or_else(|| panic!("no {prefix:?} in:\n{run}"))
        + prefix.len();
    let line = text[start..].lines().next().unwrap();
    PathBuf::from(line.strip_suffix('.').unwrap_or(line))
}

// ---------- the real ci-local recipe (rows 1, 2, 69–72) ----------
//
// The recipe is a `#!/usr/bin/env bash` script; on native Windows the real
// `just` runs it under Git for Windows' Bash, as a push from there does.
//
// A hook push through the real recipe pays for both the hook's helpers and
// the recipe's, more than an L1 test may take on native Windows, so each
// question is asked at its own public boundary, and the boundaries are
// pinned to each other:
//
// - the recipe's gate and report contract, by running the real recipe as
//   `ci-push` starts the gates (`the_real_recipe_*`, through
//   [`Evidence::run_recipe`]); `the_gates_are_started_as_the_direct_recipe_runs_start_them`
//   pins that start to the hook's;
// - the hook's receipt and publication contract, with a gate that stages
//   what the real recipe stages ([`Evidence::model_gate`]); each modeled
//   outcome is one case of
//   `the_real_recipe_stages_the_modeled_outcome_of_a_fed_plan`;
// - the verifier's retry decision, over prior evidence recorded through the
//   real evidence helpers ([`Evidence::record_evidence`]), which
//   `seeded_evidence_is_what_a_hook_push_records` pins to a hook push.

/// `plan` with `package`'s L1 cell on `environment` as the hook's overlay
/// leaves a cell that verified prior evidence covers: reused from the
/// passing local evidence of `commit`, with its build pruned.
fn reused(plan: &str, package: &str, environment: &str, commit: &str) -> String {
    let mut plan: Value = serde_json::from_str(plan).unwrap();
    let cell = plan["cells"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|cell| {
            cell["package"] == package && cell["gate"] == "L1" && cell["environment"] == environment
        })
        .unwrap_or_else(|| panic!("no {package}/{environment}/L1 cell"));
    let object = cell.as_object_mut().unwrap();
    object.remove("build");
    object.remove("profile");
    object.insert("execution".into(), json!("reuse"));
    object.insert("origin".into(), json!("prior-local"));
    object.insert("state".into(), json!("reused"));
    object.insert(
        "evidence".into(),
        json!({
            "origin": "prior-local",
            "outcome": "pass",
            "package": package,
            "gate": "L1",
            "environment": environment,
            "evidence": { "commit": commit },
        }),
    );
    plan["builds"]
        .as_array_mut()
        .unwrap()
        .retain(|build| build["package"] != package);
    plan.to_string()
}

/// What the recipe reads of `package`'s L1 cell on `environment` to decide
/// whether it runs, is skipped, or is rerun, and to name its evidence.
fn as_the_recipe_reads(plan: &Value, package: &str, environment: &str) -> Value {
    let cell = plan["cells"]
        .as_array()
        .unwrap()
        .iter()
        .find(|cell| {
            cell["package"] == package && cell["gate"] == "L1" && cell["environment"] == environment
        })
        .unwrap_or_else(|| panic!("no {package}/{environment}/L1 cell in {plan:#}"));
    json!({
        "execution": cell["execution"],
        "state": cell["state"],
        "origin": cell["origin"],
        "outcome": cell["evidence"]["outcome"],
        "commit": cell["evidence"]["evidence"]["commit"],
        "build": cell["build"],
        "test_filter": cell["test_filter"],
    })
}

// The real recipe's gate and report contract.

/// Every modeled outcome a hook test stages is what the real recipe stages
/// for the same gates: the reports, the manifest, the interruption marker,
/// the exit status, and the plan written back. Both gates of the plan start,
/// the later one after a failure too, and the planner is asked only for
/// the projection of the fed plan.
#[rstest]
#[case::pass(TestGate::Pass, TestGate::Pass)]
#[case::complete_failure(TestGate::Fail, TestGate::Pass)]
#[case::malformed_manifest(TestGate::Pass, TestGate::MalformedManifest)]
#[case::exit_130(TestGate::Pass, TestGate::Exit130)]
#[case::recorded_143(TestGate::Pass, TestGate::Recorded143)]
fn the_real_recipe_stages_the_modeled_outcome_of_a_fed_plan(
    #[case] alpha: TestGate,
    #[case] beta: TestGate,
) {
    let fixture = Evidence::without_repository();
    fixture.stage_ci_local_harness(
        &biscuit_test_harness::manifest_dir!()
            .parent()
            .unwrap()
            .join("just/ci-local.just"),
    );
    let plan = relabel(include_str!(
        "../../ci/fixtures/pre-push/plans/plan-macos-two-packages.json"
    ));
    fixture.recipe_just(alpha, beta);
    fixture.add_stub("python3", fixture.interpreter_stub(&[]));

    let recipe = fixture.run_recipe(&plan, &fixture.tools.tool("python3"), &[]);

    assert_eq!(
        recipe.outcome,
        fixture.recipe_outcome(&[("alpha", alpha), ("beta", beta)]),
        "{}",
        recipe.run
    );
    assert_eq!(recipe.plan_out.as_deref(), Some(plan.as_str()));
    for gate in ["_lint alpha", "_test alpha", "_lint beta", "_test beta"] {
        assert!(fixture.just_ran(gate), "{gate}: {:?}", fixture.just_calls());
    }
    recipe.run.assert_says("run    alpha/L1");
    recipe.run.assert_says("run    beta/L1");
    if beta == TestGate::MalformedManifest {
        recipe
            .run
            .assert_says("cannot establish run completion from the staged manifest");
    }
    fixture.assert_ci_local_never_selected();
}

/// A cell the fed plan reuses is named and skipped, its lint still runs,
/// and only the cells that ran are staged.
#[test]
fn the_real_recipe_skips_a_cell_its_plan_reuses() {
    let fixture = Evidence::without_repository();
    fixture.stage_ci_local_harness(
        &biscuit_test_harness::manifest_dir!()
            .parent()
            .unwrap()
            .join("just/ci-local.just"),
    );
    let commit = "5f36f201bf45384d2889e34be0c22ce91b8fdc6a";
    let plan = reused(
        &relabel(include_str!(
            "../../ci/fixtures/pre-push/plans/plan-macos-two-packages.json"
        )),
        "alpha",
        fixture.environment,
        commit,
    );
    fixture.recipe_just(TestGate::Pass, TestGate::Pass);
    fixture.add_stub("python3", fixture.interpreter_stub(&[]));

    let recipe = fixture.run_recipe(&plan, &fixture.tools.tool("python3"), &[]);

    assert_eq!(
        recipe.outcome,
        fixture.recipe_outcome(&[("beta", TestGate::Pass)]),
        "{}",
        recipe.run
    );
    assert!(
        !fixture.just_ran("_test alpha"),
        "{:?}",
        fixture.just_calls()
    );
    assert!(fixture.just_ran("_test beta"), "{:?}", fixture.just_calls());
    assert!(
        fixture.just_ran("_lint alpha"),
        "{:?}",
        fixture.just_calls()
    );
    recipe.run.assert_says(&format!(
        "skip   alpha/L1  (reused: prior-local pass on {})",
        &commit[..9]
    ));
    recipe.run.assert_says("run    beta/L1");
    fixture.assert_ci_local_never_selected();
}

// The hook's receipt and publication contract, gated by the modeled recipe.

#[rstest]
#[case::malformed_manifest(TestGate::MalformedManifest)]
#[case::exit_130(TestGate::Exit130)]
#[case::recorded_143(TestGate::Recorded143)]
fn a_recipe_interruption_withholds_earlier_reports(#[case] beta: TestGate) {
    let fixture = Evidence::with_feature("pkg/beta/src/lib.rs", "beta lib");
    fixture.planner_plan(&relabel(include_str!(
        "../../ci/fixtures/pre-push/plans/plan-macos-two-packages.json"
    )));
    let outcome = fixture.recipe_outcome(&[("alpha", TestGate::Pass), ("beta", beta)]);
    fixture.model_gate(&outcome);

    let run = fixture.push_feature(Some("strict"));

    assert_ne!(run.code, 0, "an interrupted recipe passed:\n{run}");
    assert!(
        outcome.reports.contains_key("L1/alpha.xml"),
        "fixture error: the earlier gate staged no report"
    );
    run.assert_says("validation was interrupted");
    assert_no_validation_note(&fixture);
}

/// A failing gate through the recipe is recorded as complete evidence that
/// the next push reruns (see
/// `a_cell_whose_prior_evidence_is_a_failure_is_rerun_and_superseded`).
#[test]
fn a_recipe_gate_failure_records_a_complete_failure() {
    let fixture = Evidence::with_feature("pkg/beta/src/lib.rs", "beta lib");
    fixture.planner_plan(&relabel(include_str!(
        "../../ci/fixtures/pre-push/plans/plan-macos-two-packages.json"
    )));
    let outcome = fixture.recipe_outcome(&[("alpha", TestGate::Fail), ("beta", TestGate::Pass)]);
    fixture.model_gate(&outcome);
    let head = fixture.head();

    let run = fixture.push_feature(Some("warn"));

    run.assert_exit(0);
    let view = receipt_view(&fixture.local_validation(head).expect("the receipt"));
    assert_eq!(view["base"], json!(fixture.main.to_string()), "{view:#}");
    assert_eq!(cell_names(&view), ["alpha/L1", "beta/L1"]);
    assert_eq!(view["cells"][0]["outcome"], json!("fail"), "{view:#}");
    assert_eq!(
        view["cells"][0]["completion"],
        json!("complete"),
        "{view:#}"
    );
    fixture.assert_retained_reports(head, &outcome);
}

#[test]
fn a_stacked_pull_request_records_its_target_tip_as_the_receipt_base() {
    let (fixture, parent, child) = Evidence::stacked(false);
    fixture.planner_plan(&relabel(include_str!(
        "../../ci/fixtures/pre-push/plans/plan-macos-two-packages.json"
    )));
    let outcome = fixture.recipe_outcome(&[("alpha", TestGate::Pass), ("beta", TestGate::Pass)]);
    fixture.model_gate(&outcome);
    fixture.open_pull_requests(&[(12, "parent")]);
    let merge_base = fixture.git().merge_base(fixture.main, child).unwrap();
    assert_ne!(
        merge_base, parent,
        "fixture error: the merge base with main is the parent tip"
    );

    let run = fixture.push(
        Some("strict"),
        GITHUB_URL,
        &format!("refs/heads/child {child} refs/heads/child {ZERO}"),
    );

    run.assert_exit(0);
    run.assert_says("pull request #12 into parent");
    let view = receipt_view(
        &fixture
            .local_validation(child)
            .expect("the child's receipt"),
    );
    assert_eq!(view["base"], json!(parent.to_string()), "{view:#}");
    assert_eq!(cell_names(&view), ["alpha/L1", "beta/L1"]);
    let scope = fixture
        .scope_receipt(child)
        .expect("the child's scope receipt");
    assert_eq!(
        view["scope_identity"],
        json!(fixture.plan_identity(&scope)),
        "{view:#}"
    );
    fixture.assert_retained_reports(child, &outcome);
}

/// The prior evidence the follow-up tests seed through the real helpers is
/// the evidence a hook push of the same commit records: the same note,
/// byte for byte, and the same retained reports.
#[test]
fn seeded_evidence_is_what_a_hook_push_records() {
    let fixture = Evidence::with_feature("pkg/beta/src/lib.rs", "beta lib");
    fixture.planner_plan(&relabel(include_str!(
        "../../ci/fixtures/pre-push/plans/plan-macos-two-packages.json"
    )));
    let outcome = fixture.recipe_outcome(&[("alpha", TestGate::Pass), ("beta", TestGate::Pass)]);
    let head = fixture.head();
    fixture.record_evidence("pull_request", fixture.main, head, &outcome);
    let seeded = fixture.local_validation(head).expect("the seeded receipt");
    fixture.assert_retained_reports(head, &outcome);
    fixture.forget_validation_evidence();
    fixture.model_gate(&outcome);

    let run = fixture.push_feature(Some("strict"));

    run.assert_exit(0);
    run.assert_says("Running pre-push validation from the reviewed plan");
    let pushed = fixture.local_validation(head).expect("the hook's receipt");
    assert_eq!(seeded, pushed);
    let view = receipt_view(&pushed);
    assert_eq!(view["base"], json!(fixture.main.to_string()), "{view:#}");
    assert_eq!(cell_names(&view), ["alpha/L1", "beta/L1"]);
    fixture.assert_retained_reports(head, &outcome);
}

/// A documentation-only follow-up is reviewed against the prior evidence:
/// the gates are fed a plan that reuses the cell the evidence covers and
/// reruns the one it cannot vouch for, whose refusal the review shows.
/// What the recipe then runs is `the_real_recipe_skips_a_cell_its_plan_reuses`;
/// what is recorded is `a_docs_only_follow_up_records_only_the_cells_that_ran`.
#[test]
fn a_docs_only_follow_up_skips_the_cells_prior_evidence_covers() {
    let fixture = Evidence::with_feature("pkg/beta/src/lib.rs", "beta lib");
    fixture.planner_plan(&relabel(include_str!(
        "../../ci/fixtures/pre-push/plans/plan-macos-two-packages.json"
    )));
    let head_a = fixture.head();
    fixture.record_evidence(
        "pull_request",
        fixture.main,
        head_a,
        &fixture.recipe_outcome(&[("alpha", TestGate::Pass), ("beta", TestGate::Pass)]),
    );
    fixture.follow_up("README.md", "more documentation");
    fixture.capture_gate();

    let run = fixture.push_feature(Some("strict"));

    run.assert_exit(0);
    run.assert_says("Running pre-push validation from the reviewed plan");
    run.assert_says(&format!("gate-inputs-changed: beta/{}/L1", fixture.environment));
    assert_reuses_alpha_from(&fixture, head_a);
}

/// The plan the gates were fed reuses alpha's L1 cell from `commit`'s
/// passing evidence, exactly as `the_real_recipe_skips_a_cell_its_plan_reuses`
/// feeds the real recipe, and runs beta's.
#[track_caller]
fn assert_reuses_alpha_from(fixture: &Evidence, commit: git2::Oid) {
    let fed = fixture.fed_plan();
    let environment = fixture.environment;
    let expected: Value =
        serde_json::from_str(&reused(&fed.to_string(), "alpha", environment, &commit.to_string()))
            .unwrap();
    assert_eq!(
        as_the_recipe_reads(&fed, "alpha", environment),
        as_the_recipe_reads(&expected, "alpha", environment),
        "{fed:#}"
    );
    assert_eq!(
        as_the_recipe_reads(&fed, "beta", environment)["execution"],
        json!("execute"),
        "{fed:#}"
    );
}

/// The verifier's half of a documentation-only follow-up: it accepts the
/// prior pass of alpha and refuses beta's, and the follow-up's receipt then
/// holds only the cell that ran, against main.
#[test]
fn a_docs_only_follow_up_records_only_the_cells_that_ran() {
    let fixture = Evidence::with_feature("pkg/beta/src/lib.rs", "beta lib");
    fixture.planner_plan(&relabel(include_str!(
        "../../ci/fixtures/pre-push/plans/plan-macos-two-packages.json"
    )));
    let head_a = fixture.head();
    fixture.record_evidence(
        "pull_request",
        fixture.main,
        head_a,
        &fixture.recipe_outcome(&[("alpha", TestGate::Pass), ("beta", TestGate::Pass)]),
    );
    let head_b = fixture.follow_up("README.md", "more documentation");
    let plan_b = fixture.planned("pull_request", fixture.main, head_b);

    let verified = fixture.verify(&plan_b, fixture.main, head_b);

    let alpha: Vec<&Value> = verified
        .accepted
        .iter()
        .filter(|cell| cell["package"] == "alpha")
        .collect();
    assert_eq!(alpha.len(), 1, "{verified:#?}");
    assert_eq!(alpha[0]["outcome"], json!("pass"), "{verified:#?}");
    assert_eq!(
        alpha[0]["evidence"]["commit"],
        json!(head_a.to_string()),
        "{verified:#?}"
    );
    assert!(!verified.accepts("beta"), "{verified:#?}");
    verified.assert_rejects(&format!("gate-inputs-changed: beta/{}/L1", fixture.environment));

    // The recipe skipped alpha (`the_real_recipe_skips_a_cell_its_plan_reuses`).
    fixture.record_evidence(
        "pull_request",
        fixture.main,
        head_b,
        &fixture.recipe_outcome(&[("beta", TestGate::Pass)]),
    );
    let view = receipt_view(&fixture.local_validation(head_b).expect("B's receipt"));
    assert_eq!(view["base"], json!(fixture.main.to_string()), "{view:#}");
    assert_eq!(cell_names(&view), ["beta/L1"]);
}

/// A follow-up to a pull request whose target advanced on the remote is
/// reviewed against the advanced tip, and the prior evidence still covers
/// alpha. What that push publishes is
/// `an_advanced_non_main_target_publishes_exact_receipts`.
#[test]
fn an_advanced_non_main_target_reuses_prior_cells_and_publishes_exact_receipts() {
    let fixture = Evidence::with_feature("pkg/beta/src/lib.rs", "beta lib");
    fixture.planner_plan(&relabel(include_str!(
        "../../ci/fixtures/pre-push/plans/plan-macos-two-packages.json"
    )));
    fixture.push_refs(&["refs/heads/main:refs/heads/parent"]);
    fixture.open_pull_requests(&[(12, "parent")]);
    // What a push of the feature into `parent`, then at main's tip, recorded.
    let head_a = fixture.head();
    fixture.record_evidence(
        "pull_request",
        fixture.main,
        head_a,
        &fixture.recipe_outcome(&[("alpha", TestGate::Pass), ("beta", TestGate::Pass)]),
    );
    let advanced = fixture.advance_remote("parent", "README.md", "advanced target\n");
    let head = fixture.follow_up("docs.md", "head documentation");
    fixture.capture_gate();

    let run = fixture.push(Some("strict"), GITHUB_URL, &fixture.creating_feature_line());

    run.assert_exit(0);
    run.assert_says("pull request #12 into parent");
    run.assert_says(&format!(
        "Running pre-push validation from the reviewed plan of {} (base {})",
        &head.to_string()[..9],
        &advanced.to_string()[..9]
    ));
    assert_reuses_alpha_from(&fixture, head_a);
}

/// The receipts a push into an advanced non-main target publishes, locally
/// and on the remote, bind the advanced tip.
#[test]
fn an_advanced_non_main_target_publishes_exact_receipts() {
    let fixture = Evidence::with_feature("pkg/beta/src/lib.rs", "beta lib");
    fixture.planner_plan(&relabel(include_str!(
        "../../ci/fixtures/pre-push/plans/plan-macos-executing.json"
    )));
    fixture.push_refs(&["refs/heads/main:refs/heads/parent"]);
    fixture.open_pull_requests(&[(12, "parent")]);
    let advanced = fixture.advance_remote("parent", "README.md", "advanced target\n");
    let head = fixture.head();
    let outcome = fixture.recipe_outcome(&[("alpha", TestGate::Pass)]);
    fixture.model_gate(&outcome);

    let run = fixture.push(Some("strict"), GITHUB_URL, &fixture.creating_feature_line());

    run.assert_exit(0);
    let local = receipt_view(
        &fixture
            .local_validation(head)
            .expect("a receipt on the head"),
    );
    assert_eq!(local["base"], json!(advanced.to_string()), "{local:#}");
    assert_eq!(cell_names(&local), ["alpha/L1"]);
    let scope = fixture
        .remote_scope_receipt(head)
        .expect("the scope receipt on the remote");
    assert_eq!(scope["base"], json!(advanced.to_string()), "{scope:#}");
    let remote = receipt_view(
        &fixture
            .remote_validation(head)
            .expect("the receipt on the remote"),
    );
    assert_eq!(remote["base"], json!(advanced.to_string()), "{remote:#}");
    assert_eq!(remote["cells"][0]["outcome"], json!("pass"), "{remote:#}");
    assert_eq!(
        remote["scope_identity"],
        json!(fixture.plan_identity(&scope)),
        "{remote:#}"
    );
    fixture.assert_retained_reports(head, &outcome);
}

// The verifier's retry decision, over prior evidence the real helpers
// recorded.

#[test]
fn a_follow_up_changing_a_packages_inputs_reruns_its_cell() {
    let fixture = Evidence::with_feature("pkg/beta/src/lib.rs", "beta lib");
    fixture.planner_plan(&relabel(include_str!(
        "../../ci/fixtures/pre-push/plans/plan-macos-two-packages.json"
    )));
    fixture.record_evidence(
        "pull_request",
        fixture.main,
        fixture.head(),
        &fixture.recipe_outcome(&[("alpha", TestGate::Pass), ("beta", TestGate::Pass)]),
    );
    let head_b = fixture.follow_up("pkg/alpha/src/lib.rs", "changed alpha");
    let plan_b = fixture.planned("pull_request", fixture.main, head_b);

    let verified = fixture.verify(&plan_b, fixture.main, head_b);

    assert!(
        !verified.accepts("alpha"),
        "prior evidence of changed inputs was accepted: {verified:#?}"
    );
    verified.assert_rejects(&format!(
        "gate-inputs-changed: alpha/{}/L1",
        fixture.environment
    ));
}

/// A follow-up's verification refuses a cell whose prior evidence is a
/// complete failure, so the follow-up reruns it; that the rerun's pass then
/// supersedes the failure is
/// `a_rerun_cells_newest_pass_supersedes_its_prior_failure`.
#[test]
fn a_cell_whose_prior_evidence_is_a_failure_is_rerun_and_superseded() {
    let fixture = Evidence::with_feature("pkg/beta/src/lib.rs", "beta lib");
    fixture.planner_plan(&relabel(include_str!(
        "../../ci/fixtures/pre-push/plans/plan-macos-two-packages.json"
    )));
    let head_a = fixture.head();
    fixture.record_evidence(
        "pull_request",
        fixture.main,
        head_a,
        &fixture.recipe_outcome(&[("alpha", TestGate::Fail), ("beta", TestGate::Pass)]),
    );
    let view = receipt_view(&fixture.local_validation(head_a).expect("A's receipt"));
    assert_eq!(view["base"], json!(fixture.main.to_string()), "{view:#}");
    assert_eq!(cell_names(&view), ["alpha/L1", "beta/L1"]);
    assert_eq!(view["cells"][0]["outcome"], json!("fail"), "{view:#}");
    assert_eq!(
        view["cells"][0]["completion"],
        json!("complete"),
        "{view:#}"
    );
    let head_b = fixture.follow_up("README.md", "more documentation");
    let plan_b = fixture.planned("pull_request", fixture.main, head_b);

    let verified = fixture.verify(&plan_b, fixture.main, head_b);

    assert!(
        !verified.accepts("alpha"),
        "a failing prior cell was accepted: {verified:#?}"
    );
    verified.assert_rejects(&format!("failed-cell: alpha/{}/L1", fixture.environment));
}

/// After a follow-up reran a cell whose prior evidence was a failure and it
/// passed, verifying the follow-up's plan resolves the cell to the newest
/// evidence: the follow-up's pass.
#[test]
fn a_rerun_cells_newest_pass_supersedes_its_prior_failure() {
    let fixture = Evidence::with_feature("pkg/beta/src/lib.rs", "beta lib");
    fixture.planner_plan(&relabel(include_str!(
        "../../ci/fixtures/pre-push/plans/plan-macos-two-packages.json"
    )));
    let head_a = fixture.head();
    let head_b = fixture.follow_up("README.md", "more documentation");
    let failed = fixture.recipe_outcome(&[("alpha", TestGate::Fail), ("beta", TestGate::Pass)]);
    let passed = fixture.recipe_outcome(&[("alpha", TestGate::Pass), ("beta", TestGate::Pass)]);
    fixture.record_runs(&[
        ("pull_request", fixture.main, head_a, &failed),
        ("pull_request", fixture.main, head_b, &passed),
    ]);
    let view = receipt_view(&fixture.local_validation(head_b).expect("B's receipt"));
    assert_eq!(view["base"], json!(fixture.main.to_string()), "{view:#}");
    assert_eq!(cell_names(&view), ["alpha/L1", "beta/L1"]);

    let verified = fixture.verify(
        &fixture.planned("pull_request", fixture.main, head_b),
        fixture.main,
        head_b,
    );

    let alpha: Vec<&Value> = verified
        .accepted
        .iter()
        .filter(|cell| cell["package"] == "alpha")
        .collect();
    assert_eq!(alpha.len(), 1, "{verified:#?}");
    assert_eq!(alpha[0]["outcome"], json!("pass"), "{verified:#?}");
    assert_eq!(
        alpha[0]["evidence"]["commit"],
        json!(head_b.to_string()),
        "{verified:#?}"
    );
}

// Interpreter selection reaches the recipe (acceptance criteria 1–2).

/// `python3` is broken, and the interpreter `ci-push` would select instead
/// (`py -3` on Windows, `python` elsewhere) is handed to the real recipe as
/// `BISCUIT_CI_PYTHON` and its NUL-delimited prefix file: every helper the
/// recipe starts runs under it, prefix first, and none goes back to
/// `python3`.
#[test]
fn a_real_recipe_runs_its_helpers_under_the_selected_fallback_interpreter() {
    let fixture = Evidence::without_repository();
    fixture.stage_ci_local_harness(
        &biscuit_test_harness::manifest_dir!()
            .parent()
            .unwrap()
            .join("just/ci-local.just"),
    );
    let plan = relabel(include_str!(
        "../../ci/fixtures/pre-push/plans/plan-macos-two-packages.json"
    ));
    fixture.recipe_just(TestGate::Pass, TestGate::Pass);
    fixture.add_stub(
        "python3",
        json!({ "exit": 9, "stderr": "Python was not found\n" }),
    );
    let (fallback, prefix) = fallback_interpreter();
    fixture.add_stub(fallback, fixture.interpreter_stub(prefix));

    let recipe = fixture.run_recipe(&plan, &fixture.tools.tool(fallback), prefix);

    assert_eq!(
        recipe.outcome,
        fixture.recipe_outcome(&[("alpha", TestGate::Pass), ("beta", TestGate::Pass)]),
        "{}",
        recipe.run
    );
    assert!(fixture.just_ran("_test alpha"), "{:?}", fixture.just_calls());
    assert!(fixture.just_ran("_test beta"), "{:?}", fixture.just_calls());
    let invocations = fixture.tools.invocations();
    assert!(
        !invocations
            .iter()
            .any(|invocation| invocation["tool"] == "python3"),
        "the recipe went back to python3: {invocations:#?}"
    );
    let calls: Vec<Vec<String>> = invocations
        .iter()
        .filter(|invocation| invocation["tool"] == fallback)
        .map(|invocation| serde_json::from_value(invocation["argv"].clone()).unwrap())
        .collect();
    for call in &calls {
        assert_eq!(call[..prefix.len()], *prefix, "{calls:#?}");
    }
    let helpers: Vec<String> = calls
        .iter()
        .map(|call| {
            call.iter()
                .skip(prefix.len())
                .take(2)
                .map(String::as_str)
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect();
    for helper in [
        "scripts/ci/constraints.py directory",
        "scripts/ci/affected_scope.py --apply-to",
    ] {
        assert!(
            helpers.iter().any(|call| call == helper),
            "the recipe never ran {helper:?} under {fallback}: {helpers:#?}"
        );
    }
    fixture.assert_ci_local_never_selected();
}

/// The interpreter `ci-push` selects when `python3` is broken, and the
/// argument prefix it runs with: `py -3` on Windows, `python` elsewhere.
fn fallback_interpreter() -> (&'static str, &'static [&'static str]) {
    if cfg!(windows) {
        ("py", &["-3"])
    } else {
        ("python", &[])
    }
}

/// `ci-push` starts the gates as [`Evidence::run_recipe`] starts the real
/// recipe: `just pre-push` from the checkout, fed the reviewed plan, with
/// the staging outputs under one directory and named as the direct runs name
/// them, and with the interpreter it selected (here the fallback, as
/// `python3` is broken) and that interpreter's prefix, NUL-terminated.
#[test]
fn the_gates_are_started_as_the_direct_recipe_runs_start_them() {
    use repo_deps::ci_push::python::{encode_prefix, PYTHON_VARIABLE};
    let fixture = Evidence::with_feature("pkg/beta/src/lib.rs", "beta lib");
    fixture.planner_plan(&relabel(include_str!(
        "../../ci/fixtures/pre-push/plans/plan-macos-two-packages.json"
    )));
    fixture.add_stub(
        "python3",
        json!({ "exit": 9, "stderr": "Python was not found\n" }),
    );
    // The fallback runs the real Python only when its prefix leads the argv.
    let (fallback, prefix) = fallback_interpreter();
    fixture.add_stub(
        fallback,
        json!({
            "dispatch": [{ "args": prefix, "behavior": { "delegate": { "program": fixture.python } } }],
            "stderr": "fixture fallback: the interpreter prefix is missing\n",
            "exit": 96,
        }),
    );
    fixture.capture_gate();
    let head = fixture.head();

    let run = fixture.push_feature(Some("strict"));

    run.assert_exit(0);
    let (gate, prefix_file) = fixture.gate_start();
    assert_eq!(gate["argv"], json!(["pre-push"]), "{gate:#}");
    assert!(
        same_path(Path::new(gate["cwd"].as_str().unwrap()), &fixture.repo),
        "{gate:#}"
    );
    let variable = |name: &str| -> PathBuf {
        PathBuf::from(
            gate["env"][name]
                .as_str()
                .unwrap_or_else(|| panic!("{name} was not set: {gate:#}")),
        )
    };
    let staging = variable("BISCUIT_CI_REPORTS_OUT").parent().unwrap().to_path_buf();
    for (name, file) in [
        ("BISCUIT_CI_REPORTS_OUT", REPORTS_DIRECTORY),
        ("BISCUIT_CI_PLAN_OUT", GATE_PLAN_FILE),
        ("BISCUIT_CI_SCOPE_OUT", GATE_SCOPE_FILE),
    ] {
        assert_eq!(variable(name), staging.join(file), "{gate:#}");
    }
    assert_eq!(fixture.fed_plan()["head"], json!(head.to_string()));
    assert!(variable("BISCUIT_CI_PLAN_IN").is_absolute(), "{gate:#}");
    assert!(
        same_path(&variable(PYTHON_VARIABLE), &fixture.tools.tool(fallback)),
        "{gate:#}"
    );
    let expected: Vec<std::ffi::OsString> = prefix.iter().map(Into::into).collect();
    assert_eq!(prefix_file, encode_prefix(&expected).unwrap());
    // python3 was asked the version probe only: neither ci-push nor a
    // helper went back to it.
    let broken = fixture
        .tools
        .invocations()
        .into_iter()
        .filter(|invocation| invocation["tool"] == "python3")
        .count();
    assert_eq!(broken, 1);
}

// ---------- gate outcomes and the publication guard (rows 17–20) ----------

#[test]
fn a_failing_warn_run_reaches_the_publication_block() {
    let fixture = Evidence::new();
    fixture.planner_plan(include_str!(
        "../../ci/fixtures/pre-push/plans/plan-wsl-executing.json"
    ));
    fixture.set_tool("just", json!({ "exit": 1 }));
    append(&fixture.repo.join("README.md"), "edited\n");

    let run = fixture.push_feature(Some("warn"));

    run.assert_exit(0);
    run.assert_says("Pre-push validation failed");
    // A dirty tree is not exact-tree evidence: the guard declines.
    run.assert_says("No validation receipt: the checkout had uncommitted changes");
    run.assert_silent_on("cell(s) of evidence");
    assert_no_validation_note(&fixture);
}

#[test]
fn a_gate_exiting_130_never_publishes_staged_evidence() {
    let plan = relabel(include_str!(
        "../../ci/fixtures/pre-push/plans/plan-macos-executing.json"
    ));
    let fixture = publishing(&plan);
    fixture.set_tool("just", fixture.staging_gate(&plan, false, 130));

    let run = fixture.push_feature(Some("strict"));

    run.assert_exit(130);
    run.assert_says("pre-push interrupted (exit 130)");
    run.assert_says("validation was interrupted");
    assert_no_validation_note(&fixture);
}

// Unix-only: the gate signals its parent, as the shell case's `kill -TERM
// "$PPID"` did; Windows has no such signal.
#[cfg(unix)]
#[test]
fn an_interrupted_hook_never_publishes_staged_evidence() {
    let plan = relabel(include_str!(
        "../../ci/fixtures/pre-push/plans/plan-macos-executing.json"
    ));
    let fixture = publishing(&plan);
    let mut gate = fixture.staging_gate(&plan, false, 0);
    gate["signal_parent"] = json!(libc::SIGTERM);
    fixture.set_tool("just", gate);

    let run = fixture.push_feature(Some("strict"));

    run.assert_exit(143);
    run.assert_says("pre-push interrupted (exit 143)");
    assert_no_validation_note(&fixture);
}

#[test]
fn publication_requires_a_clean_exact_tree_and_no_override() {
    let fixture = publishing(&relabel(include_str!(
        "../../ci/fixtures/pre-push/plans/plan-macos-executing.json"
    )));
    let url = fixture.origin_url();
    let line = format!("{}\n", fixture.creating_feature_line());

    let run = fixture.push_with(Some("strict"), &url, line.as_bytes(), |command| {
        command.env("RUSTY_BISCUIT_PRE_PUSH_AREAS", "alpha");
    });

    run.assert_exit(0);
    run.assert_says("Running pre-push validation for: alpha");
    run.assert_says("RUSTY_BISCUIT_PRE_PUSH_AREAS narrowed the gates");
    assert_eq!(fixture.just_calls(), ["pre-push alpha"]);
    assert_no_validation_note(&fixture);
}

// ---------- retained reports (rows 33–34) ----------

#[test]
fn a_published_receipt_names_retained_readable_reports() {
    let fixture = publishing(&relabel(include_str!(
        "../../ci/fixtures/pre-push/plans/plan-macos-executing.json"
    )));

    let run = fixture.push_feature(Some("strict"));

    run.assert_exit(0);
    run.assert_says(&format!("Published 1 {} cell(s)", fixture.environment));
    let head = fixture.head();
    let note = fixture
        .local_validation(head)
        .expect("a validation note on HEAD");
    assert_eq!(note["schema_version"], json!(3), "{note:#}");
    assert_eq!(
        note["observations"].as_array().unwrap().len(),
        1,
        "{note:#}"
    );
    assert_eq!(
        note["observations"][0]["producer"]["kind"],
        json!("hook"),
        "{note:#}"
    );
    let view = receipt_view(&note);
    // The staging directory is gone; the receipt names the durable copy.
    let expected = fixture
        .evidence
        .join(head.to_string())
        .join(fixture.environment);
    assert_eq!(
        Path::new(view["host"]["report_dir"].as_str().unwrap()),
        expected,
        "{view:#}"
    );
    assert!(
        expected.is_dir(),
        "{} does not exist after ci-push exited",
        expected.display()
    );
    for cell in view["cells"].as_array().unwrap() {
        let report = expected.join(cell["report"].as_str().unwrap());
        assert!(
            std::fs::read(&report).is_ok(),
            "{} is not readable",
            report.display()
        );
    }
    assert!(
        expected.join("manifest.jsonl").is_file(),
        "manifest.jsonl was not retained"
    );
}

/// Also the plan's report-retention failure case: the staged reports are kept
/// where the run says, and the exit status is the gates'.
#[test]
fn reports_that_cannot_be_retained_publish_no_receipt() {
    let fixture = publishing(&relabel(include_str!(
        "../../ci/fixtures/pre-push/plans/plan-macos-executing.json"
    )));
    // A regular file where the evidence root's parent must be a directory.
    let blocker = fixture.root().join("blocker");
    std::fs::write(&blocker, "").unwrap();
    let url = fixture.origin_url();
    let line = format!("{}\n", fixture.creating_feature_line());

    let run = fixture.push_with(Some("strict"), &url, line.as_bytes(), |command| {
        command.env("BISCUIT_CI_EVIDENCE_DIR", blocker.join("evidence"));
    });

    run.assert_exit(0);
    run.assert_says("no receipt is published");
    run.assert_says(&blocker.join("evidence").display().to_string());
    run.assert_silent_on("cell(s) of evidence");
    assert_no_validation_note(&fixture);
    let kept = printed_path(&run, "The run's staged reports remain at ");
    assert!(
        kept.join("manifest.jsonl").is_file(),
        "{} lost its manifest",
        kept.display()
    );
    assert!(
        kept.join("L1").join("alpha.xml").is_file(),
        "{} lost its report",
        kept.display()
    );
}

// ---------- complete failing runs (rows 35–36) ----------

/// The hook's half of this row; CI's rerun is
/// `ci_reruns_a_cell_whose_published_evidence_is_a_complete_failure`.
#[test]
fn a_blocked_strict_failure_publishes_its_note_but_ci_reruns_the_cell() {
    let plan = relabel(include_str!(
        "../../ci/fixtures/pre-push/plans/plan-macos-executing.json"
    ));
    let fixture = Evidence::new();
    fixture.planner_plan(&plan);
    fixture.set_tool("just", fixture.staging_gate(&plan, true, 1));
    let head = fixture.head();

    let run = fixture.push_feature(Some("strict"));

    run.assert_exit(1);
    run.assert_says("pre-push validation failed (exit 1) in strict mode");
    run.assert_says(&format!("Published 1 {} cell(s)", fixture.environment));
    // ci-push pushes the notes ref and nothing else: the branch is blocked.
    assert!(
        !fixture.remote_has_ref("refs/heads/feature"),
        "the blocked branch reached the remote"
    );
    assert_remote_complete_failure(&fixture, head);
}

/// CI's view of the note a blocked strict failure published (see
/// `a_blocked_strict_failure_publishes_its_note_but_ci_reruns_the_cell`),
/// after the `--no-verify` push of the same tree: a fresh clone with the
/// notes, verifying with the committed helper, refuses the failed cell, and
/// the real overlay leaves it to CI. The note is recorded through the real
/// helpers ([`Evidence::record_evidence`]).
#[test]
fn ci_reruns_a_cell_whose_published_evidence_is_a_complete_failure() {
    let plan = relabel(include_str!(
        "../../ci/fixtures/pre-push/plans/plan-macos-executing.json"
    ));
    let fixture = Evidence::new();
    fixture.planner_plan(&plan);
    let head = fixture.head();
    fixture.record_evidence(
        "pull_request",
        fixture.main,
        head,
        &fixture.recipe_outcome(&[("alpha", TestGate::Fail)]),
    );
    assert_remote_complete_failure(&fixture, head);

    fixture.push_refs(&["refs/heads/feature:refs/heads/feature"]);
    let ci = fixture.root().join("ci");
    let clone = git2::Repository::clone(fixture.origin.to_str().unwrap(), &ci).unwrap();
    clone
        .find_remote("origin")
        .unwrap()
        .fetch(
            &["+refs/notes/ci-local/*:refs/notes/ci-local/*"],
            None,
            None,
        )
        .unwrap();
    let plan_file = fixture.root().join("plan.json");
    std::fs::write(&plan_file, &plan).unwrap();
    let rejections = fixture.root().join("rejections.json");
    let verified = fixture
        .python_command()
        .current_dir(&ci)
        .arg("scripts/ci/local_evidence.py")
        .args(["verify", "--cells", "--plan"])
        .arg(&plan_file)
        .args([
            "--base",
            &fixture.main.to_string(),
            "--head",
            &head.to_string(),
            "--rejections",
        ])
        .arg(&rejections)
        .output()
        .unwrap();
    assert!(
        verified.status.success(),
        "{}",
        String::from_utf8_lossy(&verified.stderr)
    );
    let accepted: Value = serde_json::from_slice(&verified.stdout).unwrap();
    assert_eq!(accepted, json!([]), "verification accepted a failed cell");
    let refused: Value = serde_json::from_slice(&std::fs::read(&rejections).unwrap()).unwrap();
    let refused = refused.as_array().unwrap();
    assert_eq!(refused.len(), 1, "{refused:#?}");
    let expected = format!(
        "failed-cell: alpha/{}/L1 failed; failing evidence is diagnostic only",
        fixture.environment
    );
    assert!(
        refused[0].as_str().unwrap().starts_with(&expected),
        "{refused:#?}"
    );
    // The real overlay leaves the refused cell assigned to CI.
    let accepted_file = fixture.root().join("accepted.json");
    std::fs::write(&accepted_file, &verified.stdout).unwrap();
    let applied = fixture
        .python_command()
        .arg(biscuit_test_harness::manifest_dir!().join("ci/affected_scope.py"))
        .arg("--apply-to")
        .arg(&plan_file)
        .arg("--accepted-cells")
        .arg(&accepted_file)
        .arg("--resolved-plan")
        .output()
        .unwrap();
    assert!(
        applied.status.success(),
        "{}",
        String::from_utf8_lossy(&applied.stderr)
    );
    let applied: Value = serde_json::from_slice(&applied.stdout).unwrap();
    let cells = applied["cells"].as_array().unwrap();
    assert_eq!(cells.len(), 1, "{cells:#?}");
    assert_eq!(cells[0]["execution"], json!("execute"), "{cells:#?}");
    assert_eq!(cells[0]["origin"], json!("ci"), "{cells:#?}");
    assert_eq!(cells[0]["state"], json!("pending"), "{cells:#?}");
}

#[test]
fn a_complete_warn_failure_publishes_its_note() {
    let plan = relabel(include_str!(
        "../../ci/fixtures/pre-push/plans/plan-macos-executing.json"
    ));
    let fixture = Evidence::new();
    fixture.planner_plan(&plan);
    fixture.set_tool("just", fixture.staging_gate(&plan, true, 1));

    let run = fixture.push_feature(Some("warn"));

    run.assert_exit(0);
    run.assert_says("Pre-push validation failed");
    run.assert_says(&format!("Published 1 {} cell(s)", fixture.environment));
    assert_remote_complete_failure(&fixture, fixture.head());
}

// ---------- scope evidence (rows 37–43) ----------

#[test]
fn scope_only_publishes_committed_scope_for_a_push_to_main() {
    let fixture = publishing(&relabel(include_str!(
        "../../ci/fixtures/pre-push/plans/plan-macos-executing.json"
    )));
    let (main, head) = (fixture.main, fixture.head());
    let url = fixture.origin_url();

    let run = fixture.push(
        Some("scope-only"),
        &url,
        &format!("refs/heads/feature {head} refs/heads/main {main}"),
    );

    run.assert_exit(0);
    assert!(!gate_ran(&fixture), "{:?}", fixture.just_calls());
    run.assert_says("the push event's before");
    run.assert_says("Published scope evidence");
    assert_scope_receipt_base(&fixture, main);
    // Options, then `--`, then the committed path set of base..head.
    assert_planned(
        &fixture,
        &format!("--base {main} --head {head} -- work.txt"),
    );
    assert!(
        fixture.remote_has_ref("refs/notes/ci-local/scope"),
        "the scope ref was not pushed"
    );
}

#[test]
fn the_published_scope_receipt_binds_this_plan_generation() {
    let fixture = publishing(&relabel(include_str!(
        "../../ci/fixtures/pre-push/plans/plan-macos-executing.json"
    )));
    let (main, head) = (fixture.main, fixture.head());
    let url = fixture.origin_url();
    fixture
        .push(
            Some("scope-only"),
            &url,
            &format!("refs/heads/feature {head} refs/heads/main {main}"),
        )
        .assert_exit(0);
    let receipt = fixture
        .scope_receipt(head)
        .expect("a scope receipt on HEAD");
    let file = fixture.root().join("scope-receipt.json");
    std::fs::write(&file, receipt.to_string()).unwrap();

    // The committed (real) `schema.py` validates the receipt, and refuses the
    // previous plan generation whole, once, as `scope-schema:`.
    let check = r#"
import json, sys
from pathlib import Path
sys.path.insert(0, sys.argv[1])
import schema
path = Path(sys.argv[2])
receipt = json.loads(path.read_text(encoding="utf-8"))
version = schema.RESOLVED_PLAN_SCHEMA_VERSION
assert receipt["plan_schema_version"] == version, receipt["plan_schema_version"]
assert receipt["plan"]["schema_version"] == version, receipt["plan"]["schema_version"]
problems = schema.validate_scope_receipt(receipt)
assert not problems, problems
stale = json.loads(path.read_text(encoding="utf-8"))
stale["plan_schema_version"] = version - 1
stale["plan"]["schema_version"] = version - 1
problems = schema.validate_scope_receipt(stale)
assert len(problems) == 1 and problems[0].startswith("scope-schema:"), problems
"#;
    let output = fixture
        .python_command()
        .arg("-c")
        .arg(check)
        .arg(fixture.repo.join("scripts").join("ci"))
        .arg(&file)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn the_committed_scope_declares_deletions_and_omits_rename_sources() {
    let fixture = publishing(&relabel(include_str!(
        "../../ci/fixtures/pre-push/plans/plan-macos-executing.json"
    )));
    std::fs::remove_file(fixture.repo.join("pkg/alpha/src/lib.rs")).unwrap();
    std::fs::rename(
        fixture.repo.join("README.md"),
        fixture.repo.join("READING.md"),
    )
    .unwrap();
    let head =
        crate::evidence_fixture::commit_all(&fixture.git(), "delete one file and rename another");
    let main = fixture.main;
    let url = fixture.origin_url();

    let run = fixture.push(
        Some("scope-only"),
        &url,
        &format!("refs/heads/feature {head} refs/heads/main {main}"),
    );

    run.assert_exit(0);
    assert_planned(&fixture, "--deleted pkg/alpha/src/lib.rs");
    assert_planned(&fixture, "READING.md");
    assert_planned(&fixture, "--renamed-from README.md");
    assert!(
        !planned(&fixture, "--deleted README.md"),
        "{}",
        fixture.planner_log()
    );
    for call in fixture.planner_calls() {
        if let Some((_, paths)) = call.split_once(" -- ") {
            assert!(!paths.split(' ').any(|path| path == "README.md"), "{call}");
        }
    }
}

#[test]
fn a_feature_branch_push_records_the_pull_request_base_not_its_previous_tip() {
    let fixture = publishing(&relabel(include_str!(
        "../../ci/fixtures/pre-push/plans/plan-macos-executing.json"
    )));
    fixture.push_refs(&["refs/heads/feature:refs/heads/feature"]);
    let previous = fixture.head();
    let head = fixture.follow_up("more.txt", "more");
    let url = fixture.origin_url();

    let run = fixture.push(
        Some("scope-only"),
        &url,
        &format!("refs/heads/feature {head} refs/heads/feature {previous}"),
    );

    run.assert_exit(0);
    run.assert_says("provisional: no open pull request");
    assert_scope_receipt_base(&fixture, fixture.main);
}

#[test]
fn a_branch_creating_push_records_the_remote_main_tip_and_says_so() {
    let fixture = publishing(&relabel(include_str!(
        "../../ci/fixtures/pre-push/plans/plan-macos-executing.json"
    )));

    let run = fixture.push_feature(Some("strict"));

    run.assert_exit(0);
    run.assert_says("provisional: no open pull request");
    run.assert_says("a pull request opened against main before main advances will run this plan");
    assert_scope_receipt_base(&fixture, fixture.main);
    // Scope goes out before the gates run, whatever they then do.
    let scope = run
        .stderr
        .find("Published scope evidence")
        .expect("scope published");
    let validation = run
        .stderr
        .find(&format!("Published 1 {} cell(s)", fixture.environment))
        .expect("validation published");
    assert!(
        scope < validation,
        "scope evidence was published after the validation receipt:\n{run}"
    );
}

#[test]
fn a_dirty_strict_run_publishes_scope_but_no_validation_receipt() {
    let fixture = publishing(&relabel(include_str!(
        "../../ci/fixtures/pre-push/plans/plan-macos-executing.json"
    )));
    append(&fixture.repo.join("README.md"), "edited\n");
    std::fs::write(fixture.repo.join("untracked.txt"), "untracked\n").unwrap();

    let run = fixture.push_feature(Some("strict"));

    run.assert_exit(0);
    assert!(gate_ran(&fixture), "{:?}", fixture.just_calls());
    assert_scope_receipt_base(&fixture, fixture.main);
    // The committed path set, never the working tree's.
    let calls = fixture.planner_calls().join("\n");
    assert!(!calls.contains("README.md"), "{calls}");
    assert!(!calls.contains("untracked.txt"), "{calls}");
    assert_planned(&fixture, "-- work.txt");
    assert_no_validation_note(&fixture);
    run.assert_silent_on(&format!("Published 1 {}", fixture.environment));
}

#[test]
fn a_failed_scope_publication_is_named_and_leaves_the_exit_code_alone() {
    let fixture = publishing(&relabel(include_str!(
        "../../ci/fixtures/pre-push/plans/plan-macos-executing.json"
    )));
    // The remote stays readable and refuses only the scope notes ref.
    install_tool(
        &fixture.origin.join("hooks"),
        "pre-receive",
        json!({ "pre-receive": {
            "read_stdin": true,
            "refuse_stdin_containing": "refs/notes/ci-local/scope",
        } }),
    );

    let run = fixture.push_feature(Some("scope-only"));

    run.assert_exit(0);
    run.assert_says("Scope evidence PUBLICATION failed");
    run.assert_silent_on("RECORDING failed");
    run.assert_silent_on("Published scope evidence");
    // Calculated and recorded locally; only the transfer failed.
    assert_scope_receipt_base(&fixture, fixture.main);
    assert!(!fixture.remote_has_ref("refs/notes/ci-local/scope"));
}

// ---------- the outgoing revision's committed tree (rows 44–46) ----------

#[test]
fn a_committed_change_masked_by_an_unstaged_revert_is_still_reviewed() {
    let fixture = Evidence::with_feature("pkg/alpha/src/lib.rs", "changed lib");
    fixture.planner_plan(&relabel(include_str!(
        "../../ci/fixtures/pre-push/plans/plan-macos-executing.json"
    )));
    std::fs::write(fixture.repo.join("pkg/alpha/src/lib.rs"), "base lib\n").unwrap();
    let git = fixture.git();
    let main_tree = git.find_commit(fixture.main).unwrap().tree().unwrap();
    let diff = git.diff_tree_to_workdir(Some(&main_tree), None).unwrap();
    assert_eq!(
        diff.deltas().len(),
        0,
        "fixture error: the working tree still differs from the base"
    );
    fixture.prohibit(&fixture.constraints, fixture.environment);

    let run = fixture.push_feature(Some("strict"));

    run.assert_exit(1);
    run.assert_says(fixture.environment);
    run.assert_says(PROHIBITION_REASON);
    assert_planned(&fixture, "-- pkg/alpha/src/lib.rs");
    assert!(!gate_ran(&fixture), "{:?}", fixture.just_calls());
    // Constraints are decided before any receipt is published.
    assert!(
        fixture.scope_receipt(fixture.head()).is_none(),
        "a blocked push left a scope receipt"
    );
    run.assert_says("temporary worktree");
    fixture.assert_no_leftover_worktree();
}

#[test]
fn an_unstaged_policy_edit_never_reaches_the_committed_plan_or_receipt() {
    let fixture = publishing(&relabel(include_str!(
        "../../ci/fixtures/pre-push/plans/plan-macos-executing.json"
    )));
    std::fs::write(
        fixture.repo.join(".github/ci/policy.json"),
        "{\"preflight_reason\": \"working-tree policy\"}\n",
    )
    .unwrap();

    let run = fixture.push_feature(Some("scope-only"));

    run.assert_exit(0);
    assert_scope_receipt_base(&fixture, fixture.main);
    let receipt = fixture.scope_receipt(fixture.head()).unwrap();
    assert_eq!(
        receipt["plan"]["preflight_reason"],
        json!("committed policy"),
        "{receipt:#}"
    );
    assert_eq!(
        receipt["scope"]["preflight_reason"],
        json!("committed policy"),
        "{receipt:#}"
    );
    // The planner that produced it lived in the committed tree.
    assert!(
        !same_path(&fixture.planner_root(), &fixture.repo),
        "planned in the dirty checkout"
    );
    run.assert_says("temporary worktree");
    fixture.assert_no_leftover_worktree();
}

#[test]
fn a_clean_checkout_is_planned_in_place() {
    let fixture = publishing(&relabel(include_str!(
        "../../ci/fixtures/pre-push/plans/plan-macos-executing.json"
    )));

    let run = fixture.push_feature(Some("scope-only"));

    run.assert_exit(0);
    assert!(
        same_path(&fixture.planner_root(), &fixture.repo),
        "a clean checkout was not planned in place; the planner ran from {}",
        fixture.planner_root().display()
    );
    run.assert_silent_on("temporary worktree");
    fixture.assert_no_leftover_worktree();
}

// ---------- pull requests and their targets (rows 57–59, 61) ----------

#[test]
fn a_stacked_pull_request_is_planned_against_its_target_branch_tip() {
    let (fixture, parent, child) = Evidence::stacked(false);
    fixture.planner_plan(include_str!(
        "../../ci/fixtures/pre-push/plans/plan-wsl-executing.json"
    ));
    fixture.open_pull_requests(&[(12, "parent")]);
    fixture.prohibit(&fixture.constraints, "wsl2-ubuntu");

    let run = fixture.push(
        Some("strict"),
        GITHUB_URL,
        &format!("refs/heads/child {child} refs/heads/child {ZERO}"),
    );

    // CI diffs parent's tip against child, which selects alpha, so the WSL
    // cell would execute and the prohibition blocks.
    run.assert_exit(1);
    assert_planned(
        &fixture,
        &format!("--base {parent} --head {child} -- pkg/alpha/src/lib.rs"),
    );
    run.assert_says("pull request #12 into parent");
    assert!(
        fixture
            .tool_calls("gh")
            .iter()
            .any(|call| call.contains("--repo acme/widgets --head child")),
        "{:?}",
        fixture.tool_calls("gh")
    );
    run.assert_says(PROHIBITION_REASON);
    assert!(!gate_ran(&fixture), "{:?}", fixture.just_calls());
    fixture.assert_no_leftover_worktree();
}

#[test]
fn a_pull_request_into_main_from_a_restoring_child_selects_nothing() {
    let (fixture, _parent, child) = Evidence::stacked(false);
    fixture.planner_plan(include_str!(
        "../../ci/fixtures/pre-push/plans/plan-wsl-executing.json"
    ));
    // The review's children get null stdin: gh reads end of file at once,
    // while ci-push itself read the whole update list.
    fixture.set_tool(
        "gh",
        json!({ "stdout": "[{\"number\":12,\"baseRefName\":\"main\"}]", "read_stdin": true }),
    );
    fixture.prohibit(&fixture.constraints, "wsl2-ubuntu");

    let run = fixture.push(
        Some("strict"),
        GITHUB_URL,
        &format!("refs/heads/child {child} refs/heads/child {ZERO}"),
    );

    run.assert_exit(0);
    let main = fixture.main;
    assert!(
        fixture
            .planner_calls()
            .iter()
            .any(|call| call.ends_with(&format!("--base {main} --head {child} --"))),
        "expected an empty path set against main:\n{}",
        fixture.planner_log()
    );
    run.assert_says("pull request #12 into main");
    run.assert_silent_on("Push blocked");
    assert!(gate_ran(&fixture), "{:?}", fixture.just_calls());
    let invocations = fixture.tools.invocations();
    let gh = invocations
        .iter()
        .find(|invocation| invocation["tool"] == "gh")
        .expect("gh was asked");
    assert_eq!(gh["stdin"], json!(""), "{gh:#}");
}

#[test]
fn an_advanced_target_branch_is_read_from_the_remote_and_records_exact_scope() {
    let fixture = Evidence::new();
    fixture.planner_plan(include_str!(
        "../../ci/fixtures/pre-push/plans/plan-wsl-executing.json"
    ));
    fixture.open_pull_requests(&[(7, "main")]);
    let head = fixture.head();
    // main advances on the remote, touching a source file; the checkout's
    // tracking ref is left stale on purpose.
    let new_main = fixture.advance_remote("main", "pkg/alpha/src/lib.rs", "advanced lib\n");
    let tracking = fixture.git().refname_to_id("refs/remotes/origin/main").ok();
    assert_ne!(
        tracking,
        Some(new_main),
        "fixture error: the tracking ref is not stale"
    );

    let run = fixture.push(
        Some("scope-only"),
        GITHUB_URL,
        &fixture.creating_feature_line(),
    );

    run.assert_exit(0);
    // CI's two-dot diff from the advanced tip includes main's own change.
    assert_planned(
        &fixture,
        &format!("--base {new_main} --head {head} -- pkg/alpha/src/lib.rs work.txt"),
    );
    run.assert_says("pull request #7 into main");
    let receipt = fixture.scope_receipt(head).expect("a scope receipt");
    assert_eq!(receipt["base"], json!(new_main.to_string()), "{receipt:#}");
    run.assert_says("Published scope evidence");
}

#[test]
fn a_record_in_the_default_store_binds_a_fresh_session() {
    let fixture = Evidence::new();
    fixture.planner_plan(include_str!(
        "../../ci/fixtures/pre-push/plans/plan-wsl-executing.json"
    ));
    let store = fixture.home.join(".rusty-biscuit").join("ci-constraints");
    let other = store.join("github.com").join("other").join("repo");
    std::fs::create_dir_all(&other).unwrap();
    let record = json!({
        "environment": "wsl2-ubuntu",
        "reason": "do not rerun WSL for repository github.com/other/repo",
        "owner": "ken",
        "expiry": "2099-01-01",
        "repository": "github.com/other/repo",
    });
    std::fs::write(other.join("other.json"), record.to_string()).unwrap();
    let ours = store.join("github.com").join("acme").join("widgets");
    fixture.prohibit(&ours, "wsl2-ubuntu");
    let line = format!("{}\n", fixture.creating_feature_line());
    let fresh = |command: &mut std::process::Command| {
        command.env_remove("BISCUIT_CI_CONSTRAINTS_DIR");
    };

    let run = fixture.push_with(Some("strict"), GITHUB_URL, line.as_bytes(), fresh);

    run.assert_exit(1);
    run.assert_says(PROHIBITION_REASON);
    assert!(!gate_ran(&fixture), "{:?}", fixture.just_calls());

    // Only the other repository's record remains, outside this repository's
    // directory, so it does not bind.
    std::fs::remove_file(ours.join("current.json")).unwrap();
    fixture.reset_logs();
    let run = fixture.push_with(Some("strict"), GITHUB_URL, line.as_bytes(), fresh);

    run.assert_exit(0);
    run.assert_silent_on("Push blocked");
}

// ---------- binary-boundary cases beside the rows ----------

#[test]
fn empty_input_exits_zero_and_launches_nothing() {
    let fixture = Evidence::new();
    let url = fixture.origin_url();

    let run = fixture.push_with(Some("strict"), &url, b"", |_| {});

    run.assert_exit(0);
    run.assert_says("triggers no run and there is no plan to review");
    assert_eq!(fixture.tools.invocations(), Vec::<Value>::new());
    assert_eq!(fixture.planner_log(), "");
}

#[test]
fn a_malformed_line_is_located_and_launches_nothing() {
    let fixture = Evidence::new();
    let url = fixture.origin_url();
    let input = format!(
        "{}\nrefs/heads/feature not-an-object refs/heads/feature\n",
        fixture.creating_feature_line()
    );

    let run = fixture.push_with(Some("strict"), &url, input.as_bytes(), |_| {});

    run.assert_exit(1);
    run.assert_says("ref update line 2");
    run.assert_says("nothing was reviewed, published, or run");
    assert_eq!(fixture.tools.invocations(), Vec::<Value>::new());
    assert_eq!(fixture.planner_log(), "");
    assert!(!fixture.local_has_ref("refs/notes/ci-local/scope"));
}

#[test]
fn a_broken_python3_hands_the_fallback_to_every_helper_and_the_gates() {
    let plan = relabel(include_str!(
        "../../ci/fixtures/pre-push/plans/plan-macos-executing.json"
    ));
    let fixture = publishing(&plan);
    let mut gate = fixture.staging_gate(&plan, false, 0);
    gate["record_env"] = json!(["BISCUIT_CI_PYTHON"]);
    fixture.set_tool("just", gate);
    fixture.add_stub(
        "python3",
        json!({ "exit": 9, "stderr": "Python was not found\n" }),
    );
    fixture.add_stub(
        "python",
        json!({ "delegate": { "program": fixture.python } }),
    );

    let run = fixture.push_feature(Some("strict"));

    run.assert_exit(0);
    run.assert_says(&format!("Published 1 {} cell(s)", fixture.environment));
    let invocations = fixture.tools.invocations();
    let argv = |tool: &str| -> Vec<Vec<String>> {
        invocations
            .iter()
            .filter(|invocation| invocation["tool"] == tool)
            .map(|invocation| serde_json::from_value(invocation["argv"].clone()).unwrap())
            .collect()
    };
    // python3 was asked the version probe only, and failed it.
    let broken = argv("python3");
    assert_eq!(broken.len(), 1, "{broken:?}");
    assert_eq!(broken[0][0], "-c");
    // Every helper ran through the fallback.
    let helpers: Vec<String> = argv("python")
        .iter()
        .filter(|args| args[0] != "-c")
        .map(|args| {
            let script = Path::new(&args[0])
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned();
            format!("{script} {}", args[1])
        })
        .collect();
    assert_eq!(
        helpers,
        [
            "affected_scope.py --event",
            "local_evidence.py verify",
            "affected_scope.py --apply-to",
            "local_evidence.py scope-record",
            "local_evidence.py record-cells",
            "observation_notes.py publish-receipt",
        ]
    );
    let gate = invocations
        .iter()
        .find(|invocation| invocation["tool"] == "just")
        .unwrap();
    let handed = PathBuf::from(gate["env"]["BISCUIT_CI_PYTHON"].as_str().unwrap());
    assert!(
        same_path(&handed, &fixture.tools.tool("python")),
        "{gate:#}"
    );
}

#[test]
fn with_no_interpreter_one_error_names_every_candidate_and_nothing_runs() {
    let fixture = Evidence::new();
    let url = fixture.origin_url();
    let path = fixture.stub_path();
    let line = format!("{}\n", fixture.creating_feature_line());

    let run = fixture.push_with(Some("strict"), &url, line.as_bytes(), |command| {
        command.env("PATH", &path);
    });

    run.assert_exit(2);
    run.assert_says("no Python 3.10 or newer could be run; tried: python3 not found on PATH");
    if cfg!(windows) {
        run.assert_says("; py -3 not found on PATH");
    }
    run.assert_says("; python not found on PATH");
    assert_eq!(fixture.tools.invocations(), Vec::<Value>::new());
    assert_eq!(fixture.planner_log(), "");
}

/// The plan's publication-failure case: `observation_notes.py` cannot write
/// the validation note (its ref is locked), so nothing is published; the
/// durable reports stay retained and the push is not blocked.
///
/// A remote that refuses the notes push would reach the same outcome through
/// the helper's own five-attempt randomized backoff (2-10 s), so the cheaper
/// local failure stands in for it here; the exit-4 mapping of that path is
/// `ci_push::tests::orchestration`'s `recorded_locally` case.
#[test]
fn a_failed_validation_note_publication_keeps_the_durable_reports() {
    let fixture = publishing(&relabel(include_str!(
        "../../ci/fixtures/pre-push/plans/plan-macos-executing.json"
    )));
    let lock = fixture
        .repo
        .join(".git/refs/notes/ci-local")
        .join(format!("{}.lock", fixture.environment));
    std::fs::create_dir_all(lock.parent().unwrap()).unwrap();
    std::fs::write(&lock, "").unwrap();

    let run = fixture.push_feature(Some("strict"));

    run.assert_exit(0);
    run.assert_says("the observation could not be recorded");
    run.assert_says("Pre-push validation passed.");
    run.assert_silent_on("cell(s) of evidence");
    assert_no_validation_note(&fixture);
    let durable = fixture
        .evidence
        .join(fixture.head().to_string())
        .join(fixture.environment);
    assert!(
        durable.join("manifest.jsonl").is_file(),
        "{} lost its manifest",
        durable.display()
    );
    assert!(
        durable.join("L1").join("alpha.xml").is_file(),
        "{} lost its report",
        durable.display()
    );
}

/// SIGINT while the gates hang with a grandchild: the whole tree ends, the
/// run exits 130, and nothing is published or left behind.
#[cfg(unix)]
#[test]
fn an_interrupt_during_the_gates_ends_their_tree_and_publishes_nothing() {
    use crate::fixture::{pid_of, process_gone, read_pid, wait_until};
    use std::time::Duration;
    let plan = relabel(include_str!(
        "../../ci/fixtures/pre-push/plans/plan-macos-executing.json"
    ));
    let fixture = publishing(&plan);
    let grandchild = fixture.root().join("grandchild.pid");
    let mut gate = fixture.staging_gate(&plan, false, 0);
    gate["spawn_grandchild"] = json!(grandchild);
    gate["hang"] = json!(true);
    fixture.set_tool("just", gate);
    let url = fixture.origin_url();

    let child = fixture.spawn(Some("strict"), &url, &fixture.creating_feature_line());
    wait_until(Duration::from_secs(10), "the gate's grandchild", || {
        grandchild.exists()
    });
    let invocations = fixture.tools.invocations();
    let gate_pid = pid_of(
        invocations
            .iter()
            .find(|invocation| invocation["tool"] == "just")
            .unwrap(),
    );
    let grandchild_pid = read_pid(&grandchild);
    // SAFETY: signals the ci-push this test spawned.
    unsafe { libc::kill(child.id() as libc::pid_t, libc::SIGINT) };
    let run = crate::evidence_fixture::finished(&child.wait_with_output().unwrap());

    run.assert_exit(130);
    run.assert_says("pre-push interrupted (exit 130)");
    wait_until(
        Duration::from_secs(5),
        "the gate and its grandchild to end",
        || process_gone(gate_pid) && process_gone(grandchild_pid),
    );
    assert_no_validation_note(&fixture);
    fixture.assert_no_leftover_worktree();
    let staged: Vec<_> = std::fs::read_dir(fixture.root().join("tmp"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .filter(|name| name.to_string_lossy().starts_with("ci-push-"))
        .collect();
    assert!(staged.is_empty(), "staging outlived the run: {staged:?}");
}

/// SIGTERM while the planner runs in a temporary worktree: the planner ends,
/// the worktree and its administrative record are removed, the run exits
/// 143, and no scope receipt is recorded.
#[cfg(unix)]
#[test]
fn a_termination_during_the_review_removes_its_temporary_worktree() {
    use crate::fixture::{process_gone, read_pid, wait_until};
    use std::time::Duration;
    let fixture = Evidence::new();
    fixture.planner_plan(&relabel(include_str!(
        "../../ci/fixtures/pre-push/plans/plan-macos-executing.json"
    )));
    // Uncommitted changes send the review to a temporary worktree.
    std::fs::write(fixture.repo.join("untracked.txt"), "untracked\n").unwrap();
    let planner = fixture.root().join("planner.pid");
    fixture.planner_hangs(&planner);
    let url = fixture.origin_url();

    let child = fixture.spawn(Some("strict"), &url, &fixture.creating_feature_line());
    wait_until(Duration::from_secs(10), "the planner to start", || {
        planner.exists()
    });
    assert_eq!(
        fixture.git().worktrees().unwrap().len(),
        1,
        "the review is not in a temporary worktree"
    );
    let planner_pid = read_pid(&planner);
    // SAFETY: signals the ci-push this test spawned.
    unsafe { libc::kill(child.id() as libc::pid_t, libc::SIGTERM) };
    let run = crate::evidence_fixture::finished(&child.wait_with_output().unwrap());

    run.assert_exit(143);
    wait_until(Duration::from_secs(5), "the planner to end", || {
        process_gone(planner_pid)
    });
    fixture.assert_no_leftover_worktree();
    assert!(
        fixture.scope_receipt(fixture.head()).is_none(),
        "an interrupted review recorded scope"
    );
    assert!(!gate_ran(&fixture), "{:?}", fixture.just_calls());
}
