//! Review-row eligibility, baseline targets, the row-to-step merge, and when
//! the review screen opens.

use std::cell::Cell;
use std::path::PathBuf;

use biscuit_tui::prelude::{ABORTED_KIND, CANCELLED_KIND};
use claudine::composition::{
    AgentResolutionState, ExecutableField, ModelResolutionReason, ProviderPickerOption, ProviderPickerPlan,
    ProviderResolutionReason, ResolvedCompositionSource, ResolvedExecutionTarget, SequenceStep,
    SequenceStepDraft, StepExecutable, StepState, resolve_sequence_plan,
};
use claudine::provider::Provider;
use serde_json::{Map, Value};

use super::*;

/// The sequence from the original report. The shipped file has 22 steps (the
/// report counted 21 against an earlier revision), five of them `stage-N`
/// shell steps.
const REVIEW_LOOP: &str = include_str!("../../../../../../../prompts/review-loop.md");

fn step(index: usize, name: &str, field: Option<ExecutableField>) -> SequenceStep {
    SequenceStep {
        index,
        name: name.into(),
        raw_state: Value::String(name.into()),
        state: StepState {
            name: name.into(),
            id: name.into(),
            sequence_id: "seq".into(),
            is_first: index == 0,
            is_last: false,
            index: index + 1,
            count: 0,
            extra: Map::new(),
        },
        executable: field.map(|field| StepExecutable {
            field,
            value: Value::Null,
            options: Map::new(),
        }),
    }
}

/// Steps built from `(name, executable)` pairs, indexed in order.
fn steps(spec: &[(&str, Option<ExecutableField>)]) -> Vec<SequenceStep> {
    spec.iter()
        .enumerate()
        .map(|(i, (name, field))| step(i, name, *field))
        .collect()
}

fn draft(step_index: usize, name: &str) -> SequenceStepDraft {
    SequenceStepDraft {
        step_index,
        step_name: name.into(),
        provider_plan: ProviderPickerPlan {
            options: vec![
                ProviderPickerOption {
                    provider: Provider::Claude,
                    rank_reason: None,
                },
                ProviderPickerOption {
                    provider: Provider::Codex,
                    rank_reason: None,
                },
            ],
            default_index: 0,
        },
        proposed_model: Some(format!("baseline-{step_index}")),
        model_reason: ModelResolutionReason::FrontmatterSingle,
        provider_locked: false,
        model_locked: false,
        resolved_provider: Some(Provider::Claude),
    }
}

fn drafts_for(steps: &[SequenceStep]) -> Vec<SequenceStepDraft> {
    steps.iter().map(|s| draft(s.index, &s.name)).collect()
}

/// A reviewed target distinguishable from every baseline entry.
fn reviewed(step_index: usize) -> ResolvedExecutionTarget {
    ResolvedExecutionTarget {
        provider: if step_index.is_multiple_of(2) {
            Provider::Codex
        } else {
            Provider::Gemini
        },
        provider_reason: ProviderResolutionReason::SequenceReview,
        model: Some(format!("reviewed-{step_index}")),
        model_reason: ModelResolutionReason::SequenceReview,
    }
}

/// `(step_index, step_name)` of each row the review callback was shown.
type ShownRows = Option<Vec<(usize, String)>>;

/// Runs `review_targets` with a callback that records the rows it was shown
/// and answers each with [`reviewed`].
fn review_all(steps: &[SequenceStep]) -> (io::Result<Vec<ResolvedExecutionTarget>>, ShownRows) {
    let drafts = drafts_for(steps);
    let baseline = baseline_targets(&drafts, ProviderResolutionReason::FrontmatterSingle);
    let mut shown = None;
    let result = review_targets(steps, drafts, baseline, |rows| {
        shown = Some(
            rows.iter()
                .map(|d| (d.step_index, d.step_name.clone()))
                .collect::<Vec<_>>(),
        );
        Ok(rows.iter().map(|d| reviewed(d.step_index)).collect())
    });
    (result, shown)
}

fn baseline_for(steps: &[SequenceStep]) -> Vec<ResolvedExecutionTarget> {
    baseline_targets(&drafts_for(steps), ProviderResolutionReason::FrontmatterSingle)
}

// -- Eligibility ------------------------------------------------------------

#[test]
fn eligibility_follows_the_outer_executable() {
    let cases = [
        (None, true),
        (Some(ExecutableField::Prompt), true),
        (Some(ExecutableField::Task), true),
        (Some(ExecutableField::Group), true),
        (Some(ExecutableField::Shell), false),
        (Some(ExecutableField::SideEffect), false),
    ];
    for (field, expected) in cases {
        assert_eq!(
            is_review_eligible(&step(0, "s", field)),
            expected,
            "{field:?}"
        );
    }
}

