//! Requirement 2 of the dasherized-identifiers spec
//! (`darkmatter/features/2026-09-15-dasherized-identifiers`): an expression
//! that cannot be parsed or evaluated fails full-document composition whatever
//! `fail_fast` says, with a typed error carrying its source path and the exact
//! authored span of the failing `{{ … }}`.
//!
//! The lenient best-effort surfaces keep their contracts:
//! `compose_subtree(..., Lenient)` and the public condition API.

use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::path::{Path, PathBuf};

use darkmatter::markdown::compose::conditions::evaluate_condition;
use darkmatter::markdown::compose::expression::{ExpressionError, evaluate, parse_condition};
use darkmatter::markdown::compose::shell_expansion::ShellExpansionOptions;
use darkmatter::markdown::compose::subtree::{SubtreeStrictness, compose_subtree};
use darkmatter::markdown::compose::{ComposeContext, ComposeOperation, ComposeOptions, EffectiveStateBuilder};
use darkmatter::markdown::{Markdown, MarkdownError, SourceRef};
use serde_json::json;
use tempfile::TempDir;

fn write(dir: &Path, name: &str, content: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, content).unwrap();
    path
}

fn options(dir: &Path, path: &Path, fail_fast: bool) -> ComposeOptions {
    ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir, ""))
        .with_source_file(path.to_path_buf())
        .with_fail_fast(fail_fast)
}

/// Composes `content` from a file under both `fail_fast` settings and returns
/// each error, requiring that composition failed and the file is untouched.
fn failures(content: &str) -> Vec<MarkdownError> {
    failures_with(content, |options| options)
}

/// [`failures`] with each run's options adjusted by `configure`.
fn failures_with(content: &str, configure: impl Fn(ComposeOptions) -> ComposeOptions) -> Vec<MarkdownError> {
    [false, true]
        .into_iter()
        .map(|fail_fast| {
            let dir = TempDir::new().unwrap();
            let path = write(dir.path(), "doc.md", content);
            let markdown = Markdown::try_from(path.as_path()).unwrap();
            let error = match markdown.compose_with(configure(options(dir.path(), &path, fail_fast))) {
                Ok((composed, report)) => panic!(
                    "fail_fast={fail_fast}: composed {:?} with warnings {:?}",
                    composed.content(),
                    report.warnings
                ),
                Err(error) => error,
            };
            assert_eq!(std::fs::read_to_string(&path).unwrap(), content, "the source is never rewritten");
            error
        })
        .collect()
}

/// The failure is anchored to the exact authored `{{ … }}`: `range` indexes
/// the file as loaded (frontmatter included), and `line`/`column` are
/// one-based.
#[track_caller]
fn assert_authored_span(source: &SourceRef, content: &str, construct: &str, range: Range<usize>, line: usize, column: usize) {
    let SourceRef::OnDiskSpan { context, span } = source else {
        panic!("expected the authored span, got {source:?}");
    };
    assert!(context.display.ends_with("doc.md"), "{:?}", context.display);
    assert_eq!(&*context.content, content, "the span indexes the file as loaded");
    assert_eq!(&content[range.clone()], construct, "the expected range is the construct");
    assert_eq!((span.range(), span.line(), span.column()), (range, line, column));
}

/// A body failure names no key and carries its authored span.
#[track_caller]
fn assert_body_failure<'a>(
    error: &'a MarkdownError,
    content: &str,
    failing: &str,
    range: Range<usize>,
    (line, column): (usize, usize),
) -> &'a ExpressionError {
    let MarkdownError::Interpolation { key, expression, source, cause } = error else {
        panic!("expected a typed interpolation error, got {error:?}");
    };
    assert_eq!(*key, None, "a body failure has no frontmatter key");
    assert_eq!(expression, failing);
    assert_authored_span(source, content, &format!("{{{{ {failing} }}}}"), range, line, column);
    cause
}

