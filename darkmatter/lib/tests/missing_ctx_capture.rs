//! A known `ctx.*` variable whose capture group the request never captured
//! fails composition on every expression surface.
//!
//! Every context here is built from supplied evidence or from requirements that
//! name no discovery-backed group, so no test walks a repository or depends on
//! the host. Spec: `fixes/2026-08-02-silent-empty-ctx-values/spec.md`.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use darkmatter::markdown::compose::expression::ExpressionError;
use darkmatter::markdown::compose::shell_expansion::ShellExpansionOptions;
use darkmatter::markdown::compose::{
    ComposeContext, ComposeOperation, ComposeOptions, ContextCaptureEvidence, ContextGroup,
};
use darkmatter::markdown::{Markdown, MarkdownError, SourceRef};
use tempfile::TempDir;

/// A caller-supplied context holding only the zero-discovery DateTime group.
fn date_time_only(dir: &Path) -> ComposeContext {
    let context = ComposeContext::capture_for_content(dir, "");
    assert!(context.capture_requirements().contains(ContextGroup::DateTime));
    assert!(!context.capture_requirements().contains(ContextGroup::Os));
    assert!(!context.capture_requirements().contains(ContextGroup::Repo));
    context
}

fn write(dir: &Path, name: &str, content: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, content).unwrap();
    path
}

fn compose_file(path: &Path, options: ComposeOptions) -> Result<String, MarkdownError> {
    let md = Markdown::try_from(path).expect("document loads");
    md.compose_with(options.with_source_file(path.to_path_buf()))
        .map(|(composed, _)| composed.content().to_string())
}

fn compose_text(content: &str, options: ComposeOptions) -> Result<String, MarkdownError> {
    let md: Markdown = content.into();
    md.compose_with(options).map(|(composed, _)| composed.content().to_string())
}

/// Asserts `error` is a missing capture of `key` in `group`, reachable through
/// the typed cause chain rather than only through a message.
#[track_caller]
fn assert_not_captured(error: &MarkdownError, key: &str, group: ContextGroup) {
    match error.missing_runtime_context() {
        Some(ExpressionError::ContextNotCaptured { key: found, group: found_group }) => {
            assert_eq!(found, key, "wrong variable in {error:?}");
            assert_eq!(*found_group, group, "wrong group in {error:?}");
        }
        other => panic!("expected ContextNotCaptured for ctx.{key}, got {other:?} from {error:?}"),
    }
    assert!(
        !matches!(error, MarkdownError::Transform(_)),
        "a missing capture must not be a stringly Transform error: {error:?}"
    );
}

// ── Verification #1: typed error naming variable, group, and source ──────────

#[test]
fn body_reference_to_an_uncaptured_group_names_variable_group_and_source() {
    let dir = TempDir::new().unwrap();
    let root = write(dir.path(), "root.md", "intro\n\nrepo_root={{ ctx.repo_root }}\n");

    let error = compose_file(&root, ComposeOptions::new_with_context(date_time_only(dir.path())))
        .expect_err("an uncaptured Repo group must fail composition");

    assert_not_captured(&error, "repo_root", ContextGroup::Repo);
    let message = error.to_string();
    assert!(message.contains("ctx.repo_root") && message.contains("Repo"), "{message}");
    match &error {
        MarkdownError::Interpolation { source, .. } => match source.as_ref() {
            SourceRef::OnDisk(context) => {
                assert!(context.display.ends_with("root.md"), "source: {:?}", context.display)
            }
            other => panic!("expected the on-disk source document, got {other:?}"),
        },
        other => panic!("expected an interpolation error, got {other:?}"),
    }
}

/// The spec's original failing input, verbatim, no longer renders `repo_root=`
/// and `os=` empty; composition fails instead of returning partial output.
#[test]
fn the_regression_input_fails_instead_of_rendering_empty_values() {
    let dir = TempDir::new().unwrap();
    for fail_fast in [false, true] {
        let error = compose_text(
            "repo_root={{ ctx.repo_root }}|os={{ ctx.os }}|today={{ ctx.today }}\n",
            ComposeOptions::new_with_context(date_time_only(dir.path())).with_fail_fast(fail_fast),
        )
        .expect_err("no partially composed document");
        assert!(
            matches!(
                error.missing_runtime_context(),
                Some(ExpressionError::ContextNotCaptured {
                    group: ContextGroup::Repo | ContextGroup::Os,
                    ..
                })
            ),
            "fail_fast={fail_fast}: {error:?}"
        );
    }
}

// ── Verification #2: the same error on every expression surface ─────────────

