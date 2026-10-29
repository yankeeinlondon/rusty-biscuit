//! File predicates at the library's public API: `is_file_type`'s passive
//! structural verdict, `file_exists`'s structural rejection before any
//! lookup, the always-false call warning in the call check and composition,
//! and `is_file_type` guards making a file argument category 1.
//!
//! `is_file_type` is evaluated with a lookup that has no resolution context,
//! so no filesystem, environment, or network is reachable; template rows name
//! variables that are never set. Context-dependent rows anchor in their own
//! temporary directory.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::expression::call_binding::{FunctionRegistration, RegistrationError};
use darkmatter::markdown::compose::expression::function_test_hooks as hooks;
use darkmatter::markdown::compose::expression::{
    ALWAYS_FALSE_CALL_CODE, ArgumentCategory, CallCheck, DeclaredTypes, EvaluationLookup, ResolutionContext,
    TypeScope, check_expression_calls, evaluate, parse, parse_spanned,
};
use darkmatter::markdown::compose::page_blocks::BodyGuards;
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions, ComposeSource};
use darkmatter::markdown::schemas::{SimplifiedSchema, parse_yaml_schema};
use serde_json::{Value, json};
use tempfile::TempDir;

/// Text `is_file_type` accepts on every host: every supported reference
/// kind, including spellings of another OS and plain words.
const WELL_FORMED: &[&str] = &[
    "./docs/spec.md",
    "/tmp/spec.md",
    r"\Users\ken\data\foobar.json",
    r"C:\data\spec.md",
    r"\\server\share\spec.md",
    "@docs/spec.md",
    "vault:notes/spec.md",
    "%@spec.md",
    "&docs/spec.md",
    "^docs/spec.md",
    "~/spec.md",
    "https://example.com/spec.md",
    "http://example.com/spec.md",
    "{{DM_FILE_PREDICATES_UNSET}}/spec.md",
    "file://foo/bar",
    "file:///tmp/spec.md",
    "42",
    "hello world",
];

/// Text that is not a well-formed reference.
const MALFORMED: &[&str] = &[
    "",
    "https://",
    "http://",
    "https:///x",
    "%https://example.com/x",
    "vault:",
    "vault:/etc/hosts",
    "a\0b.md",
    "{{home}}/x",
    "ftp://example.com/x",
    "~bob/x",
    "@",
    "file://",
];

/// A lookup with one variable `p` and, optionally, a resolution context.
struct Lookup {
    value: Value,
    context: Option<ResolutionContext>,
}

impl EvaluationLookup for Lookup {
    fn get(&self, path: &str) -> Option<Value> {
        (path == "p").then(|| self.value.clone())
    }

    fn resolution_context(&self) -> Option<ResolutionContext> {
        self.context.clone()
    }
}

/// `function(p)` with `p` bound to `value`, evaluated without a resolution
/// context.
fn context_free(function: &str, value: Value) -> Value {
    let lookup = Lookup { value, context: None };
    evaluate(&parse(&format!("{function}(p)")).unwrap(), &lookup)
        .unwrap_or_else(|error| panic!("{function}: {error}"))
}

fn anchored(dir: &Path) -> ResolutionContext {
    ResolutionContext::new(FileResolutionContext::from_snapshot(dir, Some(dir.join("home")), HashMap::new()))
}

fn schema(yaml: &str) -> SimplifiedSchema {
    parse_yaml_schema(&serde_yaml_ng::from_str(yaml).expect("YAML parses")).expect("schema parses")
}

/// Every call check in `expression` at an unguarded site.
fn checks(declared: &DeclaredTypes<'_>, expression: &str) -> Vec<CallCheck> {
    check_expression_calls(&parse_spanned(expression).expect("parses"), &TypeScope::unguarded(declared))
}

#[track_caller]
fn check_of(declared: &DeclaredTypes<'_>, expression: &str, function: &str) -> CallCheck {
    checks(declared, expression)
        .into_iter()
        .find(|check| check.function == function)
        .unwrap_or_else(|| panic!("{function} is checked in {expression}"))
}

mod is_file_type {
    use super::*;

    /// Criteria 13 and 35: true for text that is a well-formed reference of
    /// any kind, false for malformed text and every non-text value, with no
    /// resolution context, so nothing can be read; no effect engine is
    /// built and no network is attempted.
    #[test]
    fn it_is_true_exactly_for_well_formed_reference_text_without_a_context() {
        #[cfg(feature = "effects-instrumentation")]
        let before = (darkmatter::effects::engine_build_count(), darkmatter::effects::network_attempt_count());
        for raw in WELL_FORMED {
            assert_eq!(context_free("is_file_type", json!(raw)), json!(true), "is_file_type({raw:?})");
        }
        for raw in MALFORMED {
            assert_eq!(context_free("is_file_type", json!(raw)), json!(false), "is_file_type({raw:?})");
        }
        for value in [json!(42), json!(null), json!(true), json!(["./a.md"]), json!({ "path": "./a.md" })] {
            assert_eq!(context_free("is_file_type", value.clone()), json!(false), "is_file_type({value})");
        }
        #[cfg(feature = "effects-instrumentation")]
        assert_eq!(
            (darkmatter::effects::engine_build_count(), darkmatter::effects::network_attempt_count()),
            before,
            "is_file_type built an effect engine or attempted network access"
        );
    }

