use super::*;
use crate::markdown::compose::shell_expansion::tokenize::{parse_pipeline, tokenize};
use biscuit_terminal::errors::SourceContext;

fn shape(text: &str) -> TemplateShape {
    TemplateShape::from_template(&parse_authored(text).unwrap_or_else(|error| panic!("{text}: {error}")))
}

fn literal_args(segment: &TemplateSegment) -> Vec<Option<String>> {
    segment
        .args
        .iter()
        .map(|arg| match arg.parts.as_slice() {
            [ArgPart::Literal(text)] => Some(text.clone()),
            _ => None,
        })
        .collect()
}

// ── authored grammar ─────────────────────────────────────────────────────

#[test]
fn a_placeholder_is_recognized_before_its_spaces_quotes_and_operators() {
    let template = parse_authored("tool {{ a || 'x y' }} \"{{ b }}\"").unwrap();
    let [segment] = template.segments.as_slice() else { panic!("one segment") };
    assert_eq!(segment.args.len(), 2);
    let expressions: Vec<&str> = template.placeholders().map(|hole| hole.expression.as_str()).collect();
    assert_eq!(expressions, ["a || 'x y'", "b"]);
    let first = template.placeholders().next().unwrap();
    assert_eq!(&template.authored[first.span.clone()], "{{ a || 'x y' }}");
}