/// A frontmatter failure names its receiving key and carries its authored
/// span inside the value.
#[track_caller]
fn assert_frontmatter_failure<'a>(
    error: &'a MarkdownError,
    content: &str,
    receiving: &str,
    failing: &str,
    range: Range<usize>,
    (line, column): (usize, usize),
) -> &'a ExpressionError {
    let MarkdownError::Interpolation { key, expression, source, cause } = error else {
        panic!("expected a typed interpolation error, got {error:?}");
    };
    assert_eq!(key.as_deref(), Some(receiving));
    assert_eq!(expression, failing);
    assert_authored_span(source, content, &format!("{{{{ {failing} }}}}"), range, line, column);
    cause
}

// ── Body interpolation ──────────────────────────────────────────────────────

#[test]
fn a_body_parse_failure_is_fatal_whatever_fail_fast_says() {
    let content = "---\ntitle: Hello\n---\nintro\n\n{{ title }} {{ 1 + }}\n";
    for error in failures(content) {
        let cause = assert_body_failure(&error, content, "1 +", 40..49, (6, 13));
        assert!(matches!(cause, ExpressionError::Parse(_)), "{cause:?}");
    }
}

#[test]
fn a_body_evaluation_failure_is_fatal_whatever_fail_fast_says() {
    let content = "---\nflag: true\n---\nResult: {{ flag + 1 }}\n";
    for error in failures(content) {
        let cause = assert_body_failure(&error, content, "flag + 1", 27..41, (4, 9));
        assert!(cause.to_string().contains("Addition requires numeric operands"), "{cause}");
    }
}

/// A false page block removed text before the expression, shifting it in the
/// body the scanner sees; the error still carries the authored span.
#[test]
fn a_body_failure_after_a_removed_page_block_keeps_its_authored_span() {
    let content = "::block when=\"false\"\nhidden\n::end-block\n\n{{ min(1) }}\n";
    for error in failures(content) {
        assert_body_failure(&error, content, "min(1)", 41..53, (5, 1));
    }
}

/// Literal replacement lengthened text before the expression.
#[test]
fn a_body_failure_after_text_replacement_keeps_its_authored_span() {
    let content = "---\nreplace:\n  NAME: a much longer replacement\n---\nNAME and NAME\n\n{{ min(1) }}\n";
    for error in failures(content) {
        assert_body_failure(&error, content, "min(1)", 66..78, (7, 1));
    }
}

/// A rescan evaluates text a replacement produced, including text a
/// `{{{ … }}}` literal left in a value; a failure there is fatal too. It has
/// no authored span, so none is claimed.
#[test]
fn a_failure_in_replacement_output_is_fatal_without_an_authored_span() {
    for error in failures("---\nnote: \"call {{{ f( }}}\"\n---\nsee {{ note }}\n") {
        let MarkdownError::Interpolation { key: None, expression, source, cause } = &error else {
            panic!("expected a body interpolation error, got {error:?}");
        };
        assert_eq!(expression, "f(");
        assert!(matches!(cause.as_ref(), ExpressionError::Parse(_)), "{cause:?}");
        assert!(matches!(source.as_ref(), SourceRef::OnDisk(_)), "{source:?}");
    }
}

// ── Frontmatter interpolation ───────────────────────────────────────────────

#[test]
fn a_mixed_frontmatter_parse_failure_is_fatal_whatever_fail_fast_says() {
    let content = "---\nnote: \"x {{ > bad }} y\"\n---\nbody\n";
    for error in failures(content) {
        let cause = assert_frontmatter_failure(&error, content, "note", "> bad", 13..24, (2, 10));
        assert!(matches!(cause, ExpressionError::Parse(_)), "{cause:?}");
    }
}

#[test]
fn a_mixed_frontmatter_evaluation_failure_is_fatal_whatever_fail_fast_says() {
    let content = "---\nnote: \"x {{ min(1) }} y\"\n---\nbody\n";
    for error in failures(content) {
        let cause = assert_frontmatter_failure(&error, content, "note", "min(1)", 13..25, (2, 10));
        assert!(cause.to_string().contains("min() requires 2 arguments"), "{cause}");
    }
}

/// Escapes before the expression make the decoded value shorter than its
/// authored text; the span is projected through them.
#[test]
fn a_mixed_frontmatter_failure_after_escapes_spans_the_authored_bytes() {
    let content = "---\nnote: \"say \\\"hi\\\"\\t{{ min(1) }}\"\n---\nbody\n";
    for error in failures(content) {
        assert_frontmatter_failure(&error, content, "note", "min(1)", 23..35, (2, 20));
    }
}

