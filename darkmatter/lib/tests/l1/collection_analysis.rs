//! Passive `::map` / `::filter` site analysis through the public API:
//! [`analyze_collection_blocks`], the [`BodyScopes`] it returns, and the
//! composition call check that consumes them.
//!
//! The test host's functions panic when entered and count their entries, so
//! a test that mentions them proves analysis evaluated no callback. Host
//! installation is process-wide; nextest runs each test in its own process.

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use darkmatter::markdown::coercion::ValueKind;
use darkmatter::markdown::compose::collection_blocks::{
    BodyScopes, CollectionAnalysis, CollectionAnalysisOptions, CollectionError, ExpressionRole, analyze_collection_blocks,
};
use darkmatter::markdown::compose::expression::{
    Availability, BindingView, DeclaredTypes, HostFunctionCatalog, HostFunctions, ScopeId, Severity, TypeScope,
    install_host_functions, parse,
};
use darkmatter::markdown::compose::page_blocks::BodyGuards;
use darkmatter::markdown::compose::policy::BlockingPolicy;
use darkmatter::markdown::compose::report_diagnostics::error_diagnostics;
use darkmatter::markdown::compose::{ComposeContext, ComposeOperation, ComposeOperationSet, ComposeOptions};
use darkmatter::markdown::schemas::catalog::{GlobalCatalog, GlobalRequirement};
use darkmatter::markdown::schemas::{SimplifiedSchema, parse_yaml_schema};
use darkmatter::markdown::{Markdown, MarkdownError, SourceRef};
use serde_json::json;
use tempfile::TempDir;

static ENTRIES: AtomicUsize = AtomicUsize::new(0);

const CATALOG: &str = "kind: schema\ncategories:\n    - Project -> Test host.\ntypes:\n    project_label: |-\n        function(\n            parameters(name: string);\n            returns(string);\n            category(Project);\n        ) -> A label.\n";

/// A host whose only function panics when entered.
fn failing_host() {
    let catalog = HostFunctionCatalog::load(CATALOG, "tests/collection_analysis.yaml").expect("catalog loads");
    let host = HostFunctions::builder(catalog)
        .function(
            "project_label",
            &[],
            |registration| registration.reject_sources("name", &[ValueKind::Number]),
            Arc::new(|_| {
                ENTRIES.fetch_add(1, Ordering::SeqCst);
                panic!("a handler was entered during passive analysis")
            }),
        )
        .build()
        .expect("host registers");
    // Process-wide: a second installation in one process is ignored.
    let _ = install_host_functions(host);
}

fn schema(yaml: &str) -> SimplifiedSchema {
    parse_yaml_schema(&serde_yaml_ng::from_str(yaml).expect("YAML parses")).expect("schema parses")
}

fn options(view: Option<&BindingView>) -> CollectionAnalysisOptions<'_> {
    CollectionAnalysisOptions {
        operations: ComposeOperationSet::all(),
        binding_view: view,
        allow_untyped_map_variables: false,
        allow_untyped_filter_variables: false,
        source_path: Some(Path::new("/doc.md")),
    }
}

fn analyze(schema_yaml: &str, source: &str) -> Result<CollectionAnalysis, CollectionError> {
    let schema = schema(schema_yaml);
    let declared = DeclaredTypes::new(Some(&schema));
    analyze_collection_blocks(source, &TypeScope::unguarded(&declared), &options(None))
}

/// The (line, authored text) a located error points at.
fn located<'a>(source: &'a str, error: &CollectionError) -> (usize, &'a str) {
    match error.site().expect("a located error").source.as_ref() {
        SourceRef::OnDiskSpan { span, .. } => (span.line(), &source[span.range()]),
        other => panic!("expected an authored span, got {other:?}"),
    }
}

/// The error's code and the authored text it points at.
fn rejected<'a>(schema_yaml: &str, source: &'a str) -> (&'static str, &'a str) {
    let error = analyze(schema_yaml, source).expect_err("the region is statically invalid");
    (error.code(), located(source, &error).1)
}

const PEOPLE: &str = "people: '{ name: string, active: boolean(required) }[](required)'\n";

// ── gates and sources ──────────────────────────────────────────────────────

