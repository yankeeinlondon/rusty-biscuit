//! The shared type scope through its public API (`2026-10-16-type-narrowing`
//! acceptance 6, 11, 12, and 13 at the analysis core): the declared layer,
//! guard facts on both sides, joins, and write invalidation.
//!
//! The differential cases evaluate each guard with the real evaluator over
//! sample values the declaration admits, and require the analyzed side to
//! admit every value that reaches it. The analysis itself never evaluates.

use std::collections::HashMap;

use darkmatter::markdown::compose::expression::{
    Availability, BindingView, DeclaredType, DeclaredTypes, EvaluationLookup, Polarity, ResolutionContext, ScopeId,
    ScopedType, Shape, Subject, TypeScope, UnavailabilityReason, WriteSet, evaluate, parse,
};
use darkmatter::markdown::schemas::simplified::serialize_property_atom;
use darkmatter::markdown::schemas::{SimplifiedSchema, parse_yaml_schema};
use serde_json::{Value, json};

fn schema(yaml: &str) -> SimplifiedSchema {
    let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(yaml).expect("schema YAML");
    parse_yaml_schema(&value).expect("schema parses")
}

fn expr(text: &str) -> darkmatter::markdown::compose::expression::Expr {
    parse(text).unwrap_or_else(|error| panic!("{text}: {error}"))
}

fn type_at(scope: &TypeScope<'_>, subject: &str) -> ScopedType {
    scope.type_of_expr(&expr(subject)).unwrap_or_else(|| panic!("{subject} is a stable subject"))
}

/// The narrowed shape as text: the canonical alternatives, plus `null`.
fn shape_text(ty: &ScopedType) -> String {
    match ty.shape() {
        Shape::Unknown { non_null: true } => "unknown(present)".to_string(),
        Shape::Unknown { non_null: false } => "unknown".to_string(),
        Shape::Arms { arms, nullable } => {
            let mut parts: Vec<String> = arms
                .iter()
                .map(|arm| {
                    let mut arm = arm.clone();
                    arm.constraints.retain(|constraint| constraint.keyword() != "required");
                    serialize_property_atom(&arm)
                })
                .collect();
            if *nullable {
                parts.push("null".to_string());
            }
            parts.join(" | ")
        }
    }
}

fn narrowed(declared: &DeclaredTypes<'_>, guard: &str, polarity: Polarity, subject: &str) -> String {
    let scope = TypeScope::unguarded(declared).assume(&expr(guard), polarity);
    shape_text(&type_at(&scope, subject))
}

// ── declared layer ──────────────────────────────────────────────────────

#[test]
fn optional_declarations_admit_null_and_required_ones_are_present() {
    let schema = schema("name: string\nneeded: string(required)\nunion: [string, number]\nmaybe: [string, \"null\"]\n");
    let declared = DeclaredTypes::new(Some(&schema));
    let scope = TypeScope::unguarded(&declared);
    assert_eq!(shape_text(&type_at(&scope, "name")), "string | null");
    assert!(!type_at(&scope, "name").proves_present());
    assert!(type_at(&scope, "needed").proves_present());
    assert!(type_at(&scope, "needed").guard_presence().is_none(), "a declaration is not guard evidence");
    assert_eq!(shape_text(&type_at(&scope, "union")), "number | string | null");
    assert_eq!(shape_text(&type_at(&scope, "maybe")), "string | null");
    assert!(matches!(type_at(&scope, "missing").declared(), DeclaredType::Undeclared));
    assert_eq!(shape_text(&type_at(&scope, "missing")), "unknown");
}

#[test]
fn bare_and_fm_reads_share_one_document_binding_unless_a_host_root_shadows_it() {
    let declared = DeclaredTypes::new(None);
    let guarded = TypeScope::unguarded(&declared).assume(&expr("fm.x"), Polarity::True);
    assert!(type_at(&guarded, "x").proves_present());
    assert!(type_at(&guarded, "fm['x']").proves_present());

    let roots = ["x"];
    let shadowed = DeclaredTypes::new(None).with_host_globals(&roots);
    let scope = TypeScope::unguarded(&shadowed).assume(&expr("fm.x"), Polarity::True);
    assert!(!type_at(&scope, "x").proves_present(), "bare `x` is the host global");
    assert!(type_at(&scope, "fm.x").proves_present());
    assert!(matches!(type_at(&scope, "x").declared(), DeclaredType::Host(Availability::Available)));
}