/// A nested value is still frontmatter mixed text.
#[test]
fn a_nested_mixed_frontmatter_failure_is_fatal() {
    let content = "---\nmeta:\n  items:\n    - \"x {{ 1 + }}\"\n---\nbody\n";
    for error in failures(content) {
        assert_frontmatter_failure(&error, content, "meta", "1 +", 28..37, (4, 10));
    }
}

// ── Frontmatter values authored across lines ────────────────────────────────
//
// The span is projected through the same decoder DMLS uses
// (`decode_scalar_node`), which follows the frontmatter parser's block-scalar
// chomping, folding, and indentation rules.

#[test]
fn a_literal_block_scalar_parse_failure_spans_the_authored_bytes() {
    let content = "---\ntitle: Hi\nnote: |\n  first line\n  x {{ > bad }} y\n---\nbody\n";
    for error in failures(content) {
        let cause = assert_frontmatter_failure(&error, content, "note", "> bad", 39..50, (5, 5));
        assert!(matches!(cause, ExpressionError::Parse(_)), "{cause:?}");
    }
}

/// The folded line and the multibyte text before the expression on its own
/// line both shift decoded offsets away from authored bytes; the column counts
/// characters.
#[test]
fn a_folded_block_scalar_evaluation_failure_after_multibyte_text_spans_the_authored_bytes() {
    let content = "---\nnote: >-\n  intro\n  café — {{ min(1) }} y\n\n  tail\n---\nbody\n";
    for error in failures(content) {
        let cause = assert_frontmatter_failure(&error, content, "note", "min(1)", 33..45, (4, 10));
        assert!(cause.to_string().contains("min() requires 2 arguments"), "{cause}");
    }
}

#[test]
fn a_keep_chomped_crlf_block_scalar_failure_spans_the_authored_bytes() {
    let content = "---\r\nnote: |+\r\n  a\r\n  b {{ 1 + }}\r\n\r\n---\r\nbody\r\n";
    for error in failures(content) {
        assert_frontmatter_failure(&error, content, "note", "1 +", 24..33, (4, 5));
    }
}

/// An explicit indentation indicator counts from the nested key's column, so
/// the extra space is content and the span starts after it.
#[test]
fn a_nested_block_scalar_with_an_indentation_indicator_spans_the_authored_bytes() {
    let content = "---\nmeta:\n  note: |2\n     {{ min(1) }}\n---\nbody\n";
    for error in failures(content) {
        assert_frontmatter_failure(&error, content, "meta", "min(1)", 26..38, (4, 6));
    }
}

#[test]
fn multi_line_plain_and_double_quoted_failures_span_the_authored_bytes() {
    let plain = "---\nnote: first\n  then {{ 1 + }}\n---\nbody\n";
    for error in failures(plain) {
        assert_frontmatter_failure(&error, plain, "note", "1 +", 23..32, (3, 8));
    }
    let quoted = "---\nnote: \"one \\\n  two\n\n  {{ min(1) }}\"\n---\nbody\n";
    for error in failures(quoted) {
        assert_frontmatter_failure(&error, quoted, "note", "min(1)", 26..38, (5, 3));
    }
}

/// A block scalar elsewhere in the frontmatter does not stop an ordinary
/// value after it from being located.
#[test]
fn a_block_scalar_before_the_failing_value_does_not_hide_its_span() {
    let content = "---\nintro: |\n  hello\nnote: \"x {{ 1 + }}\"\n---\nbody\n";
    for error in failures(content) {
        assert_frontmatter_failure(&error, content, "note", "1 +", 30..39, (4, 10));
    }
}

// ── Frontmatter values behind a tag, an anchor, or an alias ─────────────────
//
// A tag or anchor is authored before the scalar and is never part of the
// span. An alias authors no text of its own: its failure is anchored in the
// scalar that defines its anchor, the one place the expression is written.

#[test]
fn a_tagged_quoted_parse_failure_spans_the_authored_bytes_after_the_tag() {
    let content = "---\ntitle: !!str \"café — {{ upper( }}\"\n---\nbody\n";
    for error in failures(content) {
        let cause = assert_frontmatter_failure(&error, content, "title", "upper(", 28..40, (2, 22));
        assert!(matches!(cause, ExpressionError::Parse(_)), "{cause:?}");
    }
}

