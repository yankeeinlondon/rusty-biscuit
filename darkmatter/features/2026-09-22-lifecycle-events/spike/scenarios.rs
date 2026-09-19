use super::model::*;

fn claudine() -> Profile {
    Profile { success: "success", failure: "failure", finalize: "finalize",
        before: None, after: None, looping: false, resume: true }
}
fn publisher() -> Profile {
    Profile { success: "publisher.published", failure: "publisher.rejected", finalize: "publisher.closed",
        before: Some("publisher.configure"), after: Some("publisher.ready"), looping: false, resume: false }
}
fn answer(engine: &mut Engine, reply: Reply) { engine.reply(engine.pending(), reply).unwrap(); }
fn event(engine: &mut Engine, role: Role, outcome: Outcome) {
    assert_eq!(engine.pending().request, Request::Event(role));
    answer(engine, Reply::Event(outcome));
}
fn prepare(engine: &mut Engine) {
    event(engine, Role::Initialize, Outcome::default());
    assert_eq!(engine.pending().request, Request::Prepare(Entry::Initial));
    answer(engine, Reply::Prepared(Ok(())));
    event(engine, Role::Composed, Outcome::default());
}
fn work(engine: &mut Engine, result: WorkResult, session: bool) {
    assert_eq!(engine.pending().request, Request::Work);
    answer(engine, Reply::Worked { result, session });
}
fn fail() -> Engine {
    let mut engine = Engine::new(claudine()).unwrap();
    prepare(&mut engine);
    work(&mut engine, WorkResult::Failed(10), true);
    engine
}

#[test]
fn publisher_runs_same_engine_with_custom_events_and_live_fake_state() {
    let mut engine = Engine::new(publisher()).unwrap();
    let mut value = 0;
    let mut trace = Vec::new();
    for _ in 0..20 {
        let request = engine.pending().request;
        match request {
            Request::Event(role) => {
                trace.push(engine.profile().name(role));
                if role == Role::Before { assert!(!engine.shell_allowed()); value = 3; }
                if role == Role::Initialize { assert_eq!(value, 3); value += 1; }
                if role == Role::After { assert!(engine.shell_allowed()); assert_eq!(value, 4); }
                if role == Role::Success { assert_eq!(value, 8); }
                answer(&mut engine, Reply::Event(Outcome::default()));
            }
            Request::Prepare(_) => answer(&mut engine, Reply::Prepared(Ok(()))),
            Request::Work => { value *= 2; answer(&mut engine, Reply::Worked { result: WorkResult::Success, session: false }); }
            Request::Done => break,
            other => panic!("unexpected request {other:?}"),
        }
    }
    assert_eq!(trace, ["publisher.configure", "initialize", "composed", "publisher.ready", "publisher.published", "publisher.closed"]);
    assert_eq!(engine.pending().request, Request::Done);
}

#[test]
fn early_blocked_catch_preserves_shell_denial_and_later_evaluation_wins() {
    let mut engine = Engine::new(claudine()).unwrap();
    event(&mut engine, Role::Initialize, Outcome::default());
    answer(&mut engine, Reply::Prepared(Err(10)));
    assert!(!engine.shell_allowed());
    event(&mut engine, Role::Blocked, Outcome::raised(11));
    assert_eq!(engine.error(), Some(11));
    assert!(!engine.shell_allowed());
    event(&mut engine, Role::Failure, Outcome::raised(12));
    assert_eq!(engine.error(), Some(12));
    assert!(!engine.shell_allowed());
    event(&mut engine, Role::Finalize, Outcome::raised(13));
    assert_eq!(engine.error(), Some(13));
    assert_eq!(engine.terminal(), Some(Role::Failure));
    assert_eq!(engine.pending().request, Request::Done);
}

#[test]
fn success_downgrade_keeps_success_communication_and_runs_failure_once() {
    let mut engine = Engine::new(claudine()).unwrap();
    prepare(&mut engine);
    work(&mut engine, WorkResult::Success, true);
    let mut trace = vec![engine.profile().name(Role::Success)];
    event(&mut engine, Role::Success, Outcome::control(Control::Error));
    assert_eq!(engine.error(), Some(99));
    assert_eq!(engine.terminal(), Some(Role::Failure));
    trace.push(engine.profile().name(Role::Failure));
    event(&mut engine, Role::Failure, Outcome::default());
    trace.push(engine.profile().name(Role::Finalize));
    event(&mut engine, Role::Finalize, Outcome::default());
    assert_eq!(trace, ["success", "failure", "finalize"]);
    assert_eq!(engine.pending().request, Request::Done);
}