#[test]
fn frontmatter_interpolation_fails_in_whole_value_and_mixed_text_forms() {
    let dir = TempDir::new().unwrap();
    for (value, fail_fast) in [
        ("\"{{ ctx.os }}\"", false),
        ("\"{{ ctx.os }}\"", true),
        ("\"built on {{ ctx.os }}\"", false),
        ("\"built on {{ ctx.os }}\"", true),
    ] {
        let root = write(dir.path(), "fm.md", &format!("---\nplatform: {value}\n---\nbody\n"));
        let error = compose_file(
            &root,
            ComposeOptions::new_with_context(date_time_only(dir.path())).with_fail_fast(fail_fast),
        )
        .expect_err("frontmatter must not interpolate an uncaptured group");
        assert_not_captured(&error, "os", ContextGroup::Os);
        assert!(
            matches!(&error, MarkdownError::Interpolation { key: Some(key), .. } if key == "platform"),
            "value={value} fail_fast={fail_fast}: {error:?}"
        );
    }
}

/// Pass 2 re-interpolates keys that waited on a frontmatter `$()` value. The
/// command runs through a value branch, so no shell executes.
#[test]
fn frontmatter_interpolation_pass_two_fails() {
    let dir = TempDir::new().unwrap();
    let root = write(
        dir.path(),
        "pass2.md",
        "---\nflag: \"$(true ? 'ready' : echo never)\"\nlabel: \"{{ flag }}-{{ ctx.os }}\"\n---\nbody\n",
    );
    // Pre-approved so the unselected command branch passes the allowlist; the
    // true condition selects the value branch, so nothing executes.
    let options = ComposeOptions::new_with_context(date_time_only(dir.path()))
        .with_shell(ShellExpansionOptions {
            policy_root: Some(dir.path().to_path_buf()),
            ..Default::default()
        })
        .with_pre_approved_commands(HashSet::from(["echo never".to_string()]));
    let error = compose_file(&root, options).expect_err("pass 2 must not interpolate an uncaptured group");
    assert_not_captured(&error, "os", ContextGroup::Os);
    assert!(
        matches!(&error, MarkdownError::Interpolation { key: Some(key), .. } if key == "label"),
        "{error:?}"
    );
}

#[test]
fn page_block_condition_fails_with_a_typed_cause() {
    let dir = TempDir::new().unwrap();
    let error = compose_text(
        "before\n\n::block when=\"ctx.os == 'linux'\"\ninside\n::end-block\n\nafter\n",
        ComposeOptions::new_with_context(date_time_only(dir.path())),
    )
    .expect_err("a page-block condition must not read an uncaptured group");
    assert_not_captured(&error, "os", ContextGroup::Os);
    assert!(matches!(error, MarkdownError::PageBlock(_)), "{error:?}");
}

#[test]
fn transclusion_condition_fails_with_a_typed_cause() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "child.md", "child\n");
    let root = write(dir.path(), "root.md", "root\n\n::file ./child.md when=\"ctx.os == 'linux'\"\n");
    let error = compose_file(&root, ComposeOptions::new_with_context(date_time_only(dir.path())))
        .expect_err("a transclusion condition must not read an uncaptured group");
    assert_not_captured(&error, "os", ContextGroup::Os);
    assert!(matches!(error, MarkdownError::Transclusion(_)), "{error:?}");
}

/// A `$()` ternary's condition and value branch fail with a typed cause. An
/// interpolated `{{ ctx.os }}` inside the value fails earlier, in frontmatter
/// interpolation of the same key. Both fail before any command runs.
#[test]
fn frontmatter_shell_ternary_condition_and_branch_fail_with_a_typed_cause() {
    let dir = TempDir::new().unwrap();
    for (value, shell_wrapper) in [
        ("\"$(ctx.os == 'linux' ? 'linux' : echo other)\"", true),
        ("\"$(true ? echo other : ctx.os)\"", true),
        ("\"$({{ ctx.os }} ? echo linux : 'other')\"", false),
        ("\"$(true ? echo {{ ctx.os }} : 'other')\"", false),
    ] {
        let error = compose_text(
            &format!("---\nplatform: {value}\n---\nbody\n"),
            ComposeOptions::new_with_context(date_time_only(dir.path()))
                .only(&[
                    ComposeOperation::FrontmatterInterpolation,
                    ComposeOperation::FrontmatterShellExpansion,
                ])
                .with_shell(ShellExpansionOptions {
                    policy_root: Some(dir.path().to_path_buf()),
                    ..Default::default()
                })
                .with_pre_approved_commands(HashSet::from(["echo other".to_string()])),
        )
        .expect_err("a $() ternary must not read an uncaptured group");
        assert_not_captured(&error, "os", ContextGroup::Os);
        if shell_wrapper {
            assert!(matches!(error, MarkdownError::ShellExpansion(_)), "{value}: {error:?}");
        } else {
            assert!(
                matches!(&error, MarkdownError::Interpolation { key: Some(key), .. } if key == "platform"),
                "{value}: {error:?}"
            );
        }
    }
}

