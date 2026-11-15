//! Parity with the retired shell hook's decision cases: each test is the
//! in-process counterpart of one `.githooks/tests/test-pre-push.sh` case,
//! named after it, and asserts what that case asserted (exit status, what
//! the user is told, whether and how the planner and the gates ran, and
//! whether scope was published) through the fake of `orchestration`.
//!
//! The parity record, including the rows answered by tests elsewhere, is the
//! "Parity Inventory" of the `2026-11-01-rust-pre-push` implementation log.

use gix::ObjectId;
use rstest::rstest;

use super::orchestration::{
    BASE_TIP, Call, Fake, GITHUB, HEAD, MAIN_TIP, OTHER, WSL_EXECUTING, ZERO, blocked, feature, id,
    line, main_push, problem, push, push_with, record, value,
};
use crate::ci_push::{BlockReason, Diagnostic, PushMode, ReviewStatus, ScopePublication};

/// A prohibition of `wsl2-ubuntu` scoped by one optional field, whose
/// reason names that scope.
fn scoped(field: &str, scope: &str) -> String {
    format!(
        "{{\"environment\": \"wsl2-ubuntu\", \"reason\": \"do not rerun WSL for {field} {scope}\", \"owner\": \"ken\", \"expiry\": \"2099-01-01\", \"{field}\": \"{scope}\"}}"
    )
}

/// The revision each planner run was handed, in order.
fn planned_heads(fake: &Fake) -> Vec<String> {
    fake.python_args("affected_scope.py --event")
        .iter()
        .map(|args| value(args, "--head").to_owned())
        .collect()
}

fn arguments_files(fake: &Fake) -> Vec<(ObjectId, ObjectId)> {
    fake.calls
        .iter()
        .filter_map(|call| match call {
            Call::ArgumentsFile(base, head) => Some((*base, *head)),
            _ => None,
        })
        .collect()
}

fn said(fake: &Fake, text: &str) -> bool {
    fake.emitted.iter().any(|line| line.contains(text))
}

fn is_blocked(diagnostic: &Diagnostic) -> bool {
    matches!(diagnostic, Diagnostic::Blocked(_))
}

fn refused(outcome: &crate::ci_push::PushOutcome) -> (&str, &[String]) {
    match blocked(outcome) {
        BlockReason::Constraint { update, refusals } => (update, refusals),
        other => panic!("not a constraint refusal: {other:?}"),
    }
}

// ----------------------------------------------------------------- modes --

#[test]
fn scope_only_runs_no_gate_but_still_resolves_scope() {
    let mut fake = Fake::new();
    fake.gates.code = 7;
    let (outcome, fake) = push(fake, "scope-only", None, &feature(HEAD));
    assert_eq!(outcome.exit_code, 0, "{:#?}", outcome.diagnostics);
    assert_eq!(planned_heads(&fake), [HEAD]);
    assert!(fake.gate_calls().is_empty() && outcome.gates.is_none());
}

#[test]
fn off_mode_is_a_deprecated_alias_of_scope_only() {
    let mut fake = Fake::new();
    fake.gates.code = 7;
    let (outcome, fake) = push(fake, "off", None, &feature(HEAD));
    assert_eq!(outcome.exit_code, 0, "{:#?}", outcome.diagnostics);
    assert_eq!(outcome.mode, Some(PushMode::ScopeOnly));
    assert!(fake.emitted[0].contains("deprecated") && fake.emitted[0].contains("scope-only"));
    assert_eq!(planned_heads(&fake), [HEAD]);
    assert!(fake.gate_calls().is_empty());
}

#[test]
fn warn_passing_tests_exits_zero() {
    let (outcome, fake) = push(Fake::new(), "warn", None, &feature(HEAD));
    assert_eq!(outcome.exit_code, 0, "{:#?}", outcome.diagnostics);
    assert_eq!(fake.gate_calls().len(), 1);
    assert_eq!(fake.emitted.last().map(String::as_str), Some("Pre-push validation passed."));
}

#[test]
fn unset_default_runs_no_gate_for_a_push_that_starts_no_run() {
    let mut fake = Fake::new();
    fake.gates.code = 7;
    let (outcome, fake) = push_with(fake, None, None, &feature(HEAD));
    assert_eq!(outcome.exit_code, 0, "{:#?}", outcome.diagnostics);
    assert_eq!(outcome.mode, Some(PushMode::Auto));
    assert_eq!(planned_heads(&fake), [HEAD]);
    assert!(fake.gate_calls().is_empty());
    assert!(outcome.diagnostics.contains(&Diagnostic::NoGateRuns));
    assert!(said(&fake, "this push starts no CI run"));
    assert!(said(&fake, "RUSTY_BISCUIT_PRE_PUSH=strict"));
}

