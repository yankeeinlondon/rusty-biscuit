//! A required property with a resolved `default(...)` is answered by the
//! default, never by interactive collection, in direct compose and at
//! `inline-compose`'s launch phase alike. A refused default blocks as a
//! schema error; an optional default, and a default for a written `null`, are
//! inserted too.

use super::*;
use crate::composition::CompositionMode;
use crate::composition::TriggerFindings;
use std::fs;
use tempfile::TempDir;

fn make_source(dir: &TempDir, document: &str) -> ResolvedCompositionSource {
    let file = dir.path().join("test.md");
    fs::write(&file, document).unwrap();
    crate::composition::resolve_fixture_source(file.to_str().unwrap()).unwrap()
}

fn pre_validate(source: &ResolvedCompositionSource, mode: CompositionMode) -> Result<PreValidatedSchema, CompositionError> {
    pre_validate_schema_for_mode(source, None, None, &crate::test_support::context(), mode, &TriggerFindings::default())
}

fn status(source: &ResolvedCompositionSource, mode: CompositionMode) -> SchemaStatusReport {
    build_schema_status_report_for_mode(source, None, None, &crate::test_support::context(), mode, &TriggerFindings::default())
        .unwrap()
        .expect("the document declares a schema")
}

fn state(report: &SchemaStatusReport, name: &str) -> PropertyState {
    report
        .required
        .iter()
        .chain(&report.optional)
        .find(|property| property.name == name)
        .unwrap_or_else(|| panic!("{name} is in the report"))
        .state
}

#[track_caller]
fn missing_names(err: CompositionError) -> Vec<String> {
    match err {
        CompositionError::MissingProperties { missing, .. } => missing.into_iter().map(|p| p.name).collect(),
        other => panic!("expected MissingProperties, got {other:?}"),
    }
}

#[track_caller]
fn assert_refused_default(err: CompositionError, property: &str) {
    match err {
        CompositionError::SchemaValidation { message, problems, .. } => {
            assert_eq!(problems, [format!("/{property}")], "{message}");
            assert!(
                message.contains(&format!("/properties/{property}/default")),
                "the error names the schema default: {message}"
            );
        }
        other => panic!("a refused default blocks as a schema error, got {other:?}"),
    }
}

const DEFAULTED: &str = "---\n$schema:\n  n: number(required; default(3))\n---\nbody\n";

#[test]
fn an_absent_required_default_is_never_collected_in_direct_compose() {
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, DEFAULTED);

    pre_validate(&source, CompositionMode::ChainedDocument).expect("the default answers `n`");
    let prepared = prepare_direct_with_schema(&source, PrepareOptions::new(crate::test_support::context())).unwrap();

    assert_eq!(prepared.effective_frontmatter["n"], serde_json::json!(3), "composition inserts the default");
    assert_eq!(state(&status(&source, CompositionMode::ChainedDocument), "n"), PropertyState::Defaulted);
}

#[test]
fn an_absent_required_property_without_a_default_is_still_collected() {
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        "---\n$schema:\n  n: number(required; default(3))\n  r: number(required)\n---\nbody\n",
    );

    let err = pre_validate(&source, CompositionMode::ChainedDocument).unwrap_err();

    assert_eq!(missing_names(err), ["r"]);
    let report = status(&source, CompositionMode::ChainedDocument);
    assert_eq!(state(&report, "r"), PropertyState::Missing);
    assert_eq!(state(&report, "n"), PropertyState::Defaulted);
}

#[test]
fn an_optional_default_is_inserted_without_collection() {
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, "---\n$schema:\n  n: number(default(3))\n---\nbody\n");

    pre_validate(&source, CompositionMode::ChainedDocument).expect("an optional property is never collected");
    let prepared = prepare_direct_with_schema(&source, PrepareOptions::new(crate::test_support::context())).unwrap();

    assert_eq!(prepared.effective_frontmatter["n"], serde_json::json!(3), "composition inserts the default");
    assert_eq!(state(&status(&source, CompositionMode::ChainedDocument), "n"), PropertyState::Defaulted);
}

#[test]
fn an_explicit_null_takes_the_default() {
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, "---\n$schema:\n  n: number(required; default(3))\nn: null\n---\nbody\n");

    pre_validate(&source, CompositionMode::ChainedDocument).expect("the default replaces the written null");
    let prepared = prepare_direct_with_schema(&source, PrepareOptions::new(crate::test_support::context())).unwrap();

    assert_eq!(prepared.effective_frontmatter["n"], serde_json::json!(3));
    assert_eq!(state(&status(&source, CompositionMode::ChainedDocument), "n"), PropertyState::Defaulted);
}

#[test]
fn an_explicit_null_without_a_default_is_judged_by_type() {
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, "---\n$schema:\n  n: number(required)\nn: null\n---\nbody\n");

    let err = pre_validate(&source, CompositionMode::ChainedDocument).unwrap_err();

    assert!(
        matches!(&err, CompositionError::SchemaValidation { problems, message, .. }
            if problems == &["/n"] && message.contains("null is not of type")),
        "the written null meets `required` and fails the type: {err:?}"
    );
    assert_eq!(state(&status(&source, CompositionMode::ChainedDocument), "n"), PropertyState::Invalid);
}

