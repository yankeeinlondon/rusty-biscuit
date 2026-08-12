//! `ComposeOptions::new()` must still resolve every `ctx.*` group a document
//! names, despite capturing none of them at construction time.
//!
//! `new()` deliberately performs no host or repository discovery: a constructor
//! has no document, so probing Git, repository topology, working-tree changes,
//! languages, documents, OS, hardware, and GPU is speculative and walks the
//! whole working tree. The compose pipeline restores what the document actually
//! asks for, because it is the first place that sees both.
//!
//! This is a rendered-output test on purpose. The failure it guards against is
//! silent: `ctx.repo_root` interpolates to an empty string rather than raising,
//! so a document keeps composing and simply loses its values. Compose reads
//! `ctx` from the snapshot `EffectiveState` materializes, and no later stage
//! re-reads it, so nothing downstream can recover a group missing at that point.

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions};
use darkmatter::markdown::compose::context::catalog::context_variable_descriptors;

// This file compares *presence*, never text.
//
// The two renderings come from two captures taken moments apart, so any
// clock- or memory-derived value may legitimately differ between them —
// `time_utc` and `time_military_utc` change on a minute boundary, the memory
// figures on any allocation. Asserting equality means maintaining a list of
// which keys are volatile, and a stale list produces a flake rather than a
// finding: an early draft of this test omitted the two `*_utc` keys and flaked
// on the first full-suite run that straddled a minute.
//
// Presence is also the exact signature of the failure being guarded against.
// The 2026-08-02 regression did not corrupt values, it erased them: 36
// variables that rendered a value under a full capture rendered an empty string
// under ambient options. Whether a captured value is *correct* is the job of
// the per-group capture tests, which pin a fixed context and can assert text
// without racing a clock.

/// Composes `content` with bare `ComposeOptions::new()`.
fn compose_ambient(content: &str) -> String {
    let md: Markdown = content.into();
    let (composed, _report) = md
        .compose_with(ComposeOptions::new())
        .expect("compose must succeed");
    composed.content().trim().to_string()
}

/// The same content under a deliberate full capture, as the reference.
fn compose_full_capture(content: &str) -> String {
    let md: Markdown = content.into();
    let options = ComposeOptions::new_with_context(ComposeContext::capture());
    let (composed, _report) = md
        .compose_with(options)
        .expect("compose must succeed");
    composed.content().trim().to_string()
}

#[test]
fn ambient_options_resolve_date_time_without_discovery() {
    let content = "today={{ ctx.today }}\n";

    assert_eq!(compose_ambient(content), compose_full_capture(content));
}

/// Renders one `name=value` line per catalog variable into a lookup map.
fn render_every_variable(options: ComposeOptions) -> std::collections::HashMap<String, String> {
    // One paragraph, not one per variable: ~50 separate blocks cost more to
    // compose than the captures this test is actually measuring.
    let document: String = context_variable_descriptors()
        .iter()
        .map(|descriptor| format!("<{0}={{{{ ctx.{0} }}}}>", descriptor.name))
        .collect::<Vec<_>>()
        .join(" ");

    let md: Markdown = document.as_str().into();
    let (composed, _report) = md.compose_with(options).expect("compose must succeed");
    let content = composed.content().to_string();

    content
        .split('<')
        .filter_map(|segment| segment.split_once('>'))
        .filter_map(|(pair, _)| pair.split_once('='))
        .map(|(key, value)| (key.trim().to_string(), value.trim().to_string()))
        .collect()
}

/// Every `ctx.*` the catalog declares must survive ambient options.
///
/// The 2026-08-02 regression blanked `ctx.repo_root` and `ctx.os` while the
/// whole suite passed, because the tests that interpolate `ctx.*` overwhelmingly
/// pin the context with `ComposeContext::fixed_for_testing()`. A pinned context
/// already holds every value, so those tests verify that interpolation reads a
/// map — never that capture put anything in it. Only a real capture can fail
/// that way, and only two integration tests used one.
///
/// Driving this from `context_variable_descriptors()` rather than a hand-picked
/// list is the point: the regression touched two keys, and a list written by the
/// person who just fixed those two would have covered exactly those two. A new
/// `ctx.*` variable joins this test by existing.
#[test]
fn every_catalog_variable_survives_ambient_options() {
    let expected = render_every_variable(ComposeOptions::new_with_context(
        ComposeContext::capture(),
    ));
    let ambient = render_every_variable(ComposeOptions::new());

    let blanked: Vec<&str> = context_variable_descriptors()
        .iter()
        .map(|descriptor| descriptor.name)
        .filter(|name| match (expected.get(*name), ambient.get(*name)) {
            (Some(want), Some(got)) => !want.is_empty() && got.is_empty(),
            _ => false,
        })
        .collect();

    assert!(
        blanked.is_empty(),
        "ambient options rendered these `ctx.*` variables empty where a full \
         capture rendered a value: {blanked:?}"
    );
}

// The other half of the rule — that a document naming no `ctx.*` group still
// captures nothing — is asserted at construction by
// `compose::context::options::tests::new_captures_no_discovery_derived_group`,
// which reads the crate-private context directly instead of inferring the
// captured set from rendered output.
