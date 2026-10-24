//! Acceptance 8 of `2026-10-16-type-narrowing` for the `darkmatter` library:
//! static presence and type decisions read the shared `TypeScope`. A retired
//! or competing classifier, a static null decision at a named decision site,
//! and an unreviewed `TypeScope::unguarded` construction fail; runtime value
//! checks pass. The engine and its rules are documented in
//! `tests/common/type_scope_guard.rs`.

// `glob_implementation_guard.rs` loads the same stateless file.
#[allow(clippy::duplicate_mod)]
#[path = "../../../cli/tests/common/source_scan.rs"]
mod source_scan;
#[path = "../common/type_scope_guard.rs"]
mod engine;

use engine::{DecisionSite, NullRead, Owned, Policy, Unguarded, snippets, violations};

const UNGUARDED: &[Unguarded] = &[
    Unguarded {
        path: "markdown/compose/directive_targets.rs",
        item: "classify_target_nullability",
        count: 1,
        reason: "the public before-any-guard nullability: declaration, descriptor, and static value alone",
    },
    Unguarded {
        path: "markdown/compose/directive_targets.rs",
        item: "analyze_directive_targets",
        count: 2,
        reason: "a target's before-guard nullability, and the base its enclosing `BodyGuards` narrow (`scope_at`)",
    },
    Unguarded {
        path: "markdown/compose/shell_expansion/template/grammar.rs",
        item: "Parser::take_hole",
        count: 1,
        reason: "the schema-free parse-time typing; `project_types` replaces it with the site's scope",
    },
    Unguarded {
        path: "markdown/compose/shell_expansion/site.rs",
        item: "SiteScope::site",
        count: 1,
        reason: "the compose-pass base that `guards::within` narrows with the same body guards discovery used",
    },
    Unguarded {
        path: "markdown/compose/preflight/collect.rs",
        item: "DiscoverySiteContext::build",
        count: 1,
        reason: "the discovery base that `guards::within` narrows with the site's enclosing body guards",
    },
    Unguarded {
        path: "markdown/compose/expression/absence.rs",
        item: "AbsenceScope::facts",
        count: 1,
        reason: "a read outside every branch has no branch facts; the warning policy needs only presence there",
    },
    Unguarded {
        path: "lifecycle/preflight/shell_walk.rs",
        item: "LifecycleShellSites::stamp",
        count: 1,
        reason: "each lifecycle event's walk starts from the declarations alone; `walk_scoped` adds its guards",
    },
    Unguarded {
        path: "markdown/compose/call_precheck.rs",
        item: "check_document",
        count: 1,
        reason: "the composition call check's declaration-only base, which body `BodyGuards::scope_at` narrows",
    },
    Unguarded {
        path: "lifecycle/validate.rs",
        item: "reject_incompatible_calls",
        count: 1,
        reason: "a category 5 verdict holds under every guard (narrowing only removes alternatives), so the rejection reads each event's declarations alone",
    },
];

/// The functions the type-narrowing inventory named as static decisions:
/// each still exists, reads the scope, and makes no null decision of its own.
const DECISION_SITES: &[DecisionSite] = &[
    DecisionSite {
        path: "markdown/compose/shell_expansion/template/candidate.rs",
        item: "CandidateScope::build",
        requires: &["type_of_expr", "guard_presence"],
        forbids: &[],
        null_read: Some(NullRead { count: 1, consult: "guard_of(" }),
    },
    DecisionSite {
        path: "markdown/compose/directive_targets.rs",
        item: "classify_target_nullability",
        requires: &["target_nullability("],
        forbids: &["Value::Null", "is_null", "evaluate"],
        null_read: None,
    },
    DecisionSite {
        path: "markdown/compose/directive_targets.rs",
        item: "target_nullability",
        requires: &["type_of_expr", "proves_present"],
        forbids: &["Value::Null", "is_null", "evaluate"],
        null_read: None,
    },
    DecisionSite {
        path: "markdown/compose/directive_targets.rs",
        item: "analyze_directive_targets",
        requires: &["scope_at(", "guarded_subjects"],
        forbids: &["Value::Null", "is_null", "evaluate"],
        null_read: None,
    },
    DecisionSite {
        path: "markdown/compose/expression/absence.rs",
        item: "walk_static",
        requires: &[".assume("],
        forbids: &["Value::Null", "is_null", "evaluate"],
        null_read: None,
    },
];

