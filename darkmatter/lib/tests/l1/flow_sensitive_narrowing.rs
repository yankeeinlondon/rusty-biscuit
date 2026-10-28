//! Flow-sensitive narrowing at the library's public API: registered
//! narrowing facts (true-iff, true-implies, false-implies), recursive arms
//! classified by their definition, short-circuit and compound guards,
//! impossible branches, `::block` guards through `BodyGuards` (the scope
//! composition and DMLS both build), lifecycle fact lifetime, and the proof
//! that narrowing changes no stored value.
//!
//! Every type is read through `TypeScope` and every category through the
//! design-time call check; handler-entry counters prove no predicate runs.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use darkmatter::lifecycle::{LifecycleSurfaceCalls, md_host};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::expression::call_binding::{FactKind, FactType, FunctionRegistration, RegistrationError};
use darkmatter::markdown::compose::expression::function_test_hooks::{self as hooks, TestFunctionGuard};
use darkmatter::markdown::compose::expression::{
    ArgumentCategory, ArgumentType, CallCheck, DeclaredTypes, Polarity, Shape, TypeScope, check_expression_calls, parse,
    parse_spanned,
};
use darkmatter::markdown::compose::page_blocks::BodyGuards;
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions, ComposeSource};
use darkmatter::markdown::schemas::simplified::{PropertyAtom, PropertyDef, TypeExpr, grammar::parse_type_definition};
use darkmatter::markdown::schemas::{DarkmatterSchemas, EffectiveSchema, SimplifiedSchema, parse_yaml_schema};
use serde_json::json;
use tempfile::TempDir;

use ArgumentCategory::{Compatible, Incompatible, NullableMatch, Uncertain, Unknown};

fn schema(yaml: &str) -> SimplifiedSchema {
    parse_yaml_schema(&serde_yaml_ng::from_str(yaml).expect("YAML parses")).expect("schema parses")
}

fn text(shape: &Shape) -> String {
    ArgumentType::Typed(shape.clone()).to_string()
}

/// The type `path` reads on `polarity`'s side of `guard`.
fn narrowed(declared: &DeclaredTypes<'_>, guard: &str, polarity: Polarity, path: &str) -> String {
    let scope = TypeScope::unguarded(declared).assume(&parse(guard).expect("guard parses"), polarity);
    text(scope.type_of_expr(&parse(path).expect("path parses")).expect("a stable subject").shape())
}

/// Every call check in `expression` at an unguarded site.
fn checks(declared: &DeclaredTypes<'_>, expression: &str) -> Vec<CallCheck> {
    check_expression_calls(&parse_spanned(expression).expect("parses"), &TypeScope::unguarded(declared))
}

/// The category of the call to `function` in `expression`.
#[track_caller]
fn category_of(declared: &DeclaredTypes<'_>, expression: &str, function: &str) -> ArgumentCategory {
    checks(declared, expression)
        .iter()
        .find(|check| check.function == function)
        .unwrap_or_else(|| panic!("{function} is checked in {expression}"))
        .category()
}

/// The `narrows(...)` clause text declaring `fact` on `value`.
fn clause(kind: FactKind, ty: &FactType) -> String {
    let kind = match kind {
        FactKind::TrueImplies => "true-implies",
        FactKind::TrueIff => "true-iff",
        FactKind::FalseImplies => "false-implies",
    };
    match ty {
        FactType::NonNull => format!("narrows(value; {kind})"),
        FactType::Type(text) => format!("narrows(value: {text}; {kind})"),
    }
}

/// A one-argument predicate counting its entries, declaring `fact` in its
/// signature.
fn predicate(name: &str, fact: Option<(FactKind, FactType)>) -> TestFunctionGuard {
    let narrows = fact.map(|(kind, ty)| format!("; {}", clause(kind, &ty))).unwrap_or_default();
    let signature = format!("function(parameters(value: unknown | null); returns(boolean){narrows})");
    let builder = FunctionRegistration::builder(name, signature);
    hooks::register(builder.build().expect("the predicate registers"), Arc::new(|_| Ok(json!(true))))
}

fn ty(text: &'static str) -> FactType {
    FactType::Type(text.into())
}

fn write(dir: &Path, name: &str, content: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, content).expect("write fixture");
    path
}

