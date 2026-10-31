//! Permanent compatibility audit for the dasherized-identifier grammar change.
//!
//! The joined-dash rule (`darkmatter/features/2026-09-15-dasherized-identifiers`)
//! turns `a-b` from subtraction into one identifier. That is breaking for any
//! shipped expression spelling arithmetic as an identifier-like operand, an
//! unspaced `-`, and an identifier-continuation character (`iteration-1`).
//!
//! This test walks every shipped prompt/command artifact and classifies each
//! executable Darkmatter surface through the library's own extraction — never
//! a regex — so a dash inside a string literal (`'review-' + iteration`) is
//! never mistaken for an operator. Expression-typed frontmatter values are
//! found by resolving each document's effective schema and asking
//! [`frontmatter_expression_values`], the same classification DMLS uses for
//! `dm.expression.*` diagnostics, so a property of any name counts. The surviving gate is **gate B**: every
//! shipped expression parses. Typo detection is not this test's job; unknown
//! roots are the runtime warning's.
//!
//! Gate A (no shipped expression uses the breaking form) was a pre-change gate
//! (spec Resolved Decision 12). It passed before the lexer change landed and
//! was retired with it: under the joined-dash grammar the token shape it
//! looked for cannot occur. Its fixtures now pin that the same inputs lex as
//! one `Variable`.
//!
//! Excluded by construction: `{{{ … }}}` interpolation literals and fenced or
//! indented code blocks (both skipped by [`ExpressionFinder`]), and GitHub
//! Actions `${{ … }}` spans.
//!
//! A second gate, **condition call-check compatibility**, guards the unified
//! `||` grammar (`2026-10-16-unified-logical-or`): condition
//! surfaces once parsed `a || b` as a boolean `or(a, b)` call and now parse it
//! as a `LogicalOr` node, whose type is its operands'. Every shipped condition
//! is call-checked twice under its document's declarations, as parsed and
//! with each `LogicalOr` projected back to the former `or(…)` call, and the
//! two sets of rejected calls must be equal.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::directives_api::scan_darkmatter_directives;
use darkmatter::lifecycle::registry::EventRegistry;
use darkmatter::lifecycle::shell_template::lifecycle_declared_types;
use darkmatter::markdown::compose::expression::{
    DeclaredTypes, ExpressionFinder, SpannedExpr, Token, TypeScope, check_expression_calls, lex_spanned, parse,
    parse_spanned,
};
use darkmatter::markdown::compose::{
    ComposeSource, FrontmatterShellBody, parse_frontmatter_shell_value_spanned,
};
use darkmatter::markdown::schemas::{DarkmatterSchemas, EffectiveSchema, effective_property_shape};
use simplified_schema::expression::ast::{SpannedCallArgument, SpannedExprKind};
use simplified_schema::frontmatter_expression_values;
use serde_json::Value;

/// The executable surface an expression was extracted from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Surface {
    BodyInterpolation,
    FrontmatterInterpolation,
    WhenDirective,
    /// A frontmatter value whose property the document's effective schema
    /// types as `expression`.
    SchemaExpression,
    /// A Claudine lifecycle `when` or loop `while`/`until` frontmatter value.
    ConditionKey,
    ShellTernaryCondition,
    ShellTernaryValueBranch,
}

#[derive(Debug, Clone)]
struct CorpusExpression {
    path: PathBuf,
    surface: Surface,
    text: String,
}

impl fmt::Display for CorpusExpression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} [{:?}] `{}`", self.path.display(), self.surface, self.text)
    }
}

/// Frontmatter keys Claudine evaluates as Darkmatter conditions at lifecycle
/// time, per `claudine/schemas/lifecycle.yaml` (the recursive conditional
/// families and loop arms) and `claudine/schemas/action.yaml`
/// (`bundle.when`).
///
/// This is not schema-typed discovery: those schemas apply by trigger, which
/// this corpus walk does not enable, so [`frontmatter_expression_values`]
/// cannot see these keys, and Claudine applies them wherever they nest. The
/// list protects that separate lifecycle semantics;
/// `condition_keys_match_the_shipped_claudine_schema` pins it.
const CONDITION_KEYS: [&str; 3] = ["when", "while", "until"];

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is `<repo>/darkmatter/lib`.
    biscuit_test_harness::manifest_dir!()
        .ancestors()
        .nth(2)
        .expect("repository root is two levels above darkmatter/lib")
        .to_path_buf()
}