#[test]
fn constant_paths_and_indices_are_subjects_and_computed_ones_are_not() {
    let declared = DeclaredTypes::new(None);
    let scope = TypeScope::unguarded(&declared).assume(&expr("a.b && c[0] && d['k'] && e[-1].f"), Polarity::True);
    for subject in ["a.b", "c[0]", "d.k", "e[-1].f"] {
        assert!(type_at(&scope, subject).proves_present(), "{subject}");
    }
    // A fact on a path proves nothing about its parent or a sibling.
    assert!(!type_at(&scope, "a").proves_present());
    assert!(!type_at(&scope, "a.c").proves_present());
    // No schema types the path, so its declaration is `unknown`.
    assert!(matches!(type_at(&scope, "a.b").declared(), DeclaredType::NoProjection));
    for unstable in ["c[i]", "length(x)", "fm", "current.today", "null"] {
        assert!(scope.type_of_expr(&expr(unstable)).is_none(), "{unstable}");
    }
    let unstable = TypeScope::unguarded(&declared).assume(&expr("is_number(length(x))"), Polarity::True);
    assert!(!type_at(&unstable, "x").proves_present(), "a function result proves nothing about its argument");
}

#[test]
fn descriptors_and_static_values_are_presence_evidence_until_a_write() {
    let frontmatter = json!({ "title": "Release", "empty": null, "list": [1] });
    let declared = DeclaredTypes::new(None).with_static_frontmatter(&frontmatter);
    let scope = TypeScope::unguarded(&declared);
    assert!(type_at(&scope, "title").proves_present());
    assert!(!type_at(&scope, "empty").proves_present());
    assert!(!type_at(&scope, "list").proves_present(), "only a concrete scalar counts");
    let written = scope.invalidate(&WriteSet::Paths(vec![Subject::document_path("title").unwrap()]));
    assert!(!type_at(&written, "title").proves_present());
    assert!(type_at(&written, "title").was_written());

    assert!(matches!(type_at(&scope, "ctx.today").declared(), DeclaredType::Context { .. }));
    assert!(matches!(type_at(&scope, "ctx.no_such_member").declared(), DeclaredType::Namespace));
    assert!(matches!(type_at(&scope, "env.HOME").declared(), DeclaredType::Namespace));
    assert!(!type_at(&scope, "env.HOME").proves_present());
}

#[test]
fn an_unavailable_global_stays_unavailable_under_any_guard() {
    let reason = UnavailabilityReason::new("composition.event");
    let view = BindingView::builder(ScopeId::new("test.event"))
        .declare("err", "the failure", Availability::Unavailable(reason.clone()))
        .declare("seq", "the sequence", Availability::Available)
        .build()
        .expect("view builds");
    let declared = DeclaredTypes::new(None).with_binding_view(&view);
    let scope = TypeScope::unguarded(&declared).assume(&expr("err.message && seq.previous"), Polarity::True);
    let err = type_at(&scope, "err.message");
    assert_eq!(err.declared(), &DeclaredType::Host(Availability::Unavailable(reason)));
    assert!(!err.proves_present() && !err.is_refined());
    assert!(type_at(&scope, "seq.previous").proves_present());
}

#[test]
fn a_child_scope_shares_its_parent_and_never_changes_it() {
    let declared = DeclaredTypes::new(None);
    let outer = TypeScope::unguarded(&declared).assume(&expr("a"), Polarity::True);
    let inner = outer.assume(&expr("b"), Polarity::True);
    assert!(type_at(&inner, "a").proves_present() && type_at(&inner, "b").proves_present());
    assert!(!type_at(&outer, "b").proves_present());
    assert!(!type_at(&TypeScope::unguarded(&declared), "a").proves_present());
}

// ── guard facts ─────────────────────────────────────────────────────────

/// The motivating guard: `commit_message` is present only on the true side.
#[test]
fn a_conjunction_proves_each_operand_on_its_true_side_only() {
    let declared = DeclaredTypes::new(None);
    let guard = expr("ctx.dirty_files && commit_message");
    let site = TypeScope::unguarded(&declared);
    let then = site.assume(&guard, Polarity::True);
    let message = type_at(&then, "commit_message");
    assert!(message.proves_present());
    let origin = message.guard_presence().expect("the guard is the presence evidence");
    assert!(origin.label().contains("commit_message"), "{}", origin.label());
    assert!(type_at(&then, "ctx.dirty_files").proves_present());
    let otherwise = site.assume(&guard, Polarity::False);
    assert!(!type_at(&otherwise, "commit_message").proves_present());
    assert!(type_at(&otherwise, "ctx.dirty_files").guard_presence().is_none());
    // Function spelling, case, and a second conjunct.
    let spelled = site.assume(&expr("AND(a, b, c)"), Polarity::True);
    assert!(["a", "b", "c"].iter().all(|subject| type_at(&spelled, subject).proves_present()));
}

