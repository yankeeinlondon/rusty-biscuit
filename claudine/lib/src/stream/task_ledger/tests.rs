use super::*;

fn labels(outcomes: &[SubagentOutcome]) -> Vec<String> {
    outcomes.iter().map(SubagentOutcome::label).collect()
}

#[test]
fn success_allowlist_recognizes_the_three_documented_spellings() {
    for status in ["completed", "success", "succeeded", "SUCCEEDED", "  success "] {
        assert_eq!(
            terminal_outcome_for_status(Some(status)),
            Some(TaskOutcome::Succeeded),
            "`{status}` should be a recognized success"
        );
    }
}

#[test]
fn stopped_is_terminal_and_unsuccessful() {
    assert_eq!(
        terminal_outcome_for_status(Some("stopped")),
        Some(TaskOutcome::Stopped)
    );
    assert!(TaskOutcome::Stopped.is_incomplete());
    assert!(!TaskOutcome::Succeeded.is_incomplete());
}

#[test]
fn unknown_and_absent_statuses_are_not_terminal_on_their_own() {
    // The signal an event kind without inherent terminality (a
    // `task_notification`) uses to stay progress rather than invent an outcome.
    assert_eq!(terminal_outcome_for_status(Some("thinking")), None);
    assert_eq!(terminal_outcome_for_status(Some("   ")), None);
    assert_eq!(terminal_outcome_for_status(None), None);
}

#[test]
fn an_inherently_terminal_event_with_an_unknown_status_is_unresolved() {
    let mut ledger = TaskLedger::new();
    ledger.record_terminal(Some("t1"), Some("weird"), Some("evaporated"));
    let incomplete = ledger.incomplete_outcomes();
    assert_eq!(incomplete.len(), 1);
    assert_eq!(incomplete[0].outcome, TaskOutcome::UnknownStatus);
    // The raw vocabulary survives verbatim so the gap can be diagnosed.
    assert_eq!(incomplete[0].raw_status.as_deref(), Some("evaporated"));
}

#[test]
fn an_inherently_terminal_event_with_no_status_is_unresolved_not_success() {
    let mut ledger = TaskLedger::new();
    ledger.record_terminal(Some("t1"), None, None);
    let incomplete = ledger.incomplete_outcomes();
    assert_eq!(incomplete.len(), 1);
    assert_eq!(incomplete[0].outcome, TaskOutcome::UnknownStatus);
    assert!(incomplete[0].raw_status.is_none());
}

#[test]
fn clean_completion_leaves_the_ledger_with_nothing_to_report() {
    let mut ledger = TaskLedger::new();
    ledger.record_start(Some("t1"), Some("alpha"));
    ledger.record_start(Some("t2"), Some("beta"));
    ledger.record_terminal(Some("t1"), Some("alpha"), Some("completed"));
    ledger.record_terminal(Some("t2"), Some("beta"), Some("success"));
    assert!(ledger.incomplete_outcomes().is_empty());

    let mut summary = StreamExecutionSummary::default();
    ledger.apply_to_summary(&mut summary);
    assert!(!summary.is_error);
    assert!(summary.error_kind.is_none());
    assert!(summary.subagent_outcomes.is_empty());
}

#[test]
fn more_than_five_stopped_tasks_are_all_retained() {
    // The CLI watchdog's diagnostic ring holds five; the ledger must not.
    let mut ledger = TaskLedger::new();
    for index in 0..9 {
        let id = format!("task-{index}");
        let name = format!("agent-{index}");
        ledger.record_start(Some(&id), Some(&name));
        ledger.record_terminal(Some(&id), Some(&name), Some("stopped"));
    }
    let incomplete = ledger.incomplete_outcomes();
    assert_eq!(incomplete.len(), 9);
    assert_eq!(labels(&incomplete)[0], "agent-0");
    assert_eq!(labels(&incomplete)[8], "agent-8");
}