/// The four shipped prompt/command directories, walked recursively so a newly
/// shipped artifact is covered the day it lands.
fn corpus_dirs() -> [PathBuf; 4] {
    let root = repo_root();
    [
        root.join("prompts"),
        root.join(".claude/commands"),
        root.join("darkmatter/prompts"),
        root.join("claudine/prompts"),
    ]
}

fn corpus_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|error| panic!("{} is unreadable: {error}", dir.display()));
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            corpus_files(&path, files);
        } else if path.extension().is_some_and(|ext| ext == "md" || ext == "yaml") {
            files.push(path);
        }
    }
}

/// The shipped corpus: its files, every executable expression, and how many
/// documents resolved a document-level SimplifiedSchema (so a broken schema
/// resolution path cannot pass by classifying nothing).
struct Corpus {
    files: Vec<PathBuf>,
    expressions: Vec<CorpusExpression>,
    schema_documents: usize,
}

fn corpus_paths() -> Vec<PathBuf> {
    let mut files = Vec::new();
    for dir in corpus_dirs() {
        corpus_files(&dir, &mut files);
    }
    files.sort();
    files
}

fn collect_corpus() -> Corpus {
    let files = corpus_paths();
    let mut expressions = Vec::new();
    let mut schema_documents = 0;
    for path in &files {
        let classified = classify_file(path);
        schema_documents += usize::from(classified.has_document_schema);
        expressions.extend(classified.expressions);
    }
    Corpus { files, expressions, schema_documents }
}

struct ClassifiedFile {
    expressions: Vec<CorpusExpression>,
    has_document_schema: bool,
    /// The document's effective schema; `None` for a YAML artifact or a
    /// document no schema layer applies to.
    effective: Option<EffectiveSchema>,
}

/// Every executable expression in one artifact, by surface.
fn classify_file(path: &Path) -> ClassifiedFile {
    let source = fs::read_to_string(path).expect("artifact is readable");
    let mut expressions = Vec::new();
    let mut push = |surface, text: &str| {
        expressions.push(CorpusExpression { path: path.to_path_buf(), surface, text: text.to_string() });
    };
    if path.extension().is_some_and(|ext| ext == "yaml") {
        // A YAML artifact is data: its string values are scanned exactly as
        // frontmatter values are.
        let document: serde_yaml_ng::Value = serde_yaml_ng::from_str(&source)
            .unwrap_or_else(|error| panic!("{} is not YAML: {error}", path.display()));
        let document = serde_json::to_value(document).expect("YAML projects to JSON");
        frontmatter_expressions(None, &document, &mut push);
        return ClassifiedFile { expressions, has_document_schema: false, effective: None };
    }

    let document = Markdown::try_from_content(source)
        .unwrap_or_else(|error| panic!("{} does not parse: {error}", path.display()))
        .with_source(ComposeSource::File(path.to_path_buf()));
    let effective = schema_expressions(path, &document, &mut push);
    for (key, value) in document.frontmatter().as_map() {
        frontmatter_expressions(Some(key), value, &mut push);
    }
    body_expressions(document.content(), &mut push);
    let has_document_schema = effective.as_ref().is_some_and(|effective| effective.simplified.is_some());
    ClassifiedFile { expressions, has_document_schema, effective }
}

