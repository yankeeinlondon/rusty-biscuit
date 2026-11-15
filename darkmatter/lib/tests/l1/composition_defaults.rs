//! A property with a `default(...)`, required or not, whose value a document
//! leaves missing or `null`: composition writes the default into the composed
//! frontmatter before anything reads it, validation-only APIs report the
//! default without inserting it, and expression typing treats the property as
//! present.
//!
//! Every fixture lives in its own temporary directory and every request is
//! anchored there.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use darkmatter::markdown::compose::expression::{
    ArgumentCategory, CallCheck, DeclaredTypes, TypeScope, check_expression_calls, parse, parse_spanned,
};
use darkmatter::markdown::compose::shell_expansion::{
    ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest, ShellExpansionOptions,
};
use darkmatter::markdown::compose::{
    ComposeContext, ComposeFailure, ComposeOptions, ComposeReport, ComposeSource, ShellExpansionError,
};
use darkmatter::markdown::schemas::{DarkmatterSchemas, EffectiveSchema, YamlOutcome};
use darkmatter::markdown::{Markdown, MarkdownError};
use serde_json::{Value, json};
use simplified_schema::{AppliedDefault, PropertyPath, ValidationProblem, ValidationProblemCode};
use tempfile::TempDir;

fn write(dir: &Path, name: &str, content: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    std::fs::write(&path, content).expect("write fixture");
    path
}

/// Composes `content` as `name` in `dir`, adjusting the options with `adjust`.
fn try_compose_at(
    dir: &Path,
    name: &str,
    content: &str,
    adjust: impl FnOnce(ComposeOptions) -> ComposeOptions,
) -> Result<(Markdown, ComposeReport), Box<ComposeFailure>> {
    let path = write(dir, name, content);
    let base = path.parent().expect("parent").to_path_buf();
    let options = adjust(
        ComposeOptions::new_with_context(ComposeContext::capture_for_content(&base, "")).with_source_file(path.clone()),
    );
    Markdown::try_from(path.as_path())
        .expect("document loads")
        .compose_with_warnings(&crate::request_support::request_at(&base, options))
}

fn try_compose(dir: &Path, content: &str) -> Result<(Markdown, ComposeReport), Box<ComposeFailure>> {
    try_compose_at(dir, "doc.md", content, |options| options)
}

#[track_caller]
fn compose(dir: &Path, content: &str) -> Markdown {
    try_compose(dir, content).unwrap_or_else(|failure| panic!("{content:?} failed: {}", failure.error)).0
}

/// The problems of a schema failure.
#[track_caller]
fn schema_problems(failure: &ComposeFailure) -> &[ValidationProblem] {
    match &failure.error {
        MarkdownError::SchemaValidationFailed { problems, .. } => problems,
        other => panic!("expected a schema failure, got {other}"),
    }
}

fn applied(path: &str, value: Value) -> AppliedDefault {
    AppliedDefault { path: PropertyPath::parse(path).expect("path"), value }
}

fn context_at(dir: &Path) -> biscuit_file::FileResolutionContext {
    crate::request_support::context_at(dir)
}

fn markdown_at(path: &Path) -> Markdown {
    Markdown::try_from(path).expect("document loads").with_source(ComposeSource::File(path.to_path_buf()))
}

const GREETING: &str = "---\n$schema:\n  x: string(required; default(hello))\n---\nBody {{ x }}\n";

#[test]
fn a_required_default_reaches_the_body() {
    let dir = TempDir::new().unwrap();

    let composed = compose(dir.path(), GREETING);

    assert!(composed.content().contains("Body hello"), "{}", composed.content());
    assert_eq!(composed.frontmatter().as_map().get("x"), Some(&json!("hello")));
}

struct AllowAll;

impl ShellApprovalHandler for AllowAll {
    fn approve(&self, _request: ShellApprovalRequest) -> Result<ShellApprovalDecision, ShellExpansionError> {
        Ok(ShellApprovalDecision::AllowOnce)
    }
}

