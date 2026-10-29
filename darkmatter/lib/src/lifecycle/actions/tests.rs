use super::*;

#[test]
fn control_action_validity_matrix() {
    use LifecycleControlAction as A;
    use LifecycleSignal as S;

    // Stop and Error are valid in every event.
    for event in [
        S::Initialize,
        S::Start,
        S::Success,
        S::Blocked,
        S::Failure,
        S::Finalize,
        S::Loop,
    ] {
        assert!(A::Stop.is_valid_for(event), "Stop in {event:?}");
        assert!(
            A::Error { reason: None }.is_valid_for(event),
            "Error in {event:?}"
        );
    }

    // Skip is valid only in Initialize.
    assert!(A::Skip.is_valid_for(S::Initialize));
    for event in [S::Start, S::Success, S::Blocked, S::Failure, S::Finalize, S::Loop] {
        assert!(!A::Skip.is_valid_for(event), "Skip in {event:?}");
    }

    // Flow control is universal: Proxy/Retry/Resume/Defer are valid in
    // every event (placement is not error-gated).
    let every = [
        S::Initialize,
        S::Start,
        S::Success,
        S::Blocked,
        S::Failure,
        S::Finalize,
        S::Loop,
    ];
    let proxy = A::Proxy {
        target: Expr::StringLiteral("@other.md".into()),
        with: WithOverlay::default(),
    };
    let retry = A::Retry {
        max_attempts: None,
        backoff: None,
        delay: None,
        with: WithOverlay::default(),
    };
    let resume = A::Resume {
        message: Expr::StringLiteral("please retry".into()),
        max_attempts: None,
    };
    let requeue = A::Defer {
        delay: Expr::StringLiteral("5m".into()),
        reason: None,
    };
    for event in every {
        assert!(proxy.is_valid_for(event), "Proxy in {event:?}");
        assert!(retry.is_valid_for(event), "Retry in {event:?}");
        assert!(resume.is_valid_for(event), "Resume in {event:?}");
        assert!(requeue.is_valid_for(event), "Defer in {event:?}");
    }
}

#[test]
fn control_action_verb_round_trip() {
    use LifecycleControlAction as A;
    assert_eq!(A::Stop.verb(), "stop");
    assert_eq!(A::Skip.verb(), "skip");
    assert_eq!(A::Error { reason: None }.verb(), "error");
}

#[test]
fn retry_backoff_round_trip() {
    assert_eq!(RetryBackoff::parse("fixed"), Some(RetryBackoff::Fixed));
    assert_eq!(
        RetryBackoff::parse("exponential"),
        Some(RetryBackoff::Exponential)
    );
    assert_eq!(RetryBackoff::parse("bogus"), None);
    assert_eq!(RetryBackoff::Fixed.as_str(), "fixed");
    assert_eq!(RetryBackoff::Exponential.as_str(), "exponential");
}

#[test]
fn communication_channel_verb_round_trip() {
    for channel in [
        CommunicationChannel::Say,
        CommunicationChannel::Speak,
        CommunicationChannel::Effect,
        CommunicationChannel::Message,
        CommunicationChannel::Notify,
        CommunicationChannel::Stderr,
        CommunicationChannel::Info,
        CommunicationChannel::Warn,
        CommunicationChannel::Success,
        CommunicationChannel::Stdout,
    ] {
        let verb = channel.verb();
        assert_eq!(
            CommunicationChannel::from_verb(verb),
            Some(channel),
            "round-trip for {verb}"
        );
    }
    assert_eq!(CommunicationChannel::from_verb("bogus"), None);
}

#[test]
fn is_lifecycle_control_flag() {
    let lc_stop = LifecycleAction {
        kind: LifecycleActionKind::LifecycleControl(LifecycleControlAction::Stop),
        no_error: false,
        path: LifecyclePath::root("start").index(0).field("stop"),
    };
    let comm = LifecycleAction {
        kind: LifecycleActionKind::Communication(CommunicationAction {
            channel: CommunicationChannel::Say,
            text: Expr::StringLiteral("hi".into()),
        }),
        no_error: false,
        path: LifecyclePath::root("start").index(1).field("say"),
    };
    assert!(lc_stop.is_lifecycle_control());
    assert!(!comm.is_lifecycle_control());
}

// -------------------------------------------------------------------------
// Generated verb signatures (per overload, from the typed catalogs)
// -------------------------------------------------------------------------

fn overload_names(signature: &VerbSignature) -> Vec<Vec<&'static str>> {
    signature.overloads().iter().map(|overload| overload.names().collect()).collect()
}

