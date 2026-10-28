//! Host expression functions through the library's public API: the test
//! host's registration (one place, [`test_host`]), registration errors before
//! any evaluation, the generated facts artifact's freshness, the reader's
//! input robustness matrix, and composition/runtime parity with the
//! built-ins.
//!
//! The test host's catalog and artifact are committed under
//! `tests/fixtures/host_functions/`. `DARKMATTER_BLESS_HOST_FACTS=1`
//! rewrites the artifact from the registration below instead of comparing.
//! Each handler counts its entries, so a test can prove analysis entered
//! none. Tests that install functions process-wide rely on nextest running
//! each test in its own process.

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use darkmatter::markdown::Markdown;
use darkmatter::markdown::coercion::ValueKind;
use darkmatter::markdown::compose::expression::call_binding::{EvaluationMode, FactKind};
use darkmatter::markdown::compose::expression::host::FactsArtifact;
use darkmatter::markdown::compose::expression::{
    ArgumentCategory, BindingView, CallCheck, DeclaredTypes, ExpressionError, HostCatalogError, HostFunctionCatalog,
    HostFunctionError,
    HostFunctions, TypeScope, check_expression_calls, incomplete_host_calls, install_host_functions, parse_spanned,
    validate_expression,
};
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions};
use darkmatter::markdown::schemas::{SimplifiedSchema, parse_yaml_schema};
use serde_json::{Value, json};
use tempfile::TempDir;

const BLESS_VARIABLE: &str = "DARKMATTER_BLESS_HOST_FACTS";

static IS_PROJECT_FILE_ENTRIES: AtomicUsize = AtomicUsize::new(0);
static PROJECT_LABEL_ENTRIES: AtomicUsize = AtomicUsize::new(0);

fn entries() -> (usize, usize) {
    (IS_PROJECT_FILE_ENTRIES.load(Ordering::SeqCst), PROJECT_LABEL_ENTRIES.load(Ordering::SeqCst))
}

fn load(catalog: &str) -> HostFunctionCatalog {
    HostFunctionCatalog::load(catalog, "tests/fixtures/host_functions/project.yaml").expect("the test catalog loads")
}

/// The test host's registration: `is_project_file` proves its argument
/// suitable for `file` when true; `project_label` refuses a number before
/// any conversion, so `project_label(42)` is category 5 only with the
/// registered metadata.
fn test_host(catalog: &str) -> HostFunctions {
    HostFunctions::builder(load(catalog))
        .function(
            "is_project_file",
            &[],
            |registration| registration,
            Arc::new(|call| {
                IS_PROJECT_FILE_ENTRIES.fetch_add(1, Ordering::SeqCst);
                Ok(json!(call.args().text(0).is_some_and(|text| text.ends_with(".md"))))
            }),
        )
        .function(
            "project_label",
            &[],
            |registration| registration.reject_sources("name", &[ValueKind::Number]),
            Arc::new(|call| {
                PROJECT_LABEL_ENTRIES.fetch_add(1, Ordering::SeqCst);
                Ok(json!(format!("project:{}", call.args().text(0).unwrap_or_default())))
            }),
        )
        .build()
        .expect("the test host registers")
}

fn schema(yaml: &str) -> SimplifiedSchema {
    parse_yaml_schema(&serde_yaml_ng::from_str(yaml).expect("YAML parses")).expect("schema parses")
}

/// The check of `function` in `expression`, if analysis made one.
fn check_in(declared: &DeclaredTypes<'_>, expression: &str, function: &str) -> Option<CallCheck> {
    check_expression_calls(&parse_spanned(expression).expect("parses"), &TypeScope::unguarded(declared))
        .into_iter()
        .find(|check| check.function == function)
}

const GUARDED: &str = "is_project_file(source) && frontmatter(source, 'title')";
const UNGUARDED: &str = "frontmatter(source, 'title')";
const LABEL: &str = "project_label(42)";

// ── registration ────────────────────────────────────────────────────────────

/// Criterion 37, registration half: every invalid set fails `build` (or
/// installation) before anything is evaluated.
#[test]
fn registration_rejects_collisions_parity_gaps_and_invalid_facts_before_evaluation() {
    let catalog = std::fs::read_to_string(biscuit_test_harness::manifest_dir!().join("tests/fixtures/host_functions/project.yaml"))
        .expect("the test catalog");
    let handler: darkmatter::markdown::compose::expression::HostHandler = Arc::new(|_| Ok(json!(true)));
    let label: darkmatter::markdown::compose::expression::HostHandler = Arc::new(|_| Ok(json!("label")));
    let base = |aliases: &[&str]| {
        HostFunctions::builder(load(&catalog))
            .function("is_project_file", aliases, |registration| registration, handler.clone())
    };
    let error = |result: Result<HostFunctions, HostFunctionError>| result.expect_err("the set is invalid");

    let duplicate = base(&[]).function("is_project_file", &[], |r| r, handler.clone()).build();
    assert_eq!(error(duplicate), HostFunctionError::DuplicateName { name: "is_project_file".into() });
    let alias = base(&["ipf"]).function("project_label", &["IPF"], |r| r, label.clone()).build();
    assert_eq!(error(alias), HostFunctionError::DuplicateName { name: "IPF".into() });
    let builtin_alias = base(&["isstring"]).function("project_label", &[], |r| r, label.clone()).build();
    assert_eq!(error(builtin_alias), HostFunctionError::BuiltinCollision { name: "isstring".into() });
    let builtin = HostFunctionCatalog::load(
        "kind: schema\ncategories:\n  - Math\ntypes:\n  min: |-\n    function(parameters(a: number); returns(number); category(Math))\n",
        "min.yaml",
    )
    .expect("loads");
    let builtin = HostFunctions::builder(builtin).function("min", &[], |r| r, handler.clone()).build();
    assert_eq!(error(builtin), HostFunctionError::BuiltinCollision { name: "min".into() });

    let missing = base(&[]).build();
    assert_eq!(error(missing), HostFunctionError::MissingRegistration { function: "project_label".into() });
    let extra = base(&[])
        .function("project_label", &[], |r| r, label.clone())
        .function("not_in_catalog", &[], |r| r, handler.clone())
        .build();
    assert_eq!(error(extra), HostFunctionError::MissingCatalogFunction { function: "not_in_catalog".into() });

    // A fact is declared by the catalog, so an invalid clause fails the load
    // before any builder or handler exists.
    let load_error = |edited: String| {
        HostFunctionCatalog::load(&edited, "tests/fixtures/host_functions/project.yaml").expect_err("the catalog is invalid")
    };
    let unknown_parameter = load_error(catalog.replace("narrows(value: file; true-implies)", "narrows(other: file; true-implies)"));
    assert!(unknown_parameter.to_string().contains("does not declare"), "{unknown_parameter}");
    let not_boolean = load_error(catalog.replace(
        "function(\n            parameters(name: string);",
        "function(\n            parameters(name: string);\n            narrows(name; true-implies);",
    ));
    assert!(not_boolean.to_string().contains("returns(boolean)"), "a narrowing fact on a function returning string: {not_boolean}");
    let lazy = HostFunctions::builder(load(&catalog))
        .function("is_project_file", &[], |r| r.evaluation(EvaluationMode::Lazy), handler.clone())
        .function("project_label", &[], |r| r, label.clone())
        .build();
    assert_eq!(error(lazy), HostFunctionError::LazyFunction { function: "is_project_file".into() });

    install_host_functions(test_host(&catalog)).expect("the first installation");
    let again = install_host_functions(test_host(&catalog)).expect_err("names already installed");
    assert!(matches!(again, HostFunctionError::InstalledCollision { .. }), "{again:?}");
    assert_eq!(entries(), (0, 0), "no handler ran");
}