#[test]
fn a_verbose_tagged_plain_evaluation_failure_spans_the_authored_bytes_after_the_tag() {
    let content = "---\nnote: !<tag:yaml.org,2002:str> naïve {{ min(1) }} tail\n---\nbody\n";
    for error in failures(content) {
        let cause = assert_frontmatter_failure(&error, content, "note", "min(1)", 42..54, (2, 38));
        assert!(cause.to_string().contains("min() requires 2 arguments"), "{cause}");
    }
}

#[test]
fn a_tagged_crlf_block_scalar_failure_spans_the_authored_bytes() {
    let content = "---\r\nnote: !!str |\r\n  café\r\n  x {{ 1 + }}\r\n---\r\nbody\r\n";
    for error in failures(content) {
        assert_frontmatter_failure(&error, content, "note", "1 +", 33..42, (4, 5));
    }
}

#[test]
fn anchored_failures_span_the_authored_bytes_after_the_anchor_and_tag() {
    let anchored = "---\nnote: &shared \"café {{ min(1) }}\"\n---\nbody\n";
    for error in failures(anchored) {
        assert_frontmatter_failure(&error, anchored, "note", "min(1)", 25..37, (2, 21));
    }
    let anchor_then_tag = "---\nnote: &shared !!str x {{ 1 + }}\n---\nbody\n";
    for error in failures(anchor_then_tag) {
        assert_frontmatter_failure(&error, anchor_then_tag, "note", "1 +", 26..35, (2, 23));
    }
    let tag_then_anchor = "---\nmeta:\n  - !!str &item |-\n    café\n    {{ min(1) }}\n---\nbody\n";
    for error in failures(tag_then_anchor) {
        assert_frontmatter_failure(&error, tag_then_anchor, "meta", "min(1)", 43..55, (5, 5));
    }
}

/// A tag that makes the value a non-string, and a tagged collection, are not
/// expression text and do not stop another value from being located.
#[test]
fn a_non_string_tag_elsewhere_does_not_hide_a_failing_values_span() {
    let content = "---\ncount: !!int \"5\"\nflow: !!seq [a, b]\nlist: !!seq\n  - a\nmap: &shape !!map\n  k: v\nempty: !!str\nnote: \"x {{ 1 + }}\"\n---\nbody\n";
    for error in failures(content) {
        assert_frontmatter_failure(&error, content, "note", "1 +", 105..114, (9, 10));
    }
}

/// The anchored value fails first in document order, and the alias reports
/// the same authored span when it is the value that fails (the anchor's key
/// is deferred here, so only the alias is interpolated).
#[test]
fn an_alias_failure_spans_the_scalar_defining_its_anchor() {
    let content = "---\ndefaults:\n  note: &shared \"café — {{ min(1) }}\"\ntitle: *shared\n---\nbody\n";
    for error in failures(content) {
        assert_frontmatter_failure(&error, content, "defaults", "min(1)", 41..53, (3, 25));
    }
    for error in failures_with(content, |options| options.with_exclude_keys(["defaults"])) {
        let cause = assert_frontmatter_failure(&error, content, "title", "min(1)", 41..53, (3, 25));
        assert!(cause.to_string().contains("min() requires 2 arguments"), "{cause}");
    }

    let crlf = "---\r\nzeta: &shared |\r\n  café\r\n  {{ 1 + }}\r\nalpha: *shared\r\n---\r\nbody\r\n";
    for error in failures_with(crlf, |options| options.with_exclude_keys(["zeta"])) {
        let cause = assert_frontmatter_failure(&error, crlf, "alpha", "1 +", 33..42, (4, 3));
        assert!(matches!(cause, ExpressionError::Parse(_)), "{cause:?}");
    }
}

