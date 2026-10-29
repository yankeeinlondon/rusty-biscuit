//! A runtime warning a [`BlockingPolicy`] blocks stops the document where it
//! becomes known, before the next effect: a later `::shell`, a transcluded
//! child's composition, or the child's own shell. The effect each fixture
//! would perform is a `touch` of `marker` in the request directory, so the
//! marker's presence counts the effects that ran.
//!
//! Every blocked row has a control composed without the policy that creates
//! the marker, proving the fixture reaches its effect. The clean controls
//! show what the policy must not stop (a handled or unexecuted read) and the
//! stops that already preceded composition (a passive call warning, a denied
//! recovery).

use std::path::Path;

use darkmatter::markdown::compose::policy::BlockingPolicy;
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions, ComposeReport};
use darkmatter::markdown::{Markdown, MarkdownError};
use tempfile::TempDir;

const UNDECLARED: &str = "dm.expression.undeclared_property";

/// Writes `files` (relative path, content) into a fresh request directory
/// whose shell whitelist admits `touch`, `echo`, and `date`.
fn request_dir(files: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join(".darkmatter-shell-whitelist"), "prefix touch\nprefix echo\nprefix date\n").unwrap();
    for (name, content) in files {
        std::fs::write(dir.path().join(name), content).unwrap();
    }
    dir
}

fn compose(dir: &Path, policy: BlockingPolicy) -> Result<(Markdown, ComposeReport), MarkdownError> {
    let path = dir.join("doc.md");
    let options = ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir, ""))
        .with_source_file(path.clone())
        .with_shell_policy_root(dir)
        .with_shell_working_directory(dir)
        .with_blocking_policy(policy);
    Markdown::try_from(path.as_path()).unwrap().compose_with(&crate::request_support::request_at(dir, options))
}

fn marker_exists(dir: &Path) -> bool {
    dir.join("marker").exists()
}

/// The blocked rows: under `--deny all` the document fails with the
/// undeclared-property warning blocking and no marker; without a policy the
/// same document composes and creates the marker.
macro_rules! blocks_before_the_effect {
    ($($name:ident: $files:expr;)+) => {
        $(
            #[test]
            fn $name() {
                let blocked = request_dir(&$files);
                let error = compose(blocked.path(), BlockingPolicy::default().deny_all())
                    .expect_err("the undeclared read blocks");
                let blocking = error.blocked_diagnostics().unwrap_or_else(|| panic!("a policy stop: {error}"));
                assert!(
                    blocking.iter().any(|warning| warning.code.as_deref() == Some(UNDECLARED)),
                    "{blocking:#?}"
                );
                assert!(!marker_exists(blocked.path()), "no effect ran after the warning was known");

                let control = request_dir(&$files);
                let (_, report) = compose(control.path(), BlockingPolicy::default())
                    .unwrap_or_else(|error| panic!("composes without a policy: {error}"));
                assert!(report.warnings.iter().any(|warning| warning.code.as_deref() == Some(UNDECLARED)));
                assert!(marker_exists(control.path()), "the control reaches its effect");
            }
        )+
    };
}