#[test]
fn a_source_is_narrowed_by_its_own_gate() {
    let yaml = "u:\n  - string(required)\n  - string[](required)\n";
    let bare = "::map u \"x => x\"\nb\n::end-map\n";
    assert_eq!(rejected(yaml, bare), ("dm.collection.invalid_source", "u"));

    let gated = "::map u \"x => x\" when=\"is_array(u)\"\nb\n::end-map\n";
    assert!(analyze(yaml, gated).is_ok(), "a type predicate in the gate establishes iterability");

    let wrong_side = "::map u \"x => x\" when=\"!is_array(u)\"\nb\n::end-map\n";
    assert_eq!(rejected(yaml, wrong_side).0, "dm.collection.invalid_source");
}

#[test]
fn a_false_gate_does_not_excuse_an_invalid_source() {
    let source = "::map s \"x => x\" when=\"false\"\nb\n::end-map\n";
    assert_eq!(rejected("s: string(required)\n", source), ("dm.collection.invalid_source", "s"));
}

#[test]
fn a_dead_branch_is_checked_like_a_live_one() {
    let source = "::block when=\"false\"\n::map s \"x => x\"\nb\n::end-map\n::end-block\n";
    assert_eq!(rejected("s: string(required)\n", source), ("dm.collection.invalid_source", "s"));
}

#[test]
fn each_static_defect_has_its_code_and_authored_span() {
    let people = PEOPLE;
    let cases: &[(&str, &str, &str, &str)] = &[
        ("untyped map source", "x: string\n", "::map nope \"x => x\"\nb\n::end-map\n", "dm.collection.untyped_map_source"),
        (
            "untyped filter source",
            "x: string\n",
            "::filter nope \"x => x\"\nb\n::end-filter\n",
            "dm.collection.untyped_filter_source",
        ),
        ("scalar source", "s: string(required)\n", "::filter s\nb\n::end-filter\n", "dm.collection.invalid_source"),
    ];
    for (name, yaml, source, code) in cases {
        let error = analyze(yaml, source).expect_err(name);
        assert_eq!(error.code(), *code, "{name}");
        let (line, text) = located(source, &error);
        assert_eq!((line, text), (1, source.lines().next().unwrap().split(' ').nth(1).unwrap()), "{name}");
    }

    let protected = "::map people \"ctx => ctx\"\nb\n::end-map\n";
    assert_eq!(rejected(people, protected), ("dm.expression.invalid_callback", "ctx"));
    let protected_field = "::map people \"{ null, name } => name\"\nb\n::end-map\n";
    assert_eq!(rejected(people, protected_field), ("dm.expression.invalid_callback", "null"));

    let pattern = "::map people \"{ nope } => nope\"\nb\n::end-map\n";
    assert_eq!(rejected(people, pattern), ("dm.collection.pattern_mismatch", "nope"));
    let result = "::map o \"{ key, value } => value\"\nb\n::end-map\n";
    assert_eq!(rejected("o:\n  a: string(required)\n", result), ("dm.collection.pattern_mismatch", "key, value"));
    let option_pattern = "::filter people filter=\"{ nope } => nope\"\nb\n::end-filter\n";
    assert_eq!(rejected(people, option_pattern), ("dm.collection.pattern_mismatch", "nope"));

    let sort = "::map people \"p => p.name\" sort=\"p => p.active\"\nb\n::end-map\n";
    assert_eq!(rejected(people, sort), ("dm.collection.invalid_sort_key", "p.active"));
    let fine = "::map people \"p => p.name\" sort=\"p => p.name\"\nb\n::end-map\n";
    assert!(analyze(people, fine).is_ok());
    let uncertain = "::map people \"p => p.name\" sort=\"p => nobody(p)\"\nb\n::end-map\n";
    assert!(analyze(people, uncertain).is_ok(), "an unknown key type is checked at runtime");

    let malformed = "::map people \"p => p\" bogus=\"x\"\nb\n::end-map\n";
    assert_eq!(rejected(people, malformed), ("dm.collection.malformed", "bogus"));
}

