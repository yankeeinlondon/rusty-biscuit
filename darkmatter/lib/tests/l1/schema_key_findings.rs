//! Removed keys, unknown-key hints, and near-miss advice as Darkmatter's
//! schema surfaces project them: the validation report, and `md clean`'s
//! key-correction analysis, which ranks with Simplified Schema's shared
//! ranker and never offers a removed key.

use std::path::Path;

use biscuit_file::{FileResolutionContext, YamlDiagnosticCode};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::schemas::{DarkmatterSchemas, EffectiveSchema, analyze_frontmatter};
use simplified_schema::ValidationProblemCode;
use tempfile::TempDir;

const SCHEMA: &str = "$schema:\n  name: string\n  setup: \"removed(use a preceding `side_effect:` task)\"\n  $constraints: { closed: true }\n";

const OPEN: &str = "$schema:\n  prompt: string\n  \"<string>\": unknown\n  $advise-unknown-keys: [agent]\n";

fn document(dir: &Path, body: &str) -> Markdown {
    let path = dir.join("doc.md");
    std::fs::write(&path, format!("---\n{body}---\nBody\n")).expect("write document");
    Markdown::try_from(path.as_path()).expect("document loads")
}

fn effective(dir: &Path, markdown: &Markdown) -> EffectiveSchema {
    DarkmatterSchemas::new(FileResolutionContext::new(dir))
        .effective_for(markdown)
        .expect("schema resolves")
        .expect("the document declares a schema")
}

#[test]
fn the_report_names_a_removed_key_at_its_key_with_the_replacement_hint() {
    let dir = TempDir::new().unwrap();
    let markdown = document(dir.path(), &format!("{SCHEMA}name: x\nsetup: null\n"));
    let report = DarkmatterSchemas::new(FileResolutionContext::new(dir.path())).validate(&markdown).unwrap();
    assert!(!report.valid);
    let [problem] = report.problems.as_slice() else { panic!("{:#?}", report.problems) };
    assert_eq!(problem.code, ValidationProblemCode::RemovedKey);
    assert_eq!(problem.code.diagnostic_code(), "dm.schema.removed_key");
    assert_eq!(problem.offending_property.as_deref(), Some("setup"));
    assert_eq!(problem.hint.as_deref(), Some("use a preceding `side_effect:` task"));
    // The key token on file line 7: the fence, four schema lines, `name`.
    assert_eq!((problem.line, problem.column), (Some(7), Some(1)), "{problem:#?}");
}

#[test]
fn the_report_hints_the_nearest_declared_key_for_an_unknown_key() {
    let dir = TempDir::new().unwrap();
    let markdown = document(dir.path(), &format!("{SCHEMA}nmae: x\n"));
    let report = DarkmatterSchemas::new(FileResolutionContext::new(dir.path())).validate(&markdown).unwrap();
    let [problem] = report.problems.as_slice() else { panic!("{:#?}", report.problems) };
    assert_eq!(problem.code, ValidationProblemCode::UnknownKey);
    assert_eq!(problem.hint.as_deref(), Some("did you mean `name`?"));
}

#[test]
fn near_miss_advice_travels_beside_a_valid_report() {
    let dir = TempDir::new().unwrap();
    let markdown = document(dir.path(), &format!("{OPEN}promt: x\nagnet: y\nagent: z\ncolour: blue\n"));
    let report = DarkmatterSchemas::new(FileResolutionContext::new(dir.path())).validate(&markdown).unwrap();
    assert!(report.valid, "advice never invalidates: {:#?}", report.problems);
    let advised: Vec<(&str, Option<&str>)> = report
        .key_advice
        .iter()
        .map(|advice| (advice.offending_property.as_deref().unwrap(), advice.hint.as_deref()))
        .collect();
    assert_eq!(advised, [("promt", Some("did you mean `prompt`?"))], "{:#?}", report.key_advice);
    assert_eq!(report.key_advice[0].code.diagnostic_code(), "dm.schema.near_miss_key");
    // The key token on file line 6: the fence, then four schema lines.
    assert_eq!(report.key_advice[0].line, Some(6));
}

#[test]
fn clean_keeps_its_suggestion_text_and_never_suggests_a_removed_key() {
    let dir = TempDir::new().unwrap();
    for (written, expected) in [
        ("nmae", Some("key `nmae` is not declared by the schema; did you mean `name`?")),
        ("setpu", None),
    ] {
        let body = format!("{SCHEMA}{written}: x\n");
        let markdown = document(dir.path(), &body);
        let analysis = analyze_frontmatter(&effective(dir.path(), &markdown), &body);
        let suggestions: Vec<&str> = analysis
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code == YamlDiagnosticCode::SchemaKeyCorrection)
            .map(|diagnostic| diagnostic.message.as_str())
            .collect();
        match expected {
            Some(message) => assert_eq!(suggestions, [message], "{written}"),
            None => assert!(suggestions.iter().all(|message| !message.contains("did you mean")), "{written}: {suggestions:?}"),
        }
    }
}
