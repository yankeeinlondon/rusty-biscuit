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

/// Each default as `(path, value, schema location)`; the layer that
/// declared it is asserted where a test is about it.
fn located(defaults: &[AppliedDefault]) -> Vec<(String, Value, String)> {
    defaults.iter().map(|applied| (applied.path.to_string(), applied.value.clone(), applied.pointer.clone())).collect()
}

fn applied(path: &str, value: Value) -> (String, Value, String) {
    let path = PropertyPath::parse(path).expect("path");
    let pointer = path.segments().iter().map(|segment| format!("/properties/{segment}")).collect::<String>() + "/default";
    (path.to_string(), value, pointer)
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
    assert_eq!(located(&report.defaults), [applied("x", json!("hello"))]);
    assert!(!markdown.frontmatter().as_map().contains_key("x"));
    assert!(direct.valid, "{:?}", direct.problems);
    assert_eq!(located(&direct.defaults), [applied("x", json!("hello"))]);
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
    assert_eq!(located(&validation.report.defaults), [applied("retries", json!(3))]);
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
    assert_eq!(located(&validation.report.defaults), [applied("retries", json!(3)), applied("mode", json!("auto"))]);
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

/// The default paths `DarkmatterSchemas::validate` reports for a document
/// whose frontmatter is `frontmatter`, under the runtime baseline.
fn baseline_default_paths(frontmatter: &str) -> Vec<String> {
    let dir = TempDir::new().unwrap();
    let path = write(dir.path(), "doc.md", &format!("---\n{frontmatter}---\nBody\n"));

    let schemas = DarkmatterSchemas::new(context_at(dir.path())).with_darkmatter_baseline_json_schema().expect("baseline");
    let report = schemas.validate(&markdown_at(&path)).expect("schema prepares");

    assert!(report.valid, "{:?}", report.problems);
    report.defaults.iter().map(|applied| applied.path.to_string()).collect()
}

/// The shipped conditional selects `then` and `else` through a key pattern,
/// so the actions beneath them are reached only through `patternProperties`.
#[test]
fn a_lifecycle_action_inside_a_conditional_branch_reports_its_defaults() {
    let outside = baseline_default_paths("composed:\n  - edit: { file: notes.md }\n");
    let then = baseline_default_paths("composed:\n  - when: 'true'\n    then:\n      - edit: { file: notes.md }\n");
    // `else` needs a `then` beside it, which the schema checks as the
    // parser does.
    let otherwise = baseline_default_paths("composed:\n  - when: 'true'\n    then: [stop]\n    else:\n      - edit: { file: notes.md }\n");

    assert_eq!(outside, ["composed.0.no_error", "composed.0.edit.create_new", "composed.0.edit.no_error"]);
    for (branch, reported) in [("then", then), ("else", otherwise)] {
        let inner = [
            "composed.0.no_error".to_string(),
            format!("composed.0.{branch}.0.no_error"),
            format!("composed.0.{branch}.0.edit.create_new"),
            format!("composed.0.{branch}.0.edit.no_error"),
        ];
        assert_eq!(reported, inner, "the `{branch}` branch's action defaults are reported as they are outside it");
    }
}

/// The md lifecycle parser's verdict on `frontmatter` read as authored, as
/// a host reads a stack its caller supplied (Claudine's proxy overlay is
/// authored).
fn md_parser_accepts(frontmatter: &Value) -> bool {
    darkmatter::lifecycle::md_host::parse_lifecycle(frontmatter, Path::new("doc.md"), None).is_ok()
}

/// Composes `content` under the baseline with a caller data override of
/// `composed` (the way a host passes a caller's values). The lifecycle keys
/// are not deferred, so composition completes and judges the stack; a host
/// that defers them (`with_exclude_keys`) owns their verdict instead.
fn compose_with_composed_override(content: &str, override_value: Value) -> Result<(Markdown, ComposeReport), Box<ComposeFailure>> {
    let dir = TempDir::new().unwrap();
    try_compose_at(dir.path(), "doc.md", content, |options| {
        options.with_darkmatter_baseline_schema().with_data_overrides(json!({ "composed": override_value }))
    })
}

/// A caller override replaces an authored lifecycle stack before the
/// composition's schema verdict, and composition completes the replacement.
/// The flags of the replaced stack are not the override's: a `set` the
/// override supplies is not refused for the filled flag beside it, although
/// the document wrote a flag at the same place.
#[test]
fn an_overridden_lifecycle_stack_is_not_judged_by_the_flags_it_replaced() {
    let content = "---\ncomposed:\n  - info: hi\n    no_error: true\n---\nBody\n";
    let replacement = json!([{ "set": { "answer": 42 } }]);
    assert!(md_parser_accepts(&json!({ "composed": replacement })));
    let (composed, _) = compose_with_composed_override(content, replacement)
        .unwrap_or_else(|failure| panic!("the replacement is valid: {}", failure.error));
    assert_eq!(composed.frontmatter().as_map()["composed"][0]["no_error"], json!(false), "the replacement is completed");
}

/// An override's own placement is authored: a `no_error` beside `then` that
/// a caller supplies is refused, as the parser refuses it, even where the
/// document wrote no flag at the same place.
#[test]
fn an_overriding_lifecycle_stacks_own_placement_is_refused() {
    let content = "---\ncomposed:\n  - info: hi\n---\nBody\n";
    let replacement = json!([{ "when": "true", "then": ["stop"], "no_error": false }]);
    assert!(!md_parser_accepts(&json!({ "composed": replacement })));
    let failure = compose_with_composed_override(content, replacement).expect_err("the supplied flag beside `then` is refused");
    assert!(
        schema_problems(&failure).iter().any(|problem| problem.path == "/composed/0/no_error"),
        "{:?}",
        schema_problems(&failure)
    );
}

/// A transcluded document inherits its parent's completed values as data.
/// The parent judged its own stack; the filled flag beside its `set` is not
/// the child's placement.
#[test]
fn an_inherited_completed_lifecycle_stack_is_not_judged_again_as_authored() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "child.md", "---\ntitle: child\n---\nChild\n");
    let parent = "---\ncomposed:\n  - set: { answer: 42 }\n---\n::file ./child.md\n";
    let composed = compose_with_baseline(dir.path(), parent);
    assert!(composed.content().contains("Child"), "{}", composed.content());
}