#[test]
fn only_an_unknown_source_is_recoverable_and_the_warning_is_reported_once() {
    let schema = schema("x: string\ns: string(required)\n");
    let declared = DeclaredTypes::new(Some(&schema));
    let site = TypeScope::unguarded(&declared);
    let source = "text\n::map nope \"x => x\"\nb\n::end-map\n::filter nope\nb\n::end-filter\n";

    let mut waive_map = options(None);
    waive_map.allow_untyped_map_variables = true;
    let error = analyze_collection_blocks(source, &site, &waive_map).expect_err("the filter source is still untyped");
    assert_eq!(error.code(), "dm.collection.untyped_filter_source");

    waive_map.allow_untyped_filter_variables = true;
    let analysis = analyze_collection_blocks(source, &site, &waive_map).expect("both recoveries are allowed");
    let codes: Vec<_> = analysis.diagnostics.iter().map(|d| d.code.as_str()).collect();
    assert_eq!(codes, ["dm.collection.untyped_map_source", "dm.collection.untyped_filter_source"]);
    assert!(analysis.diagnostics.iter().all(|d| d.severity == Severity::Warning));
    assert_eq!(analysis.recovered.len(), 2);
    assert_eq!(&source[analysis.recovered[0].span.clone()], "nope");
    let position = analysis.diagnostics[0].source.as_ref().expect("located");
    assert_eq!((position.line, position.column), (2, 7));

    // A recovery never excuses a known scalar.
    let scalar = "::map s \"x => x\"\nb\n::end-map\n";
    let error = analyze_collection_blocks(scalar, &site, &waive_map).expect_err("a scalar is not recoverable");
    assert_eq!(error.code(), "dm.collection.invalid_source");
}

#[test]
fn disabled_collection_blocks_leave_wrappers_inert() {
    let schema = schema("x: string\n");
    let declared = DeclaredTypes::new(Some(&schema));
    let site = TypeScope::unguarded(&declared);
    let malformed = "::map\nbody\n";
    assert_eq!(
        analyze_collection_blocks(malformed, &site, &options(None)).expect_err("enabled: malformed").code(),
        "dm.collection.malformed"
    );

    let mut operations = ComposeOperationSet::all();
    operations.remove(ComposeOperation::CollectionBlocks);
    let mut disabled = options(None);
    disabled.operations = operations;
    let analysis = analyze_collection_blocks(malformed, &site, &disabled).expect("disabled: inert text");
    assert!(analysis.regions.is_empty() && analysis.scopes.is_empty());
    let untyped = "::map nope \"x => x\"\n{{ x }}\n::end-map\n";
    let analysis = analyze_collection_blocks(untyped, &site, &disabled).expect("disabled: no source check");
    assert!(analysis.scopes.expression_sites(&site).is_empty());
}

// ── scopes ────────────────────────────────────────────────────────────────

#[test]
fn each_option_callback_gets_an_independent_parameter_frame() {
    let schema = schema(PEOPLE);
    let declared = DeclaredTypes::new(Some(&schema));
    let site = TypeScope::unguarded(&declared);
    let source = "::filter people \"p => p.active\" filter=\"q => q.name\"\n{{ p.name }}\n::end-filter\n";
    let analysis = analyze_collection_blocks(source, &site, &options(None)).expect("valid");

    let sites = analysis.scopes.expression_sites(&site);
    let roles: Vec<_> = sites.iter().map(|s| s.role).collect();
    assert_eq!(roles, [ExpressionRole::Positional, ExpressionRole::Filter]);
    assert!(sites[0].scope.resolves_to_local("p") && !sites[0].scope.resolves_to_local("q"));
    assert!(sites[1].scope.resolves_to_local("q") && !sites[1].scope.resolves_to_local("p"));

    let body = analysis.scopes.scope_at(site.clone(), source.find("{{").unwrap());
    assert!(body.resolves_to_local("p") && !body.resolves_to_local("q"), "the body binds only the positional pattern");
    let name = body.type_of_expr(&parse("p.name").unwrap()).unwrap();
    assert!(name.guard_presence().is_some(), "the other callback's name is the same original item");
    let header = analysis.scopes.scope_at(site.clone(), 0);
    assert!(!header.resolves_to_local("p"), "the header is outside the body");
}

