//! The absent-property contract of full-document composition (strict-mode
//! removal, R1 and R3): an absent document property is valid and evaluates to
//! `null`; it never reads `ctx`; and only real expression failures stop
//! composition.
//!
//! Every test composes a real document and asserts the rendered output.

use std::path::{Path, PathBuf};

use darkmatter::markdown::compose::{ComposeContext, ComposeOptions, ComposeReport};
use darkmatter::markdown::{Markdown, MarkdownError};
use tempfile::TempDir;

fn write(dir: &Path, name: &str, content: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, content).unwrap();
    path
}

/// Composes `content` as `doc.md` in a fresh directory.
fn compose(content: &str) -> Result<(String, ComposeReport), MarkdownError> {
    let dir = TempDir::new().unwrap();
    let path = write(dir.path(), "doc.md", content);
    let options = ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir.path(), ""))
        .with_source_file(path.clone());
    Markdown::try_from(path.as_path())
        .unwrap()
        .compose_with(&crate::request_support::request(options))
        .map(|(composed, report)| (composed.content().trim_end().to_string(), report))
}

#[track_caller]
fn body(content: &str) -> String {
    compose(content).unwrap_or_else(|error| panic!("{content:?} failed: {error}")).0
}

// ── An absent property is null ──────────────────────────────────────────────

/// Whole-value frontmatter keeps the type of its single span, so an absent
/// bare property and an absent `fm.<p>` both store `null`, which the body
/// then tests and renders as such.
#[test]
fn an_absent_property_is_a_null_whole_value() {
    let rendered = body(
        "---\nbare: \"{{ missing }}\"\nqualified: \"{{ fm.missing }}\"\n---\n\
         {{ is_null(bare) }} {{ is_null(qualified) }} [{{ bare }}][{{ qualified }}]\n",
    );
    assert_eq!(rendered, "true true [][]");
}

#[test]
fn an_absent_property_is_empty_in_a_mixed_string() {
    assert_eq!(body("---\ntitle: T\n---\n[{{ missing }}] [{{ fm.missing }}]\n"), "[] []");
    assert_eq!(
        body("---\nlabel: \"x {{ missing }} y\"\n---\n[{{ label }}]\n"),
        "[x  y]",
        "frontmatter mixed text"
    );
}

#[test]
fn an_absent_property_takes_the_falsy_branch_and_the_fallback() {
    let rendered = body(
        "---\nname: N\nblank: \"\"\n---\n\
         {{ missing ? 'yes' : 'no' }} \
         {{ missing || 'fallback' }} \
         {{ name || 'fallback' }} \
         {{ blank || 'fallback' }} \
         {{ missing || blank || 'last' }}\n",
    );
    // `||` yields its left operand when truthy, otherwise its right: a value,
    // not a boolean.
    assert_eq!(rendered, "no fallback N fallback last");
}

// ── Declared and undeclared properties are both valid ───────────────────────

#[test]
fn a_schema_declared_unset_property_and_an_undeclared_one_are_both_valid() {
    let rendered = body("---\n$schema:\n  declared: string\n---\n[{{ declared }}][{{ undeclared }}]\n");
    assert_eq!(rendered, "[][]");
}

#[test]
fn a_required_property_violation_still_blocks() {
    let error = compose("---\n$schema:\n  needed: string(required)\n---\n[{{ needed }}]\n")
        .expect_err("an unset required property fails schema validation");
    assert!(matches!(error, MarkdownError::SchemaValidationFailed { .. }), "{error:?}");
}

// ── No bare-name fallback to ctx ────────────────────────────────────────────

/// `today` is a captured context key, so before R3 a bare `{{ today }}`
/// rendered the date. It is a document property now: absent, so empty, in
/// the body, in frontmatter, and in a page-block condition alike.
#[test]
fn a_missing_bare_property_does_not_resolve_from_ctx() {
    let rendered = body(
        "---\nlabel: \"{{ today }}\"\n---\nA{{ today }}B{{ label }}C{{ is_empty(ctx.today) }}\n\
         ::block when=\"today\"\nhidden\n::end-block\n",
    );
    assert_eq!(rendered, "ABCfalse");

    // A document that declares the name reads its own value.
    assert_eq!(body("---\ntoday: mine\n---\n{{ today }}\n"), "mine");
}

/// A reserved namespace wins over a frontmatter key of the same name, which
/// stays reachable through `fm.`.
#[test]
fn an_exact_namespace_root_is_never_a_document_property() {
    let rendered = body("---\nenv: authored\n---\n{{ fm.env }} {{ is_object(env) }}\n");
    assert_eq!(rendered, "authored true");
}

