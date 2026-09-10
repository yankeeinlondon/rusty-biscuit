use super::*;

fn labels(outcomes: &[SubagentOutcome]) -> Vec<String> {
    outcomes.iter().map(SubagentOutcome::label).collect()
}

#[test]
fn success_allowlist_recognizes_the_three_documented_spellings() {
    for status in ["completed", "success", "succeeded", "SUCCEEDED", "  success "] {
        assert_eq!(
            route_notification_status(Some(status)),
            NotificationRouting::Terminal(TaskOutcome::Succeeded),
            "`{status}` should be a recognized success"
        );
    }
}

#[test]
fn stopped_is_terminal_and_unsuccessful() {
    assert_eq!(
        route_notification_status(Some("stopped")),
        NotificationRouting::Terminal(TaskOutcome::Stopped)
    );
    assert!(TaskOutcome::Stopped.is_incomplete());
    assert!(!TaskOutcome::Succeeded.is_incomplete());
}

#[test]
fn an_absent_or_blank_notification_status_routes_to_progress() {
    // Branch 1: the provider said nothing, so there is nothing to preserve.
    assert_eq!(
        route_notification_status(None),
        NotificationRouting::Progress
    );
    assert_eq!(
        route_notification_status(Some("   ")),
        NotificationRouting::Progress
    );
    assert_eq!(
        route_notification_status(Some("")),
        NotificationRouting::Progress
    );
}

#[test]
fn every_progress_word_routes_to_progress() {
    // Branch 3: an explicit in-flight vocabulary, not a fallback.
    for status in PROGRESS_STATUSES {
        assert_eq!(
            route_notification_status(Some(status)),
            NotificationRouting::Progress,
            "`{status}` should be a recognized progress word"
        );
    }
    // Normalization applies to this vocabulary too.
    assert_eq!(
        route_notification_status(Some("  THINKING ")),
        NotificationRouting::Progress
    );
}

