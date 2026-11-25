//! DMLS serves a callable catalog (`kind: expression-schema`,
//! `kind: mutation-schema`) as a whole YAML document claimed by its kind:
//! completion and hover come from the kind's definition and the standalone
//! schema tooling `kind: schema` documents use, and each failure of the
//! library's callable checks is one `dm.schema.callable_catalog` error ranged
//! where the library located it, with every location it involves as related
//! information. A malformed catalog keeps the loader's located parse error.
//!
//! Every failing case is the shared valid control `md schema validate` uses
//! (`darkmatter/lib/tests/fixtures/host_functions/project.yaml`) with one
//! authored edit. In-process cases use one `Probe` (one overlay cache, so
//! successive texts of `project.yaml` replace each other); the session cases
//! drive a headless LSP server.

use darkmatter::markdown::schemas::CALLABLE_CATALOG_CODE;
use darkmatter::work_counters::{self as library_work, Work};
use dmls::work_counters::{work_counts, yaml_preparation_threads};
use lsp_types::{Diagnostic, Position, Range};
use serde_json::Value;

use crate::common::{LspFixture, LspWorkspace};
use crate::yaml_document_editor::{Probe, codes, write};
use crate::yaml_document_session as session;

/// The valid control, shared with the CLI.
const CATALOG: &str = include_str!("../../../lib/tests/fixtures/host_functions/project.yaml");

/// The built-in expression catalog, as a Rusty Biscuit checkout holds it.
const BUILT_IN_FUNCTIONS: &str = include_str!("../../../schemas/partials/functions.yaml");

/// The control's second callable, replaced whole by the overload case.
const PROJECT_LABEL: &str = "    project_label: |-
        function(
            parameters(name: string);
            returns(string);
            category(Project);
        ) -> Returns the project's label for a name.
";

const NAME: &str = "project.yaml";

/// The range of the first `needle` in `text` at or after byte `from`.
fn range_of(text: &str, needle: &str, from: usize) -> Range {
    let offset = from + text[from..].find(needle).expect("needle present");
    Range::new(position_at(text, offset), position_at(text, offset + needle.len()))
}

/// The `(line, UTF-16 character)` position of byte `offset` in `text`.
fn position_at(text: &str, offset: usize) -> Position {
    let line = text[..offset].matches('\n').count() as u32;
    let line_start = text[..offset].rfind('\n').map_or(0, |newline| newline + 1);
    Position::new(line, text[line_start..offset].encode_utf16().count() as u32)
}

/// The one diagnostic of `diagnostics`, which must be a catalog error.
fn only_catalog_error(diagnostics: &[Diagnostic]) -> &Diagnostic {
    assert_eq!(codes(diagnostics), vec![CALLABLE_CATALOG_CODE], "{diagnostics:#?}");
    &diagnostics[0]
}

/// The `(uri, range, message)` of each related location of `diagnostic`.
fn related(diagnostic: &Diagnostic) -> Vec<(String, Range, String)> {
    diagnostic
        .related_information
        .iter()
        .flatten()
        .map(|info| (info.location.uri.as_str().to_string(), info.location.range, info.message.clone()))
        .collect()
}

fn uri_of(probe: &Probe, name: &str) -> String {
    url::Url::from_file_path(probe.path(name)).unwrap().to_string()
}

fn edited(from: &str, to: &str) -> String {
    assert!(CATALOG.contains(from), "the control holds {from:?}");
    CATALOG.replacen(from, to, 1)
}

// ── Judgment ────────────────────────────────────────────────────────────────

#[test]
fn the_shared_catalog_control_is_validated_by_its_kind_with_no_findings() {
    let probe = Probe::new();
    assert_eq!(probe.outcome(NAME, CATALOG), "validated");
    assert_eq!(probe.schema_diagnostics(NAME, CATALOG), Vec::new());
    let mutations = edited("kind: expression-schema", "kind: mutation-schema").replace("            narrows(value: file; true-implies);\n", "");
    assert_eq!(probe.outcome(NAME, &mutations), "validated");
    assert_eq!(probe.schema_diagnostics(NAME, &mutations), Vec::new());
}

