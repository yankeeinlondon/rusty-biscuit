//! Markdown cleanup.
//!
//! [`clean_document`] is the raw-source transaction `md clean` and DMLS
//! formatting share: frontmatter repair, one resolution of the
//! `style.markdown` rules ([`rules`]), guarded body formatting, and
//! reassembly from the original frontmatter bytes. [`clean_body`] is the
//! guarded body engine underneath it: the rules run as bounded edits of the
//! authored bytes, directive lines and expressions are fixed points, and one
//! final comparison returns the input unchanged (with a
//! `clean.fidelity-fallback` finding) whenever the result would differ in
//! meaning.
//!
//! [`cleanup_content`] and [`cleanup_content_with`] (and the
//! `Markdown::cleanup*` methods built on them) are string-returning wrappers
//! over [`clean_body`]; they return the input on refusal. Incidental-newline
//! stripping and fixed-width reflow remain public as standalone, unguarded
//! helpers ([`strip_incidental_newlines`], [`reflow_to_width`]).

mod body;
mod compare;
pub mod diagnostic;
mod document;
mod edits;
pub mod frontmatter;
mod markers;
mod opaque;
mod protect;
mod reflow;
pub mod rules;
mod structure;
mod transform;
mod wrap;

#[cfg(test)]
mod tests;

use pulldown_cmark::{Options, Parser};

#[cfg(any(test, feature = "cleanup-test-hooks"))]
pub use body::test_hooks;
pub use body::{BodyCleanup, CleanOutcome, Fallback, clean_body};
pub use document::{CleanRequest, CleanResult, clean_document};
pub use reflow::{reflow_to_width, strip_incidental_newlines};

use rules::{MarkdownRuleOverrides, ResolvedMarkdownRules, resolve};

/// Returns parser options suitable for cleanup operations.
fn cleanup_parser_options() -> Options {
    Options::all() - Options::ENABLE_SMART_PUNCTUATION - Options::ENABLE_DEFINITION_LIST
}

/// Constructs the Markdown parser for a cleanup-path pass.
///
/// Every cleanup-path parse is funneled through this one constructor so the
/// parse count per public entry point is a measurable property rather than an
/// assumption; `tests::parse_count` asserts it. Adding a `Parser::new_ext`
/// call to the cleanup path without routing it here would make that assertion
/// silently under-count. Directive and expression scanners parse separately,
/// through `compose::parse_utils::scan_code_regions`, and are counted there.
fn cleanup_parser(content: &str) -> Parser<'_> {
    #[cfg(test)]
    parse_count::record();
    Parser::new_ext(content, cleanup_parser_options())
}

/// Thread-local tally of cleanup-path Markdown parses.
///
/// Thread-local rather than global so the count is unaffected by tests running
/// concurrently in one process.
#[cfg(test)]
pub(super) mod parse_count {
    use std::cell::Cell;

    thread_local! {
        static COUNT: Cell<usize> = const { Cell::new(0) };
    }

    /// Records one cleanup-path parse.
    pub(crate) fn record() {
        COUNT.with(|count| count.set(count.get() + 1));
    }

    /// Runs `operation` and returns its value alongside the number of
    /// cleanup-path Markdown parses it performed.
    pub(crate) fn measure<T>(operation: impl FnOnce() -> T) -> (T, usize) {
        let before = COUNT.with(Cell::get);
        let value = operation();
        (value, COUNT.with(Cell::get) - before)
    }
}

/// Controls whether compose's cleanup step joins incidental single newlines
/// inside prose (the `unwrap` rule).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncidentalNewlineMode {
    /// Join incidental single newlines inside prose while preserving block structure.
    Strip,
    /// Leave single newlines untouched.
    Preserve,
}

/// Formats a Markdown body under the runtime default rules.
///
/// A string wrapper over [`clean_body`]: on refusal it returns `content`
/// unchanged. Use [`clean_body`] when the outcome or diagnostics matter.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::cleanup::cleanup_content;
///
/// assert_eq!(cleanup_content("```\ncode\n```\n"), "```text\ncode\n```\n");
/// // Authored line breaks and numbering are kept.
/// assert_eq!(cleanup_content("1. a\n1. b\n"), "1. a\n1. b\n");
/// ```
#[must_use]
pub fn cleanup_content(content: &str) -> String {
    clean_body(content, &ResolvedMarkdownRules::defaults()).text
}

/// Formats a Markdown body with `overrides` taking precedence over the
/// runtime defaults, as explicit (command-line level) values.
///
/// A string wrapper over [`clean_body`]: on refusal it returns `content`
/// unchanged.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::cleanup::cleanup_content_with;
/// use darkmatter::markdown::cleanup::rules::{Leaf, ListItemPadding, MarkdownRuleOverrides};
///
/// let compact = MarkdownRuleOverrides {
///     list_item_padding: Leaf::Value(ListItemPadding::None),
///     ..MarkdownRuleOverrides::default()
/// };
/// assert_eq!(cleanup_content_with("- a\n\n- b\n", &compact), "- a\n- b\n");
/// ```
#[must_use]
pub fn cleanup_content_with(content: &str, overrides: &MarkdownRuleOverrides) -> String {
    clean_body(content, &resolve(overrides, None, None)).text
}