// ── Real failures still fail ────────────────────────────────────────────────

#[test]
fn expression_failures_still_stop_composition() {
    for (span, what) in [
        ("{{ 1 + }}", "malformed syntax"),
        ("{{ not_a_function(1) }}", "an unknown function"),
        ("{{ min(1) }}", "a rejected argument count"),
        ("{{ upper(1, 2) }}", "rejected arguments"),
        ("{{ frontmatter('absent.md', 'title') }}", "a failed file read"),
    ] {
        for content in [format!("---\ntitle: T\n---\n{span}\n"), format!("---\nv: \"x {span}\"\n---\n{{{{ v }}}}\n")] {
            let error = compose(&content).expect_err(what);
            assert!(matches!(error, MarkdownError::Interpolation { .. }), "{what}: {error:?}");
        }
    }
}

// ── Escapes are inert, with no extra evaluation pass ────────────────────────

/// Triple braces store a literal `{{ … }}` as data, in a whole value and in a
/// mixed string; reading that value back never evaluates it, so its absent
/// root raises no advisory. Backslash escapes keep the body text verbatim.
#[test]
fn every_escape_form_is_inert() {
    let (rendered, report) = compose(
        "---\nwhole: \"{{{ ghost }}}\"\nmixed: \"a {{{ ghost }}} b\"\n---\n\
         {{ whole }}|{{ mixed }}|{{{ ghost }}}|\\{{ ghost }}|\\{\\{ ghost }}\n",
    )
    .unwrap();
    assert_eq!(rendered, "{{ ghost }}|a {{ ghost }} b|{{ ghost }}|\\{{ ghost }}|\\{\\{ ghost }}");
    assert!(report.warnings.is_empty(), "nothing was evaluated: {:?}", report.warnings);
}

// ── Declared is not written ─────────────────────────────────────────────────

/// A composed body, its frontmatter, and the report.
type Composed = (String, serde_json::Map<String, serde_json::Value>, ComposeReport);

/// Composes `parent` as `doc.md` beside `child.md`, which the parent
/// transcludes. Returns the composed body, its frontmatter, and the report.
fn compose_with_child(
    parent: &str,
    child: &str,
) -> Result<Composed, MarkdownError> {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "child.md", child);
    let path = write(dir.path(), "doc.md", parent);
    let options = ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir.path(), ""))
        .with_source_file(path.clone());
    Markdown::try_from(path.as_path())
        .unwrap()
        .compose_with(&crate::request_support::request_at(dir.path(), options))
        .map(|(composed, report)| {
            let frontmatter = composed.frontmatter().as_map().clone().into_iter().collect();
            (composed.content().trim_end().to_string(), frontmatter, report)
        })
}

/// The parent declares `title` as optional; each row writes it differently.
/// `title` is never an undeclared name in the parent, but an unwritten
/// `title` is not supplied to the child: no `null` key is added to reach the
/// interpolation's empty result.
fn parent(written: &str) -> String {
    format!("---\n$schema:\n  title: string\n{written}---\n[{{{{ title }}}}]\n\n::file child.md\n")
}

const UNKNOWN_CHILD: &str = "---\n$schema:\n  title: unknown(required)\n---\nchild\n";
const STRING_CHILD: &str = "---\n$schema:\n  title: string(required)\n---\nchild\n";

/// A child whose schema refuses the values it receives is not transcluded;
/// the parent reports that as a transclusion warning naming the child.
#[track_caller]
fn assert_child_refused(
    result: Result<Composed, MarkdownError>,
) {
    let (rendered, _, report) = result.expect("a refused child is a warning, not a parent failure");
    assert!(!rendered.contains("child\n") && !rendered.ends_with("child"), "{rendered}");
    assert!(
        report.warnings.iter().any(|warning| warning.stage == "transclusion"
            && warning.message.contains("child.md")
            && warning.message.contains("did not satisfy the schema")),
        "{:?}",
        report.warnings
    );
}

/// The `unknown(required)` row is the regression: a synthesized `null` would
/// meet its presence contract although the parent wrote nothing.
#[test]
fn an_unwritten_parent_property_is_missing_in_a_child_that_requires_it() {
    assert_child_refused(compose_with_child(&parent(""), UNKNOWN_CHILD));
    assert_child_refused(compose_with_child(&parent(""), STRING_CHILD));
}

/// A written `null` meets `required` but is still judged by the type.
#[test]
fn a_written_null_fails_a_string_child_s_type() {
    assert_child_refused(compose_with_child(&parent("title: null\n"), STRING_CHILD));
}

