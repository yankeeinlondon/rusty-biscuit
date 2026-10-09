//! Pre-flight discovery walks a transcluded child with the state composition
//! gives it, so the approval set holds the bytes each child command executes.
//!
//! Every row composes a root document that transcludes a partial whose shell
//! command interpolates a value, and changes only where that value comes
//! from. Each row asserts the command is in the approval set with its
//! executed bytes, and that composing against exactly that set runs it.

use std::collections::HashSet;
use std::path::Path;

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{ComposeOptions, ShellExpansionError};
use serde_json::json;

struct Row {
    name: &'static str,
    /// `(file name, content)`; the first file is the root.
    files: &'static [(&'static str, &'static str)],
    options: fn(ComposeOptions) -> ComposeOptions,
    /// The normalized command the partial runs.
    command: &'static str,
    /// Whether the composed output contains that command's output.
    executes: bool,
}

fn unchanged(options: ComposeOptions) -> ComposeOptions {
    options
}

const PART: (&str, &str) = ("part.md", "::shell echo ran-{{ base }}\n");

const ROWS: &[Row] = &[
    Row {
        name: "parent default",
        files: &[("target.md", "---\nbase: main\n---\n::file ./part.md\n"), PART],
        options: unchanged,
        command: "echo ran-main",
        executes: true,
    },
    Row {
        name: "value derived in the parent",
        files: &[
            ("target.md", "---\nb: main\nbase: \"{{ b }}-x\"\n---\n::file ./part.md\n"),
            PART,
        ],
        options: unchanged,
        command: "echo ran-main-x",
        executes: true,
    },
    Row {
        name: "data override layer (a proxy.with: overlay)",
        files: &[("target.md", "---\nbase: main\n---\n::file ./part.md\n"), PART],
        options: |options| options.with_data_overrides(json!({ "base": "overlay" })),
        command: "echo ran-overlay",
        executes: true,
    },
    Row {
        name: "caller override",
        files: &[("target.md", "::file ./part.md\n"), PART],
        options: |options| options.with_set_overrides(json!({ "base": "caller" })),
        command: "echo ran-caller",
        executes: true,
    },
    Row {
        name: "partial without interpolation",
        files: &[
            ("target.md", "---\nbase: main\n---\n::file ./part.md\n"),
            ("part.md", "::shell echo ran-plain\n"),
        ],
        options: unchanged,
        command: "echo ran-plain",
        executes: true,
    },
    Row {
        name: "nested partial",
        files: &[
            ("target.md", "---\nbase: main\n---\n::file ./mid.md\n"),
            ("mid.md", "::file ./part.md\n"),
            PART,
        ],
        options: unchanged,
        command: "echo ran-main",
        executes: true,
    },
    Row {
        name: "transclusion-local set over the parent default",
        files: &[
            ("target.md", "---\nbase: main\n---\n::file ./part.md set.base=\"local\"\n"),
            PART,
        ],
        options: unchanged,
        command: "echo ran-local",
        executes: true,
    },
    Row {
        name: "shell block in a conditional partial",
        files: &[
            (
                "target.md",
                "---\nbase: main\nwanted: true\n---\n::file ./part.md when=\"wanted\"\n",
            ),
            (
                "part.md",
                "::shell-block when_error=\"(unavailable)\"\necho ran-{{ base }}\n::end-block\n",
            ),
        ],
        options: unchanged,
        command: "echo ran-main",
        executes: true,
    },
    Row {
        name: "untaken branch",
        files: &[
            (
                "target.md",
                "---\nbase: main\nwanted: false\n---\n::file ./part.md when=\"wanted\"\n",
            ),
            PART,
        ],
        options: unchanged,
        command: "echo ran-main",
        executes: false,
    },
    Row {
        name: "frontmatter command in the partial",
        files: &[
            ("target.md", "---\nbase: main\n---\n::file ./part.md\n"),
            ("part.md", "---\nsha: \"$(echo ran-{{ base }})\"\n---\n{{ sha }}\n"),
        ],
        options: unchanged,
        command: "echo ran-main",
        executes: true,
    },
];

fn write_files(dir: &Path, files: &[(&str, &str)]) -> std::path::PathBuf {
    for (name, content) in files {
        std::fs::write(dir.join(name), content).expect("write fixture file");
    }
    dir.join(files[0].0)
}

fn base_options(root: &Path, dir: &Path) -> ComposeOptions {
    ComposeOptions::new()
        .with_source_file(root)
        .with_shell_policy_root(dir)
        // Real `echo` subprocesses can be starved past the 10 s default under
        // full-suite load.
        .with_shell_timeout(std::time::Duration::from_secs(60))
}

