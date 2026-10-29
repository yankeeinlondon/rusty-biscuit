//! Guard-narrowed shell templates at the public template API
//! (`2026-10-16-type-narrowing` acceptance 6, 12, and the live-value half of
//! 3): a hole typed at a guarded site approves as its narrowed type, and the
//! value that fills it is checked against both its declaration and that
//! narrowed type when the command is reached.

use darkmatter::markdown::compose::ShellCommandOrigin;
use darkmatter::markdown::compose::ShellExpansionError;
use darkmatter::markdown::compose::expression::{
    DeclaredTypes, ExpressionError, GuardOrigin, Polarity, TypeScope, parse,
};
use darkmatter::markdown::compose::shell_expansion::template::{
    ApprovalType, CommandTemplate, HoleError, HoleTiming, RejectReason, TemplateShape, fill_template,
    parse_authored, resolve_late_values, rule_covers,
};
use darkmatter::markdown::schemas::{SimplifiedSchema, parse_yaml_schema};
use biscuit_terminal::errors::SourceContext;
use serde_json::{Value, json};
use std::cell::RefCell;

const SCHEMA: &str = "target: [string, number, \"null\"]\nmode: \"enum(release, debug, test; required)\"\n";

fn schema() -> SimplifiedSchema {
    let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(SCHEMA).expect("schema YAML");
    parse_yaml_schema(&value).expect("schema parses")
}

/// `command` typed at a site on the true side of `guard`, or unguarded.
fn typed(command: &str, guard: Option<&str>) -> CommandTemplate {
    let schema = schema();
    let declared = DeclaredTypes::new(Some(&schema));
    let site = TypeScope::unguarded(&declared);
    let site = match guard {
        Some(guard) => site.assume(&parse(guard).unwrap(), Polarity::True),
        None => site,
    };
    let mut template = parse_authored(command).unwrap_or_else(|error| panic!("{command}: {error}"));
    template.project_types(&site);
    template
}

fn values(values: Value) -> impl Fn(&str) -> Result<Value, ExpressionError> {
    move |expression: &str| Ok(values.get(expression).cloned().unwrap_or(Value::Null))
}

fn rejection(error: HoleError) -> (RejectReason, String) {
    match error {
        HoleError::Rejected { reason, declared_type, .. } => (reason, declared_type),
        other => panic!("expected a rejection, got {other:?}"),
    }
}

fn ctx() -> SourceContext {
    SourceContext::new("/doc.md".into(), "doc.md".into(), "::shell deploy {{ target }}\n")
}

// ── approval types ──────────────────────────────────────────────────────

/// Acceptance 6: under `is_string(target)` with `target: string | number |
/// null`, the hole approves as `{{ string }}` and is validated as the
/// narrowed `string`; a non-string narrowing keeps the broad approval and
/// says what it validates.
#[test]
fn a_type_predicate_guard_approves_its_hole_as_the_narrowed_type() {
    let guarded = typed("git checkout {{ target }}", Some("is_string(target)"));
    assert_eq!(TemplateShape::from_template(&guarded).canonical_text(), "git checkout {{ string }}");
    let hole = guarded.placeholders().next().unwrap();
    assert_eq!(hole.typing.approval, ApprovalType::BroadString);
    let refinement = hole.typing.refinement.as_ref().expect("the guard narrowed the hole");
    assert_eq!(refinement.text(), "string");
    assert_eq!(refinement.guards.len(), 1);
    assert_eq!(hole.typing.display_type(), "string");

    let number = typed("gh pr checkout {{ target }}", Some("!is_string(target) && target"));
    let hole = number.placeholders().next().unwrap();
    assert_eq!(hole.typing.approval, ApprovalType::BroadString);
    assert_eq!(hole.typing.display_type(), "string (validated as number)");

    let unguarded = typed("git checkout {{ target }}", None);
    assert!(unguarded.placeholders().next().unwrap().typing.refinement.is_none());
}

/// A string-like narrowing approves as the narrower type in the saved rule
/// grammar: its canonical text parses back to the same shape, a saved rule
/// for the whole enum still covers it, and the narrower rule does not cover
/// the unguarded hole.
#[test]
fn a_narrowed_enum_approves_in_the_saved_rule_grammar() {
    let guarded = TemplateShape::from_template(&typed("deploy {{ mode }}", Some("mode != 'release'")));
    let text = guarded.canonical_text();
    assert_eq!(text, "deploy {{ enum(debug,test) }}");
    assert_eq!(TemplateShape::parse_rule(&text).unwrap(), guarded);

    let hole = |shape: &TemplateShape| match &shape.segments[0].args[0].parts[0] {
        darkmatter::markdown::compose::shell_expansion::template::ShapePart::Hole(approval) => approval.clone(),
        other => panic!("{other:?}"),
    };
    let whole = TemplateShape::parse_rule("deploy {{ enum(release, debug, test) }}").unwrap();
    assert!(rule_covers(&hole(&whole), &hole(&guarded)));
    let unguarded = TemplateShape::from_template(&typed("deploy {{ mode }}", None));
    assert!(!rule_covers(&hole(&guarded), &hole(&unguarded)));
}