/// A frontmatter value that reads a command's output waits for interpolation
/// pass 2, which runs after the schema stage inserted the default.
#[test]
fn a_required_default_reaches_a_second_pass_frontmatter_value() {
    let dir = TempDir::new().unwrap();
    let document = "---\n$schema:\n  x: string(required; default(hello))\nversion: $(git --version)\ngreeting: \"{{ x }} {{ version }}\"\n---\n{{ greeting }}\n";

    let (composed, _) = try_compose_at(dir.path(), "doc.md", document, |options| {
        options
            .with_config_snapshot(crate::request_support::shell_config(dir.path(), &[])).with_shell(ShellExpansionOptions::default())
            .with_shell_working_directory(dir.path())
            .with_shell_approval_handler(Arc::new(AllowAll))
    })
    .unwrap_or_else(|failure| panic!("{}", failure.error));

    let greeting = composed.frontmatter().as_map().get("greeting").and_then(Value::as_str).unwrap_or_default().to_string();
    assert!(greeting.starts_with("hello git version"), "{greeting:?}");
    assert!(composed.content().contains("hello git version"), "{}", composed.content());
}

#[test]
fn composing_never_edits_the_source_file() {
    let dir = TempDir::new().unwrap();

    compose(dir.path(), GREETING);

    assert_eq!(std::fs::read_to_string(dir.path().join("doc.md")).unwrap(), GREETING);
}

#[test]
fn a_written_null_takes_the_default() {
    let dir = TempDir::new().unwrap();

    let composed = compose(dir.path(), "---\n$schema:\n  x: string(required; default(hello))\nx: null\n---\n{{ x }}\n");

    assert_eq!(composed.frontmatter().as_map().get("x"), Some(&json!("hello")));
    assert!(composed.content().contains("hello"), "{}", composed.content());
}

#[test]
fn an_optional_default_is_inserted() {
    let dir = TempDir::new().unwrap();

    let composed = compose(dir.path(), "---\n$schema:\n  x: string(default(hello))\n---\n{{ x }}\n");

    assert_eq!(composed.frontmatter().as_map().get("x"), Some(&json!("hello")), "{:?}", composed.frontmatter().as_map());
    assert!(composed.content().contains("hello"), "{}", composed.content());
}

#[test]
fn a_nested_default_fills_a_present_parent_only() {
    let schema = "$schema:\n  server: \"{ name: string(required; default(web)), port: number }\"\n";
    let dir = TempDir::new().unwrap();

    let present = compose(dir.path(), &format!("---\n{schema}server:\n  port: 80\n---\n{{{{ server.name }}}}\n"));
    let absent = compose(dir.path(), &format!("---\n{schema}---\nBody\n"));

    assert_eq!(present.frontmatter().as_map().get("server"), Some(&json!({ "port": 80, "name": "web" })));
    assert!(present.content().contains("web"), "{}", present.content());
    // An absent optional parent stays absent: no key is written for it.
    assert!(!absent.frontmatter().as_map().contains_key("server"), "{:?}", absent.frontmatter().as_map());
}

/// Composes `content` as `doc.md` in `dir` under the Darkmatter baseline.
fn compose_with_baseline(dir: &Path, content: &str) -> Markdown {
    try_compose_at(dir, "doc.md", content, ComposeOptions::with_darkmatter_baseline_schema)
        .unwrap_or_else(|failure| panic!("{content:?} failed: {}", failure.error))
        .0
}

