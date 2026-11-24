//! The one callable-catalog judgment (`DarkmatterSchemas::validate_callable_catalog`)
//! through the public whole-YAML route every consumer uses: a semantic
//! failure is a located `SchemaError::CallableCatalog`, a shape or grammar
//! failure stays the kind contract's located problem, and the built-in
//! catalogs are recognized by path identity, never by file name.

use std::path::{Path, PathBuf};

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::schemas::{DarkmatterSchemas, Route, YamlOutcome};
use simplified_schema::SchemaError;
use simplified_schema::yaml_documents::LoadedYamlDocument;

/// A context rooted at `root`, claimed as the repository root.
fn schemas(root: &Path) -> DarkmatterSchemas {
    DarkmatterSchemas::new(FileResolutionContext::new(root).with_repository_root(root))
}

fn judge(root: &Path, relative: &str, text: &str) -> Result<YamlOutcome, SchemaError> {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().expect("a parent")).expect("the folder");
    std::fs::write(&path, text).expect("the file");
    schemas(root).validate_yaml_document(text, &path)
}

/// A related location: its file and the text its span covers.
type Related = (PathBuf, Option<String>);

/// The failure's message, primary text, and each related location.
fn located(text: &str, error: &SchemaError) -> (String, Option<String>, Vec<Related>) {
    let SchemaError::CallableCatalog { message, span, related, .. } = error else {
        panic!("a callable catalog failure: {error:?}");
    };
    let related = related
        .iter()
        .map(|related| {
            let source = if related.file == Path::new("darkmatter/schemas/partials/functions.yaml") {
                include_str!("../../../schemas/partials/functions.yaml").to_string()
            } else {
                text.to_string()
            };
            (related.file.clone(), related.span.clone().map(|span| source[span].to_string()))
        })
        .collect();
    (message.clone(), span.clone().map(|span| text[span].to_string()), related)
}

#[test]
fn the_host_fixture_is_a_valid_expression_catalog() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let text = include_str!("../fixtures/host_functions/project.yaml");
    match judge(dir.path(), "host/project.yaml", text) {
        Ok(YamlOutcome::Validated(validation)) => {
            assert_eq!(validation.route, Route::Kind("expression-schema".into()));
            assert!(validation.report.valid, "{:?}", validation.report.problems);
        }
        other => panic!("a validated catalog: {other:?}"),
    }
}

#[test]
fn a_folded_name_collision_is_a_located_catalog_error_with_the_earlier_key() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let control = include_str!("../fixtures/host_functions/project.yaml");
    // One edit: a second spelling of an existing callable, ahead of nothing
    // else, after a non-ASCII description (byte offsets, not characters).
    let text = control.replacen("    is_project_file:", "    Is_Project_File: |-\n        function(parameters(value: string); returns(boolean); category(Project)) -> Café.\n    is_project_file:", 1);
    assert_ne!(text, control);
    let error = judge(dir.path(), "host/project.yaml", &text).expect_err("two spellings of one callable");
    let (message, span, related) = located(&text, &error);
    assert!(message.contains("name the same callable"), "{message}");
    assert_eq!(span.as_deref(), Some("is_project_file"));
    assert_eq!(related, [(dir.path().join("host/project.yaml"), Some("Is_Project_File".to_string()))]);
}

#[test]
fn a_built_in_name_is_refused_with_the_built_in_declaration_related() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let text = include_str!("../fixtures/host_functions/project.yaml").replacen("    project_label:", "    kebab_case:", 1);
    let error = judge(dir.path(), "host/project.yaml", &text).expect_err("`kebab_case` is a built-in");
    let (message, span, related) = located(&text, &error);
    assert!(message.contains("built-in function `kebab_case`"), "{message}");
    assert_eq!(span.as_deref(), Some("kebab_case"));
    assert_eq!(related, [(PathBuf::from("darkmatter/schemas/partials/functions.yaml"), Some("kebab_case".to_string()))]);
}