#[test]
fn inline_comments_in_callable_and_tagged_schemas_keep_editor_diagnostics_clean() {
    let probe = Probe::new();
    let expression = format!(
        "{CATALOG}    Label: string # A label.\n    default_label: function(parameters(void); returns(Label@this); category(Project)) # Read a label.\n"
    );
    let mutation = format!(
        "{}    Label: string # A label.\n",
        edited("kind: expression-schema", "kind: mutation-schema").replace("            narrows(value: file; true-implies);\n", "")
    );
    let ordinary = "kind: schema\ntypes:\n    Label: string # The displayed label.\n$schema:\n    label: Label@this\n";
    for (name, text) in [("expressions.yaml", expression.as_str()), ("actions.yaml", &mutation), ("label.yaml", ordinary)] {
        assert_eq!(probe.outcome(name, text), "validated", "{name}");
        assert_eq!(probe.schema_diagnostics(name, text), Vec::new(), "{name}");
    }
}

#[test]
fn type_reference_diagnostics_select_the_second_signature_and_preserve_the_related_key() {
    let probe = Probe::new();
    let first = "function(parameters(Missing: string); returns(string); category(Project))";
    for kind in ["expression-schema", "mutation-schema"] {
        for (reference, decoy, message, is_callable) in [
            ("Missing@this", "Missing", "which this catalog does not declare", false),
            ("Label@./helpers.yaml", "./helpers.yaml", "resolves only its own helper types", false),
            ("show_label@this", "show_label", "uses the callable", true),
        ] {
            let signature = format!("function(parameters(name: number); returns({reference}); category(Project))");
            let source = format!("kind: {kind}\ncategories: [Project]\ntypes:\n    Label: string\n    show_label:\n        - |-\n            {first} -> Mentions {decoy}.\n        - |- # {decoy}\n            {signature}\n");
            let control = source.replace(reference, "Label@this");
            assert_eq!(probe.outcome(NAME, &control), "validated", "{control}");
            assert!(probe.diagnostics(NAME, &control).is_empty(), "{control}");

            assert_eq!(probe.outcome(NAME, &source), "failed", "{source}");
            let diagnostics = probe.diagnostics(NAME, &source);
            let error = only_catalog_error(&diagnostics);
            assert!(error.message.contains(message), "{error:#?}");
            assert_eq!(error.range, range_of(&source, &signature, 0), "{error:#?}");
            let expected = if is_callable {
                vec![(uri_of(&probe, NAME), range_of(&source, "show_label", 0), "`show_label` is declared here".to_string())]
            } else {
                Vec::new()
            };
            assert_eq!(related(error), expected, "{error:#?}");
        }
    }
}

#[test]
fn uncallable_names_receive_one_editor_diagnostic_at_the_authored_key() {
    let probe = Probe::new();
    let valid = edited("project_label:", "\"Project.Label\":");
    assert_eq!(probe.outcome(NAME, &valid), "validated");
    assert!(probe.schema_diagnostics(NAME, &valid).is_empty());

    for key in ["\"project label\"", "\"2fa_label\"", "\"true\""] {
        let text = edited("project_label:", &format!("{key}:"));
        assert_eq!(probe.outcome(NAME, &text), "failed", "{key}");
        let diagnostics = probe.diagnostics(NAME, &text);
        let error = only_catalog_error(&diagnostics);
        assert_eq!(error.range, range_of(&text, key, 0), "{key}: {error:?}");
        assert!(error.message.contains("cannot be used as an expression call name"), "{error:?}");
        assert!(related(error).is_empty(), "{error:?}");
    }
}

#[test]
fn a_folded_name_collision_is_one_catalog_error_related_to_the_earlier_declaration() {
    let probe = Probe::new();
    let text = edited("project_label:", "Is_Project_File:");
    assert_eq!(probe.outcome(NAME, &text), "failed");
    let diagnostics = probe.diagnostics(NAME, &text);
    let error = only_catalog_error(&diagnostics);
    assert_eq!(error.range, range_of(&text, "Is_Project_File", 0), "{error:#?}");
    assert!(error.message.contains("without regard to ASCII case"), "{error:#?}");
    let first = range_of(&text, "is_project_file", 0);
    assert_eq!(
        related(error),
        vec![(uri_of(&probe, NAME), first, "`is_project_file` is declared here".to_string())],
        "{error:#?}"
    );
}

