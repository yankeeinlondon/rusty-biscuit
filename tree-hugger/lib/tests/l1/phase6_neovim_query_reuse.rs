//! Phase 6: Neovim Query Reuse
//!
//! Tests for query provenance, nvim-treesitter inventory, and predicate
//! compatibility.

use crate::language_cases::language_tests;
use tree_hugger::ProgrammingLanguage;
use tree_hugger::queries::{
    GrammarRef, QueryKind,
    compatibility::{
        CompatibilityRegistry, CompatibilityStatus, find_hook_predicates,
        find_unsupported_predicates,
    },
    inventory::{NvimQuerySuite, QueryInventory, suite_for_kind},
    provenance::{TranslationStatus, all_query_provenance, query_provenance},
    query_for,
};

// =============================================================================
// Query Compilation with Provenance
// =============================================================================

language_tests!(all_queries_compile_with_provenance => check_query_provenance);

/// Every supported language must have queries that compile, and every
/// query must carry a provenance record with a recorded upstream revision.
fn check_query_provenance(language: ProgrammingLanguage) {
    let grammar = language.tree_sitter_language();
    let grammar_ref = GrammarRef {
        language,
        grammar: &grammar,
        id: language.query_name(),
    };

    for kind in [
        QueryKind::Locals,
        QueryKind::Lint,
        QueryKind::References,
        QueryKind::Comments,
    ] {
        let query = query_for(grammar_ref, kind);
        assert!(query.is_ok(), "{language} {kind} query should compile");

        let prov = query_provenance(language, kind)
            .unwrap_or_else(|| panic!("{language} {kind} query has no provenance"));
        assert!(!prov.source_project.is_empty());
        assert!(!prov.local_path.is_empty());
        assert!(
            !prov.upstream_revision.is_empty(),
            "{language} {kind}: upstream_revision should not be empty"
        );
    }
}

// =============================================================================
// Capture Parity Snapshots
// =============================================================================

/// Verifies that the nvim-treesitter inventory contains expected captures for
/// representative languages and that reused captures are correctly flagged.
#[test]
fn capture_parity_for_rust() {
    let inventory = QueryInventory::new();
    let rust = inventory
        .for_language(ProgrammingLanguage::Rust)
        .expect("rust should be present");

    let locals = rust
        .suite_captures(NvimQuerySuite::Locals)
        .expect("locals should be present");

    // Key locals captures that Tree Hugger reuses
    assert!(
        locals
            .iter()
            .any(|c| c.name == "local.definition.function" && c.reused)
    );
    assert!(
        locals
            .iter()
            .any(|c| c.name == "local.definition.type" && c.reused)
    );
    assert!(
        locals
            .iter()
            .any(|c| c.name == "local.reference" && c.reused)
    );
    assert!(locals.iter().any(|c| c.name == "local.scope" && c.reused));

    // Highlights captures are inventoried but not reused
    let highlights = rust
        .suite_captures(NvimQuerySuite::Highlights)
        .expect("highlights should be present");
    assert!(highlights.iter().any(|c| c.name == "function" && !c.reused));
    assert!(highlights.iter().any(|c| c.name == "type" && !c.reused));
}

language_tests!(every_locals_definition_capture_is_inventoried => check_locals_inventory);

/// Every definition capture a locals query uses is inventoried as reused, so
/// the inventory changes together with the queries.
fn check_locals_inventory(language: ProgrammingLanguage) {
    let inventory = QueryInventory::new();
    let mut checked = 0;
    let grammar = language.tree_sitter_language();
    let grammar_ref = GrammarRef {
        language,
        grammar: &grammar,
        id: language.query_name(),
    };
    let query = query_for(grammar_ref, QueryKind::Locals).expect("locals query compiles");
    let locals = inventory
        .for_language(language)
        .and_then(|entry| entry.suite_captures(NvimQuerySuite::Locals))
        .expect("locals inventory");
    for name in query.capture_names() {
        if !name.starts_with("local.definition") || name.ends_with(".context") {
            continue;
        }
        assert!(
            locals.iter().any(|capture| capture.name == *name && capture.reused),
            "{language}: @{name} is used by the locals query but not inventoried as reused"
        );
        checked += 1;
    }

    assert!(checked >= crate::language_cases::corpus_minimums(language).captures, "{language}: no definition captures checked");
}