#[test]
fn a_disjunction_proves_what_both_routes_prove() {
    let declared = DeclaredTypes::new(None);
    let site = TypeScope::unguarded(&declared);
    let either = site.assume(&expr("a || b"), Polarity::True);
    assert!(!type_at(&either, "a").proves_present() && !type_at(&either, "b").proves_present());
    let both = site.assume(&expr("(a && b) || (b && a)"), Polarity::True);
    assert!(type_at(&both, "a").proves_present() && type_at(&both, "b").proves_present());
    let neither = site.assume(&expr("or(is_null(a), is_null(b))"), Polarity::False);
    assert!(type_at(&neither, "a").proves_present() && type_at(&neither, "b").proves_present());
    // `a || b` false: both falsy, which proves nothing.
    let falsy = site.assume(&expr("a || b"), Polarity::False);
    assert!(!type_at(&falsy, "a").proves_present());
    // The right operand is checked under the left's false facts.
    let routed = site.assume(&expr("is_null(a) || a == 'x'"), Polarity::True);
    assert!(!type_at(&routed, "a").proves_present());
}

#[test]
fn negation_swaps_sides_and_parentheses_keep_facts() {
    let declared = DeclaredTypes::new(None);
    let site = TypeScope::unguarded(&declared);
    assert!(type_at(&site.assume(&expr("!!x"), Polarity::True), "x").proves_present());
    assert!(type_at(&site.assume(&expr("!x"), Polarity::False), "x").proves_present());
    assert!(!type_at(&site.assume(&expr("!x"), Polarity::True), "x").proves_present());
    assert!(type_at(&site.assume(&expr("((x))"), Polarity::True), "x").proves_present());
    assert!(type_at(&site.assume(&expr("!is_null(x)"), Polarity::True), "x").proves_present());
    assert_eq!(shape_text(&type_at(&site.assume(&expr("!is_null(x)"), Polarity::False), "x")), "null");
}

#[test]
fn a_ternary_guard_joins_its_branch_routes() {
    let declared = DeclaredTypes::new(None);
    let site = TypeScope::unguarded(&declared);
    let both = site.assume(&expr("c ? x : x"), Polarity::True);
    assert!(type_at(&both, "x").proves_present());
    assert!(!type_at(&both, "c").proves_present());
    let one = site.assume(&expr("c ? x : y"), Polarity::True);
    assert!(!type_at(&one, "x").proves_present());
}

/// Acceptance 6 at the core, and the specification's narrowing example.
#[test]
fn type_predicates_narrow_a_declared_union_on_both_sides() {
    let schema = schema("target: [string, number, \"null\"]\nmode: \"enum(release, debug, test; required)\"\n");
    let declared = DeclaredTypes::new(Some(&schema));
    assert_eq!(narrowed(&declared, "is_string(target)", Polarity::True, "target"), "string");
    assert_eq!(narrowed(&declared, "!is_string(target) && target", Polarity::True, "target"), "number");
    assert_eq!(narrowed(&declared, "is_string(target)", Polarity::False, "target"), "number | null");
    assert_eq!(narrowed(&declared, "isNumber(target)", Polarity::True, "target"), "number");
    assert_eq!(narrowed(&declared, "is_null(target)", Polarity::True, "target"), "null");
    assert_eq!(narrowed(&declared, "mode != 'release'", Polarity::True, "mode"), "enum(debug,test)");
    assert_eq!(narrowed(&declared, "is_array(target)", Polarity::True, "target"), "number | string | null", "contradiction keeps the incoming type");
    let site = TypeScope::unguarded(&declared);
    let contradicted = site.assume(&expr("is_array(target)"), Polarity::True);
    assert!(type_at(&contradicted, "target").guard_presence().is_none(), "a contradiction is no presence proof");
    // From unknown, the tested family only.
    let open = DeclaredTypes::new(None);
    assert_eq!(narrowed(&open, "is_string(x)", Polarity::True, "x"), "string");
    assert_eq!(narrowed(&open, "is_string(x)", Polarity::False, "x"), "unknown");
    assert_eq!(narrowed(&open, "is_array(x)", Polarity::True, "x"), "unknown[]");
}

