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
        .compose_with(options)
        .map(|(composed, report)| (composed.content().trim_end().to_string(), report))
}

#[track_caller]
fn body(content: &str) -> String {
    compose(content).unwrap_or_else(|error| panic!("{content:?} failed: {error}")).0
}

// ── An absent property is null ──────────────────────────────────────────────

/// Whole-value frontmatter keeps the type of its single span, so an absent
/// bare property and an absent `doc.<p>` both store `null`, which the body
/// then tests and renders as such.
#[test]
fn an_absent_property_is_a_null_whole_value() {
    let rendered = body(
        "---\nbare: \"{{ missing }}\"\nqualified: \"{{ doc.missing }}\"\n---\n\
         {{ is_null(bare) }} {{ is_null(qualified) }} [{{ bare }}][{{ qualified }}]\n",
    );
    assert_eq!(rendered, "true true [][]");
}

#[test]
fn an_absent_property_is_empty_in_a_mixed_string() {
    assert_eq!(body("---\ntitle: T\n---\n[{{ missing }}] [{{ doc.missing }}]\n"), "[] []");
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
/// stays reachable through `doc.`.
#[test]
fn an_exact_namespace_root_is_never_a_document_property() {
    let rendered = body("---\nenv: authored\n---\n{{ doc.env }} {{ is_object(env) }}\n");
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