/// Lenient transclusion turns a failing child into a notice. A missing capture
/// is not tolerated that way, because the notice is a partial document.
#[test]
fn a_transcluded_child_reading_an_uncaptured_group_fails_the_parent() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "child.md", "child|os={{ ctx.os }}|end\n");
    write(dir.path(), "middle.md", "middle\n\n::file ./child.md\n");
    let root = write(dir.path(), "root.md", "root\n\n::file ./middle.md\n");

    for fail_fast in [false, true] {
        let error = compose_file(
            &root,
            ComposeOptions::new_with_context(date_time_only(dir.path())).with_fail_fast(fail_fast),
        )
        .expect_err("no notice-filled partial document");
        assert_not_captured(&error, "os", ContextGroup::Os);
    }
}

// ── Only evaluated expressions fail ──────────────────────────────────────────

#[test]
fn unevaluated_references_do_not_fail_when_the_group_was_never_captured() {
    let dir = TempDir::new().unwrap();
    let rendered = compose_text(
        concat!(
            "ternary={{ false ? ctx.repo_root : 'unchosen' }}\n\n",
            "fallback={{ 'set' || ctx.repo_root }}\n\n",
            "guard={{ false && ctx.os }}\n\n",
            "literal={{{ ctx.gpu }}}\n\n",
            "::block when=\"false\"\n",
            "removed={{ ctx.os }}\n",
            "::end-block\n\n",
            "::block when=\"true || ctx.os == 'linux'\"\n",
            "short-circuited condition\n",
            "::end-block\n",
        ),
        ComposeOptions::new_with_context(date_time_only(dir.path())),
    )
    .expect("no unevaluated reference may fail");

    assert!(rendered.contains("ternary=unchosen"), "{rendered}");
    assert!(rendered.contains("fallback=set"), "{rendered}");
    assert!(rendered.contains("guard=false"), "{rendered}");
    assert!(rendered.contains("literal={{ ctx.gpu }}"), "{rendered}");
    assert!(!rendered.contains("removed="), "{rendered}");
    assert!(rendered.contains("short-circuited condition"), "{rendered}");
}

// ── Verifications #3 and #4: captured null and partial capture still render ──

/// A requested group with no supplied evidence is captured with its typed
/// null/empty projection: it renders, keeps its partial-capture diagnostic,
/// and raises no missing-capture error.
#[test]
fn a_captured_group_without_evidence_renders_its_typed_projection() {
    let dir = TempDir::new().unwrap();
    let content = "branch=[{{ ctx.branch }}]|dirty={{ length(ctx.dirty_files) }}|root=[{{ ctx.repo_root }}]\n";
    let context = ComposeContext::capture_for_content_with_evidence(
        dir.path(),
        content,
        &ContextCaptureEvidence::new(HashMap::new()),
    );
    for group in [ContextGroup::Git, ContextGroup::FileChanges, ContextGroup::Repo] {
        assert!(context.capture_requirements().contains(group), "{group:?} requested");
    }
    assert!(
        !context.diagnostics().is_empty(),
        "missing evidence keeps its partial-capture diagnostic"
    );

    let md: Markdown = content.into();
    let (composed, report) = md
        .compose_with(ComposeOptions::new_with_context(context))
        .expect("captured null/empty values render");
    // Cleanup escapes the `|` separators for Markdown table safety.
    assert_eq!(composed.content().trim().replace("\\|", "|"), "branch=[]|dirty=0|root=[]");
    assert!(
        report.warnings.iter().all(|warning| !warning.message.contains("did not capture")),
        "{:?}",
        report.warnings
    );
}

// ── Verification #5 and diagnostic stability ─────────────────────────────────

/// An unknown name keeps the unknown-variable warning, emitted once, and never
/// becomes a missing-capture error.
#[test]
fn an_unknown_key_warns_once_and_is_not_a_missing_capture() {
    let dir = TempDir::new().unwrap();
    let (composed, report) = Markdown::from("os={{ ctx.oss }}\n")
        .compose_with(ComposeOptions::new_with_context(date_time_only(dir.path())))
        .expect("an unknown key is not fatal");

    assert_eq!(composed.content().trim(), "os=");
    let unknown: Vec<_> = report
        .warnings
        .iter()
        .filter(|warning| warning.message.contains("unknown context variable 'ctx.oss'"))
        .collect();
    assert_eq!(unknown.len(), 1, "{:?}", report.warnings);
    assert!(
        report.warnings.iter().all(|warning| !warning.message.contains("did not capture")),
        "{:?}",
        report.warnings
    );
}

