//! Work-count invariants for the `md clean` frontmatter pipeline.
//!
//! The spec makes performance a correctness property rather than a timing
//! one, so these are counter assertions, not benchmarks — they hold on a
//! loaded CI box exactly as they do on an idle laptop.
//!
//! Three invariants, from the spec's Performance section:
//!
//! - **No frontmatter → zero cost.** Schema resolution and trigger discovery
//!   run only behind a non-empty frontmatter block.
//! - **Per-run caching.** Trigger discovery and the built validator are
//!   cached per `clean` invocation, not rebuilt per document or per key.
//! - **Reparse only candidates.** The safety-gate double-parse applies only
//!   to candidate edits; an already-clean document parses exactly once.
//!
//! ## Scope
//!
//! The CLI's own short-circuit — the early return that keeps the
//! trigger-schema git-root walk off the no-frontmatter path — is proven at
//! Level 1 in `darkmatter/cli/tests/clean_schema.rs`, which points
//! `--baseline-schema` at a missing file and shows the command still
//! succeeds. That is a stronger proof than a counter (the walk cannot have
//! run, or resolution would have failed) and it is not duplicated here.
//! This suite covers the library-observable half: what
//! `CleanSchemaConfig` / `CleanSchemaContext` cost once they are reached.

use biscuit_file::{analyze_parse_count, analyze_yaml, reset_analyze_parse_count};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::schemas::{CleanSchemaConfig, CleanSchemaContext};
use serial_test::serial;

/// The parse counter is process-global, so every test that reads it must be
/// serialized against every other test that can trigger a YAML analysis.
fn markdown_with(yaml_body: &str) -> Markdown {
    let content = format!("---\n{yaml_body}---\nbody\n");
    content.as_str().into()
}

fn context() -> CleanSchemaContext {
    CleanSchemaConfig::new()
        .with_trigger_schemas(false)
        .resolve(None)
        .expect("baseline-only resolution must succeed")
}

// --- Reparse only candidates ---------------------------------------------

/// An already-clean block is parsed once. The safety-gate reparse is what
/// makes the second parse, and with no candidate edit there is nothing to
/// prove — so it must not happen.
#[test]
#[serial]
fn clean_frontmatter_parses_exactly_once() {
    reset_analyze_parse_count();
    let analysis = analyze_yaml("title: example\n");
    assert!(analysis.is_clean());
    assert_eq!(
        analyze_parse_count(),
        1,
        "an already-clean block must not pay for the safety-gate reparse"
    );
}

/// A block carrying a candidate edit pays exactly one reparse — the combined
/// proof — regardless of how many candidates it carries. A per-candidate
/// reparse would make cost scale with damage.
#[test]
#[serial]
fn candidate_bearing_frontmatter_reparses_once_regardless_of_candidate_count() {
    reset_analyze_parse_count();
    let _ = analyze_yaml("title: example  \n");
    let single = analyze_parse_count();

    reset_analyze_parse_count();
    let _ = analyze_yaml("\u{FEFF}title :  [ 1,2 ]  \r\nother:  [ 3,4 ]  \r\n");
    let many = analyze_parse_count();

    assert_eq!(single, 2, "one initial parse plus one combined proof");
    assert_eq!(
        many, single,
        "a multi-candidate block must not cost more parses than a single-candidate one"
    );
}

// --- No frontmatter, no schema work --------------------------------------

/// An empty frontmatter block produces no findings, whatever the schema
/// state. The baseline schema still *resolves* — it is a property of the
/// invocation, not of the document — but there is no instance to validate,
/// so the tier has nothing to do.
#[test]
#[serial]
fn empty_frontmatter_produces_no_findings() {
    let context = context();
    for empty in ["", "   ", "\n", "  \n  \n"] {
        let markdown = markdown_with(empty);
        let analysis = context
            .analyze(&markdown, empty)
            .expect("empty frontmatter must not error");
        assert!(
            analysis.is_clean(),
            "empty frontmatter {empty:?} must produce no findings, got {:?}",
            analysis.diagnostics()
        );
    }
}