fn markdown_at(path: &Path) -> Markdown {
    Markdown::try_from(path).expect("document loads").with_source(ComposeSource::File(path.to_path_buf()))
}

fn effective_at(dir: &Path, path: &Path) -> EffectiveSchema {
    let context = biscuit_file::FileResolutionContext::new(dir).with_repository_root(dir);
    DarkmatterSchemas::new(context).effective_for(&markdown_at(path)).unwrap().expect("an effective schema")
}

fn compose(dir: &Path, content: &str) -> Result<Markdown, darkmatter::markdown::MarkdownError> {
    let path = write(dir, "doc.md", content);
    let options = ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir, "")).with_source_file(path.clone());
    markdown_at(&path)
        .compose_with(&crate::request_support::request_at(dir, options))
        .map(|(composed, _)| composed)
}

const UNION: &str = "foo:\n  - string(required)\n  - number(required)\nflag: boolean(required)\nx: number\nready: boolean\nmixed:\n  - string(required)\n  - number(required)\n  - boolean(required)\n";

/// Facts come from the predicate's registration: built-ins, test functions,
/// and (through the same API) host functions narrow alike.
mod registered_facts {
    use super::*;

    /// Criterion 11: a true-implies fact narrows the true branch only, so
    /// `pred(foo) ? 0 : min(foo, 5)` keeps `foo`'s whole type; a true-iff
    /// fact of the same type narrows both. No predicate handler runs.
    #[test]
    fn a_true_implies_fact_narrows_only_its_true_side() {
        let schema = schema(UNION);
        let declared = DeclaredTypes::new(Some(&schema));
        hooks::reset_handler_entries();
        {
            let _implies = predicate("implies_number", Some((FactKind::TrueImplies, ty("number"))));
            assert_eq!(category_of(&declared, "implies_number(foo) ? min(foo, 5) : 0", "min"), Compatible);
            assert_eq!(category_of(&declared, "implies_number(foo) ? 0 : min(foo, 5)", "min"), Uncertain);
            assert_eq!(narrowed(&declared, "implies_number(foo)", Polarity::False, "foo"), "number | string");
            assert_eq!(hooks::handler_entries("implies_number"), 0);
        }
        let _iff = predicate("iff_string", Some((FactKind::TrueIff, ty("string"))));
        assert_eq!(category_of(&declared, "iff_string(foo) ? 0 : min(foo, 5)", "min"), Compatible);
        assert_eq!(narrowed(&declared, "iff_string(foo)", Polarity::True, "foo"), "string");
        // The built-in true-iff predicate behaves the same way (criterion 9).
        assert_eq!(category_of(&declared, "is_string(foo) ? 0 : min(foo, 5)", "min"), Compatible);
        assert_eq!(hooks::handler_entries("iff_string"), 0);
    }

    /// Criterion 11, shell side: a shell template hole under a `::block`
    /// guarded by a registered predicate is typed from the same narrowed
    /// scope, so it validates as the narrowed type.
    #[test]
    fn a_registered_fact_narrows_a_shell_template_hole() {
        use darkmatter::markdown::compose::shell_expansion::template::project_hole;
        let schema = schema(UNION);
        let declared = DeclaredTypes::new(Some(&schema));
        let _implies = predicate("implies_number", Some((FactKind::TrueImplies, ty("number"))));
        let body = "::block when=\"implies_number(foo)\"\n::shell echo {{ foo }}\n::end-block\n";
        let site = BodyGuards::scan(body).scope_at(TypeScope::unguarded(&declared), body.find("{{").unwrap());
        assert_eq!(project_hole("foo", &site).display_type(), "string (validated as number)");
        assert_ne!(
            project_hole("foo", &TypeScope::unguarded(&declared)).display_type(),
            "string (validated as number)",
            "the guard is what narrows the hole"
        );
    }

    /// Without a registered fact a predicate proves nothing on either side.
    #[test]
    fn a_predicate_without_a_fact_narrows_nothing() {
        let schema = schema(UNION);
        let declared = DeclaredTypes::new(Some(&schema));
        let _plain = predicate("plain", None);
        assert_eq!(narrowed(&declared, "plain(foo)", Polarity::True, "foo"), "number | string");
        assert_eq!(narrowed(&declared, "plain(x)", Polarity::True, "x"), "number | null");
    }