blocks_before_the_effect! {
    // R1-F2.1
    body_interpolation_stops_a_later_shell: [("doc.md", "{{ missing }}\n\n::shell touch marker\n")];
    // R1-F2.2
    frontmatter_interpolation_stops_a_body_shell: [("doc.md", "---\ntitle: \"{{ missing }}\"\n---\n::shell touch marker\n")];
    // R1-F2.3
    page_block_condition_stops_a_retained_shell: [(
        "doc.md",
        "::block when=\"is_empty(trim(missing))\"\n::shell touch marker\n::end-block\n",
    )];
    // R1-F2.4
    transclusion_condition_stops_the_child: [
        ("doc.md", "::file ./child.md when=\"is_empty(trim(missing))\"\n"),
        ("child.md", "::shell touch marker\n"),
    ];
    // R1-F2.5
    transclusion_condition_stops_the_next_directive: [
        ("doc.md", "::file ./child.md when=\"missing\"\n\n::file ./child.md\n"),
        ("child.md", "::shell touch marker\n"),
    ];
    // R1-F2.6
    frontmatter_shell_ternary_stops_a_body_shell: [(
        "doc.md",
        "---\ngated: \"$(missing ? echo a : echo b)\"\n---\n::shell touch marker\n",
    )];
    // R1-F2.7
    a_transcluded_child_stops_before_its_own_shell: [
        ("doc.md", "::file ./child.md\n"),
        ("child.md", "{{ missing }}\n\n::shell touch marker\n"),
    ];
    // R1-F2.13: body interpolation evaluates its spans end to start, so the
    // later `{{ missing }}` is known before the earlier nested shell runs.
    body_interpolation_stops_a_later_evaluated_nested_shell: [(
        "doc.md",
        "{{ as_markdown(\"::shell touch marker\") }}\n\n{{ missing }}\n",
    )];
    // R1-F2.13: a nested child's own blocking read is known before the next
    // nested composition runs.
    a_nested_child_warning_stops_the_next_nested_shell: [(
        "doc.md",
        "{{ as_markdown(\"::shell touch marker\") }}\n\n{{ as_markdown(\"{{ missing }}\") }}\n",
    )];
    // R1-F2.13: a nested child's blocking read is known before a later stage.
    a_nested_child_warning_stops_a_later_shell: [(
        "doc.md",
        "{{ as_markdown(\"{{ missing }}\") }}\n\n::shell touch marker\n",
    )];
    // R1-F2.13: an `||` fallback that absorbs the child's stop does not
    // let the document run on.
    an_absorbed_nested_stop_stops_a_later_shell: [(
        "doc.md",
        "{{ as_markdown(\"{{ missing }}\") || \"x\" }}\n\n::shell touch marker\n",
    )];
    // R1-F2.13: page-block conditions run in document order; a read in one
    // stops before the next condition composes anything.
    page_block_condition_stops_a_later_condition: [(
        "doc.md",
        "::block when=\"is_empty(trim(missing))\"\nx\n::end-block\n\n\
         ::block when=\"as_markdown('::shell touch marker') == ''\"\ny\n::end-block\n",
    )];
    page_block_condition_stops_a_later_nested_condition: [(
        "doc.md",
        "::block when=\"true\"\n::block when=\"is_empty(trim(missing))\"\nx\n::end-block\n\
         ::block when=\"as_markdown('::shell touch marker') == ''\"\ny\n::end-block\n::end-block\n",
    )];
    // R1-F2.13: likewise for transclusion `when=` conditions.
    transclusion_condition_stops_a_later_condition: [
        ("doc.md", "::file ./child.md when=\"is_empty(trim(missing))\"\n\n\
                    ::file ./child.md when=\"as_markdown('::shell touch marker') == ''\"\n"),
        ("child.md", "child\n"),
    ];
}

const CONTEXT_TYPO: &str = "dm.expression.unknown_context_variable";

/// R1-F2.13: a stop within a stage keeps the stage's earlier, non-blocking
/// warnings beside the blocking one.
#[test]
fn a_stop_within_a_stage_keeps_its_earlier_warnings() {
    let document = "{{ as_markdown(\"::shell touch marker\") }}\n\n{{ missing }}\n\n{{ ctx.toady }}\n";
    let dir = request_dir(&[("doc.md", document)]);
    let error = compose(dir.path(), BlockingPolicy::default().deny_code(UNDECLARED)).expect_err("the read blocks");
    let MarkdownError::BlockedDiagnostics { blocking, earlier } = &error else {
        panic!("a policy stop: {error}");
    };
    assert_eq!(blocking.len(), 1, "{blocking:#?}");
    assert_eq!(blocking[0].code.as_deref(), Some(UNDECLARED));
    assert!(blocking[0].message.contains(":3"), "the read keeps its line: {}", blocking[0].message);
    assert!(earlier.iter().any(|warning| warning.code.as_deref() == Some(CONTEXT_TYPO)), "{earlier:#?}");
    assert!(!marker_exists(dir.path()));
}