/// Composition completes an authored stack before its verdict, and the
/// flags it filled are never placements; a written `null` flag is still
/// written, so `set` refuses it after completion as before.
#[test]
fn a_completed_lifecycle_stack_keeps_its_authored_verdict_through_composition() {
    let dir = TempDir::new().unwrap();
    let valid = "---\ncomposed:\n  - when: 'true'\n    then: [stop]\n  - edit: { file: notes.md }\n  - set: { answer: 42 }\n---\nBody\n";
    let composed = compose_with_baseline(dir.path(), valid);
    assert_eq!(composed.frontmatter().as_map()["composed"][0]["no_error"], json!(false), "the stack is completed");

    let refused = "---\ncomposed:\n  - set: { answer: 42 }\n    no_error: null\n---\nBody\n";
    let failure = try_compose_at(dir.path(), "refused.md", refused, ComposeOptions::with_darkmatter_baseline_schema)
        .expect_err("a written flag on `set` is refused");
    assert!(
        schema_problems(&failure).iter().any(|problem| problem.path == "/composed/0/no_error"),
        "{:?}",
        schema_problems(&failure)
    );
}

/// An eager file check the default's own structural check cannot run fails
/// on the completed value; the failure belongs to the schema default.
mod eager_file_defaults {
    use darkmatter::markdown::compose::report_diagnostics::error_diagnostics;
    use simplified_schema::SchemaOriginKind;

    use super::*;

    const EAGER_INLINE: &str = "---\n$schema:\n  doc: file(required; eager; default(./note.md))\n---\nBody {{ doc }}\n";

    #[track_caller]
    fn attributed(problems: &[ValidationProblem]) -> &AppliedDefault {
        let [problem] = problems else { panic!("one problem expected, got {problems:?}") };
        assert_eq!(problem.code, ValidationProblemCode::InvalidFileReference);
        assert_eq!(problem.path, "/doc");
        assert_eq!((problem.line, problem.column), (None, None), "no authored text holds a default");
        assert!(problem.message.contains("schema default at schema location `/properties/doc/default`"), "{}", problem.message);
        let default = problem.default.as_ref().expect("the problem names its default");
        assert_eq!((default.path.to_string(), default.pointer.as_str()), ("doc".to_string(), "/properties/doc/default"));
        default
    }