/// A loaded catalog owns what it read: after the source text and path
/// strings it was loaded from are dropped, its metadata, the registrations
/// built over it, and the facts artifact it regenerates are unchanged.
#[test]
fn a_host_catalog_outlives_the_strings_it_was_loaded_from() {
    let fixture = std::fs::read_to_string(biscuit_test_harness::manifest_dir!().join("tests/fixtures/host_functions/project.yaml"))
        .expect("the test catalog");
    let committed = std::fs::read_to_string(biscuit_test_harness::manifest_dir!().join("tests/fixtures/host_functions/project.facts.json"))
        .expect("the committed artifact");
    let committed_hash: Value = serde_json::from_str(&committed).expect("the committed artifact is JSON");

    let source = fixture.clone();
    let path = String::from("tests/fixtures/host_functions/project.yaml");
    let catalog = HostFunctionCatalog::load(&source, &path).expect("the test catalog loads");
    drop((source, path));

    assert_eq!(catalog.function_names(), ["is_project_file", "project_label"]);
    assert_eq!(catalog.path(), Path::new("tests/fixtures/host_functions/project.yaml"));
    assert_eq!(Some(catalog.version()), committed_hash["catalog_hash"].as_str());

    let analysis = HostFunctions::analysis_only(catalog.clone(), FactsArtifact::Text(&committed)).expect("reads");
    assert!(analysis.incomplete_functions().is_empty(), "{:?}", analysis.incomplete_functions());
    let file = analysis.registration("is_project_file").expect("complete");
    let parameter = &file.overloads()[0].parameters()[0];
    assert_eq!((parameter.name(), parameter.type_text()), ("value", "unknown | null"));
    let fact = file.narrowing_fact().expect("the fact");
    assert_eq!((fact.parameter(), fact.index(), fact.kind()), ("value", 0, FactKind::TrueImplies));
    let label = analysis.registration("project_label").expect("complete");
    assert_eq!(label.overloads()[0].parameters()[0].excluded_sources(), [ValueKind::Number]);

    let handler: darkmatter::markdown::compose::expression::HostHandler = Arc::new(|_| Ok(json!(true)));
    let host = HostFunctions::builder(catalog)
        .function(
            "is_project_file",
            &[],
            |registration| registration,
            handler.clone(),
        )
        .function("project_label", &[], |registration| registration.reject_sources("name", &[ValueKind::Number]), handler)
        .build()
        .expect("the test host registers");
    assert_eq!(host.function_names(), ["is_project_file", "project_label"]);
    assert_eq!(host.facts_artifact(), committed, "the retained catalog regenerates the committed artifact");
}

/// A catalog whose earlier function loads, inline example and all, and whose
/// later function names a file-form example fails the whole load with the
/// typed host error; nothing of the partly projected catalog is returned.
#[test]
fn a_host_catalog_failing_after_a_valid_inline_example_is_a_typed_load_error() {
    let source = concat!(
        "kind: schema\n",
        "categories:\n",
        "    - Project -> Ask about the test host's project.\n",
        "types:\n",
        "    project_first: |-\n",
        "        function(\n",
        "            parameters(text: string);\n",
        "            returns(string);\n",
        "            category(Project);\n",
        "            example(project_first(\"a\") => \"a\");\n",
        "        ) -> Loads.\n",
        "    project_second: |-\n",
        "        function(\n",
        "            parameters(text: string);\n",
        "            returns(string);\n",
        "            category(Project);\n",
        "            example(./function-examples/as_tsv.yaml);\n",
        "        ) -> Names a file-form example.\n",
    );
    let error: HostCatalogError = HostFunctionCatalog::load(source, "project_partial.yaml").expect_err("the load fails");
    assert_eq!(
        error.to_string(),
        "project_partial.yaml: `types.project_second`: example file `./function-examples/as_tsv.yaml` is not embedded"
    );
}

// ── facts artifact ──────────────────────────────────────────────────────────

/// The generated artifact equals the committed one byte for byte, and two
/// generations are identical.
#[test]
fn the_committed_facts_artifact_is_fresh_and_deterministic() {
    let catalog = std::fs::read_to_string(biscuit_test_harness::manifest_dir!().join("tests/fixtures/host_functions/project.yaml"))
        .expect("the test catalog");
    let generated = test_host(&catalog).facts_artifact();
    assert_eq!(generated, test_host(&catalog).facts_artifact(), "generation is deterministic");
    let path = biscuit_test_harness::manifest_dir!().join("tests/fixtures/host_functions/project.facts.json");
    if std::env::var_os(BLESS_VARIABLE).is_some() {
        std::fs::write(&path, &generated).expect("bless the artifact");
        return;
    }
    let committed = std::fs::read_to_string(&path).expect("the committed artifact");
    assert_eq!(generated, committed, "stale `{}`; rerun with {BLESS_VARIABLE}=1", path.display());
}