const RETIRED: &[&str] = &["narrowed_expression_paths", "collect_narrowed_paths", "RootGuard", "condition_guard"];

const SCOPE: &str = "markdown/compose/expression/scope/mod.rs";
const FACTS: &str = "markdown/compose/expression/scope/facts.rs";

const OWNED: &[Owned] = &[
    Owned {
        name: "assume",
        owners: &[SCOPE, FACTS, "markdown/compose/page_blocks/guards.rs"],
        why: "guard facts come from `TypeScope::assume` (a body guard delegates to it)",
    },
    Owned { name: "PREDICATE_FACTS", owners: &[FACTS], why: "the one predicate-fact table" },
    Owned { name: "PredicateFact", owners: &[FACTS], why: "the one predicate-fact table" },
    Owned { name: "TypeScope", owners: &[SCOPE], why: "the one type scope" },
    Owned { name: "Polarity", owners: &[SCOPE], why: "the one type scope" },
    Owned {
        name: "branch_scope",
        owners: &["markdown/compose/expression/absence.rs"],
        why: "the one expression-internal scope walk",
    },
];

const PREDICATE_LITERAL_OWNERS: &[&str] = &[
    // The registry: canonical names and aliases.
    "markdown/compose/expression/functions/",
    // The fact table, keyed by canonical name.
    FACTS,
    // Remote egress classification, not a presence decision.
    "markdown/compose/remote.rs",
];

fn policy() -> Policy<'static> {
    Policy {
        unguarded: UNGUARDED,
        retired: RETIRED,
        owned: OWNED,
        predicate_literal_owners: PREDICATE_LITERAL_OWNERS,
        decision_sites: DECISION_SITES,
    }
}

#[test]
fn static_presence_decisions_read_the_shared_type_scope() {
    // Spelled for CI's test-input index, which does not count the `#[path]`
    // includes above (`source-inputs` in Cargo.toml).
    let _ = include_str!("../../../cli/tests/common/source_scan.rs");
    let _ = include_str!("../common/type_scope_guard.rs");
    let sources = engine::read_sources(&biscuit_test_harness::manifest_dir!().join("src"));
    engine::assert_owned("darkmatter/lib/src", &sources, &policy());
}

// ── self-tests: the guard rejects bypasses and permits runtime checks ───

/// The decision sites, as minimal snippets that satisfy the policy, plus one
/// live site per allowlist entry. Each self-test edits one file.
fn baseline() -> Vec<(&'static str, String)> {
    let mut files: Vec<(&'static str, String)> = vec![
        (
            "markdown/compose/shell_expansion/template/candidate.rs",
            "impl CandidateScope<'_> { pub(crate) fn build(&self) -> R {
                let guard_of = |e| types.type_of_expr(&e)?.guard_presence().cloned();
                if matches!(evaluate(&expr), Some(Value::Null)) {
                    let Some(guard) = guard_of(expression) else { return Err(ShellExpansionError::MissingValue {}); };
                }
            } }"
            .to_string(),
        ),
        (
            "markdown/compose/directive_targets.rs",
            "pub fn classify_target_nullability() { target_nullability(&TypeScope::unguarded(&d), e) }
             fn target_nullability(s: &TypeScope<'_>) { let t = s.type_of_expr(e); t.proves_present() }
             pub fn analyze_directive_targets() {
                 let n = target_nullability(&TypeScope::unguarded(&d), &e);
                 let site = guards.scope_at(TypeScope::unguarded(&d), 0);
                 site.guarded_subjects();
             }"
            .to_string(),
        ),
        ("markdown/compose/expression/absence.rs", "impl AbsenceScope<'_> { pub(crate) fn facts(&self) { TypeScope::unguarded(&UNTYPED) } }
             fn walk_static() { scope.facts().assume(&c, Polarity::True); }"
            .to_string()),
        (
            "markdown/compose/call_precheck.rs",
            "pub(crate) fn check_document() { let site = TypeScope::unguarded(&declared); }".to_string(),
        ),
        (
            "lifecycle/validate.rs",
            "pub fn reject_incompatible_calls() { let site = TypeScope::unguarded(&event); }".to_string(),
        ),
    ];
    for entry in UNGUARDED {
        if files.iter().any(|(path, _)| *path == entry.path) {
            continue;
        }
        let (owner, function) = entry.item.split_once("::").expect("a method");
        files.push((entry.path, format!("impl {owner} {{ fn {function}(&self) {{ TypeScope::unguarded(&d); }} }}")));
    }
    files
}