#[test]
fn a_grammar_failure_stays_the_kind_contracts_located_problem() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let text = include_str!("../fixtures/host_functions/project.yaml").replacen("returns(boolean)", "returns(bolean)", 1);
    match judge(dir.path(), "host/project.yaml", &text) {
        Ok(YamlOutcome::Validated(validation)) => {
            assert!(!validation.report.valid, "the bad type is reported");
            let problem = &validation.report.problems[0];
            let token = validation.byte_range(problem).map(|range| text[range].to_string()).unwrap_or_default();
            assert!(token.contains("bolean"), "{problem:?} at {token:?}");
        }
        other => panic!("the kind contract's problem: {other:?}"),
    }
}

#[test]
fn a_mutation_catalog_with_narrowing_is_refused() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let control = "kind: mutation-schema\ncategories:\n    - State\ntypes:\n    put: |-\n        function(parameters(key: unknown | null); returns(boolean); category(State))\n";
    assert!(matches!(judge(dir.path(), "m.yaml", control), Ok(YamlOutcome::Validated(validation)) if validation.report.valid));
    let text = control.replace("category(State)", "narrows(key: string; true-implies); category(State)");
    let error = judge(dir.path(), "m.yaml", &text).expect_err("a mutation proves nothing");
    let (message, span, _) = located(&text, &error);
    assert!(message.contains("a mutation declares no `narrows(...)`"), "{message}");
    assert!(span.is_some_and(|span| span.contains("narrows(")));
}

/// The built-in catalogs judge themselves at their repository path: their
/// own names and file-form examples are admitted. The same bytes under any
/// other path (the same file name included) are an authored catalog.
#[test]
fn the_built_in_catalogs_are_recognized_by_path_identity_only() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let functions = include_str!("../../../schemas/partials/functions.yaml");
    let mutations = include_str!("../../../schemas/partials/mutations.yaml");
    for (relative, text) in
        [("darkmatter/schemas/partials/functions.yaml", functions), ("darkmatter/schemas/partials/mutations.yaml", mutations)]
    {
        match judge(dir.path(), relative, text) {
            Ok(YamlOutcome::Validated(validation)) => assert!(validation.report.valid, "{relative}: {:?}", validation.report.problems),
            other => panic!("{relative}: {other:?}"),
        }
    }
    let error = judge(dir.path(), "elsewhere/functions.yaml", functions).expect_err("an authored copy reuses built-in names");
    assert!(matches!(&error, SchemaError::CallableCatalog { message, .. } if message.contains("reuses")), "{error}");
}

/// Judging a catalog is passive: an authored example names a call that is
/// never made, and a document with the same `kind` and `$schema` but
/// different text is judged on its own text.
#[test]
fn the_judgment_reads_the_whole_text_and_evaluates_nothing() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let control = include_str!("../fixtures/host_functions/project.yaml");
    let schemas = schemas(dir.path());
    let path = dir.path().join("host/project.yaml");
    let valid = LoadedYamlDocument::load(control, &path).expect("loads");
    schemas.validate_callable_catalog(&valid).expect("the control is valid");
    let invalid_text = control.replacen("    project_label:", "    PROJECT_LABEL: |-\n        function(parameters(name: string); returns(string); category(Project))\n    project_label:", 1);
    let invalid = LoadedYamlDocument::load(&invalid_text, &path).expect("loads");
    assert_eq!(valid.declaration_key(), invalid.declaration_key(), "the same declaration key");
    assert!(schemas.validate_callable_catalog(&invalid).is_err());
    schemas.validate_callable_catalog(&valid).expect("valid again");
    // Not a catalog: nothing to judge.
    let other = LoadedYamlDocument::load("kind: schema\ntypes:\n    a: string\n", &path).expect("loads");
    schemas.validate_callable_catalog(&other).expect("not a callable catalog");
}
