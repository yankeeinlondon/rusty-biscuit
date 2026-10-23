//! Context-function contracts through `evaluate(parse(...))`, the way a
//! document calls them: `file_exists` inspects its argument (criteria 42 and
//! 43), `has_command(true)` looks for a command named `true`, and
//! `validate_schema(file, obj)` validates the merged page in memory (charter
//! M9, criterion 44). Every fixture lives in a private temporary directory.

use std::collections::HashMap;
use std::path::Path;

use darkmatter::markdown::compose::expression::function_test_hooks as hooks;
use darkmatter::markdown::compose::expression::{
    EvaluationLookup, ExpressionError, ResolutionContext, evaluate, parse,
};
use serde_json::{Value, json};
use tempfile::TempDir;

/// Variables plus a resolution context anchored at a temporary directory.
struct Lookup {
    context: ResolutionContext,
    values: HashMap<String, Value>,
}

impl EvaluationLookup for Lookup {
    fn get(&self, path: &str) -> Option<Value> {
        self.values.get(path).cloned()
    }

    fn resolution_context_ref(&self) -> Option<&ResolutionContext> {
        Some(&self.context)
    }
}

fn lookup_at(dir: &Path, values: &[(&str, Value)]) -> Lookup {
    Lookup {
        context: ResolutionContext::new(biscuit_file::FileResolutionContext::new(dir)),
        values: values.iter().map(|(name, value)| (name.to_string(), value.clone())).collect(),
    }
}

fn run(lookup: &Lookup, expression: &str) -> Result<Value, ExpressionError> {
    evaluate(&parse(expression).expect("the expression parses"), lookup)
}

fn is_call_error(result: &Result<Value, ExpressionError>) -> bool {
    matches!(result, Err(ExpressionError::Call(_)))
}

/// Criteria 42 and 43: with files named `42` and `true` at the resolution
/// root, only text names a file; `null`, a number, a boolean, a list, and an
/// object are `false` without a type error and without conversion.
#[test]
fn file_exists_inspects_its_argument_without_converting_it() {
    let dir = TempDir::new().expect("tempdir");
    std::fs::write(dir.path().join("42"), "").expect("write 42");
    std::fs::write(dir.path().join("true"), "").expect("write true");
    let lookup = lookup_at(dir.path(), &[]);

    assert_eq!(run(&lookup, "file_exists(\"42\")").unwrap(), json!(true));
    assert_eq!(run(&lookup, "file_exists(\"true\")").unwrap(), json!(true));
    for expression in [
        "file_exists(42)",
        "file_exists(true)",
        "file_exists(null)",
        "file_exists([\"42\"])",
        "file_exists({\"a\": \"42\"})",
    ] {
        assert_eq!(run(&lookup, expression).unwrap(), json!(false), "{expression}");
    }
}

/// Criterion 44: a boolean reaching `has_command(cmd: string | null)` converts
/// to its text, so `has_command(true)` asks about a command named `true`.
#[test]
fn has_command_true_looks_for_a_command_named_true() {
    let dir = TempDir::new().expect("tempdir");
    let lookup = lookup_at(dir.path(), &[]);

    assert_eq!(
        run(&lookup, "has_command(true)").unwrap(),
        run(&lookup, "has_command(\"true\")").unwrap()
    );
    // `true` is a POSIX utility on every Unix host's `PATH`.
    #[cfg(unix)]
    assert_eq!(run(&lookup, "has_command(true)").unwrap(), json!(true));
    assert_eq!(run(&lookup, "has_command(null)").unwrap(), json!(false));

    hooks::reset_handler_entries();
    assert!(is_call_error(&run(&lookup, "has_command([\"true\"])")));
    assert_eq!(hooks::handler_entries("has_command"), 0);
}

const REQUIRED_TITLE: &str = "---\n$schema:\n  title: 'string(required)'\n  count: number\n";

/// A temporary directory holding `page.md` with `frontmatter` (YAML lines,
/// no fences).
fn page(frontmatter: &str) -> (TempDir, String) {
    let dir = TempDir::new().expect("tempdir");
    let content = format!("{frontmatter}---\nBody.\n");
    std::fs::write(dir.path().join("page.md"), &content).expect("write page");
    (dir, content)
}

/// M9: a supplied required field repairs an incomplete page.
#[test]
fn validate_schema_override_repairs_an_incomplete_page() {
    let (dir, _) = page(REQUIRED_TITLE);
    let lookup = lookup_at(dir.path(), &[]);

    assert_eq!(run(&lookup, "validate_schema(\"page.md\")").unwrap(), json!(false));
    assert_eq!(
        run(&lookup, "validate_schema(\"page.md\", {\"title\": \"Repaired\"})").unwrap(),
        json!(true)
    );
}

