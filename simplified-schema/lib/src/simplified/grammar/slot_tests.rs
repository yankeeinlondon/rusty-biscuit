//! `{{ … }}` slots in the type-expression grammar: where they are admitted,
//! the constraint they build, and the positioned errors around them.

use super::*;

fn parse(input: &str) -> PropertyAtom {
    parse_type_expr("test", input).unwrap_or_else(|error| panic!("{input:?}: {error}"))
}

/// The grammar error message and span for `input`.
fn rejected(input: &str) -> (String, Range<usize>) {
    match parse_type_expr("test", input) {
        Err(SchemaError::Grammar { message, span, .. }) => (message, span),
        other => panic!("{input:?}: expected a grammar error, got {other:?}"),
    }
}

/// The slot whose `{{` opens at the first occurrence of `opener` in `input`.
fn slot_at(input: &str, opener: &str, expression: &str) -> Slot {
    let start = input.find(opener).expect("opener present");
    let end = start + opener.len();
    let expression_start = start + opener.find(expression).expect("expression inside opener");
    Slot {
        expression: expression.to_string(),
        span: start..end,
        expression_span: expression_start..expression_start + expression.len(),
        origin: None,
    }
}

const OS_SLOT: &str = r#"{{ load_data("../../../os.yaml", "string[]") }}"#;

#[test]
fn an_enum_slot_mixes_with_literals_and_keeps_the_constraint_tail() {
    let input = format!("enum(none, {OS_SLOT}; required)");
    let atom = parse(&input);
    let expression = r#"load_data("../../../os.yaml", "string[]")"#;
    assert_eq!(
        atom.constraints,
        vec![
            Constraint::Dynamic(DynamicList {
                kind: DynamicKind::Members,
                target: SimplifiedType::Enum,
                elements: vec![
                    DynamicElement::Member("none".into()),
                    DynamicElement::Slot(slot_at(&input, OS_SLOT, expression)),
                ],
            }),
            Constraint::Required,
        ]
    );
}

#[test]
fn slots_and_literals_alternate_in_authored_order() {
    let input = "enum({{ a }}, b, 'c, d', {{ e || f }}, 3; default(b))";
    let atom = parse(input);
    let Constraint::Dynamic(list) = &atom.constraints[0] else {
        panic!("{:?}", atom.constraints);
    };
    assert_eq!(
        list.elements,
        vec![
            DynamicElement::Slot(slot_at(input, "{{ a }}", "a")),
            DynamicElement::Member("b".into()),
            DynamicElement::Member("c, d".into()),
            DynamicElement::Slot(slot_at(input, "{{ e || f }}", "e || f")),
            DynamicElement::Member("3".into()),
        ]
    );
    assert_eq!(atom.constraints[1], Constraint::Default(serde_json::json!("b")));
    assert_eq!(atom.constraints[0].keyword(), "{{");
}

#[test]
fn a_static_list_never_builds_a_dynamic_constraint() {
    assert_eq!(
        parse("enum(a, b; required)").constraints,
        vec![Constraint::Members(vec!["a".into(), "b".into()]), Constraint::Required]
    );
    assert!(matches!(
        parse("string(suggest(a, b))").constraints.as_slice(),
        [Constraint::Suggest(candidates)] if candidates.len() == 2
    ));
}

#[test]
fn a_brace_that_does_not_open_an_element_is_text() {
    // Quoted: always a literal member.
    assert_eq!(
        parse("enum('{{ x }}', \"{{ y }}\")").constraints,
        vec![Constraint::Members(vec!["{{ x }}".into(), "{{ y }}".into()])]
    );
    // After other text in the same word: today's bare-word member.
    assert_eq!(
        parse("enum(abc{{x}}, b)").constraints,
        vec![Constraint::Members(vec!["abc{{x}}".into(), "b".into()])]
    );
    // A triple-brace literal and a literal token are not slots.
    assert!(matches!(parse("enum('a', {{!data:abc}})").constraints.as_slice(), [Constraint::Members(_)]));
    // A negation without a space is still an expression.
    assert!(matches!(parse("enum(a, {{!flag}})").constraints.as_slice(), [Constraint::Dynamic(_)]));
    // A quoted suggestion stays a static candidate.
    assert!(matches!(
        parse("string(suggest('{{ x }}'))").constraints.as_slice(),
        [Constraint::Suggest(candidates)] if candidates[0].decoded == "{{ x }}"
    ));
}

#[test]
fn a_slot_must_be_a_complete_list_element() {
    let input = "enum(a, {{ b }} c)";
    let (message, span) = rejected(input);
    assert_eq!(message, SLOT_COMPLETE_ELEMENT);
    assert_eq!(span.start, input.find("{{").unwrap());
    let (message, _) = rejected("string(suggest({{ b }}x))");
    assert_eq!(message, SLOT_COMPLETE_ELEMENT);
}

#[test]
fn an_unterminated_or_empty_slot_is_positioned_at_its_opener() {
    let input = "enum(a, {{ b, c)";
    let (message, span) = rejected(input);
    assert_eq!(message, "unterminated `{{ … }}` slot");
    assert_eq!(span, input.find("{{").unwrap()..input.len());

    let input = "enum(a, {{  }})";
    let (message, span) = rejected(input);
    assert_eq!(message, "a `{{ … }}` slot needs an expression");
    assert_eq!(&input[span], "{{  }}");
}

#[test]
fn a_slot_expression_parse_error_maps_into_the_type_expression() {
    let input = "enum(a, {{ x + }})";
    let (message, span) = rejected(input);
    assert!(message.starts_with("invalid `{{ … }}` slot expression"), "{message}");
    let expression = input.find("x +").unwrap();
    assert!(
        span.start >= expression && span.end <= expression + "x +".len(),
        "{span:?} lies outside the expression"
    );
}