    /// A true-implies fact of a non-family type makes an argument suitable
    /// for that type in the guarded code: `unknown` becomes it (category 4 →
    /// 1), a member already within it stays, and a member of another family
    /// is removed. A false-implies non-null fact proves presence on the
    /// false side only.
    #[test]
    fn true_implies_and_false_implies_facts_use_their_type() {
        let schema = schema("source: unknown\nchoice:\n  - file(required)\n  - number(required)\nx: number\n");
        let declared = DeclaredTypes::new(Some(&schema));
        let _file = predicate("is_project_file", Some((FactKind::TrueImplies, ty("file"))));
        let _present = predicate("looks_empty", Some((FactKind::FalseImplies, FactType::NonNull)));
        assert_eq!(category_of(&declared, "frontmatter(source, 'title')", "frontmatter"), Unknown);
        assert_eq!(
            category_of(&declared, "is_project_file(source) && frontmatter(source, 'title')", "frontmatter"),
            Compatible
        );
        assert_eq!(narrowed(&declared, "is_project_file(source)", Polarity::True, "source"), "file");
        assert_eq!(narrowed(&declared, "is_project_file(choice)", Polarity::True, "choice"), "file");
        assert_eq!(narrowed(&declared, "is_project_file(source)", Polarity::False, "source"), "unknown");
        assert_eq!(narrowed(&declared, "looks_empty(x)", Polarity::False, "x"), "number");
        assert_eq!(narrowed(&declared, "looks_empty(x)", Polarity::True, "x"), "number | null");
    }

    /// Registration validates a declared fact before anything evaluates:
    /// the grammar rejects what is wrong with one clause, the shared
    /// compiler what is wrong across overloads or with a family.
    #[test]
    fn registration_rejects_invalid_facts() {
        let build = |returns: &str, parameter: &str, kind: FactKind, fact: &FactType| {
            let clause = clause(kind, fact).replacen("(value", &format!("({parameter}"), 1);
            let signature = format!("function(parameters(value: unknown | null); returns({returns}); {clause})");
            FunctionRegistration::builder("pred", signature).build()
        };
        assert!(matches!(
            build("number", "value", FactKind::TrueIff, &ty("number")),
            Err(RegistrationError::Grammar { .. })
        ));
        assert!(matches!(
            build("boolean", "other", FactKind::TrueImplies, &ty("number")),
            Err(RegistrationError::Grammar { .. })
        ));
        for not_a_family in ["file", "string(min(3))", "enum(a, b)"] {
            assert!(
                matches!(build("boolean", "value", FactKind::TrueIff, &ty(not_a_family)), Err(RegistrationError::InvalidFact { .. })),
                "{not_a_family}"
            );
            assert!(build("boolean", "value", FactKind::TrueImplies, &ty(not_a_family)).is_ok(), "{not_a_family}");
        }
        assert!(matches!(
            build("boolean", "value", FactKind::TrueImplies, &ty("string | number")),
            Err(RegistrationError::Grammar { .. })
        ));
        for family in ["string", "number", "boolean", "object", "unknown[]", "null"] {
            let fact = build("boolean", "value", FactKind::TrueIff, &ty(family)).expect(family);
            let fact = fact.narrowing_fact().expect("registered");
            assert_eq!((fact.parameter(), fact.index(), fact.kind()), ("value", 0, FactKind::TrueIff));
        }
    }