#[test]
fn an_unwritten_parent_property_renders_empty_without_a_warning_or_a_key() {
    let child = "---\n$schema:\n  title: unknown\n---\nchild\n";
    let (rendered, frontmatter, report) = compose_with_child(&parent(""), child).unwrap();
    assert!(rendered.starts_with("[]"), "{rendered}");
    assert!(!frontmatter.contains_key("title"), "composed frontmatter: {frontmatter:?}");
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
}

#[test]
fn a_written_null_meets_an_unknown_child_s_required_contract() {
    let (rendered, frontmatter, report) = compose_with_child(&parent("title: null\n"), UNKNOWN_CHILD).unwrap();
    assert!(rendered.starts_with("[]"), "{rendered}");
    assert_eq!(frontmatter.get("title"), Some(&serde_json::Value::Null));
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
}

#[test]
fn a_written_title_reaches_both_children() {
    for child in [UNKNOWN_CHILD, STRING_CHILD] {
        let (rendered, frontmatter, _) = compose_with_child(&parent("title: Hi\n"), child).unwrap();
        assert!(rendered.starts_with("[Hi]"), "{rendered}");
        assert_eq!(frontmatter.get("title"), Some(&serde_json::json!("Hi")));
    }
}

/// Composes `doc` in a fresh repository holding `files`, with trigger schemas
/// enabled, and returns the body, the composed frontmatter, and the warnings.
fn compose_in_repo(doc: &str, files: &[(&str, &str)]) -> (String, serde_json::Map<String, serde_json::Value>, Vec<String>) {
    let dir = TempDir::new().unwrap();
    // A repository marker without spawning `git`.
    std::fs::create_dir_all(dir.path().join(".git/objects")).unwrap();
    std::fs::create_dir_all(dir.path().join(".git/refs")).unwrap();
    for (name, content) in [(".git/HEAD", "ref: refs/heads/main\n"), (".git/config", "[core]\n\trepositoryformatversion = 0\n")]
        .iter()
        .chain(files)
    {
        let path = dir.path().join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }
    let base = dir.path().join("docs");
    std::fs::create_dir_all(&base).unwrap();
    let path = write(&base, "doc.md", doc);
    let options = ComposeOptions::new_with_context(ComposeContext::capture_for_content(&base, ""))
        .with_source_file(path.clone())
        .with_trigger_schemas(true);
    let (composed, report) = Markdown::try_from(path.as_path())
        .unwrap()
        .compose_with(&crate::request_support::request_at(&base, options))
        .unwrap_or_else(|error| panic!("{doc:?} failed: {error}"));
    (
        composed.content().trim_end().to_string(),
        composed.frontmatter().as_map().clone().into_iter().collect(),
        report.warnings.iter().map(|warning| warning.message.clone()).collect(),
    )
}

/// Every schema form declares `name` through the same authority, so an
/// unwritten `name` renders empty with no warning and gains no key; the
/// undeclared `ghost` beside it is the control that the warning is live.
#[test]
fn every_schema_form_declares_without_writing() {
    let raw = r#"{"$schema":"https://json-schema.org/draft/2020-12/schema","type":"object","properties":{"name":{"type":["string","null"]}}}"#;
    type Case<'a> = (&'a str, &'a str, Vec<(&'a str, &'a str)>);
    let cases: [Case<'_>; 4] = [
        ("raw JSON Schema", "$schema: ./schema.json\n", vec![("docs/schema.json", raw)]),
        ("referenced schema", "$schema: ./schema.yaml\n", vec![("docs/schema.yaml", "$schema:\n  name: string\n")]),
        ("root union", "$schema:\n  - name: string\n    kind: literal(a)\n  - kind: literal(b)\nkind: b\n", vec![]),
        (
            "trigger payload",
            "stage: draft\n",
            vec![("schemas/t.yaml", "kind: schema-trigger\nmatch:\n  stage: literal(draft; required)\n$schema:\n  name: string\n")],
        ),
    ];
    for (form, frontmatter, files) in cases {
        let doc = format!("---\n{frontmatter}---\n[{{{{ name }}}}]\n");
        let (rendered, composed, warnings) = compose_in_repo(&doc, &files);
        assert_eq!(rendered, "[]", "{form}");
        assert!(!composed.contains_key("name"), "{form}: {composed:?}");
        assert!(warnings.is_empty(), "{form}: {warnings:?}");

        let doc = format!("---\n{frontmatter}---\n[{{{{ name }}}}][{{{{ ghost }}}}]\n");
        let (_, _, warnings) = compose_in_repo(&doc, &files);
        assert!(warnings.iter().any(|message| message.contains("ghost")), "{form}: control {warnings:?}");
    }
}