/// YAML resolves an alias to the last definition before it. Choosing between
/// two `&shared` occurrences takes a full parse, so the failure is anchored
/// at the alias token, where the value is certainly referenced, rather than
/// at a guessed definition. The column counts characters (`café`), and a
/// trailing comment is not part of the token.
#[test]
fn an_alias_of_an_unprovable_anchor_spans_the_alias_token() {
    let redefined = "---\nold: &shared \"{{ min(1) }}\"\nnew: &shared \"{{ min(1) }}\"\ncafé: *shared\n---\nbody\n";
    let in_a_comment = "---\nold: &shared \"{{ min(1) }}\" # not &shared again\ncafé: *shared # trailing\n---\nbody\n";
    for (content, range, position) in [(redefined, 67..74, (4, 7)), (in_a_comment, 59..66, (3, 7))] {
        for error in failures_with(content, |options| options.with_exclude_keys(["old", "new"])) {
            let MarkdownError::Interpolation { key, expression, source, cause } = &error else {
                panic!("expected a typed interpolation error, got {error:?}");
            };
            assert_eq!((key.as_deref(), expression.as_str()), (Some("café"), "min(1)"));
            assert!(cause.to_string().contains("min() requires 2 arguments"), "{cause}");
            assert_authored_span(source, content, "*shared", range.clone(), position.0, position.1);
        }
    }
}

/// The alias-token fallback is for aliases only: a value the locator cannot
/// reach (here a flow sequence spread over lines) still claims no span.
#[test]
fn an_unlocatable_value_that_is_no_alias_still_claims_no_span() {
    let content = "---\nlist: [\n  \"x {{ min(1) }}\"\n  ]\n---\nbody\n";
    for error in failures(content) {
        let MarkdownError::Interpolation { key, source, .. } = &error else {
            panic!("expected a typed interpolation error, got {error:?}");
        };
        assert_eq!(key.as_deref(), Some("list"));
        assert!(matches!(source.as_ref(), SourceRef::OnDisk(_)), "{source:?}");
    }
}

// ── Surfaces that were already strict stay strict ───────────────────────────

#[test]
fn a_whole_value_frontmatter_failure_stays_fatal() {
    let content = "---\nnote: \"{{ min(1) }}\"\n---\nbody\n";
    for error in failures(content) {
        assert_frontmatter_failure(&error, content, "note", "min(1)", 11..23, (2, 8));
    }
}

#[test]
fn a_page_block_condition_failure_stays_fatal() {
    for error in failures("before\n\n::block when=\"1 +\"\ninside\n::end-block\n") {
        assert!(matches!(error, MarkdownError::PageBlock(_)), "{error:?}");
    }
}

#[test]
fn a_transclusion_condition_failure_stays_fatal() {
    for fail_fast in [false, true] {
        let dir = TempDir::new().unwrap();
        write(dir.path(), "child.md", "child\n");
        let path = write(dir.path(), "doc.md", "root\n\n::file ./child.md when=\"min(1)\"\n");
        let error = Markdown::try_from(path.as_path())
            .unwrap()
            .compose_with(options(dir.path(), &path, fail_fast))
            .expect_err("a bad transclusion condition fails the document");
        assert!(matches!(error, MarkdownError::Transclusion(_)), "{error:?}");
    }
}

#[test]
fn a_directive_target_failure_stays_fatal() {
    let content = "before\n\n::file {{ min(1) }}\n";
    for error in failures(content) {
        assert_body_failure(&error, content, "min(1)", 15..27, (3, 8));
    }
}

#[test]
fn shell_ternary_condition_and_branch_failures_stay_fatal() {
    for value in ["$(min(1) ? 'a' : 'b')", "$(true ? 'a {{ 1 + }}' : 'b')"] {
        for fail_fast in [false, true] {
            let dir = TempDir::new().unwrap();
            let path = write(dir.path(), "doc.md", &format!("---\nflag: \"{value}\"\n---\nbody\n"));
            let options = options(dir.path(), &path, fail_fast).with_shell(ShellExpansionOptions {
                policy_root: Some(dir.path().to_path_buf()),
                ..Default::default()
            });
            Markdown::try_from(path.as_path())
                .unwrap()
                .compose_with(options)
                .expect_err("a bad `$()` ternary fails the document");
        }
    }
}

// ── Lenient contracts outside full-document composition ─────────────────────