fn assert_row_approved_with_executed_bytes(row: &Row) {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = write_files(dir.path(), row.files);
    let document = Markdown::try_from(root.as_path()).expect("load root");
    let options = (row.options)(base_options(&root, dir.path()));

    let approved: HashSet<String> = document
        .compose_preflight(&crate::request_support::request(options.clone()))
        .unwrap_or_else(|error| panic!("{}: preflight failed: {error}", row.name))
        .approval_set()
        .into_iter()
        .collect();
    assert!(
        approved.contains(row.command),
        "{}: approval set {approved:?} lacks {:?}",
        row.name,
        row.command
    );

    let (composed, report) = document
        .compose_with(&crate::request_support::request(options.with_pre_approved_commands(approved)))
        .unwrap_or_else(|error| panic!("{}: compose failed: {error}", row.name));
    let output = row.command.trim_start_matches("echo ");
    assert_eq!(
        composed.content().contains(output),
        row.executes,
        "{}: composed output: {}",
        row.name,
        composed.content()
    );
    assert!(
        report.warnings.iter().all(|w| !w.message.contains("pre-approved")),
        "{}: {:?}",
        row.name,
        report.warnings
    );
}

/// One test per `ROWS` entry, so nextest runs the rows in parallel.
macro_rules! row_tests {
    ($($name:ident: $index:literal;)*) => {
        $(
            #[test]
            fn $name() {
                assert_row_approved_with_executed_bytes(&ROWS[$index]);
            }
        )*

        #[test]
        fn every_row_has_a_test() {
            assert_eq!(ROWS.len(), [$($index),*].len());
        }
    };
}

row_tests! {
    parent_default_is_approved_as_executed: 0;
    value_derived_in_the_parent_is_approved_as_executed: 1;
    data_override_layer_is_approved_as_executed: 2;
    caller_override_is_approved_as_executed: 3;
    partial_without_interpolation_is_approved_as_executed: 4;
    nested_partial_is_approved_as_executed: 5;
    transclusion_local_set_is_approved_as_executed: 6;
    shell_block_in_a_conditional_partial_is_approved_as_executed: 7;
    untaken_branch_is_approved_as_executed: 8;
    frontmatter_command_in_the_partial_is_approved_as_executed: 9;
}

/// A parent `$(...)` value is expanded before its child composes, so a child
/// command embedding it has no knowable bytes at discovery: pre-flight names
/// the dependency rather than approving the unexpanded text.
#[test]
fn child_command_embedding_a_parent_shell_value_is_a_dynamic_shape() {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = write_files(
        dir.path(),
        &[
            ("target.md", "---\nbase: \"$(echo main)\"\n---\n::file ./part.md\n"),
            PART,
        ],
    );
    let document = Markdown::try_from(root.as_path()).expect("load root");
    let error = document
        .compose_preflight(&crate::request_support::request(base_options(&root, dir.path())))
        .expect_err("a child command built from a pending parent value");
    assert!(
        matches!(
            error,
            darkmatter::markdown::MarkdownError::ShellExpansion(ref inner)
                if matches!(inner.as_ref(), ShellExpansionError::DynamicCommandShape { .. })
        ),
        "{error:?}"
    );
}

/// A value discovery does not observe as composition will (a shell probe
/// such as `has_alias`, answered `false` without launching the login shell)
/// reaches a child's command through inherited state. Discovery refuses the
/// command, naming the key, instead of approving bytes composition would
/// never run; the child's `when_error` does not apply, as nothing has failed
/// to run.
#[test]
fn a_child_command_built_from_an_unobserved_parent_value_is_refused() {
    for (name, files) in [
        (
            "transcluded partial",
            &[
                ("target.md", "---\nflag: \"{{ has_alias('ll') }}\"\n---\n::file ./part.md\n"),
                (
                    "part.md",
                    "::shell-block when_error=\"(unavailable)\"\necho alias-{{ flag }}\n::end-block\n",
                ),
            ][..],
        ),
        (
            "through a derived key",
            &[
                (
                    "target.md",
                    "---\nflag: \"{{ has_alias('ll') }}\"\nlabel: \"alias-{{ flag }}\"\n---\n::file ./part.md\n",
                ),
                ("part.md", "::shell echo {{ label }}\n"),
            ][..],
        ),
        (
            "nested as_markdown content",
            &[(
                "target.md",
                "---\nflag: \"{{ has_alias('ll') }}\"\n---\n{{ as_markdown('::shell echo nested-' + flag) || 'fallback' }}\n",
            )][..],
        ),
    ] {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = write_files(dir.path(), files);
        let document = Markdown::try_from(root.as_path()).expect("load root");
        let error = document
            .compose_preflight(&crate::request_support::request(base_options(&root, dir.path())))
            .expect_err("the command's executed bytes are not knowable at discovery");
        assert!(
            matches!(
                error,
                darkmatter::markdown::MarkdownError::ShellExpansion(ref inner)
                    if matches!(inner.as_ref(), ShellExpansionError::UnevaluatedDependencyShape { .. })
            ),
            "{name}: {error:?}"
        );
    }
}