#[test]
fn a_duplicate_overload_is_one_error_with_both_authored_arms() {
    let probe = Probe::new();
    let first = "function(parameters(name: string); returns(string); category(Project))";
    for kind in ["expression-schema", "mutation-schema"] {
        for (carrier, ty) in [("sequence", "string"), ("pipe", "string"), ("sequence", "Label@this"), ("pipe", "Label@this")] {
            let later = format!("function(parameters(other: {ty}); returns(string); category(Project))");
            let decl = if carrier == "sequence" {
                format!("    project_label:\n        - '{first}'\n        - '{later}'\n")
            } else {
                format!("    project_label: >-\n        {first} |\n        {later}\n")
            };
            let text = format!("{}    Label: string\n", edited(PROJECT_LABEL, &decl))
                .replace("kind: expression-schema", &format!("kind: {kind}"))
                .replace("            narrows(value: file; true-implies);\n", "");
            assert_eq!(probe.outcome(NAME, &text), "failed");
            let diagnostics = probe.diagnostics(NAME, &text);
            let error = only_catalog_error(&diagnostics);
            assert!(error.message.contains("overloads 1 and 2"), "{error:#?}");
            assert_eq!(error.range, range_of(&text, &later, 0), "{error:#?}");
            assert_eq!(related(error), vec![(
                uri_of(&probe, NAME),
                range_of(&text, first, 0),
                "the earlier overload is declared here".to_string(),
            )]);
            let control = text.replace(&format!("other: {ty}"), "other: number");
            assert!(probe.diagnostics(NAME, &control).is_empty(), "{carrier} {kind}");
        }
    }
}

#[test]
fn category_failures_have_one_diagnostic_at_the_authored_entry_or_callable() {
    let probe = Probe::new();
    for (entry, rejected, has_earlier) in [
        ("    - Project -> Again.\n", "Project -> Again.", true),
        ("    - ' -> Blank label.'\n", "' -> Blank label.'", false),
        ("    - 'Extra ->'\n", "'Extra ->'", false),
        ("", "function(", false),
    ] {
        let text = if entry.is_empty() {
            edited("category(Project);", "category(Missing);")
        } else {
            edited("types:\n", &format!("{entry}types:\n"))
        };
        let diagnostics = probe.diagnostics(NAME, &text);
        let error = only_catalog_error(&diagnostics);
        assert_eq!(error.range.start, range_of(&text, rejected, 0).start, "{error:#?}");
        if has_earlier {
            let category = text.find("    - Project").unwrap() + "    - ".len();
            assert_eq!(related(error).len(), 1, "{error:#?}");
            assert_eq!(related(error)[0].1.start, position_at(&text, category), "{error:#?}");
        } else {
            assert!(related(error).is_empty(), "{error:#?}");
        }
    }
}

#[test]
fn a_repeated_yaml_key_is_the_loaders_error_at_the_repetition_with_the_first_spelling_related() {
    let probe = Probe::new();
    let text = edited("project_label:", "is_project_file:");
    assert_eq!(probe.outcome(NAME, &text), "load-failed");
    let diagnostics = probe.diagnostics(NAME, &text);
    // The catalog's envelope does not publish its own malformed-document
    // error beside the loader's.
    assert_eq!(codes(&diagnostics), vec!["dm.frontmatter.yaml_parse"], "{diagnostics:#?}");
    let repeated = text.rfind("is_project_file:").unwrap();
    assert_eq!(diagnostics[0].range.start, position_at(&text, repeated), "{diagnostics:#?}");
    let first = text.find("is_project_file:").unwrap();
    assert_eq!(
        related(&diagnostics[0]),
        vec![(uri_of(&probe, NAME), Range::new(position_at(&text, first), position_at(&text, first)), "the key is first written here".to_string())]
    );
}

#[test]
fn a_reused_built_in_name_relates_the_built_in_declaration_under_the_repository_root() {
    let probe = Probe::new();
    let built_in = "darkmatter/schemas/partials/functions.yaml";
    write(&probe.path(built_in), BUILT_IN_FUNCTIONS);
    let text = edited("project_label:", "is_array:");
    let diagnostics = probe.diagnostics(NAME, &text);
    let error = only_catalog_error(&diagnostics);
    assert_eq!(error.range, range_of(&text, "is_array", 0), "{error:#?}");
    let declared = BUILT_IN_FUNCTIONS.find("\n    is_array:").unwrap() + 1;
    assert_eq!(
        related(error).into_iter().map(|(uri, range, _)| (uri, range)).collect::<Vec<_>>(),
        vec![(uri_of(&probe, built_in), range_of(BUILT_IN_FUNCTIONS, "is_array", declared))],
        "{error:#?}"
    );
}