/// With the baseline disabled and triggers off there is no schema at all, so
/// the tier is fully inert — the configuration `--no-baseline-schema
/// --no-trigger-schemas` selects.
#[test]
#[serial]
fn schema_free_configuration_resolves_no_effective_schema() {
    let context = CleanSchemaConfig::new()
        .without_baseline_schema()
        .with_trigger_schemas(false)
        .resolve(None)
        .expect("schema-free resolution must succeed");

    let markdown = markdown_with("release: 1.20\n");
    let analysis = context
        .analyze(&markdown, "release: 1.20\n")
        .expect("analysis must not error");

    assert!(analysis.is_clean(), "{:?}", analysis.diagnostics());
    assert!(
        analysis.effective().is_none(),
        "no baseline and no triggers must resolve no effective schema"
    );
}

/// A document with no frontmatter at all costs zero YAML parses. This is the
/// library-side companion to the CLI's early return.
#[test]
#[serial]
fn no_frontmatter_costs_zero_parses() {
    // Resolution happens before the counter is zeroed so this measures the
    // per-document cost, not the one-time setup cost.
    let context = context();
    let markdown: Markdown = "# Heading\n\nBody text with no frontmatter.\n".into();

    reset_analyze_parse_count();
    let _ = context.effective_schema(&markdown);
    assert_eq!(
        analyze_parse_count(),
        0,
        "a document with no frontmatter must not invoke YAML analysis at all"
    );
}

// --- Per-run caching ------------------------------------------------------

/// One resolved context serves many documents. `CleanSchemaContext` *is* the
/// per-invocation cache the performance contract names, so resolving once and
/// analyzing repeatedly must not rebuild the validator each time.
#[test]
#[serial]
fn one_resolved_context_serves_repeated_analyses() {
    let context = context();
    let markdown = markdown_with("$schema:\n  release: string\nrelease: 1.20\n");
    let yaml = "$schema:\n  release: string\nrelease: 1.20\n";

    let first = context.analyze(&markdown, yaml).expect("analysis");
    let cache_after_first = context.schemas().cache().len();

    for _ in 0..8 {
        let repeat = context.analyze(&markdown, yaml).expect("analysis");
        assert_eq!(
            repeat.diagnostics().len(),
            first.diagnostics().len(),
            "repeated analysis through one context must be stable"
        );
    }

    assert_eq!(
        context.schemas().cache().len(),
        cache_after_first,
        "repeated analyses must reuse the built validator, not add cache entries"
    );
}

/// Resolution is what performs trigger discovery, so it must be possible to
/// do it once and reuse the result — the context is `Clone` and holds no
/// per-document state that would make reuse incorrect.
#[test]
#[serial]
fn resolved_context_is_reusable_across_documents() {
    let context = context();

    let first = markdown_with("$schema:\n  release: string\nrelease: \"1.0\"\n");
    let second = markdown_with("$schema:\n  title: string\ntitle: hello\n");

    let a = context
        .analyze(&first, "$schema:\n  release: string\nrelease: \"1.0\"\n")
        .expect("analysis");
    let b = context
        .analyze(&second, "$schema:\n  title: string\ntitle: hello\n")
        .expect("analysis");

    assert!(a.is_clean(), "{:?}", a.diagnostics());
    assert!(b.is_clean(), "{:?}", b.diagnostics());
}

/// Disabling trigger discovery must be observable as *less* work, not just a
/// different result: resolution with triggers off never consults the
/// filesystem, so it succeeds with no path context at all.
#[test]
#[serial]
fn trigger_discovery_is_skippable() {
    let disabled = CleanSchemaConfig::new()
        .with_trigger_schemas(false)
        .resolve(None);
    assert!(
        disabled.is_ok(),
        "trigger-free resolution must not require path context"
    );
}