/// What a matrix cell leaves analysis knowing.
#[derive(Debug, Clone, Copy)]
enum Expect {
    /// Every catalog function is incomplete.
    AllIncomplete,
    /// Exactly these functions are incomplete.
    Incomplete(&'static [&'static str]),
    /// Nothing is incomplete, but `project_label` refuses no source.
    NoExclusions,
    /// Nothing is incomplete, and the check sees the edited field's rule.
    Complete(fn(&HostFunctions)),
}

/// One edit of the committed artifact.
enum Edit {
    Source(FactsArtifact<'static>),
    Text(fn(&str) -> String),
    Json(fn(&mut Value)),
}

fn function_entry<'v>(root: &'v mut Value, name: &str) -> &'v mut serde_json::Map<String, Value> {
    root["functions"][name].as_object_mut().expect("an entry")
}

/// The plan's input robustness matrix for the facts artifact reader: one
/// edit per cell of the committed, generated artifact, each read with the
/// committed catalog. The control row proves the unedited artifact gives the
/// positive result; every other row leaves the affected functions without a
/// category or narrowing, reports their calls as incomplete, and never makes
/// them unknown functions.
#[test]
fn the_facts_artifact_reader_walks_the_input_robustness_matrix() {
    let catalog = std::fs::read_to_string(biscuit_test_harness::manifest_dir!().join("tests/fixtures/host_functions/project.yaml"))
        .expect("the test catalog");
    let base = std::fs::read_to_string(biscuit_test_harness::manifest_dir!().join("tests/fixtures/host_functions/project.facts.json"))
        .expect("the committed artifact");
    let schema = schema("source: unknown\n");
    let declared = DeclaredTypes::new(Some(&schema));
    let both: &[&str] = &["is_project_file", "project_label"];

    let control = HostFunctions::analysis_only(load(&catalog), FactsArtifact::Text(&base)).expect("reads");
    {
        let _scope = control.enter();
        assert!(control.incomplete_functions().is_empty(), "{:?}", control.incomplete_functions());
        let guarded = check_in(&declared, GUARDED, "frontmatter").expect("checked");
        assert_eq!(guarded.category(), ArgumentCategory::Compatible, "control: the guard narrows");
        assert_eq!(check_in(&declared, GUARDED, "is_project_file").expect("checked").category(), ArgumentCategory::Compatible);
        assert_eq!(check_in(&declared, LABEL, "project_label").expect("checked").category(), ArgumentCategory::Incompatible);
        assert!(incomplete_host_calls(&parse_spanned(GUARDED).unwrap()).is_empty());
    }

    let cells: Vec<(&str, Edit, Expect, &str)> = vec![
        ("absent file", Edit::Source(FactsArtifact::Absent), Expect::AllIncomplete, "is absent"),
        ("unreadable file", Edit::Source(FactsArtifact::Unreadable("permission denied")), Expect::AllIncomplete, "cannot be read"),
        ("malformed JSON", Edit::Text(|text| text[..text.len() / 2].to_string()), Expect::AllIncomplete, "not valid JSON"),
        (
            "duplicate top-level key",
            Edit::Text(|text| text.replacen("\"catalog_hash\"", "\"format_version\": 1,\n  \"catalog_hash\"", 1)),
            Expect::AllIncomplete,
            "duplicate key `format_version`",
        ),
        (
            "duplicate nested key",
            Edit::Text(|text| text.replacen("\"lazy\": false", "\"lazy\": false,\n      \"lazy\": false", 1)),
            Expect::AllIncomplete,
            "duplicate key `lazy`",
        ),
        ("trailing content", Edit::Text(|text| format!("{text}{{}}\n")), Expect::AllIncomplete, "after its JSON value"),
        ("not an object", Edit::Json(|root| *root = json!([1])), Expect::AllIncomplete, "an array instead of an object"),
        ("unknown format_version", Edit::Json(|root| root["format_version"] = json!(2)), Expect::AllIncomplete, "unknown `format_version` 2"),
        ("format_version wrong type", Edit::Json(|root| root["format_version"] = json!("1")), Expect::AllIncomplete, "a string instead of an integer"),
        ("hash mismatch", Edit::Json(|root| root["catalog_hash"] = json!("0000")), Expect::AllIncomplete, "records catalog hash `0000`"),
        ("catalog_hash wrong type", Edit::Json(|root| root["catalog_hash"] = json!(7)), Expect::AllIncomplete, "a number instead of a string"),
        ("absent format_version", Edit::Json(|root| drop(root.as_object_mut().unwrap().remove("format_version"))), Expect::AllIncomplete, "has no `format_version` field"),
        ("absent catalog_hash", Edit::Json(|root| drop(root.as_object_mut().unwrap().remove("catalog_hash"))), Expect::AllIncomplete, "has no `catalog_hash` field"),
        ("absent functions", Edit::Json(|root| drop(root.as_object_mut().unwrap().remove("functions"))), Expect::AllIncomplete, "has no `functions` field"),
        ("null format_version", Edit::Json(|root| root["format_version"] = Value::Null), Expect::AllIncomplete, "has `format_version: null`"),
        ("null catalog_hash", Edit::Json(|root| root["catalog_hash"] = Value::Null), Expect::AllIncomplete, "has `catalog_hash: null`"),
        ("null functions", Edit::Json(|root| root["functions"] = Value::Null), Expect::AllIncomplete, "has `functions: null`"),
        ("functions wrong type", Edit::Json(|root| root["functions"] = json!(3)), Expect::AllIncomplete, "a number instead of an object"),
        ("unknown top-level field", Edit::Json(|root| root["extra"] = json!(true)), Expect::AllIncomplete, "unknown field `extra`"),
        ("empty functions", Edit::Json(|root| root["functions"] = json!({})), Expect::AllIncomplete, "has no entry for it"),
        (
            "entry for an undeclared function",
            Edit::Json(|root| root["functions"]["stray"] = root["functions"]["project_label"].clone()),
            Expect::AllIncomplete,
            "entry for `stray`, which the catalog does not declare",
        ),
        (
            "entry missing",
            Edit::Json(|root| drop(root["functions"].as_object_mut().unwrap().remove("is_project_file"))),
            Expect::Incomplete(&["is_project_file"]),
            "has no entry for it",
        ),
        ("entry wrong type", Edit::Json(|root| root["functions"]["is_project_file"] = json!(3)), Expect::Incomplete(&["is_project_file"]), "a number instead of an object"),
        ("entry null", Edit::Json(|root| root["functions"]["is_project_file"] = Value::Null), Expect::Incomplete(&["is_project_file"]), "null instead of an object"),
        ("absent facts", Edit::Json(|root| drop(function_entry(root, "is_project_file").remove("facts"))), Expect::Incomplete(&["is_project_file"]), "has no `facts` field"),
        ("absent aliases", Edit::Json(|root| drop(function_entry(root, "is_project_file").remove("aliases"))), Expect::Incomplete(&["is_project_file"]), "has no `aliases` field"),
        ("absent lazy", Edit::Json(|root| drop(function_entry(root, "is_project_file").remove("lazy"))), Expect::Incomplete(&["is_project_file"]), "has no `lazy` field"),
        (
            "absent conversion_recovery",
            Edit::Json(|root| drop(function_entry(root, "is_project_file").remove("conversion_recovery"))),
            Expect::Incomplete(&["is_project_file"]),
            "has no `conversion_recovery` field",
        ),
        (
            "absent source_exclusions",
            Edit::Json(|root| drop(function_entry(root, "project_label").remove("source_exclusions"))),
            Expect::Incomplete(&["project_label"]),
            "has no `source_exclusions` field",
        ),
        (
            "absent malformed_reference",
            Edit::Json(|root| drop(function_entry(root, "is_project_file").remove("malformed_reference"))),
            Expect::Incomplete(&["is_project_file"]),
            "has no `malformed_reference` field",
        ),
        ("null facts", Edit::Json(|root| root["functions"]["is_project_file"]["facts"] = Value::Null), Expect::Incomplete(&["is_project_file"]), "has `facts: null`"),
        ("null aliases", Edit::Json(|root| root["functions"]["is_project_file"]["aliases"] = Value::Null), Expect::Incomplete(&["is_project_file"]), "has `aliases: null`"),
        ("null lazy", Edit::Json(|root| root["functions"]["is_project_file"]["lazy"] = Value::Null), Expect::Incomplete(&["is_project_file"]), "has `lazy: null`"),
        (
            "null source_exclusions",
            Edit::Json(|root| root["functions"]["project_label"]["source_exclusions"] = Value::Null),
            Expect::Incomplete(&["project_label"]),
            "has `source_exclusions: null`",
        ),
        ("facts wrong type", Edit::Json(|root| root["functions"]["is_project_file"]["facts"] = json!(3)), Expect::Incomplete(&["is_project_file"]), "`facts` a number instead of an array"),
        ("lazy wrong type", Edit::Json(|root| root["functions"]["is_project_file"]["lazy"] = json!("no")), Expect::Incomplete(&["is_project_file"]), "`lazy` a string instead of a boolean"),
        ("lazy true", Edit::Json(|root| root["functions"]["is_project_file"]["lazy"] = json!(true)), Expect::Incomplete(&["is_project_file"]), "evaluated eagerly"),
        (
            "conversion_recovery wrong type",
            Edit::Json(|root| root["functions"]["is_project_file"]["conversion_recovery"] = json!(3)),
            Expect::Incomplete(&["is_project_file"]),
            "`conversion_recovery` a number instead of an object",
        ),
        (
            "malformed_reference wrong type",
            Edit::Json(|root| root["functions"]["is_project_file"]["malformed_reference"] = json!("value")),
            Expect::Incomplete(&["is_project_file"]),
            "`malformed_reference` a string instead of an object",
        ),
        (
            "one fact element of the wrong type",
            Edit::Json(|root| root["functions"]["is_project_file"]["facts"].as_array_mut().unwrap().push(json!(123))),
            Expect::Incomplete(&["is_project_file"]),
            "`facts[1]` a number instead of an object",
        ),
        (
            "every fact element of the wrong type",
            Edit::Json(|root| root["functions"]["is_project_file"]["facts"] = json!([123, "file"])),
            Expect::Incomplete(&["is_project_file"]),
            "`facts[0]` a number instead of an object",
        ),
        (
            "one exclusion element of the wrong type",
            Edit::Json(|root| root["functions"]["project_label"]["source_exclusions"].as_array_mut().unwrap().push(json!(null))),
            Expect::Incomplete(&["project_label"]),
            "`source_exclusions[1]` null instead of an object",
        ),
        (
            "null fact field",
            Edit::Json(|root| root["functions"]["is_project_file"]["facts"][0]["type"] = Value::Null),
            Expect::Incomplete(&["is_project_file"]),
            "has `facts[0].type: null`",
        ),
        (
            "absent fact field",
            Edit::Json(|root| drop(root["functions"]["is_project_file"]["facts"][0].as_object_mut().unwrap().remove("kind"))),
            Expect::Incomplete(&["is_project_file"]),
            "has no `facts[0].kind` field",
        ),
        (
            "unknown fact kind",
            Edit::Json(|root| root["functions"]["is_project_file"]["facts"][0]["kind"] = json!("maybe")),
            Expect::Incomplete(&["is_project_file"]),
            "`facts[0].kind` `maybe`",
        ),
        (
            "fact on a nonexistent parameter",
            Edit::Json(|root| root["functions"]["is_project_file"]["facts"][0]["parameter"] = json!("other")),
            Expect::Incomplete(&["is_project_file"]),
            "differs from the one the catalog declares",
        ),
        (
            "fact at the wrong position",
            Edit::Json(|root| root["functions"]["is_project_file"]["facts"][0]["position"] = json!(1)),
            Expect::Incomplete(&["is_project_file"]),
            "`facts[0].position` 1",
        ),
        (
            "unknown function field",
            Edit::Json(|root| root["functions"]["is_project_file"]["extra"] = json!(1)),
            Expect::Incomplete(&["is_project_file"]),
            "unknown field `extra`",
        ),
        (
            "facts element wrong position type",
            Edit::Json(|root| root["functions"]["is_project_file"]["facts"][0]["position"] = json!("0")),
            Expect::Incomplete(&["is_project_file"]),
            "has `facts[0].position` a string instead of a non-negative integer",
        ),
        (
            "unknown fact field",
            Edit::Json(|root| root["functions"]["is_project_file"]["facts"][0]["extra"] = json!(1)),
            Expect::Incomplete(&["is_project_file"]),
            "`facts[0]` has unknown field `extra`",
        ),
        (
            "more than one fact",
            Edit::Json(|root| {
                let fact = root["functions"]["is_project_file"]["facts"][0].clone();
                root["functions"]["is_project_file"]["facts"].as_array_mut().unwrap().push(fact);
            }),
            Expect::Incomplete(&["is_project_file"]),
            "lists 2 facts; a function registers at most one",
        ),
        (
            "aliases control",
            Edit::Json(|root| root["functions"]["is_project_file"]["aliases"] = json!(["ipf"])),
            Expect::Complete(|functions| {
                let registration = functions.registration("IPF").expect("the alias resolves");
                assert_eq!(registration.name(), "is_project_file");
            }),
            "",
        ),
        ("aliases wrong type", Edit::Json(|root| root["functions"]["is_project_file"]["aliases"] = json!("ipf")), Expect::Incomplete(&["is_project_file"]), "`aliases` a string instead of an array"),
        (
            "one alias element of the wrong type",
            Edit::Json(|root| root["functions"]["is_project_file"]["aliases"] = json!(["ipf", 3])),
            Expect::Incomplete(&["is_project_file"]),
            "`aliases[1]` a number instead of a string",
        ),
        (
            "source_exclusions wrong type",
            Edit::Json(|root| root["functions"]["project_label"]["source_exclusions"] = json!({})),
            Expect::Incomplete(&["project_label"]),
            "`source_exclusions` an object instead of an array",
        ),
        (
            "exclusion parameter wrong type",
            Edit::Json(|root| root["functions"]["project_label"]["source_exclusions"][0]["parameter"] = json!(3)),
            Expect::Incomplete(&["project_label"]),
            "has `source_exclusions[0].parameter` a number instead of a string",
        ),
        (
            "exclusion on a nonexistent parameter",
            Edit::Json(|root| root["functions"]["project_label"]["source_exclusions"][0]["parameter"] = json!("other")),
            Expect::Incomplete(&["project_label"]),
            "does not register",
        ),
        (
            "absent exclusion kinds",
            Edit::Json(|root| drop(root["functions"]["project_label"]["source_exclusions"][0].as_object_mut().unwrap().remove("kinds"))),
            Expect::Incomplete(&["project_label"]),
            "has no `source_exclusions[0].kinds` field",
        ),
        (
            "exclusion kinds wrong type",
            Edit::Json(|root| root["functions"]["project_label"]["source_exclusions"][0]["kinds"] = json!("number")),
            Expect::Incomplete(&["project_label"]),
            "has `source_exclusions[0].kinds` a string instead of an array",
        ),
        (
            "empty exclusion kinds",
            Edit::Json(|root| root["functions"]["project_label"]["source_exclusions"][0]["kinds"] = json!([])),
            Expect::Incomplete(&["project_label"]),
            "has an empty `source_exclusions[0].kinds`",
        ),
        (
            "unknown exclusion kind",
            Edit::Json(|root| root["functions"]["project_label"]["source_exclusions"][0]["kinds"] = json!(["date"])),
            Expect::Incomplete(&["project_label"]),
            "has `source_exclusions[0].kinds[0]` `date`",
        ),
        (
            "unknown exclusion field",
            Edit::Json(|root| root["functions"]["project_label"]["source_exclusions"][0]["extra"] = json!(1)),
            Expect::Incomplete(&["project_label"]),
            "`source_exclusions[0]` has unknown field `extra`",
        ),
        (
            "malformed_reference control",
            Edit::Json(|root| root["functions"]["is_project_file"]["malformed_reference"] = json!({"parameter": "value", "position": 0})),
            Expect::Complete(|functions| {
                let registration = functions.registration("is_project_file").expect("complete");
                assert_eq!(registration.malformed_reference_parameter(), Some(0));
            }),
            "",
        ),
        (
            "malformed_reference parameter wrong type",
            Edit::Json(|root| root["functions"]["is_project_file"]["malformed_reference"] = json!({"parameter": 0, "position": 0})),
            Expect::Incomplete(&["is_project_file"]),
            "has `malformed_reference.parameter` a number instead of a string",
        ),
        (
            "malformed_reference on a nonexistent parameter",
            Edit::Json(|root| root["functions"]["is_project_file"]["malformed_reference"] = json!({"parameter": "other", "position": 0})),
            Expect::Incomplete(&["is_project_file"]),
            "does not register",
        ),
        (
            "absent malformed_reference position",
            Edit::Json(|root| root["functions"]["is_project_file"]["malformed_reference"] = json!({"parameter": "value"})),
            Expect::Incomplete(&["is_project_file"]),
            "has no `malformed_reference.position` field",
        ),
        (
            "malformed_reference position wrong type",
            Edit::Json(|root| root["functions"]["is_project_file"]["malformed_reference"] = json!({"parameter": "value", "position": "0"})),
            Expect::Incomplete(&["is_project_file"]),
            "has `malformed_reference.position` a string instead of a non-negative integer",
        ),
        (
            "malformed_reference at the wrong position",
            Edit::Json(|root| root["functions"]["is_project_file"]["malformed_reference"] = json!({"parameter": "value", "position": 1})),
            Expect::Incomplete(&["is_project_file"]),
            "records `malformed_reference.position` 1, but `value` is at position 0",
        ),
        (
            "unknown malformed_reference field",
            Edit::Json(|root| {
                root["functions"]["is_project_file"]["malformed_reference"] = json!({"parameter": "value", "position": 0, "extra": 1});
            }),
            Expect::Incomplete(&["is_project_file"]),
            "`malformed_reference` has unknown field `extra`",
        ),
        ("empty facts", Edit::Json(|root| root["functions"]["is_project_file"]["facts"] = json!([])), Expect::Incomplete(&["is_project_file"]), "records no fact, but the catalog declares one"),
        (
            "a fact the catalog does not declare",
            Edit::Json(|root| root["functions"]["project_label"]["facts"] = json!([{"kind": "true_implies", "parameter": "name", "position": 0, "type": "string"}])),
            Expect::Incomplete(&["project_label"]),
            "but the catalog declares none",
        ),
        (
            "wrong fact kind",
            Edit::Json(|root| root["functions"]["is_project_file"]["facts"][0]["kind"] = json!("true_iff")),
            Expect::Incomplete(&["is_project_file"]),
            "differs from the one the catalog declares",
        ),
        (
            "wrong fact type spelling",
            Edit::Json(|root| root["functions"]["is_project_file"]["facts"][0]["type"] = json!("string")),
            Expect::Incomplete(&["is_project_file"]),
            "differs from the one the catalog declares",
        ),
        (
            "non-null spelling for a declared type",
            Edit::Json(|root| root["functions"]["is_project_file"]["facts"][0]["type"] = json!("<non-null>")),
            Expect::Incomplete(&["is_project_file"]),
            "differs from the one the catalog declares",
        ),
        (
            "empty source_exclusions",
            Edit::Json(|root| root["functions"]["project_label"]["source_exclusions"] = json!([])),
            Expect::NoExclusions,
            "",
        ),
    ];

    let mut walked = Vec::new();
    for (cell, edit, expect, reason) in cells {
        let edited;
        let artifact = match edit {
            Edit::Source(source) => source,
            Edit::Text(edit) => {
                edited = edit(&base);
                FactsArtifact::Text(&edited)
            }
            Edit::Json(edit) => {
                let mut root: Value = serde_json::from_str(&base).expect("the committed artifact is JSON");
                edit(&mut root);
                edited = serde_json::to_string_pretty(&root).unwrap();
                FactsArtifact::Text(&edited)
            }
        };
        let functions = HostFunctions::analysis_only(load(&catalog), artifact).expect("the catalog is valid");
        let _scope = functions.enter();
        let incomplete: Vec<&str> = functions.incomplete_functions().iter().map(|(name, _)| *name).collect();
        let expected: &[&str] = match expect {
            Expect::AllIncomplete => both,
            Expect::Incomplete(names) => names,
            Expect::NoExclusions | Expect::Complete(_) => &[],
        };
        if let Expect::Complete(check) = expect {
            check(&functions);
        }
        assert_eq!(incomplete, expected, "{cell}: incomplete functions");
        for (name, found) in functions.incomplete_functions() {
            assert!(found.contains(reason), "{cell}: `{name}`'s reason {found:?} lacks {reason:?}");
        }

        let file_incomplete = expected.contains(&"is_project_file");
        let label_incomplete = expected.contains(&"project_label");
        let guarded = parse_spanned(GUARDED).unwrap();
        let narrowed = check_in(&declared, GUARDED, "frontmatter").expect("frontmatter is a built-in").category();
        let narrows = !file_incomplete;
        let category = if narrows { ArgumentCategory::Compatible } else { ArgumentCategory::Unknown };
        assert_eq!(narrowed, category, "{cell}: the guarded `frontmatter` call");
        assert_eq!(check_in(&declared, GUARDED, "is_project_file").is_none(), file_incomplete, "{cell}: is_project_file check");
        let label = check_in(&declared, LABEL, "project_label");
        match (label_incomplete, expect) {
            (true, _) => assert!(label.is_none(), "{cell}: an incomplete function has no category"),
            (false, Expect::NoExclusions) => {
                assert_ne!(label.expect("checked").category(), ArgumentCategory::Incompatible, "{cell}")
            }
            (false, _) => assert_eq!(label.expect("checked").category(), ArgumentCategory::Incompatible, "{cell}"),
        }
        let mut reported: Vec<String> = incomplete_host_calls(&guarded)
            .into_iter()
            .chain(incomplete_host_calls(&parse_spanned(LABEL).unwrap()))
            .map(|call| call.function)
            .collect();
        reported.sort();
        assert_eq!(reported, expected, "{cell}: calls reported as incomplete");
        for expression in [GUARDED, LABEL] {
            let unknown = validate_expression(&parse_spanned(expression).unwrap(), &BindingView::baseline())
                .into_iter()
                .any(|(_, error)| matches!(error, ExpressionError::UnknownFunction { .. }));
            assert!(!unknown, "{cell}: `{expression}` names no unknown function");
        }
        walked.push(cell);
    }
    assert_eq!(walked.len(), 75, "every cell ran: {walked:?}");
    assert_eq!(entries(), (0, 0), "analysis never enters a host handler");

    conversion_recovery_cells();
}

