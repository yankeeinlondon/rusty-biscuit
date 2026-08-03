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
fn ambient_options_resolve_discovery_backed_ctx_groups() {
    let content = "repo_root={{ ctx.repo_root }}|os={{ ctx.os }}\n";

    let ambient = compose_ambient(content);

    assert!(
        !ambient.contains("repo_root=|"),
        "`ctx.repo_root` blanked under ambient options: {ambient}"
    );
    assert!(
        !ambient.ends_with("os="),
        "`ctx.os` blanked under ambient options: {ambient}"
    );
    assert_eq!(
        ambient,
        compose_full_capture(content),
        "ambient options must render what a full capture renders"
    );
}

#[test]
fn ambient_options_resolve_date_time_without_discovery() {
    let content = "today={{ ctx.today }}\n";

    assert_eq!(compose_ambient(content), compose_full_capture(content));
}

// The other half of the rule — that a document naming no `ctx.*` group still
// captures nothing — is asserted at construction by
// `compose::context::options::tests::new_captures_no_discovery_derived_group`,
// which reads the crate-private context directly instead of inferring the
// captured set from rendered output.