#[test]
fn file_exists_proves_a_present_file_without_touching_the_filesystem() {
    // The path names nothing on disk (and would be fetched if it were run):
    // the true side holds anyway, because no handler runs.
    let frontmatter = json!({ "plan": "https://127.0.0.1:9/never-fetched.md" });
    let schema = schema("plan: [string, number]\n");
    let declared = DeclaredTypes::new(Some(&schema)).with_static_frontmatter(&frontmatter);
    assert_eq!(narrowed(&declared, "file_exists(plan)", Polarity::True, "plan"), "file");
    assert_eq!(narrowed(&declared, "fileExists(plan)", Polarity::True, "plan"), "file");
    let unknown = DeclaredTypes::new(None);
    assert_eq!(narrowed(&unknown, "file_exists(plan)", Polarity::False, "plan"), "unknown");
    assert_eq!(narrowed(&unknown, "file_exists(plan)", Polarity::True, "plan"), "file");
}

/// The specification's equality table, row by row.
#[test]
fn equality_narrows_to_literals_of_every_scalar_family() {
    let schema = schema(concat!(
        "name: \"string(required)\"\n",
        "mode: \"enum(release, debug; required)\"\n",
        "count: \"number(required)\"\n",
        "either: [string, number]\n",
        "flag: \"boolean(required)\"\n",
        "optional: string\n",
        "pair: \"enum(a, b; required)\"\n",
    ));
    let declared = DeclaredTypes::new(Some(&schema));
    let rows: &[(&str, &str, &str, &str)] = &[
        ("name == 'release'", "name", "literal(release)", "string"),
        ("mode == 'release'", "mode", "literal(release)", "literal(debug)"),
        ("count == 4", "count", "literal(4)", "number"),
        // A number never equals text, so the numeric arm is gone.
        ("either == '4'", "either", "literal('4')", "number | string | null"),
        ("either == 4", "either", "literal(4)", "number | string | null"),
        ("flag == true", "flag", "literal(true)", "literal(false)"),
        ("flag != true", "flag", "literal(false)", "literal(true)"),
        // `null` equals only `null`: `''` proves presence, `null` proves absence.
        ("optional == ''", "optional", "literal('')", "string | null"),
        ("optional != ''", "optional", "string | null", "literal('')"),
        ("optional != null", "optional", "string", "null"),
        ("optional == null", "optional", "null", "string"),
        ("optional == 'a'", "optional", "literal(a)", "string | null"),
        ("optional != 'a'", "optional", "string | null", "literal(a)"),
        ("pair == 'z'", "pair", "enum(a,b)", "enum(a,b)"),
        // Reversed operands use the same rules.
        ("'release' == mode", "mode", "literal(release)", "literal(debug)"),
        ("4 == count", "count", "literal(4)", "number"),
        ("null == optional", "optional", "null", "string"),
        ("'' != optional", "optional", "string | null", "literal('')"),
        ("!(optional == null)", "optional", "string", "null"),
    ];
    for (guard, subject, true_side, false_side) in rows {
        assert_eq!(narrowed(&declared, guard, Polarity::True, subject), *true_side, "{guard} true");
        assert_eq!(narrowed(&declared, guard, Polarity::False, subject), *false_side, "{guard} false");
    }
}

#[test]
fn equality_keeps_constraints_and_numeric_families_it_cannot_pin() {
    let schema = schema("mail: email\ncount: \"number(min(1))\"\nbig: number\n");
    let declared = DeclaredTypes::new(Some(&schema));
    assert_eq!(narrowed(&declared, "mail == 'x'", Polarity::True, "mail"), "email");
    assert_eq!(narrowed(&declared, "count == 4", Polarity::True, "count"), "number(min(1))");
    // A number never equals text.
    assert_eq!(narrowed(&declared, "big == '4.0'", Polarity::True, "big"), "number | null", "contradiction keeps the incoming type");
    assert_eq!(narrowed(&declared, "big == '4'", Polarity::True, "big"), "number | null", "contradiction keeps the incoming type");
    assert_eq!(narrowed(&declared, "big == 4.5", Polarity::True, "big"), "literal(4.5)");
    // `4.0` and `-0` are the integers the evaluator makes of them; the
    // literal still admits the float storage of the same value.
    assert_eq!(narrowed(&declared, "big == 4.0", Polarity::True, "big"), "literal(4)");
    assert_eq!(narrowed(&declared, "big == -0", Polarity::True, "big"), "literal(0)");
    assert_eq!(narrowed(&declared, "big == -4.5", Polarity::True, "big"), "literal(-4.5)");
}