    /// The handler leaves its argument unchanged: the variable still holds
    /// the authored text after the call.
    #[test]
    fn it_never_changes_its_argument() {
        let lookup = Lookup { value: json!("file://foo/bar"), context: None };
        let expr = parse("is_file_type(p) ? p : ''").unwrap();
        assert_eq!(evaluate(&expr, &lookup).unwrap(), json!("file://foo/bar"));
        assert_eq!(lookup.value, json!("file://foo/bar"));
    }
}

mod file_exists {
    use super::*;

    /// Criteria 14 and 35: malformed text is `false` for `file_exists` and
    /// its `fileexists` alias alike, and `is_file_type` agrees on every row.
    /// Well-formed rows keep their outcomes: an existing file is `true`, a
    /// missing one `false`, and a `file://` input goes through its adapter.
    #[test]
    fn malformed_text_is_false_and_well_formed_references_keep_their_outcomes() {
        let dir = TempDir::new().unwrap();
        let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
        std::fs::write(root.join("page.md"), "# Page\n").unwrap();
        let context = anchored(&root);
        let call = |function: &str, value: Value| {
            let lookup = Lookup { value, context: Some(context.clone()) };
            evaluate(&parse(&format!("{function}(p)")).unwrap(), &lookup)
        };
        let absolute = biscuit_file::to_portable_string(&root.join("page.md"));
        let rows: Vec<(String, bool)> = MALFORMED
            .iter()
            .map(|raw| (raw.to_string(), false))
            .chain([
                ("page.md".to_string(), true),
                ("./page.md".to_string(), true),
                ("missing.md".to_string(), false),
                ("file://page.md".to_string(), true),
                (format!("file://{absolute}"), true),
                ("file://missing.md".to_string(), false),
            ])
            .collect();
        for (raw, exists) in rows {
            for function in ["file_exists", "fileexists"] {
                let got = call(function, json!(raw)).unwrap_or_else(|error| panic!("{function}({raw:?}): {error}"));
                assert_eq!(got, json!(exists), "{function}({raw:?})");
            }
        }
        assert_eq!(call("file_exists", json!(null)).unwrap(), json!(false));
    }

    /// A valid reference that cannot be answered is still an error, never a
    /// missing file: an HTTP(S) URL with no remote runtime.
    #[test]
    fn a_valid_remote_reference_without_a_runtime_is_still_an_error() {
        let dir = TempDir::new().unwrap();
        let lookup = Lookup { value: json!("https://example.com/spec.md"), context: Some(anchored(dir.path())) };
        let error = evaluate(&parse("file_exists(p)").unwrap(), &lookup).unwrap_err();
        assert!(error.to_string().contains("local-only"), "{error}");
    }
}

mod always_false {
    use super::*;