#[test]
fn a_map_result_inherits_no_input_facts() {
    let schema = schema("items: (string | null)[](required)\n");
    let declared = DeclaredTypes::new(Some(&schema));
    let site = TypeScope::unguarded(&declared);
    let source = "::filter items \"x => x\"\n{{ x }}\n::end-filter\n::map items \"x => x\" filter=\"y => y\"\n{{ x }}\n::end-map\n";
    let analysis = analyze_collection_blocks(source, &site, &options(None)).expect("valid");
    let x = parse("x").unwrap();
    let mut bodies = source.match_indices("{{ x }}").map(|(offset, _)| offset);

    let filter_body = analysis.scopes.scope_at(site.clone(), bodies.next().unwrap());
    assert!(filter_body.type_of_expr(&x).unwrap().guard_presence().is_some(), "the retained original carries the fact");
    let map_body = analysis.scopes.scope_at(site.clone(), bodies.next().unwrap());
    let result = map_body.type_of_expr(&x).unwrap();
    assert!(result.guard_presence().is_none(), "facts about the input do not migrate by spelling");
    assert!(result.admits(&json!("text")));
}

#[test]
fn nested_regions_see_the_enclosing_iteration_locals() {
    let yaml = "teams: '{ name: string(required), members: string[](required) }[](required)'\n";
    let source = "::map teams \"team => team\"\n::map team.members \"m => m\"\n{{ m }}\n::end-map\n::end-map\n";
    let schema = schema(yaml);
    let declared = DeclaredTypes::new(Some(&schema));
    let site = TypeScope::unguarded(&declared);
    let analysis = analyze_collection_blocks(source, &site, &options(None)).expect("the inner source is typed by the outer local");
    assert_eq!(analysis.regions.len(), 1);
    assert_eq!(analysis.regions[0].children.len(), 1);

    let inner = analysis.scopes.scope_at(site.clone(), source.find("{{").unwrap());
    let member = inner.type_of_expr(&parse("m").unwrap()).unwrap();
    assert!(member.admits(&json!("ada")) && !member.admits(&json!(1)));
    assert!(inner.resolves_to_local("team") && inner.resolves_to_local("m"));

    let untyped = "::map teams \"team => team\"\n::map team.nothing \"m => m\"\nb\n::end-map\n::end-map\n";
    assert_eq!(rejected(yaml, untyped), ("dm.collection.untyped_map_source", "team.nothing"));
}

#[test]
fn frontmatter_stays_outside_body_locals() {
    let schema = schema("person: number(required)\npeople: string[](required)\n");
    let declared = DeclaredTypes::new(Some(&schema));
    let site = TypeScope::unguarded(&declared);
    let source = "::map people \"person => person\"\n{{ person }} {{ fm.person }}\n::end-map\n";
    let analysis = analyze_collection_blocks(source, &site, &options(None)).expect("valid");
    let body = analysis.scopes.scope_at(site, source.find("{{").unwrap());

    let local = body.type_of_expr(&parse("person").unwrap()).unwrap();
    assert!(local.admits(&json!("text")) && !local.admits(&json!(1)), "the bare name is the local");
    let document = body.type_of_expr(&parse("fm.person").unwrap()).unwrap();
    assert!(document.admits(&json!(1)) && !document.admits(&json!("text")), "`fm.` always reads the document");
    assert!(!body.resolves_to_local("fm.person"));
}

#[test]
fn host_globals_serve_as_sources_and_captures_and_cannot_be_shadowed() {
    const HOST: &[GlobalRequirement] = &[GlobalRequirement::shape("seq")];
    let catalog = GlobalCatalog::from_documents(
        "host.yaml",
        &[("host.yaml", "kind: schema\n$schema:\n    seq:\n        items: \"string[](required)\"\n        floor: \"number(required)\"\n")],
        HOST,
    )
    .expect("catalog loads");
    let view = BindingView::builder(ScopeId::new("test.event"))
        .declare("seq", "The step.", Availability::Available)
        .build()
        .expect("view");
    let declared = DeclaredTypes::new(None).with_binding_view(&view).with_global_catalog(&catalog);
    let site = TypeScope::unguarded(&declared);

    let source = "::filter seq.items \"i => i == seq.floor\"\n{{ i }}\n::end-filter\n";
    let analysis = analyze_collection_blocks(source, &site, &options(Some(&view))).expect("a typed global is a source");
    let sites = analysis.scopes.expression_sites(&site);
    let capture = sites[0].scope.type_of_expr(&parse("seq.floor").unwrap()).unwrap();
    assert!(capture.admits(&json!(1)), "a capture of a global keeps its declared type inside the callback");
    let item = analysis.scopes.scope_at(site.clone(), source.find("{{").unwrap());
    assert!(item.type_of_expr(&parse("i").unwrap()).unwrap().admits(&json!("a")));

    let shadow = "::map seq.items \"seq => seq\"\nb\n::end-map\n";
    let error = analyze_collection_blocks(shadow, &site, &options(Some(&view))).expect_err("a host global is protected");
    assert_eq!((error.code(), located(shadow, &error).1), ("dm.expression.invalid_callback", "seq"));
    assert!(analyze_collection_blocks(shadow, &site, &options(None)).is_ok(), "protection follows the view actually in force");
}