/// `compose_subtree(..., Lenient)` is best-effort data interpolation and is
/// unchanged; `Strict` stays strict.
#[test]
fn subtree_strictness_keeps_its_contract() {
    let state = EffectiveStateBuilder::new()
        .with_frontmatter([("name".to_string(), json!("x"))].into())
        .build()
        .unwrap();

    for (value, preserved) in [
        (json!("{{ name }} {{ > broken }}"), "x {{ > broken }}"),
        (json!("{{ name }} {{ min(1) }}"), "x {{ min(1) }}"),
    ] {
        let lenient = compose_subtree(&value, &state, HashMap::new(), SubtreeStrictness::Lenient)
            .expect("the lenient subtree keeps going past a bad span");
        assert_eq!(lenient, json!(preserved));
        compose_subtree(&value, &state, HashMap::new(), SubtreeStrictness::Strict)
            .expect_err("the strict subtree rejects a bad span");
    }
}

/// The public condition API returns failures to the caller, which owns their
/// disposition (Claudine: fatal loop conditions, warn-and-skip hook `when=`).
#[test]
fn the_public_condition_api_still_returns_failures_as_results() {
    let state = EffectiveStateBuilder::new().build().unwrap();
    let context = biscuit_terminal::errors::SourceContext::new(
        PathBuf::from("/doc.md"),
        PathBuf::from("doc.md"),
        "",
    );

    assert!(evaluate_condition("1 +", &state, 1, context.clone()).is_err());
    assert!(evaluate_condition("min(1)", &state, 1, context.clone()).is_err());
    assert_eq!(evaluate_condition("missing", &state, 1, context).ok(), Some(false));

    assert!(parse_condition("1 +").is_err());
    let expression = parse_condition("min(1)").unwrap();
    assert!(evaluate(&expression, &state).is_err());
}

/// Preflight discovery tolerates a bad span so it can still enumerate the
/// commands a document will run; the real composition then fails on it.
#[test]
fn preflight_discovery_tolerates_what_composition_rejects() {
    let dir = TempDir::new().unwrap();
    let path = write(
        dir.path(),
        "doc.md",
        "---\nnote: \"x {{ 1 + }}\"\nrev: \"$(git rev-parse HEAD)\"\n---\nbody\n",
    );
    let markdown = Markdown::try_from(path.as_path()).unwrap();
    let shell = ShellExpansionOptions { policy_root: Some(dir.path().to_path_buf()), ..Default::default() };

    let commands = darkmatter::markdown::compose::collect_shell_commands(
        &markdown,
        &options(dir.path(), &path, false).with_shell(shell.clone()),
    )
    .expect("preflight discovery tolerates the bad mixed-text span");
    assert!(
        commands.iter().any(|entry| entry.normalized == "git rev-parse HEAD"),
        "{commands:?}"
    );

    let approved = HashSet::from(["git rev-parse HEAD".to_string()]);
    markdown
        .compose_with(options(dir.path(), &path, false).with_shell(shell).with_pre_approved_commands(approved))
        .expect_err("the real composition rejects the bad span");
}

/// Discovery composes without page blocks, so it evaluates a region a false
/// `::block` removes. A failure there must not reject a document whose real
/// composition never reaches it.
#[test]
fn preflight_discovery_does_not_fail_on_a_region_composition_removes() {
    let dir = TempDir::new().unwrap();
    let path = write(
        dir.path(),
        "doc.md",
        "---\nrev: \"$(git rev-parse HEAD)\"\n---\nkept\n\n::block when=\"false\"\n{{ min(1) }}\n::end-block\n",
    );
    let markdown = Markdown::try_from(path.as_path()).unwrap();
    let shell = ShellExpansionOptions { policy_root: Some(dir.path().to_path_buf()), ..Default::default() };

    let commands = darkmatter::markdown::compose::collect_shell_commands(
        &markdown,
        &options(dir.path(), &path, false).with_shell(shell.clone()),
    )
    .expect("discovery tolerates a failure inside a removed region");
    assert!(commands.iter().any(|entry| entry.normalized == "git rev-parse HEAD"), "{commands:?}");

    let (composed, _) = markdown
        .compose_with(
            options(dir.path(), &path, false)
                .only(&[ComposeOperation::FrontmatterInterpolation, ComposeOperation::PageBlocks, ComposeOperation::Interpolation]),
        )
        .expect("the removed region is never evaluated");
    assert_eq!(composed.content().trim(), "kept");
}