    /// Every overload must declare the same fact; a programmatic overload
    /// that skipped the grammar is checked by the compiler, not skipped.
    #[test]
    fn registration_rejects_overloads_that_disagree_on_the_fact() {
        let one = "function(parameters(value: unknown | null); returns(boolean); narrows(value: string; true-iff))";
        let build = |second: &str| FunctionRegistration::builder("pred", format!("{one} | {second}")).build();
        let invalid = |result: Result<FunctionRegistration, RegistrationError>| {
            matches!(result, Err(RegistrationError::InvalidFact { .. }))
        };
        assert!(
            build("function(parameters(value: unknown | null, extra: string); returns(boolean); narrows(value: string; true-iff))")
                .is_ok(),
            "control: both overloads declare the same fact"
        );
        assert!(invalid(build("function(parameters(value: unknown | null, extra: string); returns(boolean))")), "absent");
        assert!(invalid(build(
            "function(parameters(value: unknown | null, extra: string); returns(boolean); narrows(value: number; true-iff))"
        )), "different target");
        assert!(invalid(build(
            "function(parameters(value: unknown | null, extra: string); returns(boolean); narrows(value: string; true-implies))"
        )), "different kind");
        assert!(invalid(build(
            "function(parameters(extra: string, value: unknown | null); returns(boolean); narrows(value: string; true-iff))"
        )), "different position");
        assert!(invalid(build(
            "function(parameters(value: unknown | null, extra: string); returns(boolean); narrows(extra; false-implies))"
        )), "different parameter and kind");

        // Only the later overload lacks the parameter the clause names.
        let parse = |text: &str| match parse_type_definition("pred", text).expect("parses") {
            PropertyDef::Single(PropertyAtom { ty: TypeExpr::Function(function), .. }) => *function,
            other => panic!("one function, found {other:?}"),
        };
        let first = parse(one);
        let mut later = parse("function(parameters(other: unknown | null, extra: string); returns(boolean); narrows(other; false-implies))");
        later.narrows.as_mut().expect("clause").parameter = "value".into();
        let result = FunctionRegistration::builder_from_types("pred", vec![first, later]).build();
        assert!(matches!(result, Err(RegistrationError::InvalidFact { .. })), "{:?}", result.map(|_| ()));
    }
}

/// The catalog's own `narrows(...)` declarations (the shipped built-ins),
/// read through the scope: what each branch proves about the original value.
mod declared_builtin_facts {
    use super::*;

    /// `file_exists` and `is_file_type` both make an `unknown` subject a
    /// `file` on the true side. Existence is not `true-iff`: the false side
    /// excludes nothing, because a valid path to a missing file is still a
    /// possible file value.
    #[test]
    fn the_file_predicates_narrow_unknown_to_file_on_the_true_side_only() {
        let schema = schema("source: unknown\nchoice:\n  - file(required)\n  - number(required)\n");
        let declared = DeclaredTypes::new(Some(&schema));
        hooks::reset_handler_entries();
        for predicate in ["is_file_type", "file_exists"] {
            let guard = format!("{predicate}(source)");
            assert_eq!(narrowed(&declared, &guard, Polarity::True, "source"), "file", "{predicate}");
            assert_eq!(narrowed(&declared, &guard, Polarity::False, "source"), "unknown", "{predicate}");
            let guard = format!("{predicate}(choice)");
            assert_eq!(narrowed(&declared, &guard, Polarity::True, "choice"), "file", "{predicate}");
            assert_eq!(narrowed(&declared, &guard, Polarity::False, "choice"), "file | number", "{predicate}");
        }
        assert_eq!(category_of(&declared, "frontmatter(source, 'title')", "frontmatter"), Unknown);
        for predicate in ["is_file_type", "file_exists"] {
            let call = format!("{predicate}(source) && frontmatter(source, 'title')");
            assert_eq!(category_of(&declared, &call, "frontmatter"), Compatible, "{predicate}");
        }
        assert_eq!(hooks::handler_entries("file_exists"), 0);
        assert_eq!(hooks::handler_entries("is_file_type"), 0);
    }

    /// The migrated `true-iff` primitives keep both branch shapes, and
    /// `is_empty` proves only presence on its false side.
    #[test]
    fn the_migrated_primitives_keep_their_branch_shapes() {
        let schema = schema(UNION);
        let declared = DeclaredTypes::new(Some(&schema));
        assert_eq!(narrowed(&declared, "is_string(foo)", Polarity::True, "foo"), "string");
        assert_eq!(narrowed(&declared, "is_string(foo)", Polarity::False, "foo"), "number");
        assert_eq!(narrowed(&declared, "is_number(foo)", Polarity::True, "foo"), "number");
        assert_eq!(narrowed(&declared, "is_number(foo)", Polarity::False, "foo"), "string");
        assert_eq!(narrowed(&declared, "is_null(x)", Polarity::True, "x"), "null");
        assert_eq!(narrowed(&declared, "is_null(x)", Polarity::False, "x"), "number");
        assert_eq!(narrowed(&declared, "is_empty(x)", Polarity::False, "x"), "number");
        assert_eq!(narrowed(&declared, "is_empty(x)", Polarity::True, "x"), "number | null");
        assert_eq!(narrowed(&declared, "is_empty(foo)", Polarity::False, "foo"), "number | string");
        let objects = schema_with_object_and_array();
        let declared = DeclaredTypes::new(Some(&objects));
        assert_eq!(narrowed(&declared, "is_array(v)", Polarity::True, "v"), "unknown[]");
        assert_eq!(narrowed(&declared, "is_array(v)", Polarity::False, "v"), "string | {a: string}");
        assert_eq!(narrowed(&declared, "is_object(v)", Polarity::False, "v"), "string | unknown[]");
    }