#[test]
fn side_effect_signature_reads_typed_overloads() {
    let set = side_effect_signature("set_frontmatter").unwrap();
    assert_eq!(overload_names(set), [vec!["file", "prop", "value"]]);

    let ensure = side_effect_signature("ensure_file").unwrap();
    assert_eq!(overload_names(ensure), [vec!["file"], vec!["file", "content"]]);
    assert!(ensure.accepts_count(1) && ensure.accepts_count(2) && !ensure.accepts_count(3));
    assert_eq!(ensure.min_count(), 1);

    assert!(side_effect_signature("not_a_verb").is_none());
}

#[test]
fn expression_function_signature_keeps_optional_parameters_per_overload() {
    // `number(x, [fallback])`: one overload with an optional tail.
    let number = expression_function_signature("number").unwrap();
    assert_eq!(overload_names(number), [vec!["x", "fallback"]]);
    assert_eq!((number.overloads()[0].min_count(), number.overloads()[0].max_count()), (1, Some(2)));

    // Overloads are never merged: `link(file)` | `link(target, desc)`.
    let link = expression_function_signature("link").unwrap();
    assert_eq!(overload_names(link), [vec!["file"], vec!["target", "desc"]]);
    assert!(link.accepts_count(1) && link.accepts_count(2) && !link.accepts_count(3));
}

#[test]
fn expression_function_signature_variadic_preserved() {
    let and = expression_function_signature("and").unwrap();
    assert!(and.is_variadic());
    assert_eq!(and.min_count(), 0);
    assert!(and.accepts_count(0) && and.accepts_count(7));

    assert!(expression_function_signature("not_a_function").is_none());
}

#[test]
fn select_named_picks_the_overload_whose_parameters_cover_the_keys() {
    let link = expression_function_signature("link").unwrap();
    assert_eq!(link.select_named(&["file"]).unwrap().display(), "link(file)");
    assert_eq!(link.select_named(&["desc", "target"]).unwrap().display(), "link(target, desc)");
    assert!(matches!(
        link.select_named(&["target"]),
        Err(LongFormMismatch::Missing { missing, .. }) if missing == ["desc"]
    ));
    assert_eq!(link.select_named(&["bogus", "file"]), Err(LongFormMismatch::Unknown(vec!["bogus".to_string()])));
    assert_eq!(link.select_named(&["file", "desc"]), Err(LongFormMismatch::Unrelated));
}

// -------------------------------------------------------------------------
// Known-verb predicate
// -------------------------------------------------------------------------

#[test]
fn is_known_lifecycle_verb_unions_families() {
    // Communication
    assert!(is_known_lifecycle_verb("success"));
    assert!(is_known_lifecycle_verb("message"));
    assert!(is_known_lifecycle_verb("stderr"));

    // Shell
    assert!(is_known_lifecycle_verb("shell"));

    // Control
    assert!(is_known_lifecycle_verb("stop"));
    assert!(is_known_lifecycle_verb("retry"));
    assert!(is_known_lifecycle_verb("defer"));

    // Side-effect
    assert!(is_known_lifecycle_verb("set_frontmatter"));
    assert!(is_known_lifecycle_verb("ensure_file"));

    // Expression function
    assert!(is_known_lifecycle_verb("length"));
    assert!(is_known_lifecycle_verb("and"));
    assert!(is_known_lifecycle_verb("or"));
}

#[test]
fn is_known_lifecycle_verb_rejects_unknown() {
    assert!(!is_known_lifecycle_verb("sucess"));
    assert!(!is_known_lifecycle_verb("nope"));
    assert!(!is_known_lifecycle_verb(""));
}

// -------------------------------------------------------------------------
// Short-form rewrite helper
// -------------------------------------------------------------------------

#[test]
fn rewrite_to_positional_communication() {
    assert_eq!(rewrite_to_positional("success(\"x\")"), "success: \"x\"");
}

#[test]
fn rewrite_to_positional_multi_arg() {
    assert_eq!(
        rewrite_to_positional("set_frontmatter('a','b','c')"),
        "set_frontmatter: [\"a\", \"b\", \"c\"]"
    );
}

#[test]
fn rewrite_to_positional_zero_arg() {
    assert_eq!(rewrite_to_positional("stop()"), "stop: []");
}

#[test]
fn rewrite_to_positional_bare_verb() {
    assert_eq!(rewrite_to_positional("stop"), "stop: []");
}