/// A caller override replaces the unobserved authored value, so the command
/// has knowable bytes again and is approved as it executes.
#[test]
fn an_overridden_unobserved_value_is_approved_as_executed() {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = write_files(
        dir.path(),
        &[
            ("target.md", "---\nflag: \"{{ has_alias('ll') }}\"\n---\n::file ./part.md\n"),
            ("part.md", "::shell echo alias-{{ flag }}\n"),
        ],
    );
    let document = Markdown::try_from(root.as_path()).expect("load root");
    let options = base_options(&root, dir.path()).with_set_overrides(json!({ "flag": "given" }));
    let approved: HashSet<String> = document
        .compose_preflight(&crate::request_support::request(options.clone()))
        .expect("preflight succeeds")
        .approval_set()
        .into_iter()
        .collect();
    assert!(approved.contains("echo alias-given"), "{approved:?}");
    let (composed, _) = document
        .compose_with(&crate::request_support::request(options.with_pre_approved_commands(approved)))
        .expect("compose succeeds");
    assert!(composed.content().contains("alias-given"), "{}", composed.content());
}

/// A lazy `current.*` read is observed when the command runs and never at
/// discovery, so a command built from one is refused at pre-flight, whether
/// it sits in the root or in a transcluded child, instead of being approved
/// with an empty value.
#[test]
fn a_command_reading_a_lazy_root_is_an_unevaluated_dependency() {
    for (name, files) in [
        ("root", &[("target.md", "::shell echo br-{{ current.branch }}\n")][..]),
        (
            "partial",
            &[
                ("target.md", "::file ./part.md\n"),
                (
                    "part.md",
                    "::shell-block when_error=\"(unavailable)\"\necho br-{{ current.branch }}\n::end-block\n",
                ),
            ][..],
        ),
    ] {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = write_files(dir.path(), files);
        let document = Markdown::try_from(root.as_path()).expect("load root");
        let error = document
            .compose_preflight(&crate::request_support::request(base_options(&root, dir.path())))
            .expect_err("a command reading current.* has no approvable bytes");
        assert!(
            matches!(
                error,
                darkmatter::markdown::MarkdownError::ShellExpansion(ref inner)
                    if matches!(inner.as_ref(), ShellExpansionError::UnevaluatedDependencyShape { .. })
            ),
            "{name}: {error:?}"
        );
    }
}

/// `NotPreApproved` is a broken invariant, not a failed command: a
/// `when_error` fallback covers a command that ran and failed, so it must not
/// absorb a command execution refuses to launch. The block is composed inline
/// and from a transcluded partial; neither yields the fallback text nor a
/// "could not transclude" notice in place of the error.
#[test]
fn a_when_error_block_does_not_absorb_a_pre_approval_violation() {
    const BLOCK: &str = "::shell-block when_error=\"(unavailable)\"\necho ran-{{ base }}\n::end-block\n";
    let inline = format!("---\nbase: main\n---\n{BLOCK}");
    for (name, files) in [
        ("inline", vec![("target.md", inline.as_str())]),
        (
            "transcluded",
            vec![("target.md", "---\nbase: main\n---\n::file ./part.md\n"), ("part.md", BLOCK)],
        ),
    ] {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = write_files(dir.path(), &files);
        let document = Markdown::try_from(root.as_path()).expect("load root");

        // Fixture check: with its bytes approved the block runs.
        let (composed, _) = document
            .compose_with(
                &crate::request_support::request(base_options(&root, dir.path())
                    .with_pre_approved_commands(HashSet::from(["echo ran-main".to_string()]))),
            )
            .unwrap_or_else(|error| panic!("{name}: approved compose failed: {error}"));
        assert!(composed.content().contains("ran-main"), "{name}: {}", composed.content());

        let error = document
            .compose_with(&crate::request_support::request(base_options(&root, dir.path()).with_pre_approved_commands(HashSet::new())))
            .map(|(composed, _)| composed.content().to_string())
            .expect_err("an unapproved command is a hard composition failure");
        assert!(error.pre_approval_violation().is_some(), "{name}: {error:?}");
        let rendered = error.to_string();
        assert!(
            !rendered.contains("(unavailable)") && !rendered.contains("Could not transclude"),
            "{name}: {rendered}"
        );
    }
}