#[test]
fn duplicate_events_for_one_id_do_not_create_duplicate_facts() {
    let mut ledger = TaskLedger::new();
    ledger.record_start(Some("t1"), Some("alpha"));
    ledger.record_start(Some("t1"), Some("alpha"));
    ledger.record_terminal(Some("t1"), Some("alpha"), Some("stopped"));
    ledger.record_terminal(Some("t1"), Some("alpha"), Some("stopped"));
    ledger.record_terminal(Some("t1"), Some("alpha"), Some("stopped"));
    assert_eq!(ledger.incomplete_outcomes().len(), 1);
}

#[test]
fn anonymous_observations_stay_distinct_and_never_clear_one_another() {
    let mut ledger = TaskLedger::new();
    ledger.record_terminal(None, Some("first"), Some("stopped"));
    ledger.record_terminal(None, Some("second"), Some("stopped"));
    // A success with no ID cannot clear either of the two above.
    ledger.record_terminal(None, Some("third"), Some("completed"));

    let incomplete = ledger.incomplete_outcomes();
    assert_eq!(incomplete.len(), 2);
    assert_eq!(labels(&incomplete), vec!["first", "second"]);
    // Missing IDs are never collapsed into the empty-string ID, and the
    // internal anonymous identity is never leaked as a provider ID.
    assert!(incomplete.iter().all(|fact| fact.task_id.is_none()));
}

#[test]
fn stopped_then_success_for_the_same_id_succeeds() {
    let mut ledger = TaskLedger::new();
    ledger.record_start(Some("t1"), Some("alpha"));
    ledger.record_terminal(Some("t1"), Some("alpha"), Some("stopped"));
    ledger.record_terminal(Some("t1"), Some("alpha"), Some("completed"));
    assert!(ledger.incomplete_outcomes().is_empty());
}

#[test]
fn a_same_named_different_id_success_does_not_clear_a_stopped_task() {
    let mut ledger = TaskLedger::new();
    ledger.record_start(Some("task-a"), Some("commit the work"));
    ledger.record_terminal(Some("task-a"), Some("commit the work"), Some("stopped"));
    // Same prose label, different provider identity: not the same work.
    ledger.record_start(Some("task-b"), Some("commit the work"));
    ledger.record_terminal(Some("task-b"), Some("commit the work"), Some("completed"));

    let incomplete = ledger.incomplete_outcomes();
    assert_eq!(incomplete.len(), 1);
    assert_eq!(incomplete[0].task_id.as_deref(), Some("task-a"));
    assert_eq!(incomplete[0].outcome, TaskOutcome::Stopped);
}

#[test]
fn a_started_task_with_no_terminal_observation_is_incomplete() {
    let mut ledger = TaskLedger::new();
    ledger.record_start(Some("t1"), Some("alpha"));
    ledger.record_start(Some("t2"), Some("beta"));
    ledger.record_terminal(Some("t2"), Some("beta"), Some("completed"));

    let incomplete = ledger.incomplete_outcomes();
    assert_eq!(incomplete.len(), 1);
    assert_eq!(incomplete[0].task_id.as_deref(), Some("t1"));
    assert_eq!(incomplete[0].outcome, TaskOutcome::Unfinished);
    assert!(incomplete[0].raw_status.is_none());
}

#[test]
fn restarting_a_completed_id_puts_it_back_in_flight() {
    let mut ledger = TaskLedger::new();
    ledger.record_start(Some("t1"), Some("alpha"));
    ledger.record_terminal(Some("t1"), Some("alpha"), Some("completed"));
    ledger.record_start(Some("t1"), Some("alpha"));

    let incomplete = ledger.incomplete_outcomes();
    assert_eq!(incomplete.len(), 1);
    assert_eq!(incomplete[0].outcome, TaskOutcome::Unfinished);
}