#[test]
fn a_built_in_alias_without_a_checkout_reports_embedded_coordinates_and_links_matching_source() {
    let built_in = "darkmatter/schemas/partials/functions.yaml";
    let text = edited("project_label:", "KEBABCASE:");
    let offset = BUILT_IN_FUNCTIONS.find("\n    kebab_case:").unwrap() + "\n    ".len();
    let at = position_at(BUILT_IN_FUNCTIONS, offset);
    let probe = Probe::without_repository();
    for copy in [None, Some("# Different source.\n"), Some(BUILT_IN_FUNCTIONS), Some("# Changed source.\n"), None] {
        probe.change_schema_buffer(built_in, copy);
        let diagnostics = probe.diagnostics("sub/project.yaml", &text);
        let error = only_catalog_error(&diagnostics);
        assert_eq!(error.range, range_of(&text, "KEBABCASE", 0), "{error:#?}");
        if copy == Some(BUILT_IN_FUNCTIONS) {
            assert_eq!(related(error).len(), 1, "{error:#?}");
            assert_eq!(related(error)[0].0, uri_of(&probe, built_in));
            assert_eq!(related(error)[0].1.start, at);
        } else {
            assert!(related(error).is_empty(), "{error:#?}");
            assert!(error.message.contains(&format!("{built_in}:{}:{}", at.line + 1, at.character + 1)), "{error:#?}");
        }
    }
}

#[test]
fn a_mutation_catalog_refuses_narrowing_at_the_declaration() {
    let probe = Probe::new();
    let text = edited("kind: expression-schema", "kind: mutation-schema");
    let diagnostics = probe.diagnostics(NAME, &text);
    let error = only_catalog_error(&diagnostics);
    assert!(error.message.contains("narrows"), "{error:#?}");
    assert_eq!(error.range.start, position_at(&text, text.find("|-").unwrap()), "{error:#?}");
}

/// A catalog is no import library and imports none: a `Name@file` inside it
/// is refused by its callable checks, and neither the editor's authoring
/// worker nor the library resolves the named file.
#[test]
fn a_catalog_import_is_refused_without_resolving_the_imported_file() {
    let probe = Probe::new();
    write(&probe.path("lib.yaml"), "kind: schema\ntypes:\n    Label: string\n");
    let text = edited("returns(string)", "returns(Label@./lib.yaml)");
    let before = library_work::snapshot();
    let diagnostics = probe.diagnostics(NAME, &text);
    let error = only_catalog_error(&diagnostics);
    let signature = "function(\n            parameters(name: string);\n            returns(Label@./lib.yaml);\n            category(Project);\n        )";
    assert_eq!(error.range, range_of(&text, signature, 0), "{error:#?}");
    let work = library_work::snapshot().since(&before);
    for kind in [Work::StandaloneImportCheck, Work::NamedTypeTableParse] {
        assert_eq!(work.get(kind), 0, "{kind:?} in {:?}", work.nonzero());
    }
    let named_types = probe.with(NAME, &text, |ctx| ctx.overlay.map(|overlay| overlay.named_types.dependencies().count()));
    assert_eq!(named_types, Some(0));
}

/// A catalog that fails its callable checks stays claimed by its kind, so a
/// root `$schema` in it is data, never a fallback contract; a valid one
/// judges such a key an unknown field, as the CLI does.
#[test]
fn a_catalog_keeps_its_kind_claim_and_its_root_schema_is_data() {
    let probe = Probe::new();
    let failed = edited("project_label:", "Is_Project_File:");
    let reference = probe.with(NAME, &failed, |ctx| ctx.overlay.map(|overlay| overlay.schema_key_is_reference()));
    assert_eq!(reference, Some(false), "a failed catalog keeps its kind's claim");

    let text = format!("{CATALOG}$schema: ./missing.yaml\n");
    assert_eq!(probe.outcome(NAME, &text), "validated");
    assert_eq!(codes(&probe.schema_diagnostics(NAME, &text)), vec!["dm.schema.unknown_key"]);
}