/// One `conversion_recovery` cell: its name, its edit of the generated
/// artifact, the recovery rule a complete read registers, and the reason an
/// incomplete read gives (empty when the read is complete).
type RecoveryCell = (&'static str, fn(&mut Value), Option<(&'static str, &'static str)>, &'static str);

/// The `conversion_recovery` rows of the matrix. The test catalog has no
/// function a recovery can apply to, so these cells read an artifact the
/// writer generates for `project_round`, whose `fallback` recovers `x`.
fn conversion_recovery_cells() {
    fn recovery(root: &mut Value) -> &mut Value {
        &mut root["functions"]["project_round"]["conversion_recovery"]
    }
    const ROUND_CATALOG: &str = concat!(
        "kind: schema\n",
        "categories:\n",
        "    - Project -> Ask about the test host's project.\n",
        "types:\n",
        "    project_round: |-\n",
        "        function(\n",
        "            parameters(x: number, fallback?: number);\n",
        "            returns(number);\n",
        "            category(Project);\n",
        "        ) -> Rounds `x`, or `fallback` when `x` is not a number.\n",
    );
    let round_catalog = || HostFunctionCatalog::load(ROUND_CATALOG, "project_round.yaml").expect("the round catalog loads");
    let base = HostFunctions::builder(round_catalog())
        .function(
            "project_round",
            &[],
            |registration| registration.numeric_recovery("x", "fallback"),
            Arc::new(|_| Ok(json!(0))),
        )
        .build()
        .expect("the round host registers")
        .facts_artifact();

    let cells: Vec<RecoveryCell> = vec![
        ("conversion_recovery control", |_| {}, Some(("x", "fallback")), ""),
        ("explicit null conversion_recovery", |root| *recovery(root) = Value::Null, None, ""),
        (
            "absent conversion_recovery subject",
            |root| drop(recovery(root).as_object_mut().unwrap().remove("subject")),
            None,
            "has no `conversion_recovery.subject` field",
        ),
        (
            "conversion_recovery subject wrong type",
            |root| recovery(root)["subject"] = json!(0),
            None,
            "has `conversion_recovery.subject` a number instead of a string",
        ),
        (
            "conversion_recovery subject on a nonexistent parameter",
            |root| recovery(root)["subject"] = json!("other"),
            None,
            "does not register",
        ),
        (
            "null conversion_recovery fallback",
            |root| recovery(root)["fallback"] = Value::Null,
            None,
            "has `conversion_recovery.fallback: null`",
        ),
        (
            "unknown conversion_recovery field",
            |root| recovery(root)["extra"] = json!(1),
            None,
            "`conversion_recovery` has unknown field `extra`",
        ),
    ];
    for (cell, edit, rule, reason) in cells {
        let mut root: Value = serde_json::from_str(&base).expect("the generated artifact is JSON");
        edit(&mut root);
        let edited = serde_json::to_string_pretty(&root).unwrap();
        let functions =
            HostFunctions::analysis_only(round_catalog(), FactsArtifact::Text(&edited)).expect("the catalog is valid");
        let incomplete = functions.incomplete_functions();
        if reason.is_empty() {
            assert!(incomplete.is_empty(), "{cell}: {incomplete:?}");
            let registration = functions.registration("project_round").expect("complete");
            let recovered = registration.numeric_recovery().map(|recovery| (recovery.subject.as_str(), recovery.fallback.as_str()));
            assert_eq!(recovered, rule, "{cell}: the recovery rule");
        } else {
            assert_eq!(incomplete.len(), 1, "{cell}: {incomplete:?}");
            let (name, found) = incomplete[0];
            assert_eq!(name, "project_round", "{cell}");
            assert!(found.contains(reason), "{cell}: the reason {found:?} lacks {reason:?}");
            assert!(functions.registration("project_round").is_none(), "{cell}: an incomplete function registers nothing");
        }
    }
}