/// Perl's locals query is Tree Hugger's own work, compiles, and uses only
/// natively supported predicates.
#[test]
fn perl_locals_query_is_an_original() {
    let prov = query_provenance(ProgrammingLanguage::Perl, QueryKind::Locals)
        .expect("perl locals provenance");
    assert_eq!(prov.source_project, "tree-hugger");
    assert_eq!(prov.translation_status, TranslationStatus::Original);
    assert_ne!(prov.license, "Apache-2.0");

    let source = include_str!("../../queries/vendor/perl/locals.scm");
    assert!(find_unsupported_predicates(source).is_empty());
    assert!(find_hook_predicates(source).is_empty());
}

#[test]
fn capture_parity_for_javascript() {
    let inventory = QueryInventory::new();
    let js = inventory
        .for_language(ProgrammingLanguage::JavaScript)
        .expect("javascript should be present");

    let locals = js
        .suite_captures(NvimQuerySuite::Locals)
        .expect("locals should be present");
    assert!(
        locals
            .iter()
            .any(|c| c.name == "local.definition" && c.reused)
    );

    let highlights = js
        .suite_captures(NvimQuerySuite::Highlights)
        .expect("highlights should be present");
    assert!(
        highlights
            .iter()
            .any(|c| c.name == "function.call" && !c.reused)
    );
}

#[test]
fn capture_parity_for_typescript() {
    let inventory = QueryInventory::new();
    let ts = inventory
        .for_language(ProgrammingLanguage::TypeScript)
        .expect("typescript should be present");

    let locals = ts
        .suite_captures(NvimQuerySuite::Locals)
        .expect("locals should be present");
    assert!(
        locals
            .iter()
            .any(|c| c.name == "local.definition.interface" && c.reused)
    );
}

#[test]
fn capture_parity_for_python() {
    let inventory = QueryInventory::new();
    let py = inventory
        .for_language(ProgrammingLanguage::Python)
        .expect("python should be present");

    let locals = py
        .suite_captures(NvimQuerySuite::Locals)
        .expect("locals should be present");
    assert!(
        locals
            .iter()
            .any(|c| c.name == "local.definition" && c.reused)
    );
}

#[test]
fn suite_for_kind_mapping() {
    assert_eq!(
        suite_for_kind(QueryKind::Locals),
        Some(NvimQuerySuite::Locals)
    );
    assert!(suite_for_kind(QueryKind::Lint).is_none());
    assert!(suite_for_kind(QueryKind::References).is_none());
    assert!(suite_for_kind(QueryKind::Comments).is_none());
}

// =============================================================================
// Provenance Assertions
// =============================================================================

/// Vendored locals queries must report nvim-treesitter as their source.
#[test]
fn vendor_queries_have_nvim_provenance() {
    for language in ProgrammingLanguage::all() {
        let prov = query_provenance(language, QueryKind::Locals)
            .expect("all languages should have locals provenance");

        // Locals are vendored from nvim-treesitter or overlayed
        assert!(
            prov.source_project == "nvim-treesitter" || prov.source_project == "tree-hugger",
            "{language} locals provenance should be nvim-treesitter or tree-hugger"
        );
        assert!(!prov.upstream_revision.is_empty());
        assert!(!prov.license.is_empty());
    }
}

/// Custom lint queries must report tree-hugger as their source.
#[test]
fn lint_queries_have_local_provenance() {
    for language in ProgrammingLanguage::all() {
        if let Some(prov) = query_provenance(language, QueryKind::Lint) {
            assert_eq!(
                prov.source_project, "tree-hugger",
                "{language} lint should be a tree-hugger original"
            );
            assert_eq!(prov.translation_status, TranslationStatus::Original);
        }
    }
}

/// References queries must report tree-hugger as their source.
#[test]
fn references_queries_have_local_provenance() {
    for language in ProgrammingLanguage::all() {
        if let Some(prov) = query_provenance(language, QueryKind::References) {
            assert_eq!(
                prov.source_project, "tree-hugger",
                "{language} references should be a tree-hugger original"
            );
        }
    }
}

/// Provenance metadata must be serializable (all fields non-empty where
/// required).
#[test]
fn provenance_fields_are_populated() {
    let all = all_query_provenance();
    assert!(!all.is_empty());

    for ((language, kind), prov) in &all {
        assert!(
            !prov.source_project.is_empty(),
            "{language} {kind}: source_project should not be empty"
        );
        assert!(
            !prov.upstream_revision.is_empty(),
            "{language} {kind}: upstream_revision should not be empty"
        );
        assert!(
            !prov.local_path.is_empty(),
            "{language} {kind}: local_path should not be empty"
        );
        assert!(
            !prov.license.is_empty(),
            "{language} {kind}: license should not be empty"
        );
    }
}