    fn schema_with_object_and_array() -> SimplifiedSchema {
        schema("v:\n  - string(required)\n  - unknown[](required)\n  - \"{ a: string }\"\n")
    }

    /// A function's aliases resolve to the same registration, so they narrow
    /// identically.
    #[test]
    fn aliases_narrow_like_the_canonical_name() {
        let schema = schema("source: unknown\n");
        let union = super::schema(UNION);
        let declared = DeclaredTypes::new(Some(&schema));
        for alias in ["fileExists", "fileexists"] {
            assert_eq!(narrowed(&declared, &format!("{alias}(source)"), Polarity::True, "source"), "file", "{alias}");
        }
        let declared = DeclaredTypes::new(Some(&union));
        assert_eq!(narrowed(&declared, "isstring(foo)", Polarity::True, "foo"), "string");
        assert_eq!(narrowed(&declared, "isstring(foo)", Polarity::False, "foo"), "number");
    }

    /// When the argument carrying the fact is omitted, the call proves
    /// nothing about any other argument.
    #[test]
    fn an_omitted_optional_fact_argument_narrows_nothing() {
        let schema = schema(UNION);
        let declared = DeclaredTypes::new(Some(&schema));
        let signature = "function(parameters(first: unknown | null, second?: unknown | null); returns(boolean); narrows(second: string; true-iff))";
        let builder = FunctionRegistration::builder("optional_fact", signature);
        let _guard = hooks::register(builder.build().expect("registers"), Arc::new(|_| Ok(json!(true))));
        assert_eq!(narrowed(&declared, "optional_fact(foo)", Polarity::True, "foo"), "number | string");
        assert_eq!(narrowed(&declared, "optional_fact(foo)", Polarity::False, "foo"), "number | string");
        assert_eq!(narrowed(&declared, "optional_fact(1, foo)", Polarity::True, "foo"), "string");
        assert_eq!(narrowed(&declared, "optional_fact(1, foo)", Polarity::False, "foo"), "number");
    }
}

/// A recursive arm is classified by its definition body, one level, and
/// kept as its named reference.
mod recursive_arms {
    use super::*;

    #[test]
    fn a_recursive_arm_is_classified_by_its_definition() {
        let dir = TempDir::new().unwrap();
        write(
            dir.path(),
            "types.yaml",
            "kind: schema\ntypes:\n    node:\n        label: string(required)\n        children: node[](required)\n",
        );
        let path = write(
            dir.path(),
            "doc.md",
            "---\n$schema:\n  x:\n    - number(required)\n    - node(required)@./types.yaml\n---\nBody\n",
        );
        let effective = effective_at(dir.path(), &path);
        let declared = DeclaredTypes::for_schema(Some(&effective));
        assert_eq!(narrowed(&declared, "is_number(x)", Polarity::True, "x"), "number");
        assert_eq!(narrowed(&declared, "is_number(x)", Polarity::False, "x"), "node");
        assert_eq!(narrowed(&declared, "is_object(x)", Polarity::True, "x"), "node");
        assert_eq!(narrowed(&declared, "is_object(x)", Polarity::False, "x"), "number");
        assert_eq!(narrowed(&declared, "is_string(x)", Polarity::False, "x"), "node | number");
        assert_eq!(category_of(&declared, "is_number(x) ? min(x, 5) : 0", "min"), Compatible);
        // Without the definitions a reference keeps every family: the
        // delivered conservative result.
        let schema = schema("x:\n  - number(required)\n  - string(required)\n");
        assert_eq!(narrowed(&DeclaredTypes::new(Some(&schema)), "is_number(x)", Polarity::True, "x"), "number");
    }
}