#[test]
fn applying_an_incomplete_ledger_poisons_success_without_touching_the_exit_code() {
    let mut ledger = TaskLedger::new();
    ledger.record_start(Some("t1"), Some("commit-a"));
    ledger.record_start(Some("t2"), Some("commit-b"));
    ledger.record_terminal(Some("t1"), Some("commit-a"), Some("stopped"));
    ledger.record_terminal(Some("t2"), Some("commit-b"), Some("stopped"));

    let mut summary = StreamExecutionSummary {
        exit_code: 0,
        ..Default::default()
    };
    ledger.apply_to_summary(&mut summary);

    assert!(summary.is_error);
    assert_eq!(
        summary.error_kind.as_deref(),
        Some(INCOMPLETE_SUBAGENTS_ERROR_KIND)
    );
    // Claudine did not kill the child; the native exit stays honest.
    assert_eq!(summary.exit_code, 0);
    assert_eq!(summary.subagent_outcomes.len(), 2);
    let message = summary.error_message.unwrap();
    assert!(message.contains("2 sub-agent tasks did not complete"), "{message}");
    assert!(message.contains("commit-a"), "{message}");
    assert!(message.contains("commit-b"), "{message}");
}

#[test]
fn a_more_specific_provider_error_kind_survives_ledger_finalization() {
    let mut ledger = TaskLedger::new();
    ledger.record_start(Some("t1"), Some("alpha"));

    let mut summary = StreamExecutionSummary {
        is_error: true,
        error_kind: Some("rate_limit".into()),
        error_message: Some("Too many requests".into()),
        ..Default::default()
    };
    ledger.apply_to_summary(&mut summary);

    assert_eq!(summary.error_kind.as_deref(), Some("rate_limit"));
    assert_eq!(summary.error_message.as_deref(), Some("Too many requests"));
    // The facts are still recorded even though the headline stays.
    assert_eq!(summary.subagent_outcomes.len(), 1);
}

#[test]
fn the_headline_names_tasks_while_the_machine_list_stays_complete() {
    let mut ledger = TaskLedger::new();
    for index in 0..40 {
        let id = format!("task-{index}");
        let name = format!("a-very-long-sub-agent-task-name-number-{index}");
        ledger.record_start(Some(&id), Some(&name));
        ledger.record_terminal(Some(&id), Some(&name), Some("stopped"));
    }
    let incomplete = ledger.incomplete_outcomes();
    let message = incomplete_message(&incomplete);
    assert!(message.starts_with("40 sub-agent tasks did not complete:"), "{message}");

    let mut summary = StreamExecutionSummary::default();
    ledger.apply_to_summary(&mut summary);
    // Whatever the headline does, every fact is retained.
    assert_eq!(summary.subagent_outcomes.len(), 40);
}

#[test]
fn a_fact_without_a_name_falls_back_to_the_provider_id_then_a_placeholder() {
    let with_id = SubagentOutcome {
        task_id: Some("task-7".into()),
        name: None,
        outcome: TaskOutcome::Stopped,
        raw_status: Some("stopped".into()),
    };
    assert_eq!(with_id.label(), "task-7");
    assert_eq!(with_id.describe(), "task-7 (stopped)");

    let anonymous = SubagentOutcome {
        task_id: None,
        name: None,
        outcome: TaskOutcome::Unfinished,
        raw_status: None,
    };
    assert_eq!(anonymous.label(), "unnamed task");
    assert_eq!(anonymous.describe(), "unnamed task (never finished)");
}

#[test]
fn an_empty_string_task_id_is_treated_as_anonymous() {
    let mut ledger = TaskLedger::new();
    ledger.record_terminal(Some(""), Some("first"), Some("stopped"));
    ledger.record_terminal(Some(""), Some("second"), Some("stopped"));
    // Both survive: `""` is not an identity that can reconcile.
    let incomplete = ledger.incomplete_outcomes();
    assert_eq!(incomplete.len(), 2);
    assert!(incomplete.iter().all(|fact| fact.task_id.is_none()));
}