/// Pushes every Expression-typed frontmatter value under the document's
/// effective schema (Darkmatter baseline plus its own `$schema`, resolved
/// passively as DMLS does), and returns that effective schema.
///
/// A value still holding `$(` or `{{` is pending, exactly as the `expression`
/// format validator defers it; its interpolations and ternaries are audited
/// by the other surfaces.
fn schema_expressions(path: &Path, document: &Markdown, push: &mut impl FnMut(Surface, &str)) -> Option<EffectiveSchema> {
    let mut schemas = DarkmatterSchemas::new(crate::request_support::cwd_context())
        .with_darkmatter_baseline_json_schema()
        .expect("the Darkmatter baseline schema loads");
    if let Some(dir) = path.parent() {
        schemas = schemas.with_file_ref_fallback_dir(dir.to_path_buf());
    }
    let effective = schemas
        .effective_for(document)
        .unwrap_or_else(|error| panic!("{}: the effective schema does not resolve: {error}", path.display()));
    let simplified = effective.as_ref().and_then(|effective| effective.simplified.as_ref());
    let frontmatter: serde_json::Map<String, Value> = document
        .frontmatter()
        .as_map()
        .iter()
        .filter(|(key, _)| key.as_str() != "$schema")
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    let frontmatter = Value::Object(frontmatter);
    let shape = effective_property_shape(&[], simplified, &frontmatter);
    for value in frontmatter_expression_values(&shape, &frontmatter) {
        if !value.expression.contains("$(") && !value.expression.contains("{{") {
            push(Surface::SchemaExpression, &value.expression);
        }
    }
    effective
}

fn body_expressions(body: &str, push: &mut impl FnMut(Surface, &str)) {
    for location in ExpressionFinder::new(body).scan().expressions {
        if !is_github_actions_span(body, location.start) {
            push(Surface::BodyInterpolation, &location.expression);
        }
    }
    for directive in scan_darkmatter_directives(body) {
        for option in directive.options {
            if option.key.value == "when"
                && let Some(value) = option.value
            {
                push(Surface::WhenDirective, &value.value);
            }
        }
    }
}

