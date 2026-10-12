//! Cleanup never changes what a document composes to: the A1 and B1
//! reproductions (a block-quote and a list next to `::block`/`::end-block`)
//! compose to the same bytes before and after `clean_document` under the
//! defaults, with each gate both true and false. Under every other rule value
//! that edits them, the composed documents render to the same HTML: those
//! rules change spelling on purpose (`indentation: 8` pads a parent marker,
//! which compose's own cleanup keeps), never structure.
//!
//! The documents are fixture-owned strings written to a temporary directory;
//! the compose context captures no repository or environment group, and no
//! document runs a shell command or reads a remote.

use std::convert::Infallible;
use std::path::Path;

use darkmatter::markdown::Markdown;
use darkmatter::markdown::cleanup::rules::{
    FixedWidth, Indentation, Leaf, ListInteriorPadding, ListItemPadding, MarkdownRuleOverrides,
};
use darkmatter::markdown::cleanup::{CleanOutcome, CleanRequest, clean_document};
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions};
use darkmatter::markdown::schemas::CleanSchemaConfig;
use tempfile::TempDir;

/// A1 with an unlabeled fence after the block, so the defaults edit it.
const A1: &str = "::block when=\"a\"\n\n> note\n::end-block\n\n```\ncode\n```\n";
/// B1 as reported: a tight list continuing across a page block.
const B1: &str = "1. first\n::block when=\"m\"\n    - nested\n::end-block\n2. second\n";

fn profiles() -> Vec<(&'static str, MarkdownRuleOverrides)> {
    let with = |edit: fn(&mut MarkdownRuleOverrides)| {
        let mut overrides = MarkdownRuleOverrides::default();
        edit(&mut overrides);
        overrides
    };
    vec![
        ("defaults", MarkdownRuleOverrides::default()),
        ("indentation 2", with(|o| o.indentation = Leaf::Value(Indentation::Two))),
        ("indentation 8", with(|o| o.indentation = Leaf::Value(Indentation::Eight))),
        ("unwrap", with(|o| o.unwrap = Leaf::Value(true))),
        ("fixed-width 12", with(|o| o.fixed_width = Leaf::Value(FixedWidth::Columns(12)))),
        ("list-item-padding one", with(|o| o.list_item_padding = Leaf::Value(ListItemPadding::One))),
        ("list-interior-padding all", with(|o| o.list_interior_padding = Leaf::Value(ListInteriorPadding::All))),
    ]
}

fn clean(source: &str, overrides: &MarkdownRuleOverrides) -> (String, CleanOutcome) {
    let request = CleanRequest { source, overrides, workspace: None, document_path: None };
    let result = clean_document(&request, || {
        Ok::<_, Infallible>(CleanSchemaConfig::new(crate::request_support::cwd_context()))
    })
    .expect("clean runs");
    assert_ne!(result.outcome, CleanOutcome::Declined, "{:?}", result.diagnostics);
    (result.text, result.outcome)
}

fn compose(dir: &Path, name: &str, source: &str) -> String {
    let path = dir.join(name);
    std::fs::write(&path, source).unwrap();
    let options = ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir, ""))
        .with_source_file(path.clone());
    let (composed, _) = Markdown::try_from(path.as_path())
        .expect("document loads")
        .compose_with(&crate::request_support::request(options))
        .expect("the fixture composes");
    composed.content().to_string()
}

fn html(markdown: &str) -> String {
    let options = pulldown_cmark::Options::all() - pulldown_cmark::Options::ENABLE_SMART_PUNCTUATION;
    let mut out = String::new();
    pulldown_cmark::html::push_html(&mut out, pulldown_cmark::Parser::new_ext(markdown, options));
    out
}

#[track_caller]
fn assert_composes_the_same(body: &str, gate: &str) {
    let dir = TempDir::new().unwrap();
    let mut changed_by = Vec::new();
    for value in [true, false] {
        let source = format!("---\n{gate}: {value}\n---\n{body}");
        let expected = compose(dir.path(), "input.md", &source);
        for (label, overrides) in profiles() {
            let (cleaned, outcome) = clean(&source, &overrides);
            if outcome == CleanOutcome::Changed {
                changed_by.push(label);
            }
            let composed = compose(dir.path(), "cleaned.md", &cleaned);
            if label == "defaults" {
                assert_eq!(composed, expected, "{gate}={value}; cleaned:\n{cleaned}");
            }
            assert_eq!(
                html(&composed),
                html(&expected),
                "{gate}={value} under {label}; cleaned:\n{cleaned}"
            );
        }
    }
    assert!(!changed_by.is_empty(), "no profile edits {body:?}, so the comparison would be vacuous");
}

#[test]
fn a1_composes_the_same_after_cleanup() {
    assert_composes_the_same(A1, "a");
}

#[test]
fn b1_composes_the_same_after_cleanup() {
    assert_composes_the_same(B1, "m");
}