#[test]
fn a_present_but_unrecognized_notification_status_is_terminal_and_unresolved() {
    // Branch 4, the fail-closed residue: an uninterpretable word must not be
    // read as either progress or success.
    for status in ["evaporated", "vaporized", "  Evaporated  "] {
        assert_eq!(
            route_notification_status(Some(status)),
            NotificationRouting::Terminal(TaskOutcome::UnknownStatus),
            "`{status}` should fail closed"
        );
    }
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
fn a_padded_status_is_classified_normalized_but_stored_verbatim() {
    // Review-3 finding 2: trim/case normalization is for matching only. The
    // stored fact must be the authored bytes so the live `SubagentStop.status`
    // and the final machine data never disagree.
    let mut ledger = TaskLedger::new();
    ledger.record_terminal(Some("t1"), Some("alpha"), Some("  Evaporated  "));
    ledger.record_terminal(Some("t2"), Some("beta"), Some("  stopped "));
    let incomplete = ledger.incomplete_outcomes();
    assert_eq!(incomplete.len(), 2);
    assert_eq!(incomplete[0].outcome, TaskOutcome::UnknownStatus);
    assert_eq!(incomplete[0].raw_status.as_deref(), Some("  Evaporated  "));
    assert_eq!(incomplete[1].outcome, TaskOutcome::Stopped);
    assert_eq!(incomplete[1].raw_status.as_deref(), Some("  stopped "));
    // Display still trims; verbatim storage must not leak padding there.
    assert_eq!(incomplete[0].describe(), "alpha (Evaporated)");
    assert_eq!(incomplete[1].describe(), "beta (stopped)");
}

#[test]
fn a_whitespace_only_terminal_status_is_stored_verbatim_and_described_by_outcome() {
    let mut ledger = TaskLedger::new();
    ledger.record_terminal(Some("t1"), Some("alpha"), Some("   "));
    let incomplete = ledger.incomplete_outcomes();
    assert_eq!(incomplete.len(), 1);
    assert_eq!(incomplete[0].outcome, TaskOutcome::UnknownStatus);
    assert_eq!(incomplete[0].raw_status.as_deref(), Some("   "));
    assert_eq!(incomplete[0].describe(), "alpha (unknown status)");
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
fn incomplete_work_displaces_an_earlier_provider_error_kind_but_keeps_its_text() {
    // Spec §4: `incomplete_subagents` is the stable machine-facing exit reason
    // whenever unresolved work remains, so an earlier parser error kind cannot
    // hold the slot. The displaced failure is operationally valuable and
    // `error_message` is the only place it still has, so the composed headline
    // must name both.
    let mut ledger = TaskLedger::new();
    ledger.record_start(Some("t1"), Some("alpha"));

    let mut summary = StreamExecutionSummary {
        is_error: true,
        error_kind: Some("rate_limit".into()),
        error_message: Some("Too many requests".into()),
        ..Default::default()
    };
    ledger.apply_to_summary(&mut summary);

    assert_eq!(
        summary.error_kind.as_deref(),
        Some(INCOMPLETE_SUBAGENTS_ERROR_KIND)
    );
    let message = summary.error_message.as_deref().unwrap();
    assert!(
        message.contains("1 sub-agent task did not complete"),
        "{message}"
    );
    assert!(message.contains("alpha"), "{message}");
    assert!(message.contains("rate_limit"), "{message}");
    assert!(message.contains("Too many requests"), "{message}");
    assert_eq!(summary.subagent_outcomes.len(), 1);
}

#[test]
fn a_displaced_provider_failure_without_text_is_still_named() {
    let mut ledger = TaskLedger::new();
    ledger.record_terminal(Some("t1"), Some("alpha"), Some("stopped"));

    let mut summary = StreamExecutionSummary {
        is_error: true,
        error_kind: Some("repeated_stream_error".into()),
        ..Default::default()
    };
    ledger.apply_to_summary(&mut summary);

    let message = summary.error_message.as_deref().unwrap();
    assert!(message.contains("repeated_stream_error"), "{message}");
}

#[test]
fn the_composed_message_still_honors_the_concise_message_contract() {
    // The displaced text is arbitrary provider output: multi-line, escape
    // bearing, and far past the 240-character budget. The composed headline
    // must still be one clamped line, and must still name the displaced
    // failure — the reservation exists so the task list is what gives way.
    let mut ledger = TaskLedger::new();
    for index in 0..30 {
        let id = format!("task-{index}");
        let name = format!("a-very-long-sub-agent-task-name-number-{index}");
        ledger.record_start(Some(&id), Some(&name));
        ledger.record_terminal(Some(&id), Some(&name), Some("stopped"));
    }

    let mut summary = StreamExecutionSummary {
        is_error: true,
        error_kind: Some("rate_limit".into()),
        error_message: Some(format!(
            "\u{1b}[31mToo many requests\u{1b}[0m\nretry later\n{}",
            "x".repeat(500)
        )),
        ..Default::default()
    };
    ledger.apply_to_summary(&mut summary);

    let message = summary.error_message.as_deref().unwrap();
    assert!(!message.contains('\u{1b}'), "escapes leaked: {message:?}");
    assert!(
        !message.contains('\n') && !message.contains('\r'),
        "multi-line: {message:?}"
    );
    assert!(
        message.chars().count() <= 240,
        "{} chars: {message}",
        message.chars().count()
    );
    assert!(
        message.starts_with("30 sub-agent tasks did not complete"),
        "{message}"
    );
    assert!(message.contains("rate_limit"), "{message}");
    assert!(message.contains("Too many requests"), "{message}");
    // Non-vacuity: the list genuinely had to truncate, which is the direction
    // the reservation guarantees.
    assert!(
        !message.contains("a-very-long-sub-agent-task-name-number-29"),
        "{message}"
    );
    assert_eq!(summary.subagent_outcomes.len(), 30);
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

#[test]
fn a_message_only_provider_failure_is_preserved_in_the_headline() {
    // Review-3 finding 1: Claude's `result.is_error=true` records text with no
    // error kind. That text is the actionable cause and must survive
    // finalization even though no kind exists to hang it on.
    let mut ledger = TaskLedger::new();
    ledger.record_start(Some("t1"), Some("alpha"));
    ledger.record_terminal(Some("t1"), Some("alpha"), Some("Evaporated"));

    let mut summary = StreamExecutionSummary {
        is_error: true,
        error_message: Some("Provider rejected the request".into()),
        ..Default::default()
    };
    ledger.apply_to_summary(&mut summary);

    assert_eq!(
        summary.error_kind.as_deref(),
        Some(INCOMPLETE_SUBAGENTS_ERROR_KIND)
    );
    let message = summary.error_message.as_deref().unwrap();
    assert!(
        message.contains("after provider failure: Provider rejected the request"),
        "{message}"
    );
    assert!(message.contains("alpha"), "{message}");
}

#[test]
fn a_blank_error_kind_with_a_message_reads_as_message_only() {
    let mut ledger = TaskLedger::new();
    ledger.record_terminal(Some("t1"), Some("alpha"), Some("stopped"));

    let mut summary = StreamExecutionSummary {
        is_error: true,
        error_kind: Some("   ".into()),
        error_message: Some("Provider rejected the request".into()),
        ..Default::default()
    };
    ledger.apply_to_summary(&mut summary);

    let message = summary.error_message.as_deref().unwrap();
    assert!(
        message.contains("after provider failure: Provider rejected the request"),
        "{message}"
    );
    // A blank kind must not leave a dangling gap before the colon.
    assert!(!message.contains("failure :"), "{message}");
    assert!(!message.contains("failure  "), "{message}");
}

#[test]
fn a_long_message_only_failure_keeps_its_reservation_and_the_task_list_gives_way() {
    let mut ledger = TaskLedger::new();
    for index in 0..30 {
        let id = format!("task-{index}");
        let name = format!("a-very-long-sub-agent-task-name-number-{index}");
        ledger.record_start(Some(&id), Some(&name));
        ledger.record_terminal(Some(&id), Some(&name), Some("stopped"));
    }

    let mut summary = StreamExecutionSummary {
        is_error: true,
        error_message: Some(format!(
            "\u{1b}[31mProvider rejected the request\u{1b}[0m\n{}",
            "x".repeat(500)
        )),
        ..Default::default()
    };
    ledger.apply_to_summary(&mut summary);

    let message = summary.error_message.as_deref().unwrap();
    assert!(!message.contains('\u{1b}'), "escapes leaked: {message:?}");
    assert!(
        !message.contains('\n') && !message.contains('\r'),
        "multi-line: {message:?}"
    );
    assert!(
        message.chars().count() <= 240,
        "{} chars: {message}",
        message.chars().count()
    );
    assert!(
        message.starts_with("30 sub-agent tasks did not complete; after provider failure: Provider rejected the request"),
        "{message}"
    );
    let clause_start = message.find("after provider failure").unwrap();
    let clause_end = message.find("; incomplete:").expect("task list follows the clause");
    let clause_chars = message[clause_start..clause_end].chars().count();
    assert!(
        clause_chars <= PRIOR_FAILURE_CLAUSE_MAX_CHARS,
        "clause of {clause_chars} chars exceeds its reservation: {message}"
    );
    // The truncation landed on the tail (task list), not on the clause.
    assert!(
        !message.contains("a-very-long-sub-agent-task-name-number-29"),
        "{message}"
    );
    assert_eq!(summary.subagent_outcomes.len(), 30);
}