/// The baseline's `hash.kind` default fills a written `hash` mapping, and the
/// stored-hash reader then judges the completed value: the default supplied
/// `kind`, so what remains missing is `value`. An unwritten or `null` `hash`
/// gains nothing and reads as no stored hash.
#[test]
fn a_written_hash_mapping_gains_its_kind_default() {
    let dir = TempDir::new().unwrap();
    let options = darkmatter::markdown::hash::MdHashOptions::default();

    let written = compose_with_baseline(dir.path(), "---\nhash: {}\n---\nBody\n");
    assert_eq!(written.frontmatter().as_map().get("hash"), Some(&json!({ "kind": "simple" })));
    let error = written.stored_hash(&options).expect_err("a hash with no value is malformed");
    assert!(error.to_string().contains("'value'"), "{error}");

    let absent = compose_with_baseline(dir.path(), "---\ntitle: T\n---\nBody\n");
    assert!(!absent.frontmatter().as_map().contains_key("hash"));
    assert!(absent.stored_hash(&options).unwrap().is_none());

    let null = compose_with_baseline(dir.path(), "---\nhash: null\n---\nBody\n");
    assert_eq!(null.frontmatter().as_map().get("hash"), Some(&Value::Null));
    assert!(null.stored_hash(&options).unwrap().is_none());
}

/// A written `style.markdown` mapping gains every cleanup rule default, a
/// written rule and a written `null` rule keep (or take) theirs, and the
/// cleanup rule reader resolves the completed mapping to exactly the runtime
/// defaults plus the written rule. An unwritten `style` gains nothing.
#[test]
fn a_written_markdown_style_mapping_gains_the_cleanup_defaults() {
    use darkmatter::markdown::cleanup::rules::{ResolvedMarkdownRules, RuleSource, resolve, rules_from_json};

    let dir = TempDir::new().unwrap();
    let composed = compose_with_baseline(
        dir.path(),
        "---\nstyle:\n  markdown:\n    indentation: 2\n    unwrap: null\n---\nBody\n",
    );
    let markdown = &composed.frontmatter().as_map()["style"]["markdown"];
    assert_eq!(markdown["indentation"], json!(2), "{markdown}");
    assert_eq!(markdown["unwrap"], json!(false), "a written null takes the default: {markdown}");
    assert_eq!(markdown["default-language"], json!("text"), "{markdown}");
    assert_eq!(markdown["table-layout"], json!(true), "{markdown}");

    let overrides = rules_from_json(markdown).expect("the completed rules read back");
    let resolved = resolve(&Default::default(), Some(&overrides), None);
    let defaults = ResolvedMarkdownRules::defaults();
    assert_eq!(resolved.indentation.source, RuleSource::Document);
    assert_ne!(resolved.indentation.value, defaults.indentation.value);
    assert_eq!(resolved.unwrap.value, defaults.unwrap.value);
    assert_eq!(resolved.default_language.value, defaults.default_language.value);
    assert_eq!(resolved.table_layout.value, defaults.table_layout.value);
    assert_eq!(resolved.list_item_padding.value, defaults.list_item_padding.value);

    let unwritten = compose_with_baseline(dir.path(), "---\ntitle: T\n---\nBody\n");
    assert!(!unwritten.frontmatter().as_map().contains_key("style"));
    let empty_style = compose_with_baseline(dir.path(), "---\nstyle: {}\n---\nBody\n");
    assert_eq!(empty_style.frontmatter().as_map().get("style"), Some(&json!({})), "no `markdown` is created");
}

/// A repository whose trigger matches `stage: draft` and requires `level`
/// with a default.
fn trigger_repository() -> TempDir {
    let dir = TempDir::new().unwrap();
    // A repository marker without spawning `git`.
    write(dir.path(), ".git/HEAD", "ref: refs/heads/main\n");
    write(dir.path(), ".git/config", "[core]\n\trepositoryformatversion = 0\n");
    std::fs::create_dir_all(dir.path().join(".git/objects")).unwrap();
    std::fs::create_dir_all(dir.path().join(".git/refs")).unwrap();
    write(
        dir.path(),
        "schemas/drafts.yaml",
        "kind: schema-trigger\nmatch:\n  stage: literal(draft; required)\n$schema:\n  level: number(required; default(3))\n",
    );
    dir
}