/// Short-circuit and compound guards keep only facts valid on every route.
mod compound_guards {
    use super::*;

    /// Criteria 10 and 33.
    #[test]
    fn short_circuit_operands_and_ternaries_see_only_justified_facts() {
        let schema = schema(UNION);
        let declared = DeclaredTypes::new(Some(&schema));
        for (expression, expected) in [
            ("x ? min(x, 5) : 0", Compatible),
            ("x && min(x, 5)", Compatible),
            ("!x ? 0 : min(x, 5)", Compatible),
            ("min(x, 5)", NullableMatch),
            // A false truthiness branch does not prove `null` (0 is falsy).
            ("x ? 0 : min(x, 5)", NullableMatch),
            ("!x ? min(x, 5) : 0", NullableMatch),
            ("(is_number(foo)) ? min(foo, 5) : 0", Compatible),
            ("is_number(foo) || ready ? min(foo, 5) : 0", Uncertain),
            ("ready || is_number(foo) ? min(foo, 5) : 0", Uncertain),
            ("ready && is_number(foo) ? min(foo, 5) : 0", Compatible),
            // A guard on a call refines nothing about the call's arguments.
            ("is_number(trim(foo)) ? min(foo, 5) : 0", Uncertain),
            ("is_number(abs(x)) ? min(x, 5) : 0", NullableMatch),
        ] {
            assert_eq!(category_of(&declared, expression, "min"), expected, "{expression}");
        }
        assert_eq!(narrowed(&declared, "is_number(mixed) || is_string(mixed)", Polarity::True, "mixed"), "number | string");
        assert_eq!(narrowed(&declared, "is_number(mixed) || ready", Polarity::True, "mixed"), "boolean | number | string");
        assert_eq!(narrowed(&declared, "is_number(mixed) || is_string(mixed)", Polarity::False, "mixed"), "boolean");
        assert_eq!(narrowed(&declared, "!is_number(mixed)", Polarity::True, "mixed"), "boolean | string");
    }

    /// A contradicted subject keeps its incoming type, so a known-invalid
    /// call in a branch that can never run is still reported.
    #[test]
    fn an_impossible_branch_is_checked_with_the_incoming_type() {
        let schema = schema(UNION);
        let declared = DeclaredTypes::new(Some(&schema));
        assert_eq!(category_of(&declared, "is_number(flag) ? min(flag, 5) : 0", "min"), Incompatible);
        assert_eq!(narrowed(&declared, "is_number(flag)", Polarity::True, "flag"), "boolean");
    }
}

/// The same fixtures through the `::block` guards composition and DMLS
/// both build, and through composition itself.
mod body_guards {
    use super::*;

    /// The category of `min` in the body expression `expression`, at its
    /// site under the enclosing `::block` guards.
    fn body_category(declared: &DeclaredTypes<'_>, body: &str, expression: &str) -> ArgumentCategory {
        let offset = body.find(expression).expect("the expression is in the body");
        let site = BodyGuards::scan(body).scope_at(TypeScope::unguarded(declared), offset);
        check_expression_calls(&parse_spanned(expression).unwrap(), &site)
            .iter()
            .find(|check| check.function == "min")
            .expect("min is checked")
            .category()
    }

    /// Criterion 10: a `::block when="x"` body narrows `x`; nested blocks
    /// compose; the same call outside the block is category 2.
    #[test]
    fn a_block_guard_narrows_its_body() {
        let schema = schema(UNION);
        let declared = DeclaredTypes::new(Some(&schema));
        let body = "::block when=\"x\"\n{{ min(x, 5) }}\n::block when=\"is_number(foo)\"\n{{ min(foo, 5) }}\n::end-block\n::end-block\n{{ min(x, 6) }}\n";
        assert_eq!(body_category(&declared, body, "min(x, 5)"), Compatible);
        assert_eq!(body_category(&declared, body, "min(foo, 5)"), Compatible);
        assert_eq!(body_category(&declared, body, "min(x, 6)"), NullableMatch);
    }