#[test]
fn explicit_auto_is_the_default() {
    let mut fake = Fake::new();
    fake.gates.code = 7;
    let (outcome, fake) = push(fake, "auto", None, &feature(HEAD));
    assert_eq!(outcome.exit_code, 0, "{:#?}", outcome.diagnostics);
    assert_eq!(outcome.mode, Some(PushMode::Auto));
    assert!(fake.gate_calls().is_empty());
    assert!(outcome.diagnostics.contains(&Diagnostic::NoGateRuns));
}

#[test]
fn unset_default_is_strict_for_a_push_to_main() {
    let mut fake = Fake::new();
    fake.gates.code = 7;
    let (outcome, fake) = push_with(fake, None, None, &main_push(MAIN_TIP));
    assert_eq!(outcome.exit_code, 7, "{:#?}", outcome.diagnostics);
    assert_eq!(fake.gate_calls().len(), 1);
    assert_eq!(outcome.gates.as_ref().unwrap().mode, PushMode::Strict);
    assert_eq!(blocked(&outcome), &BlockReason::GateFailed { exit_code: 7 });
}

#[test]
fn unset_default_is_strict_for_a_branch_with_an_open_pull_request() {
    let mut fake = Fake::new();
    fake.url = Some(GITHUB.to_owned());
    fake.gh_by_branch.insert("feature".to_owned(), r#"[{"number": 12, "baseRefName": "main"}]"#);
    fake.gates.code = 7;
    let (outcome, fake) = push_with(fake, None, None, &feature(HEAD));
    assert_eq!(outcome.exit_code, 7, "{:#?}", outcome.diagnostics);
    assert!(said(&fake, "pull request #12 into main"));
    assert_eq!(fake.gate_calls().len(), 1);
    assert_eq!(outcome.gates.as_ref().unwrap().mode, PushMode::Strict);
}

// ------------------------------------------------------- gate invocation --

#[test]
fn default_delegates_scope_to_pre_push() {
    let (outcome, fake) = push(Fake::new(), "warn", None, &feature(HEAD));
    assert_eq!(outcome.exit_code, 0, "{:#?}", outcome.diagnostics);
    assert_eq!(planned_heads(&fake), [HEAD]);
    // Exactly one gate run, with no selection of its own.
    match fake.gate_calls().as_slice() {
        [Call::Gates { selection, .. }] => assert!(selection.is_empty(), "{selection:?}"),
        other => panic!("expected one gate run, got {other:?}"),
    }
}

#[test]
fn run_hook_forwards_no_selection_without_an_override() {
    let (outcome, fake) = push(Fake::new(), "warn", None, &feature(HEAD));
    assert_eq!(outcome.exit_code, 0);
    assert_eq!(outcome.mode, Some(PushMode::Warn));
    let Call::Gates { selection, variables } = fake.gate_calls()[0].clone() else { panic!() };
    assert!(selection.is_empty());
    assert!(!variables.keys().any(|name| name.contains("PRE_PUSH_AREAS")), "{variables:?}");
}

#[test]
fn areas_override_is_passed_through() {
    let (outcome, fake) = push(Fake::new(), "warn", Some("biscuit-file sniff"), &feature(HEAD));
    assert_eq!(outcome.exit_code, 0, "{:#?}", outcome.diagnostics);
    let Call::Gates { selection, .. } = fake.gate_calls()[0].clone() else { panic!() };
    assert_eq!(selection, ["biscuit-file", "sniff"]);
    assert!(said(&fake, "Running pre-push validation for: biscuit-file sniff"));
    // The override narrows the gates only, never the plan the push triggers.
    for call in &fake.calls {
        if let Call::Python { args, .. } = call {
            assert!(!args.iter().any(|arg| arg.contains("biscuit-file")), "{args:?}");
        }
    }
}

// ----------------------------------------------------------- constraints --

#[test]
fn an_unreadable_plan_blocks() {
    let mut fake = Fake::new();
    fake.plans.insert(id(HEAD), "{not json");
    fake.constraint("current.json", &record("wsl2-ubuntu", ""));
    let (outcome, fake) = push(fake, "strict", None, &feature(HEAD));
    assert_ne!(outcome.exit_code, 0);
    let (update, refusals) = refused(&outcome);
    assert_eq!(update, "refs/heads/feature -> refs/heads/feature");
    assert!(refusals[0].contains("review.json"), "{refusals:?}");
    assert!(fake.gate_calls().is_empty() && fake.notes() == 0);
}

#[test]
fn a_prohibition_on_the_pushed_branch_blocks_whatever_is_checked_out() {
    // `feature` is checked out; only `other` is pushed.
    let mut fake = Fake::new();
    fake.plans.insert(id(OTHER), WSL_EXECUTING);
    fake.constraint("other.json", &scoped("branch", "other"));
    let (outcome, fake) = push(fake, "strict", None, &line("refs/heads/other", OTHER, "refs/heads/other", ZERO));
    assert_eq!(outcome.exit_code, 1);
    let (update, refusals) = refused(&outcome);
    assert_eq!(update, "refs/heads/other -> refs/heads/other");
    assert!(refusals[0].contains("do not rerun WSL for branch other"), "{refusals:?}");
    assert_eq!(planned_heads(&fake), [OTHER]);
    assert!(fake.gate_calls().is_empty());
    assert_eq!(fake.calls.last(), Some(&Call::Release));
}

#[test]
fn a_prohibition_on_the_checked_out_branch_does_not_bind_another_branch_push() {
    let mut fake = Fake::new();
    fake.plans.insert(id(OTHER), WSL_EXECUTING);
    fake.constraint("feature.json", &scoped("branch", "feature"));
    let (outcome, fake) = push(fake, "strict", None, &line("refs/heads/other", OTHER, "refs/heads/other", ZERO));
    assert_eq!(outcome.exit_code, 0, "{:#?}", outcome.diagnostics);
    assert!(!outcome.diagnostics.iter().any(is_blocked));
    assert!(fake.emitted.iter().any(|line| line == "Branch identity: other"));
    assert_eq!(fake.gate_calls().len(), 1);
}

// --------------------------------------------------------------- updates --

#[test]
fn a_push_that_does_not_carry_head_reviews_the_pushed_revision() {
    let (outcome, fake) = push(Fake::new(), "scope-only", None, &feature(OTHER));
    assert_eq!(outcome.exit_code, 0, "{:#?}", outcome.diagnostics);
    assert_eq!(outcome.reviews[0].trigger, id(OTHER));
    assert_eq!(planned_heads(&fake), [OTHER]);
    assert_eq!(arguments_files(&fake), [(id(MAIN_TIP), id(OTHER))]);
    assert!(said(&fake, "222222222 is not HEAD"));
    // Scope evidence binds HEAD, and HEAD is not going anywhere.
    assert_eq!(outcome.scope, ScopePublication::NotRecorded);
    assert_eq!(fake.notes(), 0);
    assert_eq!(fake.calls.last(), Some(&Call::Release));
}

#[test]
fn a_deletion_only_push_triggers_nothing_and_is_not_reviewed() {
    let (outcome, fake) = push(Fake::new(), "scope-only", None, &line("(delete)", ZERO, "refs/heads/feature", HEAD));
    assert_eq!(outcome.exit_code, 0);
    assert!(fake.emitted.contains(&"Not reviewing (delete) -> refs/heads/feature: a deletion triggers no run.".to_owned()));
    assert!(said(&fake, "triggers no run and there is no plan to review"));
    assert_eq!(fake.calls, [], "no session boundary was crossed");
}

#[test]
fn a_tag_only_push_in_strict_mode_runs_no_gate() {
    let mut fake = Fake::new();
    fake.gates.code = 1;
    let (outcome, fake) = push(fake, "strict", None, &line("refs/tags/v1", HEAD, "refs/tags/v1", ZERO));
    assert_eq!(outcome.exit_code, 0);
    assert_eq!(outcome.diagnostics.last(), Some(&Diagnostic::NothingToReview));
    assert!(said(&fake, "triggers no run and there is no plan to review"));
    assert_eq!(fake.calls, []);
}

#[test]
fn every_pushed_branch_is_planned_with_its_own_path_set() {
    let mut fake = Fake::new();
    fake.plans.insert(id(HEAD), WSL_EXECUTING);
    fake.plans.insert(id(OTHER), WSL_EXECUTING);
    fake.constraint("other.json", &scoped("branch", "other"));
    let input = [feature(HEAD), line("refs/heads/other", OTHER, "refs/heads/other", ZERO)].concat();
    let (outcome, fake) = push(fake, "strict", None, &input);
    // `feature` passes; the second update is what the prohibition binds.
    assert_eq!(outcome.exit_code, 1);
    let (update, refusals) = refused(&outcome);
    assert_eq!(update, "refs/heads/other -> refs/heads/other");
    assert!(refusals[0].contains("do not rerun WSL for branch other"), "{refusals:?}");
    assert_eq!(planned_heads(&fake), [HEAD, OTHER]);
    assert_eq!(arguments_files(&fake), [(id(MAIN_TIP), id(HEAD)), (id(MAIN_TIP), id(OTHER))]);
    let statuses: Vec<ReviewStatus> = outcome.reviews.iter().map(|review| review.status).collect();
    assert_eq!(statuses, [ReviewStatus::Passed, ReviewStatus::Blocked]);
    // Blocked before publication: the passing HEAD update left no scope note.
    assert!(fake.gate_calls().is_empty());
    assert_eq!(fake.notes(), 0);
    assert_eq!(outcome.scope, ScopePublication::NotRecorded);
}

#[rstest]
#[case::feature_first(false, false)]
#[case::other_first(true, false)]
#[case::feature_first_other_prohibited(false, true)]
#[case::other_first_other_prohibited(true, true)]
fn ref_updates_are_reviewed_in_the_order_git_supplies_them(#[case] other_first: bool, #[case] prohibited: bool) {
    let mut fake = Fake::new();
    fake.plans.insert(id(OTHER), WSL_EXECUTING);
    if prohibited {
        fake.constraint("other.json", &scoped("branch", "other"));
    }
    let mut order = [(HEAD, feature(HEAD)), (OTHER, line("refs/heads/other", OTHER, "refs/heads/other", ZERO))];
    if other_first {
        order.reverse();
    }
    let input: String = order.iter().map(|(_, line)| line.as_str()).collect();
    let (outcome, fake) = push(fake, "scope-only", None, &input);
    if prohibited {
        assert_eq!(outcome.exit_code, 1);
        assert!(refused(&outcome).1[0].contains("do not rerun WSL for branch other"));
    } else {
        assert_eq!(outcome.exit_code, 0, "{:#?}", outcome.diagnostics);
        let expected: Vec<&str> = order.iter().map(|(head, _)| *head).collect();
        assert_eq!(planned_heads(&fake), expected);
        let reviewed: Vec<ObjectId> = outcome.reviews.iter().map(|review| review.trigger).collect();
        assert_eq!(reviewed, expected.iter().map(|head| id(head)).collect::<Vec<_>>());
    }
}

#[test]
fn deletions_and_tags_among_branch_updates_are_skipped_by_name() {
    let input = [
        line("(delete)", ZERO, "refs/heads/stale", MAIN_TIP),
        feature(HEAD),
        line("refs/tags/v1", HEAD, "refs/tags/v1", ZERO),
    ]
    .concat();
    let (outcome, fake) = push(Fake::new(), "scope-only", None, &input);
    assert_eq!(outcome.exit_code, 0, "{:#?}", outcome.diagnostics);
    assert!(said(&fake, "Not reviewing (delete) -> refs/heads/stale: a deletion triggers no run."));
    assert!(said(&fake, "refs/tags/v1 -> refs/tags/v1: only a branch update triggers a run"));
    assert_eq!(planned_heads(&fake), [HEAD]);
    assert!(said(&fake, "Published scope evidence"));
    assert_eq!(outcome.scope, ScopePublication::Published { base: id(MAIN_TIP) });
}

#[test]
fn a_crlf_push_is_reviewed_as_its_lf_spelling_is() {
    let lf = [feature(HEAD), line("refs/heads/other", OTHER, "refs/heads/other", ZERO)].concat();
    let crlf = lf.replace('\n', "\r\n");
    let (expected, _) = push(Fake::new(), "scope-only", None, &lf);
    let (outcome, fake) = push(Fake::new(), "scope-only", None, &crlf);
    assert_eq!(outcome.exit_code, 0, "{:#?}", outcome.diagnostics);
    assert_eq!(outcome.reviews, expected.reviews);
    assert_eq!(planned_heads(&fake), [HEAD, OTHER]);
}

// --------------------------------------------------------- pull requests --

#[test]
fn two_open_pull_requests_from_one_head_are_each_reviewed() {
    let setup = |prohibited: bool| {
        let mut fake = Fake::new();
        fake.url = Some(GITHUB.to_owned());
        fake.gh_by_branch.insert(
            "feature".to_owned(),
            r#"[{"number": 12, "baseRefName": "main"}, {"number": 13, "baseRefName": "parent"}]"#,
        );
        fake.tips.insert("parent".to_owned(), id(BASE_TIP));
        // Only the second context's comparison executes the WSL cell.
        fake.context_plans.insert((id(BASE_TIP), id(HEAD)), WSL_EXECUTING);
        if prohibited {
            fake.constraint("current.json", &record("wsl2-ubuntu", ""));
        }
        push(fake, "scope-only", None, &feature(HEAD))
    };

    let (outcome, fake) = setup(false);
    assert_eq!(outcome.exit_code, 0, "{:#?}", outcome.diagnostics);
    assert_eq!(fake.emitted.iter().filter(|line| line.starts_with("Comparison base for ")).count(), 2);
    assert!(said(&fake, "pull request #12 into main"));
    assert!(said(&fake, "pull request #13 into parent"));
    assert_eq!(arguments_files(&fake), [(id(MAIN_TIP), id(HEAD)), (id(BASE_TIP), id(HEAD))]);
    // One note holds one receipt: the first context's, and the hook says so.
    assert!(said(&fake, "binds its first comparison base only"));
    assert_eq!(outcome.scope, ScopePublication::Published { base: id(MAIN_TIP) });

    // A prohibition only the second context violates still blocks.
    let (outcome, fake) = setup(true);
    assert_eq!(outcome.exit_code, 1);
    assert!(said(&fake, "pull request #12 into main") && said(&fake, "pull request #13 into parent"));
    assert_eq!(refused(&outcome).0, "refs/heads/feature -> refs/heads/feature");
    let statuses: Vec<ReviewStatus> = outcome.reviews.iter().map(|review| review.status).collect();
    assert_eq!(statuses, [ReviewStatus::Passed, ReviewStatus::Blocked]);
    assert_eq!(fake.notes(), 0);
}

/// A child branch (HEAD) with pull request #12 into `parent`, pushed with
/// `parent`, which the same push advances from BASE_TIP to OTHER.
fn simultaneous(child_first: bool, mode: &str, prohibited: bool) -> (crate::ci_push::PushOutcome, Fake) {
    let mut fake = Fake::new();
    fake.url = Some(GITHUB.to_owned());
    fake.gh_by_branch.insert("child".to_owned(), r#"[{"number": 12, "baseRefName": "parent"}]"#);
    fake.tips.insert("parent".to_owned(), id(BASE_TIP));
    // Against the incoming parent the child changes the package, so the WSL
    // cell would execute; against the current parent it does not.
    fake.context_plans.insert((id(OTHER), id(HEAD)), WSL_EXECUTING);
    if prohibited {
        fake.constraint("child.json", &scoped("branch", "child"));
    }
    let parent = line("refs/heads/parent", OTHER, "refs/heads/parent", BASE_TIP);
    let child = line("refs/heads/child", HEAD, "refs/heads/child", ZERO);
    let input = if child_first { [child, parent].concat() } else { [parent, child].concat() };
    push(fake, mode, None, &input)
}

#[rstest]
#[case::parent_first(false)]
#[case::child_first(true)]
fn a_target_updated_in_the_same_push_is_reviewed_at_its_incoming_revision(#[case] child_first: bool) {
    let (outcome, fake) = simultaneous(child_first, "strict", true);
    assert_eq!(outcome.exit_code, 1, "{:#?}", outcome.diagnostics);
    // The current parent is reviewed, and so is the incoming one.
    let files = arguments_files(&fake);
    assert!(files.contains(&(id(BASE_TIP), id(HEAD))) && files.contains(&(id(OTHER), id(HEAD))), "{files:?}");
    assert!(said(&fake, "pull request #12 into parent, the revision this push also writes to parent"));
    let (update, refusals) = refused(&outcome);
    assert_eq!(update, "refs/heads/child -> refs/heads/child");
    assert!(refusals[0].contains("do not rerun WSL for branch child"), "{refusals:?}");
    assert!(fake.gate_calls().is_empty());
    let trees = |wanted: fn(&Call) -> bool| fake.calls.iter().filter(|call| wanted(call)).count();
    assert_eq!(trees(|call| matches!(call, Call::Materialize(_))), trees(|call| matches!(call, Call::Release)));
}

#[rstest]
#[case::parent_first(false)]
#[case::child_first(true)]
fn a_simultaneous_push_with_no_prohibited_work_passes_in_either_order(#[case] child_first: bool) {
    let (outcome, fake) = simultaneous(child_first, "scope-only", false);
    assert_eq!(outcome.exit_code, 0, "{:#?}", outcome.diagnostics);
    assert_eq!(fake.emitted.iter().filter(|line| line.starts_with("Comparison base for 111111111: ")).count(), 2);
    assert!(said(
        &fake,
        "parent is updated by this push too, so the run of pull request #12 may see either its current tip or the incoming revision as its base"
    ));
    assert!(said(&fake, "pull request #12 into parent, the tip of parent on origin"));
    assert!(said(&fake, "pull request #12 into parent, the revision this push also writes to parent"));
    let files = arguments_files(&fake);
    assert!(files.contains(&(id(BASE_TIP), id(HEAD))) && files.contains(&(id(OTHER), id(HEAD))), "{files:?}");
    // The parent has no pull request of its own: one provisional plan.
    assert!(files.contains(&(id(MAIN_TIP), id(OTHER))), "{files:?}");
    assert!(!outcome.diagnostics.iter().any(is_blocked));
    assert!(said(&fake, "binds its first comparison base only"));
    assert_eq!(outcome.scope, ScopePublication::Published { base: id(BASE_TIP) });
}

#[rstest]
#[case::deletion_first(false)]
#[case::child_first(true)]
fn a_pull_request_into_a_branch_this_push_deletes_is_refused(#[case] child_first: bool) {
    let mut fake = Fake::new();
    fake.url = Some(GITHUB.to_owned());
    fake.gh_by_branch.insert("child".to_owned(), r#"[{"number": 12, "baseRefName": "parent"}]"#);
    fake.tips.insert("parent".to_owned(), id(BASE_TIP));
    let deletion = line("(delete)", ZERO, "refs/heads/parent", BASE_TIP);
    let child = line("refs/heads/child", HEAD, "refs/heads/child", ZERO);
    let input = if child_first { [child, deletion].concat() } else { [deletion, child].concat() };
    let (outcome, fake) = push(fake, "scope-only", None, &input);
    assert_eq!(outcome.exit_code, 2);
    let text = problem(&outcome);
    assert!(text.contains("the base of pull request #12 into parent cannot be established"), "{text}");
    assert!(text.contains("this push also deletes parent (refs/heads/parent)"), "{text}");
    assert!(text.contains("--no-verify"), "{text}");
    assert!(fake.helpers().is_empty(), "no plan was resolved");
    // The deletion itself is still skipped by name.
    assert!(said(&fake, "Not reviewing (delete) -> refs/heads/parent: a deletion triggers no run."));
}

#[test]
fn each_update_is_reviewed_against_its_own_pull_requests_base() {
    let mut fake = Fake::new();
    fake.url = Some(GITHUB.to_owned());
    fake.gh_by_branch.insert("feature".to_owned(), r#"[{"number": 1, "baseRefName": "main"}]"#);
    fake.gh_by_branch.insert("second".to_owned(), r#"[{"number": 2, "baseRefName": "develop"}]"#);
    fake.tips.insert("develop".to_owned(), id(BASE_TIP));
    let input = [feature(HEAD), line("refs/heads/second", OTHER, "refs/heads/second", ZERO)].concat();
    let (outcome, fake) = push(fake, "strict", None, &input);
    assert_eq!(outcome.exit_code, 0, "{:#?}", outcome.diagnostics);
    let bases: Vec<(usize, ObjectId)> =
        outcome.reviews.iter().map(|review| (review.context.update, review.context.base)).collect();
    assert_eq!(bases, [(0, id(MAIN_TIP)), (1, id(BASE_TIP))]);
    assert_eq!(arguments_files(&fake), [(id(MAIN_TIP), id(HEAD)), (id(BASE_TIP), id(OTHER))]);
    // HEAD's own context, not the later update's, binds scope, the gates'
    // input, and the validation receipt.
    assert_eq!(outcome.scope, ScopePublication::Published { base: id(MAIN_TIP) });
    let review_plan = value(&fake.python_args("affected_scope.py --apply-to")[0], "--plan-out").to_owned();
    assert_eq!(outcome.gates.as_ref().unwrap().plan_in.as_deref(), Some(std::path::Path::new(&review_plan)));
    assert_eq!(value(&fake.python_args("local_evidence.py record-cells")[0], "--base"), MAIN_TIP);
}