#[test]
fn a_document_with_only_page_blocks_is_unchanged() {
    let schema = schema("name: string\n");
    let declared = DeclaredTypes::new(Some(&schema));
    let site = TypeScope::unguarded(&declared);
    let body = "before {{ name }}\n::block when=\"name\"\n{{ name }}\n::end-block\nafter {{ name }}\n";
    let analysis = analyze_collection_blocks(body, &site, &options(None)).expect("valid");
    assert!(analysis.regions.is_empty() && analysis.diagnostics.is_empty() && analysis.recovered.is_empty());

    let guards = BodyGuards::scan(body);
    let name = parse("name").unwrap();
    for (offset, _) in body.match_indices("{{") {
        let expected = guards.scope_at(site.clone(), offset).type_of_expr(&name).unwrap();
        let actual = analysis.scopes.scope_at(site.clone(), offset).type_of_expr(&name).unwrap();
        assert_eq!(actual.guard_presence().is_some(), expected.guard_presence().is_some(), "offset {offset}");
        let conditional = analysis.scopes.conditional().scope_at(site.clone(), offset).type_of_expr(&name).unwrap();
        assert_eq!(conditional.guard_presence().is_some(), expected.guard_presence().is_some());
    }
    assert!(BodyScopes::scan(body, &ComposeOperationSet::all()).expression_sites(&site).is_empty());
}

#[test]
fn analysis_evaluates_no_callback_and_resolves_nothing() {
    failing_host();
    let schema = schema(PEOPLE);
    let declared = DeclaredTypes::new(Some(&schema));
    let site = TypeScope::unguarded(&declared);
    let source = concat!(
        "::map people \"p => project_label(p.name)\" when=\"file_exists('/missing/never.md')\" ",
        "filter=\"p => file_exists('https://example.invalid/x.md')\" sort=\"p => project_label(p)\"\n",
        "{{ project_label(1) }}\n",
        "::end-map\n",
    );
    let analysis = analyze_collection_blocks(source, &site, &options(None)).expect("valid");
    assert_eq!(analysis.scopes.expression_sites(&site).len(), 4);
    assert_eq!(ENTRIES.load(Ordering::SeqCst), 0, "no handler was entered");
}

// ── composition ───────────────────────────────────────────────────────────

fn compose(
    dir: &Path,
    content: &str,
    configure: impl FnOnce(ComposeOptions) -> ComposeOptions,
) -> Result<(Markdown, Vec<darkmatter::markdown::compose::ComposeWarning>), MarkdownError> {
    let path = dir.join("doc.md");
    std::fs::write(&path, content).unwrap();
    let options = configure(
        ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir, "")).with_source_file(path.clone()),
    );
    Markdown::try_from(path.as_path())
        .unwrap()
        .compose_with(&crate::request_support::request_at(dir, options))
        .map(|(composed, report)| (composed, report.warnings))
}

fn codes(error: &MarkdownError) -> Vec<String> {
    error_diagnostics(error).into_iter().map(|diagnostic| diagnostic.code).collect()
}

const PEOPLE_DOCUMENT: &str = "---\n$schema:\n  people: '{ name: string(required) }[](required)'\npeople:\n  - name: ada\n---\n";