/// R1-F2.13: frontmatter interpolation evaluates keys in document order and a
/// value's spans end to start. A context typo evaluated first stops the
/// nested shell evaluated after it; evaluated after the shell, it still stops
/// the document but the shell already ran. Nothing is reordered.
#[test]
fn frontmatter_context_warning_stops_a_later_nested_shell() {
    let shell = "{{ as_markdown(\\\"::shell touch marker\\\") }}";
    for (document, shell_runs) in [
        (format!("---\na: \"{{{{ ctx.toady }}}}\"\nb: \"{shell}\"\n---\nbody\n"), false),
        (format!("---\nb: \"x {shell} {{{{ ctx.toady }}}}\"\n---\nbody\n"), false),
        (format!("---\nb: \"{shell}\"\na: \"{{{{ ctx.toady }}}}\"\n---\nbody\n"), true),
        (format!("---\nb: \"x {{{{ ctx.toady }}}} {shell}\"\n---\nbody\n"), true),
    ] {
        let dir = request_dir(&[("doc.md", &document)]);
        let error = compose(dir.path(), BlockingPolicy::default().deny_code(CONTEXT_TYPO))
            .expect_err("the context typo blocks");
        let blocking = error.blocked_diagnostics().unwrap_or_else(|| panic!("a policy stop: {error}"));
        assert!(blocking.iter().any(|warning| warning.code.as_deref() == Some(CONTEXT_TYPO)), "{blocking:#?}");
        assert_eq!(marker_exists(dir.path()), shell_runs, "{document}");

        let control = request_dir(&[("doc.md", &document)]);
        compose(control.path(), BlockingPolicy::default()).unwrap_or_else(|error| panic!("composes: {error}"));
        assert!(marker_exists(control.path()), "the control reaches its effect: {document}");
    }
}

/// R1-F2.13 clean control: the nested shell evaluated before the read runs,
/// since nothing blocking is known yet; the read then stops the document.
/// Nothing is reordered to find the warning first.
#[test]
fn a_nested_shell_evaluated_before_the_read_runs() {
    let dir = request_dir(&[("doc.md", "{{ missing }}\n\n{{ as_markdown(\"::shell touch marker\") }}\n")]);
    let error = compose(dir.path(), BlockingPolicy::default().deny_all()).expect_err("the read blocks");
    let blocking = error.blocked_diagnostics().unwrap_or_else(|| panic!("a policy stop: {error}"));
    assert!(blocking.iter().any(|warning| warning.code.as_deref() == Some(UNDECLARED)), "{blocking:#?}");
    assert!(marker_exists(dir.path()), "the earlier-evaluated effect ran");
}

/// R1-F2.13: a coded warning an expression reports as it is evaluated (an
/// unknown context variable) stops the next expression in the same stage,
/// keeping the warnings reported before it.
#[test]
fn a_context_warning_stops_the_next_expression() {
    let document = "{{ as_markdown(\"::shell touch marker\") }}\n\n{{ ctx.toady }}\n";
    let dir = request_dir(&[("doc.md", document)]);
    let error = compose(dir.path(), BlockingPolicy::default().deny_code(CONTEXT_TYPO))
        .expect_err("the context typo blocks");
    let MarkdownError::BlockedDiagnostics { blocking, .. } = &error else {
        panic!("a policy stop: {error}");
    };
    assert!(blocking.iter().any(|warning| warning.message.contains("ctx.toady")), "{blocking:#?}");
    assert!(!marker_exists(dir.path()), "the nested shell did not run");

    let control = request_dir(&[("doc.md", document)]);
    compose(control.path(), BlockingPolicy::default()).unwrap_or_else(|error| panic!("composes: {error}"));
    assert!(marker_exists(control.path()));
}

/// A warning reported before a stop is kept with it, beside the blocking one.
#[test]
fn a_stop_keeps_the_warnings_reported_before_it() {
    let dir = request_dir(&[("doc.md", "{{ missing }}\n\n::shell touch marker\n")]);
    let error = compose(dir.path(), BlockingPolicy::default().deny_code(UNDECLARED)).expect_err("the code is denied");
    let MarkdownError::BlockedDiagnostics { blocking, earlier } = &error else {
        panic!("a policy stop: {error}");
    };
    assert_eq!(blocking.len(), 1, "{blocking:#?}");
    assert!(earlier.iter().all(|warning| warning.code.as_deref() != Some(UNDECLARED)), "{earlier:#?}");
    assert!(!marker_exists(dir.path()));

    // A denial of an unrelated code leaves the read a warning.
    let dir = request_dir(&[("doc.md", "{{ missing }}\n\n::shell touch marker\n")]);
    compose(dir.path(), BlockingPolicy::default().deny_code("dm.shell.timeout_recovered"))
        .unwrap_or_else(|error| panic!("composes: {error}"));
    assert!(marker_exists(dir.path()));
}

