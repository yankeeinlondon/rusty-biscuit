//! Host-declared globals in main composition
//! (`ComposeOptions::with_host_globals`).
//!
//! A root is a reserved namespace, then a declared global, then a document
//! property. Without declared globals every non-reserved root is a document
//! property, so a `loop:` frontmatter key is ordinary data; a host that
//! declares `loop` makes bare, dotted, and bracket reads of it read the
//! global in the body and in compose-time frontmatter values, while `fm.loop`
//! still reads the key.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use darkmatter::markdown::compose::expression::{
    Availability, BindingView, HostGlobals, RuntimeBinding, ScopeId, UnavailabilityReason,
};
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions};
use darkmatter::markdown::{Markdown, MarkdownError};
use serde_json::{Value, json};
use tempfile::TempDir;

const REMOVED: &str = "host.removed-name";

fn write(dir: &Path, name: &str, content: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, content).unwrap();
    path
}

/// Composes `content` as `doc.md`, with `child.md` beside it when given.
fn compose_with(
    content: &str,
    child: Option<&str>,
    configure: impl FnOnce(ComposeOptions) -> ComposeOptions,
) -> Result<Markdown, MarkdownError> {
    let dir = TempDir::new().unwrap();
    let path = write(dir.path(), "doc.md", content);
    if let Some(child) = child {
        write(dir.path(), "child.md", child);
    }
    let options = configure(
        ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir.path(), ""))
            .with_source_file(path.clone()),
    );
    Markdown::try_from(path.as_path())
        .unwrap()
        .compose_with(&crate::request_support::request(options))
        .map(|(composed, _)| composed)
}

/// `loop` available with `value`, and `retired` declared unavailable.
fn loop_globals(value: Value) -> HostGlobals {
    let view = BindingView::builder(ScopeId::new("host.iteration"))
        .declare("loop", "The iteration's values.", Availability::Available)
        .declare(
            "retired",
            "A removed name.",
            Availability::Unavailable(UnavailabilityReason::new(REMOVED)),
        )
        .build()
        .unwrap();
    HostGlobals::associate(
        Arc::new(view),
        [
            ("loop".to_string(), RuntimeBinding::eager(value)),
            (
                "retired".to_string(),
                RuntimeBinding::unavailable(UnavailabilityReason::new(REMOVED)),
            ),
        ],
    )
    .unwrap()
}

fn iteration() -> Value {
    json!({"count": 2, "is_last": true, "last_output": "prior answer"})
}

const LOOP_DOCUMENT: &str = "---\nloop:\n  max: 3\n---\n";

fn unavailable_reason(error: &MarkdownError) -> Option<String> {
    error
        .unavailable_binding()
        .map(|read| format!("{}:{}", read.root, read.reason.code()))
}

#[test]
fn without_declared_globals_loop_is_an_ordinary_document_property() {
    let composed = compose_with(
        &format!("{LOOP_DOCUMENT}[{{{{ loop.max }}}}] [{{{{ loop['max'] }}}}] [{{{{ fm.loop.max }}}}]\n"),
        None,
        |options| options,
    )
    .unwrap();

    assert_eq!(composed.content().trim_end(), "[3] [3] [3]");
}

#[test]
fn a_declared_global_wins_over_the_same_named_key_in_the_body() {
    let composed = compose_with(
        &format!(
            "{LOOP_DOCUMENT}[{{{{ loop.count }}}}] [{{{{ loop['last_output'] }}}}] \
             [{{{{ loop.is_last ? 'last' : 'more' }}}}] [{{{{ loop.max }}}}] [{{{{ fm.loop.max }}}}]\n"
        ),
        None,
        |options| options.with_host_globals(loop_globals(iteration())),
    )
    .unwrap();

    // `loop.max` is a missing member of the global: `null`, never the key.
    assert_eq!(composed.content().trim_end(), "[2] [prior answer] [last] [] [3]");
}

#[test]
fn a_compose_time_value_reads_a_declared_global_named_like_a_deferred_key() {
    let composed = compose_with(
        "---\nloop:\n  max: 3\nicon: \"{{ loop.is_last ? 'done' : 'next' }}\"\n\
         pass: \"{{ loop['count'] }}\"\nwhole: \"{{ loop }}\"\n---\n[{{ icon }}] [{{ pass }}]\n",
        None,
        |options| {
            options
                .with_exclude_keys(["loop"])
                .with_host_globals(loop_globals(iteration()))
        },
    )
    .unwrap();

    assert_eq!(composed.content().trim_end(), "[done] [2]");
    let frontmatter = composed.frontmatter().as_map();
    assert_eq!(frontmatter.get("icon"), Some(&json!("done")));
    assert_eq!(frontmatter.get("pass"), Some(&json!(2)));
    assert_eq!(frontmatter.get("whole"), Some(&iteration()));
    // The deferred key itself survives raw as configuration.
    assert_eq!(frontmatter.get("loop"), Some(&json!({"max": 3})));
}

#[test]
fn an_explicit_fm_read_of_a_deferred_key_is_still_rejected() {
    let error = compose_with(
        "---\nloop:\n  max: 3\nlimit: \"{{ fm.loop.max }}\"\n---\nbody\n",
        None,
        |options| {
            options
                .with_exclude_keys(["loop"])
                .with_host_globals(loop_globals(iteration()))
        },
    )
    .unwrap_err();

    assert!(
        error.to_string().contains("references deferred key 'loop'"),
        "{error}"
    );
}

#[test]
fn without_declared_globals_a_compose_time_read_of_a_deferred_key_is_rejected() {
    let error = compose_with(
        "---\nloop:\n  max: 3\nicon: \"{{ loop.is_last }}\"\n---\nbody\n",
        None,
        |options| options.with_exclude_keys(["loop"]),
    )
    .unwrap_err();

    assert!(
        error.to_string().contains("references deferred key 'loop'"),
        "{error}"
    );
}

#[test]
fn reading_an_unavailable_global_is_a_typed_binding_error_in_body_and_frontmatter() {
    let body = compose_with("---\ntitle: X\n---\n{{ retired }}\n", None, |options| {
        options.with_host_globals(loop_globals(iteration()))
    })
    .unwrap_err();
    let frontmatter = compose_with("---\nlabel: \"{{ retired.x }}\"\n---\nbody\n", None, |options| {
        options.with_host_globals(loop_globals(iteration()))
    })
    .unwrap_err();

    let expected = Some(format!("retired:{REMOVED}"));
    assert_eq!(unavailable_reason(&body), expected, "{body:?}");
    assert_eq!(unavailable_reason(&frontmatter), expected, "{frontmatter:?}");
}

#[test]
fn a_transcluded_child_reads_the_parent_compositions_globals() {
    let composed = compose_with(
        "---\ntitle: X\n---\n::file ./child.md\n",
        Some("---\ntitle: child\n---\nchild pass {{ loop.count }}\n"),
        |options| options.with_host_globals(loop_globals(iteration())),
    )
    .unwrap();
    assert!(composed.content().contains("child pass 2"), "{}", composed.content());

    let error = compose_with(
        "---\ntitle: X\n---\n::file ./child.md\n",
        Some("---\ntitle: child\n---\n{{ retired }}\n"),
        |options| options.with_host_globals(loop_globals(iteration())),
    )
    .unwrap_err();
    assert_eq!(unavailable_reason(&error), Some(format!("retired:{REMOVED}")), "{error:?}");
}