/// The analysis compares the value the evaluator produces for a literal. A
/// whole literal no `f64` holds exactly never reaches either: the lexer
/// rejects it.
#[test]
fn literal_numbers_narrow_to_the_value_the_evaluator_compares() {
    let schema = schema("big: number
");
    let declared = DeclaredTypes::new(Some(&schema));
    let lookup = Values(HashMap::from([("big".to_string(), json!(9_007_199_254_740_992_u64))]), None);
    assert_eq!(evaluate(&expr("big == 9007199254740992"), &lookup).unwrap(), json!(true));
    assert_eq!(narrowed(&declared, "big == 9007199254740992", Polarity::True, "big"), "literal(9007199254740992)");
    for inexact in ["big == 9007199254740993", "big == 99999999999999999999"] {
        let error = parse(inexact).expect_err(inexact).to_string();
        assert!(error.contains("cannot be represented exactly"), "{inexact}: {error}");
    }
}

#[test]
fn equality_pins_an_unknown_value_and_a_container_to_its_family() {
    let schema = schema("items: \"string[]\"\nchoice: [\"string[]\", object, string]\n");
    let declared = DeclaredTypes::new(Some(&schema));
    // An undeclared value equal to a scalar literal is that literal.
    assert_eq!(narrowed(&declared, "x == 'a'", Polarity::True, "x"), "literal(a)");
    assert_eq!(narrowed(&declared, "x == ''", Polarity::True, "x"), "literal('')");
    assert_eq!(narrowed(&declared, "x == 4", Polarity::True, "x"), "literal(4)");
    assert_eq!(narrowed(&declared, "x == false", Polarity::True, "x"), "literal(false)");
    assert_eq!(narrowed(&declared, "x == null", Polarity::True, "x"), "null");
    assert_eq!(narrowed(&declared, "x != null", Polarity::True, "x"), "unknown(present)");
    assert_eq!(narrowed(&declared, "x != 'a'", Polarity::True, "x"), "unknown");
    assert_eq!(narrowed(&declared, "x == ''", Polarity::False, "x"), "unknown");
    // A container never equals its JSON text.
    assert_eq!(narrowed(&declared, "items == '[]'", Polarity::True, "items"), "string[] | null", "contradiction keeps the incoming type");
    // Equal to a container literal: present, of that family.
    assert_eq!(narrowed(&declared, "x == []", Polarity::True, "x"), "unknown[]");
    assert_eq!(narrowed(&declared, "x == {}", Polarity::True, "x"), "object");
    assert_eq!(narrowed(&declared, "items == ['a']", Polarity::True, "items"), "string[]");
    assert_eq!(narrowed(&declared, "choice == []", Polarity::True, "choice"), "string[]");
    assert_eq!(narrowed(&declared, "choice == {'k': x}", Polarity::True, "choice"), "object");
    assert_eq!(narrowed(&declared, "[] == choice", Polarity::True, "choice"), "string[]");
    assert_eq!(narrowed(&declared, "choice != []", Polarity::True, "choice"), "object | string | string[] | null");
    assert_eq!(narrowed(&declared, "choice == []", Polarity::False, "choice"), "object | string | string[] | null");
}

#[test]
fn equality_between_two_subjects_narrows_against_a_finite_other_side() {
    let schema = schema("left: \"enum(a, b, c; required)\"\nright: \"enum(b, c, d; required)\"\nfree: string\n");
    let declared = DeclaredTypes::new(Some(&schema));
    assert_eq!(narrowed(&declared, "left == right", Polarity::True, "left"), "literal(b) | literal(c)");
    assert_eq!(narrowed(&declared, "left == right", Polarity::True, "right"), "literal(b) | literal(c)");
    assert_eq!(narrowed(&declared, "left == right", Polarity::False, "left"), "enum(a,b,c)");
    assert_eq!(narrowed(&declared, "free == left", Polarity::True, "free"), "literal(a) | literal(b) | literal(c)");
    assert_eq!(narrowed(&declared, "free == other", Polarity::True, "free"), "string | null");
}

/// A declaration that admits only `null` is a usable one-value set.
#[test]
fn equality_with_a_null_only_subject_narrows_the_other_side() {
    let schema = schema("free: string\nnothing: \"null\"\n");
    let declared = DeclaredTypes::new(Some(&schema));
    assert_eq!(narrowed(&declared, "free == nothing", Polarity::True, "free"), "null");
    assert_eq!(narrowed(&declared, "free == nothing", Polarity::False, "free"), "string");
    assert_eq!(narrowed(&declared, "free != nothing", Polarity::True, "free"), "string");
    assert_eq!(narrowed(&declared, "nothing != free", Polarity::False, "free"), "null");
}

// ── differential soundness against the evaluator ────────────────────────

/// The sample values, and the directory `file_exists` resolves against.
struct Values(HashMap<String, Value>, Option<ResolutionContext>);

impl EvaluationLookup for Values {
    fn get(&self, path: &str) -> Option<Value> {
        self.0.get(path).cloned()
    }

    fn resolution_context(&self) -> Option<ResolutionContext> {
        self.1.clone()
    }
}

fn samples() -> Vec<Value> {
    vec![
        Value::Null,
        json!(""),
        json!("a"),
        json!("b"),
        json!("4"),
        json!("4.0"),
        json!("release"),
        json!("true"),
        json!("[]"),
        json!(0),
        json!(4),
        json!(4.5),
        json!(-1),
        json!(true),
        json!(false),
        json!([]),
        json!(["a"]),
        json!({}),
        json!({ "k": "v" }),
        // Equal values in other storage, and values that differ only deeply.
        json!(4.0),
        json!(0.0),
        json!(-0.0),
        json!(9_007_199_254_740_992_u64),
        json!(9_007_199_254_740_993_u64),
        json!(["a", ["b"]]),
        json!({ "k": null }),
        json!({ "j": "v" }),
        json!("null"),
    ]
}

/// For every sample pair the declarations admit, the side the evaluator
/// takes admits both values, and a side that proves presence never sees
/// `null`.
#[track_caller]
fn assert_sound(declared: &DeclaredTypes<'_>, guard: &str, files: &ResolutionContext) {
    let site = TypeScope::unguarded(declared);
    let parsed = expr(guard);
    let sides = [
        (Polarity::True, site.assume(&parsed, Polarity::True)),
        (Polarity::False, site.assume(&parsed, Polarity::False)),
    ];
    let mut reached = [false, false];
    for x in samples().into_iter().filter(|value| type_at(&site, "x").admits(value)) {
        for y in samples().into_iter().filter(|value| type_at(&site, "y").admits(value)) {
            let lookup = Values(
                HashMap::from([("x".to_string(), x.clone()), ("y".to_string(), y.clone())]),
                Some(files.clone()),
            );
            let Ok(outcome) = evaluate(&parsed, &lookup) else {
                continue;
            };
            let taken = if darkmatter::markdown::compose::expression::is_truthy(&outcome) { 0 } else { 1 };
            reached[taken] = true;
            let (polarity, scope) = &sides[taken];
            for (name, value) in [("x", &x), ("y", &y)] {
                let ty = type_at(scope, name);
                assert!(
                    ty.admits(value),
                    "{guard} is {polarity:?} for x={x}, y={y}, but the analyzed {name} ({}) excludes {value}",
                    shape_text(&ty)
                );
            }
        }
    }
    assert!(reached[0] || reached[1], "{guard}: the evaluator must take a side");
}

const GUARDS: &[&str] = &[
    "x",
    "!x",
    "x && y",
    "x || y",
    "!(x && y)",
    "x ? y : !y",
    "x == ''",
    "x != ''",
    "x == null",
    "x != null",
    "null == x",
    "x == y",
    "x != y",
    "x == 'a'",
    "'a' == x",
    "x != 'a'",
    "x == 4",
    "x == '4'",
    "x == 4.5",
    "x != 4",
    "x == -1",
    "x == true",
    "x != false",
    "x == 'true'",
    "x == '[]'",
    "x == 4.0",
    "x == 0",
    "x == -0",
    "x == 9007199254740992",
    "x != 9007199254740992",
    "x == 'null'",
    "x == []",
    "x != []",
    "x == ['a']",
    "x == {}",
    "{'k': 'v'} == x",
    "null != x",
    "!(x == null)",
    "y == null",
    "y == 9007199254740992",
    "y != 9007199254740992",
    "x == y && y == null",
    "is_string(x)",
    "!isString(x)",
    "is_number(x)",
    "is_array(x)",
    "IS_OBJECT(x)",
    "is_null(x)",
    "!isnull(x)",
    "is_empty(x)",
    "!is_empty(x)",
    "is_string(x) && x == y",
    "and(is_number(x), x == y)",
    "or(x == 'a', y == 'b')",
    "x == 'a' || x == 'b'",
    "!(x == 'a' || x == 4)",
    "file_exists(x)",
    "!fileExists(x)",
];

/// The declarations the differential guards run against; `None` is no
/// schema at all.
const DIFFERENTIAL_SCHEMAS: &[(&str, Option<&str>)] = &[
    ("optional_strings", Some("x: string\ny: string\n")),
    ("required_string_optional_number", Some("x: \"string(required)\"\ny: number\n")),
    ("scalar_unions", Some("x: [string, number]\ny: [string, number, boolean]\n")),
    ("enums", Some("x: \"enum(a, b, release; required)\"\ny: \"enum(b, 4)\"\n")),
    ("booleans", Some("x: \"boolean(required)\"\ny: boolean\n")),
    ("numbers", Some("x: \"number(required)\"\ny: [number, \"null\"]\n")),
    ("literals", Some("x: \"literal('a')\"\ny: \"literal(4; required)\"\n")),
    ("containers", Some("x: \"string[]\"\ny: object\n")),
    ("container_unions", Some("x: [\"string[]\", string]\ny: [object, number]\n")),
    ("null_other_side", Some("x: [string, number]\ny: \"null\"\n")),
    ("large_integers", Some("x: number\ny: [\"literal(9007199254740992)\", string]\n")),
    ("undeclared", None),
];

fn assert_differential(name: &str) {
    let (_, text) = DIFFERENTIAL_SCHEMAS.iter().find(|(row, _)| *row == name).expect("a declared row");
    let schema = text.map(schema);
    let declared = DeclaredTypes::new(schema.as_ref());
    // Only the sample `a` names a file, so `file_exists(x)` reaches both sides.
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(dir.path().join("a"), "").unwrap();
    let files = ResolutionContext::new(biscuit_file::FileResolutionContext::new(dir.path()));
    let named = Values(HashMap::from([("x".to_string(), json!("a"))]), Some(files.clone()));
    assert_eq!(evaluate(&expr("file_exists(x)"), &named).unwrap(), json!(true));
    for guard in GUARDS {
        assert_sound(&declared, guard, &files);
    }
    // Where a string is declared, `file_exists(x)` holding leaves nothing
    // else: the evaluator answers false for every non-text value.
    let site = TypeScope::unguarded(&declared);
    if type_at(&site, "x").admits(&json!("a")) {
        let exists = type_at(&site.assume(&expr("file_exists(x)"), Polarity::True), "x");
        for other in [Value::Null, json!(4), json!(true), json!(["a"]), json!({ "k": "v" })] {
            assert!(!exists.admits(&other), "{name}: the true side ({}) admits {other}", shape_text(&exists));
        }
    }
}

/// One test per declaration, so nextest spreads the rows across cores.
macro_rules! differential {
    ($($name:ident),* $(,)?) => {
        mod every_guard_side_admits_every_value_the_evaluator_sends_there {
            $(
                #[test]
                fn $name() {
                    super::assert_differential(stringify!($name));
                }
            )*
        }

        #[test]
        fn every_differential_declaration_has_a_test() {
            let tested = [$(stringify!($name)),*];
            let declared: Vec<&str> = DIFFERENTIAL_SCHEMAS.iter().map(|(name, _)| *name).collect();
            assert_eq!(tested.as_slice(), declared.as_slice());
        }
    };
}

differential!(
    optional_strings,
    required_string_optional_number,
    scalar_unions,
    enums,
    booleans,
    numbers,
    literals,
    containers,
    container_unions,
    null_other_side,
    large_integers,
    undeclared,
);

#[test]
fn two_nulls_are_equal() {
    let declared = DeclaredTypes::new(None);
    let lookup = Values(HashMap::new(), None);
    assert_eq!(evaluate(&expr("x == y"), &lookup).unwrap(), json!(true));
    assert_eq!(evaluate(&expr("x != y"), &lookup).unwrap(), json!(false));
    // So the sides that two nulls reach admit them, and neither proves
    // presence.
    let site = TypeScope::unguarded(&declared);
    for (guard, polarity) in [("x == y", Polarity::True), ("x != y", Polarity::False)] {
        let scope = site.assume(&expr(guard), polarity);
        for subject in ["x", "y"] {
            assert!(type_at(&scope, subject).admits(&Value::Null), "{guard} {polarity:?} {subject}");
        }
    }
}

// ── joins and invalidation ──────────────────────────────────────────────

fn writes(paths: &[&str]) -> WriteSet {
    WriteSet::Paths(paths.iter().map(|path| Subject::document_path(path).unwrap()).collect())
}

#[test]
fn a_write_invalidates_its_aliases_ancestors_and_descendants_only() {
    let declared = DeclaredTypes::new(None);
    let scope = TypeScope::unguarded(&declared).assume(&expr("x && fm.x.deep && y.a && y.b && z"), Polarity::True);
    let after = scope.invalidate(&writes(&["x"]));
    assert!(!type_at(&after, "fm.x").proves_present());
    assert!(!type_at(&after, "x").proves_present());
    assert!(!type_at(&after, "x.deep").proves_present());
    assert!(type_at(&after, "z").proves_present());
    let nested = scope.invalidate(&writes(&["y.a.inner"]));
    assert!(!type_at(&nested, "y.a").proves_present(), "an ancestor of the destination");
    assert!(type_at(&nested, "y.b").proves_present(), "a sibling is untouched");
    let everything = scope.invalidate(&WriteSet::AllDocument);
    assert!(["x", "y.a", "z"].iter().all(|subject| !type_at(&everything, subject).proves_present()));
    // A guard after the write computes its facts again.
    let again = after.assume(&expr("x"), Polarity::True);
    assert!(type_at(&again, "x").proves_present());
}

#[test]
fn a_join_restores_only_incoming_facts_no_branch_could_have_written() {
    let declared = DeclaredTypes::new(None);
    let incoming = TypeScope::unguarded(&declared).assume(&expr("x && y"), Polarity::True);
    let condition = expr("c");
    // then: an inner conditional writes `x` on one of its routes.
    let then = incoming.assume(&condition, Polarity::True);
    let inner_write = then.assume(&expr("d"), Polarity::True).invalidate(&writes(&["x"]));
    let inner_skip = then.assume(&expr("d"), Polarity::False);
    let then_end = TypeScope::join(&inner_write, &inner_skip);
    let otherwise = incoming.assume(&condition, Polarity::False);
    let after = TypeScope::join(&then_end, &otherwise);
    assert!(!type_at(&after, "x").proves_present(), "a possible write survives the inner join");
    assert!(type_at(&after, "y").proves_present(), "an untouched incoming fact is restored");
    assert!(!type_at(&after, "c").proves_present(), "a branch's own facts end with it");
    assert!(!type_at(&after, "d").proves_present());
}

#[test]
fn a_join_keeps_what_both_branches_learned_and_drops_one_sided_facts() {
    let schema = schema("target: [string, number, \"null\"]\n");
    let declared = DeclaredTypes::new(Some(&schema));
    let site = TypeScope::unguarded(&declared);
    let strings = site.assume(&expr("is_string(target)"), Polarity::True);
    let numbers = site.assume(&expr("is_number(target)"), Polarity::True);
    let joined = TypeScope::join(&strings, &numbers);
    assert_eq!(shape_text(&type_at(&joined, "target")), "number | string");
    assert!(type_at(&joined, "target").guard_presence().is_some());
    let one_sided = TypeScope::join(&strings, &site);
    assert_eq!(shape_text(&type_at(&one_sided, "target")), "number | string | null");
    assert!(type_at(&one_sided, "target").guard_presence().is_none());
}

#[test]
fn a_static_destination_writes_its_path_and_any_other_destination_writes_everything() {
    assert_eq!(WriteSet::destination("x"), writes(&["x"]));
    assert_eq!(WriteSet::destination("fm.x.y"), writes(&["x.y"]));
    for dynamic in ["{{ key }}", "items[0]", "$(name)", "", "a..b"] {
        assert_eq!(WriteSet::destination(dynamic), WriteSet::AllDocument, "{dynamic}");
    }
    let declared = DeclaredTypes::new(None);
    let scope = TypeScope::unguarded(&declared).assume(&expr("x && ctx.branch"), Polarity::True);
    let after = scope.invalidate(&WriteSet::destination("{{ key }}"));
    assert!(!type_at(&after, "x").proves_present());
    assert!(type_at(&after, "ctx.branch").proves_present(), "a document write leaves namespaces alone");
}