#[test]
fn a_trigger_supplied_default_is_applied() {
    let dir = trigger_repository();

    let (composed, _) = try_compose_at(dir.path(), "docs/doc.md", "---\nstage: draft\n---\nLevel {{ level }}\n", |options| {
        options.with_trigger_schemas(true)
    })
    .unwrap_or_else(|failure| panic!("{}", failure.error));

    assert!(composed.content().contains("Level 3"), "{}", composed.content());
}

#[test]
fn a_trigger_a_default_newly_matches_is_evaluated_and_its_defaults_applied() {
    let dir = trigger_repository();
    let document = "---\n$schema:\n  stage: string(required; default(draft))\n---\n{{ stage }} {{ level }}\n";

    let (composed, _) = try_compose_at(dir.path(), "docs/doc.md", document, |options| options.with_trigger_schemas(true))
        .unwrap_or_else(|failure| panic!("{}", failure.error));

    assert!(composed.content().contains("draft 3"), "{}", composed.content());
}

#[test]
fn an_invalid_default_is_reported_at_the_schema_default_without_an_authored_position() {
    let dir = TempDir::new().unwrap();

    let failure = try_compose(dir.path(), "---\n$schema:\n  n: 'number(required; default(\"x\"))'\n---\nBody\n")
        .expect_err("the default is not a number");

    let problems = schema_problems(&failure);
    assert_eq!(problems.len(), 1, "{problems:?}");
    let problem = &problems[0];
    assert_eq!((problem.line, problem.column), (None, None), "{problem:?}");
    assert_eq!(problem.path, "/n");
    assert_eq!(problem.schema_path.as_ref().map(|pointer| pointer.as_pointer_string()).as_deref(), Some("/properties/n/default"));
    assert!(problem.message.contains("default of property `n`"), "{}", problem.message);
}

#[test]
fn a_missing_required_property_without_a_default_still_fails() {
    let dir = TempDir::new().unwrap();

    let failure = try_compose(dir.path(), "---\n$schema:\n  name: string(required)\n---\n{{ name }}\n")
        .expect_err("name is required");

    assert_eq!(schema_problems(&failure)[0].code, ValidationProblemCode::MissingRequired);
}

#[test]
fn passive_validation_reports_defaults_and_leaves_the_input_unchanged() {
    let dir = TempDir::new().unwrap();
    let path = write(dir.path(), "doc.md", "---\n$schema:\n  x: string(required; default(hello))\n---\nBody\n");
    let markdown = markdown_at(&path);
    let schemas = DarkmatterSchemas::new(context_at(dir.path()));

    let report = schemas.validate(&markdown).expect("schema prepares");
    let effective: EffectiveSchema = schemas.effective_for(&markdown).expect("schema prepares").expect("a schema applies");
    let input = json!({});
    let direct = effective.validate(&input);

    assert!(report.valid, "{:?}", report.problems);
    assert_eq!(report.defaults, [applied("x", json!("hello"))]);
    assert!(!markdown.frontmatter().as_map().contains_key("x"));
    assert!(direct.valid, "{:?}", direct.problems);
    assert_eq!(direct.defaults, [applied("x", json!("hello"))]);
    assert_eq!(input, json!({}));
}

#[test]
fn an_unconditional_pass_reports_no_defaults() {
    let dir = TempDir::new().unwrap();
    let path = write(dir.path(), "doc.md", "---\n$schema:\n  x: string(required; default(hello))\nx: given\n---\nBody\n");

    let report = DarkmatterSchemas::new(context_at(dir.path())).validate(&markdown_at(&path)).expect("schema prepares");

    assert!(report.valid);
    assert!(report.defaults.is_empty());
}