// ── composition and runtime parity ─────────────────────────────────────────

fn compose_in(dir: &Path, content: &str) -> Result<Markdown, darkmatter::markdown::MarkdownError> {
    let path = dir.join("doc.md");
    std::fs::write(&path, content).unwrap();
    let options =
        ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir, "")).with_source_file(path.clone());
    Markdown::try_from(path.as_path())
        .unwrap()
        .compose_with(&crate::request_support::request_at(dir, options))
        .map(|(composed, _)| composed)
}

/// Criteria 17 and 37, composition side: an installed host function is
/// checked, narrowed, rejected, and run exactly like a built-in, and only an
/// evaluated call enters its handler.
#[test]
fn installed_host_functions_get_the_built_in_checks_and_run_only_when_evaluated() {
    let catalog = std::fs::read_to_string(biscuit_test_harness::manifest_dir!().join("tests/fixtures/host_functions/project.yaml"))
        .expect("the test catalog");
    install_host_functions(test_host(&catalog)).expect("installs");

    let schema = schema("source: unknown\n");
    let declared = DeclaredTypes::new(Some(&schema));
    assert_eq!(check_in(&declared, GUARDED, "frontmatter").unwrap().category(), ArgumentCategory::Compatible);
    assert_eq!(check_in(&declared, UNGUARDED, "frontmatter").unwrap().category(), ArgumentCategory::Unknown);
    assert_eq!(check_in(&declared, LABEL, "project_label").unwrap().category(), ArgumentCategory::Incompatible);
    assert_eq!(entries(), (0, 0), "analysis entered no handler");

    let dir = TempDir::new().unwrap();
    let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
    let rejected = compose_in(&root, "---\ntitle: t\n---\n{{ is_project_file('a.md') }} {{ project_label(42) }}\n")
        .expect_err("a category 5 host call rejects the document");
    let check = rejected.rejected_call().unwrap_or_else(|| panic!("rejected without a call check: {rejected}"));
    assert_eq!(check.function, "project_label");
    assert_eq!(entries(), (0, 0), "a rejected document runs nothing");

    std::fs::write(root.join("note.md"), "---\ntitle: The Note\n---\nBody\n").unwrap();
    let composed = compose_in(
        &root,
        "---\n$schema:\n  source: unknown\nsource: note.md\n---\n\
         {{ is_project_file(source) ? frontmatter(source, 'title') : 'none' }} {{ false ? project_label('x') : 'skipped' }}\n",
    )
    .unwrap_or_else(|error| panic!("composes: {error}"));
    assert!(composed.content().contains("The Note skipped"), "{}", composed.content());
    assert_eq!(entries(), (1, 0), "only the evaluated host call entered its handler");
}