#[test]
fn composition_rejects_calls_in_skipped_bodies_and_callbacks_before_any_handler_runs() {
    failing_host();
    let dir = TempDir::new().unwrap();
    let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();

    let skipped_body = format!("{PEOPLE_DOCUMENT}::map people \"p => p\" when=\"false\"\n{{{{ project_label(42) }}}}\n::end-map\n");
    let error = compose(&root, &skipped_body, |options| options).expect_err("a false gate does not excuse the call");
    assert_eq!(error.rejected_call().map(|check| check.function.as_str()), Some("project_label"), "{error}");

    let callback = format!("{PEOPLE_DOCUMENT}::filter people \"p => project_label(42)\"\nbody\n::end-filter\n");
    let error = compose(&root, &callback, |options| options).expect_err("the callback's call is judged");
    assert_eq!(error.rejected_call().map(|check| check.function.as_str()), Some("project_label"), "{error}");

    let gate = format!("{PEOPLE_DOCUMENT}::map people \"p => p\" when=\"project_label(42)\"\nbody\n::end-map\n");
    let error = compose(&root, &gate, |options| options).expect_err("the gate's call is judged");
    assert_eq!(error.rejected_call().map(|check| check.function.as_str()), Some("project_label"), "{error}");
    assert_eq!(ENTRIES.load(Ordering::SeqCst), 0, "a rejected document runs nothing");
}

#[test]
fn composition_reports_static_collection_defects_with_their_codes() {
    let dir = TempDir::new().unwrap();
    let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();

    for (name, tail, code) in [
        ("untyped source", "::map nope \"x => x\"\nbody\n::end-map\n", "dm.collection.untyped_map_source"),
        ("protected name", "::map people \"ctx => ctx\"\nbody\n::end-map\n", "dm.expression.invalid_callback"),
        ("pattern", "::map people \"{ nope } => nope\"\nbody\n::end-map\n", "dm.collection.pattern_mismatch"),
        ("malformed", "::map people \"p => p\" bogus=\"x\"\nbody\n::end-map\n", "dm.collection.malformed"),
        ("unclosed", "::map people \"p => p\"\nbody\n", "dm.collection.malformed"),
    ] {
        let error = compose(&root, &format!("{PEOPLE_DOCUMENT}{tail}"), |options| options).expect_err(name);
        assert_eq!(codes(&error), [code], "{name}: {error}");
        let diagnostic = &error_diagnostics(&error)[0];
        let line = diagnostic.source.as_ref().map(|source| source.line);
        assert!(line.is_some_and(|line| line >= 7), "{name}: the finding is located in the file, got {line:?}");
    }
}

#[test]
fn a_recovered_source_is_a_warning_the_blocking_policy_sees_before_evaluation() {
    let dir = TempDir::new().unwrap();
    let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
    let document = format!("{PEOPLE_DOCUMENT}::map nope \"x => x\"\nbody\n::end-map\n");

    let (_, warnings) = compose(&root, &document, |options| options.with_allow_untyped_map_variables(true))
        .unwrap_or_else(|error| panic!("the recovery waives the typing requirement: {error}"));
    let found: Vec<_> = warnings.iter().filter(|warning| warning.code.as_deref() == Some("dm.collection.untyped_map_source")).collect();
    assert_eq!(found.len(), 1, "once, at the authored source: {warnings:?}");
    assert_eq!(found[0].line_number, Some(7));

    let error = compose(&root, &document, |options| {
        options.with_allow_untyped_map_variables(true).with_blocking_policy(BlockingPolicy::default().deny_all())
    })
    .expect_err("a blocking policy stops the document at the recovery warning");
    assert!(matches!(error, MarkdownError::BlockedDiagnostics { .. }), "{error}");

    let filter = format!("{PEOPLE_DOCUMENT}::filter nope\nbody\n::end-filter\n");
    let error = compose(&root, &filter, |options| options.with_allow_untyped_map_variables(true))
        .expect_err("the map recovery does not waive a filter source");
    assert_eq!(codes(&error), ["dm.collection.untyped_filter_source"]);
    assert!(compose(&root, &filter, |options| options.with_allow_untyped_filter_variables(true)).is_ok());
}

#[test]
fn disabled_collection_blocks_compose_without_judging_the_wrappers() {
    let dir = TempDir::new().unwrap();
    let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
    let document = format!("{PEOPLE_DOCUMENT}::map nope \"x => x\"\nbody\n::end-map\n");
    assert!(compose(&root, &document, |options| options).is_err());
    let (composed, _) = compose(&root, &document, |options| options.disable(ComposeOperation::CollectionBlocks))
        .unwrap_or_else(|error| panic!("a disabled wrapper is inert text: {error}"));
    assert!(composed.content().contains("::map nope"), "{}", composed.content());
}