#[test]
fn terminal_evaluation_does_not_replay_failure_or_allow_cleanup_recovery() {
    for role in [Role::Success, Role::Failure] {
        let mut engine = Engine::new(claudine()).unwrap();
        prepare(&mut engine);
        work(&mut engine, if role == Role::Success { WorkResult::Success } else { WorkResult::Failed(10) }, true);
        event(&mut engine, role, Outcome::raised(11));
        event(&mut engine, Role::Finalize, Outcome::control(Control::Retry));
        assert_eq!(engine.pending().request, Request::Done);
        assert_eq!(engine.error(), Some(11));
    }
}

#[test]
fn terminal_retry_skips_finalize_but_finalize_retry_does_not_repeat_it() {
    for from_finalize in [false, true] {
        let mut engine = fail();
        if from_finalize { event(&mut engine, Role::Failure, Outcome::default()); }
        let role = if from_finalize { Role::Finalize } else { Role::Failure };
        event(&mut engine, role, Outcome::control(Control::Retry));
        assert_eq!(engine.finalized(), from_finalize);
        assert_eq!(engine.pending().request, Request::Recover(Control::Retry));
        answer(&mut engine, Reply::Recovered(Ok(())));
        assert_eq!(engine.pending().request, Request::Prepare(Entry::Retry));
        assert!(!engine.finalized());
        assert!(!engine.shell_allowed());
        answer(&mut engine, Reply::Prepared(Ok(())));
        event(&mut engine, Role::Composed, Outcome::default());
        work(&mut engine, WorkResult::Failed(12), false);
        event(&mut engine, Role::Failure, Outcome::control(Control::Retry));
        // Repeated requests cannot refresh the one-additional-attempt budget.
        assert_eq!(engine.pending().request, Request::Event(Role::Finalize));
    }
}

#[test]
fn resume_requires_session_and_host_acceptance() {
    let mut accepted = fail();
    event(&mut accepted, Role::Failure, Outcome::control(Control::Resume));
    answer(&mut accepted, Reply::Recovered(Ok(())));
    assert_eq!(accepted.pending().request, Request::Prepare(Entry::Resume));

    let mut refused = fail();
    event(&mut refused, Role::Failure, Outcome::control(Control::Resume));
    answer(&mut refused, Reply::Recovered(Err(42)));
    event(&mut refused, Role::Finalize, Outcome::control(Control::Retry));
    assert_eq!(refused.pending().request, Request::Done);
    assert_eq!(refused.error(), Some(42));

    let mut missing = Engine::new(claudine()).unwrap();
    prepare(&mut missing);
    work(&mut missing, WorkResult::Failed(10), false);
    event(&mut missing, Role::Failure, Outcome::control(Control::Resume));
    assert_eq!(missing.pending().request, Request::Event(Role::Finalize));
    assert_eq!(missing.error(), Some(90));
}

#[test]
fn proxy_changes_identity_only_on_acceptance_and_revokes_shells() {
    let mut engine = fail();
    event(&mut engine, Role::Failure, Outcome::control(Control::Proxy));
    assert_eq!(engine.document(), 1);
    assert!(!engine.finalized());
    answer(&mut engine, Reply::Recovered(Ok(())));
    assert_eq!(engine.document(), 2);
    assert_eq!(engine.pending().request, Request::Event(Role::Initialize));
    assert!(!engine.shell_allowed());
    assert_eq!(engine.terminal(), None);
}

