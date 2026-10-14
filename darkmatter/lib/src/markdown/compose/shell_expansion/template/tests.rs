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