#[test]
fn a_refused_default_blocks_instead_of_prompting() {
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, "---\n$schema:\n  n: number(required; min(5); default(3))\n---\nbody\n");

    assert_refused_default(pre_validate(&source, CompositionMode::ChainedDocument).unwrap_err(), "n");
    assert_refused_default(
        prepare_direct_with_schema(&source, PrepareOptions::new(crate::test_support::context())).unwrap_err(),
        "n",
    );
    assert_eq!(state(&status(&source, CompositionMode::ChainedDocument), "n"), PropertyState::Invalid);
}

#[test]
fn a_nested_default_under_an_absent_parent_leaves_the_parent_to_collect() {
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        "---\n$schema:\n  parent: \"{ child: number(required; default(1)) }(required)\"\n---\nbody\n",
    );

    let err = pre_validate(&source, CompositionMode::ChainedDocument).unwrap_err();

    assert_eq!(missing_names(err), ["parent"]);
}

mod launch {
    use super::*;

    const PROMPT: &str = "  prompt: string(required; eager)\n";

    #[test]
    fn a_required_default_is_defaulted_rather_than_deferred() {
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            &format!(
                "---\n$schema:\n{PROMPT}  n: number(required; default(3))\n  e: number(required; eager; default(4))\n  later: string(required)\nprompt: hi\n---\nbody\n"
            ),
        );

        pre_validate(&source, CompositionMode::InlineFrontmatterPrompt).expect("defaults answer `n` and `e`");
        let report = status(&source, CompositionMode::InlineFrontmatterPrompt);
        assert_eq!(state(&report, "n"), PropertyState::Defaulted, "a non-eager default composition fills");
        assert_eq!(state(&report, "e"), PropertyState::Defaulted, "an eager default");
        assert_eq!(state(&report, "later"), PropertyState::Deferred, "control: no default, left for the agent");

        let prepared = prepare_inline_with_schema(&source, PrepareOptions::new(crate::test_support::context())).unwrap();
        assert_eq!(prepared.effective_frontmatter["n"], serde_json::json!(3));
        assert_eq!(prepared.effective_frontmatter["e"], serde_json::json!(4));
    }

    #[test]
    fn an_eager_required_property_without_a_default_is_still_collected() {
        let dir = TempDir::new().unwrap();
        let source = make_source(&dir, &format!("---\n$schema:\n{PROMPT}  n: number(required; default(3))\n---\nbody\n"));

        let err = pre_validate(&source, CompositionMode::InlineFrontmatterPrompt).unwrap_err();

        assert_eq!(missing_names(err), ["prompt"]);
    }

    #[test]
    fn a_refused_default_of_a_non_eager_property_blocks_at_launch() {
        // The launch projection does not require `n`, but default
        // preparation ignores `required`, so the launch verdict refuses the
        // default once.
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            &format!("---\n$schema:\n{PROMPT}  n: number(required; min(5); default(3))\nprompt: hi\n---\nbody\n"),
        );

        assert_refused_default(pre_validate(&source, CompositionMode::InlineFrontmatterPrompt).unwrap_err(), "n");
        assert_refused_default(
            prepare_inline_with_schema(&source, PrepareOptions::new(crate::test_support::context())).unwrap_err(),
            "n",
        );
        assert_eq!(state(&status(&source, CompositionMode::InlineFrontmatterPrompt), "n"), PropertyState::Invalid);
    }

    /// At launch a non-eager required property may still be missing, but a
    /// value that was supplied is judged by its declared type: a written
    /// `null` meets `required` and fails `number`, and so does text.
    #[test]
    fn a_supplied_null_or_wrong_type_is_judged_by_type_at_launch() {
        for written in ["null", "abc"] {
            let dir = TempDir::new().unwrap();
            let source = make_source(
                &dir,
                &format!("---\n$schema:\n{PROMPT}  count: number(required)\nprompt: hi\ncount: {written}\n---\nbody\n"),
            );

            let err = pre_validate(&source, CompositionMode::InlineFrontmatterPrompt).unwrap_err();
            assert!(
                matches!(&err, CompositionError::SchemaValidation { problems, .. } if problems == &["/count"]),
                "count: {written}: {err:?}"
            );
            assert_eq!(
                state(&status(&source, CompositionMode::InlineFrontmatterPrompt), "count"),
                PropertyState::Invalid,
                "count: {written}"
            );
        }
    }

    /// The same missing value is deferred to the agent at launch and is a
    /// missing property when the whole contract applies.
    #[test]
    fn a_missing_non_eager_required_value_is_deferred_at_launch_and_missing_when_composed() {
        let dir = TempDir::new().unwrap();
        let source = make_source(&dir, &format!("---\n$schema:\n{PROMPT}  count: number(required)\nprompt: hi\n---\nbody\n"));

        pre_validate(&source, CompositionMode::InlineFrontmatterPrompt).expect("left for the agent at launch");
        assert_eq!(state(&status(&source, CompositionMode::InlineFrontmatterPrompt), "count"), PropertyState::Deferred);

        let err = pre_validate(&source, CompositionMode::ChainedDocument).unwrap_err();
        assert_eq!(missing_names(err), ["count"]);
    }
}