// =============================================================================
// Predicate and Directive Compatibility
// =============================================================================

/// Native predicates must be correctly identified.
#[test]
fn native_predicates_are_supported() {
    let registry = CompatibilityRegistry::new();
    let native = [
        "eq",
        "match",
        "lua-match",
        "any-of",
        "contains",
        "any-contains",
    ];

    for name in native {
        let entry = registry.get(name).expect("{name} should be registered");
        assert_eq!(
            entry.status,
            CompatibilityStatus::Native,
            "{name} should be native"
        );
    }
}

/// Unsupported predicates must be detectable in query text.
#[test]
fn unsupported_predicates_are_found() {
    let query_text = r#"
(function_declaration
  (#make-range! "start" "end"))
"#;
    let found = find_unsupported_predicates(query_text);
    assert!(found.contains(&"make-range".to_string()));
}

/// Hook-requiring predicates must be detectable in query text.
#[test]
fn hook_predicates_are_found() {
    let query_text = r#"
(function_declaration
  (#has-parent? "program"))
"#;
    let found = find_hook_predicates(query_text);
    assert!(found.iter().any(|(name, _)| name == "has-parent"));
}

/// Simple queries with only native predicates should report nothing.
#[test]
fn simple_query_has_no_issues() {
    let query_text = r#"
(call_expression
  function: (field_expression
    field: (field_identifier) @_method)
  (#eq? @_method "unwrap"))
"#;
    let unsupported = find_unsupported_predicates(query_text);
    let hooks = find_hook_predicates(query_text);
    assert!(unsupported.is_empty());
    assert!(hooks.is_empty());
}

// =============================================================================
// Integration: Provenance + Compatibility
// =============================================================================

/// A full workflow that checks a query file for provenance and
/// compatibility together.
#[test]
fn full_query_audit_workflow() {
    let language = ProgrammingLanguage::Rust;
    let kind = QueryKind::Locals;

    // 1. Provenance check
    let prov = query_provenance(language, kind).expect("rust locals should have provenance");
    assert_eq!(prov.source_project, "nvim-treesitter");

    // 2. Compatibility check on the query text
    let query_text = include_str!("../../queries/vendor/rust/locals.scm");
    let unsupported = find_unsupported_predicates(query_text);
    let _hooks = find_hook_predicates(query_text);
    assert!(
        unsupported.is_empty(),
        "rust locals should not contain unsupported predicates"
    );
}

/// Audit all vendored queries for unsupported predicates.
#[test]
fn no_unsupported_predicates_in_vendored_queries() {
    let vendored_queries = [
        (
            ProgrammingLanguage::Rust,
            include_str!("../../queries/vendor/rust/locals.scm"),
        ),
        (
            ProgrammingLanguage::JavaScript,
            include_str!("../../queries/vendor/javascript/locals.scm"),
        ),
        (
            ProgrammingLanguage::TypeScript,
            include_str!("../../queries/vendor/typescript/locals.scm"),
        ),
        (
            ProgrammingLanguage::Go,
            include_str!("../../queries/vendor/go/locals.scm"),
        ),
        (
            ProgrammingLanguage::Python,
            include_str!("../../queries/vendor/python/locals.scm"),
        ),
    ];

    for (language, query_text) in vendored_queries {
        let unsupported = find_unsupported_predicates(query_text);
        assert!(
            unsupported.is_empty(),
            "{language} vendored query contains unsupported predicates: {unsupported:?}"
        );
    }
}

/// Audit all overlay queries for unsupported predicates.
#[test]
fn no_unsupported_predicates_in_overlay_queries() {
    let overlay_queries = [
        (
            ProgrammingLanguage::Rust,
            include_str!("../../queries/rust/lint.scm"),
        ),
        (
            ProgrammingLanguage::JavaScript,
            include_str!("../../queries/javascript/lint.scm"),
        ),
        (
            ProgrammingLanguage::TypeScript,
            include_str!("../../queries/typescript/lint.scm"),
        ),
    ];

    for (language, query_text) in overlay_queries {
        let unsupported = find_unsupported_predicates(query_text);
        assert!(
            unsupported.is_empty(),
            "{language} overlay query contains unsupported predicates: {unsupported:?}"
        );
    }
}