#[test]
fn shipped_review_loop_hides_exactly_its_stage_steps() {
    let path = PathBuf::from("prompts/review-loop.md");
    let source = ResolvedCompositionSource {
        original_ref: path.display().to_string(),
        resolved_path: path,
        original_text: REVIEW_LOOP.to_string(),
        markdown: REVIEW_LOOP.to_string().into(),
    };
    let plan = resolve_sequence_plan(&source)
        .expect("review-loop.md normalizes")
        .expect("review-loop.md declares a sequence");
    assert_eq!(plan.steps.len(), 22);

    let hidden: Vec<&str> = plan
        .steps
        .iter()
        .filter(|s| !is_review_eligible(s))
        .map(|s| s.name.as_str())
        .collect();
    assert_eq!(hidden, ["stage-1", "stage-2", "stage-3", "stage-4", "stage-5"]);

    let (result, shown) = review_all(&plan.steps);
    let shown = shown.expect("the review opens");
    assert_eq!(shown.len(), 17);
    assert_eq!(shown[0], (0, "implement".to_string()));
    assert_eq!(shown[3], (4, "commit-1".to_string()));
    assert_eq!(result.expect("review succeeds").len(), 22);
}

// -- Baseline ---------------------------------------------------------------

#[test]
fn baseline_targets_match_the_review_bypassed_conversion() {
    let mut unresolved = draft(1, "unresolved");
    unresolved.resolved_provider = None;
    unresolved.proposed_model = None;
    unresolved.model_reason = ModelResolutionReason::ProviderDefault;
    let drafts = vec![draft(0, "resolved"), unresolved];

    let targets = baseline_targets(&drafts, ProviderResolutionReason::FrontmatterList);
    assert_eq!(
        targets,
        vec![
            ResolvedExecutionTarget {
                provider: Provider::Claude,
                provider_reason: ProviderResolutionReason::FrontmatterList,
                model: Some("baseline-0".into()),
                model_reason: ModelResolutionReason::FrontmatterSingle,
            },
            // No resolved provider falls back to Claude, as the bypass did.
            ResolvedExecutionTarget {
                provider: Provider::Claude,
                provider_reason: ProviderResolutionReason::FrontmatterList,
                model: None,
                model_reason: ModelResolutionReason::ProviderDefault,
            },
        ]
    );
}

// -- Mapping ----------------------------------------------------------------

#[test]
fn interleaved_steps_review_only_eligible_rows_and_merge_by_step_index() {
    let steps = steps(&[
        ("plan", Some(ExecutableField::Prompt)),
        ("stage", Some(ExecutableField::Shell)),
        ("notify", Some(ExecutableField::SideEffect)),
        ("lint", Some(ExecutableField::Task)),
        ("fan-out", Some(ExecutableField::Group)),
        ("body", None),
    ]);
    let baseline = baseline_for(&steps);
    let (result, shown) = review_all(&steps);

    assert_eq!(
        shown.expect("the review opens"),
        vec![
            (0, "plan".to_string()),
            (3, "lint".to_string()),
            (4, "fan-out".to_string()),
            (5, "body".to_string()),
        ]
    );
    let targets = result.expect("review succeeds");
    assert_eq!(targets.len(), 6);
    for index in [0, 3, 4, 5] {
        assert_eq!(targets[index], reviewed(index), "reviewed step {index}");
    }
    for index in [1, 2] {
        assert_eq!(targets[index], baseline[index], "hidden step {index}");
    }
}

#[test]
fn repeated_names_map_by_position_not_name() {
    let steps = steps(&[
        ("x", Some(ExecutableField::Prompt)),
        ("x", Some(ExecutableField::Shell)),
        ("x", Some(ExecutableField::Prompt)),
    ]);
    let baseline = baseline_for(&steps);
    let (result, shown) = review_all(&steps);

    assert_eq!(
        shown.expect("the review opens"),
        vec![(0, "x".to_string()), (2, "x".to_string())]
    );
    let targets = result.expect("review succeeds");
    assert_eq!(targets, vec![reviewed(0), baseline[1].clone(), reviewed(2)]);
}