fn frontmatter_expressions(key: Option<&str>, value: &Value, push: &mut impl FnMut(Surface, &str)) {
    match value {
        Value::String(text) => {
            // Claudine's lifecycle conditions, not schema-typed discovery; see
            // `CONDITION_KEYS`.
            if key.is_some_and(|key| CONDITION_KEYS.contains(&key)) {
                push(Surface::ConditionKey, text);
                return;
            }
            if let Some(shell) = parse_frontmatter_shell_value_spanned(text)
                && let FrontmatterShellBody::Ternary(ternary) = shell.body
            {
                push(Surface::ShellTernaryCondition, &text[ternary.condition_span]);
                for branch in [&text[ternary.then_span], &text[ternary.else_span]] {
                    if is_value_branch(branch) {
                        push(Surface::ShellTernaryValueBranch, branch);
                    }
                }
            }
            for location in ExpressionFinder::scan_plain(text).expressions {
                if !is_github_actions_span(text, location.start) {
                    push(Surface::FrontmatterInterpolation, &location.expression);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                frontmatter_expressions(key, item, push);
            }
        }
        Value::Object(map) => {
            for (key, value) in map {
                frontmatter_expressions(Some(key), value, push);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}

/// `${{ … }}` is GitHub Actions syntax, not a Darkmatter interpolation.
fn is_github_actions_span(text: &str, start: usize) -> bool {
    text[..start].ends_with('$')
}

/// Whether a `$()` ternary branch resolves through the expression engine.
///
/// The executor's §2 ladder sends a quoted/numeric/boolean literal, a
/// `name(...)` call, a `doc.*` reference, or a lone bare name to the expression
/// engine; a multi-word branch is a shell pipeline. Whether a lone bare name is
/// on `PATH` is host-dependent, so every whitespace-free branch is audited.
fn is_value_branch(branch: &str) -> bool {
    if !branch.contains(char::is_whitespace) {
        return true;
    }
    lex_spanned(branch).is_ok_and(|tokens| {
        matches!(
            tokens.as_slice(),
            [first, second, ..] if matches!(first.value, Token::Variable(_)) && second.value == Token::LParen
        )
    })
}

/// Every `Variable` token in `text` whose name contains a joined `-`.
///
/// ## Errors
///
/// Returns the lexer's message when `text` does not lex.
fn dashed_variables(text: &str) -> Result<Vec<String>, String> {
    let tokens = lex_spanned(text).map_err(|error| error.to_string())?;
    Ok(tokens
        .into_iter()
        .filter_map(|token| match token.value {
            Token::Variable(name) if name.contains('-') => Some(name),
            _ => None,
        })
        .collect())
}

fn parses(expression: &CorpusExpression) -> Result<(), String> {
    parse(&expression.text).map(drop).map_err(|error| error.to_string())
}

/// Gate B's per-expression verdict: one line per expression that does not
/// parse.
fn gate_b_failures(expressions: &[CorpusExpression]) -> Vec<String> {
    expressions
        .iter()
        .filter_map(|expression| parses(expression).err().map(|error| format!("{expression}: {error}")))
        .collect()
}

/// Gate B: every shipped executable expression parses. Parse-only; an unknown
/// root is the runtime warning's concern, not this gate's.
#[test]
fn shipped_corpus_expressions_all_parse() {
    let Corpus { files, expressions, .. } = collect_corpus();
    assert!(files.len() >= 50, "the shipped corpus shrank to {} files", files.len());
    assert!(
        expressions.len() >= 100,
        "the shipped corpus shrank to {} executable expressions",
        expressions.len()
    );
    let failures = gate_b_failures(&expressions);
    assert!(failures.is_empty(), "shipped expressions do not parse:\n{}", failures.join("\n"));
}

/// Every surface the walk classifies is actually present in the shipped
/// corpus, so a broken extractor cannot pass the gates by finding nothing.
/// `$()` ternaries are absent from today's corpus; their extraction is pinned
/// by `ternary_conditions_and_value_branches_are_extracted` instead. No
/// shipped schema declares an `expression` property today, so
/// `SchemaExpression` is pinned by the `schema_typed_*` fixtures; the walk
/// must still resolve document schemas, or it could not find one when it
/// ships.
#[test]
fn corpus_walk_reaches_every_present_surface() {
    let Corpus { expressions, schema_documents, .. } = collect_corpus();
    assert!(schema_documents > 0, "the corpus walk resolved no document `$schema`");
    for surface in [
        Surface::BodyInterpolation,
        Surface::FrontmatterInterpolation,
        Surface::WhenDirective,
        Surface::ConditionKey,
    ] {
        assert!(
            expressions.iter().any(|expression| expression.surface == surface),
            "the corpus walk found no {surface:?} expressions"
        );
    }
}

#[test]
fn condition_keys_match_the_shipped_claudine_schema() {
    let read = |path: PathBuf| -> serde_yaml_ng::Value {
        let source = fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        serde_yaml_ng::from_str(&source).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
    };
    let lifecycle = read(repo_root().join("claudine/schemas/lifecycle.yaml"));
    let action = read(repo_root().join("claudine/schemas/action.yaml"));
    for (file, types, type_name, key) in [
        ("lifecycle.yaml", &lifecycle, "conditional", "when"),
        ("lifecycle.yaml", &lifecycle, "initialize-conditional", "when"),
        ("lifecycle.yaml", &lifecycle, "cancelled-conditional", "when"),
        ("lifecycle.yaml", &lifecycle, "loop-bundle", "while"),
        ("lifecycle.yaml", &lifecycle, "loop-bundle", "until"),
        ("lifecycle.yaml", &lifecycle, "loop-single", "while"),
        ("lifecycle.yaml", &lifecycle, "loop-single", "until"),
        ("action.yaml", &action, "bundle", "when"),
    ] {
        let definition = types["types"][type_name][key]
            .as_str()
            .unwrap_or_else(|| panic!("{file} no longer declares `{type_name}.{key}`"));
        assert!(
            definition.starts_with("expression"),
            "`{type_name}.{key}` in {file} is no longer typed as an expression: {definition}"
        );
    }
}

/// Gate A's fixtures: each was a breaking-form subtraction before the
/// joined-dash grammar and is now one identifier, with no `-` operator left.
#[test]
fn former_breaking_forms_now_lex_as_one_variable() {
    for (text, expected) in [
        // The motivating incident, verbatim.
        ("spec-name", vec!["spec-name"]),
        ("iteration-1", vec!["iteration-1"]),
        // The collision gate A found in `prompts/_reviews/review-spec-inline.md`.
        ("depends-on", vec!["depends-on"]),
        ("fm.spec-name", vec!["fm.spec-name"]),
        ("a-_b", vec!["a-_b"]),
        ("false-1", vec!["false-1"]),
        ("true-value", vec!["true-value"]),
        ("phase-2 > 0 && level-3", vec!["phase-2", "level-3"]),
    ] {
        assert_eq!(dashed_variables(text), Ok(expected.iter().map(ToString::to_string).collect()), "`{text}`");
        let tokens = lex_spanned(text).expect("fixture lexes");
        assert!(tokens.iter().all(|token| token.value != Token::Minus), "`{text}` still lexes a `-` operator");
    }
}

/// Gate A's negative fixtures: spaced, numeric, non-identifier, double-dash,
/// and string-literal dashes never join into an identifier.
#[test]
fn subtraction_and_literal_dashes_never_join() {
    for text in [
        "a - b",
        "a -b",
        "a- b",
        "4-2",
        "f(x)-1",
        "arr[0]-1",
        "(a)-1",
        "\"x\"-1",
        "false - 1",
        "-5",
        "a * -1",
        "_iter_count - 1",
        // The second `-` cannot continue an identifier.
        "foo--bar",
        // Dashes inside string literals are never operators.
        "'review-' + iteration",
        "dirname(spec) + '/review-plan-' + iteration + '.md'",
        "fm['spec-name']",
    ] {
        assert_eq!(dashed_variables(text), Ok(vec![]), "`{text}`");
    }
}

/// The exact inputs that broke shipped prompts, which gate B must keep out of
/// the corpus.
#[test]
fn gate_b_rejects_the_corpus_failures_found_in_phase_one() {
    for (surface, text) in [
        (Surface::WhenDirective, "!ctx.is_monorepo\""),
        (Surface::WhenDirective, "file_exists({{fm.doc}})"),
        (Surface::FrontmatterInterpolation, "…"),
    ] {
        let expression = CorpusExpression { path: PathBuf::from("fixture.md"), surface, text: text.to_string() };
        assert!(parses(&expression).is_err(), "`{text}` must fail gate B");
    }
}

#[test]
fn extraction_skips_literals_fences_and_github_actions() {
    let body = "\
Real {{ spec-name }} and {{{ literal-name }}} and ${{ github.event-name }}.

```yaml
run: echo {{ fenced-name }}
```

::block when=\"phase-2\"
inside
::end-block
";
    let mut found = Vec::new();
    body_expressions(body, &mut |surface, text| found.push((surface, text.to_string())));
    assert_eq!(
        found,
        vec![
            (Surface::BodyInterpolation, "spec-name".to_string()),
            (Surface::WhenDirective, "phase-2".to_string()),
        ]
    );
}

#[test]
fn ternary_conditions_and_value_branches_are_extracted() {
    let frontmatter = serde_json::json!({
        "review": "$(has-review ? file_path(review-file) : git log --oneline)",
        "nested": { "until": "phase-1 >= total" },
        "list": ["{{ spec-name }}"],
    });
    let mut found = Vec::new();
    for (key, value) in frontmatter.as_object().unwrap() {
        frontmatter_expressions(Some(key), value, &mut |surface, text| found.push((surface, text.to_string())));
    }
    found.sort_by(|left, right| left.1.cmp(&right.1));
    assert_eq!(
        found,
        vec![
            (Surface::ShellTernaryValueBranch, "file_path(review-file)".to_string()),
            (Surface::ShellTernaryCondition, "has-review".to_string()),
            (Surface::ConditionKey, "phase-1 >= total".to_string()),
            (Surface::FrontmatterInterpolation, "spec-name".to_string()),
        ]
    );
    for (_, text) in &found {
        assert!(
            !dashed_variables(text).expect("fixture lexes").is_empty(),
            "fixture `{text}` must exercise a joined-dash identifier"
        );
    }
}

/// Writes a test-owned prompt whose `$schema` file declares an `expression`
/// property named `review-gate` (none of Claudine's condition keys), with
/// `value` as its authored content.
fn schema_typed_fixture(value: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("fixture directory");
    fs::write(dir.path().join("gate.schema.yaml"), "$schema:\n  review-gate: expression\n")
        .expect("fixture schema is writable");
    let prompt = dir.path().join("prompt.md");
    fs::write(&prompt, format!("---\n$schema: ./gate.schema.yaml\nreview-gate: \"{value}\"\n---\n\nBody.\n"))
        .expect("fixture prompt is writable");
    (dir, prompt)
}

#[test]
fn schema_typed_expressions_of_any_name_are_discovered() {
    let (_dir, prompt) = schema_typed_fixture("phase-2 > 0 && has-review");

    let classified = classify_file(&prompt);

    assert!(classified.has_document_schema);
    let found: Vec<(Surface, &str)> =
        classified.expressions.iter().map(|expression| (expression.surface, expression.text.as_str())).collect();
    assert_eq!(found, [(Surface::SchemaExpression, "phase-2 > 0 && has-review")]);
    assert!(gate_b_failures(&classified.expressions).is_empty());
}

#[test]
fn a_malformed_schema_typed_expression_fails_gate_b() {
    let (_dir, prompt) = schema_typed_fixture("phase-2 >");

    let classified = classify_file(&prompt);

    let failures = gate_b_failures(&classified.expressions);
    assert_eq!(failures.len(), 1, "{failures:?}");
    assert!(failures[0].contains("[SchemaExpression] `phase-2 >`"), "{}", failures[0]);
}

/// The surfaces whose `||` was lowered to a boolean `or(…)` call before the
/// unified grammar: everything once parsed in condition mode.
const CONDITION_SURFACES: [Surface; 4] =
    [Surface::ConditionKey, Surface::WhenDirective, Surface::ShellTernaryCondition, Surface::SchemaExpression];

/// `expr` in the representation condition parsing produced before the
/// unified grammar: every `LogicalOr` becomes `or(left, right)` over the
/// node's own span. Returns the projected tree and the spans of the calls it
/// synthesized, so an authored `or(…)` is still judged.
fn project_logical_or(expr: &SpannedExpr) -> (SpannedExpr, HashSet<(usize, usize)>) {
    fn project(expr: &mut SpannedExpr, synthesized: &mut HashSet<(usize, usize)>) {
        match &mut expr.kind {
            SpannedExprKind::Variable(_)
            | SpannedExprKind::StringLiteral(_)
            | SpannedExprKind::NumberLiteral(_)
            | SpannedExprKind::BoolLiteral(_) => {}
            SpannedExprKind::ArrayLiteral(items) | SpannedExprKind::FunctionCall { args: items, .. } => {
                items.iter_mut().for_each(|item| project(item, synthesized));
            }
            SpannedExprKind::ObjectLiteral(entries) => {
                entries.iter_mut().for_each(|(_, value)| project(value, synthesized));
            }
            SpannedExprKind::UnaryNot(inner)
            | SpannedExprKind::UnaryMinus(inner)
            | SpannedExprKind::Paren(inner)
            | SpannedExprKind::MemberAccess { base: inner, .. } => project(inner, synthesized),
            SpannedExprKind::Binary { left, right, .. }
            | SpannedExprKind::Comparison { left, right, .. }
            | SpannedExprKind::Index { base: left, index: right }
            | SpannedExprKind::LogicalOr { left, right } => {
                project(left, synthesized);
                project(right, synthesized);
            }
            SpannedExprKind::Ternary { condition, then_branch, else_branch } => {
                for branch in [condition, then_branch, else_branch] {
                    project(branch, synthesized);
                }
            }
            SpannedExprKind::CallbackCall { args, .. } => {
                for arg in args {
                    match arg {
                        SpannedCallArgument::Value(value) => project(value, synthesized),
                        SpannedCallArgument::Callback(callback) => project(&mut callback.body, synthesized),
                    }
                }
            }
        }
        if matches!(expr.kind, SpannedExprKind::LogicalOr { .. }) {
            let placeholder = SpannedExprKind::BoolLiteral(false);
            let SpannedExprKind::LogicalOr { left, right } = std::mem::replace(&mut expr.kind, placeholder) else {
                unreachable!("matched above");
            };
            expr.kind = SpannedExprKind::FunctionCall { name: "or".to_string(), args: vec![*left, *right] };
            synthesized.insert((expr.span.start, expr.span.end));
        }
    }
    let mut projected = expr.clone();
    let mut synthesized = HashSet::new();
    project(&mut projected, &mut synthesized);
    (projected, synthesized)
}

/// One call composition would reject: function name and call span.
type RejectedCall = (String, usize, usize);

/// The calls in `expr` composition rejects at `scope`, by the predicate
/// composition and lifecycle rejection share
/// ([`CallCheck::rejection_diagnostic`]), excluding calls at `synthesized`.
///
/// [`CallCheck::rejection_diagnostic`]: darkmatter::markdown::compose::expression::CallCheck::rejection_diagnostic
fn rejected_calls(expr: &SpannedExpr, scope: &TypeScope<'_>, synthesized: &HashSet<(usize, usize)>) -> BTreeSet<RejectedCall> {
    check_expression_calls(expr, scope)
        .into_iter()
        .filter(|check| check.rejection_diagnostic().is_some())
        .filter(|check| !(check.function == "or" && synthesized.contains(&(check.span.start, check.span.end))))
        .map(|check| (check.function, check.span.start, check.span.end))
        .collect()
}

/// The rejected calls of `text` as parsed and as the former `or(…)`
/// projection, when they differ.
///
/// ## Errors
///
/// Returns the parser's message when `text` does not parse (gate B's to
/// report).
fn call_verdict_change(text: &str, scope: &TypeScope<'_>) -> Result<Option<String>, String> {
    let parsed = parse_spanned(text).map_err(|error| error.to_string())?;
    let (former, synthesized) = project_logical_or(&parsed);
    let now = rejected_calls(&parsed, scope, &HashSet::new());
    let before = rejected_calls(&former, scope, &synthesized);
    Ok((now != before).then(|| format!("rejected as `or(…)`: {before:?}; rejected as `||`: {now:?}")))
}

/// A condition's site scope as far as the corpus can build it passively.
///
/// Lifecycle conditions get the declarations
/// [`lifecycle_call_checks`](darkmatter::lifecycle::lifecycle_call_checks)
/// gives them under Darkmatter's `md` host: Claudine's host globals belong to
/// a registry this crate cannot build, and the surface's event is not known
/// here, so those reads are untyped. Every other condition gets the
/// document's declarations, as the compose call precheck does. Enclosing
/// guards are not applied. Both trees are judged in the same scope, so a
/// missing narrowing can hide a change only where that narrowing alone would
/// decide the verdict.
fn condition_declarations<'s>(
    surface: Surface,
    effective: Option<&'s EffectiveSchema>,
    registry: &'s EventRegistry,
) -> DeclaredTypes<'s> {
    match surface {
        Surface::ConditionKey => lifecycle_declared_types(effective, registry),
        _ => DeclaredTypes::for_schema(effective),
    }
}

/// What the call-check gate compared over the shipped corpus.
#[derive(Default)]
struct ConditionComparison {
    /// Conditions compared, by surface.
    compared: BTreeMap<String, usize>,
    /// Conditions compared whose text contains `||`, by surface.
    with_logical_or: BTreeMap<String, usize>,
    /// Conditions present, by surface, whether or not they parse.
    present: BTreeMap<String, usize>,
    changes: Vec<String>,
}

fn compare_shipped_conditions() -> ConditionComparison {
    let registry = EventRegistry::md();
    let mut comparison = ConditionComparison::default();
    for path in corpus_paths() {
        let classified = classify_file(&path);
        for expression in classified.expressions.iter().filter(|expression| CONDITION_SURFACES.contains(&expression.surface)) {
            let surface = format!("{:?}", expression.surface);
            *comparison.present.entry(surface.clone()).or_default() += 1;
            let declared = condition_declarations(expression.surface, classified.effective.as_ref(), &registry);
            let Ok(change) = call_verdict_change(&expression.text, &TypeScope::unguarded(&declared)) else {
                continue;
            };
            *comparison.compared.entry(surface.clone()).or_default() += 1;
            if expression.text.contains("||") {
                *comparison.with_logical_or.entry(surface).or_default() += 1;
            }
            if let Some(change) = change {
                comparison.changes.push(format!("{expression}: {change}"));
            }
        }
    }
    comparison
}

/// Gate: every shipped condition keeps the passive call-check verdict it had
/// when its `||` was a boolean `or(…)` call.
#[test]
fn shipped_conditions_keep_their_call_check_verdict() {
    let comparison = compare_shipped_conditions();

    assert!(
        comparison.changes.is_empty(),
        "shipped conditions whose call-check verdict changed with `||`:\n{}",
        comparison.changes.join("\n")
    );
    assert_eq!(comparison.compared, comparison.present, "every shipped condition must parse and be compared");
    for surface in [Surface::ConditionKey, Surface::WhenDirective] {
        let surface = format!("{surface:?}");
        assert!(
            comparison.with_logical_or.get(&surface).is_some_and(|count| *count > 0),
            "no shipped {surface} condition uses `||`, so the comparison is vacuous there: {:?}",
            comparison.with_logical_or
        );
    }
}

/// The checker tells a valid condition from one with a rejected call, both
/// before and after the projection.
#[test]
fn call_check_distinguishes_a_rejected_condition_call() {
    let declared = DeclaredTypes::for_schema(None);
    let scope = TypeScope::unguarded(&declared);
    let none = HashSet::new();

    for text in ["!total_phases || total_phases <= 0", "length(true) > 0 || (!total_phases || total_phases <= 0)"] {
        assert_eq!(call_verdict_change(text, &scope), Ok(None), "`{text}`");
    }
    let rejected = |text: &str| rejected_calls(&parse_spanned(text).expect("fixture parses"), &scope, &none);
    assert!(rejected("!total_phases || total_phases <= 0").is_empty());
    assert_eq!(
        rejected("length(true) > 0 || (!total_phases || total_phases <= 0)"),
        BTreeSet::from([("length".to_string(), 0, 12)])
    );
}

/// The gate fails when `||` changes a verdict: as a boolean `or(…)` the
/// operand of `length` is rejected, as `||` it is not.
#[test]
fn call_verdict_comparison_reports_a_changed_verdict() {
    let declared = DeclaredTypes::for_schema(None);
    let scope = TypeScope::unguarded(&declared);

    let change = call_verdict_change("length(items || []) > 0", &scope).expect("fixture parses");

    let change = change.expect("the `or(…)` projection rejects `length`, `||` does not");
    assert!(change.contains("rejected as `or(…)`: {(\"length\", 0, 19)}"), "{change}");
    assert!(change.contains("rejected as `||`: {}"), "{change}");
}

/// The projection keeps an authored `or(…)` call and every span, and only
/// the synthesized calls are excluded from the verdict.
#[test]
fn projection_marks_only_synthesized_or_calls() {
    let parsed = parse_spanned("or(a, b || c) || d").expect("fixture parses");

    let (former, synthesized) = project_logical_or(&parsed);

    assert_eq!(synthesized, HashSet::from([(6, 12), (0, 18)]));
    assert_eq!(former.span, parsed.span);
    let SpannedExprKind::FunctionCall { name, args } = &former.kind else { panic!("{former:?}") };
    assert_eq!(name, "or");
    let SpannedExprKind::FunctionCall { name: authored, args: inner } = &args[0].kind else { panic!("{:?}", args[0]) };
    assert_eq!((authored.as_str(), args[0].span.clone()), ("or", 0..13));
    assert!(matches!(&inner[1].kind, SpannedExprKind::FunctionCall { name, .. } if name == "or"));
}