/// M9: an invalid override fails a valid page, and an explicit `null`
/// replaces the page's value rather than leaving it in place.
#[test]
fn validate_schema_invalid_override_fails_a_valid_page() {
    let (dir, _) = page(&format!("{REQUIRED_TITLE}title: Valid\ncount: 3\n"));
    let lookup = lookup_at(dir.path(), &[]);

    assert_eq!(run(&lookup, "validate_schema(\"page.md\")").unwrap(), json!(true));
    assert_eq!(
        run(&lookup, "validate_schema(\"page.md\", {\"count\": \"lots\"})").unwrap(),
        json!(false)
    );
    assert_eq!(
        run(&lookup, "validate_schema(\"page.md\", {\"title\": null})").unwrap(),
        json!(false)
    );
}

/// M9: nested objects merge, so an override that supplies one nested field
/// keeps the page's other nested fields.
#[test]
fn validate_schema_merges_nested_objects() {
    let (dir, _) = page(
        "---\n$schema:\n  meta: '{ author: string(required), team: string(required) }'\nmeta:\n  author: Ada\n",
    );
    let lookup = lookup_at(dir.path(), &[]);

    assert_eq!(run(&lookup, "validate_schema(\"page.md\")").unwrap(), json!(false));
    assert_eq!(
        run(&lookup, "validate_schema(\"page.md\", {\"meta\": {\"team\": \"core\"}})").unwrap(),
        json!(true)
    );
}

/// M9: an empty object answers exactly as the one-argument form, for a valid
/// and an invalid page.
#[test]
fn validate_schema_empty_override_equals_the_one_argument_form() {
    for frontmatter in [REQUIRED_TITLE.to_string(), format!("{REQUIRED_TITLE}title: Valid\n")] {
        let (dir, _) = page(&frontmatter);
        let lookup = lookup_at(dir.path(), &[]);
        assert_eq!(
            run(&lookup, "validate_schema(\"page.md\", {})").unwrap(),
            run(&lookup, "validate_schema(\"page.md\")").unwrap(),
            "{frontmatter}"
        );
    }
}

/// M9: repeated calls change neither the file nor the supplied object, and
/// keep answering the same.
#[test]
fn validate_schema_is_nonmutating_across_repeated_calls() {
    let (dir, content) = page(REQUIRED_TITLE);
    let overrides = json!({"title": "Repaired", "count": 2});
    let lookup = lookup_at(dir.path(), &[("overrides", overrides.clone())]);

    for _ in 0..3 {
        assert_eq!(run(&lookup, "validate_schema(\"page.md\", overrides)").unwrap(), json!(true));
        assert_eq!(run(&lookup, "validate_schema(\"page.md\")").unwrap(), json!(false));
    }
    assert_eq!(std::fs::read_to_string(dir.path().join("page.md")).unwrap(), content);
    assert_eq!(lookup.values["overrides"], overrides);
}

/// M9: a combined page that declares no schema is valid.
#[test]
fn validate_schema_without_a_schema_is_valid_with_or_without_an_override() {
    let (dir, _) = page("---\ntitle: Plain\n");
    let lookup = lookup_at(dir.path(), &[]);

    assert_eq!(run(&lookup, "validate_schema(\"page.md\")").unwrap(), json!(true));
    assert_eq!(
        run(&lookup, "validate_schema(\"page.md\", {\"anything\": [1, 2]})").unwrap(),
        json!(true)
    );
}

/// Ledger row 110: a `null` file passes through; a `null` or non-object
/// `obj` is a type error raised before the handler; a file that cannot be
/// loaded stays an error in both forms.
#[test]
fn validate_schema_argument_and_load_failures() {
    let (dir, _) = page(REQUIRED_TITLE);
    let lookup = lookup_at(dir.path(), &[]);

    assert_eq!(run(&lookup, "validate_schema(null)").unwrap(), Value::Null);
    assert_eq!(run(&lookup, "validate_schema(null, {\"title\": \"x\"})").unwrap(), Value::Null);

    hooks::reset_handler_entries();
    for expression in ["validate_schema(\"page.md\", null)", "validate_schema(\"page.md\", \"title\")"] {
        assert!(is_call_error(&run(&lookup, expression)), "{expression}");
    }
    assert_eq!(hooks::handler_entries("validate_schema"), 0);

    for expression in ["validate_schema(\"missing.md\")", "validate_schema(\"missing.md\", {\"title\": \"x\"})"] {
        let error = run(&lookup, expression).unwrap_err();
        assert!(matches!(error, ExpressionError::FileReference(_)), "{expression}: {error:?}");
    }
}