#[test]
fn hidden_first_and_last_steps_keep_their_baseline() {
    let steps = steps(&[
        ("stage", Some(ExecutableField::Shell)),
        ("implement", Some(ExecutableField::Prompt)),
        ("body", None),
        ("notify", Some(ExecutableField::SideEffect)),
    ]);
    let baseline = baseline_for(&steps);
    let (result, shown) = review_all(&steps);

    assert_eq!(
        shown.expect("the review opens"),
        vec![(1, "implement".to_string()), (2, "body".to_string())]
    );
    let targets = result.expect("review succeeds");
    assert_eq!(
        targets,
        vec![baseline[0].clone(), reviewed(1), reviewed(2), baseline[3].clone()]
    );
}

#[test]
fn all_hidden_steps_never_open_the_review() {
    let steps = steps(&[
        ("stage", Some(ExecutableField::Shell)),
        ("notify", Some(ExecutableField::SideEffect)),
    ]);
    let drafts = drafts_for(&steps);
    let baseline = baseline_targets(&drafts, ProviderResolutionReason::FrontmatterSingle);
    let called = Cell::new(false);

    let targets = review_targets(&steps, drafts, baseline.clone(), |_| {
        called.set(true);
        Ok(Vec::new())
    })
    .expect("no review, no error");

    assert!(!called.get(), "an empty review table was opened");
    assert_eq!(targets, baseline);
}

#[test]
fn unexpected_row_count_is_rejected_before_merging() {
    let steps = steps(&[
        ("a", Some(ExecutableField::Prompt)),
        ("stage", Some(ExecutableField::Shell)),
        ("b", Some(ExecutableField::Prompt)),
    ]);

    for returned in [vec![reviewed(0)], vec![reviewed(0), reviewed(2), reviewed(4)]] {
        let found = returned.len();
        let drafts = drafts_for(&steps);
        let baseline = baseline_targets(&drafts, ProviderResolutionReason::FrontmatterSingle);
        let error = review_targets(&steps, drafts, baseline, |_| Ok(returned))
            .expect_err("a short or long review must not merge");
        assert_eq!(error.kind(), io::ErrorKind::InvalidData, "{found} rows");
        assert_ne!(error.kind(), CANCELLED_KIND);
        let message = error.to_string();
        assert!(message.contains('2') && message.contains(&found.to_string()), "{message}");
    }
}

#[test]
fn merge_rejects_a_count_mismatch_without_touching_the_baseline() {
    let steps = steps(&[("a", None), ("b", None)]);
    let baseline = baseline_for(&steps);
    assert_eq!(
        merge_reviewed_targets(baseline, &[0, 1], vec![reviewed(0)]),
        Err(ReviewRowCountMismatch {
            expected: 2,
            found: 1,
        })
    );
}

#[test]
fn review_cancellation_propagates_its_kind() {
    let steps = steps(&[("a", Some(ExecutableField::Prompt))]);
    let drafts = drafts_for(&steps);
    let baseline = baseline_targets(&drafts, ProviderResolutionReason::FrontmatterSingle);
    let error = review_targets(&steps, drafts, baseline, |_| {
        Err(io::Error::new(CANCELLED_KIND, "interrupted"))
    })
    .expect_err("cancellation is an error");
    assert_eq!(error.kind(), CANCELLED_KIND);
}

// -- Opening the review -----------------------------------------------------

#[test]
fn review_opens_only_for_a_prompting_state_on_a_terminal() {
    let prompting = [
        AgentResolutionState::NoAgent,
        AgentResolutionState::SingleInvalid {
            hint: "not-real".into(),
        },
        AgentResolutionState::SingleNotInstalled {
            provider: Provider::Gemini,
        },
        AgentResolutionState::ListMultipleInstalled {
            installed: vec![Provider::Claude, Provider::Codex],
            not_installed: Vec::new(),
            invalid: Vec::new(),
        },
        AgentResolutionState::ZeroInstalledList {
            not_installed: vec![Provider::Gemini],
            invalid: Vec::new(),
        },
    ];
    for state in &prompting {
        assert!(needs_review(true, Some(state)), "{state:?} on a terminal");
        assert!(!needs_review(false, Some(state)), "{state:?} without a terminal");
    }

    let auto_selected = [
        AgentResolutionState::Selected {
            provider: Provider::Claude,
        },
        AgentResolutionState::ListOneInstalled {
            selected: Provider::Codex,
            not_installed: Vec::new(),
            invalid: Vec::new(),
        },
    ];
    for state in &auto_selected {
        assert!(!needs_review(true, Some(state)), "{state:?}");
    }

    // An explicit `--<provider>` flag leaves no classified state.
    assert!(!needs_review(true, None));
}

