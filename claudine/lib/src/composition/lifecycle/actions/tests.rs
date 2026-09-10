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
        with: ProxyWith::default(),
    };
    let retry = A::Retry {
        max_attempts: None,
        backoff: None,
        delay: None,
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
    };
    let comm = LifecycleAction {
        kind: LifecycleActionKind::Communication(CommunicationAction {
            channel: CommunicationChannel::Say,
            message: Expr::StringLiteral("hi".into()),
            route: None,
        }),
        no_error: false,
    };
    assert!(lc_stop.is_lifecycle_control());
    assert!(!comm.is_lifecycle_control());
}

// -------------------------------------------------------------------------
// Signature parser
// -------------------------------------------------------------------------

#[test]
fn parse_signature_fixed_positional() {
    let sig = parse_signature("set_frontmatter(file, prop, value)").unwrap();
    assert_eq!(sig.verb, "set_frontmatter");
    assert_eq!(sig.params, vec!["file", "prop", "value"]);
    assert_eq!(sig.optional_tail, 0);
    assert!(!sig.variadic);
    assert_eq!(sig.required_count(), 3);
    assert_eq!(sig.max_count(), Some(3));
}

#[test]
fn parse_signature_optional_tail() {
    let sig = parse_signature("ensure_file(file, content?)").unwrap();
    assert_eq!(sig.verb, "ensure_file");
    assert_eq!(sig.params, vec!["file", "content"]);
    assert_eq!(sig.optional_tail, 1);
    assert!(!sig.variadic);
    assert_eq!(sig.required_count(), 1);
    assert_eq!(sig.max_count(), Some(2));
}

#[test]
fn parse_signature_bracket_optional_tail() {
    // Darkmatter's catalog form for an optional trailing parameter.
    let sig = parse_signature("number(x, [default])").unwrap();
    assert_eq!(sig.verb, "number");
    assert_eq!(sig.params, vec!["x", "default"]);
    assert_eq!(sig.optional_tail, 1);
    assert!(!sig.variadic);
    assert_eq!(sig.required_count(), 1);
    assert_eq!(sig.max_count(), Some(2));
}

#[test]
fn parse_signature_variadic() {
    let sig = parse_signature("and(...)").unwrap();
    assert_eq!(sig.verb, "and");
    assert!(sig.params.is_empty());
    assert_eq!(sig.optional_tail, 0);
    assert!(sig.variadic);
    assert_eq!(sig.required_count(), 0);
    assert_eq!(sig.max_count(), None);
}

#[test]
fn parse_signature_zero_arg() {
    let sig = parse_signature("stop()").unwrap();
    assert_eq!(sig.verb, "stop");
    assert!(sig.params.is_empty());
    assert!(!sig.variadic);
    assert_eq!(sig.required_count(), 0);
    assert_eq!(sig.max_count(), Some(0));
}

#[test]
fn parse_signature_malformed() {
    assert!(parse_signature("no_parens").is_none());
    assert!(parse_signature("(file)").is_none());
    assert!(parse_signature("verb(").is_none());
    assert!(parse_signature("verb(file").is_none());
    assert!(parse_signature("verb(a, ?)").is_none());
    assert!(parse_signature("verb(a?, b)").is_none());
}

// -------------------------------------------------------------------------
// Side-effect signature derivation
// -------------------------------------------------------------------------

#[test]
fn side_effect_signature_derives_from_descriptors() {
    let set = side_effect_signature("set_frontmatter").unwrap();
    assert_eq!(set.params, vec!["file", "prop", "value"]);
    assert_eq!(set.optional_tail, 0);

    let ensure = side_effect_signature("ensure_file").unwrap();
    assert_eq!(ensure.params, vec!["file", "content"]);
    assert_eq!(ensure.optional_tail, 1);

    assert!(side_effect_signature("not_a_verb").is_none());
}

// -------------------------------------------------------------------------
// Expression-function signature derivation
// -------------------------------------------------------------------------

#[test]
fn expression_function_signature_bracket_optional() {
    // `number(x, [default])` — the bracketed param is optional, so the
    // one-argument form `number("{{ value }}")` is valid arity.
    let sig = expression_function_signature("number").unwrap();
    assert_eq!(sig.params, vec!["x", "default"]);
    assert_eq!(sig.optional_tail, 1);
    assert_eq!(sig.required_count(), 1);
    assert_eq!(sig.max_count(), Some(2));

    let round = expression_function_signature("round").unwrap();
    assert_eq!(round.required_count(), 1);
    assert_eq!(round.max_count(), Some(2));
}

#[test]
fn expression_function_signature_merges_overloads() {
    // Overloaded functions: the shorter overload's missing parameters are
    // optional, so the one-argument positional form is valid arity.
    let frontmatter = expression_function_signature("frontmatter").unwrap();
    assert_eq!(frontmatter.params, vec!["file", "prop"]);
    assert_eq!(frontmatter.optional_tail, 1);
    assert_eq!(frontmatter.required_count(), 1);
    assert_eq!(frontmatter.max_count(), Some(2));

    let link = expression_function_signature("link").unwrap();
    assert_eq!(link.required_count(), 1);
    assert_eq!(link.max_count(), Some(2));

    let validate = expression_function_signature("validate_schema").unwrap();
    assert_eq!(validate.required_count(), 1);
    assert_eq!(validate.max_count(), Some(2));
}

#[test]
fn expression_function_signature_variadic_preserved() {
    let and = expression_function_signature("and").unwrap();
    assert!(and.variadic);
    assert_eq!(and.required_count(), 0);
    assert_eq!(and.max_count(), None);

    assert!(expression_function_signature("not_a_function").is_none());
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