/// The same bad reference authored in two files is two issues, each carrying
/// its own source; composition stops at the first.
#[test]
fn the_same_reference_in_two_files_reports_each_files_own_source() {
    let dir = TempDir::new().unwrap();
    let first = write(dir.path(), "first.md", "os={{ ctx.os }}\n");
    let second = write(dir.path(), "second.md", "os={{ ctx.os }}\n");

    for path in [&first, &second] {
        let error = compose_file(path, ComposeOptions::new_with_context(date_time_only(dir.path())))
            .expect_err("each file fails on its own");
        assert_not_captured(&error, "os", ContextGroup::Os);
        match &error {
            MarkdownError::Interpolation { source, .. } => match source.as_ref() {
                SourceRef::OnDisk(context) => {
                    assert_eq!(context.display.file_name(), path.file_name())
                }
                other => panic!("expected an on-disk source, got {other:?}"),
            },
            other => panic!("expected an interpolation error, got {other:?}"),
        }
    }
}

// ── Pre-flight passes tolerate; the compose pass owns the verdict ────────────

/// Pre-flight discovery of a `$()` ternary runs on the context the compose
/// pass will use. Under ambient options that context captures the group, so
/// the branch command is discovered with its real value.
#[test]
fn ambient_preflight_discovers_a_ternary_branch_that_reads_runtime_context() {
    let md: Markdown =
        "---\nlabel: \"{{ ctx.os }}\"\nplatform: \"$(true ? echo {{ ctx.os }} : echo none)\"\n---\nbody\n"
            .into();
    let preflight = md
        .compose_preflight(&ComposeOptions::new())
        .expect("pre-flight reads the upgraded context");

    let commands: Vec<_> = preflight.entries.iter().map(|entry| entry.normalized.as_str()).collect();
    assert!(commands.contains(&"echo none"), "{commands:?}");
    assert!(
        commands.iter().any(|command| command.starts_with("echo ") && *command != "echo none"
            && command.len() > "echo ".len()),
        "the ctx.os branch must be discovered with a value: {commands:?}"
    );
}

/// Shell-command discovery composes without page blocks, so it reaches body
/// content a false `::block` removes. With a caller-supplied context lacking
/// the group, discovery tolerates that reference and the compose pass owns the
/// verdict: it succeeds when the block is removed and fails when it is kept.
#[test]
fn preflight_tolerates_a_body_missing_capture_that_the_compose_pass_owns() {
    let dir = TempDir::new().unwrap();
    let options = || ComposeOptions::new_with_context(date_time_only(dir.path()));
    let document = |condition: &str| {
        format!(
            "---\nplatform: \"$(true ? echo ready : echo none)\"\n---\n::block when=\"{condition}\"\nos={{{{ ctx.os }}}}\n::end-block\n\ndone\n"
        )
    };

    for condition in ["false", "true"] {
        let md: Markdown = document(condition).as_str().into();
        let preflight = md
            .compose_preflight(&options())
            .expect("discovery is not the verdict on a missing capture");
        let mut commands: Vec<_> = preflight.entries.iter().map(|entry| entry.normalized.as_str()).collect();
        commands.sort_unstable();
        assert_eq!(commands, ["echo none", "echo ready"], "when={condition}");
    }

    let body_stages = [ComposeOperation::PageBlocks, ComposeOperation::Interpolation];
    let removed = compose_text(&document("false"), options().only(&body_stages))
        .expect("a removed block is never evaluated");
    assert!(!removed.contains("os=") && removed.contains("done"), "{removed}");

    let error = compose_text(&document("true"), options().only(&body_stages))
        .expect_err("the compose pass owns the verdict");
    assert_not_captured(&error, "os", ContextGroup::Os);
}

/// A `$()` command that embeds a key left raw by a missing capture is reported
/// as that missing capture, not as a dynamic command shape: the compose pass
/// fails the same key in frontmatter interpolation whatever the conditions.
#[test]
fn preflight_reports_a_missing_capture_instead_of_a_dynamic_command_shape() {
    let dir = TempDir::new().unwrap();
    let md: Markdown =
        "---\nplatform: \"$(true ? echo {{ ctx.os }} : echo none)\"\n---\nbody\n".into();
    let error = md
        .compose_preflight(&ComposeOptions::new_with_context(date_time_only(dir.path())))
        .expect_err("the command's shape cannot be known");
    assert_not_captured(&error, "os", ContextGroup::Os);
    assert!(!error.to_string().contains("dynamic command shape"), "{error}");
}