/// A function catalog in a schema root is discovered for its role only: it
/// is not a global catalog, contributes no document property, and is no
/// trigger.
#[test]
fn a_function_catalog_in_a_schema_root_is_never_a_document_schema() {
    use darkmatter::markdown::schemas::catalog::{
        host_catalogs, host_function_catalogs, is_global_catalog, is_host_function_catalog,
    };
    let catalog = std::fs::read_to_string(biscuit_test_harness::manifest_dir!().join("tests/fixtures/host_functions/project.yaml"))
        .expect("the test catalog");
    let artifact = std::fs::read_to_string(biscuit_test_harness::manifest_dir!().join("tests/fixtures/host_functions/project.facts.json"))
        .expect("the committed artifact");
    assert!(is_host_function_catalog(&catalog) && !is_global_catalog(&catalog));
    assert!(!is_host_function_catalog("kind: schema\n$schema:\n  seq: object\n"), "a global catalog");

    let dir = TempDir::new().unwrap();
    let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
    std::fs::create_dir_all(root.join(".git")).unwrap();
    std::fs::create_dir_all(root.join("schemas")).unwrap();
    std::fs::write(root.join("schemas/project.yaml"), &catalog).unwrap();
    std::fs::write(root.join("schemas/project.facts.json"), &artifact).unwrap();
    let context =
        biscuit_file::FileResolutionContext::from_snapshot(&root, None, Default::default()).with_repository_root(&root);

    let found = host_function_catalogs(&context);
    assert_eq!(found.len(), 1, "{found:?}");
    let functions = found[0].as_ref().expect("the catalog loads");
    assert!(functions.incomplete_functions().is_empty(), "{:?}", functions.incomplete_functions());
    assert!(host_catalogs(&context).is_empty(), "a function catalog is not a global catalog");

    let api = darkmatter::markdown::schemas::DarkmatterSchemas::new(context)
        .with_darkmatter_baseline_json_schema()
        .expect("baseline")
        .with_trigger_discovery()
        .expect("the function catalog is not a trigger");
    let doc = root.join("doc.md");
    std::fs::write(&doc, "---\nis_project_file: 3\n---\nBody\n").unwrap();
    let markdown = Markdown::try_from(doc.as_path())
        .unwrap()
        .with_source(darkmatter::markdown::compose::ComposeSource::File(doc));
    let effective = api.effective_for(&markdown).expect("prepares").expect("baseline");
    assert!(
        effective.origins.values().all(|origin| origin.kind == darkmatter::markdown::schemas::SchemaOriginKind::Baseline),
        "only the baseline applies"
    );
    assert!(!effective.declares_top_level_property("is_project_file"));
}