// ── Authoring ───────────────────────────────────────────────────────────────

#[test]
fn completion_and_hover_describe_the_catalog_kinds_fields() {
    let probe = Probe::new();
    for kind in ["expression-schema", "mutation-schema"] {
        let text = format!("kind: {kind}\n");
        let labels = probe.completion_labels(NAME, &text, text.len());
        for field in ["description", "categories", "types"] {
            assert!(labels.iter().any(|label| label == field), "{kind}: {field} in {labels:?}");
        }
        assert!(labels.iter().all(|label| label != "$schema" && label != "title"), "{kind}: {labels:?}");
    }
    let text = format!("{CATALOG}\n");
    assert_eq!(probe.completion_labels(NAME, &text, text.len()), vec!["description"]);
    for (field, says) in [("kind", "expression-schema"), ("categories", "category"), ("types", "callables")] {
        let hover = probe.hover(NAME, CATALOG, CATALOG.find(field).unwrap() + 1);
        assert!(hover.as_deref().is_some_and(|body| body.contains(says)), "{field}: {hover:?}");
    }
    let described = edited("categories:", "description: the test host's project functions\ncategories:");
    let hover = probe.hover(NAME, &described, described.find("description").unwrap() + 1);
    assert!(hover.as_deref().is_some_and(|body| body.contains("**`description`**")), "{hover:?}");
}

/// A catalog's local types are authored with the standalone schema tooling
/// `kind: schema` uses: the type grammar inside a helper declaration, and the
/// helper's hover.
#[test]
fn a_catalogs_local_types_are_authored_as_in_a_schema_document() {
    let probe = Probe::new();
    let catalog = edited("types:\n", "types:\n    Label: string -> a label\n").replace("returns(string)", "returns(Label)");
    assert_eq!(probe.schema_diagnostics(NAME, &catalog), Vec::new());
    let standalone = probe.with(NAME, &catalog, |ctx| ctx.overlay.map(|overlay| overlay.is_standalone_schema()));
    assert_eq!(standalone, Some(true));

    let hover = probe.hover(NAME, &catalog, catalog.find("Label:").unwrap() + 1);
    assert!(hover.as_deref().is_some_and(|body| body.contains("**`Label`**") && body.contains("string")), "{hover:?}");
    let partial = catalog.replace("    Label: string -> a label\n", "    Label: str\n");
    let labels = probe.completion_labels(NAME, &partial, partial.find(": str\n").unwrap() + 5);
    assert!(labels.iter().any(|label| label == "string"), "{labels:?}");

    // The same buffer as a `kind: schema` document is served the same way.
    let schema = catalog.replace("kind: expression-schema", "kind: schema");
    assert_eq!(probe.hover("schema.yaml", &schema, schema.find("Label:").unwrap() + 1), hover);
}

// ── Sessions ────────────────────────────────────────────────────────────────

/// The `dm.schema.callable_catalog` diagnostics among `diagnostics`.
fn catalog_errors(diagnostics: &[Value]) -> Vec<&Value> {
    diagnostics.iter().filter(|diagnostic| diagnostic["code"] == CALLABLE_CATALOG_CODE).collect()
}