#[test]
fn yaml_document_validation_reports_defaults() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "settings-schema.yaml", "kind: schema\n$schema:\n  name: string(required)\n  retries: number(required; default(3))\n");
    let text = "$schema: ./settings-schema.yaml\nname: app\n";
    let path = write(dir.path(), "settings.yaml", text);

    let outcome = DarkmatterSchemas::new(context_at(dir.path())).validate_yaml_document(text, &path).expect("selects");

    let YamlOutcome::Validated(validation) = outcome else {
        panic!("a schema applies");
    };
    assert!(validation.report.valid, "{:?}", validation.report.problems);
    assert_eq!(validation.report.defaults, [applied("retries", json!(3))]);
}

/// Whole-YAML validation follows the same rule as frontmatter: an optional
/// default fills a missing value, a written `null` takes its default, a
/// required `null` without one is judged by type, and the file is untouched.
#[test]
fn yaml_document_validation_defaults_missing_and_null_values() {
    let dir = TempDir::new().unwrap();
    write(
        dir.path(),
        "settings-schema.yaml",
        "kind: schema\n$schema:\n  name: string(required)\n  retries: number(default(3))\n  mode: string(default(auto))\n",
    );
    let schemas = DarkmatterSchemas::new(context_at(dir.path()));

    let text = "$schema: ./settings-schema.yaml\nname: app\nmode: null\n";
    let path = write(dir.path(), "settings.yaml", text);
    let YamlOutcome::Validated(validation) = schemas.validate_yaml_document(text, &path).expect("selects") else {
        panic!("a schema applies");
    };
    assert!(validation.report.valid, "{:?}", validation.report.problems);
    assert_eq!(validation.report.defaults, [applied("retries", json!(3)), applied("mode", json!("auto"))]);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), text);

    let text = "$schema: ./settings-schema.yaml\nname: null\n";
    let path = write(dir.path(), "null-name.yaml", text);
    let YamlOutcome::Validated(validation) = schemas.validate_yaml_document(text, &path).expect("selects") else {
        panic!("a schema applies");
    };
    assert!(!validation.report.valid);
    assert!(validation.report.problems.iter().any(|problem| problem.path == "/name"), "{:?}", validation.report.problems);
}

fn checks(declared: &DeclaredTypes<'_>, expression: &str) -> Vec<CallCheck> {
    let expr = parse_spanned(expression).expect("the expression parses");
    check_expression_calls(&expr, &TypeScope::unguarded(declared))
}

#[test]
fn a_defaulted_property_is_typed_present_whether_or_not_it_is_required() {
    let dir = TempDir::new().unwrap();
    let path = write(
        dir.path(),
        "doc.md",
        "---\n$schema:\n  count: number(required; default(4))\n  spare: number(default(4))\n  bare: number\n---\nBody\n",
    );
    let effective = DarkmatterSchemas::new(context_at(dir.path()))
        .effective_for(&markdown_at(&path))
        .expect("schema prepares")
        .expect("a schema applies");
    let declared = DeclaredTypes::for_schema(Some(&effective));
    let scope = TypeScope::unguarded(&declared);
    let type_of = |name: &str| scope.type_of_expr(&parse(name).expect("parses")).expect("a subject").shape().clone();

    // Composition fills a missing or `null` value with its default, required
    // or not, so only the property without a default reads as `null`.
    assert!(!type_of("count").admits_null());
    assert!(!type_of("spare").admits_null());
    assert!(type_of("bare").admits_null());
    assert_eq!(checks(&declared, "min(count, 5)")[0].category(), ArgumentCategory::Compatible);
    assert_eq!(checks(&declared, "min(spare, 5)")[0].category(), ArgumentCategory::Compatible);
    assert_eq!(checks(&declared, "min(bare, 5)")[0].category(), ArgumentCategory::NullableMatch);
}

#[test]
fn composing_a_required_default_raises_no_presence_warning() {
    let dir = TempDir::new().unwrap();

    let (composed, report) =
        try_compose(dir.path(), "---\n$schema:\n  count: number(required; default(4))\n---\n{{ min(count, 5) }}\n")
            .unwrap_or_else(|failure| panic!("{}", failure.error));

    assert!(composed.content().contains('4'), "{}", composed.content());
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
}