/// Criterion 36 on a host function whose result may itself be null:
/// `project_trim(value: string | null): string | null` handles a missing
/// direct argument statically and at runtime, while the same call nested in
/// another call's argument does not.
#[test]
fn a_host_parameter_declaring_null_handles_a_missing_argument_whatever_it_returns() {
    use darkmatter::markdown::compose::ComposeWarning;
    use darkmatter::markdown::compose::expression::static_variable_reads;

    let catalog = concat!(
        "kind: schema\n",
        "categories:\n",
        "    - Project -> Ask about the test host's project.\n",
        "types:\n",
        "    project_trim: |-\n",
        "        function(\n",
        "            parameters(value: string | null);\n",
        "            returns(string | null);\n",
        "            category(Project);\n",
        "        ) -> Trims text; null passes through.\n",
    );
    let host = HostFunctions::builder(load(catalog))
        .function(
            "project_trim",
            &[],
            |registration| registration,
            Arc::new(|call| Ok(call.args().text(0).map_or(Value::Null, |text| json!(text.trim())))),
        )
        .build()
        .expect("the pass-through host registers");
    install_host_functions(host).expect("installs");

    for (source, handled) in [("project_trim(missing)", true), ("is_empty(project_trim(missing))", false)] {
        let expr = parse_spanned(source).unwrap();
        assert_eq!(static_variable_reads(&expr)[0].handles_absence, handled, "{source}");
    }

    let dir = TempDir::new().unwrap();
    let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
    let path = root.join("doc.md");
    std::fs::write(&path, "---\ntitle: t\n---\n[{{ project_trim(missing) }}] {{ is_empty(project_trim(nested)) }}\n")
        .unwrap();
    let options =
        ComposeOptions::new_with_context(ComposeContext::capture_for_content(&root, "")).with_source_file(path.clone());
    let (composed, report) = Markdown::try_from(path.as_path())
        .unwrap()
        .compose_with(&crate::request_support::request_at(&root, options))
        .unwrap_or_else(|error| panic!("composes: {error}"));
    assert_eq!(composed.content().trim_end(), "[] true");
    let undeclared: Vec<&str> = report
        .warnings
        .iter()
        .filter(|warning| warning.code.as_deref() == Some(ComposeWarning::UNDECLARED_PROPERTY_CODE))
        .map(|warning| warning.message.as_str())
        .collect();
    assert_eq!(undeclared.len(), 1, "{undeclared:#?}");
    assert!(undeclared[0].starts_with("`nested`"), "{undeclared:#?}");
}