    #[test]
    fn composition_attributes_a_missing_eager_default_target_to_the_default() {
        let dir = TempDir::new().unwrap();

        let failure = try_compose(dir.path(), EAGER_INLINE).expect_err("note.md does not exist");

        attributed(schema_problems(&failure));
        let [diagnostic] = error_diagnostics(&failure.error).try_into().expect("one record");
        assert_eq!(diagnostic.code, "dm.schema.invalid_file_reference");
        let source = diagnostic.source.as_ref().expect("the document is named");
        assert_eq!((source.line, source.column), (None, None), "no authored position is invented");
        let origin = diagnostic.schema_default.as_ref().expect("the record names the default");
        assert_eq!((origin.path.as_str(), origin.schema_location.as_str()), ("doc", "/properties/doc/default"));
        assert_eq!(origin.schema_file, None, "the document's own inline schema declares it");
    }

    #[test]
    fn composition_accepts_an_eager_default_whose_target_exists() {
        let dir = TempDir::new().unwrap();
        write(dir.path(), "note.md", "note\n");

        let composed = compose(dir.path(), EAGER_INLINE);

        assert!(composed.frontmatter().as_map().contains_key("doc"), "the default is inserted");
    }

    #[test]
    fn a_lazy_file_default_composes_whether_or_not_its_target_exists() {
        let dir = TempDir::new().unwrap();
        let lazy = "---\n$schema:\n  doc: file(required; default(./note.md))\n---\nBody {{ doc }}\n";

        assert!(compose(dir.path(), lazy).content().contains("Body ./note.md"));
        write(dir.path(), "note.md", "note\n");
        assert!(compose(dir.path(), lazy).content().contains("Body ./note.md"));
    }

    #[test]
    fn a_default_from_a_referenced_schema_file_names_that_file() {
        let dir = TempDir::new().unwrap();
        let schema = write(dir.path(), "schemas/doc-schema.yaml", "$schema:\n  doc: file(required; eager; default(./note.md))\n");
        let path = write(dir.path(), "doc.md", "---\n$schema: ./schemas/doc-schema.yaml\n---\nBody\n");
        let schema_file = biscuit_file::canonicalize_simplified(&schema).expect("schema file");
        let canonical = |file: &Path| biscuit_file::canonicalize_simplified(file).expect("a named schema file exists");

        let report = DarkmatterSchemas::new(context_at(dir.path())).validate(&markdown_at(&path)).expect("schema prepares");
        let failure = try_compose(dir.path(), "---\n$schema: ./schemas/doc-schema.yaml\n---\nBody\n").expect_err("note.md does not exist");

        assert!(!report.valid);
        let passive = attributed(&report.problems);
        assert_eq!(passive.origin.as_ref().map(|origin| origin.kind), Some(SchemaOriginKind::ReferencedFile));
        let named = passive.origin.as_ref().and_then(|origin| origin.file()).expect("the schema file is named");
        assert_eq!(canonical(named), schema_file);
        assert!(report.problems[0].message.contains(&named.display().to_string()), "{}", report.problems[0].message);
        attributed(schema_problems(&failure));
        let [diagnostic] = error_diagnostics(&failure.error).try_into().expect("one record");
        let origin = diagnostic.schema_default.as_ref().expect("the record names the default");
        assert_eq!(origin.schema_file.as_deref().map(canonical), Some(schema_file));
    }

    /// Named types whose eager file defaults name a missing target.
    const IMPORTED_TYPES: &str = "kind: schema\ntypes:\n  Doc: file(required; eager; default(./note.md))\n  Holder:\n    target: file(required; eager; default(./note.md))\n";

    #[track_caller]
    fn canonical(file: &Path) -> std::path::PathBuf {
        biscuit_file::canonicalize_simplified(file).expect("a named schema file exists")
    }

    #[track_caller]
    fn declaring_file(applied: &AppliedDefault) -> std::path::PathBuf {
        canonical(applied.origin.as_ref().and_then(|origin| origin.file()).expect("a schema file is named"))
    }

    #[test]
    fn a_default_imported_through_a_named_type_names_the_file_that_declares_it() {
        let dir = TempDir::new().unwrap();
        let types = canonical(&write(dir.path(), "types/types.yaml", IMPORTED_TYPES));
        let document = "---\n$schema:\n  doc: Doc@./types/types.yaml\n---\nBody\n";
        let path = write(dir.path(), "doc.md", document);

        let report = DarkmatterSchemas::new(context_at(dir.path())).validate(&markdown_at(&path)).expect("schema prepares");
        let failure = try_compose(dir.path(), document).expect_err("note.md does not exist");

        let passive = attributed(&report.problems);
        assert_eq!(declaring_file(passive), types, "the importing document declares no default");
        assert!(report.problems[0].message.contains("types.yaml"), "{}", report.problems[0].message);
        attributed(schema_problems(&failure));
        let [diagnostic] = error_diagnostics(&failure.error).try_into().expect("one record");
        let origin = diagnostic.schema_default.as_ref().expect("the record names the default");
        assert_eq!(origin.schema_file.as_deref().map(canonical), Some(types));
    }