    /// Criterion 9 at composition: the guarded calls compose and evaluate,
    /// and an impossible block or ternary branch holding a proven-bad call
    /// rejects the document before anything evaluates.
    #[test]
    fn composition_reads_the_same_narrowing() {
        let dir = TempDir::new().unwrap();
        let head = "---\n$schema:\n  foo:\n    - string(required)\n    - number(required)\n  flag: boolean(required)\nfoo: 4\nflag: true\n---\n";
        let composed = compose(
            dir.path(),
            &format!("{head}{{{{ is_number(foo) ? min(foo, 5) : 0 }}}} {{{{ is_string(foo) ? 0 : min(foo, 3) }}}}\n"),
        )
        .expect("the guarded calls compose");
        assert_eq!(composed.content().trim_end(), "4 3");
        for body in [
            "{{ is_number(flag) ? min(flag, 5) : 0 }}\n",
            "::block when=\"is_number(flag)\"\n{{ min(flag, 5) }}\n::end-block\n",
        ] {
            let error = compose(dir.path(), &format!("{head}{body}")).expect_err(body);
            let check = error.rejected_call().unwrap_or_else(|| panic!("{body}: {error}"));
            assert_eq!((check.function.as_str(), check.category()), ("min", Incompatible), "{body}");
        }
    }

    /// Criterion 12: analysis and evaluation of guarded expressions change no
    /// stored value. Every property is present, so schema-stage `null`
    /// materialization has nothing to do; the authored bytes, the parsed
    /// frontmatter, and the composed frontmatter keep every value.
    #[test]
    fn narrowing_never_changes_a_stored_value() {
        let dir = TempDir::new().unwrap();
        let content = "---\n$schema:\n  x: number\n  foo:\n    - string(required)\n    - number(required)\nx: 2\nfoo: pear\n---\n{{ x ? min(x, 5) : 0 }} {{ x && min(x, 5) }} {{ is_string(foo) ? 0 : min(foo, 5) }}\n::block when=\"is_string(foo) && x\"\n{{ foo }} {{ min(x, 1) }}\n::end-block\n";
        let path = write(dir.path(), "doc.md", content);
        let markdown = markdown_at(&path);
        let before = markdown.frontmatter().clone();
        let effective = effective_at(dir.path(), &path);
        let json_before = effective.json_schema.as_ref().clone();

        hooks::reset_handler_entries();
        let declared = DeclaredTypes::for_schema(Some(&effective));
        let body = markdown.content();
        let guards = BodyGuards::scan(body);
        for (offset, _) in body.match_indices("{{") {
            let end = offset + body[offset..].find("}}").unwrap();
            let site = guards.scope_at(TypeScope::unguarded(&declared), offset);
            let found = check_expression_calls(&parse_spanned(&body[offset + 2..end]).unwrap(), &site);
            assert!(found.iter().all(|check| check.category() == Compatible), "{found:?}");
        }
        assert_eq!(hooks::handler_entries("min"), 0, "analysis runs no handler");
        assert_eq!(hooks::handler_entries("is_string"), 0);
        assert_eq!(markdown.frontmatter(), &before, "analysis changes no value");
        assert_eq!(effective.json_schema.as_ref(), &json_before);

        let composed = compose(dir.path(), content).expect("composes");
        assert_eq!(composed.content().trim_end(), "2 true 0 pear 1");
        let data = composed.frontmatter().as_map();
        assert_eq!((data.get("x"), data.get("foo")), (Some(&json!(2)), Some(&json!("pear"))));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), content, "authored bytes are unchanged");
    }
}

/// Criterion 34: a lifecycle guard fact ends at a write and never crosses an
/// event; re-reading the stack recomputes it.
mod lifecycle_fact_lifetime {
    use super::*;

    const SCHEMA: &str = "$schema:\n  count: number\n  meta: \"{ count: number, other: string }\"\n  foo:\n    - string(required)\n    - number(required)\n  flag: boolean(required)\n";

    fn surfaces(dir: &Path, stacks: &str) -> Vec<LifecycleSurfaceCalls> {
        let frontmatter = format!("{SCHEMA}flag: true\nfoo: 1\n{stacks}");
        let path = write(dir, "doc.md", &format!("---\n{frontmatter}---\nBody\n"));
        let effective = effective_at(dir, &path);
        let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(&frontmatter).expect("YAML parses");
        let value: serde_json::Value = serde_json::to_value(yaml).expect("JSON");
        let config = md_host::parse_lifecycle(&value, &path, None).expect("the lifecycle parses");
        darkmatter::lifecycle::lifecycle_call_checks(&value, &config, md_host::registry(), Some(&effective))
    }