/// Criterion 36 on a host function: `is_project_file(value: unknown | null)`
/// handles a missing direct argument exactly as a built-in with that
/// parameter does, statically and at runtime, while a missing argument to
/// `project_label(name: string)` is still flagged.
#[test]
fn a_host_parameter_declaring_null_handles_a_missing_argument() {
    use darkmatter::markdown::compose::ComposeWarning;
    use darkmatter::markdown::compose::expression::static_variable_reads;

    let catalog = std::fs::read_to_string(biscuit_test_harness::manifest_dir!().join("tests/fixtures/host_functions/project.yaml"))
        .expect("the test catalog");
    install_host_functions(test_host(&catalog)).expect("installs");

    for (source, handled) in [("is_project_file(missing)", true), ("project_label(missing)", false)] {
        let expr = parse_spanned(source).unwrap();
        assert_eq!(static_variable_reads(&expr)[0].handles_absence, handled, "{source}");
    }

    let dir = TempDir::new().unwrap();
    let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
    let path = root.join("doc.md");
    std::fs::write(&path, "---\ntitle: t\n---\n{{ is_project_file(missing) }}\n").unwrap();
    let options =
        ComposeOptions::new_with_context(ComposeContext::capture_for_content(&root, "")).with_source_file(path.clone());
    let (composed, report) = Markdown::try_from(path.as_path())
        .unwrap()
        .compose_with(&crate::request_support::request_at(&root, options))
        .unwrap_or_else(|error| panic!("composes: {error}"));
    assert_eq!(composed.content().trim_end(), "false");
    let undeclared: Vec<_> = report
        .warnings
        .iter()
        .filter(|warning| warning.code.as_deref() == Some(ComposeWarning::UNDECLARED_PROPERTY_CODE))
        .collect();
    assert!(undeclared.is_empty(), "{undeclared:#?}");
}