    /// Criteria 15 and 35, library side: a malformed literal makes the
    /// call always false, for the alias too; a `string`-typed variable, a
    /// well-formed literal, and `is_file_type` itself do not.
    #[test]
    fn only_a_malformed_literal_reference_is_always_false() {
        let schema = schema("path: string(required)\n");
        let declared = DeclaredTypes::new(Some(&schema));
        for (expression, function) in [
            (r#"file_exists("https://")"#, "file_exists"),
            (r#"file_exists("")"#, "file_exists"),
            (r#"fileexists("https://")"#, "fileexists"),
            (r#"is_null(x) || file_exists("vault:")"#, "file_exists"),
        ] {
            let check = check_of(&declared, expression, function);
            assert_eq!(check.always_false, Some(0), "{expression}");
            assert_eq!(check.category(), ArgumentCategory::Compatible, "{expression}");
            let finding = check.always_false_finding().expect("a finding");
            assert!(finding.message.contains("always false"), "{}", finding.message);
        }
        for (expression, function) in [
            ("file_exists(path)", "file_exists"),
            (r#"file_exists("./a.md")"#, "file_exists"),
            (r#"file_exists("https://example.com/a.md")"#, "file_exists"),
            (r#"is_file_type("https://")"#, "is_file_type"),
            ("file_exists(42)", "file_exists"),
        ] {
            assert_eq!(check_of(&declared, expression, function).always_false, None, "{expression}");
        }
    }

    /// The rule belongs to the registration, not to a name: a test-registered
    /// predicate declaring it gets the same verdict, and registration rejects
    /// it on a non-boolean function or an undeclared parameter.
    #[test]
    fn the_rule_is_registered_and_validated() {
        let signature = "function(parameters(value: unknown | null); returns(boolean))";
        let registration = FunctionRegistration::builder("is_project_file", signature)
            .false_for_malformed_reference("value")
            .build()
            .expect("the rule registers");
        assert_eq!(registration.malformed_reference_parameter(), Some(0));
        let _guard = hooks::register(registration, Arc::new(|_| Ok(json!(true))));
        hooks::reset_handler_entries();
        let declared = DeclaredTypes::new(None);
        assert_eq!(check_of(&declared, r#"is_project_file("https://")"#, "is_project_file").always_false, Some(0));
        assert_eq!(hooks::handler_entries("is_project_file"), 0, "analysis never runs the handler");

        let not_boolean = FunctionRegistration::builder("f", "function(parameters(value: unknown | null); returns(string))")
            .false_for_malformed_reference("value")
            .build();
        assert!(matches!(not_boolean, Err(RegistrationError::InvalidReferenceRule { .. })), "{not_boolean:?}");
        let unknown = FunctionRegistration::builder("g", signature).false_for_malformed_reference("other").build();
        assert!(matches!(unknown, Err(RegistrationError::UnknownParameter { .. })), "{unknown:?}");
    }
}

mod guarded_file_arguments {
    use super::*;

    /// Criterion 16: with `source: unknown`, `frontmatter(source, 'title')`
    /// is category 4 and the `is_file_type` guard makes it category 1; the
    /// guard itself is category 1 (its parameter is `unknown | null`).
    #[test]
    fn an_is_file_type_guard_makes_the_file_argument_category_1() {
        let schema = schema("source: unknown\n");
        let declared = DeclaredTypes::new(Some(&schema));
        let unguarded = "frontmatter(source, 'title')";
        let guarded = "is_file_type(source) && frontmatter(source, 'title')";
        assert_eq!(check_of(&declared, unguarded, "frontmatter").category(), ArgumentCategory::Unknown);
        assert_eq!(check_of(&declared, guarded, "frontmatter").category(), ArgumentCategory::Compatible);
        assert_eq!(check_of(&declared, guarded, "is_file_type").category(), ArgumentCategory::Compatible);
        // The fact is true-implies: the false side proves nothing.
        let negated = "is_file_type(source) ? '' : frontmatter(source, 'title')";
        assert_eq!(check_of(&declared, negated, "frontmatter").category(), ArgumentCategory::Unknown);
    }

    /// Criterion 11, `is_file_type` leg: its registered fact narrows a
    /// shell template hole under a `::block` guard, from the same scope.
    #[test]
    fn an_is_file_type_guard_narrows_a_shell_template_hole() {
        use darkmatter::markdown::compose::shell_expansion::template::project_hole;
        let schema = schema("source: unknown\n");
        let declared = DeclaredTypes::new(Some(&schema));
        let body = "::block when=\"is_file_type(source)\"\n::shell cat {{ source }}\n::end-block\n";
        let site = BodyGuards::scan(body).scope_at(TypeScope::unguarded(&declared), body.find("{{").unwrap());
        let narrowed = project_hole("source", &site).display_type();
        assert!(narrowed.contains("file"), "{narrowed}");
        assert_ne!(project_hole("source", &TypeScope::unguarded(&declared)).display_type(), narrowed);
    }
}

mod composition {
    use super::*;

    fn compose(dir: &Path, content: &str) -> (Markdown, darkmatter::markdown::compose::ComposeReport) {
        let path = dir.join("doc.md");
        std::fs::write(&path, content).unwrap();
        let options =
            ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir, "")).with_source_file(path.clone());
        Markdown::try_from(path.as_path())
            .unwrap()
            .with_source(ComposeSource::File(path.clone()))
            .compose_with(&crate::request_support::request_at(dir, options))
            .expect("the document composes")
    }

    /// Criteria 35 and 16 at composition: each malformed literal argument
    /// is one always-false warning at its line, `fileexists` included; a
    /// `string`-typed variable gets none; the guarded `frontmatter` call
    /// composes; nothing reaches the network.
    #[test]
    fn composition_warns_for_each_always_false_call_and_composes_the_guarded_read() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("target.md"), "---\ntitle: Target\n---\n").unwrap();
        let content = concat!(
            "---\n",
            "$schema:\n",
            "  source: unknown\n",
            "  path: string\n",
            "source: './target.md'\n",
            "path: './target.md'\n",
            "---\n",
            "A {{ file_exists(\"https://\") }}\n\n",
            "B {{ fileexists(\"\") }}\n\n",
            "C {{ file_exists(path) }} {{ is_file_type(source) ? frontmatter(source, 'title') : 'none' }}\n",
        );
        #[cfg(feature = "effects-instrumentation")]
        let before = darkmatter::effects::network_attempt_count();
        let (composed, report) = compose(dir.path(), content);
        assert_eq!(composed.content().trim_end(), "A false\n\nB false\n\nC true Target");
        let warnings: Vec<_> =
            report.warnings.iter().filter(|warning| warning.code.as_deref() == Some(ALWAYS_FALSE_CALL_CODE)).collect();
        let lines: Vec<_> = warnings.iter().map(|warning| warning.line_number).collect();
        assert_eq!(lines, [Some(8), Some(10)], "{warnings:#?}");
        assert!(warnings[0].message.contains("`file_exists(…)`"), "{}", warnings[0].message);
        assert!(warnings[1].message.contains("`fileexists(…)`"), "{}", warnings[1].message);
        #[cfg(feature = "effects-instrumentation")]
        assert_eq!(darkmatter::effects::network_attempt_count(), before);
    }
}