#[test]
fn a_suggestion_slot_keeps_every_element_and_its_target() {
    let input = "number(suggest(1, {{ n }}, 1.50))";
    let atom = parse(input);
    let Constraint::Dynamic(list) = &atom.constraints[0] else {
        panic!("{:?}", atom.constraints);
    };
    assert_eq!(list.kind, DynamicKind::Suggest);
    assert_eq!(list.target, SimplifiedType::Number);
    assert!(matches!(
        list.elements.as_slice(),
        [
            DynamicElement::Suggestion(one),
            DynamicElement::Slot(_),
            DynamicElement::Suggestion(decimal),
        ] if one.interpreted == serde_json::json!(1) && decimal.canonical_decimal.as_deref() == Some("1.5")
    ));

    // A reference's candidates are strings until resolution.
    let atom = parse("slug(suggest({{ s }}))");
    assert!(matches!(
        &atom.constraints[0],
        Constraint::Dynamic(DynamicList { kind: DynamicKind::Suggest, target: SimplifiedType::String, .. })
    ));
}

#[test]
fn duplicate_literals_are_kept_only_in_a_dynamic_suggestion_list() {
    let atom = parse("string(suggest(a, {{ x }}, a))");
    let Constraint::Dynamic(list) = &atom.constraints[0] else {
        panic!("{:?}", atom.constraints);
    };
    assert_eq!(list.elements.len(), 3);
    let (message, _) = rejected("string(suggest(a, a))");
    assert_eq!(message, "duplicate suggestion candidate");
}

#[test]
fn a_definition_still_holds_one_suggestion_list() {
    let (message, _) = rejected("string(suggest({{ a }}); suggest(b))");
    assert!(message.contains("at most one `suggest`"), "{message}");
    let error = parse_type_definition("test", "string(suggest({{ a }})) | number(suggest(1))")
        .expect_err("a second list in another option");
    assert!(error.to_string().contains("suggest"), "{error}");
}

#[test]
fn suggestion_slots_follow_the_exact_type_rule() {
    let (message, _) = rejected("date(suggest({{ d }}))");
    assert!(message.contains("`suggest` is not valid"), "{message}");
}

#[test]
fn template_constraints_take_a_whole_argument_slot() {
    let input = "literal({{ revision }}; required)";
    assert_eq!(
        parse(input).constraints,
        vec![
            Constraint::TemplateConstraint {
                kind: TemplateConstraintKind::Literal,
                target: SimplifiedType::Literal,
                slot: slot_at(input, "{{ revision }}", "revision"),
            },
            Constraint::Required,
        ]
    );

    let input = "number(min({{ low }}); max({{ high }}); default({{ d }}))";
    let kinds: Vec<_> = parse(input)
        .constraints
        .iter()
        .map(|constraint| match constraint {
            Constraint::TemplateConstraint { kind, target, slot } => (*kind, *target, slot.expression.clone()),
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(
        kinds,
        vec![
            (TemplateConstraintKind::Min, SimplifiedType::Number, "low".to_string()),
            (TemplateConstraintKind::Max, SimplifiedType::Number, "high".to_string()),
            (TemplateConstraintKind::Default, SimplifiedType::Number, "d".to_string()),
        ]
    );

    let atom = parse("string(pattern({{ p }}))[](max({{ n }}))");
    assert!(matches!(
        &atom.constraints[..],
        [Constraint::TemplateConstraint { kind: TemplateConstraintKind::Pattern, target: SimplifiedType::String, .. }]
    ));
    // An array-level slot stays in the array list, which tells resolution it
    // counts items.
    assert!(matches!(
        &atom.array_constraints[..],
        [Constraint::TemplateConstraint { kind: TemplateConstraintKind::Max, .. }]
    ));
    // On a reference the target is not known yet.
    assert!(matches!(
        &parse("slug(default({{ s }}))").constraints[..],
        [Constraint::TemplateConstraint { kind: TemplateConstraintKind::Default, target: SimplifiedType::Unknown, .. }]
    ));
}

#[test]
fn a_template_constraint_slot_must_be_the_whole_argument() {
    for input in [
        "number(default({{ d }} + 1))",
        "string(default(a, {{ d }}))",
        "number(min({{ a }}, 2))",
        "string(pattern({{ p }}$))",
        "literal({{ v }} tail)",
    ] {
        let (message, span) = rejected(input);
        assert_eq!(message, SLOT_WHOLE_ARGUMENT, "for {input:?}");
        assert_eq!(&input[span.start..span.start + 2], "{{", "for {input:?}");
    }
    let (message, _) = rejected("literal({{ a }}, {{ b }})");
    assert!(message.contains("exactly one value"), "{message}");
}

#[test]
fn template_constraints_keep_the_type_rules_of_their_position() {
    let (message, _) = rejected("null(default({{ x }}))");
    assert!(message.contains("`null` default"), "{message}");
    let (message, _) = rejected("type-definition(default({{ x }}))");
    assert!(message.contains("not supported on `type-definition`"), "{message}");
    // Other argument lists keep today's reading of a `{{`.
    assert!(parse_type_expr("test", "file(match({{ x }}))").is_err());
}

#[test]
fn a_slot_inside_a_union_and_an_array_parses_as_one_option() {
    let def = parse_type_definition("test", "enum({{ a || b }}, c)[] | null -> the targets")
        .expect("parses");
    let PropertyDef::Union(arms) = def else {
        panic!("{def:?}");
    };
    assert_eq!(arms.len(), 2);
    assert!(arms[0].is_array);
    assert!(matches!(&arms[0].constraints[0], Constraint::Dynamic(_)));
}
