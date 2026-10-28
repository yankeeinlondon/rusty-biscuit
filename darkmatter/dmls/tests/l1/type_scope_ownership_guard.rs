//! Acceptance 8 of `2026-10-16-type-narrowing` for `dmls`: the editor reads
//! guard-narrowed types from Darkmatter's one `TypeScope` and keeps no guard
//! algebra of its own. The engine is documented in
//! `darkmatter/lib/tests/common/type_scope_guard.rs`.

#[allow(clippy::duplicate_mod)]
#[path = "../../../cli/tests/common/source_scan.rs"]
mod source_scan;
#[path = "../../../lib/tests/common/type_scope_guard.rs"]
mod engine;

use engine::{DecisionSite, Owned, Policy, Unguarded, snippets, violations};

const UNGUARDED: &[Unguarded] = &[
    Unguarded {
        path: "providers/dsl.rs",
        item: "with_site_scope",
        count: 1,
        reason: "the one editor site scope: a body offset narrows it by its enclosing guards and collection regions (`BodyScopes`), a frontmatter site has none",
    },
    Unguarded {
        path: "providers/dsl.rs",
        item: "expression_diagnostics",
        count: 1,
        reason: "the body call check's base scope: each call narrows it by its enclosing guards and collection regions (`BodyScopes`) before checking",
    },
    Unguarded {
        path: "providers/collections.rs",
        item: "diagnostics",
        count: 1,
        reason: "the collection analysis's base scope: the library's `analyze_collection_blocks` and `BodyScopes::expression_sites` derive every region and callback scope from it",
    },
    Unguarded {
        path: "diagnostics/frontmatter.rs",
        item: "FrontmatterCalls::new",
        count: 1,
        reason: "the frontmatter call check: no guard applies, since a lifecycle or task `when` is not applied to the actions it selects (composition checks them unguarded too)",
    },
];

const DECISION_SITES: &[DecisionSite] = &[
    DecisionSite {
        path: "providers/dsl.rs",
        item: "with_site_scope",
        requires: &["collections::scopes", "scope_at("],
        forbids: &[],
        null_read: None,
    },
    DecisionSite {
        path: "providers/dsl.rs",
        item: "expression_diagnostics",
        requires: &["collections::scopes", "scope_at("],
        forbids: &[],
        null_read: None,
    },
    DecisionSite {
        path: "providers/dsl.rs",
        item: "transclusion_diagnostics",
        requires: &["analyze_directive_targets", "is_narrowed"],
        forbids: &["Value::Null", "is_null"],
        null_read: None,
    },
    DecisionSite {
        path: "overlay/expressions.rs",
        item: "local_type",
        requires: &["scoped_type("],
        forbids: &[],
        null_read: None,
    },
    DecisionSite {
        path: "overlay/expressions.rs",
        item: "scoped_type",
        requires: &["branch_scope("],
        forbids: &[],
        null_read: None,
    },
];

const LIBRARY: &str = "DMLS calls the library's scope; it defines no guard algebra";

/// Names the library owns. DMLS may define none of them.
const OWNED: &[Owned] = &[
    Owned { name: "assume", owners: &[], why: LIBRARY },
    Owned { name: "branch_scope", owners: &[], why: LIBRARY },
    Owned { name: "TypeScope", owners: &[], why: LIBRARY },
    Owned { name: "Polarity", owners: &[], why: LIBRARY },
    Owned { name: "BodyGuards", owners: &[], why: LIBRARY },
    Owned { name: "DeclaredTypes", owners: &[], why: LIBRARY },
    Owned { name: "NarrowingFact", owners: &[], why: LIBRARY },
    Owned { name: "FactKind", owners: &[], why: LIBRARY },
];

fn policy() -> Policy<'static> {
    Policy {
        unguarded: UNGUARDED,
        retired: &[
            "narrowed_expression_paths",
            "collect_narrowed_paths",
            "RootGuard",
            "condition_guard",
            "PREDICATE_FACTS",
            "PredicateFact",
        ],
        owned: OWNED,
        predicate_literal_owners: &[],
        decision_sites: DECISION_SITES,
    }
}

#[test]
fn dmls_reads_types_only_through_the_library_scope() {
    // Spelled for CI's test-input index, which does not count the `#[path]`
    // includes above (`source-inputs` in Cargo.toml).
    let _ = include_str!("../../../cli/tests/common/source_scan.rs");
    let _ = include_str!("../../../lib/tests/common/type_scope_guard.rs");
    let sources = engine::read_sources(&biscuit_test_harness::manifest_dir!().join("src"));
    engine::assert_owned("darkmatter/dmls/src", &sources, &policy());
}

/// The rules DMLS adds to the library's (whose self-tests cover the engine):
/// its own guard algebra, an unreviewed site scope, and a predicate table.
#[test]
fn a_dmls_guard_algebra_or_unreviewed_scope_is_rejected() {
    let baseline = [
        (
            "providers/dsl.rs",
            "fn with_site_scope() { let u = TypeScope::unguarded(&d); collections::scopes(t).scope_at(u, 0) }
             fn expression_diagnostics() { let u = TypeScope::unguarded(&d); collections::scopes(t).scope_at(u, 0) }
             fn transclusion_diagnostics() { analyze_directive_targets(t); a.is_narrowed(); }",
        ),
        ("overlay/expressions.rs", "pub fn local_type() { scoped_type(e, n, site) }\n             fn scoped_type() { branch_scope(e, s, site) }"),
        ("providers/collections.rs", "fn diagnostics() { TypeScope::unguarded(d) }"),
        (
            "diagnostics/frontmatter.rs",
            "impl FrontmatterCalls { fn new() -> Self { Self { site: TypeScope::unguarded(d) } } }",
        ),
    ];
    assert_eq!(violations("snippet", &snippets(&baseline), &policy()), Vec::<String>::new());

    for (snippet, expected) in [
        ("fn assume(guard: &Expr, side: bool) -> Facts { Facts::default() }", "defines `assume` outside its owner"),
        ("enum Polarity { True, False }", "defines `Polarity` outside its owner"),
        ("const GUARDS: &[&str] = &[\"isNull\", \"file_exists\"];", "presence-predicate name \"isNull\""),
        ("fn hover_scope() { TypeScope::unguarded(&d) }", "unreviewed `TypeScope::unguarded` in `hover_scope`"),
    ] {
        let mut files = baseline.map(|(path, text)| (path, text.to_string()));
        files[1].1.push('\n');
        files[1].1.push_str(snippet);
        let borrowed: Vec<(&str, &str)> = files.iter().map(|(path, text)| (*path, text.as_str())).collect();
        let found = violations("snippet", &snippets(&borrowed), &policy());
        assert!(found.iter().any(|v| v.contains(expected)), "{snippet}: {found:#?}");
    }
}