/// Valid, invalid, and valid again with the same `kind`: each buffer version
/// is prepared on the registry worker (never the dispatch thread) and
/// published with its own findings, never a cached result of another text.
/// Repeated reads of the installed revision then prepare nothing, read
/// nothing, and queue no registry job.
#[test]
fn catalog_findings_follow_the_buffer_version_and_repeated_reads_prepare_nothing() {
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    session::init_repository(root);
    let path = root.join(NAME);
    write(&path, CATALOG);
    let uri = session::uri(&path);
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(session::watched_params(&[root]));
    let before = library_work::snapshot();

    session::open(&fixture, &uri, CATALOG);
    assert_eq!(catalog_errors(&session::latest_diagnostics(&mut fixture, &uri)), Vec::<&Value>::new());
    let collided = edited("project_label:", "Is_Project_File:");
    session::edit(&fixture, &uri, 2, &collided);
    let diagnostics = session::latest_diagnostics(&mut fixture, &uri);
    let errors = catalog_errors(&diagnostics);
    assert_eq!(errors.len(), 1, "{diagnostics:#?}");
    let line = position_at(&collided, collided.find("Is_Project_File").unwrap()).line;
    assert_eq!(errors[0]["range"]["start"]["line"], line, "{diagnostics:#?}");
    assert_eq!(errors[0]["relatedInformation"][0]["location"]["uri"], uri.as_str(), "{diagnostics:#?}");
    session::edit(&fixture, &uri, 3, CATALOG);
    assert_eq!(catalog_errors(&session::latest_diagnostics(&mut fixture, &uri)), Vec::<&Value>::new());

    let prepared = library_work::snapshot().since(&before);
    assert!(prepared.get(Work::YamlContractPreparation) >= 3, "one preparation per text: {:?}", prepared.nonzero());
    let dispatch = fixture.server_thread_id();
    let threads = library_work::threads(Work::YamlContractPreparation);
    assert!(!threads.is_empty() && !threads.contains(&dispatch), "{threads:?}");
    let threads = yaml_preparation_threads();
    assert!(!threads.is_empty() && !threads.contains(&dispatch), "{threads:?}");

    fixture.registry_jobs_until_idle();
    let (dmls_before, library_before) = (work_counts(), library_work::snapshot());
    for _ in 0..2 {
        assert!(session::hover(&mut fixture, &uri, 0, 1).contains("expression-schema"));
        session::completion_labels(&mut fixture, &uri, 0);
    }
    fixture.flush_server();
    assert_eq!(fixture.registry_jobs_until_idle(), Vec::new(), "no registry job for an installed revision");
    let dmls_work = work_counts().since(dmls_before);
    assert_eq!((dmls_work.reads, dmls_work.yaml_preparations, dmls_work.registry_scans), (0, 0, 0), "{dmls_work:?}");
    let library = library_work::snapshot().since(&library_before);
    for work in [Work::YamlContractPreparation, Work::YamlKindCheck, Work::ValidatorCompile, Work::EffectivePreparation] {
        assert_eq!(library.get(work), 0, "{work:?} in {:?}", library.nonzero());
    }
    fixture.shutdown();
}

#[test]
fn aliased_sequence_arms_keep_editor_admission_and_related_ranges() {
    let probe = Probe::new();
    let string = "function(parameters(x: string); returns(string); category(Project))";
    let number = "function(parameters(x: number); returns(string); category(Project))";
    let resolved = string.replace("x: string", "x: Label@this");
    let category = string.replace("category(Project)", "category(Extra)");
    let missing = number.replace("returns(string)", "returns(Missing@this)");
    for kind in ["expression-schema", "mutation-schema"] {
        let text = |signature: &str, arms: &str| {
            format!("kind: {kind}\ndescription: &other '{signature}'\ncategories: [Project, Extra]\ntypes:\n    Label: string\n    project_label:{arms}\n")
        };
        for control in [
            text(number, &format!(" ['{number}', '{string}']")),
            text(number, &format!(" [&arm !!str '{string}', *other]")),
        ] {
            assert_eq!(probe.outcome(NAME, &control), "validated", "{control}");
            assert!(probe.diagnostics(NAME, &control).is_empty(), "{control}");
        }
        for (source, primary, earlier, message) in [
            (text(number, &format!("\n        - &arm |- \n            {string}\n        - *arm")), "*arm", Some(string), "same binding contract"),
            (text(string, &format!(" [*other, '{resolved}']")), resolved.as_str(), Some("*other"), "same binding contract"),
            (text(number, &format!(" [*other, '{category}']")), &format!("'{category}'"), Some("*other"), "different categories"),
            (text(&missing, &format!(" ['{string}', *other]")), "*other", None, "which this catalog does not declare"),
        ] {
            let arms = source.find("    project_label:").unwrap();
            assert_eq!(probe.outcome(NAME, &source), "failed", "{source}");
            let diagnostics = probe.diagnostics(NAME, &source);
            let error = only_catalog_error(&diagnostics);
            assert_eq!(error.range, range_of(&source, primary, arms), "{error:#?}");
            assert!(error.message.contains(message), "{error:#?}");
            let notes = related(error);
            assert_eq!(notes.len(), usize::from(earlier.is_some()), "{error:#?}");
            if let Some(earlier) = earlier {
                assert_eq!((&notes[0].0, notes[0].1), (&uri_of(&probe, NAME), range_of(&source, earlier, arms)), "{error:#?}");
            }
        }
    }
}