/// A warning one `::shell` reports stops the next: a repeated volatile
/// command reuses its first result and says so, an uncoded warning that
/// `--deny all` blocks.
#[test]
fn a_shell_warning_stops_the_next_shell() {
    let document = "::shell date\n\n::shell date\n\n::shell touch marker\n";
    let dir = request_dir(&[("doc.md", document)]);
    let error = compose(dir.path(), BlockingPolicy::default().deny_all()).expect_err("the cache notice blocks");
    let blocking = error.blocked_diagnostics().unwrap_or_else(|| panic!("a policy stop: {error}"));
    assert!(blocking.iter().any(|warning| warning.message.contains("volatile")), "{blocking:#?}");
    assert!(!marker_exists(dir.path()), "the later shell did not run");

    let control = request_dir(&[("doc.md", document)]);
    compose(control.path(), BlockingPolicy::default()).unwrap_or_else(|error| panic!("composes: {error}"));
    assert!(marker_exists(control.path()));
}

/// The clean controls under `--deny all`: (R1-F2.8) a passive category 3
/// call already stops before anything runs; (R1-F2.9) a handled read and
/// (R1-F2.10) a read in a removed block report nothing and the effect runs.
macro_rules! deny_all_control {
    ($($name:ident: $document:expr => $composes:expr;)+) => {
        $(
            #[test]
            fn $name() {
                let dir = request_dir(&[("doc.md", $document)]);
                match (compose(dir.path(), BlockingPolicy::default().deny_all()), $composes) {
                    (Ok((_, report)), true) => {
                        assert!(report.warnings.is_empty(), "{:#?}", report.warnings);
                        assert!(marker_exists(dir.path()), "the effect ran");
                    }
                    (Err(error), false) => {
                        assert!(error.blocked_diagnostics().is_some(), "a policy stop: {error}");
                        assert!(!marker_exists(dir.path()), "no effect ran");
                    }
                    (Ok((_, report)), false) => panic!("expected a stop, composed with {:#?}", report.warnings),
                    (Err(error), true) => panic!("expected success: {error}"),
                }
            }
        )+
    };
}

deny_all_control! {
    passive_call_warning_stops_before_any_shell:
        "---\n$schema:\n  label: string\nlabel: \"7\"\n---\n{{ min(label, 5) }}\n\n::shell touch marker\n" => false;
    handled_absence_does_not_block: "{{ is_null(missing) }}\n\n::shell touch marker\n" => true;
    a_read_in_a_removed_block_does_not_block:
        "::block when=\"false\"\n{{ missing }}\n::end-block\n\n::shell touch marker\n" => true;
}

/// R1-F2.12: a recovery that is not allowed is an error before the later
/// effect, whatever the blocking policy says.
macro_rules! unrecovered_stop {
    ($($name:ident: $files:expr;)+) => {
        $(
            #[test]
            fn $name() {
                let dir = request_dir(&$files);
                compose(dir.path(), BlockingPolicy::default().deny_all()).expect_err("the recovery is not allowed");
                assert!(!marker_exists(dir.path()), "no effect ran");
            }
        )+
    };
}

unrecovered_stop! {
    a_scalar_ctx_stops_before_the_body_shell: [("doc.md", "---\nctx: 5\n---\n::shell touch marker\n")];
    an_invalid_assignment_stops_before_the_child: [
        ("doc.md", "::file ./child.md set=42 set.name=\"Bob\"\n"),
        ("child.md", "::shell touch marker\n"),
    ];
    a_duplicate_assignment_stops_before_the_child: [
        ("doc.md", "::file ./child.md set.name=\"Bob\" set.name=\"Mary\"\n"),
        ("child.md", "::shell touch marker\n"),
    ];
}