#[test]
fn loop_reentry_keeps_authorization_but_does_not_reinitialize_or_prepare() {
    let mut profile = claudine();
    profile.looping = true;
    let mut engine = Engine::new(profile).unwrap();
    prepare(&mut engine);
    work(&mut engine, WorkResult::Success, true);
    event(&mut engine, Role::Success, Outcome::default());
    event(&mut engine, Role::Finalize, Outcome::default());
    event(&mut engine, Role::Loop, Outcome::control(Control::Again));
    assert_eq!(engine.pending().request, Request::Event(Role::Composed));
    assert!(engine.shell_allowed());
    assert_eq!((engine.document(), engine.iteration()), (1, 2));
    event(&mut engine, Role::Composed, Outcome::default());
    work(&mut engine, WorkResult::Success, true);
    event(&mut engine, Role::Success, Outcome::default());
    event(&mut engine, Role::Finalize, Outcome::default());
    event(&mut engine, Role::Loop, Outcome::raised(44));
    assert_eq!(engine.pending().request, Request::Done);
    assert_eq!(engine.error(), Some(44));
}

#[test]
fn request_protocol_rejects_stale_duplicate_wrong_phase_and_recursive_replies() {
    let mut engine = Engine::new(claudine()).unwrap();
    let initial = engine.pending();
    assert!(engine.reply(initial, Reply::Prepared(Ok(()))).is_err());
    assert_eq!(engine.pending(), initial);
    assert!(engine.reply(initial, Reply::Event(Outcome::control(Control::Again))).is_err());
    assert_eq!(engine.pending(), initial);
    event(&mut engine, Role::Initialize, Outcome::default());
    assert!(engine.reply(initial, Reply::Event(Outcome::default())).is_err());
    answer(&mut engine, Reply::Prepared(Ok(())));
    event(&mut engine, Role::Composed, Outcome::default());
    let work_ticket = engine.pending();
    assert!(engine.reply(work_ticket, Reply::Event(Outcome::default())).is_err());
    work(&mut engine, WorkResult::Success, false);
    assert!(engine.reply(work_ticket, Reply::Worked { result: WorkResult::Success, session: false }).is_err());
}

#[test]
fn registration_cannot_shadow_core_aliases() {
    for name in ["initialize", "blocked", "composed", "start", "failure"] {
        let mut profile = claudine();
        profile.after = Some(name);
        assert!(Engine::new(profile).is_err(), "accepted {name}");
    }
}

#[test]
fn host_driven_api_detects_missing_cleanup_only_when_host_finishes() {
    let mut host = HostDriven::default();
    host.emit(Role::Success).unwrap();
    assert_eq!(host.finish(), Err("host omitted cleanup"));
    let mut correct = HostDriven::default();
    correct.emit(Role::Success).unwrap();
    correct.emit(Role::Finalize).unwrap();
    assert!(correct.finish().is_ok());
    let mut engine = Engine::new(claudine()).unwrap();
    prepare(&mut engine);
    work(&mut engine, WorkResult::Success, false);
    event(&mut engine, Role::Success, Outcome::default());
    assert_eq!(engine.pending().request, Request::Event(Role::Finalize));
    assert!(engine.reply(engine.pending(), Reply::Worked { result: WorkResult::Success, session: false }).is_err());
}

#[test]
fn cross_run_tickets_cannot_advance_an_identical_phase() {
    let first = Engine::new(claudine()).unwrap();
    let mut second = Engine::new(claudine()).unwrap();
    let pending = second.pending();
    assert!(second.reply(first.pending(), Reply::Event(Outcome::default())).is_err());
    assert_eq!(second.pending(), pending);
}

#[test]
fn composed_and_start_resolve_to_one_identity() {
    let profile = claudine();
    assert_eq!(profile.resolve("start"), Some(Role::Composed));
    assert_eq!(profile.resolve("composed"), Some(Role::Composed));
    assert_eq!(profile.resolve("strat"), None);
}

#[test]
fn blocked_explicit_downgrade_in_normal_recovery_allows_retry() {
    let mut engine = Engine::new(claudine()).unwrap();
    event(&mut engine, Role::Initialize, Outcome::default());
    answer(&mut engine, Reply::Prepared(Err(10)));
    event(&mut engine, Role::Blocked, Outcome::control(Control::Error));
    assert!(!engine.shell_allowed());
    event(&mut engine, Role::Failure, Outcome::control(Control::Retry));
    assert_eq!(engine.pending().request, Request::Recover(Control::Retry));
    assert!(!engine.finalized());
}