// ── live values ─────────────────────────────────────────────────────────

/// Acceptance 12: a value that passes its declaration but no longer passes
/// the guard's narrowing (it changed type after the guard) is a typed
/// template-value refusal; a value its declaration refuses is still refused
/// as such first.
#[test]
fn a_live_value_outside_its_guard_refinement_is_refused() {
    let template = typed("git checkout {{ target }}", Some("is_string(target)"));
    let (reason, declared) = rejection(fill_template(&template, &values(json!({ "target": 4 })), None).unwrap_err());
    assert_eq!(reason, RejectReason::GuardRefinement);
    assert!(declared.ends_with("narrowed to string"), "{declared}");
    let shell = fill_template(&template, &values(json!({ "target": 4 })), None)
        .unwrap_err()
        .into_shell_error(&ctx(), "git checkout {{ target }}", &ShellCommandOrigin::Body { line: 1 });
    assert!(matches!(shell, ShellExpansionError::TemplateValueRejected { .. }), "{shell:?}");
    let filled = fill_template(&template, &values(json!({ "target": "main" })), None).unwrap();
    assert_eq!(filled.segments[0].args, ["checkout", "main"]);

    let template = typed("deploy {{ mode }}", Some("mode != 'release'"));
    let refused = |value| rejection(fill_template(&template, &values(json!({ "mode": value })), None).unwrap_err()).0;
    assert_eq!(refused("staging"), RejectReason::DeclaredType, "the declaration is checked first");
    assert_eq!(refused("release"), RejectReason::GuardRefinement);
    assert!(fill_template(&template, &values(json!({ "mode": "debug" })), None).is_ok());
}

/// A guard-proven value that is `null` when its command is reached is
/// missing, never refused or filled empty.
#[test]
fn a_null_when_the_command_is_reached_is_missing() {
    let template = typed("gh pr checkout {{ target }}", Some("target"));
    let error = fill_template(&template, &values(json!({ "target": null })), None).unwrap_err();
    assert!(matches!(&error, HoleError::Missing { expression } if expression == "target"), "{error:?}");
    let shell = error.into_shell_error(&ctx(), "gh pr checkout {{ target }}", &ShellCommandOrigin::Body { line: 1 });
    assert!(matches!(shell, ShellExpansionError::MissingValue { .. }), "{shell:?}");
}

/// A refused value in a later segment refuses the whole chain: no filled
/// chain is returned, so no segment can start.
#[test]
fn a_refused_value_in_any_segment_refuses_the_whole_chain() {
    let template = typed("git fetch && git checkout {{ target }}", Some("is_string(target)"));
    let error = fill_template(&template, &values(json!({ "target": true })), None).unwrap_err();
    assert!(matches!(&error, HoleError::Rejected { expression, .. } if expression == "target"), "{error:?}");
}

/// The value that passed the checks is the value that fills every position
/// its expression holds: it is read once.
#[test]
fn one_checked_value_fills_every_position_of_its_expression() {
    let template = typed("tool {{ target }} --again={{ target }}", Some("is_string(target)"));
    let reads = RefCell::new(0);
    let evaluate = |_: &str| -> Result<Value, ExpressionError> {
        *reads.borrow_mut() += 1;
        Ok(if *reads.borrow() == 1 { json!("first") } else { json!(2) })
    };
    let filled = fill_template(&template, &evaluate, None).unwrap();
    assert_eq!(filled.segments[0].args, ["first", "--again=first"]);
    assert_eq!(*reads.borrow(), 1);
}

// ── the barrier ─────────────────────────────────────────────────────────

/// An ordinary late value keeps its barrier, which checks the declaration;
/// the guard's narrowing is checked only when the command is reached, so a
/// branch that does not run cannot fail on it. A guarded hole is skipped by
/// the barrier entirely.
#[test]
fn the_barrier_leaves_guard_checks_to_the_reached_command() {
    let mut template = typed("tool {{ target }}", Some("is_string(target)"));
    for hole in template.placeholders_mut() {
        hole.timing = HoleTiming::Deferred { roots: vec!["seq".into()] };
    }
    let late = resolve_late_values([("site", &template)], &values(json!({ "target": 4 }))).unwrap();
    let (reason, _) = rejection(fill_template(&template, &values(json!({})), late.site("site")).unwrap_err());
    assert_eq!(reason, RejectReason::GuardRefinement);
    let missing = resolve_late_values([("site", &template)], &values(json!({}))).unwrap_err();
    assert!(matches!(missing.error, HoleError::Missing { .. }), "{missing:?}");

    for hole in template.placeholders_mut() {
        hole.timing = HoleTiming::Guarded { guard: GuardOrigin::new("success[4].when"), roots: Vec::new() };
    }
    let skipped = resolve_late_values([("site", &template)], &values(json!({}))).unwrap();
    assert!(skipped.is_empty());
    let filled = fill_template(&template, &values(json!({ "target": "main" })), skipped.site("site")).unwrap();
    assert_eq!(filled.segments[0].args, ["main"]);
}