#[test]
fn literals_and_holes_stay_in_their_containing_argument() {
    let template = parse_authored(r#"git commit -m "repairs from {{ name }}" {{ a }}{{ b }} pre-{{ c }}/post"#).unwrap();
    let args = &template.segments[0].args;
    assert_eq!(args.len(), 5);
    assert!(matches!(&args[2].parts[..], [ArgPart::Literal(text), ArgPart::Hole(_)] if text == "repairs from "));
    assert!(matches!(&args[3].parts[..], [ArgPart::Hole(_), ArgPart::Hole(_)]));
    assert!(
        matches!(&args[4].parts[..], [ArgPart::Literal(a), ArgPart::Hole(_), ArgPart::Literal(b)] if a == "pre-" && b == "/post")
    );
}

#[test]
fn an_empty_quoted_argument_is_one_present_empty_literal() {
    let template = parse_authored(r#"tool "" '' x"#).unwrap();
    assert_eq!(literal_args(&template.segments[0]), [Some(String::new()), Some(String::new()), Some("x".into())]);
    assert_eq!(shape(r#"tool "" x"#).canonical_text(), r#"tool "" x"#);
}

#[test]
fn quoting_and_whitespace_spellings_have_one_shape() {
    let expected = shape("ls a {{ dir }}/build");
    for variant in ["ls   'a'   {{ dir }}/build", "ls \"a\" \"{{ dir }}/build\"", "ls a '{{ dir }}'/build", "ls\ta {{ dir }}\"/build\""] {
        assert_eq!(shape(variant), expected, "{variant}");
    }
}

#[test]
fn chains_keep_their_operators_and_redirections_their_order() {
    let template = parse_authored("a {{ x }} > /dev/null 2>&1 || b 2>&1 > /dev/null && c >&2").unwrap();
    let operators: Vec<ChainOperator> = template.segments.iter().map(|segment| segment.operator).collect();
    assert_eq!(operators, [ChainOperator::None, ChainOperator::Or, ChainOperator::And]);
    assert_eq!(template.segments[0].redirects, [RedirectOp::StdoutNull, RedirectOp::StderrToStdout]);
    assert_eq!(template.segments[1].redirects, [RedirectOp::StderrToStdout, RedirectOp::StdoutNull]);
    assert_eq!(template.segments[2].redirects, [RedirectOp::StdoutToStderr]);
    assert_ne!(shape("a > /dev/null 2>&1"), shape("a 2>&1 > /dev/null"));
}

#[test]
fn redirections_lower_exactly_as_the_resolved_byte_tokenizer_does() {
    let ctx = SourceContext::new("/t".into(), "t".into(), "");
    for command in [
        "a > /dev/null 2>&1",
        "a 2>&1 > /dev/null",
        "a 2> /dev/null >&2",
        "a >&2 2> /dev/null",
        "a 2>/dev/null",
    ] {
        let pipeline = parse_pipeline(&tokenize(command, &ctx).unwrap(), &ctx).unwrap();
        let template = parse_authored(command).unwrap();
        assert_eq!(
            redirection_config(&template.segments[0].redirects),
            pipeline.actions[0].command.redirection,
            "{command}"
        );
    }
}

#[test]
fn unsupported_template_grammar_is_a_typed_error() {
    use TemplateGrammarError::*;
    for (command, expected) in [
        ("{{ tool }} run", ProgramWordHole),
        ("ok && {{ tool }} run", ProgramWordHole),
        ("pre{{ tool }} run", ProgramWordHole),
        ("a > {{ out }}", RedirectHole),
        ("a > /dev/{{ out }}", RedirectHole),
        ("a 2> {{ out }}", RedirectHole),
        ("a {{ x }} | b", Pipe),
        ("a {{ x }}; b", Semicolon),
        ("a {{ x }} &", Background),
        ("a < {{ x }}", InputRedirect),
        ("a >> /dev/null", ArbitraryRedirect),
        ("a > out.txt", ArbitraryRedirect),
        ("a 2>&3", ArbitraryRedirect),
        ("a $(id) {{ x }}", CommandSubstitution),
        ("a '{{ x }}", Unterminated),
        ("a \"{{ x }}", Unterminated),
        ("a {{{ raw }}}", InterpolationLiteral),
        ("a \u{7}{{ x }}", ControlCharacter),
        ("", Empty),
        ("a {{ x }} &&", Empty),
        ("&& a", Empty),
    ] {
        assert_eq!(parse_authored(command), Err(expected), "{command:?}");
    }
}

#[test]
fn values_are_never_parsed_only_authored_text_is() {
    // An authored literal that looks like shell syntax is literal text inside
    // a quoted argument; the hole's own text never reaches the tokenizer.
    let template = parse_authored(r#"echo "x; rm -rf ~ | $HOME" {{ a; b }}"#).unwrap();
    assert_eq!(literal_args(&template.segments[0])[0].as_deref(), Some("x; rm -rf ~ | $HOME"));
    assert_eq!(template.placeholders().next().unwrap().expression, "a; b");
}

// ── canonical rule text ──────────────────────────────────────────────────

#[test]
fn the_canonical_text_of_the_build_clean_example_is_bare() {
    assert_eq!(shape("just clean --out {{ out }}/build").canonical_text(), "just clean --out {{ string }}/build");
}

#[test]
fn canonical_text_round_trips_every_literal_and_boundary() {
    for command in [
        "just clean --out {{ out }}/build",
        r#"tool "" {{ a }}{{ b }} "a b" 'q"uote' back\\slash"#,
        "tool \"line\nbreak\" \"tab\there\" \"cr\rhere\" ünïcødé 100% wow!",
        r#"tool "literal \{{ not a hole }}" "{x}" "{{ y }}""#,
        "a {{ x }} > /dev/null 2>&1 || b 2>&1 && c >&2 2> /dev/null",
        r##"tool "&&" "||" "2>&1" ">" "#""##,
    ] {
        let original = shape(command);
        let text = original.canonical_text();
        let reread = TemplateShape::parse_rule(&text).unwrap_or_else(|error| panic!("{text}: {error}"));
        assert_eq!(reread, original, "{command:?} → {text:?}");
        assert_eq!(reread.canonical_text(), text);
    }
}

#[test]
fn the_reader_accepts_bare_and_quoted_literal_runs_alike() {
    let canonical = TemplateShape::parse_rule("just clean --out {{ string }}/build").unwrap();
    for variant in [
        r#"just "clean" --out {{ string }}"/build""#,
        r#""just"   clean  "--out" {{string}}"/build""#,
        "just clean --out {{ string }}\"/build\"",
    ] {
        assert_eq!(TemplateShape::parse_rule(variant).unwrap(), canonical, "{variant}");
    }
    // An escaped brace inside quotes is literal, never a hole.
    let literal = TemplateShape::parse_rule(r#"echo "\{\{ string \}\}""#).unwrap();
    assert!(!literal.has_holes());
}

#[test]
fn malformed_rule_text_is_rejected() {
    use TemplateRuleError as E;
    let reject = |text: &str| TemplateShape::parse_rule(text).expect_err(text);
    assert!(matches!(reject("{{ string }} run"), E::Grammar(TemplateGrammarError::ProgramWordHole)));
    assert!(matches!(reject("ok && {{ string }}"), E::Grammar(TemplateGrammarError::ProgramWordHole)));
    assert!(matches!(reject("build {{ alphanumeric(allow-slashes; min(3)) }}"), E::Constraint(_)));
    assert!(matches!(reject("build {{ alphanumeric(min(3)) }}"), E::Constraint(_)));
    assert!(matches!(reject("build {{ string(min(3)) }}"), E::Constraint(_)));
    assert!(matches!(reject("build {{ string -> a description }}"), E::Annotation(_)));
    assert!(matches!(reject("build {{ slug }}"), E::NamedType(_)));
    assert!(matches!(reject("build {{ slug@this }}"), E::NamedType(_)));
    assert!(matches!(reject("build {{ number }}"), E::UnsupportedType(_)));
    assert!(matches!(reject("build {{ boolean }}"), E::UnsupportedType(_)));
    assert!(matches!(reject("build {{ unknown }}"), E::UnsupportedType(_)));
    assert!(matches!(reject("build {{ string[] }}"), E::UnsupportedType(_)));
    assert!(matches!(reject("build {{ 123 }}"), E::NamedType(_)));
    assert!(matches!(reject("build {{ alphanumeric(allow-everything) }}"), E::InvalidType { .. }));
    assert!(matches!(reject("build {{ [string, number] }}"), E::InvalidType { .. } | E::UnsupportedType(_)));
    assert!(matches!(reject("build {{ string"), E::UnterminatedHole));
    assert!(matches!(reject("build \"open"), E::Grammar(TemplateGrammarError::Unterminated)));
    assert!(matches!(reject(r#"build "\q""#), E::InvalidEscape('q')));
    assert!(matches!(reject(r#"build "{{ string }}""#), E::UnescapedBrace('{')));
    assert!(matches!(reject("build {a}"), E::UnquotedCharacter('{')));
    assert!(matches!(reject("build a\u{7}"), E::Grammar(TemplateGrammarError::ControlCharacter)));
    assert!(matches!(reject("build \"a\u{1b}\""), E::Grammar(TemplateGrammarError::ControlCharacter)));
    assert!(matches!(reject(""), E::Grammar(TemplateGrammarError::Empty)));
}

#[test]
fn rule_types_keep_only_their_canonical_identity() {
    let parsed = TemplateShape::parse_rule(
        "build {{ alphanumeric(allow-slashes, block-leading-dash) }} {{ enum(b, a) }} {{ email }} {{ string }}",
    )
    .unwrap();
    // `block-leading-dash` is redundant without `allow-dashes`; members sort.
    assert_eq!(
        parsed.canonical_text(),
        "build {{ alphanumeric(allow-slashes) }} {{ enum(a,b) }} {{ email }} {{ string }}"
    );
}

/// The widened structural relation reaches rule matching only for sound
/// string-like subsets; the saved rule grammar and the broad-string rules are
/// unchanged.
#[test]
fn rule_coverage_follows_the_structural_relation_for_string_like_holes() {
    use crate::markdown::schemas::simplified::grammar::parse_type_expr;
    let approval = |text: &str| {
        StringLikeType::from_atom(&parse_type_expr("p", text).unwrap())
            .map_or(ApprovalType::BroadString, ApprovalType::StringLike)
    };
    assert!(rule_covers(&approval("enum(a, b, c)"), &approval("enum(a, b)")));
    assert!(rule_covers(&approval("enum(a, b, c)"), &approval("literal('a')")));
    assert!(!rule_covers(&approval("enum(a, b)"), &approval("enum(a, b, c)")));
    assert!(!rule_covers(&approval("enum(a, b)"), &approval("literal('z')")));
    // Bounded: a one-member enum is not compared against a literal.
    assert!(!rule_covers(&approval("literal('a')"), &approval("enum(a)")));
    // A non-string declaration keeps the broad approval and needs a broad rule.
    assert_eq!(approval("number"), ApprovalType::BroadString);
    assert_eq!(approval("literal(4)"), ApprovalType::BroadString);
    assert!(!rule_covers(&approval("enum('4')"), &approval("literal(4)")));
    assert!(rule_covers(&ApprovalType::BroadString, &approval("literal(4)")));
}

#[test]
fn session_reservations_are_keyed_by_mode_and_policy_root() {
    use crate::markdown::compose::shell_expansion::types::{ReserveOutcome, ShellExpansionRuntime};
    use std::path::Path;

    let mut runtime = ShellExpansionRuntime::new();
    let root = Path::new("/policy/a");
    let shape = TemplateShape::parse_rule("git --version {{ string }}").unwrap();
    let exact = ApprovalKey::exact(Some(root), "git --version {{ string }}");
    let template = ApprovalKey::template(Some(root), shape.clone());

    assert_eq!(runtime.reserve_allow_once(&exact, false), ReserveOutcome::Reserved);
    runtime.complete_allow_once(&exact, true);
    // Bytes that spell a shape's text never answer for the shape.
    assert_eq!(runtime.reserve_allow_once(&template, false), ReserveOutcome::Reserved);
    // A failed approval releases the reservation and approves nothing.
    runtime.complete_allow_once(&template, false);
    assert!(runtime.pending_allow_once_for_test().is_empty());
    assert_eq!(runtime.reserve_allow_once(&template, false), ReserveOutcome::Reserved);
    runtime.complete_allow_once(&template, true);
    assert_eq!(runtime.reserve_allow_once(&template, false), ReserveOutcome::AlreadyAllowed);
    // Another policy root's approval does not answer.
    let elsewhere = ApprovalKey::template(Some(Path::new("/policy/b")), shape);
    assert_eq!(runtime.reserve_allow_once(&elsewhere, false), ReserveOutcome::Reserved);
}

#[test]
fn shell_template_program_choices_keep_authored_program_evidence() {
    for (authored, prefix) in [
        ("{{ tool }} probe", false),
        ("> /dev/null {{ tool }} probe", false),
        ("2> /dev/null {{ tool }} probe", false),
        (">&2 {{ tool }} probe", false),
        ("2>&1 {{ tool }} probe", false),
        ("> /dev/null tool {{ word }}", true),
        ("2> /dev/null tool {{ word }}", true),
        (">&2 tool {{ word }}", true),
        ("2>&1 tool {{ word }}", true),
        ("git --version\n{{ tool }} probe", false),
        ("git --version\r\n{{ tool }} probe", false),
        ("git --version\n> /dev/null {{ tool }} probe", false),
        ("git --version\r\n2> /dev/null {{ tool }} probe", false),
        ("git --version\n>&2 {{ tool }} probe", false),
        ("git --version\r\n2>&1 {{ tool }} probe", false),
        ("git --version\nother {{ word }}", true),
        ("git --version\r\nother {{ word }}", true),
        ("git --version\n> /dev/null other {{ word }}", true),
        ("git --version\r\n2> /dev/null other {{ word }}", true),
        ("git --version\n>&2 other {{ word }}", true),
        ("git --version\r\n2>&1 other {{ word }}", true),
        ("tool \"quoted\n{{ word }}\"", true),
        ("tool \"quoted\r\n{{ word }}\"", true),
        ("git --version && {{ tool }} probe", false),
        ("git --version || pre{{ tool }} probe", false),
        ("'{{ tool }}' probe", false),
        ("\"pre{{ tool }}\" probe", false),
        ("tool `literal` && {{ tool }} probe", false),
        ("tool {{ word || 'a && b' }}", true),
        ("tool {{ word }} && other {{ word }}", true),
        ("tool `literal` {{ word }}", true),
        ("tool {{ word }} ; other literal", true),
        ("tool {{ word }} ; {{ tool }} probe", false),
    ] {
        let candidate = ShellCandidate {
            authored_command: authored.to_string(),
            site: CommandSite { source_file: "doc.md".into(), origin: crate::markdown::compose::ShellCommandOrigin::Body { line: 1 }, owner: None },
            template: parse_authored(authored),
            resolved: Some(vec!["resolved probe".to_string()]),
            requirement: ModeRequirement::AnyMode,
        };
        let offered = candidate.offered();
        assert!(offered.exact, "{authored}");
        assert_eq!(offered.prefix, prefix, "{authored}");
        assert_eq!(offered.shape, candidate.template.is_ok(), "{authored}");
    }
}

// ── guarded discovery (candidate construction at a type scope) ──────────

mod guarded_discovery {
    use super::super::candidate::CandidateScope;
    use super::*;
    use crate::markdown::compose::ShellCommandOrigin;
    use crate::markdown::compose::expression::{DeclaredTypes, Expr, Polarity, TypeScope, parse};
    use crate::markdown::compose::shell_expansion::types::ShellExpansionError;
    use serde_json::{Value, json};
    use std::collections::HashSet;

    /// The guarded command of the motivating lifecycle action.
    const COMMIT: &str = r#"git commit -m "{{ commit_message }}""#;
    const COMMIT_GUARD: &str = "ctx.dirty_files && commit_message";

    /// Builds `command`'s candidate at a site enclosed by `guard` (true side),
    /// reading approval-time values from `values` by variable path.
    fn candidate(
        command: &str,
        schema: Option<&str>,
        guard: Option<(&str, Polarity)>,
        values: Value,
        deferred: &DeferredShellRoots,
        host_globals: &[&str],
    ) -> Result<ShellCandidate, ShellExpansionError> {
        let schema = schema.map(|yaml| {
            let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(yaml).expect("schema YAML");
            crate::markdown::schemas::parse_yaml_schema(&value).expect("schema parses")
        });
        let declared = DeclaredTypes::new(schema.as_ref()).with_host_globals(host_globals);
        let site = TypeScope::unguarded(&declared);
        let types = match guard {
            Some((guard, polarity)) => site.assume(&parse(guard).unwrap(), polarity),
            None => site,
        };
        let derived = HashSet::new();
        let ctx = SourceContext::new("/doc.md".into(), "doc.md".into(), command);
        let scope = CandidateScope {
            deferred,
            derived: &derived,
            source_file: "doc.md".into(),
            owner: None,
            ctx: &ctx,
        };
        let evaluate = |expr: &Expr| match expr {
            Expr::Variable(path) => Some(values.get(path).cloned().unwrap_or(Value::Null)),
            _ => None,
        };
        scope.build(
            command,
            parse_authored(command),
            ShellCommandOrigin::Body { line: 1 },
            &types,
            &evaluate,
        )
    }

    fn timings(candidate: &ShellCandidate) -> Vec<HoleTiming> {
        candidate.template.as_ref().unwrap().placeholders().map(|hole| hole.timing.clone()).collect()
    }

    fn missing(result: Result<ShellCandidate, ShellExpansionError>) -> String {
        match result {
            Err(ShellExpansionError::MissingValue { expression, .. }) => expression,
            other => panic!("expected a missing value, got {other:?}"),
        }
    }

    /// The motivating case: a guard proving the value present lets a value
    /// missing at discovery wait for its command, as a template only, with
    /// its authored expression kept and no resolved bytes.
    #[test]
    fn a_guard_proven_missing_value_waits_for_its_command_as_a_template() {
        let none = DeferredShellRoots::default();
        let site = candidate(COMMIT, None, Some((COMMIT_GUARD, Polarity::True)), json!({}), &none, &[]).unwrap();
        let holes = timings(&site);
        let [HoleTiming::Guarded { guard, roots }] = holes.as_slice() else {
            panic!("one guarded hole: {holes:?}");
        };
        // `assume` labels a guard with its expression; hosts with an authored
        // location use `assume_at`.
        assert_eq!(guard.label(), parse(COMMIT_GUARD).unwrap().to_string());
        assert!(roots.is_empty());
        assert_eq!(site.requirement, ModeRequirement::TemplateRequired);
        assert_eq!(site.authored_command, COMMIT);
        assert_eq!(site.resolved, None, "no discovery value stands in for the live one");
        assert_eq!(site.template.as_ref().unwrap().placeholders().next().unwrap().expression, "commit_message");

        let offered = site.offered();
        assert!(!offered.exact && offered.shape && offered.prefix, "{offered:?}");
        let shape = site.shape().unwrap();
        assert_eq!(shape.canonical_text(), r#"git commit -m {{ string }}"#);
        let mut exact_only = ShellPreApprovals::new();
        exact_only.insert_exact(r#"git commit -m """#.to_string());
        assert!(!site.authorized_by(&exact_only), "exact bytes never authorize a guarded site");
        let mut by_shape = ShellPreApprovals::new();
        by_shape.insert_template(shape);
        assert!(site.authorized_by(&by_shape));
    }

    /// Without the guard, or on the guard's false side, or under a guard
    /// that does not prove the value, a missing value still fails discovery.
    #[test]
    fn a_missing_value_no_guard_proves_still_fails_discovery() {
        let none = DeferredShellRoots::default();
        assert_eq!(missing(candidate(COMMIT, None, None, json!({}), &none, &[])), "commit_message");
        assert_eq!(
            missing(candidate(COMMIT, None, Some((COMMIT_GUARD, Polarity::False)), json!({}), &none, &[])),
            "commit_message"
        );
        for guard in ["!commit_message", "ctx.dirty_files || commit_message", "is_null(commit_message)", "other"] {
            assert_eq!(
                missing(candidate(COMMIT, None, Some((guard, Polarity::True)), json!({}), &none, &[])),
                "commit_message",
                "{guard}"
            );
        }
    }

    /// A guard on one placeholder never defers another in the same command.
    #[test]
    fn a_guard_on_one_value_does_not_defer_another() {
        let none = DeferredShellRoots::default();
        let guard = Some(("x", Polarity::True));
        assert_eq!(missing(candidate("tool {{ x }} {{ y }}", None, guard, json!({}), &none, &[])), "y");
        let site = candidate("tool {{ x }} {{ y }}", None, guard, json!({ "y": "present" }), &none, &[]).unwrap();
        assert!(matches!(timings(&site).as_slice(), [HoleTiming::Guarded { .. }, HoleTiming::ApprovalTime]));
    }

    /// A non-null declaration is not presence: an actual null still fails,
    /// and the declaration is not guard evidence.
    #[test]
    fn a_required_declaration_does_not_excuse_an_actual_null() {
        let none = DeferredShellRoots::default();
        let schema = Some("commit_message: string(required)\n");
        assert_eq!(
            missing(candidate(COMMIT, schema, None, json!({ "commit_message": null }), &none, &[])),
            "commit_message"
        );
        let guarded = candidate(COMMIT, schema, Some((COMMIT_GUARD, Polarity::True)), json!({}), &none, &[]).unwrap();
        assert!(matches!(timings(&guarded).as_slice(), [HoleTiming::Guarded { .. }]));
    }

    /// A value present at discovery keeps its approval-time timing, so the
    /// site may still run as resolved bytes.
    #[test]
    fn a_guarded_value_present_at_discovery_stays_approval_time() {
        let none = DeferredShellRoots::default();
        let values = json!({ "commit_message": "msg" });
        let site = candidate(COMMIT, None, Some((COMMIT_GUARD, Polarity::True)), values, &none, &[]).unwrap();
        assert_eq!(timings(&site), [HoleTiming::ApprovalTime]);
        assert_eq!(site.requirement, ModeRequirement::AnyMode);
    }

    /// A guarded missing value requires template mode, so a command outside
    /// the template grammar, or with a placeholder as its program, fails.
    #[test]
    fn a_guarded_missing_value_in_a_non_template_command_fails_its_grammar() {
        let none = DeferredShellRoots::default();
        for (command, reason) in [
            ("git commit -m {{ m }} | cat", TemplateGrammarError::Pipe),
            ("{{ m }} run", TemplateGrammarError::ProgramWordHole),
        ] {
            match candidate(command, None, Some(("m", Polarity::True)), json!({}), &none, &[]) {
                Err(ShellExpansionError::TemplateGrammar { reason: actual, .. }) => assert_eq!(actual, reason, "{command}"),
                other => panic!("{command}: expected a grammar refusal, got {other:?}"),
            }
        }
    }

    /// A late placeholder its guard proves present is guarded, so the
    /// unconditional barrier skips it; an unguarded late one keeps the
    /// barrier.
    #[test]
    fn only_unguarded_late_values_meet_the_barrier() {
        let late = DeferredShellRoots::new(["seq"]);
        let globals = ["seq"];
        let guarded = candidate("tool {{ seq.previous }}", None, Some(("seq.previous", Polarity::True)), json!({}), &late, &globals)
            .unwrap();
        let holes = timings(&guarded);
        let [HoleTiming::Guarded { roots, .. }] = holes.as_slice() else {
            panic!("{holes:?}");
        };
        assert_eq!(roots, &["seq".to_string()]);
        assert_eq!(guarded.requirement, ModeRequirement::TemplateRequired);
        let ordinary = candidate("tool {{ seq.previous }}", None, None, json!({}), &late, &globals).unwrap();
        assert!(matches!(timings(&ordinary).as_slice(), [HoleTiming::Deferred { .. }]));

        let absent = |_: &str| -> Result<Value, crate::markdown::compose::expression::ExpressionError> { Ok(Value::Null) };
        let swept = resolve_late_values([("guarded", guarded.template.as_ref().unwrap())], &absent).unwrap();
        assert!(swept.is_empty(), "an unselected guarded command cannot fail at the barrier");
        let error = resolve_late_values([("ordinary", ordinary.template.as_ref().unwrap())], &absent).unwrap_err();
        assert!(matches!(error.error, HoleError::Missing { .. }), "{error:?}");
    }
}