fn check(edit: impl FnOnce(&mut Vec<(&'static str, String)>)) -> Vec<String> {
    let mut files = baseline();
    edit(&mut files);
    let borrowed: Vec<(&str, &str)> = files.iter().map(|(path, text)| (*path, text.as_str())).collect();
    violations("snippet", &snippets(&borrowed), &policy())
}

fn append(files: &mut [(&'static str, String)], path: &str, code: &str) {
    let file = files.iter_mut().find(|(candidate, _)| *candidate == path).expect("baseline file");
    file.1.push('\n');
    file.1.push_str(code);
}

#[test]
fn the_baseline_snippets_satisfy_the_policy() {
    assert_eq!(check(|_| {}), Vec::<String>::new());
}

#[test]
fn an_unreviewed_or_miscounted_unguarded_scope_is_rejected() {
    let unlisted = check(|files| {
        files.push(("markdown/compose/new_site.rs", "fn decide() { let s = TypeScope::unguarded(&d); }".to_string()))
    });
    assert!(unlisted.iter().any(|v| v.contains("new_site.rs:1: unreviewed `TypeScope::unguarded` in `decide`")), "{unlisted:#?}");

    // A fully qualified path and a method in an impl count the same way.
    let qualified = check(|files| {
        append(files, "markdown/compose/directive_targets.rs", "impl Probe { fn guess(&self) { crate::markdown::compose::expression::TypeScope::unguarded(&d); } }")
    });
    assert!(qualified.iter().any(|v| v.contains("unreviewed `TypeScope::unguarded` in `Probe::guess`")), "{qualified:#?}");

    let miscounted = check(|files| {
        let file = files.iter_mut().find(|(path, _)| *path == "markdown/compose/directive_targets.rs").unwrap();
        file.1 = file.1.replace("let site = guards.scope_at(TypeScope::unguarded(&d), 0);", "let site = guards.scope_at(base, 0);");
    });
    assert!(miscounted.iter().any(|v| v.contains("`analyze_directive_targets` constructs 1 unguarded scope(s); the allowlist says 2")), "{miscounted:#?}");

    let stale = check(|files| files.retain(|(path, _)| *path != "markdown/compose/shell_expansion/site.rs"));
    assert!(stale.iter().any(|v| v.contains("stale allowlist entry") && v.contains("SiteScope::site")), "{stale:#?}");
}

#[test]
fn a_comment_or_doc_example_is_not_a_construction_site() {
    let found = check(|files| {
        append(
            files,
            "markdown/compose/directive_targets.rs",
            "/// let site = TypeScope::unguarded(&declared);\n// TypeScope::unguarded(&x)\nconst S: &str = \"TypeScope::unguarded(&d)\";",
        )
    });
    assert_eq!(found, Vec::<String>::new());
}

#[test]
fn a_competing_static_classifier_is_rejected() {
    for (path, snippet, expected) in [
        (
            "markdown/compose/directive_targets.rs",
            "fn narrowed_expression_paths(e: &Expr) -> Vec<String> { collect_narrowed_paths(e) }",
            "`narrowed_expression_paths` is a retired static presence classifier",
        ),
        ("markdown/compose/expression/absence.rs", "struct RootGuard { root: String }", "`RootGuard` is a retired"),
        (
            "markdown/compose/preflight/collect.rs",
            "const PREDICATE_FACTS: &[&str] = &[];",
            "defines `PREDICATE_FACTS` outside its owner",
        ),
        (
            "markdown/compose/shell_expansion/site.rs",
            "fn assume(guard: &Expr) -> Vec<String> { vec![] }",
            "defines `assume` outside its owner",
        ),
        (
            "markdown/compose/preflight/collect.rs",
            "fn proves(name: &str) -> bool { matches!(name, \"is_string\" | \"fileExists\") }",
            "presence-predicate name \"is_string\"",
        ),
    ] {
        let found = check(|files| append(files, path, snippet));
        assert!(found.iter().any(|v| v.contains(expected)), "{snippet}: {found:#?}");
    }
}

#[test]
fn a_static_null_decision_at_a_named_decision_site_is_rejected() {
    // `classify_target_nullability` deciding from the value instead of the scope.
    let evaluated = check(|files| {
        let file = files.iter_mut().find(|(path, _)| *path == "markdown/compose/directive_targets.rs").unwrap();
        file.1 = file.1.replace(
            "let t = s.type_of_expr(e); t.proves_present()",
            "if evaluate(e) == Some(Value::Null) { return Nullable }",
        );
    });
    for expected in ["`target_nullability` reads `Value::Null`", "`target_nullability` no longer reads `type_of_expr`"] {
        assert!(evaluated.iter().any(|v| v.contains(expected)), "{expected}: {evaluated:#?}");
    }

    // A second null read in `CandidateScope::build` that fails without asking the guard.
    let unguarded_null = check(|files| {
        let file = files.iter_mut().find(|(path, _)| path.ends_with("candidate.rs")).unwrap();
        file.1 = file.1.replace(
            "if matches!(evaluate(&expr), Some(Value::Null)) {",
            "if value == Value::Null { return Err(ShellExpansionError::MissingValue {}); }\n if matches!(evaluate(&expr), Some(Value::Null)) {",
        );
    });
    for expected in ["has 2 `Value::Null` read(s), expected 1", "decides without consulting `guard_of(`"] {
        assert!(unguarded_null.iter().any(|v| v.contains(expected)), "{expected}: {unguarded_null:#?}");
    }

    let walk = check(|files| append(files, "markdown/compose/expression/absence.rs", "fn walk_static_extra() {}"));
    assert_eq!(walk, Vec::<String>::new(), "a similarly named function is not the decision site");
    let gone = check(|files| {
        let file = files.iter_mut().find(|(path, _)| path.ends_with("absence.rs")).unwrap();
        file.1 = file.1.replace("fn walk_static()", "fn walk_reads()");
    });
    assert!(gone.iter().any(|v| v.contains("decision site `walk_static` is gone")), "{gone:#?}");
}

/// Runtime validation reads values, not static facts: a hole filled with
/// `null`, the `is_null` and `file_exists` handlers, and the evaluator's
/// missing-value observation all pass.
#[test]
fn runtime_null_validation_is_permitted() {
    let found = check(|files| {
        files.push((
            "markdown/compose/shell_expansion/template/fill.rs",
            "fn resolve_hole(value: Value) -> Result<String, HoleError> { match value { Value::Null => Err(HoleError::Missing), _ => Ok(s) } }"
                .to_string(),
        ));
        files.push((
            "markdown/compose/expression/functions/mod.rs",
            "pub(crate) fn is_null(args: &[Value]) -> Value { Value::Bool(args[0].is_null()) }
             const PREDICATES: &[&str] = &[\"is_null\", \"file_exists\"];
             fn file_exists_fn(v: &Value) -> bool { if v.is_null() { return false; } true }"
                .to_string(),
        ));
        files.push((
            "markdown/compose/expression/evaluator.rs",
            "fn observe_missing(&self, path: &str, value: &Value) { if value.is_null() { self.missing.push(path) } }"
                .to_string(),
        ));
    });
    assert_eq!(found, Vec::<String>::new());
}