    /// `(property, category)` of every surface whose `min` call is not
    /// category 1, in walk order.
    fn shown(surfaces: &[LifecycleSurfaceCalls]) -> Vec<(String, ArgumentCategory)> {
        surfaces
            .iter()
            .flat_map(|surface| {
                surface.checks.iter().map(move |found| (surface.property.to_string(), found.check.category()))
            })
            .collect()
    }

    #[test]
    fn a_set_ends_the_guard_and_events_never_share_facts() {
        let dir = TempDir::new().unwrap();
        let stacks = "composed:\n  - when: count\n    then:\n      - stdout: \"{{ min(count, 5) }}\"\n      - set:\n          count: 7\n      - stdout: \"{{ min(count, 5) }}\"\n  - stdout: \"{{ min(count, 5) }}\"\nfinalize:\n  - stdout: \"{{ min(count, 5) }}\"\n";
        let found = surfaces(dir.path(), stacks);
        assert_eq!(
            shown(&found),
            [
                ("composed[0].then[2].stdout.text".to_string(), NullableMatch),
                ("composed[1].stdout.text".to_string(), NullableMatch),
                ("finalize[0].stdout.text".to_string(), NullableMatch),
            ]
        );
        // Every walked surface is reported with its authored location, so an
        // editor ranges the same verdict.
        let first = found.iter().find(|surface| surface.property.to_string() == "composed[0].then[0].stdout.text").unwrap();
        assert!(first.checks.is_empty(), "the guarded call is category 1");
        let location: Vec<String> = first.location.iter().map(|segment| format!("{segment:?}")).collect();
        assert_eq!(location, ["Key(\"composed\")", "Index(0)", "Key(\"then\")", "Index(0)", "Key(\"stdout\")"]);
        // Re-reading the stack recomputes every fact identically.
        assert_eq!(shown(&surfaces(dir.path(), stacks)), shown(&found));
    }

    /// A write to an ancestor or an alias ends the fact; a sibling write
    /// does not; a `fm` read and the bare read share the fact.
    #[test]
    fn overlapping_writes_end_facts_and_siblings_do_not() {
        let dir = TempDir::new().unwrap();
        let stacks = "composed:\n  - when: meta.count\n    then:\n      - set:\n          meta.other: x\n      - stdout: \"{{ min(fm.meta.count, 5) }}\"\n      - set:\n          meta:\n            count: 1\n      - stdout: \"{{ min(meta.count, 6) }}\"\n  - when: fm.count\n    then:\n      - stdout: \"{{ min(count, 7) }}\"\n      - set:\n          fm.count: 2\n      - stdout: \"{{ min(count, 8) }}\"\n";
        let found = surfaces(dir.path(), stacks);
        assert_eq!(
            shown(&found),
            [
                ("composed[0].then[3].stdout.text".to_string(), NullableMatch),
                ("composed[1].then[2].stdout.text".to_string(), NullableMatch),
            ]
        );
    }

    /// The composition-side rejection reads the guarded site too: a call
    /// a guard proves wrong is rejected, one it narrows is not, and an
    /// impossible guard keeps the incoming type.
    #[test]
    fn the_lifecycle_rejection_reads_each_site() {
        let dir = TempDir::new().unwrap();
        let narrowed_ok = "composed:\n  - when: is_number(foo)\n    then:\n      - stdout: \"{{ min(foo, 5) }}\"\n";
        assert!(shown(&surfaces(dir.path(), narrowed_ok)).is_empty());
        let impossible = "composed:\n  - when: is_number(flag)\n    then:\n      - stdout: \"{{ min(flag, 5) }}\"\n";
        assert_eq!(shown(&surfaces(dir.path(), impossible)), [("composed[0].then[0].stdout.text".to_string(), Incompatible)]);
        let unguarded = "composed:\n  - stdout: \"{{ min(foo, 5) }}\"\n";
        assert_eq!(shown(&surfaces(dir.path(), unguarded)), [("composed[0].stdout.text".to_string(), Uncertain)]);
    }
}