#[test]
fn a_bypassed_review_never_calls_the_callback_and_keeps_the_baseline() {
    let steps = steps(&[
        ("plan", Some(ExecutableField::Prompt)),
        ("stage", Some(ExecutableField::Shell)),
    ]);
    let drafts = drafts_for(&steps);
    let called = Cell::new(false);

    let targets = live_targets(
        false,
        &steps,
        drafts.clone(),
        ProviderResolutionReason::ExplicitFlag,
        |_| {
            called.set(true);
            Ok(Vec::new())
        },
    )
    .expect("no review, no error");

    assert!(!called.get(), "a bypassed review opened the table");
    assert_eq!(
        targets,
        Some(baseline_targets(&drafts, ProviderResolutionReason::ExplicitFlag))
    );
}

#[test]
fn a_shell_only_sequence_on_a_terminal_never_opens_the_review() {
    let steps = steps(&[
        ("stage", Some(ExecutableField::Shell)),
        ("notify", Some(ExecutableField::SideEffect)),
    ]);
    let drafts = drafts_for(&steps);
    let called = Cell::new(false);

    let targets = live_targets(
        true,
        &steps,
        drafts.clone(),
        ProviderResolutionReason::FrontmatterSingle,
        |_| {
            called.set(true);
            Ok(Vec::new())
        },
    )
    .expect("no review, no error");

    assert!(!called.get(), "an empty review table was opened");
    let targets = targets.expect("not cancelled");
    assert_eq!(targets.len(), 2);
    assert_eq!(
        targets,
        baseline_targets(&drafts, ProviderResolutionReason::FrontmatterSingle)
    );
}

#[test]
fn leaving_the_review_with_esc_or_ctrl_c_yields_no_targets() {
    let steps = steps(&[
        ("a", Some(ExecutableField::Prompt)),
        ("stage", Some(ExecutableField::Shell)),
    ]);
    // `Esc` aborts and `Ctrl+C` cancels; both mean no step may start.
    for (kind, message) in [(ABORTED_KIND, "cancelled"), (CANCELLED_KIND, "interrupted")] {
        let outcome = live_targets(
            true,
            &steps,
            drafts_for(&steps),
            ProviderResolutionReason::FrontmatterSingle,
            |_| Err(io::Error::new(kind, message)),
        )
        .expect("cancellation is not an error");
        assert_eq!(outcome, None, "{kind:?}");
    }
}

#[test]
fn a_failed_review_is_an_error_not_a_cancellation() {
    let steps = steps(&[("a", Some(ExecutableField::Prompt))]);
    let error = live_targets(
        true,
        &steps,
        drafts_for(&steps),
        ProviderResolutionReason::FrontmatterSingle,
        |_| Ok(Vec::new()),
    )
    .expect_err("a zero-row answer for one shown row must fail");
    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
}

#[test]
fn a_wrong_row_count_with_hidden_steps_fails_the_run_instead_of_cancelling_or_truncating() {
    let steps = steps(&[
        ("stage-1", Some(ExecutableField::Shell)),
        ("plan", Some(ExecutableField::Prompt)),
        ("notify", Some(ExecutableField::SideEffect)),
        ("build", Some(ExecutableField::Task)),
    ]);
    // Two rows are shown. Four is the full step count, the answer a review
    // that ignored the filter would give.
    for found in [0, 1, 3, 4, 5] {
        let error = live_targets(
            true,
            &steps,
            drafts_for(&steps),
            ProviderResolutionReason::FrontmatterSingle,
            |rows| {
                assert_eq!(rows.len(), 2);
                Ok((0..found).map(reviewed).collect())
            },
        )
        .expect_err("a mismatched answer is neither targets nor a cancellation");
        assert_eq!(error.kind(), io::ErrorKind::InvalidData, "{found} rows");
        assert_eq!(
            error.get_ref().and_then(|source| source.downcast_ref::<ReviewRowCountMismatch>()),
            Some(&ReviewRowCountMismatch { expected: 2, found }),
        );
    }
}

#[test]
fn a_submitted_review_returns_one_target_per_step() {
    let steps = steps(&[
        ("stage", Some(ExecutableField::Shell)),
        ("plan", Some(ExecutableField::Prompt)),
    ]);
    let drafts = drafts_for(&steps);
    let baseline = baseline_targets(&drafts, ProviderResolutionReason::FrontmatterSingle);

    let targets = live_targets(
        true,
        &steps,
        drafts,
        ProviderResolutionReason::FrontmatterSingle,
        |rows| Ok(rows.iter().map(|d| reviewed(d.step_index)).collect()),
    )
    .expect("review succeeds")
    .expect("not cancelled");

    assert_eq!(targets, vec![baseline[0].clone(), reviewed(1)]);
}