    #[test]
    fn a_nested_default_of_a_named_type_imported_by_a_schema_file_names_the_type_file() {
        let dir = TempDir::new().unwrap();
        let types = canonical(&write(dir.path(), "types/types.yaml", IMPORTED_TYPES));
        write(dir.path(), "schemas/doc-schema.yaml", "$schema:\n  holder: Holder@../types/types.yaml\n");
        let path = write(dir.path(), "doc.md", "---\n$schema: ./schemas/doc-schema.yaml\nholder: {}\n---\nBody\n");

        let report = DarkmatterSchemas::new(context_at(dir.path())).validate(&markdown_at(&path)).expect("schema prepares");

        let [problem] = report.problems.as_slice() else { panic!("one problem expected, got {:?}", report.problems) };
        assert_eq!(problem.path, "/holder/target");
        assert_eq!((problem.line, problem.column), (None, None), "no authored text holds a default");
        let default = problem.default.as_ref().expect("the problem names its default");
        // `holder` is optional, so it lowers to a null-or-object union.
        assert_eq!(default.pointer, "/properties/holder/anyOf/1/properties/target/default");
        assert_eq!(declaring_file(default), types, "the type file declares it, not the importing schema file");
    }

    /// The trigger cannot match in the first validation pass, where `stage`
    /// is still an unrun `$( … )`; only the post-shell pass matches it,
    /// inserts its eager default, and finds the default's target missing.
    #[test]
    fn a_default_a_post_shell_trigger_inserts_is_attributed_in_the_second_pass() {
        let dir = trigger_repository();
        let trigger = canonical(&write(
            dir.path(),
            "schemas/eager.yaml",
            "kind: schema-trigger\nmatch:\n  stage: literal(review; required)\n$schema:\n  doc: file(required; eager; default(./note.md))\n",
        ));
        write(dir.path(), "docs/stage.ini", "[doc]\n\tstage = review\n");
        let document = "---\nstage: $(git config --file stage.ini --get doc.stage)\n---\nBody\n";

        let failure = try_compose_at(dir.path(), "docs/doc.md", document, |options| {
            options
                .with_trigger_schemas(true)
                .with_config_snapshot(crate::request_support::shell_config(dir.path(), &[]))
                .with_shell(ShellExpansionOptions::default())
                .with_shell_working_directory(dir.path().join("docs"))
                .with_shell_approval_handler(Arc::new(AllowAll))
        })
        .expect_err("the trigger's default names a missing note.md");

        let default = attributed(schema_problems(&failure));
        assert_eq!(default.origin.as_ref().map(|origin| origin.kind), Some(SchemaOriginKind::Trigger));
        assert_eq!(declaring_file(default), trigger);
        let [diagnostic] = error_diagnostics(&failure.error).try_into().expect("one record");
        let source = diagnostic.source.as_ref().expect("the document is named");
        assert_eq!((source.line, source.column), (None, None), "no authored position is invented");
        let origin = diagnostic.schema_default.as_ref().expect("the record names the default");
        assert_eq!(origin.schema_location, "/properties/doc/default");
        assert_eq!(origin.schema_file.as_deref().map(canonical), Some(trigger));
    }

    #[test]
    fn passive_validation_attributes_a_missing_eager_default_target_to_the_default() {
        let dir = TempDir::new().unwrap();
        let path = write(dir.path(), "doc.md", EAGER_INLINE);

        let report = DarkmatterSchemas::new(context_at(dir.path())).validate(&markdown_at(&path)).expect("schema prepares");

        assert!(!report.valid);
        let default = attributed(&report.problems);
        assert_eq!(default.origin.as_ref().map(|origin| origin.kind), Some(SchemaOriginKind::Document));
    }

    #[test]
    fn an_authored_missing_eager_file_keeps_its_authored_position() {
        let dir = TempDir::new().unwrap();

        let failure = try_compose(dir.path(), "---\n$schema:\n  doc: file(required; eager; default(./note.md))\ndoc: ./absent.md\n---\nBody\n")
            .expect_err("absent.md does not exist");

        let [problem] = schema_problems(&failure) else { panic!("one problem") };
        assert!(problem.default.is_none());
        assert_eq!(problem.line, Some(4), "{problem:?}");
        let [diagnostic] = error_diagnostics(&failure.error).try_into().expect("one record");
        assert!(diagnostic.schema_default.is_none());
    }
}
